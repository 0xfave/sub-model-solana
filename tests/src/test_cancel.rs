use solana_program_test::tokio;
use solana_sdk::signature::Keypair;
use solana_sdk::pubkey::Pubkey;
use subscription_model::SubscriptionStatus;

#[cfg(test)]
mod test_cancel {
    use super::*;
    use crate::test_util::*;

    #[testtest]
    fn test_cancel_immediate() {
        let (mut program_test, payer, owner) = setup_program();
        let (mut banks_client, _payer, _slot) = program_test.start();

        let user = Keypair::new();
        banks_client.airdrop(&user.pubkey(), 1_000_000_000).await.unwrap();

        let plan_pubkey = create_plan(
            &mut banks_client,
            &payer,
            &owner,
            "cancel_plan",
            1000,
            86400,
            0,
        )
        .await
        .unwrap();

        let mint: Pubkey = USDC_MINT.parse().unwrap();
        let user_token = create_token_accounts(
            &mut banks_client,
            &payer,
            &mint,
            &user,
            1_000_000,
        )
        .await
        .unwrap();

        let merchant_token = create_token_accounts(
            &mut banks_client,
            &payer,
            &mint,
            &owner,
            0,
        )
        .await
        .unwrap();

        let subscription_pubkey = subscribe(
            &mut banks_client,
            &payer,
            &user,
            &plan_pubkey,
            &user_token,
            &merchant_token,
        )
        .await
        .unwrap();

        cancel(
            &mut banks_client,
            &payer,
            &user,
            &plan_pubkey,
            &subscription_pubkey,
            true,
        )
        .await
        .unwrap();

        let subscription = get_subscription(&mut banks_client, &subscription_pubkey).await.unwrap();
        assert_eq!(subscription.status, SubscriptionStatus::Canceled);
    }

    #[testtest]
    fn test_cancel_at_period_end() {
        let (mut program_test, payer, owner) = setup_program();
        let (mut banks_client, _payer, _slot) = program_test.start();

        let user = Keypair::new();
        banks_client.airdrop(&user.pubkey(), 1_000_000_000).await.unwrap();

        let plan_pubkey = create_plan(
            &mut banks_client,
            &payer,
            &owner,
            "cancel_end_plan",
            1000,
            86400,
            0,
        )
        .await
        .unwrap();

        let mint: Pubkey = USDC_MINT.parse().unwrap();
        let user_token = create_token_accounts(
            &mut banks_client,
            &payer,
            &mint,
            &user,
            1_000_000,
        )
        .await
        .unwrap();

        let merchant_token = create_token_accounts(
            &mut banks_client,
            &payer,
            &mint,
            &owner,
            0,
        )
        .await
        .unwrap();

        let subscription_pubkey = subscribe(
            &mut banks_client,
            &payer,
            &user,
            &plan_pubkey,
            &user_token,
            &merchant_token,
        )
        .await
        .unwrap();

        cancel(
            &mut banks_client,
            &payer,
            &user,
            &plan_pubkey,
            &subscription_pubkey,
            false,
        )
        .await
        .unwrap();

        let subscription = get_subscription(&mut banks_client, &subscription_pubkey).await.unwrap();
        assert!(subscription.cancel_at_period_end);
        assert_eq!(subscription.status, SubscriptionStatus::Active);
    }

    #[testtest]
    fn test_cancel_unauthorized() {
        let (mut program_test, payer, owner) = setup_program();
        let (mut banks_client, _payer, _slot) = program_test.start();

        let user = Keypair::new();
        let unauthorized = Keypair::new();
        banks_client.airdrop(&user.pubkey(), 1_000_000_000).await.unwrap();
        banks_client.airdrop(&unauthorized.pubkey(), 1_000_000_000).await.unwrap();

        let plan_pubkey = create_plan(
            &mut banks_client,
            &payer,
            &owner,
            "auth_plan",
            1000,
            86400,
            0,
        )
        .await
        .unwrap();

        let mint: Pubkey = USDC_MINT.parse().unwrap();
        let user_token = create_token_accounts(
            &mut banks_client,
            &payer,
            &mint,
            &user,
            1_000_000,
        )
        .await
        .unwrap();

        let merchant_token = create_token_accounts(
            &mut banks_client,
            &payer,
            &mint,
            &owner,
            0,
        )
        .await
        .unwrap();

        let subscription_pubkey = subscribe(
            &mut banks_client,
            &payer,
            &user,
            &plan_pubkey,
            &user_token,
            &merchant_token,
        )
        .await
        .unwrap();

        let result = cancel(
            &mut banks_client,
            &payer,
            &unauthorized,
            &plan_pubkey,
            &subscription_pubkey,
            true,
        )
        ;

        assert!(result.is_err());
    }

    #[testtest]
    fn test_cancel_already_canceled() {
        let (mut program_test, payer, owner) = setup_program();
        let (mut banks_client, _payer, _slot) = program_test.start();

        let user = Keypair::new();
        banks_client.airdrop(&user.pubkey(), 1_000_000_000).await.unwrap();

        let plan_pubkey = create_plan(
            &mut banks_client,
            &payer,
            &owner,
            "double_cancel_plan",
            1000,
            86400,
            0,
        )
        .await
        .unwrap();

        let mint: Pubkey = USDC_MINT.parse().unwrap();
        let user_token = create_token_accounts(
            &mut banks_client,
            &payer,
            &mint,
            &user,
            1_000_000,
        )
        .await
        .unwrap();

        let merchant_token = create_token_accounts(
            &mut banks_client,
            &payer,
            &mint,
            &owner,
            0,
        )
        .await
        .unwrap();

        let subscription_pubkey = subscribe(
            &mut banks_client,
            &payer,
            &user,
            &plan_pubkey,
            &user_token,
            &merchant_token,
        )
        .await
        .unwrap();

        cancel(
            &mut banks_client,
            &payer,
            &user,
            &plan_pubkey,
            &subscription_pubkey,
            true,
        )
        .await
        .unwrap();

        let result = cancel(
            &mut banks_client,
            &payer,
            &user,
            &plan_pubkey,
            &subscription_pubkey,
            true,
        )
        ;

        assert!(result.is_err());
    }
}
