// ============================================================================
// Common Test Helpers
// ============================================================================
// Shared utilities for LiteSVM tests

use borsh::BorshDeserialize;
use litesvm::LiteSVM;
use solana_sdk::pubkey::Pubkey;
use your_wallet_stake_pool::state::{StakeAccount, StakePool};

/// Program ID constant
pub const PROGRAM_ID: &str = "Bdm2SF3wrRLmo2t9MyGKydLHAgU5Bhxif8wN9HNMYfSH";

// ============================================================================
// Program Loading
// ============================================================================

/// Load the compiled program binary
/// Tries multiple paths to handle different test execution contexts
pub fn load_program() -> Vec<u8> {
    let paths = [
        "target/sbpf-solana-solana/release/your_wallet_stake_pool.so",
        "../target/sbpf-solana-solana/release/your_wallet_stake_pool.so",
        "../../target/sbpf-solana-solana/release/your_wallet_stake_pool.so",
        "target/deploy/your_wallet_stake_pool.so",
        "../target/deploy/your_wallet_stake_pool.so",
    ];

    for path in &paths {
        if let Ok(data) = std::fs::read(path) {
            return data;
        }
    }

    panic!(
        "Failed to load program. Run 'cargo build-sbf' first.\nTried paths: {:?}",
        paths
    );
}

// ============================================================================
// PDA Derivation
// ============================================================================

/// Derive the pool PDA address
pub fn get_pool_pda(authority: &Pubkey, stake_mint: &Pubkey) -> (Pubkey, u8) {
    let program_id = PROGRAM_ID.parse::<Pubkey>().unwrap();
    Pubkey::find_program_address(
        &[b"stake_pool", authority.as_ref(), stake_mint.as_ref()],
        &program_id,
    )
}

/// Derive the stake account PDA address
pub fn get_stake_account_pda(pool: &Pubkey, owner: &Pubkey, index: u64) -> (Pubkey, u8) {
    let program_id = PROGRAM_ID.parse::<Pubkey>().unwrap();
    Pubkey::find_program_address(
        &[
            b"stake_account",
            pool.as_ref(),
            owner.as_ref(),
            &index.to_le_bytes(),
        ],
        &program_id,
    )
}

/// Derive the stake vault PDA address
pub fn get_stake_vault_pda(pool: &Pubkey) -> (Pubkey, u8) {
    let program_id = PROGRAM_ID.parse::<Pubkey>().unwrap();
    Pubkey::find_program_address(&[b"stake_vault", pool.as_ref()], &program_id)
}

/// Derive the reward vault PDA address
pub fn get_reward_vault_pda(pool: &Pubkey) -> (Pubkey, u8) {
    let program_id = PROGRAM_ID.parse::<Pubkey>().unwrap();
    Pubkey::find_program_address(&[b"reward_vault", pool.as_ref()], &program_id)
}

// ============================================================================
// Account Deserialization
// ============================================================================

/// Load and deserialize a StakePool account
pub fn load_stake_pool(svm: &LiteSVM, pool_address: &Pubkey) -> StakePool {
    let account = svm
        .get_account(pool_address)
        .expect("Pool account should exist");
    StakePool::try_from_slice(&account.data).expect("Failed to deserialize pool")
}

/// Load and deserialize a StakeAccount
pub fn load_stake_account(svm: &LiteSVM, stake_account_address: &Pubkey) -> StakeAccount {
    let account = svm
        .get_account(stake_account_address)
        .expect("Stake account should exist");
    StakeAccount::try_from_slice(&account.data).expect("Failed to deserialize stake account")
}

// ============================================================================
// Assertions
// ============================================================================

/// Assert that a PDA is valid (off-curve)
pub fn assert_valid_pda(pda: &Pubkey) {
    assert!(!pda.is_on_curve(), "PDA should be off-curve");
}

/// Assert that two PDAs are consistent
pub fn assert_pda_consistency(pda1: &Pubkey, pda2: &Pubkey, bump1: u8, bump2: u8) {
    assert_eq!(pda1, pda2, "PDAs should be consistent");
    assert_eq!(bump1, bump2, "Bumps should be consistent");
}
