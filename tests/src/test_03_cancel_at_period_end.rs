use crate::test_util::{
    cancel, create_plan, get_subscription, setup, subscribe, PROGRAM_PUBKEY,
};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[test]
fn test_3_cancel_at_period_end() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    let price = 1_000_000;
    let duration_seconds = 30 * 24 * 60 * 60;
    let trial_days = 7;

    create_plan(&mut svm, &merchant, &mint, plan_id, 1, price, duration_seconds, trial_days);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let plan_pda =
        Pubkey::find_program_address(&[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()], &PROGRAM_PUBKEY).0;

    let sub_before = get_subscription(&svm, &sub_pda);
    assert_eq!(sub_before.status, SubscriptionStatus::Trialing, "Should start as Trialing");
    assert_eq!(sub_before.cancel_at_period_end, false, "cancel_at_period_end should be false initially");

    cancel(&mut svm, &user, &merchant.pubkey(), plan_id, false);

    let sub_after = get_subscription(&svm, &sub_pda);
    assert_eq!(sub_after.cancel_at_period_end, true, "cancel_at_period_end should be true after cancel");
}
