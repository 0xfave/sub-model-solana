use crate::test_util::{create_plan, get_plan, get_subscription, setup, subscribe, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use subscription_model::SubscriptionStatus;

#[test]
fn test_13_multiple_plans() {
    let (mut svm, mint, merchant1, user, merchant1_ata, user_ata) = setup();

    let merchant2 = solana_keypair::Keypair::new();
    svm.airdrop(&merchant2.pubkey(), 1_000_000_000).unwrap();
    let merchant2_ata = litesvm_token::CreateAssociatedTokenAccount::new(&mut svm, &user, &mint)
        .owner(&merchant2.pubkey())
        .send()
        .unwrap();

    let plan_id_1 = "basic_plan";
    let trial_days_1 = 7;

    create_plan(&mut svm, &merchant1, &mint, plan_id_1, 1, 1_000_000, 30 * 24 * 60 * 60, trial_days_1);

    subscribe(&mut svm, &user, &merchant1.pubkey(), plan_id_1, &user_ata, &merchant1_ata);

    let plan_pda_1 =
        Pubkey::find_program_address(&[b"plan", merchant1.pubkey().as_ref(), plan_id_1.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda_1 =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda_1.as_ref()], &PROGRAM_PUBKEY)
            .0;

    let sub_1 = get_subscription(&svm, &sub_pda_1);
    assert_eq!(sub_1.status, SubscriptionStatus::Trialing);
    assert_eq!(sub_1.plan, plan_pda_1);

    let plan_id_2 = "premium_plan";
    let trial_days_2 = 7;

    create_plan(&mut svm, &merchant2, &mint, plan_id_2, 1, 2_000_000, 30 * 24 * 60 * 60, trial_days_2);

    subscribe(&mut svm, &user, &merchant2.pubkey(), plan_id_2, &user_ata, &merchant2_ata);

    let plan_pda_2 =
        Pubkey::find_program_address(&[b"plan", merchant2.pubkey().as_ref(), plan_id_2.as_bytes()], &PROGRAM_PUBKEY).0;

    let sub_pda_2 =
        Pubkey::find_program_address(&[b"subscription", user.pubkey().as_ref(), plan_pda_2.as_ref()], &PROGRAM_PUBKEY)
            .0;

    let sub_2 = get_subscription(&svm, &sub_pda_2);
    assert_eq!(sub_2.status, SubscriptionStatus::Trialing);
    assert_eq!(sub_2.plan, plan_pda_2);

    let plan_1 = get_plan(&svm, &plan_pda_1);
    let plan_2 = get_plan(&svm, &plan_pda_2);
    assert_eq!(plan_1.active_subscribers, 1);
    assert_eq!(plan_2.active_subscribers, 1);
}
