use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    sysvar::{clock::Clock, Sysvar},
};

use crate::assertions::*;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::state::{Key, StakePool};
use crate::utils::create_account;
use solana_program::pubkey::Pubkey;

use super::helpers::verify_token_account;

pub fn initialize_pool<'a>(
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
