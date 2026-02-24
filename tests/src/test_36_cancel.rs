use crate::test_util::{cancel, cancel_with_result, create_plan, get_subscription, setup, subscribe, PROGRAM_PUBKEY};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[test]
fn test_cancel_at_period_end() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    let trial_days = 7;

    create_plan(&mut svm, &merchant, &mint, plan_id, 1, 1_000_000, 30 * 24 * 60 * 60, trial_days);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let plan_pda =
        Pubkey::find_program_address(&[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()], &PROGRAM_PUBKEY).0;

    let sub = get_subscription(&svm, &sub_pda);
    assert_eq!(sub.status, SubscriptionStatus::Trialing);
    assert_eq!(sub.cancel_at_period_end, false);

    cancel(&mut svm, &user, &merchant.pubkey(), plan_id, false);

    let sub = get_subscription(&svm, &sub_pda);
    assert_eq!(sub.cancel_at_period_end, true, "cancel_at_period_end should be true after cancel");
    assert_eq!(sub.status, SubscriptionStatus::Trialing, "status should still be Trialing");
}

#[test]
fn test_cancel_immediate() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    let trial_days = 7;

    create_plan(&mut svm, &merchant, &mint, plan_id, 1, 1_000_000, 30 * 24 * 60 * 60, trial_days);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let plan_pda =
        Pubkey::find_program_address(&[b"plan", merchant.pubkey().as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda.as_ref()], &PROGRAM_PUBKEY).0;

    let sub = get_subscription(&svm, &sub_pda);
    assert_eq!(sub.status, SubscriptionStatus::Trialing);

    cancel(&mut svm, &user, &merchant.pubkey(), plan_id, true);

    let sub = get_subscription(&svm, &sub_pda);
    assert_eq!(sub.status, SubscriptionStatus::Canceled, "status should be Canceled after immediate cancel");
}

#[test]
fn test_cancel_unauthorized() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), 1_000_000_000).unwrap();

    let plan_id = "test_plan";
    let trial_days = 7;

    create_plan(&mut svm, &merchant, &mint, plan_id, 1, 1_000_000, 30 * 24 * 60 * 60, trial_days);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    let result = cancel_with_result(&mut svm, &unauthorized, &merchant.pubkey(), plan_id, true);

    assert!(result.is_err(), "Cancel should fail for unauthorized user");
}

#[test]
fn test_cancel_already_canceled() {
    let (mut svm, mint, merchant, user, merchant_ata, user_ata) = setup();

    let plan_id = "test_plan";
    let trial_days = 7;

    create_plan(&mut svm, &merchant, &mint, plan_id, 1, 1_000_000, 30 * 24 * 60 * 60, trial_days);

    subscribe(&mut svm, &user, &merchant.pubkey(), plan_id, &user_ata, &merchant_ata);

    cancel(&mut svm, &user, &merchant.pubkey(), plan_id, true);

    let result = cancel_with_result(&mut svm, &user, &merchant.pubkey(), plan_id, true);

    assert!(result.is_err(), "Cancel should fail for already canceled subscription");
}
