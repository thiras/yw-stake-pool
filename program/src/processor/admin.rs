use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    sysvar::{clock::Clock, Sysvar},
};

use crate::assertions::*;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::state::{Key, StakePool};

/// Minimum delay before a reward rate change can be finalized (7 days)
/// This gives users time to react and unstake if they disagree with the new rate
const REWARD_RATE_CHANGE_DELAY: i64 = 604800; // 7 days in seconds

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

    // Defensive validation: ensure timestamp is reasonable
    // While Clock::get() should always return valid timestamps, this prevents issues
    // if the system clock is misconfigured or in an invalid state
    if current_time < 0 {
        msg!("Invalid system time: {}", current_time);
        return Err(StakePoolError::InvalidTimestamp.into());
    }

    // Update fields
    if let Some(rate) = reward_rate {
        if rate > 1_000_000_000_000 {
            msg!("Reward rate too high: {}", rate);
            return Err(StakePoolError::InvalidParameters.into());
        }

        // Special case: If proposing the current active rate, cancel any pending change
        // This allows authority to revert/cancel unwanted proposals
        // Note: Under normal operation, pending_reward_rate should never equal current_rate
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

            // Set pending reward rate change instead of immediate change
            // This gives users 7 days to exit if they disagree
            pool_data.pending_reward_rate = Some(rate);
            pool_data.reward_rate_change_timestamp = Some(current_time);

            msg!(
                "Reward rate change proposed: {} -> {}. Will take effect after {} (7 days from now)",
                pool_data.reward_rate,
                rate,
                current_time + REWARD_RATE_CHANGE_DELAY
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

    // Check if there is a pending reward rate change
    let pending_rate = pool_data
        .pending_reward_rate
        .ok_or(StakePoolError::NoPendingRewardRateChange)?;

    let change_timestamp = pool_data
        .reward_rate_change_timestamp
        .ok_or(StakePoolError::NoPendingRewardRateChange)?;

    // Check if the delay period has elapsed
    let current_time = Clock::get()?.unix_timestamp;

    // Validate timestamp is not in the future (clock manipulation detection)
    if change_timestamp > current_time {
        msg!(
            "Invalid timestamp: change timestamp {} is in the future (current: {}). Possible clock manipulation.",
            change_timestamp,
            current_time
        );
        return Err(StakePoolError::InvalidTimestamp.into());
    }

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
            REWARD_RATE_CHANGE_DELAY - time_elapsed
        );
        return Err(StakePoolError::RewardRateChangeDelayNotElapsed.into());
    }

    // Validate pending rate is within acceptable bounds (defense in depth)
    // Even though validated when proposed, validation logic could have changed
    if pending_rate > 1_000_000_000_000 {
        msg!("Pending reward rate too high: {}", pending_rate);
        return Err(StakePoolError::InvalidParameters.into());
    }

    // Apply the pending change
    let old_rate = pool_data.reward_rate;
    pool_data.reward_rate = pending_rate;
    pool_data.pending_reward_rate = None;
    pool_data.reward_rate_change_timestamp = None;

    msg!(
        "Reward rate change finalized: {} -> {}",
        old_rate,
        pool_data.reward_rate
    );

    pool_data.save(ctx.accounts.pool)
}
