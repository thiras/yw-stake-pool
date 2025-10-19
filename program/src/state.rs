use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::error::StakePoolError;

/// Generic function to validate and deserialize account data
/// This prevents the UnvalidatedAccount vulnerability by ensuring:
/// 1. Account is owned by this program
/// 2. Account has data
/// 3. Account data meets minimum size requirements
fn validate_and_deserialize<T: BorshDeserialize>(
    account: &AccountInfo,
    account_type_name: &str,
) -> Result<T, ProgramError> {
    // Validate account ownership
    if account.owner != &crate::ID {
        msg!(
            "{} account not owned by this program. Owner: {}",
            account_type_name,
            account.owner
        );
        return Err(ProgramError::IllegalOwner);
    }

    // Ensure account has data
    if account.data_is_empty() {
        msg!("{} account data is empty", account_type_name);
        return Err(ProgramError::UninitializedAccount);
    }

    let data = account.data.borrow();

    // Check minimum size (at least 1 byte for Key discriminator)
    if data.is_empty() {
        msg!("{} account data too short", account_type_name);
        return Err(ProgramError::InvalidAccountData);
    }

    // Deserialize and validate Key discriminator
    let mut bytes: &[u8] = &data;
    let deserialized = T::deserialize(&mut bytes).map_err(|error| {
        msg!("{} deserialization error: {}", account_type_name, error);
        StakePoolError::DeserializationError
    })?;

    // Verify the account type matches expected (discriminator check)
    // This is done after deserialization to access the key field
    // Note: We rely on borsh deserialization failing if the structure doesn't match

    Ok(deserialized)
}

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
    /// Pending authority for two-step authority transfer (None if no transfer pending)
    pub pending_authority: Option<Pubkey>,
    /// Optional pool end date (Unix timestamp). If set, no new stakes allowed after this time.
    /// None means the pool runs indefinitely.
    pub pool_end_date: Option<i64>,
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
    // Size calculation:
    // - key (Key enum): 1 byte
    // - authority (Pubkey): 32 bytes
    // - stake_mint (Pubkey): 32 bytes
    // - reward_mint (Pubkey): 32 bytes
    // - stake_vault (Pubkey): 32 bytes
    // - reward_vault (Pubkey): 32 bytes
    // - total_staked (u64): 8 bytes
    // - reward_rate (u64): 8 bytes
    // - min_stake_amount (u64): 8 bytes
    // - lockup_period (i64): 8 bytes
    // - is_paused (bool): 1 byte
    // - bump (u8): 1 byte
    // - pending_authority (Option<Pubkey>): 1 byte when None, 33 bytes when Some
    // - pool_end_date (Option<i64>): 1 byte when None, 9 bytes when Some
    //
    // We allocate for the maximum size (both Options as Some) to support future updates
    // None: 1 + 32*5 + 8*3 + 1*2 + 1 + 1 = 197 bytes
    // Some: 1 + 32*5 + 8*3 + 1*2 + 33 + 9 = 237 bytes
    pub const LEN: usize = 1 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1 + 33 + 9;

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
        let pool = validate_and_deserialize::<Self>(account, "StakePool")?;

        // Verify discriminator matches expected type
        if !matches!(pool.key, Key::StakePool) {
            msg!("Invalid StakePool discriminator");
            return Err(StakePoolError::InvalidAccountDiscriminator.into());
        }

        Ok(pool)
    }

    pub fn save(&self, account: &AccountInfo) -> ProgramResult {
        // Serialize to a vec first to get the exact size
        let serialized = borsh::to_vec(self).map_err(|error| {
            msg!("Serialization error: {}", error);
            ProgramError::from(StakePoolError::SerializationError)
        })?;

        // Zero-fill the account data
        let mut data = account.data.borrow_mut();
        data[..].fill(0);

        // Copy serialized data
        data[..serialized.len()].copy_from_slice(&serialized);

        Ok(())
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
        let stake_account = validate_and_deserialize::<Self>(account, "StakeAccount")?;

        // Verify discriminator matches expected type
        if !matches!(stake_account.key, Key::StakeAccount) {
            msg!("Invalid StakeAccount discriminator");
            return Err(StakePoolError::InvalidAccountDiscriminator.into());
        }

        Ok(stake_account)
    }

    pub fn save(&self, account: &AccountInfo) -> ProgramResult {
        // Serialize to a vec first to get the exact size
        let serialized = borsh::to_vec(self).map_err(|error| {
            msg!("Serialization error: {}", error);
            ProgramError::from(StakePoolError::SerializationError)
        })?;

        // Zero-fill the account data
        let mut data = account.data.borrow_mut();
        data[..].fill(0);

        // Copy serialized data
        data[..serialized.len()].copy_from_slice(&serialized);

        Ok(())
    }
}
