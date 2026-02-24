use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("6PyMsXWBKo77maWZir1kpE8i71Kuwprgm5hR9e5Ng2r3");

pub const GRACE_PERIOD_SECONDS: i64 = 3 * 24 * 60 * 60; // 3 days grace for past_due

pub const MAX_RETRIES: u8 = 3;

pub const MAX_TRIAL_DAYS: u64 = 14; // prevent absurdly long trials

#[program]
pub mod subscription_model {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }

    /// Create a new subscription plan (merchant / admin only)
    pub fn create_plan(
        ctx: Context<CreatePlan>,
        plan_id: String,
        version: u16,
        price: u64, // in token smallest units (e.g. 1_000_000 for 1 USDC)
        duration_seconds: u64,
        trial_days: u64, // 0 = no trial
        token_mint: Pubkey,
    ) -> Result<()> {
        // Sanity checks
        require!(price > 0, ErrorCode::InvalidPrice);
        require!(duration_seconds > 0, ErrorCode::InvalidDuration);
        require!(trial_days <= MAX_TRIAL_DAYS, ErrorCode::TrialTooLong);
        // Ensure duration fits in i64 for timestamp arithmetic
        require!(duration_seconds <= i64::MAX as u64, ErrorCode::DurationOverflow);

        let plan = &mut ctx.accounts.plan;
        plan.owner = ctx.accounts.owner.key();
        plan.plan_id = plan_id;
        plan.version = version;
        plan.price = price;
        plan.duration_seconds = duration_seconds;
        plan.trial_days = trial_days;
        plan.token_mint = token_mint;
        plan.bump = ctx.bumps.plan;

        emit!(PlanCreated { plan: plan.key(), owner: plan.owner, price, duration_seconds, trial_days, token_mint });
        Ok(())
    }

    /// User subscribes to a plan (pays first period or starts trial)
    pub fn subscribe(ctx: Context<Subscribe>) -> Result<()> {
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        let plan = &mut ctx.accounts.plan;
        let subscription = &mut ctx.accounts.subscription;

        // If subscription account already exists (re-subscribing), ensure it's in a terminal state
        if subscription.user != Pubkey::default() {
            require!(
                matches!(subscription.status, SubscriptionStatus::Canceled | SubscriptionStatus::Unpaid),
                ErrorCode::SubscriptionStillActive
            );
        }

        subscription.user = ctx.accounts.user.key();
        subscription.plan = plan.key();
        subscription.start_ts = now;
        subscription.current_period_start = now;
        subscription.failed_attempts_count = 0;
        subscription.cancel_at_period_end = false;
        subscription.paused_at = None;
        subscription.bump = ctx.bumps.subscription;

        // Determine initial status and period
        if plan.trial_days > 0 {
            subscription.status = SubscriptionStatus::Trialing;
            subscription.previous_status = subscription.status;
            subscription.current_period_end = now + (plan.trial_days as i64 * 24 * 60 * 60);
            plan.active_subscribers = plan.active_subscribers.checked_add(1).ok_or(ErrorCode::SubscribersOverflow)?;
            // No payment on trial start
        } else {
            // Pay for first period
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_account.to_account_info(),
                        to: ctx.accounts.merchant_token_account.to_account_info(),
                        authority: ctx.accounts.user.to_account_info(),
                    },
                ),
                plan.price,
            )?;

            subscription.status = SubscriptionStatus::Active;
            subscription.current_period_end = now + (plan.duration_seconds as i64);
            subscription.last_payment_ts = Some(now);
            subscription.previous_status = SubscriptionStatus::Active;
            plan.active_subscribers = plan.active_subscribers.checked_add(1).ok_or(ErrorCode::SubscribersOverflow)?;
            plan.lifetime_revenue = plan.lifetime_revenue.checked_add(plan.price).ok_or(ErrorCode::RevenueOverflow)?;
        }

        emit!(SubscriptionCreated {
            subscription: subscription.key(),
            user: ctx.accounts.user.key(),
            plan: ctx.accounts.plan.key(),
            status: subscription.status,
            start_ts: subscription.start_ts,
            current_period_end: subscription.current_period_end,
        });

        Ok(())
    }

    /// Trigger renewal – requires the user's signature.
    pub fn renew(ctx: Context<Renew>) -> Result<()> {
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        let subscription = &mut ctx.accounts.subscription;
        let plan = &mut ctx.accounts.plan;

        // Ensure subscription belongs to the provided plan and user
        require!(subscription.plan == plan.key(), ErrorCode::PlanMismatch);
        require!(subscription.user == ctx.accounts.user.key(), ErrorCode::UserMismatch);

        let old_status = subscription.status;

        // Determine if eligible for renewal
        require!(subscription.eligible_for_renewal(now), ErrorCode::NotEligibleForRenewal);

        // Check user's token balance
        let user_balance = ctx.accounts.user_token_account.amount;
        if user_balance >= plan.price {
            // Sufficient funds: perform transfer
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_account.to_account_info(),
                        to: ctx.accounts.merchant_token_account.to_account_info(),
                        authority: ctx.accounts.user.to_account_info(),
                    },
                ),
                plan.price,
            )?;

            // Update subscription for successful renewal
            subscription.current_period_start = now;
            subscription.current_period_end =
                now.checked_add(plan.duration_seconds as i64).ok_or(ErrorCode::TimestampOverflow)?;
            subscription.status = SubscriptionStatus::Active;
            subscription.last_payment_ts = Some(now);
            subscription.failed_attempts_count = 0;
            subscription.cancel_at_period_end = false;
            plan.lifetime_revenue = plan.lifetime_revenue.checked_add(plan.price).ok_or(ErrorCode::RevenueOverflow)?;

            emit!(RenewalSucceeded {
                subscription: subscription.key(),
                timestamp: now,
                new_end: subscription.current_period_end,
            });

            emit!(StatusChanged {
                subscription: subscription.key(),
                old_status,
                new_status: SubscriptionStatus::Active,
                reason: "Renewal payment succeeded".to_string(),
            });
        } else {
            // Insufficient funds: mark as failed but do NOT attempt transfer
            subscription.failed_attempts_count = subscription.failed_attempts_count.saturating_add(1);

            let new_status = if subscription.failed_attempts_count >= MAX_RETRIES {
                SubscriptionStatus::Unpaid
            } else {
                SubscriptionStatus::PastDue
            };
            subscription.status = new_status;

            emit!(RenewalFailed {
                subscription: subscription.key(),
                timestamp: now,
                attempt: subscription.failed_attempts_count,
                reason: "Insufficient funds".to_string(),
            });
            emit!(StatusChanged {
                subscription: subscription.key(),
                old_status,
                new_status,
                reason: "Payment failed (insufficient funds)".to_string(),
            });
        }

        Ok(())
    }

    /// user cancels their subscription
    pub fn cancel(ctx: Context<Cancel>, immediate: bool) -> Result<()> {
        let subscription = &mut ctx.accounts.subscription;

        require_keys_eq!(subscription.user, ctx.accounts.user.key(), ErrorCode::Unauthorized);

        // Prevent canceling an already canceled subscription
        require!(subscription.status != SubscriptionStatus::Canceled, ErrorCode::SubscriptionStillActive);

        let old_status = subscription.status;

        if immediate {
            subscription.status = SubscriptionStatus::Canceled;
            subscription.current_period_end = Clock::get()?.unix_timestamp; // ends now
                                                                            // Decrement active subscribers
            let plan = &mut ctx.accounts.plan;
            plan.active_subscribers = plan.active_subscribers.checked_sub(1).ok_or(ErrorCode::SubscribersUnderflow)?;
        // or custom Underflow error
        } else {
            subscription.cancel_at_period_end = true;
            // Status stays Active/PastDue/Trailing until period ends
        }

        emit!(SubscriptionCanceled {
            subscription: subscription.key(),
            user: ctx.accounts.user.key(),
            immediate,
            timestamp: Clock::get()?.unix_timestamp,
        });

        emit!(StatusChanged {
            subscription: subscription.key(),
            old_status,
            new_status: subscription.status,
            reason: if immediate { "Immediate cancel".to_string() } else { "Cancel at period end".to_string() }
        });

        Ok(())
    }

    /// Pause a subscription (user or merchant can call)
    pub fn pause(ctx: Context<Pause>) -> Result<()> {
        let subscription = &mut ctx.accounts.subscription;
        let caller = ctx.accounts.caller.key();

        // Only user or plan owner (merchant) can pause
        let is_authorized = caller == subscription.user || caller == ctx.accounts.plan.owner;
        require!(is_authorized, ErrorCode::Unauthorized);

        let old_status = subscription.status;

        require!(old_status != SubscriptionStatus::Paused, ErrorCode::AlreadyPaused);

        require!(
            old_status != SubscriptionStatus::Canceled && old_status != SubscriptionStatus::Unpaid,
            ErrorCode::CannotPauseInFinalState
        );

        require!(old_status != SubscriptionStatus::PastDue, ErrorCode::CannotPauseWhenPastDue);

        // Save the state we'll return to on resume
        subscription.previous_status = old_status.clone();
        subscription.status = SubscriptionStatus::Paused;
        subscription.paused_at = Some(Clock::get()?.unix_timestamp);

        emit!(StatusChanged {
            subscription: subscription.key(),
            old_status,
            new_status: SubscriptionStatus::Paused,
            reason: "Subscription paused by user or merchant".to_string(),
        });

        Ok(())
    }

    /// Resume a paused subscription (user or merchant can call)
    pub fn resume(ctx: Context<Resume>) -> Result<()> {
        let subscription = &mut ctx.accounts.subscription;
        let caller = ctx.accounts.caller.key();

        let is_authorized = caller == subscription.user || caller == ctx.accounts.plan.owner;

        let now = Clock::get()?.unix_timestamp;

        require!(now <= subscription.current_period_end, ErrorCode::PeriodExpired);

        require!(is_authorized, ErrorCode::Unauthorized);

        let old_status = subscription.status;

        require!(old_status == SubscriptionStatus::Paused, ErrorCode::NotPaused);

        // Prevent resuming a subscription that expired while paused
        require!(now <= subscription.current_period_end, ErrorCode::PeriodExpired);

        let paused_at = subscription.paused_at;
        subscription.paused_at = None;

        let restored_status = subscription.previous_status;
        subscription.status = restored_status;

        // Extend period by pause duration (freeze time)
        if let Some(paused_at) = paused_at {
            let pause_duration =
                Clock::get()?.unix_timestamp.checked_sub(paused_at).ok_or(ErrorCode::TimestampOverflow)?;
            subscription.current_period_end =
                subscription.current_period_end.checked_add(pause_duration).ok_or(ErrorCode::TimestampOverflow)?;
        }

        emit!(StatusChanged {
            subscription: subscription.key(),
            old_status,
            new_status: subscription.status,
            reason: "Subscription resumed".to_string(),
        });

        Ok(())
    }

    /// Process expired subscriptions:
    /// - If cancel_at_period_end is true, mark as Canceled.
    /// - If Active and expired, move to PastDue.
    /// - If status is PastDue and beyond grace, mark as Unpaid.
    pub fn process_expired(ctx: Context<ProcessExpired>) -> Result<()> {
        let subscription = &mut ctx.accounts.subscription;
        let now = Clock::get()?.unix_timestamp;

        // Only process if period has ended
        if now <= subscription.current_period_end {
            return Ok(());
        }

        let old_status = subscription.status;

        match subscription.status {
            SubscriptionStatus::Active => {
                msg!("❤️Entering match process expired active");
                if subscription.cancel_at_period_end {
                    subscription.status = SubscriptionStatus::Canceled;
                    emit!(StatusChanged {
                        subscription: subscription.key(),
                        old_status,
                        new_status: SubscriptionStatus::Canceled,
                        reason: "Cancel at period end executed".to_string(),
                    });
                } else {
                    subscription.status = SubscriptionStatus::PastDue;
                    emit!(StatusChanged {
                        subscription: subscription.key(),
                        old_status,
                        new_status: SubscriptionStatus::PastDue,
                        reason: "Period ended, entering grace".to_string(),
                    });
                }
            }
            SubscriptionStatus::Trialing => {
                if subscription.cancel_at_period_end {
                    subscription.status = SubscriptionStatus::Canceled;
                    ctx.accounts.plan.active_subscribers =
                        ctx.accounts.plan.active_subscribers.checked_sub(1).ok_or(ErrorCode::SubscribersUnderflow)?;
                    emit!(StatusChanged {
                        subscription: subscription.key(),
                        old_status,
                        new_status: SubscriptionStatus::Canceled,
                        reason: "Cancel at period end executed".to_string(),
                    });
                } else if now >= subscription.grace_deadline() {
                    // Grace period also expired, move directly to Unpaid
                    subscription.status = SubscriptionStatus::Unpaid;
                    ctx.accounts.plan.active_subscribers =
                        ctx.accounts.plan.active_subscribers.checked_sub(1).ok_or(ErrorCode::SubscribersUnderflow)?;
                    emit!(StatusChanged {
                        subscription: subscription.key(),
                        old_status,
                        new_status: SubscriptionStatus::Unpaid,
                        reason: "Trial and grace period ended".to_string(),
                    });
                } else {
                    // Trial ended, move to PastDue (within grace period)
                    subscription.status = SubscriptionStatus::PastDue;
                    // Don't decrement - user still active during grace
                    emit!(StatusChanged {
                        subscription: subscription.key(),
                        old_status,
                        new_status: SubscriptionStatus::PastDue,
                        reason: "Trial ended".to_string(),
                    });
                }
            }
            SubscriptionStatus::PastDue => {
                if now >= subscription.grace_deadline() {
                    subscription.status = SubscriptionStatus::Unpaid;
                    ctx.accounts.plan.active_subscribers =
                        ctx.accounts.plan.active_subscribers.checked_sub(1).ok_or(ErrorCode::SubscribersUnderflow)?;

                    emit!(StatusChanged {
                        subscription: subscription.key(),
                        old_status,
                        new_status: SubscriptionStatus::Unpaid,
                        reason: "Grace period expired".to_string(),
                    });
                }
            }
            _ => {
                msg!("Exhausted match arm🔥")
            }
        }

        Ok(())
    }

    /// Reactivate an unpaid subscription by paying for a new period
    pub fn reactivate(ctx: Context<Reactivate>) -> Result<()> {
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        let subscription = &mut ctx.accounts.subscription;
        let plan = &mut ctx.accounts.plan;

        // Only allow reactivation if currently Unpaid
        require!(subscription.status == SubscriptionStatus::Unpaid, ErrorCode::NotUnpaid);

        // Transfer payment for a new period
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.merchant_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            plan.price,
        )?;

        // Reset subscription to active with a new period starting now
        subscription.status = SubscriptionStatus::Active;
        subscription.current_period_start = now;
        subscription.current_period_end =
            now.checked_add(plan.duration_seconds as i64).ok_or(ErrorCode::TimestampOverflow)?;
        subscription.last_payment_ts = Some(now);
        subscription.failed_attempts_count = 0;
        subscription.cancel_at_period_end = false;
        subscription.previous_status = SubscriptionStatus::Active;
        subscription.paused_at = None;

        plan.active_subscribers = plan.active_subscribers.checked_add(1).ok_or(ErrorCode::SubscribersOverflow)?;
        plan.lifetime_revenue = plan.lifetime_revenue.checked_add(plan.price).ok_or(ErrorCode::RevenueOverflow)?;

        emit!(StatusChanged {
            subscription: subscription.key(),
            old_status: SubscriptionStatus::Unpaid,
            new_status: SubscriptionStatus::Active,
            reason: "Reactivation payment succeeded".to_string(),
        });

        Ok(())
    }
}

// ---------- Events ----------
#[event]
pub struct PlanCreated {
    pub plan: Pubkey,
    pub owner: Pubkey,
    pub price: u64,
    pub duration_seconds: u64,
    pub trial_days: u64,
    pub token_mint: Pubkey,
}

#[event]
pub struct SubscriptionCreated {
    pub subscription: Pubkey,
    pub user: Pubkey,
    pub plan: Pubkey,
    pub status: SubscriptionStatus,
    pub start_ts: i64,
    pub current_period_end: i64,
}

#[event]
pub struct RenewalSucceeded {
    pub subscription: Pubkey,
    pub timestamp: i64,
    pub new_end: i64,
}

#[event]
pub struct RenewalFailed {
    pub subscription: Pubkey,
    pub timestamp: i64,
    pub attempt: u8,
    pub reason: String,
}

#[event]
pub struct SubscriptionCanceled {
    pub subscription: Pubkey,
    pub user: Pubkey,
    pub immediate: bool,
    pub timestamp: i64,
}

#[event]
pub struct StatusChanged {
    pub subscription: Pubkey,
    pub old_status: SubscriptionStatus,
    pub new_status: SubscriptionStatus,
    pub reason: String,
}

#[event]
pub struct PlanUpgraded {
    pub subscription: Pubkey,
    pub from_plan: Pubkey,
    pub to_plan: Pubkey,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct Initialize {}

// ---------- Enums ----------
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug, Copy, InitSpace)]
#[repr(u8)]
pub enum SubscriptionStatus {
    // In free trial; no charge yet. Access granted.
    Trialing = 0,
    // Paid and current. Normal access; eligible for auto-renew.
    Active = 1,
    // Recent invoice failed but still granting grace access.
    PastDue = 3,
    // Retries exhausted; invoice unpaid. Access should be revoked.
    Unpaid = 4,
    // Explicitly canceled (immediate or at period end).
    Canceled = 5,
    // Paused (e.g., trial end with no payment method, or admin pause).
    Paused = 6,
}

// ---------- Accounts ----------
#[account]
#[derive(Debug, InitSpace)]
pub struct Plan {
    pub owner: Pubkey,
    #[max_len(32)]
    pub plan_id: String,
    pub version: u16,
    pub price: u64,
    pub duration_seconds: u64,
    pub trial_days: u64,
    pub token_mint: Pubkey,
    pub bump: u8,
    pub active_subscribers: u64,
    pub lifetime_revenue: u64,
}

#[account]
#[derive(Debug, InitSpace)]
pub struct Subscription {
    pub user: Pubkey,
    pub plan: Pubkey,
    pub status: SubscriptionStatus,
    pub previous_status: SubscriptionStatus,
    pub start_ts: i64,
    pub current_period_start: i64,
    pub current_period_end: i64,
    pub cancel_at_period_end: bool,
    pub paused_at: Option<i64>,
    pub bump: u8,
    pub last_payment_ts: Option<i64>,
    pub failed_attempts_count: u8,
}

impl Subscription {
    /// Returns true if the user should currently have access
    pub fn has_access(&self, now: i64) -> bool {
        match self.status {
            SubscriptionStatus::Trialing | SubscriptionStatus::Active => self.current_period_end > now,
            SubscriptionStatus::PastDue => now < self.grace_deadline(),
            // All other statuses: no access
            _ => false,
        }
    }

    /// Returns if this subscription is eligible for renewal attempt
    pub fn eligible_for_renewal(&self, now: i64) -> bool {
        match self.status {
            SubscriptionStatus::Active | SubscriptionStatus::PastDue => {
                now >= self.current_period_end && !self.cancel_at_period_end
            }
            SubscriptionStatus::Trialing => {
                // Trial can be renewed only if it has ended
                now >= self.current_period_end
            }
            _ => false,
        }
    }

    /// Whether access should be revoked right now (for off-chain enforcement)
    pub fn should_revoke_access(&self, now: i64) -> bool {
        matches!(self.status, SubscriptionStatus::Unpaid | SubscriptionStatus::Canceled)
            || (self.status == SubscriptionStatus::PastDue && now >= self.grace_deadline())
    }

    pub fn grace_deadline(&self) -> i64 {
        self.current_period_end.checked_add(GRACE_PERIOD_SECONDS).unwrap_or(i64::MAX)
    }
}
// ────────────────────────────────────────────────
// Account structs (constraints)
// ────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(plan_id: String, version: u16, price: u64, duration_seconds: u64, trial_days: u64, token_mint: Pubkey)]
pub struct CreatePlan<'info> {
    #[account(
        init,
        payer = owner,
        space = Plan::DISCRIMINATOR.len() + Plan::INIT_SPACE,
        seeds = [b"plan", owner.key().as_ref(), plan_id.as_bytes()],
        bump
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        constraint = token_mint_account.key() == token_mint @ ErrorCode::InvalidMint
    )]
    pub token_mint_account: Account<'info, Mint>,

    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Subscribe<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"plan", plan.owner.as_ref(), plan.plan_id.as_bytes()],
        bump = plan.bump
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        init_if_needed,
        payer = user,
        space = Subscription::DISCRIMINATOR.len() + Subscription::INIT_SPACE,
        seeds = [b"subscription", user.key().as_ref(), plan.key().as_ref()],
        bump
    )]
    pub subscription: Account<'info, Subscription>,

    #[account(
        constraint = user_token_account.owner == user.key() @ ErrorCode::InvalidUserTokenAccount,
        constraint = user_token_account.mint == plan.token_mint @ ErrorCode::MintMismatch
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        constraint = merchant_token_account.owner == plan.owner @ ErrorCode::InvalidMerchantAccount,
        constraint = merchant_token_account.mint == plan.token_mint @ ErrorCode::MintMismatch
    )]
    pub merchant_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Renew<'info> {
    #[account(mut)]
    pub user: Signer<'info>, // payer of tx fees + token transfer authority

    #[account(
        mut,
        seeds = [b"plan", plan.owner.as_ref(), plan.plan_id.as_bytes()],
        bump = plan.bump
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        mut,
        seeds = [b"subscription", user.key().as_ref(), plan.key().as_ref()],
        bump = subscription.bump,
        constraint = subscription.plan == plan.key() @ ErrorCode::PlanMismatch,
        constraint = subscription.user == user.key() @ ErrorCode::UserMismatch,
    )]
    pub subscription: Account<'info, Subscription>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ ErrorCode::InvalidUserTokenAccount,
        constraint = user_token_account.mint == plan.token_mint @ ErrorCode::MintMismatch
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = merchant_token_account.owner == plan.owner @ ErrorCode::InvalidMerchantAccount,
        constraint = merchant_token_account.mint == plan.token_mint @ ErrorCode::MintMismatch
    )]
    pub merchant_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"plan", plan.owner.as_ref(), plan.plan_id.as_bytes()],
        bump = plan.bump
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        mut,
        seeds = [b"subscription", user.key().as_ref(), subscription.plan.as_ref()],
        bump = subscription.bump,
        constraint = subscription.user == user.key() @ ErrorCode::UserMismatch,
        constraint = subscription.plan == plan.key() @ ErrorCode::PlanMismatch,
    )]
    pub subscription: Account<'info, Subscription>,
}

#[derive(Accounts)]
pub struct Pause<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        seeds = [b"plan", plan.owner.as_ref(), plan.plan_id.as_bytes()],
        bump = plan.bump
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        mut,
        seeds = [b"subscription", subscription.user.as_ref(), plan.key().as_ref()],
        bump = subscription.bump,
        constraint = subscription.plan == plan.key() @ ErrorCode::PlanMismatch,
    )]
    pub subscription: Account<'info, Subscription>,
}

#[derive(Accounts)]
pub struct Resume<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        seeds = [b"plan", plan.owner.as_ref(), plan.plan_id.as_bytes()],
        bump = plan.bump
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        mut,
        seeds = [b"subscription", subscription.user.as_ref(), plan.key().as_ref()],
        bump = subscription.bump,
        constraint = subscription.plan == plan.key() @ ErrorCode::PlanMismatch,
    )]
    pub subscription: Account<'info, Subscription>,
}

#[derive(Accounts)]
pub struct ProcessExpired<'info> {
    #[account(
        mut,
        seeds = [b"plan", plan.owner.as_ref(), plan.plan_id.as_bytes()],
        bump = plan.bump
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        mut,
        seeds = [b"subscription", subscription.user.as_ref(), subscription.plan.as_ref()],
        bump = subscription.bump,
        constraint = subscription.plan == plan.key() @ ErrorCode::PlanMismatch,
    )]
    pub subscription: Account<'info, Subscription>,
}

#[derive(Accounts)]
pub struct Reactivate<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"plan", plan.owner.as_ref(), plan.plan_id.as_bytes()],
        bump = plan.bump
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        mut,
        seeds = [b"subscription", user.key().as_ref(), plan.key().as_ref()],
        bump = subscription.bump,
        constraint = subscription.plan == plan.key() @ ErrorCode::PlanMismatch,
        constraint = subscription.user == user.key() @ ErrorCode::UserMismatch,
    )]
    pub subscription: Account<'info, Subscription>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ ErrorCode::InvalidUserTokenAccount,
        constraint = user_token_account.mint == plan.token_mint @ ErrorCode::MintMismatch
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = merchant_token_account.owner == plan.owner @ ErrorCode::InvalidMerchantAccount,
        constraint = merchant_token_account.mint == plan.token_mint @ ErrorCode::MintMismatch
    )]
    pub merchant_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// ---------- Error Codes ----------
#[error_code]
pub enum ErrorCode {
    #[msg("Subscription not eligible for renewal")]
    NotEligibleForRenewal,
    #[msg("Too early to renew")]
    TooEarlyForRenewal,
    #[msg("Unauthorized caller")]
    Unauthorized,
    #[msg("Invalid merchant token account")]
    InvalidMerchantAccount,
    #[msg("Invalid user token account")]
    InvalidUserTokenAccount,
    #[msg("Token mint mismatch")]
    MintMismatch,
    #[msg("Subscription is already paused")]
    AlreadyPaused,
    #[msg("Cannot pause in final state (canceled/unpaid)")]
    CannotPauseInFinalState,
    #[msg("Subscription is not paused")]
    NotPaused,
    #[msg("Price must be greater than zero")]
    InvalidPrice,
    #[msg("Duration must be greater than zero")]
    InvalidDuration,
    #[msg("Trial period too long")]
    TrialTooLong,
    #[msg("Duration exceeds maximum timestamp")]
    DurationOverflow,
    #[msg("Timestamp arithmetic overflow")]
    TimestampOverflow,
    #[msg("Subscription still active (cannot re-subscribe)")]
    SubscriptionStillActive,
    #[msg("Plan mismatch")]
    PlanMismatch,
    #[msg("User mismatch")]
    UserMismatch,
    #[msg("Subscription is not in Unpaid state")]
    NotUnpaid,
    #[msg("Invalid Mint")]
    InvalidMint,
    #[msg("Cannot pause a subscription that is PastDue")]
    CannotPauseWhenPastDue,
    #[msg("Plan ID exceeds maximum length")]
    PlanIdTooLong,
    #[msg("Downgrade not supported")]
    DowngradeNotSupported,
    #[msg("Subscription period has expired")]
    PeriodExpired,
    #[msg("Subscriber Underflow")]
    SubscribersUnderflow,
    #[msg("Subscriber Overflow")]
    SubscribersOverflow,
    #[msg("Revenue Overflow")]
    RevenueOverflow,
}
