use solana_program::{account_info::AccountInfo, program_error::ProgramError};
use spl_token_2022::{
    extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions},
    state::{Account as TokenAccount, Mint},
};

use crate::error::StakePoolError;
use solana_program::{msg, pubkey::Pubkey};

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
/// correctly calculates the actual transferred amount after fees.
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
    // dangerous extensions like TransferHook, PermanentDelegate, TransferFeeConfig, etc.
    // Extension validation is only needed at initialization - runtime operations can skip it
    // since the pool has already been validated.
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
