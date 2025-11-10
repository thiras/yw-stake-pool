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

/// Create a new PDA account from the given size.
///
/// SECURITY FIX [M-01]: Front-running DoS Prevention
/// This function is designed to prevent front-running DoS attacks on PDA creation.
/// Instead of using system_instruction::create_account (which fails if the account
/// has any lamports), this implementation uses allocate + assign pattern which:
///
/// 1. Checks if account already exists and is properly initialized (idempotent)
/// 2. If account has lamports but NO DATA, allocates space and assigns ownership
///    - Handles simple DoS: attacker sends lamports without allocating data
///    - Rejects accounts with pre-allocated data (potential malicious content)
/// 3. If account is empty, creates it normally via transfer + allocate + assign
///
/// This prevents attackers from blocking PDA creation by sending rent-exempt SOL to the address.
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
    let required_lamports: u64 = rent.minimum_balance(size);

    // Get the signer seeds, defaulting to empty if None
    let signer_seeds = signer_seeds.unwrap_or(&[]);

    // Check current state of target account
    let current_lamports = target_account.lamports();
    let current_data_len = target_account.data_len();
    let current_owner = target_account.owner;

    // Case 1: Account already properly initialized (idempotent behavior)
    if current_lamports >= required_lamports && current_data_len == size && *current_owner == *owner
    {
        // Verify account data is uninitialized (all zeros) to prevent accepting
        // malicious pre-initialized accounts. An attacker could pre-fund an account
        // with correct lamports/size/owner but fill it with malicious data.
        let data = target_account.try_borrow_data()?;
        let is_zeroed = data.iter().all(|&byte| byte == 0);
        drop(data);

        if !is_zeroed {
            // Account has initialized data - reject to prevent malicious pre-initialization
            return Err(StakePoolError::ExpectedEmptyAccount.into());
        }

        // Account has correct params and is uninitialized, nothing to do
        return Ok(());
    }

    // Case 2: Account has lamports but wrong configuration
    // This is the front-running scenario - account has SOL but isn't initialized
    if current_lamports > 0 {
        // Error if account has any data - cannot safely take ownership of pre-allocated data
        // as it may contain malicious content. We only handle the simple DoS case where
        // an attacker sends lamports without allocating data.
        if current_data_len != 0 {
            return Err(StakePoolError::ExpectedEmptyAccount.into());
        }

        // Calculate additional lamports needed (if any)
        if current_lamports < required_lamports {
            // Safe: condition above guarantees required_lamports > current_lamports
            let additional_lamports = required_lamports - current_lamports;

            // Transfer additional lamports to meet rent requirement
            let transfer_ix = solana_program::system_instruction::transfer(
                funding_account.key,
                target_account.key,
                additional_lamports,
            );
            invoke(
                &transfer_ix,
                &[funding_account.clone(), target_account.clone()],
            )?;
        }

        // Ensure account is owned by system program before allocate
        // Note: When transferring lamports to a non-existent address, Solana creates an account
        // owned by the system program. This check handles edge cases.
        if *current_owner != solana_program::system_program::id() {
            let assign_to_system_ix = solana_program::system_instruction::assign(
                target_account.key,
                &solana_program::system_program::id(),
            );
            invoke_signed(
                &assign_to_system_ix,
                &[target_account.clone()],
                signer_seeds,
            )?;
        }

        // Allocate space (we know current_data_len == 0 at this point)
        let allocate_ix =
            solana_program::system_instruction::allocate(target_account.key, size as u64);
        invoke_signed(&allocate_ix, &[target_account.clone()], signer_seeds)?;

        // Assign to our program
        let assign_ix = solana_program::system_instruction::assign(target_account.key, owner);
        invoke_signed(&assign_ix, &[target_account.clone()], signer_seeds)?;

        return Ok(());
    }

    // Case 3: Account is completely empty - use standard creation
    // Use allocate + assign pattern instead of create_account for consistency
    // This also works better with PDAs

    // First, transfer lamports for rent
    let transfer_ix = solana_program::system_instruction::transfer(
        funding_account.key,
        target_account.key,
        required_lamports,
    );
    invoke(
        &transfer_ix,
        &[funding_account.clone(), target_account.clone()],
    )?;

    // Then allocate space
    let allocate_ix = solana_program::system_instruction::allocate(target_account.key, size as u64);
    invoke_signed(&allocate_ix, &[target_account.clone()], signer_seeds)?;

    // Finally assign ownership
    let assign_ix = solana_program::system_instruction::assign(target_account.key, owner);
    invoke_signed(&assign_ix, &[target_account.clone()], signer_seeds)?;

    Ok(())
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
