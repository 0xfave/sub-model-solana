use crate::test_util::{create_plan, get_plan, get_subscription, setup, subscribe, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[test]
fn test_4_active_subscription_with_trial() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    let price = 1_000_000;
    let duration_seconds = 30 * 24 * 60 * 60;
    let trial_days = 7; // Use trial to work with LiteSVM

    create_plan(&mut svm, &merchant, &mint, plan_id, 1, price, duration_seconds, trial_days);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let plan_pda =
        Pubkey::find_program_address(&[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()], &PROGRAM_PUBKEY).0;

    // With trial, subscription should be Trialing initially
    let sub = get_subscription(&svm, &sub_pda);
    assert_eq!(sub.status, SubscriptionStatus::Trialing, "With trial, subscription should be Trialing");

    // Plan should have 1 active subscriber
    let plan = get_plan(&svm, &plan_pda);
    assert_eq!(plan.active_subscribers, 1, "Plan should have 1 active subscriber");
}
