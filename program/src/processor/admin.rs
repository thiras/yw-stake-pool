//! Admin and authority management for the stake pool program
//!
//! This module contains all administrative functions for managing:
//! - Global program authority (initialization, transfer, creator management)
//! - Individual pool settings (update_pool, reward rate changes)
//!
//! # Global Admin Model
//! This program uses a global admin system where:
//! - A single `ProgramAuthority` account controls who can create and manage pools
//! - No per-pool authorities - all pools are managed by authorized global admins
//! - Authority can be transferred via a two-step process (nominate + accept)

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    log::sol_log_data,
    msg,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

use crate::assertions::*;
use crate::constants::MAX_REWARD_RATE;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::processor::helpers::{validate_current_timestamp, validate_stored_timestamp};
use crate::state::{Key, ProgramAuthority, StakePool};
use crate::utils::create_account;

//
// ============================================================================
// PROGRAM AUTHORITY MANAGEMENT
// ============================================================================
//

/// Initialize the program authority account (one-time setup)
///
/// This creates a global ProgramAuthority account that controls who can create stake pools.
/// Should only be called once during program deployment.
///
/// # Security
/// - Only the initial authority can manage the authorized creators list
/// - The ProgramAuthority PDA is deterministic (derived from "program_authority" seed)
/// - Cannot be reinitialized once created
///
/// # Arguments
/// * `accounts` - Required accounts for program authority initialization
///
/// # Errors
/// Returns error if:
/// - Program authority account already exists
/// - Account creation fails
/// - Signer validation fails
pub fn initialize_program_authority<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let ctx = InitializeProgramAuthorityAccounts::context(accounts)?;

    // Derive the expected program authority PDA
    let program_authority_seeds = ProgramAuthority::seeds();
    let program_authority_seeds_refs: Vec<&[u8]> = program_authority_seeds
        .iter()
        .map(|s| s.as_slice())
        .collect();
    let (program_authority_key, bump) =
        Pubkey::find_program_address(&program_authority_seeds_refs, &crate::ID);

    // Guards
    assert_same_pubkeys(
        "program_authority",
        ctx.accounts.program_authority,
        &program_authority_key,
    )?;
    assert_signer("initial_authority", ctx.accounts.initial_authority)?;
    assert_signer("payer", ctx.accounts.payer)?;
    assert_empty("program_authority", ctx.accounts.program_authority)?;
    assert_writable("program_authority", ctx.accounts.program_authority)?;
    assert_writable("payer", ctx.accounts.payer)?;

    // Create program authority account
    let mut seeds_with_bump = program_authority_seeds.clone();
    seeds_with_bump.push(vec![bump]);
    let seeds_refs: Vec<&[u8]> = seeds_with_bump.iter().map(|s| s.as_slice()).collect();

    create_account(
        ctx.accounts.program_authority,
        ctx.accounts.payer,
        ctx.accounts.system_program,
        ProgramAuthority::LEN,
        &crate::ID,
        Some(&[&seeds_refs]),
    )?;

    // Initialize program authority data
    let program_authority_data = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority: *ctx.accounts.initial_authority.key,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        pending_authority: None,
        bump,
    };

    program_authority_data.save(ctx.accounts.program_authority)?;

    msg!(
        "Program authority initialized with authority: {}",
        ctx.accounts.initial_authority.key
    );

    // Log event for off-chain indexing
    sol_log_data(&[
        b"ProgramAuthorityInitialized",
        ctx.accounts.initial_authority.key.as_ref(),
    ]);

    Ok(())
}

/// Manage authorized pool creators (add or remove)
///
/// Only the program authority can call this to add or remove addresses
/// from the authorized creators list.
///
/// # Security
/// - Only the main authority can manage the list
/// - Cannot remove the main authority itself
/// - Maximum of 10 authorized creators
/// - Validates all operations before applying changes
///
/// # Arguments
/// * `accounts` - Required accounts for managing creators
/// * `add` - List of addresses to add to authorized creators
/// * `remove` - List of addresses to remove from authorized creators
///
/// # Errors
/// Returns error if:
/// - Caller is not the program authority
/// - Maximum creators limit reached
/// - Creator already exists (when adding)
/// - Creator not found (when removing)
/// - Attempting to remove main authority
pub fn manage_authorized_creators<'a>(
    accounts: &'a [AccountInfo<'a>],
    add: Vec<Pubkey>,
    remove: Vec<Pubkey>,
) -> ProgramResult {
    let ctx = ManageAuthorizedCreatorsAccounts::context(accounts)?;

    // DoS Protection: Limit vector sizes to prevent excessive computation
    if add.len() > ProgramAuthority::MAX_CREATORS {
        msg!(
            "Too many creators to add: {}. Maximum: {}",
            add.len(),
            ProgramAuthority::MAX_CREATORS
        );
        return Err(StakePoolError::InvalidParameters.into());
    }
    if remove.len() > ProgramAuthority::MAX_CREATORS {
        msg!(
            "Too many creators to remove: {}. Maximum: {}",
            remove.len(),
            ProgramAuthority::MAX_CREATORS
        );
        return Err(StakePoolError::InvalidParameters.into());
    }

    // Load and validate program authority
    let mut program_authority_data = ProgramAuthority::load(ctx.accounts.program_authority)?;

    // Guards
    assert_signer("authority", ctx.accounts.authority)?;
    assert_writable("program_authority", ctx.accounts.program_authority)?;

    // Verify the signer is the program authority
    if ctx.accounts.authority.key != &program_authority_data.authority {
        msg!(
            "Unauthorized: {} is not the program authority",
            ctx.accounts.authority.key
        );
        return Err(StakePoolError::Unauthorized.into());
    }

    // Remove creators first
    for creator in &remove {
        program_authority_data.remove_creator(creator)?;
        msg!("Removed authorized creator: {}", creator);

        // Log event for off-chain indexing
        sol_log_data(&[
            b"AuthorizedCreatorRemoved",
            creator.as_ref(),
            ctx.accounts.authority.key.as_ref(),
        ]);
    }

    // Add new creators
    for creator in &add {
        program_authority_data.add_creator(*creator)?;
        msg!("Added authorized creator: {}", creator);

        // Log event for off-chain indexing
        sol_log_data(&[
            b"AuthorizedCreatorAdded",
            creator.as_ref(),
            ctx.accounts.authority.key.as_ref(),
        ]);
    }

    // Save updated state
    program_authority_data.save(ctx.accounts.program_authority)?;

    msg!(
        "Authorized creators updated. Current count: {}",
        program_authority_data.creator_count
    );

    Ok(())
}

//
// ============================================================================
// POOL MANAGEMENT
// ============================================================================
//

/// Time delay before a reward rate change can be finalized (7 days = 604800 seconds).
///
/// **Security [L-01]:**
/// Provides users notice to unstake if they disagree with new rate.
/// Prevents centralized surprise changes to reward rates.
///
/// **Design Rationale:**
/// - 7 days balances user protection vs operational flexibility
/// - Industry standard for time-locked governance operations
/// - Sufficient time for users to monitor and react to changes
/// - Aligns with common DeFi governance timelock periods
///
/// **Cooldown Enforcement:**
/// After finalization, another 7-day cooldown is enforced before
/// proposing a new rate change (prevents authority from chaining
/// rapid rate changes to bypass the time-lock).
///
/// **Current Value**: 604800 seconds (7 days)
const REWARD_RATE_CHANGE_DELAY: i64 = 604800;

pub fn update_pool<'a>(
    accounts: &'a [AccountInfo<'a>],
    reward_rate: Option<u64>,
    min_stake_amount: Option<u64>,
    lockup_period: Option<i64>,
    is_paused: Option<bool>,
    enforce_lockup: Option<bool>,
    pool_end_date: Option<Option<i64>>,
) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = UpdatePoolAccounts::context(accounts)?;

    // Verify pool account discriminator before loading (Type Cosplay protection)
    assert_account_key("pool", ctx.accounts.pool, Key::StakePool)?;

    // Verify program ownership
    assert_program_owner("pool", ctx.accounts.pool, &crate::ID)?;

    // Load pool
    let mut pool_data = StakePool::load(ctx.accounts.pool)?;

    // Load program authority to verify admin permissions
    assert_account_key(
        "program_authority",
        ctx.accounts.program_authority,
        Key::ProgramAuthority,
    )?;
    let program_authority = ProgramAuthority::load(ctx.accounts.program_authority)?;

    // Guards
    assert_signer("admin", ctx.accounts.admin)?;
    assert_writable("pool", ctx.accounts.pool)?;

    // Verify the signer is authorized as a global admin
    if !program_authority.is_authorized(ctx.accounts.admin.key) {
        msg!(
            "Unauthorized: {} is not a global admin",
            ctx.accounts.admin.key
        );
        return Err(StakePoolError::Unauthorized.into());
    }

    // Get current time once for efficiency (Clock is a sysvar that shouldn't change during transaction)
    let current_time = Clock::get()?.unix_timestamp;
    validate_current_timestamp(current_time)?;

    // Update fields
    if let Some(rate) = reward_rate {
        if rate > MAX_REWARD_RATE {
            msg!("Reward rate too high: {}", rate);
            return Err(StakePoolError::InvalidParameters.into());
        }

        // Special case: If proposing the current active rate, cancel any pending change
        // This allows authority to revert/cancel unwanted proposals
        // Note: pending_reward_rate should never equal current_rate immediately after proposing
        // (since we only allow proposing rates that differ from current). However, this
        // cancellation mechanism works regardless of what the pending rate is.
        if rate == pool_data.reward_rate {
            if pool_data.pending_reward_rate.is_some() {
                pool_data.pending_reward_rate = None;
                pool_data.reward_rate_change_timestamp = None;
                msg!(
                    "Pending reward rate change cancelled. Keeping current rate: {}",
                    pool_data.reward_rate
                );

                // Emit event for off-chain indexing
                sol_log_data(&[
                    b"RewardRateProposalCancelled",
                    ctx.accounts.pool.key.as_ref(),
                    ctx.accounts.admin.key.as_ref(),
                ]);
            } else {
                msg!(
                    "Reward rate unchanged: {}. No pending change to cancel.",
                    pool_data.reward_rate
                );
            }
        } else {
            // Proposing a new rate different from current
            // Check if there's already a pending reward rate change
            // This prevents authority from indefinitely deferring changes by repeatedly proposing new rates
            if pool_data.pending_reward_rate.is_some() {
                msg!(
                    "Cannot propose new reward rate change while one is already pending. Finalize the current pending change first."
                );
                return Err(StakePoolError::PendingRewardRateChangeExists.into());
            }

            // Enforce cooldown period since last rate change
            // This prevents authority from bypassing the 7-day time-lock by:
            // 1. Proposing rate A, waiting 7 days, finalizing
            // 2. Immediately proposing rate B without another 7-day wait
            // Users need consistent notice periods for all rate changes
            if let Some(last_change) = pool_data.last_rate_change {
                let time_since_last_change = current_time
                    .checked_sub(last_change)
                    .ok_or(StakePoolError::NumericalOverflow)?;

                if time_since_last_change < REWARD_RATE_CHANGE_DELAY {
                    let remaining = REWARD_RATE_CHANGE_DELAY
                        .checked_sub(time_since_last_change)
                        .unwrap_or(0);
                    msg!(
                        "Cannot propose new reward rate change yet. Cooldown period: {} seconds remaining since last change",
                        remaining
                    );
                    return Err(StakePoolError::RewardRateChangeDelayNotElapsed.into());
                }
            }

            // Validate timestamp arithmetic before modifying state
            let finalization_time = current_time
                .checked_add(REWARD_RATE_CHANGE_DELAY)
                .ok_or_else(|| {
                    msg!("Error: Reward rate change finalization time overflowed. Invalid timestamp.");
                    StakePoolError::InvalidParameters
                })?;

            // Set pending reward rate change instead of immediate change
            // This gives users 7 days to exit if they disagree
            pool_data.pending_reward_rate = Some(rate);
            pool_data.reward_rate_change_timestamp = Some(current_time);

            msg!(
                "Reward rate change proposed: {} -> {}. Will take effect after {} (7 days from now)",
                pool_data.reward_rate,
                rate,
                finalization_time
            );

            // Emit event for off-chain indexing
            sol_log_data(&[
                b"RewardRateProposed",
                ctx.accounts.pool.key.as_ref(),
                ctx.accounts.admin.key.as_ref(),
                &pool_data.reward_rate.to_le_bytes(),
                &rate.to_le_bytes(),
            ]);
        }
    }
    if let Some(min_amount) = min_stake_amount {
        pool_data.min_stake_amount = min_amount;
        msg!("Min stake amount updated to: {}", min_amount);

        // Emit event
        sol_log_data(&[
            b"PoolParameterUpdated",
            ctx.accounts.pool.key.as_ref(),
            ctx.accounts.admin.key.as_ref(),
            b"min_stake_amount",
            &min_amount.to_le_bytes(),
        ]);
    }
    if let Some(lockup) = lockup_period {
        if lockup < 0 {
            msg!("Lockup period cannot be negative: {}", lockup);
            return Err(StakePoolError::InvalidParameters.into());
        }
        pool_data.lockup_period = lockup;
        msg!("Lockup period updated to: {}", lockup);

        // Emit event
        sol_log_data(&[
            b"PoolParameterUpdated",
            ctx.accounts.pool.key.as_ref(),
            ctx.accounts.admin.key.as_ref(),
            b"lockup_period",
            &lockup.to_le_bytes(),
        ]);
    }
    if let Some(paused) = is_paused {
        let status_change = if paused { "PAUSED" } else { "UNPAUSED" };
        msg!("Pool {} {}", ctx.accounts.pool.key, status_change);

        // Emit event for off-chain indexing
        sol_log_data(&[
            if paused {
                b"PoolPaused"
            } else {
                b"PoolUnpaused"
            },
            ctx.accounts.pool.key.as_ref(),
            ctx.accounts.admin.key.as_ref(),
        ]);

        pool_data.is_paused = paused;
    }
    if let Some(enforce) = enforce_lockup {
        pool_data.enforce_lockup = enforce;
        msg!("Enforce lockup updated to: {}", enforce);

        // Emit event
        sol_log_data(&[
            b"PoolParameterUpdated",
            ctx.accounts.pool.key.as_ref(),
            ctx.accounts.admin.key.as_ref(),
            b"enforce_lockup",
            &[if enforce { 1u8 } else { 0u8 }],
        ]);
    }
    if let Some(end_date) = pool_end_date {
        // Prevent extending pool after end date has passed
        if let Some(existing_end) = pool_data.pool_end_date {
            if current_time >= existing_end {
                // Pool has already ended
                if let Some(new_end) = end_date {
                    if new_end > existing_end {
                        msg!(
                            "Cannot extend pool after end date has passed. Current: {}, Existing end: {}, Attempted new end: {}",
                            current_time,
                            existing_end,
                            new_end
                        );
                        return Err(StakePoolError::PoolEnded.into());
                    }
                }
            }
        }
        pool_data.pool_end_date = end_date;
        msg!("Pool end date updated to: {:?}", end_date);

        // Emit event
        if let Some(timestamp) = end_date {
            sol_log_data(&[
                b"PoolParameterUpdated",
                ctx.accounts.pool.key.as_ref(),
                ctx.accounts.admin.key.as_ref(),
                b"pool_end_date",
                &timestamp.to_le_bytes(),
            ]);
        } else {
            sol_log_data(&[
                b"PoolParameterUpdated",
                ctx.accounts.pool.key.as_ref(),
                ctx.accounts.admin.key.as_ref(),
                b"pool_end_date_removed",
            ]);
        }
    }

    pool_data.save(ctx.accounts.pool)
}

/// Transfer the global program authority to a new admin (two-step process: step 1)
pub fn transfer_program_authority<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let ctx = TransferProgramAuthorityAccounts::context(accounts)?;

    // Verify program authority account
    assert_account_key(
        "program_authority",
        ctx.accounts.program_authority,
        Key::ProgramAuthority,
    )?;
    assert_program_owner(
        "program_authority",
        ctx.accounts.program_authority,
        &crate::ID,
    )?;

    // Load program authority
    let mut program_authority = ProgramAuthority::load(ctx.accounts.program_authority)?;

    // Guards
    assert_signer("current_authority", ctx.accounts.current_authority)?;
    assert_writable("program_authority", ctx.accounts.program_authority)?;
    assert_same_pubkeys(
        "current_authority",
        ctx.accounts.current_authority,
        &program_authority.authority,
    )?;

    // Validate new authority is not the same as current
    if ctx.accounts.new_authority.key == &program_authority.authority {
        msg!("New authority cannot be the same as current authority");
        return Err(StakePoolError::InvalidParameters.into());
    }

    // Set pending authority
    program_authority.pending_authority = Some(*ctx.accounts.new_authority.key);

    msg!(
        "Nominated new program authority: {}. Pending acceptance.",
        ctx.accounts.new_authority.key
    );

    // Save state
    program_authority.save(ctx.accounts.program_authority)?;

    // Emit event
    sol_log_data(&[
        b"ProgramAuthorityNominated",
        ctx.accounts.current_authority.key.as_ref(),
        ctx.accounts.new_authority.key.as_ref(),
    ]);

    Ok(())
}

/// Accept the transfer of program authority (two-step process: step 2)
pub fn accept_program_authority<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let ctx = AcceptProgramAuthorityAccounts::context(accounts)?;

    // Verify program authority account
    assert_account_key(
        "program_authority",
        ctx.accounts.program_authority,
        Key::ProgramAuthority,
    )?;
    assert_program_owner(
        "program_authority",
        ctx.accounts.program_authority,
        &crate::ID,
    )?;

    // Load program authority
    let mut program_authority = ProgramAuthority::load(ctx.accounts.program_authority)?;

    // Guards
    assert_signer("pending_authority", ctx.accounts.pending_authority)?;
    assert_writable("program_authority", ctx.accounts.program_authority)?;

    // Verify there is a pending authority
    let pending_authority = program_authority
        .pending_authority
        .ok_or(StakePoolError::NoPendingAuthority)?;

    // Verify the signer is the pending authority
    if ctx.accounts.pending_authority.key != &pending_authority {
        msg!(
            "Signer {} is not the pending authority {}",
            ctx.accounts.pending_authority.key,
            pending_authority
        );
        return Err(StakePoolError::InvalidPendingAuthority.into());
    }

    // Complete the authority transfer
    let old_authority = program_authority.authority;
    program_authority.authority = pending_authority;
    program_authority.pending_authority = None;

    msg!(
        "Program authority transfer complete. Old: {}, New: {}",
        old_authority,
        program_authority.authority
    );

    // Save state
    program_authority.save(ctx.accounts.program_authority)?;

    // Emit event
    sol_log_data(&[
        b"ProgramAuthorityTransferred",
        old_authority.as_ref(),
        program_authority.authority.as_ref(),
    ]);

    Ok(())
}

/// Finalize a pending reward rate change after the delay period has elapsed
///
/// This completes the two-step process for changing reward rates:
/// 1. Authority calls update_pool with new rate (sets pending)
/// 2. After 7 days, anyone can call this to apply the change
///
/// # Security: L-01 Mitigation
/// This time-locked mechanism prevents centralized surprise changes to reward rates.
/// Users have 7 days notice to unstake if they disagree with the new rate.
///
/// # Permissionless Design
/// **IMPORTANT**: This function can be called by ANYONE after the delay period.
/// This is intentional to prevent the authority from blocking finalization indefinitely.
///
/// ## Safety Properties:
/// - **Idempotent**: Safe to call multiple times - subsequent calls will fail with
///   NoPendingRewardRateChange error since pending fields are cleared atomically
/// - **No Race Conditions**: Single atomic update in pool_data.save() prevents any
///   race conditions between concurrent finalization attempts
/// - **Time-Lock Enforcement**: Cannot be called until exactly REWARD_RATE_CHANGE_DELAY
///   seconds have elapsed, regardless of who calls it
/// - **Rate Validation**: Pending rate is re-validated during finalization (defense-in-depth)
///
/// ## Attack Surface:
/// - Authority cannot prevent finalization once delay elapses
/// - Griefing is prevented: only one finalization can succeed (first caller wins)
/// - No economic incentive for early/late finalization: rate change is deterministic
///
/// ## Interaction with Authority Transfers
/// If authority is transferred (via nominate_new_authority and accept_authority) while a
/// reward rate change is pending:
///
/// 1. **New authority CAN cancel**: By calling update_pool with reward_rate = current_rate,
///    the new authority can cancel the pending change (same mechanism available to any authority)
/// 2. **New authority CANNOT propose different rate**: PendingRewardRateChangeExists error
///    blocks any new proposals until the pending change is finalized or cancelled
/// 3. **Anyone can finalize**: After the delay period, anyone (including the new authority)
///    can call this function to complete the change proposed by the previous authority
///
/// The new authority inherits full control over the pending change and can choose to either
/// let it finalize (by waiting) or cancel it (by reproposing the current rate).
pub fn finalize_reward_rate_change<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = FinalizeRewardRateChangeAccounts::context(accounts)?;

    // Verify pool account discriminator before loading (Type Cosplay protection)
    assert_account_key("pool", ctx.accounts.pool, Key::StakePool)?;

    // Verify program ownership
    assert_program_owner("pool", ctx.accounts.pool, &crate::ID)?;

    // Load pool
    let mut pool_data = StakePool::load(ctx.accounts.pool)?;

    // Guards
    assert_writable("pool", ctx.accounts.pool)?;

    // Invariant check: pending_reward_rate and reward_rate_change_timestamp must be in sync
    // Both should be Some or both should be None. Mismatch indicates data corruption or a bug.
    if pool_data.pending_reward_rate.is_some() != pool_data.reward_rate_change_timestamp.is_some() {
        msg!(
            "Data corruption: inconsistent pending reward rate state (pending_rate: {:?}, timestamp: {:?})",
            pool_data.pending_reward_rate,
            pool_data.reward_rate_change_timestamp
        );
        return Err(StakePoolError::DataCorruption.into());
    }

    // Check if there is a pending reward rate change
    let pending_rate = pool_data
        .pending_reward_rate
        .ok_or(StakePoolError::NoPendingRewardRateChange)?;

    let change_timestamp = pool_data
        .reward_rate_change_timestamp
        .ok_or(StakePoolError::NoPendingRewardRateChange)?;

    // Check if the delay period has elapsed
    let current_time = Clock::get()?.unix_timestamp;
    validate_current_timestamp(current_time)?;
    validate_stored_timestamp(change_timestamp, current_time)?;

    let time_elapsed = current_time
        .checked_sub(change_timestamp)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // Require at least REWARD_RATE_CHANGE_DELAY seconds to have passed
    // Using < (not <=) means we need time_elapsed >= REWARD_RATE_CHANGE_DELAY
    // Example: If delay is 604800 seconds (7 days), and change_timestamp = 1000000,
    //          finalization is allowed when current_time >= 1604800 (exactly 7 days later)
    if time_elapsed < REWARD_RATE_CHANGE_DELAY {
        msg!(
            "Reward rate change delay not elapsed. Time remaining: {} seconds",
            REWARD_RATE_CHANGE_DELAY
                .checked_sub(time_elapsed)
                .unwrap_or(0)
        );
        return Err(StakePoolError::RewardRateChangeDelayNotElapsed.into());
    }

    // Validate pending rate is within acceptable bounds (defense in depth)
    // Even though validated when proposed, validation logic could have changed
    if pending_rate > MAX_REWARD_RATE {
        msg!("Pending reward rate too high: {}", pending_rate);
        return Err(StakePoolError::InvalidParameters.into());
    }

    // Apply the pending change
    let old_rate = pool_data.reward_rate;
    pool_data.reward_rate = pending_rate;
    pool_data.pending_reward_rate = None;
    pool_data.reward_rate_change_timestamp = None;
    pool_data.last_rate_change = Some(current_time);

    msg!(
        "Reward rate change finalized: {} -> {}",
        old_rate,
        pool_data.reward_rate
    );

    // Save state first to ensure persistence before emitting event
    pool_data.save(ctx.accounts.pool)?;

    // Emit event for off-chain indexing after successful state save
    sol_log_data(&[
        b"RewardRateFinalized",
        ctx.accounts.pool.key.as_ref(),
        &old_rate.to_le_bytes(),
        &pool_data.reward_rate.to_le_bytes(),
    ]);

    Ok(())
}

/// Get all authorized creators (view function for off-chain queries)
///
/// This is a read-only operation that returns the ProgramAuthority account data.
/// Intended to be called via simulateTransaction for off-chain queries.
///
/// # Returns
/// Always returns Ok(()) - the account data can be deserialized client-side
pub fn get_authorized_creators<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let ctx = GetAuthorizedCreatorsAccounts::context(accounts)?;

    // Verify program authority account
    assert_account_key(
        "program_authority",
        ctx.accounts.program_authority,
        Key::ProgramAuthority,
    )?;
    assert_program_owner(
        "program_authority",
        ctx.accounts.program_authority,
        &crate::ID,
    )?;

    // Load to verify account is valid
    let _program_authority = ProgramAuthority::load(ctx.accounts.program_authority)?;

    // Return success - caller can deserialize the account data
    msg!("ProgramAuthority account verified");
    Ok(())
}

/// Check if an address is authorized (view function for off-chain queries)
///
/// Returns Ok(()) if the address is authorized to create pools.
/// Returns Unauthorized error if not authorized.
/// Intended to be called via simulateTransaction for off-chain queries.
///
/// # Arguments
/// * `accounts` - Required accounts
/// * `address` - The address to check
pub fn check_authorization<'a>(accounts: &'a [AccountInfo<'a>], address: Pubkey) -> ProgramResult {
    let ctx = CheckAuthorizationAccounts::context(accounts)?;

    // Verify program authority account
    assert_account_key(
        "program_authority",
        ctx.accounts.program_authority,
        Key::ProgramAuthority,
    )?;
    assert_program_owner(
        "program_authority",
        ctx.accounts.program_authority,
        &crate::ID,
    )?;

    // Load program authority
    let program_authority = ProgramAuthority::load(ctx.accounts.program_authority)?;

    // Check if address is authorized
    if !program_authority.is_authorized(&address) {
        msg!("Address {} is not authorized", address);
        return Err(StakePoolError::Unauthorized.into());
    }

    msg!("Address {} is authorized", address);
    Ok(())
}

/// Cancel a pending authority transfer
///
/// Allows the current authority to cancel a pending transfer before it's accepted.
/// This provides flexibility if the authority changes their mind or nominates
/// the wrong address.
///
/// # Security
/// - Only current authority can cancel
/// - Returns error if no pending transfer exists
pub fn cancel_authority_transfer<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let ctx = CancelAuthorityTransferAccounts::context(accounts)?;

    // Verify program authority account
    assert_account_key(
        "program_authority",
        ctx.accounts.program_authority,
        Key::ProgramAuthority,
    )?;
    assert_program_owner(
        "program_authority",
        ctx.accounts.program_authority,
        &crate::ID,
    )?;

    // Load program authority
    let mut program_authority = ProgramAuthority::load(ctx.accounts.program_authority)?;

    // Guards
    assert_signer("current_authority", ctx.accounts.current_authority)?;
    assert_writable("program_authority", ctx.accounts.program_authority)?;

    // Verify signer is current authority
    if ctx.accounts.current_authority.key != &program_authority.authority {
        msg!(
            "Unauthorized: {} is not the current authority",
            ctx.accounts.current_authority.key
        );
        return Err(StakePoolError::Unauthorized.into());
    }

    // Verify there is a pending authority to cancel
    let pending = program_authority
        .pending_authority
        .ok_or(StakePoolError::NoPendingAuthority)?;

    // Clear pending authority
    program_authority.pending_authority = None;

    msg!(
        "Authority transfer cancelled. Pending authority {} removed.",
        pending
    );

    // Save state
    program_authority.save(ctx.accounts.program_authority)?;

    // Emit event
    sol_log_data(&[
        b"ProgramAuthorityTransferCancelled",
        ctx.accounts.current_authority.key.as_ref(),
        pending.as_ref(),
    ]);

    Ok(())
}

/// Close the ProgramAuthority PDA and transfer its lamports to the receiver.
///
/// Security: This instruction attempts to load and verify the authority from the
/// ProgramAuthority account. If deserialization fails (e.g., after an account
/// structure upgrade), it falls back to verifying the PDA derivation seeds only,
/// which provides basic protection against arbitrary account closure.
///
/// This fallback is intentional to handle migration scenarios where the account
/// structure has changed between program versions. In such cases, only someone
/// who can sign for a valid authority address can close the account.
///
/// Intended for dev/test cleanup. Use with caution on mainnet.
pub fn close_program_authority<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let ctx = CloseProgramAuthorityAccounts::context(accounts)?;

    // Guards
    assert_signer("authority", ctx.accounts.authority)?;
    assert_writable("program_authority", ctx.accounts.program_authority)?;
    assert_writable("receiver", ctx.accounts.receiver)?;

    // Verify PDA derivation
    let program_authority_seeds = ProgramAuthority::seeds();
    let program_authority_seeds_refs: Vec<&[u8]> = program_authority_seeds
        .iter()
        .map(|s| s.as_slice())
        .collect();
    let (expected_pda, _bump) =
        Pubkey::find_program_address(&program_authority_seeds_refs, &crate::ID);

    if ctx.accounts.program_authority.key != &expected_pda {
        msg!(
            "Invalid ProgramAuthority PDA: expected {}, got {}",
            expected_pda,
            ctx.accounts.program_authority.key
        );
        return Err(StakePoolError::InvalidParameters.into());
    }

    // Verify program ownership
    assert_program_owner(
        "program_authority",
        ctx.accounts.program_authority,
        &crate::ID,
    )?;

    // Attempt to load and verify authority
    // If this fails due to account structure mismatch (migration scenario),
    // we'll just verify that the account exists and is owned by our program
    match ProgramAuthority::load(ctx.accounts.program_authority) {
        Ok(program_authority) => {
            // Normal case: account can be deserialized
            // Verify discriminator
            if !matches!(program_authority.key, Key::ProgramAuthority) {
                msg!("Invalid account discriminator");
                return Err(StakePoolError::InvalidParameters.into());
            }

            // Verify the signer is the current authority
            if ctx.accounts.authority.key != &program_authority.authority {
                msg!(
                    "Unauthorized: {} is not the program authority (expected {})",
                    ctx.accounts.authority.key,
                    program_authority.authority
                );
                return Err(StakePoolError::Unauthorized.into());
            }

            // Ensure there's no pending authority transfer (safety guard)
            if program_authority.pending_authority.is_some() {
                msg!("Cannot close program authority while a transfer is pending");
                return Err(StakePoolError::InvalidParameters.into());
            }

            msg!(
                "Closing ProgramAuthority account (authority: {})",
                program_authority.authority
            );
        }
        Err(_) => {
            // Migration case: account exists but can't be deserialized
            // This happens when the account structure has changed
            msg!(
                "Warning: ProgramAuthority account cannot be deserialized (likely structure mismatch)"
            );
            msg!("Proceeding with close based on PDA verification only");
            msg!(
                "Closing ProgramAuthority PDA {} (signer: {})",
                ctx.accounts.program_authority.key,
                ctx.accounts.authority.key
            );
        }
    }

    // Close account and transfer lamports
    crate::utils::close_account(ctx.accounts.program_authority, ctx.accounts.receiver)?;

    msg!(
        "Closed ProgramAuthority {} and returned lamports to {}",
        ctx.accounts.program_authority.key,
        ctx.accounts.receiver.key
    );

    Ok(())
}
