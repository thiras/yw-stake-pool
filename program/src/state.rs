use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::error::StakePoolError;

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub enum Key {
    Uninitialized,
    StakePool,
    StakeAccount,
}

/// The main stake pool configuration
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, ShankAccount)]
pub struct StakePool {
    pub key: Key,
    /// The authority that can modify pool settings
    pub authority: Pubkey,
    /// The token mint being staked (supports Token-2022)
    pub stake_mint: Pubkey,
    /// The token mint for rewards (supports Token-2022)
    pub reward_mint: Pubkey,
    /// The pool's stake token vault
    pub stake_vault: Pubkey,
    /// The pool's reward token vault
    pub reward_vault: Pubkey,
    /// Total amount staked in the pool
    pub total_staked: u64,
    /// Reward rate per second (scaled by 1e9)
    pub reward_rate_per_second: u64,
    /// Last time rewards were updated
    pub last_update_time: i64,
    /// Accumulated rewards per token (scaled by 1e18)
    pub reward_per_token_stored: u128,
    /// Minimum stake amount
    pub min_stake_amount: u64,
    /// Lockup period in seconds (0 for no lockup)
    pub lockup_period: i64,
    /// Whether the pool is paused
    pub is_paused: bool,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

/// Individual user stake account
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, ShankAccount)]
pub struct StakeAccount {
    pub key: Key,
    /// The stake pool this account belongs to
    pub pool: Pubkey,
    /// The owner of this stake account
    pub owner: Pubkey,
    /// Amount staked
    pub amount_staked: u64,
    /// Reward per token paid to this account
    pub reward_per_token_paid: u128,
    /// Pending rewards not yet claimed
    pub rewards_earned: u64,
    /// Timestamp when stake was deposited
    pub stake_timestamp: i64,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl StakePool {
    pub const LEN: usize = 1 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 16 + 8 + 8 + 1 + 1;

    pub fn seeds(authority: &Pubkey, stake_mint: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            b"stake_pool".to_vec(),
            authority.as_ref().to_vec(),
            stake_mint.as_ref().to_vec(),
        ]
    }

    pub fn find_pda(authority: &Pubkey, stake_mint: &Pubkey) -> (Pubkey, u8) {
        let seeds: Vec<&[u8]> = vec![
            b"stake_pool",
            authority.as_ref(),
            stake_mint.as_ref(),
        ];
        Pubkey::find_program_address(&seeds, &crate::ID)
    }

    pub fn load(account: &AccountInfo) -> Result<Self, ProgramError> {
        let mut bytes: &[u8] = &(*account.data).borrow();
        StakePool::deserialize(&mut bytes).map_err(|error| {
            msg!("Error: {}", error);
            StakePoolError::DeserializationError.into()
        })
    }

    pub fn save(&self, account: &AccountInfo) -> ProgramResult {
        borsh::to_writer(&mut account.data.borrow_mut()[..], self).map_err(|error| {
            msg!("Error: {}", error);
            StakePoolError::SerializationError.into()
        })
    }

    /// Calculate current reward per token
    pub fn reward_per_token(&self, current_time: i64) -> Result<u128, ProgramError> {
        if self.total_staked == 0 {
            return Ok(self.reward_per_token_stored);
        }

        let time_delta = current_time
            .checked_sub(self.last_update_time)
            .ok_or(StakePoolError::NumericalOverflow)?;

        let reward_increase = (self.reward_rate_per_second as u128)
            .checked_mul(time_delta as u128)
            .ok_or(StakePoolError::NumericalOverflow)?
            .checked_mul(1_000_000_000_000_000_000) // Scale by 1e18
            .ok_or(StakePoolError::NumericalOverflow)?
            .checked_div(self.total_staked as u128)
            .ok_or(StakePoolError::NumericalOverflow)?;

        self.reward_per_token_stored
            .checked_add(reward_increase)
            .ok_or(StakePoolError::NumericalOverflow.into())
    }

    /// Calculate earned rewards for a stake amount
    pub fn calculate_earned(
        &self,
        amount_staked: u64,
        reward_per_token_paid: u128,
        rewards_earned: u64,
        current_time: i64,
    ) -> Result<u64, ProgramError> {
        let reward_per_token = self.reward_per_token(current_time)?;

        let reward_diff = reward_per_token
            .checked_sub(reward_per_token_paid)
            .ok_or(StakePoolError::NumericalOverflow)?;

        let new_rewards = (amount_staked as u128)
            .checked_mul(reward_diff)
            .ok_or(StakePoolError::NumericalOverflow)?
            .checked_div(1_000_000_000_000_000_000) // Unscale from 1e18
            .ok_or(StakePoolError::NumericalOverflow)? as u64;

        rewards_earned
            .checked_add(new_rewards)
            .ok_or(StakePoolError::NumericalOverflow.into())
    }
}

impl StakeAccount {
    pub const LEN: usize = 1 + 32 + 32 + 8 + 16 + 8 + 8 + 1;

    pub fn seeds(pool: &Pubkey, owner: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            b"stake_account".to_vec(),
            pool.as_ref().to_vec(),
            owner.as_ref().to_vec(),
        ]
    }

    pub fn find_pda(pool: &Pubkey, owner: &Pubkey) -> (Pubkey, u8) {
        let seeds: Vec<&[u8]> = vec![
            b"stake_account",
            pool.as_ref(),
            owner.as_ref(),
        ];
        Pubkey::find_program_address(&seeds, &crate::ID)
    }

    pub fn load(account: &AccountInfo) -> Result<Self, ProgramError> {
        let mut bytes: &[u8] = &(*account.data).borrow();
        StakeAccount::deserialize(&mut bytes).map_err(|error| {
            msg!("Error: {}", error);
            StakePoolError::DeserializationError.into()
        })
    }

    pub fn save(&self, account: &AccountInfo) -> ProgramResult {
        borsh::to_writer(&mut account.data.borrow_mut()[..], self).map_err(|error| {
            msg!("Error: {}", error);
            StakePoolError::SerializationError.into()
        })
    }
}
