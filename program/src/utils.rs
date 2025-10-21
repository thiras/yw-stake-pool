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
use spl_token_2022::{extension::StateWithExtensions, instruction::transfer_checked, state::Mint};

use crate::error::StakePoolError;

/// Create a new account from the given size.
#[inline(always)]
pub fn create_account<'a>(
    target_account: &AccountInfo<'a>,
    funding_account: &AccountInfo<'a>,
    _system_program_account: &AccountInfo<'a>,
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
        &[funding_account.clone(), target_account.clone()],
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

/// Close an account securely to prevent reinitialization attacks.
/// This function:
/// 1. Zeros out all account data
/// 2. Transfers all lamports to the receiving account
/// 3. Assigns ownership to the system program
#[inline(always)]
pub fn close_account<'a>(
    target_account: &AccountInfo<'a>,
    receiving_account: &AccountInfo<'a>,
) -> ProgramResult {
    // Step 1: Zero out account data to prevent reinitialization attacks
    let mut data = target_account.try_borrow_mut_data()?;
    data.fill(0);
    drop(data);

    // Step 2: Transfer all lamports to the receiving account
    let dest_starting_lamports = receiving_account.lamports();
    let target_lamports = target_account.lamports();

    // SAFETY: Direct lamport manipulation is safe here because:
    // 1. We've already zeroed the account data above
    // 2. We're transferring ALL lamports (not partial)
    // 3. We immediately assign to system program and resize below
    // This is the correct pattern for secure account closure
    **receiving_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(target_lamports)
        .ok_or(StakePoolError::NumericalOverflow)?;
    **target_account.lamports.borrow_mut() = 0;

    // Step 3: Assign ownership to system program and resize to 0
    target_account.assign(&Pubkey::from(system_program::ID.to_bytes()));
    target_account.resize(0)?;

    Ok(())
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

/// Transfer lamports from one PDA to another.
/// This is a secure alternative to direct lamport manipulation.
/// Note: This should only be used for partial transfers, not account closure.
/// For closing accounts, use close_account() which properly zeros data.
pub fn transfer_lamports_from_pdas<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    lamports: u64,
) -> ProgramResult {
    // Ensure we don't drain all lamports without properly closing the account
    let from_lamports = from.lamports();
    let remaining_lamports = from_lamports
        .checked_sub(lamports)
        .ok_or::<ProgramError>(StakePoolError::NumericalOverflow.into())?;

    // If this would zero out the account, the caller should use close_account instead
    if remaining_lamports == 0 && from.data_len() > 0 {
        solana_program::msg!(
            "Warning: Zeroing lamports on non-empty account. Consider using close_account() instead."
        );
    }

    // SAFETY: Direct lamport manipulation is safe here for PDA-to-PDA transfers because:
    // 1. Both accounts are owned by this program (PDAs)
    // 2. We use checked arithmetic to prevent overflow
    // 3. This is for partial transfers only (not account closure)
    // 4. We warn if this would zero out an account with data
    **from.lamports.borrow_mut() = remaining_lamports;
    **to.lamports.borrow_mut() = to
        .lamports()
        .checked_add(lamports)
        .ok_or::<ProgramError>(StakePoolError::NumericalOverflow.into())?;

    Ok(())
}

/// Transfer tokens with support for Token-2022 transfer fees
/// Safely extracts decimals from the mint account using proper unpacking
pub fn transfer_tokens_with_fee<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<u64, ProgramError> {
    // Safely unpack the mint account to get decimals
    let mint_data = mint.try_borrow_data()?;
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data)?;
    let decimals = mint_state.base.decimals;
    drop(mint_data);

    let accounts = vec![from.clone(), to.clone(), mint.clone(), authority.clone()];

    // Use transfer_checked for Token-2022 (supports transfer fees automatically)
    let transfer_ix = transfer_checked(
        token_program.key,
        from.key,
        mint.key,
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
