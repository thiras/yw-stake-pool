use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    log::sol_log_data,
    msg,
    program_error::ProgramError,
    sysvar::{clock::Clock, Sysvar},
};

use crate::assertions::*;
use crate::constants::MAX_REWARD_RATE;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::processor::helpers::{validate_current_timestamp, validate_stored_timestamp};
use crate::state::{Key, StakePool};

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

    // Guards
    assert_signer("authority", ctx.accounts.authority)?;
    assert_writable("pool", ctx.accounts.pool)?;
    assert_same_pubkeys("authority", ctx.accounts.authority, &pool_data.authority)?;

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
        }
    }
    if let Some(min_amount) = min_stake_amount {
        pool_data.min_stake_amount = min_amount;
    }
    if let Some(lockup) = lockup_period {
        if lockup < 0 {
            msg!("Lockup period cannot be negative: {}", lockup);
            return Err(StakePoolError::InvalidParameters.into());
        }
        pool_data.lockup_period = lockup;
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
            ctx.accounts.authority.key.as_ref(),
        ]);

        pool_data.is_paused = paused;
    }
    if let Some(enforce) = enforce_lockup {
        pool_data.enforce_lockup = enforce;
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
    }

    pool_data.save(ctx.accounts.pool)
}

pub fn nominate_new_authority<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = NominateNewAuthorityAccounts::context(accounts)?;

    // Verify pool account discriminator before loading (Type Cosplay protection)
    assert_account_key("pool", ctx.accounts.pool, Key::StakePool)?;

    // Verify program ownership
    assert_program_owner("pool", ctx.accounts.pool, &crate::ID)?;

    // Load pool
    let mut pool_data = StakePool::load(ctx.accounts.pool)?;

    // Guards
    assert_signer("current_authority", ctx.accounts.current_authority)?;
    assert_writable("pool", ctx.accounts.pool)?;
    assert_same_pubkeys(
        "current_authority",
        ctx.accounts.current_authority,
        &pool_data.authority,
    )?;

    // Validate new authority is not the same as current authority
    if ctx.accounts.new_authority.key == &pool_data.authority {
        msg!("New authority cannot be the same as current authority");
        return Err(ProgramError::InvalidArgument);
    }

    // Set pending authority
    pool_data.pending_authority = Some(*ctx.accounts.new_authority.key);

    msg!(
        "Nominated new authority: {}. Pending acceptance.",
        ctx.accounts.new_authority.key
    );

    pool_data.save(ctx.accounts.pool)
}

pub fn accept_authority<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = AcceptAuthorityAccounts::context(accounts)?;

    // Verify pool account discriminator before loading (Type Cosplay protection)
    assert_account_key("pool", ctx.accounts.pool, Key::StakePool)?;

    // Verify program ownership
    assert_program_owner("pool", ctx.accounts.pool, &crate::ID)?;

    // Load pool
    let mut pool_data = StakePool::load(ctx.accounts.pool)?;

    // Guards
    assert_signer("pending_authority", ctx.accounts.pending_authority)?;
    assert_writable("pool", ctx.accounts.pool)?;

    // Verify there is a pending authority
    let pending_authority = pool_data
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
    let old_authority = pool_data.authority;
    pool_data.authority = pending_authority;
    pool_data.pending_authority = None;

    msg!(
        "Authority transfer complete. Old: {}, New: {}",
        old_authority,
        pool_data.authority
    );

    pool_data.save(ctx.accounts.pool)
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

    pool_data.save(ctx.accounts.pool)
}
