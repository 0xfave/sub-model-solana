use crate::test_util::{
    create_plan, get_plan, get_subscription, get_token_balance, renew, set_clock, setup,
    setup_with_user_without_tokens, subscribe, PROGRAM_PUBKEY,
};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[test]
fn test_renew_success() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    let duration_seconds = 30 * 24 * 60 * 60;
    let trial_days = 0;

    create_plan(&mut svm, &merchant, &mint, plan_id, 1, 1_000_000, duration_seconds, trial_days);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let plan_pda =
        Pubkey::find_program_address(&[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()], &PROGRAM_PUBKEY).0;

    let sub = get_subscription(&svm, &sub_pda);
    assert_eq!(sub.status, SubscriptionStatus::Active);
    let period_end_before = sub.current_period_end;

    let merchant_balance_before = get_token_balance(&svm, &merchant_ata);

    set_clock(&mut svm, period_end_before + 1);

    renew(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let sub = get_subscription(&svm, &sub_pda);
    assert_eq!(sub.status, SubscriptionStatus::Active, "status should still be Active");
    assert_eq!(sub.failed_attempts_count, 0, "failed_attempts_count should be 0");
    assert_eq!(sub.cancel_at_period_end, false, "cancel_at_period_end should be false");
    assert!(sub.current_period_end > period_end_before, "current_period_end should be extended");

    let merchant_balance_after = get_token_balance(&svm, &merchant_ata);
    assert_eq!(merchant_balance_after, merchant_balance_before + 1_000_000, "Merchant should receive payment on renew");

    let plan = get_plan(&svm, &plan_pda);
    assert_eq!(plan.lifetime_revenue, 2_000_000, "lifetime_revenue should be 2x price after subscribe + renew");
    assert_eq!(plan.active_subscribers, 1, "active_subscribers should remain 1");
}

#[test]
fn test_renew_with_no_funds_becomes_pastdue() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup_with_user_without_tokens();

    let plan_id = "test_plan";
    let duration_seconds = 30 * 24 * 60 * 60;
    let trial_days = 7;

    create_plan(&mut svm, &merchant, &mint, plan_id, 1, 1_000_000, duration_seconds, trial_days);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let plan_pda =
        Pubkey::find_program_address(&[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()], &PROGRAM_PUBKEY).0;

    let sub = get_subscription(&svm, &sub_pda);
    assert_eq!(sub.status, SubscriptionStatus::Trialing, "Should be Trialing with trial days");
    let trial_period = trial_days as i64 * 24 * 60 * 60;

    set_clock(&mut svm, trial_period + 1);

    renew(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let sub = get_subscription(&svm, &sub_pda);
    assert_eq!(sub.status, SubscriptionStatus::PastDue, "status should be PastDue when renewal fails");
    assert_eq!(sub.failed_attempts_count, 1, "failed_attempts_count should be 1");

    let plan = get_plan(&svm, &plan_pda);
    assert_eq!(plan.lifetime_revenue, 0, "lifetime_revenue should be 0 - no payment made");
}
