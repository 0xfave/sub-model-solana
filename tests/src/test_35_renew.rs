use crate::test_util::{create_plan, get_subscription, set_clock, setup, subscribe, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[test]
fn test_renew_eligibility() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

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
    assert_eq!(sub.status, SubscriptionStatus::Trialing);
    assert_eq!(sub.cancel_at_period_end, false);

    let trial_period = trial_days as i64 * 24 * 60 * 60;
    assert_eq!(sub.current_period_end, trial_period);

    let sub = get_subscription(&svm, &sub_pda);
    assert!(!sub.eligible_for_renewal(0), "Should not be eligible for renewal before trial ends");

    set_clock(&mut svm, trial_period);

    let sub = get_subscription(&svm, &sub_pda);
    assert!(sub.eligible_for_renewal(trial_period), "Should be eligible for renewal when trial ends");

    set_clock(&mut svm, trial_period + 1);

    let sub = get_subscription(&svm, &sub_pda);
    assert!(sub.eligible_for_renewal(trial_period + 1), "Should be eligible for renewal after trial expires");
}
