use solana_program::{account_info::AccountInfo, program_error::ProgramError};
use spl_token_2022::{extension::StateWithExtensions, state::Account as TokenAccount};

use crate::error::StakePoolError;
use solana_program::{msg, pubkey::Pubkey};

/// Verify that a token account belongs to the expected mint
pub fn verify_token_account(
    token_account: &AccountInfo,
    expected_mint: &Pubkey,
) -> Result<(), ProgramError> {
    let account_data = token_account.try_borrow_data()?;

    // Support both Token and Token-2022
    let account = StateWithExtensions::<TokenAccount>::unpack(&account_data)
        .map_err(|_| StakePoolError::InvalidTokenProgram)?;

    if &account.base.mint != expected_mint {
        return Err(StakePoolError::InvalidMint.into());
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
