use crate::test_util::{create_plan, get_plan, get_subscription, process_expired, set_clock, subscribe, setup, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[tokio::test]
async fn test_5_trial_expiry_active_subscribers() {
    // This test verifies that active_subscribers is properly maintained during trial expiry
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup().await;

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
    )
    .await;

    let plan_pda = Pubkey::find_program_address(
        &[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()],
        &PROGRAM_PUBKEY,
    )
    .0;

    // Subscribe with trial
    subscribe(
        &mut svm,
        &user,
        &merchant.pubkey(),
        plan_id,
        &user_ata,
        &merchant_ata,
    )
    .await;

    let sub_pda = Pubkey::find_program_address(
        &[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()],
        &PROGRAM_PUBKEY,
    )
    .0;

    // Check initial state - should be Trialing with active_subscribers = 1
    let sub = get_subscription(&svm, &sub_pda);
    assert_eq!(sub.status, SubscriptionStatus::Trialing);
    
    let plan = get_plan(&svm, &plan_pda);
    assert_eq!(plan.active_subscribers, 1, "Active subscribers should be 1 during trial");

    // Advance past trial and process expired
    let trial_end = sub.current_period_end + 1;
    set_clock(&mut svm, trial_end);
    process_expired(&mut svm, &user, &merchant.pubkey(), plan_id, &user.pubkey()).await;

    // After trial expiry to PastDue, active_subscribers should still be 1
    let plan_after = get_plan(&svm, &plan_pda);
    assert_eq!(plan_after.active_subscribers, 1, "Active subscribers should remain 1 after trial → PastDue");
}
