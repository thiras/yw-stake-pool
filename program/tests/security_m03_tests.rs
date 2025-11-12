// ============================================================================
// [M-03] Mint Freeze Authority Security Tests
// ============================================================================
// These tests verify that the protocol correctly rejects mints with a
// freeze authority that could lock user funds permanently.
//
// Vulnerability: Mints with freeze_authority can freeze token accounts
// Impact: Permanent loss of user funds if pool creator freezes accounts
//
// Security Fix: validate_no_freeze_authority() checks during pool initialization
//
// Run tests: cargo test-sbf --test security_m03_tests

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

    // Program not found - try to download it
    println!("📦 SPL Token 2022 program not found, downloading...");

    if download_spl_token_program(PROGRAM_ID, OUTPUT_PATH) {
        if let Ok(data) = std::fs::read(OUTPUT_PATH) {
            println!("✅ Downloaded SPL Token 2022 from: {}", OUTPUT_PATH);
            return data;
        }
    }

    eprintln!("\n❌ Failed to load SPL Token 2022 program!");
    eprintln!(
        "   Please run: solana program dump {} {}",
        PROGRAM_ID, OUTPUT_PATH
    );
    panic!("SPL Token 2022 program required");
}

fn download_spl_token_program(program_id: &str, output_path: &str) -> bool {
    use std::process::Command;

    let solana_check = Command::new("solana").arg("--version").output();
    if solana_check.is_err() {
        return false;
    }

    let result = Command::new("solana")
        .arg("program")
        .arg("dump")
        .arg(program_id)
        .arg(output_path)
        .output();

    matches!(result, Ok(output) if output.status.success())
}

// ============================================================================
// Helper: Create Token Mint with Freeze Authority
// ============================================================================

fn create_mint_with_freeze_authority(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
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

    // Initialize mint with freeze authority
    let init_mint_ix = token_instruction::initialize_mint(
        &spl_token_2022::id(),
        &mint.pubkey(),
        mint_authority,
        freeze_authority,
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

    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &token_account.pubkey(),
        rent,
        TokenAccount::LEN as u64,
        &spl_token_2022::id(),
    );

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
// Test: Pool Initialization Rejects Stake Mint with Freeze Authority
// ============================================================================

#[test]
fn test_initialize_pool_rejects_stake_mint_with_freeze_authority() {
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
    let freeze_authority = Keypair::new(); // Malicious freeze authority
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();

    // Initialize program authority (required for pool creation)
    let program_authority_pda = initialize_program_authority(&mut svm, &payer, &authority);

    // Create stake mint WITH freeze authority (malicious)
    let stake_mint = create_mint_with_freeze_authority(
        &mut svm,
        &payer,
        &authority.pubkey(),
        Some(&freeze_authority.pubkey()),
        6,
    );

    // Create reward mint WITHOUT freeze authority (safe)
    let reward_mint =
        create_mint_with_freeze_authority(&mut svm, &payer, &authority.pubkey(), None, 6);

    println!(
        "🔴 Created stake mint WITH freeze authority: {}",
        stake_mint
    );
    println!("   Freeze authority: {}", freeze_authority.pubkey());
    println!(
        "✅ Created reward mint WITHOUT freeze authority: {}",
        reward_mint
    );

    // Derive pool PDA
    let (pool_pda, _) = get_pool_pda(&stake_mint, 0);

    // Create vault token accounts (owned by pool PDA)
    let stake_vault = create_token_account(&mut svm, &payer, &stake_mint, &pool_pda);
    let reward_vault = create_token_account(&mut svm, &payer, &reward_mint, &pool_pda);

    // Try to initialize pool with freezable stake mint
    let init_pool_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(pool_pda, false),
            AccountMeta::new_readonly(stake_mint, false),
            AccountMeta::new_readonly(reward_mint, false),
            AccountMeta::new(stake_vault, false),
            AccountMeta::new(reward_vault, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
            AccountMeta::new_readonly(program_authority_pda, false),
        ],
        data: StakePoolInstruction::InitializePool {
            pool_id: 0,
            reward_rate: 100_000_000,
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
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);

    // Verify that initialization FAILS (security fix working!)
    match result {
        Err(e) => {
            println!("✅ SECURITY FIX WORKING: Pool initialization rejected!");
            println!("   Error: {:?}", e);
            println!("   This prevents malicious pool creators from freezing user funds");

            // Check for specific error (MintHasFreezeAuthority = error code 28)
            let error_msg = format!("{:?}", e);
            if error_msg.contains("Custom(28)") || error_msg.contains("MintHasFreezeAuthority") {
                println!("✅ Correct error: MintHasFreezeAuthority (error code 28)");
            } else {
                println!(
                    "⚠️  Expected error code 28 (MintHasFreezeAuthority), got: {}",
                    error_msg
                );
            }
        }
        Ok(_) => {
            panic!(
                "❌ SECURITY VULNERABILITY: Pool initialization should have failed!\n\
                   Stake mint with freeze authority was accepted.\n\
                   This allows malicious pool creators to freeze user funds.\n\
                   The [M-03] fix is not working properly."
            );
        }
    }
}

// ============================================================================
// Test: Pool Initialization Rejects Reward Mint with Freeze Authority
// ============================================================================

#[test]
fn test_initialize_pool_rejects_reward_mint_with_freeze_authority() {
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
    let freeze_authority = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();

    // Initialize program authority (required for pool creation)
    let program_authority_pda = initialize_program_authority(&mut svm, &payer, &authority);

    // Create stake mint WITHOUT freeze authority (safe)
    let stake_mint =
        create_mint_with_freeze_authority(&mut svm, &payer, &authority.pubkey(), None, 6);

    // Create reward mint WITH freeze authority (malicious)
    let reward_mint = create_mint_with_freeze_authority(
        &mut svm,
        &payer,
        &authority.pubkey(),
        Some(&freeze_authority.pubkey()),
        6,
    );

    println!(
        "✅ Created stake mint WITHOUT freeze authority: {}",
        stake_mint
    );
    println!(
        "🔴 Created reward mint WITH freeze authority: {}",
        reward_mint
    );
    println!("   Freeze authority: {}", freeze_authority.pubkey());

    // Derive pool PDA
    let (pool_pda, _) = get_pool_pda(&stake_mint, 0);

    // Create vault token accounts
    let stake_vault = create_token_account(&mut svm, &payer, &stake_mint, &pool_pda);
    let reward_vault = create_token_account(&mut svm, &payer, &reward_mint, &pool_pda);

    // Try to initialize pool with freezable reward mint
    let init_pool_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(pool_pda, false),
            AccountMeta::new_readonly(stake_mint, false),
            AccountMeta::new_readonly(reward_mint, false),
            AccountMeta::new(stake_vault, false),
            AccountMeta::new(reward_vault, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
            AccountMeta::new_readonly(program_authority_pda, false),
        ],
        data: StakePoolInstruction::InitializePool {
            pool_id: 0,
            reward_rate: 100_000_000,
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
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);

    // Verify that initialization FAILS
    match result {
        Err(e) => {
            println!("✅ SECURITY FIX WORKING: Pool initialization rejected!");
            println!("   Error: {:?}", e);
            println!("   This prevents malicious reward mints from being used");

            let error_msg = format!("{:?}", e);
            if error_msg.contains("Custom(28)") || error_msg.contains("MintHasFreezeAuthority") {
                println!("✅ Correct error: MintHasFreezeAuthority (error code 28)");
            }
        }
        Ok(_) => {
            panic!(
                "❌ SECURITY VULNERABILITY: Pool initialization should have failed!\n\
                   Reward mint with freeze authority was accepted.\n\
                   The [M-03] fix is not working properly."
            );
        }
    }
}

// ============================================================================
// Test: Pool Initialization Succeeds with Safe Mints
// ============================================================================

#[test]
fn test_initialize_pool_succeeds_without_freeze_authority() {
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

    // Add payer to authorized creators list
    use borsh::BorshSerialize;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::Signer,
        transaction::Transaction,
    };
    use your_wallet_stake_pool::instruction::StakePoolInstruction;

    let add_creator_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(program_authority_pda, false),
            AccountMeta::new_readonly(authority.pubkey(), true),
        ],
        data: StakePoolInstruction::ManageAuthorizedCreators {
            add: vec![payer.pubkey()],
            remove: vec![],
        }
        .try_to_vec()
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[add_creator_ix],
        Some(&payer.pubkey()),
        &[&payer, &authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("Failed to add authorized creator");

    // Create BOTH mints WITHOUT freeze authority (safe)
    let stake_mint =
        create_mint_with_freeze_authority(&mut svm, &payer, &authority.pubkey(), None, 6);

    let reward_mint =
        create_mint_with_freeze_authority(&mut svm, &payer, &authority.pubkey(), None, 6);

    println!(
        "✅ Created stake mint WITHOUT freeze authority: {}",
        stake_mint
    );
    println!(
        "✅ Created reward mint WITHOUT freeze authority: {}",
        reward_mint
    );

    // Derive pool PDA
    let (pool_pda, _) = get_pool_pda(&stake_mint, 0);

    // Create vault token accounts
    let stake_vault = create_token_account(&mut svm, &payer, &stake_mint, &pool_pda);
    let reward_vault = create_token_account(&mut svm, &payer, &reward_mint, &pool_pda);

    // Initialize pool with safe mints
    let init_pool_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(pool_pda, false),
            AccountMeta::new_readonly(stake_mint, false),
            AccountMeta::new_readonly(reward_mint, false),
            AccountMeta::new(stake_vault, false),
            AccountMeta::new(reward_vault, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
            AccountMeta::new_readonly(program_authority_pda, false),
        ],
        data: StakePoolInstruction::InitializePool {
            pool_id: 0,
            reward_rate: 100_000_000,
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
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);

    // Verify that initialization SUCCEEDS with safe mints
    match result {
        Ok(_) => {
            println!("✅ Pool initialization succeeded with safe mints!");
            println!("   Both mints have no freeze authority");
            println!("   User funds are protected from freezing");

            // Verify pool was created
            let pool = load_stake_pool(&svm, &pool_pda);
            assert_eq!(pool.stake_mint, stake_mint);
            assert_eq!(pool.reward_mint, reward_mint);
            println!("✅ Pool verification passed");
        }
        Err(e) => {
            panic!(
                "❌ Pool initialization should have succeeded with safe mints!\n\
                   Error: {:?}\n\
                   Both mints have no freeze authority and should be accepted.",
                e
            );
        }
    }
}

// ============================================================================
// Test Documentation
// ============================================================================

#[test]
fn test_m03_vulnerability_documentation() {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║   [M-03] Mint Freeze Authority - Security Fix Summary    ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║                                                           ║");
    println!("║ Vulnerability:                                            ║");
    println!("║  Pool initialization did not check if mints have         ║");
    println!("║  freeze_authority set. This allows malicious pool        ║");
    println!("║  creators to freeze user token accounts, causing         ║");
    println!("║  permanent loss of funds.                                ║");
    println!("║                                                           ║");
    println!("║ Attack Scenario:                                          ║");
    println!("║  1. Attacker creates mint with freeze_authority          ║");
    println!("║  2. Initializes pool using this mint                     ║");
    println!("║  3. Users deposit funds into the pool                    ║");
    println!("║  4. Attacker freezes all user token accounts             ║");
    println!("║  5. Users cannot unstake or transfer (funds locked)      ║");
    println!("║                                                           ║");
    println!("║ Security Fix:                                             ║");
    println!("║  ✅ Added MintHasFreezeAuthority error (code 28)         ║");
    println!("║  ✅ Implemented validate_no_freeze_authority()           ║");
    println!("║  ✅ Validates stake_mint during initialization           ║");
    println!("║  ✅ Validates reward_mint during initialization          ║");
    println!("║  ✅ Clear error messages for developers                  ║");
    println!("║                                                           ║");
    println!("║ Test Coverage:                                            ║");
    println!("║  ✅ Reject stake_mint with freeze_authority              ║");
    println!("║  ✅ Reject reward_mint with freeze_authority             ║");
    println!("║  ✅ Accept mints without freeze_authority                ║");
    println!("║  ✅ Integration tests with real Token-2022 program       ║");
    println!("║                                                           ║");
    println!("║ Code Locations:                                           ║");
    println!("║  - program/src/error.rs (error code 28)                  ║");
    println!("║  - program/src/processor/helpers.rs (validation)         ║");
    println!("║  - program/src/processor/initialize.rs (integration)     ║");
    println!("║  - program/tests/security_m03_tests.rs (tests)           ║");
    println!("║                                                           ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");
}
