use crate::test_util::{create_plan, get_plan, get_subscription, subscribe, setup, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[tokio::test]
async fn test_4_active_subscription_no_trial() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup().await;

    let plan_id = "test_plan";
    let price = 1_000_000;
    let duration_seconds = 30 * 24 * 60 * 60;
    let trial_days = 0; // No trial

    create_plan(
        &mut svm,
        &merchant,
        &mint,
        plan_id,
        1,
        price,
        duration_seconds,
        trial_days,
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

    // Without trial, subscription should be Active immediately
    let sub = get_subscription(&svm, &sub_pda);
    assert_eq!(
        sub.status,
        SubscriptionStatus::Active,
        "Without trial, subscription should be Active"
    );

    // Plan should have 1 active subscriber
    let plan = get_plan(&svm, &plan_pda);
    assert_eq!(plan.active_subscribers, 1, "Plan should have 1 active subscriber");
    assert_eq!(plan.lifetime_revenue, price, "Lifetime revenue should equal price");
}
