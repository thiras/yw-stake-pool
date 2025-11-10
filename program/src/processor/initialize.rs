use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    sysvar::{clock::Clock, Sysvar},
};

use crate::assertions::*;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::state::{Key, StakePool};
use crate::utils::create_account;
use solana_program::pubkey::Pubkey;

use super::helpers::{verify_token_account, verify_vault_ownership};

/// Initialize a new staking pool with the provided parameters.
///
/// # Security
/// This function includes critical security validations (see [H-01] fix):
/// - Validates vault token accounts are owned by the pool PDA (not by arbitrary users)
/// - Prevents attackers from passing malicious token accounts they control
/// - Ensures only the pool program can authorize transfers from vaults
///
/// # Arguments
/// * `accounts` - Accounts required for pool initialization
/// * `pool_id` - Unique identifier for this pool (allows multiple pools per authority+mint)
/// * `reward_rate` - Fixed reward percentage (scaled by 1e9, e.g., 10_000_000_000 = 10%)
/// * `min_stake_amount` - Minimum amount users must stake
/// * `lockup_period` - Time in seconds before rewards are earned
/// * `enforce_lockup` - Whether to prevent early unstaking
/// * `pool_end_date` - Optional timestamp after which no new stakes allowed
///
/// # Errors
/// Returns error if:
/// - Parameters are invalid (reward rate too high, negative lockup, past end date)
/// - Pool account doesn't match expected PDA derivation
/// - Required signers are missing
/// - Vault accounts are not owned by the pool PDA (CRITICAL SECURITY CHECK)
/// - Account creation fails
pub fn initialize_pool<'a>(
    accounts: &'a [AccountInfo<'a>],
    pool_id: u64,
    reward_rate: u64,
    min_stake_amount: u64,
    lockup_period: i64,
    enforce_lockup: bool,
    pool_end_date: Option<i64>,
) -> ProgramResult {
    // Validate parameters
    if reward_rate > 1_000_000_000_000 {
        // > 1000% reward rate seems unreasonable
        msg!("Reward rate too high: {}", reward_rate);
        return Err(StakePoolError::InvalidParameters.into());
    }

    if lockup_period < 0 {
        msg!("Lockup period cannot be negative: {}", lockup_period);
        return Err(StakePoolError::InvalidParameters.into());
    }

    if let Some(end_date) = pool_end_date {
        let current_time = Clock::get()?.unix_timestamp;
        if end_date <= current_time {
            msg!(
                "Pool end date must be in the future. Current: {}, End date: {}",
                current_time,
                end_date
            );
            return Err(StakePoolError::InvalidParameters.into());
        }
    }

    // Use ShankContext to parse accounts
    let ctx = InitializePoolAccounts::context(accounts)?;

    // Guards
    // Derive the expected pool PDA from authority, stake_mint, and pool_id
    // This ensures the provided pool account matches the pool_id parameter
    let pool_seeds = StakePool::seeds(
        ctx.accounts.authority.key,
        ctx.accounts.stake_mint.key,
        pool_id,
    );
    let pool_seeds_refs: Vec<&[u8]> = pool_seeds.iter().map(|s| s.as_slice()).collect();
    let (pool_key, bump) = Pubkey::find_program_address(&pool_seeds_refs, &crate::ID);

    // Validate that the provided pool address matches the expected PDA
    // This prevents initialization with wrong pool_id
    assert_same_pubkeys("pool", ctx.accounts.pool, &pool_key)?;
    assert_signer("authority", ctx.accounts.authority)?;
    assert_signer("payer", ctx.accounts.payer)?;
    assert_empty("pool", ctx.accounts.pool)?;
    assert_writable("pool", ctx.accounts.pool)?;
    assert_writable("stake_vault", ctx.accounts.stake_vault)?;
    assert_writable("reward_vault", ctx.accounts.reward_vault)?;
    assert_writable("payer", ctx.accounts.payer)?;

    // Verify token accounts have correct mints
    verify_token_account(ctx.accounts.stake_vault, ctx.accounts.stake_mint.key)?;
    verify_token_account(ctx.accounts.reward_vault, ctx.accounts.reward_mint.key)?;

    // CRITICAL SECURITY FIX [H-01]: Verify vault ownership
    // This prevents an attacker from passing token accounts they control as pool vaults.
    // Without this check, an attacker could:
    // 1. Create token accounts they own
    // 2. Pass them as stake_vault and reward_vault during pool initialization
    // 3. Steal all user deposits since funds would be sent to attacker's accounts
    // 4. Lock the system permanently since vault addresses cannot be changed
    //
    // The pool PDA must own both vaults to ensure:
    // - Only the pool program can authorize transfers from vaults
    // - User funds are protected by program logic
    // - No external parties can drain the vaults
    verify_vault_ownership(ctx.accounts.stake_vault, &pool_key, "stake_vault")?;
    verify_vault_ownership(ctx.accounts.reward_vault, &pool_key, "reward_vault")?;

    // Create pool account
    let mut seeds_with_bump = pool_seeds.clone();
    seeds_with_bump.push(vec![bump]);
    let seeds_refs: Vec<&[u8]> = seeds_with_bump.iter().map(|s| s.as_slice()).collect();

    create_account(
        ctx.accounts.pool,
        ctx.accounts.payer,
        ctx.accounts.system_program,
        StakePool::LEN,
        &crate::ID,
        Some(&[&seeds_refs]),
    )?;

    // Initialize pool
    let pool_data = StakePool {
        key: Key::StakePool,
        authority: *ctx.accounts.authority.key,
        stake_mint: *ctx.accounts.stake_mint.key,
        reward_mint: *ctx.accounts.reward_mint.key,
        pool_id,
        stake_vault: *ctx.accounts.stake_vault.key,
        reward_vault: *ctx.accounts.reward_vault.key,
        total_staked: 0,
        total_rewards_owed: 0,
        reward_rate,
        min_stake_amount,
        lockup_period,
        is_paused: false,
        enforce_lockup,
        bump,
        pending_authority: None,
        pool_end_date,
        _reserved: [0; 32],
    };

    pool_data.save(ctx.accounts.pool)
}
