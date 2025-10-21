use solana_program::{account_info::AccountInfo, program_error::ProgramError};
use spl_token_2022::{extension::StateWithExtensions, state::Account as TokenAccount};

use crate::error::StakePoolError;
use solana_program::pubkey::Pubkey;

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

/// Get the balance of a token account
pub fn get_token_account_balance(token_account: &AccountInfo) -> Result<u64, ProgramError> {
    let account_data = token_account.try_borrow_data()?;
    let account = StateWithExtensions::<TokenAccount>::unpack(&account_data)
        .map_err(|_| StakePoolError::InvalidTokenProgram)?;
    Ok(account.base.amount)
}
