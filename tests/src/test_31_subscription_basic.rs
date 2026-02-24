use crate::test_util::{create_plan, get_subscription, setup, subscribe, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_31_subscription_has_valid_timestamps() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    let trial_days = 7;
    create_plan(&mut svm, &merchant, &mint, plan_id, 1, 1_000_000, 30 * 24 * 60 * 60, trial_days);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let plan_pda =
        Pubkey::find_program_address(&[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()], &PROGRAM_PUBKEY).0;

    let account = svm.get_account(&sub_pda);
    assert!(account.is_some(), "Subscription account should exist");

    let sub = get_subscription(&svm, &sub_pda);
    let trial_period = trial_days as i64 * 24 * 60 * 60;
    assert_eq!(sub.current_period_end, trial_period, "current_period_end should equal trial period");
}
