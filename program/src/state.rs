use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::error::StakePoolError;

/// Helper function to safely write serialized data to an account with size validation
/// This prevents silent data truncation if new fields are added in future versions
fn save_account_data<T: BorshSerialize>(
    account: &AccountInfo,
    data: &T,
    account_type: &str,
) -> ProgramResult {
    // Serialize to a vec first to get the exact size
    let serialized = borsh::to_vec(data).map_err(|error| {
        msg!("{} serialization error: {}", account_type, error);
        ProgramError::from(StakePoolError::SerializationError)
    })?;

    // Defensive check: Ensure account is large enough for serialized data
    let account_size = account.data_len();
    if serialized.len() > account_size {
        msg!(
            "{} account size too small: need {} bytes, have {} bytes",
            account_type,
            serialized.len(),
            account_size
        );
        return Err(StakePoolError::AccountSizeTooSmall.into());
    }

    // Zero-fill the account data
    let mut account_data = account.data.borrow_mut();
    account_data[..].fill(0);

    // Copy serialized data
    account_data[..serialized.len()].copy_from_slice(&serialized);

    Ok(())
}

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
    ProgramAuthority,
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
    /// Unique identifier to allow multiple pools for same authority + stake_mint
    pub pool_id: u64,
    /// The pool's stake token vault
    pub stake_vault: Pubkey,
    /// The pool's reward token vault
    pub reward_vault: Pubkey,
    /// Total amount staked in the pool
    pub total_staked: u64,
    /// Total rewards owed to all stakers (to prevent over-allocation)
    pub total_rewards_owed: u64,
    /// Fixed reward rate as a percentage (scaled by 1e9, e.g., 10_000_000_000 = 10% reward after lockup)
    pub reward_rate: u64,
    /// Minimum stake amount
    pub min_stake_amount: u64,
    /// Lockup period in seconds (0 for no lockup)
    pub lockup_period: i64,
    /// Whether the pool is paused
    pub is_paused: bool,
    /// Whether to enforce lockup period (prevent early withdrawals)
    pub enforce_lockup: bool,
    /// Bump seed for PDA derivation
    pub bump: u8,
    /// Pending authority for two-step authority transfer (None if no transfer pending)
    pub pending_authority: Option<Pubkey>,
    /// Optional pool end date (Unix timestamp). If set, no new stakes allowed after this time.
    /// None means the pool runs indefinitely.
    pub pool_end_date: Option<i64>,
    /// Pending reward rate change (None if no change pending)
    /// Set by update_pool, applied by finalize_reward_rate_change after delay
    ///
    /// BREAKING CHANGE [L-01 Security Fix]:
    /// These two fields (pending_reward_rate + reward_rate_change_timestamp) were added
    /// to implement time-locked reward rate changes with a 7-day delay, preventing
    /// centralized surprise changes to reward rates.
    ///
    /// COMPATIBILITY WARNING:
    /// This change BREAKS compatibility with existing deployed pools. The old structure
    /// had _reserved: [u8; 32] after pool_end_date, but the new structure adds two new
    /// Option fields before reducing reserved space to 16 bytes.
    ///
    /// When deserializing existing pool accounts:
    /// - Old structure: pool_end_date + [u8; 32] reserved
    /// - New structure: pool_end_date + Option<u64> + Option<i64> + [u8; 16] reserved
    ///
    /// The first 1-2 bytes of the old reserved space will be misinterpreted as Option
    /// discriminators for the new fields, causing data corruption.
    ///
    /// MIGRATION REQUIRED:
    /// If you have existing pools deployed, you MUST:
    /// 1. Drain all stakes and rewards from existing pools
    /// 2. Close existing pool accounts
    /// 3. Redeploy the new program version
    /// 4. Recreate pools with new structure
    ///
    /// This is a fresh devnet deployment with no production pools, so the breaking
    /// change is acceptable. DO NOT deploy this to a cluster with existing pools
    /// without proper migration.
    pub pending_reward_rate: Option<u64>,
    /// Timestamp when pending reward rate change was proposed
    /// Used to enforce REWARD_RATE_CHANGE_DELAY before finalizing
    /// Must always be in sync with pending_reward_rate (both Some or both None)
    pub reward_rate_change_timestamp: Option<i64>,
    /// Reserved space for future use. Not currently used.
    /// This field allows for future upgrades without breaking compatibility.
    /// REDUCED from 32 bytes to 16 bytes to accommodate new time-lock fields.
    pub _reserved: [u8; 16],
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
    // - pool_id (u64): 8 bytes
    // - stake_vault (Pubkey): 32 bytes
    // - reward_vault (Pubkey): 32 bytes
    // - total_staked (u64): 8 bytes
    // - total_rewards_owed (u64): 8 bytes
    // - reward_rate (u64): 8 bytes
    // - min_stake_amount (u64): 8 bytes
    // - lockup_period (i64): 8 bytes
    // - is_paused (bool): 1 byte
    // - enforce_lockup (bool): 1 byte
    // - bump (u8): 1 byte
    // - pending_authority (Option<Pubkey>): 1 byte when None, 33 bytes when Some
    // - pool_end_date (Option<i64>): 1 byte when None, 9 bytes when Some
    // - pending_reward_rate (Option<u64>): 1 byte when None, 9 bytes when Some
    // - reward_rate_change_timestamp (Option<i64>): 1 byte when None, 9 bytes when Some
    // - _reserved: 16 bytes
    //
    // We allocate for the maximum size (all Options as Some) to support future updates
    // None: 1 + 160 + 40 + 8 + 3 + 1 + 1 + 1 + 1 + 16 = 232 bytes
    // Some: 1 + 160 + 40 + 8 + 3 + 33 + 9 + 9 + 9 + 16 = 288 bytes
    pub const LEN: usize =
        1 + 32 + 32 + 32 + 8 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 1 + 1 + 1 + 33 + 9 + 9 + 9 + 16;

    pub fn seeds(authority: &Pubkey, stake_mint: &Pubkey, pool_id: u64) -> Vec<Vec<u8>> {
        vec![
            b"stake_pool".to_vec(),
            authority.as_ref().to_vec(),
            stake_mint.as_ref().to_vec(),
            pool_id.to_le_bytes().to_vec(),
        ]
    }

    pub fn find_pda(authority: &Pubkey, stake_mint: &Pubkey, pool_id: u64) -> (Pubkey, u8) {
        let pool_id_bytes = pool_id.to_le_bytes();
        let seeds: Vec<&[u8]> = vec![
            b"stake_pool",
            authority.as_ref(),
            stake_mint.as_ref(),
            &pool_id_bytes,
        ];
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
        save_account_data(account, self, "StakePool")
    }

    /// Calculate rewards for a stake based on fixed reward rate
    ///
    /// # Security Fix [H-02]: Minimum Lockup Period
    /// Rewards are only earned if the lockup period is complete. The attack vector
    /// of setting trivially short lockup periods (e.g., 1 second) is prevented by
    /// MIN_LOCKUP_PERIOD validation in initialize_pool, which requires at least 1 day.
    ///
    /// # Reward Model
    /// - Binary distribution: 0% before lockup completes, 100% after
    /// - Formula: (amount * reward_rate) / 1e9
    /// - Users must wait full lockup period before earning any rewards
    ///
    /// # Arguments
    /// * `amount_staked` - Amount of tokens staked
    /// * `stake_timestamp` - Unix timestamp when stake was created
    /// * `current_time` - Current Unix timestamp
    ///
    /// # Returns
    /// The reward amount: 0 if lockup incomplete, full reward if lockup complete
    ///
    /// # Example
    /// - reward_rate = 100_000_000 (10% when scaled by 1e9)
    /// - lockup_period = 86400 seconds (1 day)
    /// - amount_staked = 1000 tokens
    /// - Before 24 hours: 0 tokens
    /// - After 24 hours: 100 tokens (full reward)
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
        // reward_rate is scaled by 1e9 (e.g., 100_000_000 = 10% of staked amount)
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
        save_account_data(account, self, "StakeAccount")
    }
}

/// Program authority configuration for managing pool creation permissions
/// This account controls who can create new stake pools
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, ShankAccount)]
pub struct ProgramAuthority {
    pub key: Key,
    /// The main authority who can manage the authorized creators list
    pub authority: Pubkey,
    /// List of addresses authorized to create pools
    /// Using a fixed-size array for predictable memory layout
    pub authorized_creators: [Option<Pubkey>; 10],
    /// Number of active authorized creators (for iteration)
    pub creator_count: u8,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl ProgramAuthority {
    // Size calculation:
    // - key (Key enum): 1 byte
    // - authority (Pubkey): 32 bytes
    // - authorized_creators (10 x Option<Pubkey>): 10 * 33 = 330 bytes (1 byte discriminator + 32 bytes pubkey)
    // - creator_count (u8): 1 byte
    // - bump (u8): 1 byte
    // Total: 1 + 32 + 330 + 1 + 1 = 365 bytes
    pub const LEN: usize = 1 + 32 + (10 * 33) + 1 + 1;
    pub const MAX_CREATORS: usize = 10;

    pub fn seeds() -> Vec<Vec<u8>> {
        vec![b"program_authority".to_vec()]
    }

    pub fn find_pda() -> (Pubkey, u8) {
        let seeds: Vec<&[u8]> = vec![b"program_authority"];
        Pubkey::find_program_address(&seeds, &crate::ID)
    }

    pub fn load(account: &AccountInfo) -> Result<Self, ProgramError> {
        let program_authority = validate_and_deserialize::<Self>(account, "ProgramAuthority")?;

        // Verify discriminator matches expected type
        if !matches!(program_authority.key, Key::ProgramAuthority) {
            msg!("Invalid ProgramAuthority discriminator");
            return Err(StakePoolError::InvalidAccountDiscriminator.into());
        }

        Ok(program_authority)
    }

    pub fn save(&self, account: &AccountInfo) -> ProgramResult {
        save_account_data(account, self, "ProgramAuthority")
    }

    /// Check if a given pubkey is authorized to create pools
    pub fn is_authorized(&self, pubkey: &Pubkey) -> bool {
        // Main authority is always authorized
        if pubkey == &self.authority {
            return true;
        }

        // Check authorized creators list
        for creator in &self.authorized_creators {
            if let Some(authorized) = creator {
                if authorized == pubkey {
                    return true;
                }
            }
        }

        false
    }

    /// Add a new authorized creator
    pub fn add_creator(&mut self, creator: Pubkey) -> Result<(), ProgramError> {
        // Check if already exists
        for existing_creator in &self.authorized_creators {
            if let Some(authorized) = existing_creator {
                if authorized == &creator {
                    msg!("Creator already authorized: {}", creator);
                    return Err(StakePoolError::CreatorAlreadyAuthorized.into());
                }
            }
        }

        // Find empty slot
        for slot in &mut self.authorized_creators {
            if slot.is_none() {
                *slot = Some(creator);
                self.creator_count = self
                    .creator_count
                    .checked_add(1)
                    .ok_or(StakePoolError::NumericalOverflow)?;
                return Ok(());
            }
        }

        // No empty slots
        msg!("Maximum number of authorized creators reached");
        Err(StakePoolError::MaxAuthorizedCreatorsReached.into())
    }

    /// Remove an authorized creator
    pub fn remove_creator(&mut self, creator: &Pubkey) -> Result<(), ProgramError> {
        // Cannot remove the main authority
        if creator == &self.authority {
            msg!("Cannot remove main authority from authorized creators");
            return Err(StakePoolError::CannotRemoveMainAuthority.into());
        }

        // Find and remove
        for slot in &mut self.authorized_creators {
            if let Some(authorized) = slot {
                if authorized == creator {
                    *slot = None;
                    self.creator_count = self
                        .creator_count
                        .checked_sub(1)
                        .ok_or(StakePoolError::NumericalOverflow)?;
                    return Ok(());
                }
            }
        }

        // Not found
        msg!("Creator not found in authorized list: {}", creator);
        Err(StakePoolError::CreatorNotFound.into())
    }
}
