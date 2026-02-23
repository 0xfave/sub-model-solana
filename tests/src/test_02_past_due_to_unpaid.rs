use crate::test_util::{
    create_plan, get_plan, get_subscription, process_expired, set_clock, setup, subscribe, PROGRAM_PUBKEY,
};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[test]
fn test_2_past_due_to_unpaid() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    let price = 1_000_000;
    let duration_seconds = 30 * 24 * 60 * 60;
    let trial_days = 7;

    create_plan(&mut svm, &merchant, &mint, plan_id, 1, price, duration_seconds, trial_days);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let plan_pda =
        Pubkey::find_program_address(&[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()], &PROGRAM_PUBKEY).0;

    // Advance past trial AND grace period
    let sub = get_subscription(&svm, &sub_pda);
    let trial_end = sub.current_period_end + 1;
    let grace_deadline = 3 * 24 * 60 * 60;
    set_clock(&mut svm, trial_end + grace_deadline);

    // Process expired: should go directly Trialing → Unpaid (grace expired)
    process_expired(&mut svm, &user, &merchant.pubkey(), plan_id, &user.pubkey());

    let sub_after = get_subscription(&svm, &sub_pda);
    assert_eq!(sub_after.status, SubscriptionStatus::Unpaid, "After trial and grace expiry, status should be Unpaid");

    let sub_after_plan = get_plan(&svm, &plan_pda);
    assert_eq!(sub_after_plan.active_subscribers, 0, "Active subscribers should be 0 since grace expired");
}
