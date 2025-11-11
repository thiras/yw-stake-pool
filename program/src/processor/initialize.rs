use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    log::sol_log_data,
    msg,
    sysvar::{clock::Clock, Sysvar},
};

use crate::assertions::*;
use crate::constants::MAX_REWARD_RATE;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::state::{Key, ProgramAuthority, StakePool};
use crate::utils::create_account;
use solana_program::pubkey::Pubkey;

use super::helpers::{
    validate_current_timestamp, validate_no_freeze_authority, verify_pool_vaults_at_init,
    verify_vault_ownership,
};

/// Minimum lockup period enforced during pool initialization (1 day = 86400 seconds).
///
/// **Security Rationale [H-02]:**
/// Prevents reward vault drain attacks where attackers could:
/// 1. Set lockup to 1 second
/// 2. Stake tokens
/// 3. Wait 1 second
/// 4. Claim full rewards instantly
/// 5. Drain the reward vault
///
/// **Business Rationale:**
/// Ensures meaningful staking commitment and prevents gaming of reward mechanics.
/// Can be adjusted per deployment requirements (7 days, 30 days, etc.)
///
/// **Current Value**: 86400 seconds (1 day)
const MIN_LOCKUP_PERIOD: i64 = 86400;

/// Initialize a new staking pool with the provided parameters.
///
/// # Security
/// This function includes critical security validations:
/// - [H-01] Validates vault token accounts are owned by the pool PDA (not by arbitrary users)
/// - [H-02] Enforces minimum lockup period to prevent reward drain attacks
/// - Prevents attackers from passing malicious token accounts they control
/// - Ensures only the pool program can authorize transfers from vaults
///
/// # Arguments
/// * `accounts` - Accounts required for pool initialization
/// * `pool_id` - Unique identifier for this pool (allows multiple pools per authority+mint)
/// * `reward_rate` - Fixed reward percentage (scaled by 1e9, e.g., 100_000_000 = 10%)
/// * `min_stake_amount` - Minimum amount users must stake
/// * `lockup_period` - Time in seconds before rewards are earned (minimum 1 day)
/// * `enforce_lockup` - Whether to prevent early unstaking
/// * `pool_end_date` - Optional timestamp after which no new stakes allowed
///
/// # Errors
/// Returns error if:
/// - Parameters are invalid (reward rate too high, lockup below minimum, past end date)
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
    if reward_rate > MAX_REWARD_RATE {
        msg!("Reward rate too high: {}", reward_rate);
        return Err(StakePoolError::InvalidParameters.into());
    }

    // [H-02] Security Fix: Enforce minimum lockup period
    // Without this check, admins could set lockup to 1 second, allowing users to:
    // 1. Stake tokens
    // 2. Wait 1 second
    // 3. Claim full rewards instantly
    // 4. Drain the reward vault
    // The minimum lockup ensures rewards are earned over a meaningful timeframe.
    if lockup_period < MIN_LOCKUP_PERIOD {
        msg!(
            "Lockup period too short: {} seconds. Minimum required: {} seconds (1 day)",
            lockup_period,
            MIN_LOCKUP_PERIOD
        );
        return Err(StakePoolError::InvalidParameters.into());
    }

    if let Some(end_date) = pool_end_date {
        let current_time = Clock::get()?.unix_timestamp;
        validate_current_timestamp(current_time)?;
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

    // [Q-01] Security Fix: Validate pool creator is authorized
    // Only addresses in the ProgramAuthority's authorized_creators list can create pools.
    // This prevents spam/scam pools and maintains quality control.
    let program_authority = ProgramAuthority::load(ctx.accounts.program_authority)?;

    if !program_authority.is_authorized(ctx.accounts.authority.key) {
        msg!(
            "Unauthorized pool creator: {}. Only authorized admins can create pools.",
            ctx.accounts.authority.key
        );
        return Err(StakePoolError::UnauthorizedPoolCreator.into());
    }

    // Guards
    // Derive the expected pool PDA from stake_mint and pool_id
    // This ensures the provided pool account matches the pool_id parameter
    let pool_seeds = StakePool::seeds(ctx.accounts.stake_mint.key, pool_id);
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

    // [M-03] Security Fix: Validate mints don't have freeze authority
    // The freeze_authority allows freezing token accounts, which would lock user funds permanently.
    // This validation prevents malicious actors from creating pools with freezable tokens.
    // Without this check, the freeze authority holder can:
    // 1. Freeze any user's stake account after they deposit
    // 2. Prevent unstaking and token transfers
    // 3. Cause permanent loss of user funds
    validate_no_freeze_authority(ctx.accounts.stake_mint, "stake_mint")?;
    validate_no_freeze_authority(ctx.accounts.reward_mint, "reward_mint")?;

    // Verify token accounts have correct mints and validate Token-2022 extensions
    verify_pool_vaults_at_init(
        ctx.accounts.stake_vault,
        ctx.accounts.reward_vault,
        ctx.accounts.stake_mint,
        ctx.accounts.reward_mint,
        ctx.accounts.stake_mint.key,
        ctx.accounts.reward_mint.key,
    )?;

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
        pending_reward_rate: None,
        reward_rate_change_timestamp: None,
        last_rate_change: None,
        _reserved: [0; 7],
    };

    msg!(
        "Pool initialized: pool_id={}, reward_rate={}, lockup_period={}, min_stake_amount={}",
        pool_id,
        reward_rate,
        lockup_period,
        min_stake_amount
    );

    // Save state first to ensure persistence before emitting event
    pool_data.save(ctx.accounts.pool)?;

    // Emit event for off-chain indexing after successful state save
    sol_log_data(&[
        b"InitializePool",
        ctx.accounts.pool.key.as_ref(),
        ctx.accounts.authority.key.as_ref(),
        &pool_id.to_le_bytes(),
        &reward_rate.to_le_bytes(),
    ]);

    Ok(())
}
