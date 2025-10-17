use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey,
    program_error::ProgramError, sysvar::{clock::Clock, Sysvar},
};
use spl_token_2022::{
    extension::StateWithExtensions,
    state::Account as TokenAccount,
};

use crate::assertions::*;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::instruction::StakePoolInstruction;
use crate::state::{Key, StakeAccount, StakePool};
use crate::utils::*;

pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction: StakePoolInstruction = StakePoolInstruction::try_from_slice(instruction_data)?;
    match instruction {
        StakePoolInstruction::InitializePool {
            reward_rate_per_second,
            min_stake_amount,
            lockup_period,
        } => {
            msg!("Instruction: InitializePool");
            initialize_pool(
                accounts,
                reward_rate_per_second,
                min_stake_amount,
                lockup_period,
            )
        }
        StakePoolInstruction::InitializeStakeAccount => {
            msg!("Instruction: InitializeStakeAccount");
            initialize_stake_account(accounts)
        }
        StakePoolInstruction::Stake { amount } => {
            msg!("Instruction: Stake");
            stake(accounts, amount)
        }
        StakePoolInstruction::Unstake { amount } => {
            msg!("Instruction: Unstake");
            unstake(accounts, amount)
        }
        StakePoolInstruction::ClaimRewards => {
            msg!("Instruction: ClaimRewards");
            claim_rewards(accounts)
        }
        StakePoolInstruction::UpdatePool {
            reward_rate_per_second,
            min_stake_amount,
            lockup_period,
            is_paused,
        } => {
            msg!("Instruction: UpdatePool");
            update_pool(
                accounts,
                reward_rate_per_second,
                min_stake_amount,
                lockup_period,
                is_paused,
            )
        }
        StakePoolInstruction::FundRewards { amount } => {
            msg!("Instruction: FundRewards");
            fund_rewards(accounts, amount)
        }
    }
}

fn initialize_pool<'a>(
    accounts: &'a [AccountInfo<'a>],
    reward_rate_per_second: u64,
    min_stake_amount: u64,
    lockup_period: i64,
) -> ProgramResult {
    let ctx = InitializePoolAccounts::context(accounts)?;

    // Guards
    let pool_seeds = StakePool::seeds(ctx.accounts.authority.key, ctx.accounts.stake_mint.key);
    let pool_seeds_refs: Vec<&[u8]> = pool_seeds.iter().map(|s| s.as_slice()).collect();
    let (pool_key, bump) = Pubkey::find_program_address(&pool_seeds_refs, &crate::ID);
    
    assert_same_pubkeys("pool", ctx.accounts.pool, &pool_key)?;
    assert_signer("authority", ctx.accounts.authority)?;
    assert_signer("payer", ctx.accounts.payer)?;
    assert_empty("pool", ctx.accounts.pool)?;

    // Verify token accounts
    verify_token_account(ctx.accounts.stake_vault, ctx.accounts.stake_mint.key)?;
    verify_token_account(ctx.accounts.reward_vault, ctx.accounts.reward_mint.key)?;

    // Get current time
    let clock = Clock::get()?;

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
    let pool = StakePool {
        key: Key::StakePool,
        authority: *ctx.accounts.authority.key,
        stake_mint: *ctx.accounts.stake_mint.key,
        reward_mint: *ctx.accounts.reward_mint.key,
        stake_vault: *ctx.accounts.stake_vault.key,
        reward_vault: *ctx.accounts.reward_vault.key,
        total_staked: 0,
        reward_rate_per_second,
        last_update_time: clock.unix_timestamp,
        reward_per_token_stored: 0,
        min_stake_amount,
        lockup_period,
        is_paused: false,
        bump,
    };

    pool.save(ctx.accounts.pool)
}

fn initialize_stake_account<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let ctx = InitializeStakeAccountAccounts::context(accounts)?;

    // Guards
    let stake_account_seeds = StakeAccount::seeds(ctx.accounts.pool.key, ctx.accounts.owner.key);
    let stake_seeds_refs: Vec<&[u8]> = stake_account_seeds.iter().map(|s| s.as_slice()).collect();
    let (stake_account_key, bump) = Pubkey::find_program_address(&stake_seeds_refs, &crate::ID);

    assert_same_pubkeys("stake_account", ctx.accounts.stake_account, &stake_account_key)?;
    assert_signer("owner", ctx.accounts.owner)?;
    assert_signer("payer", ctx.accounts.payer)?;
    assert_empty("stake_account", ctx.accounts.stake_account)?;

    // Verify pool exists
    assert_non_empty("pool", ctx.accounts.pool)?;

    // Create stake account
    let mut seeds_with_bump = stake_account_seeds.clone();
    seeds_with_bump.push(vec![bump]);
    let seeds_refs: Vec<&[u8]> = seeds_with_bump.iter().map(|s| s.as_slice()).collect();

    create_account(
        ctx.accounts.stake_account,
        ctx.accounts.payer,
        ctx.accounts.system_program,
        StakeAccount::LEN,
        &crate::ID,
        Some(&[&seeds_refs]),
    )?;

    // Initialize stake account
    let stake_account = StakeAccount {
        key: Key::StakeAccount,
        pool: *ctx.accounts.pool.key,
        owner: *ctx.accounts.owner.key,
        amount_staked: 0,
        reward_per_token_paid: 0,
        rewards_earned: 0,
        stake_timestamp: 0,
        bump,
    };

    stake_account.save(ctx.accounts.stake_account)
}

fn stake<'a>(accounts: &'a [AccountInfo<'a>], amount: u64) -> ProgramResult {
    let ctx = StakeAccounts::context(accounts)?;

    // Load accounts
    let mut pool = StakePool::load(ctx.accounts.pool)?;
    let mut stake_account = StakeAccount::load(ctx.accounts.stake_account)?;

    // Guards
    assert_signer("owner", ctx.accounts.owner)?;
    assert_same_pubkeys("owner", ctx.accounts.owner, &stake_account.owner)?;
    assert_same_pubkeys("pool", ctx.accounts.pool, &stake_account.pool)?;
    
    if pool.is_paused {
        return Err(StakePoolError::PoolPaused.into());
    }

    if amount < pool.min_stake_amount {
        return Err(StakePoolError::AmountBelowMinimum.into());
    }

    // Get current time
    let clock = Clock::get()?;

    // Update pool rewards
    pool.reward_per_token_stored = pool.reward_per_token(clock.unix_timestamp)?;
    pool.last_update_time = clock.unix_timestamp;

    // Update stake account rewards
    stake_account.rewards_earned = pool.calculate_earned(
        stake_account.amount_staked,
        stake_account.reward_per_token_paid,
        stake_account.rewards_earned,
        clock.unix_timestamp,
    )?;
    stake_account.reward_per_token_paid = pool.reward_per_token_stored;

    // Transfer tokens with transfer fee support
    let transfer_amount = transfer_tokens_with_fee(
        ctx.accounts.user_token_account,
        ctx.accounts.stake_vault,
        ctx.accounts.owner,
        ctx.accounts.token_program,
        amount,
        &[],
    )?;

    // Update balances
    stake_account.amount_staked = stake_account
        .amount_staked
        .checked_add(transfer_amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    pool.total_staked = pool
        .total_staked
        .checked_add(transfer_amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // Set stake timestamp on first stake
    if stake_account.stake_timestamp == 0 {
        stake_account.stake_timestamp = clock.unix_timestamp;
    }

    // Save state
    pool.save(ctx.accounts.pool)?;
    stake_account.save(ctx.accounts.stake_account)
}

fn unstake<'a>(accounts: &'a [AccountInfo<'a>], amount: u64) -> ProgramResult {
    let ctx = UnstakeAccounts::context(accounts)?;

    // Load accounts
    let mut pool = StakePool::load(ctx.accounts.pool)?;
    let mut stake_account = StakeAccount::load(ctx.accounts.stake_account)?;

    // Guards
    assert_signer("owner", ctx.accounts.owner)?;
    assert_same_pubkeys("owner", ctx.accounts.owner, &stake_account.owner)?;
    assert_same_pubkeys("pool", ctx.accounts.pool, &stake_account.pool)?;

    if stake_account.amount_staked < amount {
        return Err(StakePoolError::InsufficientStakedBalance.into());
    }

    // Get current time
    let clock = Clock::from_account_info(ctx.accounts.clock)?;

    // Check lockup period
    if pool.lockup_period > 0 {
        let time_staked = clock
            .unix_timestamp
            .checked_sub(stake_account.stake_timestamp)
            .ok_or(StakePoolError::NumericalOverflow)?;

        if time_staked < pool.lockup_period {
            return Err(StakePoolError::LockupNotExpired.into());
        }
    }

    // Update pool rewards
    pool.reward_per_token_stored = pool.reward_per_token(clock.unix_timestamp)?;
    pool.last_update_time = clock.unix_timestamp;

    // Update stake account rewards
    stake_account.rewards_earned = pool.calculate_earned(
        stake_account.amount_staked,
        stake_account.reward_per_token_paid,
        stake_account.rewards_earned,
        clock.unix_timestamp,
    )?;
    stake_account.reward_per_token_paid = pool.reward_per_token_stored;

    // Transfer tokens back (with PDA signer)
    let pool_seeds = StakePool::seeds(&pool.authority, &pool.stake_mint);
    let mut seeds_with_bump = pool_seeds.clone();
    seeds_with_bump.push(vec![pool.bump]);
    let seeds_refs: Vec<&[u8]> = seeds_with_bump.iter().map(|s| s.as_slice()).collect();

    let _transfer_amount = transfer_tokens_with_fee(
        ctx.accounts.stake_vault,
        ctx.accounts.user_token_account,
        ctx.accounts.pool,
        ctx.accounts.token_program,
        amount,
        &[&seeds_refs],
    )?;

    // Update balances
    stake_account.amount_staked = stake_account
        .amount_staked
        .checked_sub(amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    pool.total_staked = pool
        .total_staked
        .checked_sub(amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // Save state
    pool.save(ctx.accounts.pool)?;
    stake_account.save(ctx.accounts.stake_account)
}

fn claim_rewards<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let ctx = ClaimRewardsAccounts::context(accounts)?;

    // Load accounts
    let mut pool = StakePool::load(ctx.accounts.pool)?;
    let mut stake_account = StakeAccount::load(ctx.accounts.stake_account)?;

    // Guards
    assert_signer("owner", ctx.accounts.owner)?;
    assert_same_pubkeys("owner", ctx.accounts.owner, &stake_account.owner)?;
    assert_same_pubkeys("pool", ctx.accounts.pool, &stake_account.pool)?;

    // Get current time
    let clock = Clock::from_account_info(ctx.accounts.clock)?;

    // Update pool rewards
    pool.reward_per_token_stored = pool.reward_per_token(clock.unix_timestamp)?;
    pool.last_update_time = clock.unix_timestamp;

    // Calculate total rewards
    let total_rewards = pool.calculate_earned(
        stake_account.amount_staked,
        stake_account.reward_per_token_paid,
        stake_account.rewards_earned,
        clock.unix_timestamp,
    )?;

    if total_rewards == 0 {
        msg!("No rewards to claim");
        return Ok(());
    }

    // Check reward vault has sufficient balance
    let reward_vault_balance = get_token_account_balance(ctx.accounts.reward_vault)?;
    if reward_vault_balance < total_rewards {
        return Err(StakePoolError::InsufficientRewards.into());
    }

    // Transfer rewards (with PDA signer)
    let pool_seeds = StakePool::seeds(&pool.authority, &pool.stake_mint);
    let mut seeds_with_bump = pool_seeds.clone();
    seeds_with_bump.push(vec![pool.bump]);
    let seeds_refs: Vec<&[u8]> = seeds_with_bump.iter().map(|s| s.as_slice()).collect();

    transfer_tokens_with_fee(
        ctx.accounts.reward_vault,
        ctx.accounts.user_reward_account,
        ctx.accounts.pool,
        ctx.accounts.token_program,
        total_rewards,
        &[&seeds_refs],
    )?;

    // Update stake account
    stake_account.rewards_earned = 0;
    stake_account.reward_per_token_paid = pool.reward_per_token_stored;

    // Save state
    pool.save(ctx.accounts.pool)?;
    stake_account.save(ctx.accounts.stake_account)
}

fn update_pool<'a>(
    accounts: &'a [AccountInfo<'a>],
    reward_rate_per_second: Option<u64>,
    min_stake_amount: Option<u64>,
    lockup_period: Option<i64>,
    is_paused: Option<bool>,
) -> ProgramResult {
    let ctx = UpdatePoolAccounts::context(accounts)?;

    // Load pool
    let mut pool = StakePool::load(ctx.accounts.pool)?;

    // Guards
    assert_signer("authority", ctx.accounts.authority)?;
    assert_same_pubkeys("authority", ctx.accounts.authority, &pool.authority)?;

    // Update fields
    if let Some(rate) = reward_rate_per_second {
        pool.reward_rate_per_second = rate;
    }
    if let Some(min_amount) = min_stake_amount {
        pool.min_stake_amount = min_amount;
    }
    if let Some(lockup) = lockup_period {
        pool.lockup_period = lockup;
    }
    if let Some(paused) = is_paused {
        pool.is_paused = paused;
    }

    pool.save(ctx.accounts.pool)
}

fn fund_rewards<'a>(accounts: &'a [AccountInfo<'a>], amount: u64) -> ProgramResult {
    let ctx = FundRewardsAccounts::context(accounts)?;

    // Load pool
    let pool = StakePool::load(ctx.accounts.pool)?;

    // Guards
    assert_signer("funder", ctx.accounts.funder)?;
    assert_same_pubkeys("reward_vault", ctx.accounts.reward_vault, &pool.reward_vault)?;

    // Transfer reward tokens to pool
    transfer_tokens_with_fee(
        ctx.accounts.funder_token_account,
        ctx.accounts.reward_vault,
        ctx.accounts.funder,
        ctx.accounts.token_program,
        amount,
        &[],
    )?;

    msg!("Funded pool with {} reward tokens", amount);
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
