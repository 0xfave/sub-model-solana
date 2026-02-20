Here is a clean, professional, and comprehensive **README.md** file for your Solana subscription billing program (`sub_model`).  
It’s written in Markdown, optimized for GitHub visibility, and tailored to impress judges/reviewers for the Superteam bounty.

```markdown
# Solana On-Chain Subscription Billing – sub_model

A production-grade, trustless subscription billing system built on Solana using Anchor.  
Supports recurring payments in SPL tokens (e.g. USDC), trials, pauses/resumes with time freeze, grace periods, failure retries, cancel-at-period-end, reactivation, and merchant analytics (active subscribers + lifetime revenue).

**Key features**:
- Multi-merchant / multi-tenant (anyone can create plans)
- Direct token transfers to merchant ATA (no escrow/vault)
- Trial periods (up to 14 days)
- Permissionless-but-secure renewals (user-signed)
- Pause/resume with billing period freeze
- Grace period (3 days) for past-due payments
- Automatic state transitions via `process_expired` (keeper-callable)
- Reactivation from unpaid state
- Mint validation + overflow protection
- Full event emission for off-chain indexing

Built for the [Superteam Earn bounty](https://superteam.fun/earn/listing/rebuild-production-backend-systems-as-on-chain-rust-programs/).

## Table of Contents

- [Features](#features)
- [Architecture Overview](#architecture-overview)
- [Web2 vs Solana Comparison](#web2-vs-solana-comparison)
- [Installation & Setup](#installation--setup)
- [Usage](#usage)
  - [Create a Plan](#create-a-plan)
  - [Subscribe](#subscribe)
  - [Renew](#renew)
  - [Cancel](#cancel)
  - [Pause / Resume](#pause--resume)
  - [Process Expired](#process-expired)
  - [Reactivate Unpaid](#reactivate-unpaid)
- [Security & Best Practices](#security--best-practices)
- [Limitations & Future Work](#limitations--future-work)
- [Testing](#testing)
- [Deployment](#deployment)
- [Contributing](#contributing)
- [License](#license)

## Features

- **Plans** — merchant-defined (price, duration, trial, token mint)
- **Subscriptions** — per-user-per-plan PDA
- **Payment** — direct SPL token transfer (no custody)
- **States** — Trialing, Active, PastDue, Unpaid, Canceled, Paused
- **Grace** — 3-day access window for PastDue
- **Retry** — up to 3 failed renewals before Unpaid
- **Pause/Resume** — user or merchant, freezes billing period
- **Cancel** — immediate or at-period-end
- **Reactivate** — pay to restart from Unpaid
- **Events** — emitted for all major actions (indexing-friendly)
- **Validation** — mint match, overflow checks, terminal-state re-subscribe

## Architecture Overview

- **Plan** PDA: `[plan, owner, plan_id]`
- **Subscription** PDA: `[subscription, user, plan]`
- Payments go straight to merchant's ATA
- No cron → use `process_expired` (permissionless keeper) for state transitions
- Time-based logic uses `Clock::unix_timestamp`
- Checked arithmetic prevents overflows

## Web2 vs Solana Comparison

| Aspect                  | Web2 (e.g. Stripe)                          | Solana On-Chain (this program)                          | Trade-offs / Notes                                      |
|-------------------------|---------------------------------------------|----------------------------------------------------------|---------------------------------------------------------|
| Billing trigger         | Server cron                                 | Permissionless `renew` + keeper / bot                    | No central server, but requires off-chain automation    |
| Payment custody         | Platform holds funds                        | Direct to merchant ATA                                   | Trustless, no withdrawal step                           |
| Retry logic             | Automatic dunning                           | Manual retry (up to 3), state persists                   | Needs off-chain notification                            |
| Grace period            | Server-side access control                  | On-chain `has_access()` + grace deadline                 | Fully auditable                                         |
| Cancellation            | Immediate or end-of-period                  | Immediate or `cancel_at_period_end` + `process_expired`  | Requires keeper call for end-of-period transition       |
| Pause                   | Admin/user toggle                           | User/merchant, freezes period duration                   | Fair to users                                           |
| Data transparency       | Opaque                                      | All state on-chain, events for indexing                  | Public auditability                                     |
| Cost per renewal        | ~0–2.9% + fixed                             | ~0.000005 SOL + token transfer fee                       | Extremely cheap                                         |
| Automation reliability  | 99.99%                                      | Depends on keepers/bots                                  | Can use Switchboard or custom Node.js bot               |

## Installation & Setup

```bash
# Clone repo
git clone <your-repo-url>
cd sub-model

# Install Anchor
anchor --version   # should be 0.30.x or later

# Build
anchor build

# Test
anchor test
```

Dependencies (in `Cargo.toml`):

```toml
[dependencies]
anchor-lang = "0.32.1"
anchor-spl = "0.32.1"
```

## Usage

### Create a Plan

```ts
await program.methods
  .createPlan("pro_monthly", 1, new BN(1000000), new BN(2592000), new BN(7), mintPubkey)
  .accounts({ owner, plan: planPda, tokenMintAccount: mint })
  .rpc();
```

### Subscribe

```ts
await program.methods
  .subscribe()
  .accounts({ user, plan: planPda, subscription: subPda, userTokenAccount, merchantTokenAccount })
  .signers([userKeypair])
  .rpc();
```

### Renew

```ts
await program.methods
  .renew()
  .accounts({ user, plan: planPda, subscription: subPda, userTokenAccount, merchantTokenAccount })
  .signers([userKeypair])
  .rpc();
```

### Cancel

```ts
// immediate
await program.methods
  .cancel(true)
  .accounts({ user, plan: planPda, subscription: subPda })
  .signers([userKeypair])
  .rpc();

// at period end
await program.methods
  .cancel(false)
  .accounts({ ... })
  .rpc();
```

### Pause / Resume

```ts
await program.methods.pause().accounts({ caller: user.publicKey, plan, subscription }).signers([user]).rpc();
await program.methods.resume().accounts({ caller: user.publicKey, plan, subscription }).signers([user]).rpc();
```

### Process Expired (keeper / anyone)

```ts
await program.methods
  .processExpired()
  .accounts({ plan, subscription })
  .rpc();
```

### Reactivate Unpaid

```ts
await program.methods
  .reactivate()
  .accounts({ user, plan, subscription, userTokenAccount, merchantTokenAccount })
  .signers([user])
  .rpc();
```

## Security & Best Practices

- All PDAs use deterministic seeds
- Mint & owner checks on token accounts
- Checked arithmetic for timestamps & counters
- No reentrancy (Anchor CPI safety)
- Permissionless keeper pattern (no central cron)
- Events for full audit trail

**Recommendations**:
- Use Helius/QuickNode webhooks for real-time event listening
- Run a Node.js bot for automated `renew` + `process_expired`
- Audit before mainnet

## Limitations & Future Work

- Renew requires user signature (not fully auto without delegation)
- No upgrade/downgrade between plans
- No proration/refunds on cancel
- No multi-token plans
- No subscription metadata (name, features)

## Testing

```bash
anchor test
```

Tests cover:
- Plan creation
- Trial & paid subscribe
- Renewal success/failure
- Pause/resume time freeze
- Cancel immediate & end-of-period
- Process expired transitions
- Reactivate from unpaid

## Deployment

```bash
# Build
anchor build

# Deploy to devnet
anchor deploy --provider.cluster devnet

# Update program ID in Anchor.toml and lib.rs
```
