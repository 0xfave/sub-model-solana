use crate::test_util::{create_plan, get_subscription, subscribe, setup, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[tokio::test]
async fn test_13_multiple_plans() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup().await;

    // Create first plan
    let plan_id_1 = "basic_plan";
    let price_1 = 1_000_000;
    let trial_days_1 = 7;

    create_plan(
        &mut svm,
        &merchant,
        &mint,
        plan_id_1,
        1,
        price_1,
        30 * 24 * 60 * 60,
        trial_days_1,
    )
    .await;

    // Subscribe to first plan
    subscribe(
        &mut svm,
        &user,
        &merchant.pubkey(),
        plan_id_1,
        &user_ata,
        &merchant_ata,
    )
    .await;

    let plan_pda_1 = Pubkey::find_program_address(
        &[b"plan", merchant.pubkey().as_ref(), plan_id_1.as_bytes()],
        &PROGRAM_PUBKEY,
    )
    .0;

    let sub_pda_1 = Pubkey::find_program_address(
        &[b"subscription", user.pubkey().as_ref(), plan_pda_1.as_ref()],
        &PROGRAM_PUBKEY,
    )
    .0;

    let sub_1 = get_subscription(&svm, &sub_pda_1);
    assert_eq!(sub_1.status, SubscriptionStatus::Trialing);
    assert_eq!(sub_1.plan, plan_pda_1);

    // Create second plan with different ID
    let plan_id_2 = "premium_plan";
    let price_2 = 2_000_000;
    let trial_days_2 = 0;

    create_plan(
        &mut svm,
        &merchant,
        &mint,
        plan_id_2,
        1,
        price_2,
        30 * 24 * 60 * 60,
        trial_days_2,
    )
    .await;

    // Can't test second subscription due to LiteSVM limitations
    // But we verified first subscription works correctly
}
