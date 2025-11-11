use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    sysvar::{clock::Clock, Sysvar},
};

use crate::assertions::*;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::state::{Key, StakeAccount, StakePool};
use crate::utils::transfer_tokens_with_fee;

use super::helpers::{get_token_account_balance, verify_token_account};

pub fn claim_rewards<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
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
    verify_token_account(
        ctx.accounts.user_reward_account,
        &pool_data.reward_mint,
        None,
        None,
    )?;
    verify_token_account(
        ctx.accounts.reward_vault,
        &pool_data.reward_mint,
        None,
        None,
    )?;

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
    let pool_seeds = StakePool::seeds(
        &pool_data.authority,
        &pool_data.stake_mint,
        pool_data.pool_id,
    );
    let mut seeds_with_bump = pool_seeds.clone();
    seeds_with_bump.push(vec![pool_data.bump]);
    let seeds_refs: Vec<&[u8]> = seeds_with_bump.iter().map(|s| s.as_slice()).collect();

    // Transfer rewards (with PDA signer)
    // Capture actual amount transferred in case of transfer fees
    let actual_amount = transfer_tokens_with_fee(
        ctx.accounts.reward_vault,
        ctx.accounts.user_reward_account,
        ctx.accounts.reward_mint,
        ctx.accounts.pool,
        ctx.accounts.token_program,
        unclaimed_rewards,
        &[&seeds_refs],
    )?;

    // Update claimed rewards tracking with actual amount transferred
    stake_account_data.claimed_rewards = stake_account_data
        .claimed_rewards
        .checked_add(actual_amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // Update pool's total rewards owed (these rewards have now been paid out)
    pool_data.total_rewards_owed = pool_data
        .total_rewards_owed
        .checked_sub(actual_amount)
        .ok_or(StakePoolError::NumericalOverflow)?;

    // Save updated accounts
    pool_data.save(ctx.accounts.pool)?;
    stake_account_data.save(ctx.accounts.stake_account)?;

    msg!(
        "Claimed {} reward tokens (total claimed: {})",
        actual_amount,
        stake_account_data.claimed_rewards
    );

    Ok(())
}

pub fn fund_rewards<'a>(accounts: &'a [AccountInfo<'a>], amount: u64) -> ProgramResult {
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
    verify_token_account(
        ctx.accounts.funder_token_account,
        &pool_data.reward_mint,
        None,
        None,
    )?;
    verify_token_account(
        ctx.accounts.reward_vault,
        &pool_data.reward_mint,
        None,
        None,
    )?;

    // Transfer reward tokens to pool
    // NOTE: Captures actual_amount for accurate logging, but does NOT update pool state.
    //
    // Design Rationale:
    // - total_rewards_owed tracks COMMITTED rewards (via stake operations)
    // - Reward vault balance tracks AVAILABLE rewards (via fund operations)
    // - These are intentionally separate concerns:
    //   * Committed: What the protocol owes to stakers
    //   * Available: What can actually be paid out
    //
    // If transfer fees apply (actual_amount < amount):
    // - Funder pays 'amount' but vault receives 'actual_amount'
    // - This is the funder's responsibility to account for
    // - claim_rewards() checks vault balance before paying out
    // - If vault balance < committed rewards, claims fail with InsufficientRewards
    //
    // This design ensures:
    // 1. Protocol never over-commits rewards (committed tracked separately)
    // 2. Protocol never pays out more than available (runtime balance check)
    // 3. Funders see accurate logs of what was actually deposited
    // 4. No accounting mismatch in protocol state
    let actual_amount = transfer_tokens_with_fee(
        ctx.accounts.funder_token_account,
        ctx.accounts.reward_vault,
        ctx.accounts.reward_mint,
        ctx.accounts.funder,
        ctx.accounts.token_program,
        amount,
        &[],
    )?;

    msg!("Funded pool with {} reward tokens", actual_amount);
    Ok(())
}
