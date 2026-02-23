use crate::test_util::{create_plan, get_plan, get_subscription, process_expired, set_clock, subscribe, setup, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[tokio::test]
async fn test_2_past_due_to_unpaid() {
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

    // Advance past trial end
    let sub = get_subscription(&svm, &sub_pda);
    let trial_end = sub.current_period_end + 1;
    set_clock(&mut svm, trial_end);

    // First process_expired: Trialing → PastDue
    process_expired(&mut svm, &user, &merchant.pubkey(), plan_id, &user.pubkey()).await;

    let sub_after_trial = get_subscription(&svm, &sub_pda);
    assert_eq!(
        sub_after_trial.status,
        SubscriptionStatus::PastDue,
        "After trial expiry, status should be PastDue"
    );

    // Verify grace deadline calculation
    let grace_deadline = sub_after_trial.grace_deadline();
    println!("Grace deadline: {}", grace_deadline);

    // Advance past grace deadline
    let past_grace = grace_deadline + 1;
    set_clock(&mut svm, past_grace);

    // Second process_expired: PastDue → Unpaid
    process_expired(&mut svm, &user, &merchant.pubkey(), plan_id, &user.pubkey()).await;

    let sub_after_grace = get_subscription(&svm, &sub_pda);
    assert_eq!(
        sub_after_grace.status,
        SubscriptionStatus::Unpaid,
        "After grace period expires, status should be Unpaid"
    );

    // Active subscribers should be decremented
    let plan = get_plan(&svm, &plan_pda);
    assert_eq!(plan.active_subscribers, 0, "Active subscribers should be 0 after Unpaid");
}
