use crate::test_util::{create_plan, setup, subscribe, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_30_subscription_creation() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    create_plan(&mut svm, &merchant, &mint, plan_id, 1, 1_000_000, 30 * 24 * 60 * 60, 7);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let plan_pda =
        Pubkey::find_program_address(&[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()], &PROGRAM_PUBKEY).0;

    // Verify subscription was created
    let account = svm.get_account(&sub_pda);
    assert!(account.is_some(), "Subscription account should exist");
}
