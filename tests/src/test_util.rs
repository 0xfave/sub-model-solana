use anchor_lang::{AccountDeserialize, InstructionData};
use anchor_spl::token::TokenAccount;
use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};
use solana_clock::Clock;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_system_program::id as SYSTEM_PROGRAM_ID;
use solana_transaction::Transaction;
use subscription_model::{instruction as subs_instruction, Subscription};

pub const PROGRAM_ID: &str = "6PyMsXWBKo77maWZir1kpE8i71Kuwprgm5hR9e5Ng2r3";
pub const PROGRAM_PUBKEY: Pubkey = Pubkey::from_str_const(PROGRAM_ID);
pub const TOKEN_PROGRAM_ID: Pubkey = Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

pub fn setup() -> (LiteSVM, Pubkey, Keypair, Keypair, Pubkey, Pubkey) {
    let mut svm = LiteSVM::new();

    // Load the compiled program
    let program_bytes = include_bytes!("../../target/deploy/subscription_model.so");
    svm.add_program(PROGRAM_PUBKEY, program_bytes);

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let mint = CreateMint::new(&mut svm, &payer).decimals(6).send().unwrap();

    let merchant = Keypair::new();
    let user = Keypair::new();

    svm.airdrop(&merchant.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();

    let merchant_ata =
        CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint).owner(&merchant.pubkey()).send().unwrap();
    let user_ata = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint).owner(&user.pubkey()).send().unwrap();

    MintTo::new(&mut svm, &payer, &mint, &user_ata, 1_000_000_000).send().unwrap();

    (svm, mint, merchant, user, merchant_ata, user_ata)
}

pub fn setup_with_user_without_tokens() -> (LiteSVM, Pubkey, Keypair, Keypair, Pubkey, Pubkey) {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../target/deploy/subscription_model.so");
    svm.add_program(PROGRAM_PUBKEY, program_bytes);

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let mint = CreateMint::new(&mut svm, &payer).decimals(6).send().unwrap();

    let merchant = Keypair::new();
    let user = Keypair::new();

    svm.airdrop(&merchant.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();

    let merchant_ata =
        CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint).owner(&merchant.pubkey()).send().unwrap();
    let user_ata = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint).owner(&user.pubkey()).send().unwrap();

    // Don't mint any tokens to user - they have 0 balance

    (svm, mint, merchant, user, merchant_ata, user_ata)
}

fn derive_plan_address(owner: &Pubkey, plan_id: &str) -> Pubkey {
    Pubkey::find_program_address(&[b"plan", owner.as_ref(), plan_id.as_bytes()], &PROGRAM_PUBKEY).0
}

fn derive_subscription_address(user: &Pubkey, plan: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"subscription", user.as_ref(), plan.as_ref()], &PROGRAM_PUBKEY).0
}

pub fn create_plan(
    svm: &mut LiteSVM,
    owner: &Keypair,
    mint: &Pubkey,
    plan_id: &str,
    version: u16,
    price: u64,
    duration_seconds: u64,
    trial_days: u64,
) -> Pubkey {
    let plan_pda = derive_plan_address(&owner.pubkey(), plan_id);

    let data = subs_instruction::CreatePlan {
        plan_id: plan_id.to_string(),
        version,
        price,
        duration_seconds,
        trial_days,
        token_mint: *mint,
    };

    // Manually create AccountMeta to ensure proper signer flags
    let accounts = vec![
        AccountMeta::new(plan_pda, false),               // plan - init (will be signed via CPI)
        AccountMeta::new_readonly(*mint, false),         // token_mint_account
        AccountMeta::new_readonly(owner.pubkey(), true), // owner - signer
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID(), false), // system_program
    ];

    let instruction = Instruction { program_id: PROGRAM_PUBKEY, accounts, data: data.data() };

    println!("Creating plan with mint: {:?}", mint);
    println!("Owner pubkey: {:?}", owner.pubkey());
    println!("Plan PDA: {:?}", plan_pda);

    // Use new_with_payer which adds payer as signer automatically
    let mut tx = Transaction::new_with_payer(&[instruction], Some(&owner.pubkey()));
    tx.sign(&[owner], svm.latest_blockhash());
    let result = svm.send_transaction(tx);
    println!("Create plan result: {:?}", result);
    result.unwrap();

    plan_pda
}

pub fn subscribe(
    svm: &mut LiteSVM,
    user: &Keypair,
    plan_owner: &Pubkey,
    plan_id: &str,
    user_token_account: &Pubkey,
    merchant_token_account: &Pubkey,
) -> Pubkey {
    let plan_pda = derive_plan_address(plan_owner, plan_id);
    let sub_pda = derive_subscription_address(&user.pubkey(), &plan_pda);

    let accounts = vec![
        AccountMeta::new(user.pubkey(), true),                 // user - signer, mut (payer)
        AccountMeta::new(plan_pda, false),                     // plan - writable (Subscribe modifies it)
        AccountMeta::new(sub_pda, false),                      // subscription - writable for init
        AccountMeta::new(*user_token_account, false),          // user_token_account - writable for transfer
        AccountMeta::new(*merchant_token_account, false),      // merchant_token_account - writable for transfer
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),    // token_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID(), false), // system_program
    ];

    let instruction = Instruction { program_id: PROGRAM_PUBKEY, accounts, data: subs_instruction::Subscribe {}.data() };

    // Use new_with_payer for init_if_needed transactions
    let mut tx = Transaction::new_with_payer(&[instruction], Some(&user.pubkey()));
    tx.sign(&[user], svm.latest_blockhash());
    let result = svm.send_transaction(tx);
    println!("Subscribe result: {:?}", result);
    result.unwrap();

    sub_pda
}

pub fn process_expired(svm: &mut LiteSVM, payer: &Keypair, plan_owner: &Pubkey, plan_id: &str, user: &Pubkey) {
    let plan_pda = derive_plan_address(plan_owner, plan_id);
    let sub_pda = derive_subscription_address(user, &plan_pda);

    // For LiteSVM bug workaround: split into two transactions if needed
    // But first try with subscription only mut (we know this fails due to Anchor constraint)
    // Then try with plan only mut

    // First try: subscription only mutable (will fail with ConstraintMut)
    let accounts = vec![
        AccountMeta::new(plan_pda, false), // plan - readonly (will fail)
        AccountMeta::new(sub_pda, false),  // subscription - mut workaround
    ];

    let instruction =
        Instruction { program_id: PROGRAM_PUBKEY, accounts, data: subs_instruction::ProcessExpired {}.data() };

    let tx = Transaction::new_signed_with_payer(
        &[instruction.clone()],
        Some(&payer.pubkey()),
        &[payer],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    println!("Process expired result: {:?}", result);

    // Handle AlreadyProcessed error - retry with fresh blockhash
    if let Err(e) = &result {
        let err_str = format!("{:?}", e);
        if err_str.contains("AlreadyProcessed") {
            println!("Retrying with fresh blockhash...");
            let tx2 = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&payer.pubkey()),
                &[payer],
                svm.latest_blockhash(),
            );
            svm.send_transaction(tx2).unwrap();
            return;
        }
    }

    result.unwrap();
}

pub fn get_subscription(svm: &LiteSVM, sub_pda: &Pubkey) -> subscription_model::Subscription {
    let account_data = svm.get_account(sub_pda).expect("Subscription account should exist");

    // Try full data first
    let result = Subscription::try_deserialize(&mut account_data.data.as_ref());
    // subscription_model::Subscription::try_deserialize(&account_data.data);

    match result {
        Ok(sub) => {
            println!("Result {:?}", &sub);
            return sub;
        }
        Err(e) => {
            panic!("Full data deserialization error: {:?}", e);
        }
    }
}

pub fn get_plan(svm: &LiteSVM, plan_pda: &Pubkey) -> subscription_model::Plan {
    let account_data = svm.get_account(plan_pda).expect("Plan account should exist");

    if let Ok(plan) = subscription_model::Plan::try_deserialize(&mut account_data.data.as_ref()) {
        println!("Plan Result {:?}", &plan);
        return plan;
    } else {
        panic!("Full data deserialization error");
    }
}

pub fn get_token_balance(svm: &LiteSVM, token_account: &Pubkey) -> u64 {
    let account_data = svm.get_account(token_account).expect("Token account should exist");
    if let Ok(ata) = TokenAccount::try_deserialize(&mut account_data.data.as_ref()) {
        return ata.amount;
    }
    panic!("Failed to deserialize token account");
}

pub fn set_clock(svm: &mut LiteSVM, unix_timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_timestamp;
    clock.slot = clock.slot.saturating_add(1);
    svm.set_sysvar(&clock);
}

pub fn cancel(svm: &mut LiteSVM, user: &Keypair, plan_owner: &Pubkey, plan_id: &str, immediate: bool) {
    let plan_pda = derive_plan_address(plan_owner, plan_id);
    let sub_pda = derive_subscription_address(&user.pubkey(), &plan_pda);

    let accounts = vec![
        AccountMeta::new(user.pubkey(), true),
        AccountMeta::new(plan_pda, false),
        AccountMeta::new(sub_pda, false),
    ];

    let data = subs_instruction::Cancel { immediate };
    let instruction = Instruction { program_id: PROGRAM_PUBKEY, accounts, data: data.data() };

    let tx = Transaction::new_signed_with_payer(&[instruction], Some(&user.pubkey()), &[user], svm.latest_blockhash());
    let result = svm.send_transaction(tx);
    println!("Cancel result: {:?}", result);
    result.unwrap();
}

pub fn cancel_with_result(
    svm: &mut LiteSVM,
    user: &Keypair,
    plan_owner: &Pubkey,
    plan_id: &str,
    immediate: bool,
) -> Result<(), ()> {
    let plan_pda = derive_plan_address(plan_owner, plan_id);
    let sub_pda = derive_subscription_address(&user.pubkey(), &plan_pda);

    let accounts = vec![
        AccountMeta::new(user.pubkey(), true),
        AccountMeta::new(plan_pda, false),
        AccountMeta::new(sub_pda, false),
    ];

    let data = subs_instruction::Cancel { immediate };
    let instruction = Instruction { program_id: PROGRAM_PUBKEY, accounts, data: data.data() };

    let tx = Transaction::new_signed_with_payer(&[instruction], Some(&user.pubkey()), &[user], svm.latest_blockhash());
    let result = svm.send_transaction(tx);
    println!("Cancel result: {:?}", result);
    if result.is_ok() {
        Ok(())
    } else {
        Err(())
    }
}

pub fn renew(
    svm: &mut LiteSVM,
    user: &Keypair,
    plan_owner: &Pubkey,
    plan_id: &str,
    user_token_account: &Pubkey,
    merchant_token_account: &Pubkey,
) {
    let plan_pda = derive_plan_address(plan_owner, plan_id);
    let sub_pda = derive_subscription_address(&user.pubkey(), &plan_pda);

    let accounts = vec![
        AccountMeta::new(user.pubkey(), true),
        AccountMeta::new(plan_pda, false),
        AccountMeta::new(sub_pda, false),
        AccountMeta::new(*user_token_account, false),
        AccountMeta::new(*merchant_token_account, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID(), false),
    ];

    let instruction = Instruction { program_id: PROGRAM_PUBKEY, accounts, data: subs_instruction::Renew {}.data() };

    let tx = Transaction::new_signed_with_payer(
        &[instruction.clone()],
        Some(&user.pubkey()),
        &[user],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    println!("Renew result: {:?}", result);

    if let Err(e) = &result {
        let err_str = format!("{:?}", e);
        if err_str.contains("AlreadyProcessed") {
            println!("Retrying with fresh blockhash...");
            let mut retries = 3;
            while retries > 0 {
                let tx2 = Transaction::new_signed_with_payer(
                    &[instruction.clone()],
                    Some(&user.pubkey()),
                    &[user],
                    svm.latest_blockhash(),
                );
                let result2 = svm.send_transaction(tx2);
                if let Ok(_) = result2 {
                    return;
                }
                retries -= 1;
            }
        }
    }
    result.unwrap();
}

pub fn mint_tokens(svm: &mut LiteSVM, mint: &Pubkey, to: &Pubkey, amount: u64) {
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    MintTo::new(svm, &payer, mint, to, amount).send().unwrap();
}
