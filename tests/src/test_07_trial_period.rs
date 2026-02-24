use crate::test_util::{create_plan, get_subscription, setup, subscribe, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_7_trial_period_calculation() {
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

    let sub = get_subscription(&svm, &sub_pda);

    // Trial period should be 7 days in seconds
    let expected_trial_seconds = trial_days as i64 * 24 * 60 * 60;
    let actual_trial_period = sub.current_period_end - sub.current_period_start;

    assert_eq!(
        actual_trial_period, expected_trial_seconds,
        "Trial period should be {} seconds",
        expected_trial_seconds
    );
}
