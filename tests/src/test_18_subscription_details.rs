use crate::test_util::{create_plan, get_subscription, subscribe, setup, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_18_plan_version() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    create_plan(
        &mut svm,
        &merchant,
        &mint,
        plan_id,
        1,
        1_000_000,
        30 * 24 * 60 * 60,
        7,
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
    
    // The subscription should reference the plan
    assert_eq!(sub.plan, plan_pda);
}
