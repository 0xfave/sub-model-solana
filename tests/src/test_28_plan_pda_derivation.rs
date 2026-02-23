use crate::test_util::{create_plan, get_plan, setup, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_28_plan_pda_derivation() {
    let (mut svm, mint, merchant, _user, _merchant_ata, _user_ata) = setup();

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

    let expected_pda = Pubkey::find_program_address(
        &[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()],
        &PROGRAM_PUBKEY,
    )
    .0;

    let plan = get_plan(&svm, &expected_pda);
    
    // If we get here, PDA derivation works
    assert_eq!(plan.owner, merchant.pubkey());
}