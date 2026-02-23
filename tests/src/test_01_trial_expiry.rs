use crate::test_util::{
    create_plan, get_plan, get_subscription, process_expired, set_clock, setup, subscribe, PROGRAM_PUBKEY,
};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[test]
fn test_1_trial_expiry_to_past_due() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    let price = 1_000_000; // 1 USDC
    let duration_seconds = 30 * 24 * 60 * 60; // 30 days
    let trial_days = 7; // Use trial to avoid token transfer issue

    create_plan(
        &mut svm,
        &merchant,
        &mint,
        plan_id,
        1, // version
        price,
        duration_seconds,
        trial_days,
    );

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let plan_pda =
        Pubkey::find_program_address(&[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0;
    println!("Plan PDA: {:?}", plan_pda);

    let sub_pda =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()], &PROGRAM_PUBKEY).0;
    println!("Subscription PDA: {:?}", sub_pda);

    let sub_before = get_subscription(&svm, &sub_pda);
    assert_eq!(sub_before.status, SubscriptionStatus::Trialing, "Should start as Trialing");

    // Advance time past trial end (7 days + 1 second)
    let trial_end = sub_before.current_period_end + 1;
    set_clock(&mut svm, trial_end);

    // Process expired
    process_expired(&mut svm, &user, &merchant.pubkey(), plan_id, &user.pubkey());

    let sub_after = get_subscription(&svm, &sub_pda);
    assert_eq!(sub_after.status, SubscriptionStatus::PastDue, "After trial expiry, status should be PastDue");

    let plan = get_plan(&svm, &plan_pda);
    assert_eq!(plan.active_subscribers, 1, "Active subscribers should still be 1 - grace period not expired");
}
