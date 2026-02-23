use crate::test_util::{create_plan, get_subscription, subscribe, setup, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_10_grace_deadline() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    let price = 1_000_000;
    let duration_seconds = 30 * 24 * 60 * 60;
    let trial_days = 7;

    create_plan(
        &mut svm,
        &merchant,
        &mint,
        plan_id,
        1,
        price,
        duration_seconds,
        trial_days,
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
    
    let grace_deadline = sub.grace_deadline();
    let expected_grace = sub.current_period_end + (3 * 24 * 60 * 60); // 3 days grace
    
    assert_eq!(grace_deadline, expected_grace, "Grace deadline should be period end + 3 days");
}
