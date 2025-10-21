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

pub fn update_pool<'a>(
    accounts: &'a [AccountInfo<'a>],
    reward_rate: Option<u64>,
    min_stake_amount: Option<u64>,
    lockup_period: Option<i64>,
    is_paused: Option<bool>,
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

    // Update fields
    if let Some(rate) = reward_rate {
        if rate > 1_000_000_000_000 {
            msg!("Reward rate too high: {}", rate);
            return Err(StakePoolError::InvalidParameters.into());
        }
        pool_data.reward_rate = rate;
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
    if let Some(end_date) = pool_end_date {
        // Prevent extending pool after end date has passed
        let current_time = Clock::get()?.unix_timestamp;
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
