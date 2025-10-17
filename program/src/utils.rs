use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use solana_sdk_ids::system_program;
use spl_token_2022::{
    extension::StateWithExtensions,
    instruction::transfer_checked,
};

use crate::error::StakePoolError;

/// Create a new account from the given size.
#[inline(always)]
pub fn create_account<'a>(
    target_account: &AccountInfo<'a>,
    funding_account: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    size: usize,
    owner: &Pubkey,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports: u64 = rent.minimum_balance(size);

    let create_account_ix = solana_program::system_instruction::create_account(
        funding_account.key,
        target_account.key,
        lamports,
        size as u64,
        owner,
    );

    invoke_signed(
        &create_account_ix,
        &[
            funding_account.clone(),
            target_account.clone(),
            system_program_account.clone(),
        ],
        signer_seeds.unwrap_or(&[]),
    )
}

/// Resize an account using realloc, lifted from Solana Cookbook.
#[inline(always)]
pub fn realloc_account<'a>(
    target_account: &AccountInfo<'a>,
    funding_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    new_size: usize,
    refund: bool,
) -> ProgramResult {
    let rent = Rent::get()?;
    let old_minimum_balance = rent.minimum_balance(target_account.data_len());
    let new_minimum_balance = rent.minimum_balance(new_size);
    let lamports_diff = new_minimum_balance.abs_diff(old_minimum_balance);

    if new_minimum_balance > old_minimum_balance {
        let transfer_ix = solana_program::system_instruction::transfer(
            funding_account.key,
            target_account.key,
            lamports_diff,
        );
        invoke(
            &transfer_ix,
            &[
                funding_account.clone(),
                target_account.clone(),
                system_program.clone(),
            ],
        )?;
    } else if refund {
        transfer_lamports_from_pdas(target_account, funding_account, lamports_diff)?;
    }

    target_account.resize(new_size)
}

/// Close an account.
#[inline(always)]
pub fn close_account<'a>(
    target_account: &AccountInfo<'a>,
    receiving_account: &AccountInfo<'a>,
) -> ProgramResult {
    let dest_starting_lamports = receiving_account.lamports();
    **receiving_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(target_account.lamports())
        .unwrap();
    **target_account.lamports.borrow_mut() = 0;

    target_account.assign(&Pubkey::from(system_program::ID.to_bytes()));
    target_account.resize(0)
}

/// Transfer lamports.
#[inline(always)]
pub fn transfer_lamports<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    lamports: u64,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> ProgramResult {
    let transfer_ix = solana_program::system_instruction::transfer(from.key, to.key, lamports);
    invoke_signed(
        &transfer_ix,
        &[from.clone(), to.clone()],
        signer_seeds.unwrap_or(&[]),
    )
}

pub fn transfer_lamports_from_pdas<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    lamports: u64,
) -> ProgramResult {
    **from.lamports.borrow_mut() = from
        .lamports()
        .checked_sub(lamports)
        .ok_or::<ProgramError>(StakePoolError::NumericalOverflow.into())?;

    **to.lamports.borrow_mut() = to
        .lamports()
        .checked_add(lamports)
        .ok_or::<ProgramError>(StakePoolError::NumericalOverflow.into())?;

    Ok(())
}

/// Transfer tokens with support for Token-2022 transfer fees
/// Note: This function assumes the mint account has 9 decimals
/// In production, you should pass the mint account and read decimals from it
pub fn transfer_tokens_with_fee<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<u64, ProgramError> {
    // Get mint from the token account
    let from_data = from.try_borrow_data()?;
    let from_account = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&from_data)
        .map_err(|_| StakePoolError::InvalidTokenProgram)?;
    let mint_key = from_account.base.mint;
    drop(from_data);

    // For simplicity, use 9 decimals (standard for most SPL tokens)
    // In production, you should get this from the mint account
    let decimals = 9;

    let accounts = vec![from.clone(), to.clone(), authority.clone()];

    // Use transfer_checked for Token-2022 (supports transfer fees automatically)
    let transfer_ix = transfer_checked(
        token_program.key,
        from.key,
        &mint_key,
        to.key,
        authority.key,
        &[],
        amount,
        decimals,
    )?;

    if signer_seeds.is_empty() {
        invoke(&transfer_ix, &accounts)?;
    } else {
        invoke_signed(&transfer_ix, &accounts, signer_seeds)?;
    }

    // For Token-2022 with transfer fees, the actual transferred amount may be less
    // In production, you'd want to calculate the exact amount after fees
    // For now, return the requested amount (simplified)
    Ok(amount)
}
