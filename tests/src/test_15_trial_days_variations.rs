use crate::test_util::{create_plan, get_subscription, subscribe, setup, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_15_trial_days_variations() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    // Test with 14 day trial
    let plan_id = "long_trial";
    create_plan(
        &mut svm,
        &merchant,
        &mint,
        plan_id,
        1,
        1_000_000,
        30 * 24 * 60 * 60,
        14, // 14 day trial
    );

    subscribe(
        &mut svm,
        &user,
        &merchant.pubkey(),
        plan_id,
        &user_ata,
        &merchant_ata,
    );

    let plan_pda = Pubkey::find_program_address(
        &[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()],
        &PROGRAM_PUBKEY,
    )
    .0;

    let sub_pda = Pubkey::find_program_address(
        &[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()],
        &PROGRAM_PUBKEY,
    )
    .0;

    let sub = get_subscription(&svm, &sub_pda);
    
    // Trial period should be 14 days
    let expected_trial = 14 * 24 * 60 * 60;
    let actual_trial = sub.current_period_end - sub.current_period_start;
    assert_eq!(actual_trial, expected_trial);
}
