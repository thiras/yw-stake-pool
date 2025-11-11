use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

use crate::assertions::*;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::state::{Key, StakeAccount, StakePool};
use crate::utils::{create_account, transfer_tokens_with_fee};

use super::helpers::{get_token_account_balance, validate_current_timestamp, verify_token_account};

pub fn stake<'a>(
    accounts: &'a [AccountInfo<'a>],
    amount: u64,
    index: u64,
    expected_reward_rate: Option<u64>,
    expected_lockup_period: Option<i64>,
) -> ProgramResult {
    // Validate amount
    if amount == 0 {
        msg!("Stake amount must be greater than zero");
        return Err(StakePoolError::InvalidParameters.into());
    }

    // Parse accounts using ShankContext-generated struct
    let ctx = StakeAccounts::context(accounts)?;

    // Verify pool account discriminator before loading (Type Cosplay protection)
    assert_account_key("pool", ctx.accounts.pool, Key::StakePool)?;

    // Verify program ownership
    assert_program_owner("pool", ctx.accounts.pool, &crate::ID)?;

    // Load pool
    let mut pool_data = StakePool::load(ctx.accounts.pool)?;

    // Frontrunning protection: Verify expected pool parameters if provided
    if let Some(expected_rate) = expected_reward_rate {
        if pool_data.reward_rate != expected_rate {
            msg!(
                "Reward rate mismatch: expected {}, got {}",
                expected_rate,
                pool_data.reward_rate
            );
            return Err(StakePoolError::PoolParametersChanged.into());
        }
    }

    if let Some(expected_lockup) = expected_lockup_period {
        if pool_data.lockup_period != expected_lockup {
            msg!(
                "Lockup period mismatch: expected {}, got {}",
                expected_lockup,
                pool_data.lockup_period
            );
            return Err(StakePoolError::PoolParametersChanged.into());
        }
    }

    // Guards
    assert_signer("owner", ctx.accounts.owner)?;
    assert_signer("payer", ctx.accounts.payer)?;
    assert_empty("stake_account", ctx.accounts.stake_account)?;
    assert_writable("pool", ctx.accounts.pool)?;
    assert_writable("stake_account", ctx.accounts.stake_account)?;
    assert_writable("user_token_account", ctx.accounts.user_token_account)?;
    assert_writable("stake_vault", ctx.accounts.stake_vault)?;
    assert_writable("payer", ctx.accounts.payer)?;
    assert_same_pubkeys(
        "stake_vault",
        ctx.accounts.stake_vault,
        &pool_data.stake_vault,
    )?;
    assert_same_pubkeys(
        "reward_vault",
        ctx.accounts.reward_vault,
        &pool_data.reward_vault,
    )?;
    assert_same_pubkeys("stake_mint", ctx.accounts.stake_mint, &pool_data.stake_mint)?;

    // Verify token accounts belong to correct mints
    verify_token_account(
        ctx.accounts.user_token_account,
        &pool_data.stake_mint,
        None,
        None,
    )?;
    verify_token_account(ctx.accounts.stake_vault, &pool_data.stake_mint, None, None)?;

    if pool_data.is_paused {
        return Err(StakePoolError::PoolPaused.into());
    }

    // Get current time once for efficiency and reuse throughout function
    let clock = Clock::get()?;
    validate_current_timestamp(clock.unix_timestamp)?;

    // Check if pool has ended
    if let Some(end_date) = pool_data.pool_end_date {
        if clock.unix_timestamp >= end_date {
            msg!(
                "Pool has ended. End date: {}, Current time: {}",
                end_date,
                clock.unix_timestamp
            );
            return Err(StakePoolError::PoolEnded.into());
        }
    }

    if amount < pool_data.min_stake_amount {
        return Err(StakePoolError::AmountBelowMinimum.into());
    }

    // Calculate expected rewards for this stake
    let expected_rewards = (amount as u128)
        .checked_mul(pool_data.reward_rate as u128)
        .ok_or(StakePoolError::NumericalOverflow)?
        .checked_div(1_000_000_000)
        .ok_or(StakePoolError::NumericalOverflow)? as u64;

    // Check if reward vault has sufficient balance to cover total rewards owed plus this new stake
    let reward_vault_balance = get_token_account_balance(ctx.accounts.reward_vault)?;
    let total_required = pool_data
        .total_rewards_owed
        .checked_add(expected_rewards)
        .ok_or(StakePoolError::NumericalOverflow)?;

    if reward_vault_balance < total_required {
        msg!(
            "Insufficient rewards in pool. Required (total): {}, Available: {}, Already owed: {}, New stake needs: {}",
            total_required,
            reward_vault_balance,
            pool_data.total_rewards_owed,
            expected_rewards
        );
        return Err(StakePoolError::InsufficientRewards.into());
    }

    // Verify stake account PDA
    let stake_account_seeds =
        StakeAccount::seeds(ctx.accounts.pool.key, ctx.accounts.owner.key, index);
    let stake_seeds_refs: Vec<&[u8]> = stake_account_seeds.iter().map(|s| s.as_slice()).collect();
    let (stake_account_key, bump) = Pubkey::find_program_address(&stake_seeds_refs, &crate::ID);

    assert_same_pubkeys(
        "stake_account",
        ctx.accounts.stake_account,
        &stake_account_key,
    )?;

    // Create the new stake account
    let mut seeds_with_bump = stake_account_seeds.clone();
    seeds_with_bump.push(vec![bump]);
    let seeds_refs: Vec<&[u8]> = seeds_with_bump.iter().map(|s| s.as_slice()).collect();

    // Diagnostic logging: print payer and target account lamports and pubkeys
    msg!(
        "Stake() - creating stake account: target={} payer={} target_lamports={} payer_lamports={}",
        ctx.accounts.stake_account.key,
        ctx.accounts.payer.key,
        ctx.accounts.stake_account.lamports(),
        ctx.accounts.payer.lamports()
    );

    create_account(
        ctx.accounts.stake_account,
        ctx.accounts.payer,
        ctx.accounts.system_program,
        StakeAccount::LEN,
        &crate::ID,
        Some(&[&seeds_refs]),
    )?;

    // Transfer tokens with transfer fee support
    let transfer_amount = transfer_tokens_with_fee(
        ctx.accounts.user_token_account,
        ctx.accounts.stake_vault,
        ctx.accounts.stake_mint,
        ctx.accounts.owner,
        ctx.accounts.token_program,
        amount,
        &[],
    )?;

    // Update pool total staked and rewards owed
    pool_data.total_staked = pool_data
        .total_staked
        .checked_add(transfer_amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    pool_data.total_rewards_owed = pool_data
        .total_rewards_owed
        .checked_add(expected_rewards)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // Initialize new stake account with the deposit
    let stake_account_data = StakeAccount {
        key: Key::StakeAccount,
        pool: *ctx.accounts.pool.key,
        owner: *ctx.accounts.owner.key,
        index,
        amount_staked: transfer_amount,
        stake_timestamp: clock.unix_timestamp,
        claimed_rewards: 0,
        bump,
    };

    // Save state
    pool_data.save(ctx.accounts.pool)?;
    stake_account_data.save(ctx.accounts.stake_account)
}

pub fn unstake<'a>(
    accounts: &'a [AccountInfo<'a>],
    amount: u64,
    expected_reward_rate: Option<u64>,
) -> ProgramResult {
    // Validate amount
    if amount == 0 {
        msg!("Unstake amount must be greater than zero");
        return Err(StakePoolError::InvalidParameters.into());
    }

    // Parse accounts using ShankContext-generated struct
    let ctx = UnstakeAccounts::context(accounts)?;

    // Verify account discriminators before loading (Type Cosplay protection)
    assert_account_key("pool", ctx.accounts.pool, Key::StakePool)?;
    assert_account_key(
        "stake_account",
        ctx.accounts.stake_account,
        Key::StakeAccount,
    )?;

    // Verify program ownership
    assert_program_owner("pool", ctx.accounts.pool, &crate::ID)?;
    assert_program_owner("stake_account", ctx.accounts.stake_account, &crate::ID)?;

    // Load accounts
    let mut pool_data = StakePool::load(ctx.accounts.pool)?;
    let mut stake_account_data = StakeAccount::load(ctx.accounts.stake_account)?;

    // Frontrunning protection: Verify expected reward rate if provided
    if let Some(expected_rate) = expected_reward_rate {
        if pool_data.reward_rate != expected_rate {
            msg!(
                "Reward rate mismatch: expected {}, got {}",
                expected_rate,
                pool_data.reward_rate
            );
            return Err(StakePoolError::PoolParametersChanged.into());
        }
    }

    // Guards
    assert_signer("owner", ctx.accounts.owner)?;
    assert_writable("pool", ctx.accounts.pool)?;
    assert_writable("stake_account", ctx.accounts.stake_account)?;
    assert_writable("user_token_account", ctx.accounts.user_token_account)?;
    assert_writable("stake_vault", ctx.accounts.stake_vault)?;
    assert_same_pubkeys("owner", ctx.accounts.owner, &stake_account_data.owner)?;
    assert_same_pubkeys("pool", ctx.accounts.pool, &stake_account_data.pool)?;
    assert_same_pubkeys(
        "stake_vault",
        ctx.accounts.stake_vault,
        &pool_data.stake_vault,
    )?;
    assert_same_pubkeys("stake_mint", ctx.accounts.stake_mint, &pool_data.stake_mint)?;

    // Verify token accounts belong to correct mints
    verify_token_account(
        ctx.accounts.user_token_account,
        &pool_data.stake_mint,
        None,
        None,
    )?;
    verify_token_account(ctx.accounts.stake_vault, &pool_data.stake_mint, None, None)?;

    if stake_account_data.amount_staked < amount {
        return Err(StakePoolError::InsufficientStakedBalance.into());
    }

    // Get current time
    let clock = Clock::from_account_info(ctx.accounts.clock)?;

    // Check lockup period
    let time_staked = clock
        .unix_timestamp
        .checked_sub(stake_account_data.stake_timestamp)
        .ok_or(StakePoolError::NumericalOverflow)?;

    let lockup_complete = time_staked >= pool_data.lockup_period;

    // If enforce_lockup is true, prevent early withdrawals
    if pool_data.enforce_lockup && !lockup_complete {
        msg!(
            "Lockup period not expired. Time staked: {}, Required: {}",
            time_staked,
            pool_data.lockup_period
        );
        return Err(StakePoolError::LockupNotExpired.into());
    }

    // If lockup not enforced and not complete, warn about forfeiting rewards
    if !pool_data.enforce_lockup && !lockup_complete {
        msg!("Warning: Unstaking before lockup period complete. Forfeiting proportional rewards.");
    }

    // Calculate how much of the stake is being removed (as a fraction)
    let total_staked_before = stake_account_data.amount_staked;
    let remaining_stake = total_staked_before
        .checked_sub(amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // Calculate total potential rewards for the original stake
    let total_potential_rewards = if lockup_complete {
        pool_data.calculate_rewards(
            total_staked_before,
            stake_account_data.stake_timestamp,
            clock.unix_timestamp,
        )?
    } else {
        0 // No rewards if lockup not complete
    };

    // Calculate proportional rewards being forfeited
    let forfeited_rewards = if remaining_stake == 0 {
        // Full unstake - forfeit all unclaimed rewards
        total_potential_rewards
            .checked_sub(stake_account_data.claimed_rewards)
            .ok_or(StakePoolError::NumericalOverflow)?
    } else {
        // Partial unstake - forfeit proportional amount of unclaimed rewards
        let unstake_fraction = (amount as u128)
            .checked_mul(1_000_000_000)
            .ok_or(StakePoolError::NumericalOverflow)?
            .checked_div(total_staked_before as u128)
            .ok_or(StakePoolError::NumericalOverflow)? as u64;

        let unclaimed_rewards = total_potential_rewards
            .checked_sub(stake_account_data.claimed_rewards)
            .ok_or(StakePoolError::NumericalOverflow)?;

        (unclaimed_rewards as u128)
            .checked_mul(unstake_fraction as u128)
            .ok_or(StakePoolError::NumericalOverflow)?
            .checked_div(1_000_000_000)
            .ok_or(StakePoolError::NumericalOverflow)? as u64
    };

    // Transfer tokens back (with PDA signer)
    let pool_seeds = StakePool::seeds(
        &pool_data.authority,
        &pool_data.stake_mint,
        pool_data.pool_id,
    );
    let mut seeds_with_bump = pool_seeds.clone();
    seeds_with_bump.push(vec![pool_data.bump]);
    let seeds_refs: Vec<&[u8]> = seeds_with_bump.iter().map(|s| s.as_slice()).collect();

    let actual_amount = transfer_tokens_with_fee(
        ctx.accounts.stake_vault,
        ctx.accounts.user_token_account,
        ctx.accounts.stake_mint,
        ctx.accounts.pool,
        ctx.accounts.token_program,
        amount,
        &[&seeds_refs],
    )?;

    // Update balances with actual transferred amount
    stake_account_data.amount_staked = stake_account_data
        .amount_staked
        .checked_sub(actual_amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    pool_data.total_staked = pool_data
        .total_staked
        .checked_sub(actual_amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // Update rewards owed to reflect forfeited rewards
    pool_data.total_rewards_owed = pool_data
        .total_rewards_owed
        .checked_sub(forfeited_rewards)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // If fully unstaking, reset claimed rewards and timestamp
    // For partial unstakes, keep the original timestamp and adjust expectations
    if stake_account_data.amount_staked == 0 {
        stake_account_data.claimed_rewards = 0;
        stake_account_data.stake_timestamp = 0;
        msg!("Full unstake - stake account reset");
    }

    msg!(
        "Unstaked {} tokens (actual: {}), forfeited {} reward tokens",
        amount,
        actual_amount,
        forfeited_rewards
    );

    // Save state
    pool_data.save(ctx.accounts.pool)?;
    stake_account_data.save(ctx.accounts.stake_account)
}
