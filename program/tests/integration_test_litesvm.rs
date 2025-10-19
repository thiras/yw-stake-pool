// LiteSVM 0.7.x Integration Tests
//
// These tests use LiteSVM 0.7.1 which is compatible with Solana SDK 2.x
// For comprehensive testing documentation, see LITESVM_TESTS.md

use litesvm::LiteSVM;
use solana_program::program_pack::Pack;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};
use spl_token_2022::{instruction as token_instruction, state::Mint};
use your_wallet_stake_pool::instruction::StakePoolInstruction;

// Program ID from declare_id in lib.rs
const PROGRAM_ID: &str = "Bdm2SF3wrRLmo2t9MyGKydLHAgU5Bhxif8wN9HNMYfSH";

/// Helper to load compiled program
fn load_program() -> Vec<u8> {
    // When running from workspace root
    let workspace_path = std::env::current_dir()
        .unwrap()
        .join("target/deploy/your_wallet_stake_pool.so");

    // When running from program directory
    let program_path = std::env::current_dir()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/deploy/your_wallet_stake_pool.so");

    // Try workspace path first, then program path
    if workspace_path.exists() {
        std::fs::read(workspace_path).expect("Failed to read program binary")
    } else if program_path.exists() {
        std::fs::read(program_path).expect("Failed to read program binary")
    } else {
        panic!(
            "Failed to load program binary. Run 'cargo build-sbf' first. \
                Tried:\n  - {}\n  - {}",
            workspace_path.display(),
            program_path.display()
        )
    }
}

/// Helper to create a token mint
fn create_mint(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_authority: &Pubkey,
    decimals: u8,
) -> Pubkey {
    let mint = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(Mint::LEN);

    // Create mint account
    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        Mint::LEN as u64,
        &spl_token_2022::id(),
    );

    // Initialize mint
    let init_mint_ix = token_instruction::initialize_mint(
        &spl_token_2022::id(),
        &mint.pubkey(),
        mint_authority,
        None,
        decimals,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_account_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &[payer, &mint],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();
    mint.pubkey()
}

/// Helper to derive pool PDA
fn get_pool_pda(authority: &Pubkey, stake_mint: &Pubkey) -> (Pubkey, u8) {
    let program_id = PROGRAM_ID.parse::<Pubkey>().unwrap();
    Pubkey::find_program_address(
        &[b"stake_pool", authority.as_ref(), stake_mint.as_ref()],
        &program_id,
    )
}

#[test]
fn test_litesvm_basic_airdrop() {
    // Basic test to verify LiteSVM 0.7 works with our Solana SDK 2.x setup
    let mut svm = LiteSVM::new();
    let keypair = Keypair::new();

    svm.airdrop(&keypair.pubkey(), 1_000_000_000).unwrap();

    let account = svm.get_account(&keypair.pubkey()).unwrap();
    assert_eq!(account.lamports, 1_000_000_000);
}

#[test]
fn test_initialize_pool() {
    let mut svm = LiteSVM::new();
    let program_id = PROGRAM_ID.parse::<Pubkey>().unwrap();

    // Load and add program
    let program_data = load_program();
    svm.add_program(program_id, &program_data).unwrap();

    let payer = Keypair::new();
    let authority = Keypair::new();

    // Airdrop SOL
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    // Create mints
    let stake_mint = create_mint(&mut svm, &payer, &authority.pubkey(), 6);
    let reward_mint = create_mint(&mut svm, &payer, &authority.pubkey(), 6);

    // Derive PDAs
    let (pool_pda, _) = get_pool_pda(&authority.pubkey(), &stake_mint);
    let (stake_vault, _) =
        Pubkey::find_program_address(&[b"stake_vault", pool_pda.as_ref()], &program_id);
    let (reward_vault, _) =
        Pubkey::find_program_address(&[b"reward_vault", pool_pda.as_ref()], &program_id);

    // Build InitializePool instruction
    let init_pool_data = StakePoolInstruction::InitializePool {
        reward_rate: 100_000_000, // 10%
        min_stake_amount: 1_000_000,
        lockup_period: 86400, // 1 day
        pool_end_date: None,
    };

    let init_pool_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(pool_pda, false),
            AccountMeta::new_readonly(authority.pubkey(), true),
            AccountMeta::new_readonly(stake_mint, false),
            AccountMeta::new_readonly(reward_mint, false),
            AccountMeta::new(stake_vault, false),
            AccountMeta::new(reward_vault, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
        ],
        data: borsh::to_vec(&init_pool_data).unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[init_pool_ix],
        Some(&payer.pubkey()),
        &[&payer, &authority],
        svm.latest_blockhash(),
    );

    // Send transaction
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "InitializePool failed: {:?}", result.err());

    // Verify pool was created
    let pool_account = svm
        .get_account(&pool_pda)
        .expect("Pool account should exist");
    assert!(pool_account.data.len() > 0, "Pool should have data");
}
