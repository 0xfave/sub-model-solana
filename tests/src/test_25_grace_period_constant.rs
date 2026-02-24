use crate::test_util::{create_plan, get_subscription, setup, subscribe, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_25_grace_period_constant() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    create_plan(&mut svm, &merchant, &mint, plan_id, 1, 1_000_000, 30 * 24 * 60 * 60, 7);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let plan_pda =
        Pubkey::find_program_address(&[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()], &PROGRAM_PUBKEY).0;

    let sub = get_subscription(&svm, &sub_pda);

    let grace_deadline = sub.grace_deadline();
    let expected_grace = sub.current_period_end + (3 * 24 * 60 * 60);
    assert_eq!(grace_deadline, expected_grace);
}
