use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use spl_token_2022::{
    extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions},
    state::{Account as TokenAccount, Mint},
};

use crate::error::StakePoolError;

/// Minimum valid Unix timestamp (Jan 1, 2021)
/// Timestamps before this indicate clock misconfiguration for this modern blockchain
pub const MIN_VALID_TIMESTAMP: i64 = 1609459200;

/// Validates that a timestamp is reasonable for current time from Clock::get()
///
/// # Errors
/// Returns InvalidTimestamp if the timestamp is before MIN_VALID_TIMESTAMP
pub fn validate_current_timestamp(timestamp: i64) -> Result<(), ProgramError> {
    if timestamp < MIN_VALID_TIMESTAMP {
        msg!("Invalid system time (before 2021-01-01): {}", timestamp);
        return Err(StakePoolError::InvalidTimestamp.into());
    }
    Ok(())
}

/// Validates that a stored timestamp is reasonable and not in the future
///
/// This checks for both data corruption (stored timestamp is too old) and
/// clock manipulation (stored timestamp is in the future compared to current time)
///
/// # Errors
/// Returns InvalidTimestamp if:
/// - The stored timestamp is before MIN_VALID_TIMESTAMP (data corruption)
/// - The stored timestamp is greater than current time (clock manipulation)
pub fn validate_stored_timestamp(stored: i64, current: i64) -> Result<(), ProgramError> {
    if stored < MIN_VALID_TIMESTAMP {
        msg!(
            "Data corruption detected: stored timestamp is before 2021-01-01: {}",
            stored
        );
        return Err(StakePoolError::InvalidTimestamp.into());
    }
    if stored > current {
        msg!(
            "Invalid timestamp: stored {} is in the future (current: {}). Possible clock manipulation.",
            stored,
            current
        );
        return Err(StakePoolError::InvalidTimestamp.into());
    }
    Ok(())
}

/// Validates that a Token-2022 mint does not have dangerous extensions enabled.
///
/// # Security [M-02]
/// Token-2022 introduces powerful extensions that can break protocol invariants if not validated.
/// This function checks for and rejects mints with the following dangerous extensions:
///
/// 1. **TransferHook**: Custom logic on transfers that can:
///    - Block transfers entirely, preventing unstaking
///    - Redirect tokens to different accounts
///    - Cause reentrancy issues
///
/// 2. **PermanentDelegate**: Allows forcible transfer of tokens from any account,
///    including the protocol's vaults, bypassing all authorization checks.
///
/// 3. **MintCloseAuthority**: Allows closing the mint account, rendering all
///    staked tokens worthless and breaking the protocol entirely.
///
/// 4. **DefaultAccountState (Frozen)**: Accounts could be created in frozen state,
///    preventing any token movement.
///
/// Note: TransferFeeConfig is now properly supported as transfer_tokens_with_fee()
/// correctly determines the actual transferred amount after fees via balance checking.
///
/// # Arguments
/// * `mint_account` - The mint account to validate (can be Token or Token-2022)
/// * `mint_name` - Name for error messages (e.g., "stake_mint" or "reward_mint")
///
/// # Returns
/// * `Ok(())` if the mint is safe to use (no dangerous extensions)
/// * `Err(ProgramError)` if dangerous extensions are detected
pub fn validate_token_extensions(
    mint_account: &AccountInfo,
    mint_name: &str,
) -> Result<(), ProgramError> {
    let account_data = mint_account.try_borrow_data()?;

    // Try to unpack as Token-2022 mint with extensions
    let mint_with_extensions = StateWithExtensions::<Mint>::unpack(&account_data)
        .map_err(|_| StakePoolError::InvalidTokenProgram)?;

    // Get all extension types present on this mint
    let extension_types = mint_with_extensions.get_extension_types()?;

    // List of dangerous extensions that break protocol security
    const DANGEROUS_EXTENSIONS: &[ExtensionType] = &[
        ExtensionType::TransferHook,
        ExtensionType::PermanentDelegate,
        ExtensionType::MintCloseAuthority,
        ExtensionType::DefaultAccountState,
    ];

    // Check if any dangerous extensions are present
    for extension_type in extension_types {
        if DANGEROUS_EXTENSIONS.contains(&extension_type) {
            msg!(
                "Security Error [M-02]: {} has dangerous Token-2022 extension: {:?}",
                mint_name,
                extension_type
            );
            msg!("This extension can break protocol invariants and is not allowed.");
            return Err(StakePoolError::UnsafeTokenExtension.into());
        }
    }

    Ok(())
}

/// Verify that a token account belongs to the expected mint and optionally validate Token-2022 extensions.
///
/// # Security [M-02]
/// This function performs two critical security checks:
/// 1. Verifies the token account belongs to the expected mint
/// 2. (Optional) Validates that Token-2022 mints don't have dangerous extensions that could break protocol
///
/// # Arguments
/// * `token_account` - The token account to verify (vault or user account)
/// * `expected_mint` - The expected mint pubkey
/// * `mint_account` - Optional mint account to validate for dangerous Token-2022 extensions.
///                     Should be provided during pool initialization to ensure safety.
///                     Can be None for runtime operations after the pool is already validated.
/// * `mint_name` - Optional name for error messages (required if mint_account is Some)
///
/// # Returns
/// * `Ok(())` if validation passes
/// * `Err(ProgramError)` if mint mismatch or dangerous extensions detected
pub fn verify_token_account(
    token_account: &AccountInfo,
    expected_mint: &Pubkey,
    mint_account: Option<&AccountInfo>,
    mint_name: Option<&str>,
) -> Result<(), ProgramError> {
    let account_data = token_account.try_borrow_data()?;

    // Support both Token and Token-2022
    let account = StateWithExtensions::<TokenAccount>::unpack(&account_data)
        .map_err(|_| StakePoolError::InvalidTokenProgram)?;

    if &account.base.mint != expected_mint {
        return Err(StakePoolError::InvalidMint.into());
    }

    // [M-02] Security Fix: Validate Token-2022 extensions during pool initialization
    // This prevents pools from being created with malicious Token-2022 mints that have
    // dangerous extensions like TransferHook, PermanentDelegate, MintCloseAuthority, or DefaultAccountState.
    // Extension validation is only needed at initialization - runtime operations can skip it
    // since the pool has already been validated.
    // Note: TransferFeeConfig is properly supported and not blocked.
    if let (Some(mint_acc), Some(name)) = (mint_account, mint_name) {
        validate_token_extensions(mint_acc, name)?;
    }

    Ok(())
}

/// Verify that a token account is owned by the expected owner (typically a PDA)
/// This is critical for security to prevent attackers from passing token accounts they control
/// as pool vaults during initialization or other operations.
///
/// # Security
/// Without this validation, an attacker could:
/// - Pass their own token account as the pool vault during initialization
/// - Steal all user deposits since the pool would transfer tokens to attacker's account
/// - Lock funds permanently since vault addresses cannot be changed after initialization
///
/// # Arguments
/// * `token_account` - The token account to validate
/// * `expected_owner` - The expected owner pubkey (should be the pool PDA)
/// * `account_name` - Name for error messaging (e.g., "stake_vault")
pub fn verify_vault_ownership(
    token_account: &AccountInfo,
    expected_owner: &Pubkey,
    account_name: &str,
) -> Result<(), ProgramError> {
    let account_data = token_account.try_borrow_data()?;

    // Support both Token and Token-2022
    let account = StateWithExtensions::<TokenAccount>::unpack(&account_data)
        .map_err(|_| StakePoolError::InvalidTokenProgram)?;

    // Critical security check: verify the token account owner matches expected owner
    if &account.base.owner != expected_owner {
        msg!(
            "Security Error: {} token account owner mismatch. Expected: {}, Got: {}",
            account_name,
            expected_owner,
            account.base.owner
        );
        return Err(StakePoolError::InvalidVaultOwner.into());
    }

    Ok(())
}

/// Get the balance of a token account
pub fn get_token_account_balance(token_account: &AccountInfo) -> Result<u64, ProgramError> {
    let account_data = token_account.try_borrow_data()?;
    let account = StateWithExtensions::<TokenAccount>::unpack(&account_data)
        .map_err(|_| StakePoolError::InvalidTokenProgram)?;
    Ok(account.base.amount)
}

/// Validates that a mint does not have a freeze authority set.
///
/// # Security [M-03]
/// The freeze_authority in an SPL token mint allows a designated account to freeze any
/// token account holding that token, preventing all transfers. If a pool is created with
/// a mint that has a freeze authority, the authority holder can unilaterally and permanently
/// freeze the stake accounts of any user who deposits into the pool, rendering their funds
/// inaccessible.
///
/// This validation must be performed during pool initialization to protect users from:
/// - Permanent loss of funds via account freezing
/// - Malicious pool creators who can lock user deposits at will
/// - Centralized control over user assets
///
/// # Arguments
/// * `mint_account` - The mint account to validate (can be Token or Token-2022)
/// * `mint_name` - Name for error messages (e.g., "stake_mint" or "reward_mint")
///
/// # Returns
/// * `Ok(())` if the mint has no freeze authority (safe to use)
/// * `Err(ProgramError)` if a freeze authority is set
pub fn validate_no_freeze_authority(
    mint_account: &AccountInfo,
    mint_name: &str,
) -> Result<(), ProgramError> {
    let account_data = mint_account.try_borrow_data()?;

    // Try to unpack as Token/Token-2022 mint with extensions
    let mint_with_extensions = StateWithExtensions::<Mint>::unpack(&account_data)
        .map_err(|_| StakePoolError::InvalidTokenProgram)?;

    // Check if freeze authority is set
    if mint_with_extensions.base.freeze_authority.is_some() {
        msg!(
            "Security Error [M-03]: {} has a freeze authority set",
            mint_name
        );
        msg!(
            "Freeze authority: {:?}",
            mint_with_extensions.base.freeze_authority
        );
        msg!("This allows locking user funds and is not allowed for pool mints.");
        return Err(StakePoolError::MintHasFreezeAuthority.into());
    }

    Ok(())
}
