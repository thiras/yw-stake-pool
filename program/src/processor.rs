use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};
use spl_token_2022::{extension::StateWithExtensions, state::Account as TokenAccount};

use crate::assertions::*;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::instruction::StakePoolInstruction;
use crate::state::{Key, StakeAccount, StakePool};
use crate::utils::{close_account, create_account, transfer_tokens_with_fee};

pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    // Validate instruction data before deserialization to prevent type cosplay attacks
    if instruction_data.is_empty() {
        msg!("Instruction data is empty");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Deserialize instruction with explicit error handling
    let instruction: StakePoolInstruction = StakePoolInstruction::try_from_slice(instruction_data)
        .map_err(|_| {
            msg!("Failed to deserialize instruction");
            ProgramError::InvalidInstructionData
        })?;

    match instruction {
        StakePoolInstruction::InitializePool {
            reward_rate,
            min_stake_amount,
            lockup_period,
            pool_end_date,
        } => {
            msg!("Instruction: InitializePool");
            initialize_pool(
                accounts,
                reward_rate,
                min_stake_amount,
                lockup_period,
                pool_end_date,
            )
        }
        StakePoolInstruction::Stake {
            amount,
            index,
            expected_reward_rate,
            expected_lockup_period,
        } => {
            msg!("Instruction: Stake");
            stake(
                accounts,
                amount,
                index,
                expected_reward_rate,
                expected_lockup_period,
            )
        }
        StakePoolInstruction::Unstake {
            amount,
            expected_reward_rate,
        } => {
            msg!("Instruction: Unstake");
            unstake(accounts, amount, expected_reward_rate)
        }
        StakePoolInstruction::ClaimRewards => {
            msg!("Instruction: ClaimRewards");
            claim_rewards(accounts)
        }
        StakePoolInstruction::UpdatePool {
            reward_rate,
            min_stake_amount,
            lockup_period,
            is_paused,
            pool_end_date,
        } => {
            msg!("Instruction: UpdatePool");
            update_pool(
                accounts,
                reward_rate,
                min_stake_amount,
                lockup_period,
                is_paused,
                pool_end_date,
            )
        }
        StakePoolInstruction::FundRewards { amount } => {
            msg!("Instruction: FundRewards");
            fund_rewards(accounts, amount)
        }
        StakePoolInstruction::NominateNewAuthority => {
            msg!("Instruction: NominateNewAuthority");
            nominate_new_authority(accounts)
        }
        StakePoolInstruction::AcceptAuthority => {
            msg!("Instruction: AcceptAuthority");
            accept_authority(accounts)
        }
        StakePoolInstruction::CloseStakeAccount => {
            msg!("Instruction: CloseStakeAccount");
            close_stake_account(accounts)
        }
    }
}

fn initialize_pool<'a>(
    accounts: &'a [AccountInfo<'a>],
    reward_rate: u64,
    min_stake_amount: u64,
    lockup_period: i64,
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
    let pool_seeds = StakePool::seeds(ctx.accounts.authority.key, ctx.accounts.stake_mint.key);
    let pool_seeds_refs: Vec<&[u8]> = pool_seeds.iter().map(|s| s.as_slice()).collect();
    let (pool_key, bump) = Pubkey::find_program_address(&pool_seeds_refs, &crate::ID);

    assert_same_pubkeys("pool", ctx.accounts.pool, &pool_key)?;
    assert_signer("authority", ctx.accounts.authority)?;
    assert_signer("payer", ctx.accounts.payer)?;
    assert_empty("pool", ctx.accounts.pool)?;
    assert_writable("pool", ctx.accounts.pool)?;
    assert_writable("stake_vault", ctx.accounts.stake_vault)?;
    assert_writable("reward_vault", ctx.accounts.reward_vault)?;
    assert_writable("payer", ctx.accounts.payer)?;

    // Verify token accounts
    verify_token_account(ctx.accounts.stake_vault, ctx.accounts.stake_mint.key)?;
    verify_token_account(ctx.accounts.reward_vault, ctx.accounts.reward_mint.key)?;

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
        stake_vault: *ctx.accounts.stake_vault.key,
        reward_vault: *ctx.accounts.reward_vault.key,
        total_staked: 0,
        total_rewards_owed: 0,
        reward_rate,
        min_stake_amount,
        lockup_period,
        is_paused: false,
        bump,
        pending_authority: None,
        pool_end_date,
        _reserved: [0; 32],
    };

    pool_data.save(ctx.accounts.pool)
}

fn stake<'a>(
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
    verify_token_account(ctx.accounts.user_token_account, &pool_data.stake_mint)?;
    verify_token_account(ctx.accounts.stake_vault, &pool_data.stake_mint)?;

    if pool_data.is_paused {
        return Err(StakePoolError::PoolPaused.into());
    }

    // Check if pool has ended
    if let Some(end_date) = pool_data.pool_end_date {
        let current_time = Clock::get()?.unix_timestamp;
        if current_time >= end_date {
            msg!(
                "Pool has ended. End date: {}, Current time: {}",
                end_date,
                current_time
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

    // Get current time
    let clock = Clock::get()?;

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

fn unstake<'a>(
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
    verify_token_account(ctx.accounts.user_token_account, &pool_data.stake_mint)?;
    verify_token_account(ctx.accounts.stake_vault, &pool_data.stake_mint)?;

    if stake_account_data.amount_staked < amount {
        return Err(StakePoolError::InsufficientStakedBalance.into());
    }

    // Get current time
    let clock = Clock::from_account_info(ctx.accounts.clock)?;

    // Check lockup period - allow unstaking anytime but warn if not complete
    let time_staked = clock
        .unix_timestamp
        .checked_sub(stake_account_data.stake_timestamp)
        .ok_or(StakePoolError::NumericalOverflow)?;

    let lockup_complete = time_staked >= pool_data.lockup_period;

    if !lockup_complete {
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
    let pool_seeds = StakePool::seeds(&pool_data.authority, &pool_data.stake_mint);
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

fn claim_rewards<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = ClaimRewardsAccounts::context(accounts)?;

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

    // Guards
    assert_signer("owner", ctx.accounts.owner)?;
    assert_writable("pool", ctx.accounts.pool)?;
    assert_writable("stake_account", ctx.accounts.stake_account)?;
    assert_writable("user_reward_account", ctx.accounts.user_reward_account)?;
    assert_writable("reward_vault", ctx.accounts.reward_vault)?;
    assert_same_pubkeys("owner", ctx.accounts.owner, &stake_account_data.owner)?;
    assert_same_pubkeys("pool", ctx.accounts.pool, &stake_account_data.pool)?;
    assert_same_pubkeys(
        "reward_vault",
        ctx.accounts.reward_vault,
        &pool_data.reward_vault,
    )?;
    assert_same_pubkeys(
        "reward_mint",
        ctx.accounts.reward_mint,
        &pool_data.reward_mint,
    )?;

    // Verify token accounts belong to correct mints
    verify_token_account(ctx.accounts.user_reward_account, &pool_data.reward_mint)?;
    verify_token_account(ctx.accounts.reward_vault, &pool_data.reward_mint)?;

    // Get current time
    let clock = Clock::from_account_info(ctx.accounts.clock)?;

    // Calculate total rewards based on stake duration and reward rate
    // Rewards are only given if lockup period is complete
    let total_rewards = pool_data.calculate_rewards(
        stake_account_data.amount_staked,
        stake_account_data.stake_timestamp,
        clock.unix_timestamp,
    )?;

    // Calculate unclaimed rewards (total - already claimed)
    let unclaimed_rewards = total_rewards
        .checked_sub(stake_account_data.claimed_rewards)
        .ok_or(StakePoolError::NumericalOverflow)?;

    if unclaimed_rewards == 0 {
        msg!("No rewards to claim - lockup period not complete, no stake, or rewards already claimed");
        return Ok(());
    }

    // Check reward vault has sufficient balance
    let reward_vault_balance = get_token_account_balance(ctx.accounts.reward_vault)?;
    if reward_vault_balance < unclaimed_rewards {
        return Err(StakePoolError::InsufficientRewards.into());
    }

    // Transfer rewards (with PDA signer)
    let pool_seeds = StakePool::seeds(&pool_data.authority, &pool_data.stake_mint);
    let mut seeds_with_bump = pool_seeds.clone();
    seeds_with_bump.push(vec![pool_data.bump]);
    let seeds_refs: Vec<&[u8]> = seeds_with_bump.iter().map(|s| s.as_slice()).collect();

    transfer_tokens_with_fee(
        ctx.accounts.reward_vault,
        ctx.accounts.user_reward_account,
        ctx.accounts.reward_mint,
        ctx.accounts.pool,
        ctx.accounts.token_program,
        unclaimed_rewards,
        &[&seeds_refs],
    )?;

    // Update claimed rewards tracking
    stake_account_data.claimed_rewards = stake_account_data
        .claimed_rewards
        .checked_add(unclaimed_rewards)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // Update pool's total rewards owed (these rewards have now been paid out)
    pool_data.total_rewards_owed = pool_data
        .total_rewards_owed
        .checked_sub(unclaimed_rewards)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // Save updated accounts
    pool_data.save(ctx.accounts.pool)?;
    stake_account_data.save(ctx.accounts.stake_account)?;

    msg!(
        "Claimed {} reward tokens (total claimed: {})",
        unclaimed_rewards,
        stake_account_data.claimed_rewards
    );

    Ok(())
}

fn update_pool<'a>(
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

fn fund_rewards<'a>(accounts: &'a [AccountInfo<'a>], amount: u64) -> ProgramResult {
    // Validate amount
    if amount == 0 {
        msg!("Fund amount must be greater than zero");
        return Err(StakePoolError::InvalidParameters.into());
    }

    // Parse accounts using ShankContext-generated struct
    let ctx = FundRewardsAccounts::context(accounts)?;

    // Verify pool account discriminator before loading (Type Cosplay protection)
    assert_account_key("pool", ctx.accounts.pool, Key::StakePool)?;

    // Verify program ownership
    assert_program_owner("pool", ctx.accounts.pool, &crate::ID)?;

    // Load pool
    let pool_data = StakePool::load(ctx.accounts.pool)?;

    // Guards
    assert_signer("funder", ctx.accounts.funder)?;
    assert_writable("funder_token_account", ctx.accounts.funder_token_account)?;
    assert_writable("reward_vault", ctx.accounts.reward_vault)?;
    assert_same_pubkeys(
        "reward_vault",
        ctx.accounts.reward_vault,
        &pool_data.reward_vault,
    )?;
    assert_same_pubkeys(
        "reward_mint",
        ctx.accounts.reward_mint,
        &pool_data.reward_mint,
    )?;

    // Verify token accounts belong to correct mints
    verify_token_account(ctx.accounts.funder_token_account, &pool_data.reward_mint)?;
    verify_token_account(ctx.accounts.reward_vault, &pool_data.reward_mint)?;

    // Transfer reward tokens to pool
    transfer_tokens_with_fee(
        ctx.accounts.funder_token_account,
        ctx.accounts.reward_vault,
        ctx.accounts.reward_mint,
        ctx.accounts.funder,
        ctx.accounts.token_program,
        amount,
        &[],
    )?;

    msg!("Funded pool with {} reward tokens", amount);
    Ok(())
}

fn nominate_new_authority<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
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

fn accept_authority<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
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

fn close_stake_account<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = CloseStakeAccountAccounts::context(accounts)?;

    // Verify stake account discriminator before loading (Type Cosplay protection)
    assert_account_key(
        "stake_account",
        ctx.accounts.stake_account,
        Key::StakeAccount,
    )?;

    // Verify program ownership
    assert_program_owner("stake_account", ctx.accounts.stake_account, &crate::ID)?;

    // Load stake account
    let stake_account_data = StakeAccount::load(ctx.accounts.stake_account)?;

    // Guards
    assert_signer("owner", ctx.accounts.owner)?;
    assert_writable("stake_account", ctx.accounts.stake_account)?;
    assert_writable("receiver", ctx.accounts.receiver)?;
    assert_same_pubkeys("owner", ctx.accounts.owner, &stake_account_data.owner)?;

    // Ensure stake account is empty (no staked amount)
    if stake_account_data.amount_staked != 0 {
        msg!(
            "Cannot close stake account with balance. Amount staked: {}",
            stake_account_data.amount_staked
        );
        return Err(StakePoolError::InsufficientStakedBalance.into());
    }

    // Close the account and recover rent
    close_account(ctx.accounts.stake_account, ctx.accounts.receiver)?;

    msg!(
        "Closed stake account {} and returned rent to {}",
        ctx.accounts.stake_account.key,
        ctx.accounts.receiver.key
    );

    Ok(())
}

// Helper functions
fn verify_token_account(token_account: &AccountInfo, expected_mint: &Pubkey) -> ProgramResult {
    let account_data = token_account.try_borrow_data()?;

    // Support both Token and Token-2022
    let account = StateWithExtensions::<TokenAccount>::unpack(&account_data)
        .map_err(|_| StakePoolError::InvalidTokenProgram)?;

    if &account.base.mint != expected_mint {
        return Err(StakePoolError::InvalidMint.into());
    }

    Ok(())
}

fn get_token_account_balance(token_account: &AccountInfo) -> Result<u64, ProgramError> {
    let account_data = token_account.try_borrow_data()?;
    let account = StateWithExtensions::<TokenAccount>::unpack(&account_data)
        .map_err(|_| StakePoolError::InvalidTokenProgram)?;
    Ok(account.base.amount)
}
