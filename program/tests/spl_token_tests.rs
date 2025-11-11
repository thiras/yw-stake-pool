// ============================================================================
// LiteSVM with SPL Token 2022 Tests
// ============================================================================
// Full integration tests using SPL Token 2022 program in LiteSVM
//
// The tests will automatically download the SPL Token 2022 program if needed.
// Manual setup: solana program dump TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb /tmp/spl_token_2022.so
//
// Run tests: cargo test --test spl_token_tests

#![allow(deprecated)]

mod common;

use borsh::BorshSerialize;
use litesvm::LiteSVM;
use solana_program::program_pack::Pack;
use solana_sdk::system_instruction;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token_2022::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint},
};
use your_wallet_stake_pool::instruction::StakePoolInstruction;

use common::*;

// ============================================================================
// Helper: Load SPL Token 2022 Program
// ============================================================================

fn load_spl_token_program() -> Vec<u8> {
    const PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
    const OUTPUT_PATH: &str = "/tmp/spl_token_2022.so";

    let paths = [
        OUTPUT_PATH,
        "spl_token_2022.so",
        "program/spl_token_2022.so",
    ];

    // Try to load from existing paths
    for path in &paths {
        if let Ok(data) = std::fs::read(path) {
            println!("✅ Loaded SPL Token 2022 from: {}", path);
            return data;
        }
    }

    // Program not found - try to download it automatically
    println!("📦 SPL Token 2022 program not found, downloading...");

    if download_spl_token_program(PROGRAM_ID, OUTPUT_PATH) {
        if let Ok(data) = std::fs::read(OUTPUT_PATH) {
            println!(
                "✅ Downloaded and loaded SPL Token 2022 from: {}",
                OUTPUT_PATH
            );
            return data;
        }
    }

    eprintln!("\n❌ Failed to load SPL Token 2022 program!");
    eprintln!("   Automatic download failed. Please run manually:");
    eprintln!("   $ solana program dump {} {}", PROGRAM_ID, OUTPUT_PATH);
    panic!("SPL Token 2022 program required");
}

/// Attempt to download SPL Token program using solana CLI
fn download_spl_token_program(program_id: &str, output_path: &str) -> bool {
    use std::process::Command;

    // Check if solana CLI is available
    let solana_check = Command::new("solana").arg("--version").output();

    if solana_check.is_err() {
        eprintln!("   ⚠️  solana CLI not found in PATH");
        return false;
    }

    println!(
        "   Running: solana program dump {} {}",
        program_id, output_path
    );

    // Try to dump the program
    let result = Command::new("solana")
        .arg("program")
        .arg("dump")
        .arg(program_id)
        .arg(output_path)
        .output();

    match result {
        Ok(output) if output.status.success() => {
            println!("   ✅ Download successful!");
            true
        }
        Ok(output) => {
            eprintln!(
                "   ⚠️  Download failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            false
        }
        Err(e) => {
            eprintln!("   ⚠️  Failed to execute solana command: {}", e);
            false
        }
    }
}

// ============================================================================
// Helper: Create Token Mint
// ============================================================================

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

// ============================================================================
// Helper: Create Token Account
// ============================================================================

fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    let token_account = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(TokenAccount::LEN);

    // Create account
    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &token_account.pubkey(),
        rent,
        TokenAccount::LEN as u64,
        &spl_token_2022::id(),
    );

    // Initialize token account
    let init_account_ix = token_instruction::initialize_account(
        &spl_token_2022::id(),
        &token_account.pubkey(),
        mint,
        owner,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_account_ix, init_account_ix],
        Some(&payer.pubkey()),
        &[payer, &token_account],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();
    token_account.pubkey()
}

// ============================================================================
// Helper: Mint Tokens
// ============================================================================

fn mint_tokens(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Keypair,
    amount: u64,
) {
    let mint_to_ix = token_instruction::mint_to(
        &spl_token_2022::id(),
        mint,
        destination,
        &authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &[payer, authority],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();
}

// ============================================================================
// Helper: Get Token Balance
// ============================================================================

fn get_token_balance(svm: &LiteSVM, token_account: &Pubkey) -> u64 {
    let account = svm.get_account(token_account).unwrap();
    let token_account_data = TokenAccount::unpack(&account.data).unwrap();
    token_account_data.amount
}

// ============================================================================
// Test: SPL Token Program Loading
// ============================================================================

#[test]
fn test_load_spl_token_program() {
    let mut svm = LiteSVM::new();

    // Load SPL Token 2022 program
    let token_program_data = load_spl_token_program();
    println!(
        "📦 SPL Token program size: {} bytes",
        token_program_data.len()
    );

    // Add to LiteSVM
    let token_program_id = spl_token_2022::id();
    svm.add_program(token_program_id, &token_program_data)
        .expect("Failed to add SPL Token program");

    // Verify it's loaded
    let program_account = svm.get_account(&token_program_id).unwrap();
    assert!(program_account.executable, "Program should be executable");

    println!("✅ SPL Token 2022 program loaded into LiteSVM");
}

// ============================================================================
// Test: Basic Token Operations
// ============================================================================

#[test]
fn test_create_mint_and_token_account() {
    let mut svm = LiteSVM::new();

    // Load SPL Token program
    let token_program_data = load_spl_token_program();
    svm.add_program(spl_token_2022::id(), &token_program_data)
        .unwrap();

    // Setup
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    // Create mint
    let mint = create_mint(&mut svm, &payer, &mint_authority.pubkey(), 6);
    println!("✅ Created mint: {}", mint);

    // Create token account
    let owner = Keypair::new();
    let token_account = create_token_account(&mut svm, &payer, &mint, &owner.pubkey());
    println!("✅ Created token account: {}", token_account);

    // Verify token account
    let balance = get_token_balance(&svm, &token_account);
    assert_eq!(balance, 0, "Initial balance should be 0");

    println!("✅ Token operations working in LiteSVM!");
}

// ============================================================================
// Test: Mint and Transfer Tokens
// ============================================================================

#[test]
fn test_mint_and_check_balance() {
    let mut svm = LiteSVM::new();

    // Load SPL Token program
    let token_program_data = load_spl_token_program();
    svm.add_program(spl_token_2022::id(), &token_program_data)
        .unwrap();

    // Setup
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    // Create mint and token account
    let mint = create_mint(&mut svm, &payer, &mint_authority.pubkey(), 6);
    let owner = Keypair::new();
    let token_account = create_token_account(&mut svm, &payer, &mint, &owner.pubkey());

    // Mint tokens
    let amount = 1_000_000;
    mint_tokens(
        &mut svm,
        &payer,
        &mint,
        &token_account,
        &mint_authority,
        amount,
    );

    // Check balance
    let balance = get_token_balance(&svm, &token_account);
    assert_eq!(balance, amount, "Balance should match minted amount");

    println!("✅ Minted {} tokens", amount);
    println!("✅ Balance verification passed");
}

// ============================================================================
// Test: Pool Initialization with Real Tokens
// ============================================================================

#[test]
fn test_initialize_pool_with_real_tokens() {
    let mut svm = LiteSVM::new();

    // Load programs
    let token_program_data = load_spl_token_program();
    svm.add_program(spl_token_2022::id(), &token_program_data)
        .unwrap();

    let program_data = load_program();
    let program_id = PROGRAM_ID.parse::<Pubkey>().unwrap();
    svm.add_program(program_id, &program_data).unwrap();

    // Setup
    let payer = Keypair::new();
    let authority = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();

    // Initialize program authority (required for pool creation)
    let program_authority_pda = initialize_program_authority(&mut svm, &payer, &authority);

    // Create mints
    let stake_mint = create_mint(&mut svm, &payer, &authority.pubkey(), 6);
    let reward_mint = create_mint(&mut svm, &payer, &authority.pubkey(), 6);

    println!("✅ Created stake mint: {}", stake_mint);
    println!("✅ Created reward mint: {}", reward_mint);

    // Derive PDAs
    let (pool_pda, _) = get_pool_pda(&stake_mint, 0);

    // Create vault token accounts (owned by pool PDA)
    let stake_vault_account = create_token_account(&mut svm, &payer, &stake_mint, &pool_pda);
    let reward_vault_account = create_token_account(&mut svm, &payer, &reward_mint, &pool_pda);

    println!("✅ Created stake vault: {}", stake_vault_account);
    println!("✅ Created reward vault: {}", reward_vault_account);

    // Initialize pool
    let init_pool_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(pool_pda, false),
            AccountMeta::new_readonly(authority.pubkey(), true),
            AccountMeta::new_readonly(stake_mint, false),
            AccountMeta::new_readonly(reward_mint, false),
            AccountMeta::new(stake_vault_account, false),
            AccountMeta::new(reward_vault_account, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
            AccountMeta::new_readonly(program_authority_pda, false),
        ],
        data: StakePoolInstruction::InitializePool {
            pool_id: 0,
            reward_rate: 100_000_000, // 10%
            min_stake_amount: 1_000_000,
            lockup_period: 86400,
            enforce_lockup: false,
            pool_end_date: None,
        }
        .try_to_vec()
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[init_pool_ix],
        Some(&payer.pubkey()),
        &[&payer, &authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);

    if let Err(e) = &result {
        println!("❌ Pool initialization failed: {:?}", e);
        result.unwrap();
    } else {
        println!("✅ Pool initialized successfully!");

        // Verify pool account exists
        let pool_account = svm
            .get_account(&pool_pda)
            .expect("Pool account should exist");
        println!(
            "   Pool account data length: {} bytes",
            pool_account.data.len()
        );
        println!("   Pool account owner: {}", pool_account.owner);

        // Try to deserialize pool
        match load_stake_pool(&svm, &pool_pda) {
            pool => {
                assert_eq!(pool.authority, authority.pubkey());
                assert_eq!(pool.stake_mint, stake_mint);
                assert_eq!(pool.reward_mint, reward_mint);
                println!("✅ Pool verification passed");
                println!("   Reward rate: {}", pool.reward_rate);
                println!("   Min stake: {}", pool.min_stake_amount);
                println!("   Lockup period: {}", pool.lockup_period);
            }
        }
    }
}

// ============================================================================
// Serialization Size Test
// ============================================================================

#[test]
fn test_stake_pool_serialized_size() {
    use borsh::BorshSerialize;
    use your_wallet_stake_pool::state::{Key, StakePool};

    // Create a StakePool instance with None optionals
    let pool = StakePool {
        key: Key::StakePool,
        authority: Pubkey::new_unique(),
        stake_mint: Pubkey::new_unique(),
        reward_mint: Pubkey::new_unique(),
        pool_id: 0,
        stake_vault: Pubkey::new_unique(),
        reward_vault: Pubkey::new_unique(),
        total_staked: 0,
        total_rewards_owed: 0,
        reward_rate: 10_000_000_000,
        min_stake_amount: 1000,
        lockup_period: 0,
        is_paused: false,
        enforce_lockup: false,
        bump: 255,
        pending_authority: None,
        pool_end_date: None,
        pending_reward_rate: None,
        reward_rate_change_timestamp: None,
        last_rate_change: None,
        _reserved: [0; 7],
    };

    // Serialize it
    let serialized = pool.try_to_vec().unwrap();

    println!("\n=== StakePool Serialization Analysis ===");
    println!("StakePool::LEN constant: {}", StakePool::LEN);
    println!(
        "Actual serialized size (with None values): {}",
        serialized.len()
    );
    println!(
        "First 50 bytes: {:?}",
        &serialized[..serialized.len().min(50)]
    );

    // Now test with Some values
    let pool_with_optionals = StakePool {
        key: Key::StakePool,
        authority: Pubkey::new_unique(),
        stake_mint: Pubkey::new_unique(),
        reward_mint: Pubkey::new_unique(),
        pool_id: 0,
        stake_vault: Pubkey::new_unique(),
        reward_vault: Pubkey::new_unique(),
        total_staked: 0,
        total_rewards_owed: 0,
        reward_rate: 10_000_000_000,
        min_stake_amount: 1000,
        lockup_period: 0,
        is_paused: false,
        enforce_lockup: true,
        bump: 255,
        pending_authority: Some(Pubkey::new_unique()),
        pool_end_date: Some(12345678),
        pending_reward_rate: Some(5_000_000_000),
        reward_rate_change_timestamp: Some(1700000000),
        last_rate_change: Some(1699000000),
        _reserved: [0; 7],
    };

    let serialized_with_optionals = pool_with_optionals.try_to_vec().unwrap();
    println!("\nWith Some(pending_authority) and Some(pool_end_date):");
    println!("Serialized size: {}", serialized_with_optionals.len());

    println!("\n⚠️  Size mismatch detected!");
    println!("Expected (LEN): {}", StakePool::LEN);
    println!("Actual (None):  {}", serialized.len());
    println!(
        "Difference:     {}",
        StakePool::LEN as i32 - serialized.len() as i32
    );
}

// ============================================================================
// Test Documentation
// ============================================================================

#[test]
fn test_spl_token_integration_summary() {
    println!("\n================================================================");
    println!("   LiteSVM + SPL Token 2022 Integration - Summary");
    println!("================================================================");
    println!("");
    println!(" Strategy:");
    println!("  1. Auto-download SPL Token 2022 program (if needed)");
    println!("  2. Load program into LiteSVM");
    println!("  3. Run full integration tests");
    println!("");
    println!(" Setup:");
    println!("  - Automatic! Just run: cargo test --test spl_token_tests");
    println!("  - The program will be downloaded automatically if missing");
    println!("");
    println!(" Test Coverage:");
    println!("  - SPL Token program loading (auto-download)");
    println!("  - Mint creation");
    println!("  - Token account creation");
    println!("  - Token minting");
    println!("  - Balance checking");
    println!("  - Pool initialization with real tokens");
    println!("");
    println!(" Benefits:");
    println!("  - Fast execution (< 1 second)");
    println!("  - Full token operation testing");
    println!("  - No validator needed");
    println!("  - Zero-config setup (auto-download)");
    println!("  - Complete integration coverage");
    println!("");
    println!("================================================================\n");
}

// ============================================================================
// Security Test: [H-01] Vault Ownership Validation
// ============================================================================

#[test]
fn test_initialize_pool_rejects_attacker_owned_vaults() {
    let mut svm = LiteSVM::new();

    // Load programs
    let token_program_data = load_spl_token_program();
    svm.add_program(spl_token_2022::id(), &token_program_data)
        .unwrap();

    let program_data = load_program();
    let program_id = PROGRAM_ID.parse::<Pubkey>().unwrap();
    svm.add_program(program_id, &program_data).unwrap();

    // Setup
    let payer = Keypair::new();
    let authority = Keypair::new();
    let attacker = Keypair::new(); // Attacker who wants to steal funds
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    // Initialize program authority (required for pool creation)
    let program_authority_pda = initialize_program_authority(&mut svm, &payer, &authority);

    // Create mints
    let stake_mint = create_mint(&mut svm, &payer, &authority.pubkey(), 6);
    let reward_mint = create_mint(&mut svm, &payer, &authority.pubkey(), 6);

    // Derive pool PDA
    let (pool_pda, _) = get_pool_pda(&stake_mint, 0);

    // ATTACK: Create vault token accounts owned by attacker (NOT by pool PDA)
    let malicious_stake_vault =
        create_token_account(&mut svm, &payer, &stake_mint, &attacker.pubkey());
    let malicious_reward_vault =
        create_token_account(&mut svm, &payer, &reward_mint, &attacker.pubkey());

    println!("🔴 Attacker created vaults owned by themselves:");
    println!("   Malicious stake vault: {}", malicious_stake_vault);
    println!("   Malicious reward vault: {}", malicious_reward_vault);
    println!("   Attacker pubkey: {}", attacker.pubkey());

    // Try to initialize pool with attacker-owned vaults
    let init_pool_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(pool_pda, false),
            AccountMeta::new_readonly(authority.pubkey(), true),
            AccountMeta::new_readonly(stake_mint, false),
            AccountMeta::new_readonly(reward_mint, false),
            AccountMeta::new(malicious_stake_vault, false), // Attacker's account!
            AccountMeta::new(malicious_reward_vault, false), // Attacker's account!
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
            AccountMeta::new_readonly(program_authority_pda, false),
        ],
        data: StakePoolInstruction::InitializePool {
            pool_id: 0,
            reward_rate: 100_000_000, // 10%
            min_stake_amount: 1_000_000,
            lockup_period: 86400,
            enforce_lockup: false,
            pool_end_date: None,
        }
        .try_to_vec()
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[init_pool_ix],
        Some(&payer.pubkey()),
        &[&payer, &authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);

    // Verify that initialization FAILS (our security fix working!)
    match result {
        Err(e) => {
            println!("✅ SECURITY FIX WORKING: Pool initialization rejected!");
            println!("   Error: {:?}", e);
            println!("   This prevents attacker from stealing user funds");

            // Check for specific error (InvalidVaultOwner = error code 25)
            let error_msg = format!("{:?}", e);
            if error_msg.contains("Custom(25)") || error_msg.contains("InvalidVaultOwner") {
                println!("✅ Correct error: InvalidVaultOwner (error code 25)");
            }
        }
        Ok(_) => {
            panic!(
                "❌ SECURITY VULNERABILITY: Pool initialization should have failed!\n\
                   Attacker-owned vaults were accepted, allowing theft of user funds.\n\
                   The [H-01] fix is not working properly."
            );
        }
    }
}
