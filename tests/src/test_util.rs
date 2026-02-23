use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::str::FromStr;
use subscription_model::{self, accounts as subs_accounts, instruction as subs_instruction};

pub async fn setup() -> (LiteSVM, Pubkey, Keypair, Keypair) {
    let mut svm = LiteSVM::new();
    let _program_id = Pubkey::from_str("DTdDF7uKkhVp71NjeDo4U4SqSPVsrVxhLgW3f5bADZzs").unwrap();

    let payer = Keypair::new();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let mint = CreateMint::new(&mut svm, &payer)
        .decimals(6)
        .send()
        .unwrap();

    let merchant = Keypair::new();
    let user = Keypair::new();

    svm.airdrop(&merchant.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();

    let _merchant_ata = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint)
        .owner(&merchant.pubkey())
        .send()
        .unwrap();
    let _user_ata = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint)
        .owner(&user.pubkey())
        .send()
        .unwrap();

    (svm, mint, merchant, user)
}
