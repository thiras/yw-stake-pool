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
    /// Fixed reward rate as a percentage (scaled by 1e9, e.g., 10_000_000_000 = 10% reward after lockup)
    pub reward_rate: u64,
    /// Minimum stake amount
    pub min_stake_amount: u64,
    /// Lockup period in seconds (0 for no lockup)
    pub lockup_period: i64,
    /// Whether the pool is paused
    pub is_paused: bool,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

/// Individual user stake account (one per deposit)
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, ShankAccount)]
pub struct StakeAccount {
    pub key: Key,
    /// The stake pool this account belongs to
    pub pool: Pubkey,
    /// The owner of this stake account
    pub owner: Pubkey,
    /// The index of this stake account (0, 1, 2, ...)
    pub index: u64,
    /// Amount staked
    pub amount_staked: u64,
    /// Timestamp when stake was deposited
    pub stake_timestamp: i64,
    /// Total rewards already claimed
    pub claimed_rewards: u64,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl StakePool {
    pub const LEN: usize = 1 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1;

    pub fn seeds(authority: &Pubkey, stake_mint: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            b"stake_pool".to_vec(),
            authority.as_ref().to_vec(),
            stake_mint.as_ref().to_vec(),
        ]
    }

    pub fn find_pda(authority: &Pubkey, stake_mint: &Pubkey) -> (Pubkey, u8) {
        let seeds: Vec<&[u8]> = vec![b"stake_pool", authority.as_ref(), stake_mint.as_ref()];
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

    /// Calculate rewards for a stake based on fixed reward rate
    /// Rewards are only earned if lockup period is complete
    /// Formula: (amount * reward_rate) / 1e9
    pub fn calculate_rewards(
        &self,
        amount_staked: u64,
        stake_timestamp: i64,
        current_time: i64,
    ) -> Result<u64, ProgramError> {
        // Check if lockup period is complete
        let time_staked = current_time
            .checked_sub(stake_timestamp)
            .ok_or(StakePoolError::NumericalOverflow)?;

        if time_staked < self.lockup_period {
            // No rewards if lockup not complete
            return Ok(0);
        }

        // Calculate fixed rewards based on reward rate
        // reward_rate is scaled by 1e9 (e.g., 10_000_000_000 = 10% of staked amount)
        const SCALE: u128 = 1_000_000_000;

        let rewards = (amount_staked as u128)
            .checked_mul(self.reward_rate as u128)
            .ok_or(StakePoolError::NumericalOverflow)?
            .checked_div(SCALE)
            .ok_or(StakePoolError::NumericalOverflow)? as u64;

        Ok(rewards)
    }
}

impl StakeAccount {
    pub const LEN: usize = 1 + 32 + 32 + 8 + 8 + 8 + 8 + 1;

    pub fn seeds(pool: &Pubkey, owner: &Pubkey, index: u64) -> Vec<Vec<u8>> {
        vec![
            b"stake_account".to_vec(),
            pool.as_ref().to_vec(),
            owner.as_ref().to_vec(),
            index.to_le_bytes().to_vec(),
        ]
    }

    pub fn find_pda(pool: &Pubkey, owner: &Pubkey, index: u64) -> (Pubkey, u8) {
        let index_bytes = index.to_le_bytes();
        let seeds: Vec<&[u8]> = vec![
            b"stake_account",
            pool.as_ref(),
            owner.as_ref(),
            &index_bytes,
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
