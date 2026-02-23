use crate::test_util::{create_plan, get_plan, setup, PROGRAM_PUBKEY};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[tokio::test]
async fn test_8_plan_fields() {
    let (mut svm, mint, merchant, _user, _merchant_ata, _user_ata) = setup().await;

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

    let plan = get_plan(&svm, &plan_pda);
    
    assert_eq!(plan.owner, merchant.pubkey(), "Plan owner should match");
    assert_eq!(plan.price, price, "Plan price should match");
    assert_eq!(plan.duration_seconds, duration_seconds, "Plan duration should match");
    assert_eq!(plan.trial_days, trial_days, "Plan trial_days should match");
    assert_eq!(plan.token_mint, mint, "Plan mint should match");
    assert_eq!(plan.active_subscribers, 0, "Initial active subscribers should be 0");
    assert_eq!(plan.lifetime_revenue, 0, "Initial lifetime revenue should be 0");
}
