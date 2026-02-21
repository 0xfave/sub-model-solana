use litesvm::LiteSVM;
use solana_sdk::{
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
    pubkey::Pubkey,
    instruction::Instruction,
    account::Account,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Mint;
use subscription_model::{self, accounts as subs_accounts, instruction as subs_instruction};

pub async fn setup() -> (LiteSVM, Pubkey, Keypair, Keypair) {
    let mut svm = LiteSVM::new();
    let program_id = Pubkey::from_str("DTdDF7uKkhVp71NjeDo4U4SqSPVsrVxhLgW3f5bADZzs").unwrap();

    // Create mint account
    let mint = Keypair::new();
    let mint_account = Account::new(
        0,
        Mint {
            mint_authority: COption::None,
            supply: 0,
            decimals: 6,
            is_initialized: true,
            freeze_authority: COption::None,
        }
        .pack()
        .len(),
        &spl_token::id(),
    );
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    // Create merchant and user
    let merchant = Keypair::new();
    let user = Keypair::new();

    // Airdrop SOL (just add lamports)
    svm.set_sysvar(&solana_sdk::sysvar::rent::ID, &solana_sdk::sysvar::rent::Rent::default());
    svm.airdrop(&merchant.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();

    // Create ATAs (you'd need to simulate or call the actual create ATA instruction)
    // For simplicity, you can manually add token accounts with correct ownership.

    (svm, mint.pubkey(), merchant, user)
}
