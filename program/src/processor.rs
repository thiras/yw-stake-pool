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
use crate::utils::{create_account, transfer_tokens_with_fee};

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
        } => {
            msg!("Instruction: InitializePool");
            initialize_pool(accounts, reward_rate, min_stake_amount, lockup_period)
        }
        StakePoolInstruction::InitializeStakeAccount { index } => {
            msg!("Instruction: InitializeStakeAccount");
            initialize_stake_account(accounts, index)
        }
        StakePoolInstruction::Stake { amount, index } => {
            msg!("Instruction: Stake");
            stake(accounts, amount, index)
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
            reward_rate,
            min_stake_amount,
            lockup_period,
            is_paused,
        } => {
            msg!("Instruction: UpdatePool");
            update_pool(
                accounts,
                reward_rate,
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
    reward_rate: u64,
    min_stake_amount: u64,
    lockup_period: i64,
) -> ProgramResult {
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
        reward_rate,
        min_stake_amount,
        lockup_period,
        is_paused: false,
        bump,
    };

    pool_data.save(ctx.accounts.pool)
}

fn initialize_stake_account<'a>(accounts: &'a [AccountInfo<'a>], index: u64) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = InitializeStakeAccountAccounts::context(accounts)?;

    // Guards
    let stake_account_seeds =
        StakeAccount::seeds(ctx.accounts.pool.key, ctx.accounts.owner.key, index);
    let stake_seeds_refs: Vec<&[u8]> = stake_account_seeds.iter().map(|s| s.as_slice()).collect();
    let (stake_account_key, bump) = Pubkey::find_program_address(&stake_seeds_refs, &crate::ID);

    assert_same_pubkeys(
        "stake_account",
        ctx.accounts.stake_account,
        &stake_account_key,
    )?;
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

    // Initialize stake account with index
    let stake_account_data = StakeAccount {
        key: Key::StakeAccount,
        pool: *ctx.accounts.pool.key,
        owner: *ctx.accounts.owner.key,
        index,
        amount_staked: 0,
        stake_timestamp: 0,
        claimed_rewards: 0,
        bump,
    };

    stake_account_data.save(ctx.accounts.stake_account)
}

fn stake<'a>(accounts: &'a [AccountInfo<'a>], amount: u64, index: u64) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = StakeAccounts::context(accounts)?;

    // Verify pool account discriminator before loading (Type Cosplay protection)
    assert_account_key("pool", ctx.accounts.pool, Key::StakePool)?;

    // Load pool
    let mut pool_data = StakePool::load(ctx.accounts.pool)?;

    // Guards
    assert_signer("owner", ctx.accounts.owner)?;
    assert_signer("payer", ctx.accounts.payer)?;
    assert_empty("stake_account", ctx.accounts.stake_account)?;
    assert_same_pubkeys(
        "reward_vault",
        ctx.accounts.reward_vault,
        &pool_data.reward_vault,
    )?;

    if pool_data.is_paused {
        return Err(StakePoolError::PoolPaused.into());
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

    // Check if reward vault has sufficient balance to cover the expected rewards
    let reward_vault_balance = get_token_account_balance(ctx.accounts.reward_vault)?;
    if reward_vault_balance < expected_rewards {
        msg!(
            "Insufficient rewards in pool. Required: {}, Available: {}",
            expected_rewards,
            reward_vault_balance
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
        ctx.accounts.owner,
        ctx.accounts.token_program,
        amount,
        &[],
    )?;

    // Update pool total staked
    pool_data.total_staked = pool_data
        .total_staked
        .checked_add(transfer_amount)
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

fn unstake<'a>(accounts: &'a [AccountInfo<'a>], amount: u64) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = UnstakeAccounts::context(accounts)?;

    // Verify account discriminators before loading (Type Cosplay protection)
    assert_account_key("pool", ctx.accounts.pool, Key::StakePool)?;
    assert_account_key(
        "stake_account",
        ctx.accounts.stake_account,
        Key::StakeAccount,
    )?;

    // Load accounts
    let mut pool_data = StakePool::load(ctx.accounts.pool)?;
    let mut stake_account_data = StakeAccount::load(ctx.accounts.stake_account)?;

    // Guards
    assert_signer("owner", ctx.accounts.owner)?;
    assert_same_pubkeys("owner", ctx.accounts.owner, &stake_account_data.owner)?;
    assert_same_pubkeys("pool", ctx.accounts.pool, &stake_account_data.pool)?;

    if stake_account_data.amount_staked < amount {
        return Err(StakePoolError::InsufficientStakedBalance.into());
    }

    // Get current time
    let clock = Clock::from_account_info(ctx.accounts.clock)?;

    // Allow unstaking anytime, but warn if lockup not complete (no rewards will be given)
    let time_staked = clock
        .unix_timestamp
        .checked_sub(stake_account_data.stake_timestamp)
        .ok_or(StakePoolError::NumericalOverflow)?;

    if time_staked < pool_data.lockup_period {
        msg!("Warning: Unstaking before lockup period complete. No rewards will be earned.");
    }

    // Transfer tokens back (with PDA signer)
    let pool_seeds = StakePool::seeds(&pool_data.authority, &pool_data.stake_mint);
    let mut seeds_with_bump = pool_seeds.clone();
    seeds_with_bump.push(vec![pool_data.bump]);
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
    stake_account_data.amount_staked = stake_account_data
        .amount_staked
        .checked_sub(amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    pool_data.total_staked = pool_data
        .total_staked
        .checked_sub(amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // If fully unstaking, reset claimed rewards
    // For partial unstakes, claimed_rewards remains to track what's already been claimed
    if stake_account_data.amount_staked == 0 {
        stake_account_data.claimed_rewards = 0;
        stake_account_data.stake_timestamp = 0;
        msg!("Full unstake - stake account reset");
    }

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

    // Load accounts
    let pool_data = StakePool::load(ctx.accounts.pool)?;
    let mut stake_account_data = StakeAccount::load(ctx.accounts.stake_account)?;

    // Guards
    assert_signer("owner", ctx.accounts.owner)?;
    assert_same_pubkeys("owner", ctx.accounts.owner, &stake_account_data.owner)?;
    assert_same_pubkeys("pool", ctx.accounts.pool, &stake_account_data.pool)?;

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

    // Save updated stake account
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
) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = UpdatePoolAccounts::context(accounts)?;

    // Verify pool account discriminator before loading (Type Cosplay protection)
    assert_account_key("pool", ctx.accounts.pool, Key::StakePool)?;

    // Load pool
    let mut pool_data = StakePool::load(ctx.accounts.pool)?;

    // Guards
    assert_signer("authority", ctx.accounts.authority)?;
    assert_same_pubkeys("authority", ctx.accounts.authority, &pool_data.authority)?;

    // Update fields
    if let Some(rate) = reward_rate {
        pool_data.reward_rate = rate;
    }
    if let Some(min_amount) = min_stake_amount {
        pool_data.min_stake_amount = min_amount;
    }
    if let Some(lockup) = lockup_period {
        pool_data.lockup_period = lockup;
    }
    if let Some(paused) = is_paused {
        pool_data.is_paused = paused;
    }

    pool_data.save(ctx.accounts.pool)
}

fn fund_rewards<'a>(accounts: &'a [AccountInfo<'a>], amount: u64) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = FundRewardsAccounts::context(accounts)?;

    // Verify pool account discriminator before loading (Type Cosplay protection)
    assert_account_key("pool", ctx.accounts.pool, Key::StakePool)?;

    // Load pool
    let pool_data = StakePool::load(ctx.accounts.pool)?;

    // Guards
    assert_signer("funder", ctx.accounts.funder)?;
    assert_same_pubkeys(
        "reward_vault",
        ctx.accounts.reward_vault,
        &pool_data.reward_vault,
    )?;

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
