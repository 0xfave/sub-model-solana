use crate::test_util::{create_plan, get_subscription, subscribe, setup, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[tokio::test]
async fn test_19_subscription_timestamp_fields() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup().await;

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
    )
    .await;

    subscribe(
        &mut svm,
        &user,
        &merchant.pubkey(),
        plan_id,
        &user_ata,
        &merchant_ata,
    )
    .await;

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
    
    // start_ts should be set to current period start
    assert_eq!(sub.start_ts, sub.current_period_start);
    
    // Both should be positive (in the future from epoch)
    assert!(sub.start_ts > 0);
    assert!(sub.current_period_start > 0);
    assert!(sub.current_period_end > sub.current_period_start);
}
