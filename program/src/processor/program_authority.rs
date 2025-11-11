use solana_program::log::sol_log_data;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

use crate::assertions::*;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::state::{Key, ProgramAuthority};
use crate::utils::create_account;

/// Initialize the program authority account (one-time setup)
///
/// This creates a global ProgramAuthority account that controls who can create stake pools.
/// Should only be called once during program deployment.
///
/// # Security
/// - Only the initial authority can manage the authorized creators list
/// - The ProgramAuthority PDA is deterministic (derived from "program_authority" seed)
/// - Cannot be reinitialized once created
///
/// # Arguments
/// * `accounts` - Required accounts for program authority initialization
///
/// # Errors
/// Returns error if:
/// - Program authority account already exists
/// - Account creation fails
/// - Signer validation fails
pub fn initialize_program_authority<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let ctx = InitializeProgramAuthorityAccounts::context(accounts)?;

    // Derive the expected program authority PDA
    let program_authority_seeds = ProgramAuthority::seeds();
    let program_authority_seeds_refs: Vec<&[u8]> = program_authority_seeds
        .iter()
        .map(|s| s.as_slice())
        .collect();
    let (program_authority_key, bump) =
        Pubkey::find_program_address(&program_authority_seeds_refs, &crate::ID);

    // Guards
    assert_same_pubkeys(
        "program_authority",
        ctx.accounts.program_authority,
        &program_authority_key,
    )?;
    assert_signer("initial_authority", ctx.accounts.initial_authority)?;
    assert_signer("payer", ctx.accounts.payer)?;
    assert_empty("program_authority", ctx.accounts.program_authority)?;
    assert_writable("program_authority", ctx.accounts.program_authority)?;
    assert_writable("payer", ctx.accounts.payer)?;

    // Create program authority account
    let mut seeds_with_bump = program_authority_seeds.clone();
    seeds_with_bump.push(vec![bump]);
    let seeds_refs: Vec<&[u8]> = seeds_with_bump.iter().map(|s| s.as_slice()).collect();

    create_account(
        ctx.accounts.program_authority,
        ctx.accounts.payer,
        ctx.accounts.system_program,
        ProgramAuthority::LEN,
        &crate::ID,
        Some(&[&seeds_refs]),
    )?;

    // Initialize program authority data
    let program_authority_data = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority: *ctx.accounts.initial_authority.key,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump,
    };

    program_authority_data.save(ctx.accounts.program_authority)?;

    msg!(
        "Program authority initialized with authority: {}",
        ctx.accounts.initial_authority.key
    );

    // Log event for off-chain indexing
    sol_log_data(&[
        b"ProgramAuthorityInitialized",
        ctx.accounts.initial_authority.key.as_ref(),
    ]);

    Ok(())
}

/// Manage authorized pool creators (add or remove)
///
/// Only the program authority can call this to add or remove addresses
/// from the authorized creators list.
///
/// # Security
/// - Only the main authority can manage the list
/// - Cannot remove the main authority itself
/// - Maximum of 10 authorized creators
/// - Validates all operations before applying changes
///
/// # Arguments
/// * `accounts` - Required accounts for managing creators
/// * `add` - List of addresses to add to authorized creators
/// * `remove` - List of addresses to remove from authorized creators
///
/// # Errors
/// Returns error if:
/// - Caller is not the program authority
/// - Maximum creators limit reached
/// - Creator already exists (when adding)
/// - Creator not found (when removing)
/// - Attempting to remove main authority
pub fn manage_authorized_creators<'a>(
    accounts: &'a [AccountInfo<'a>],
    add: Vec<Pubkey>,
    remove: Vec<Pubkey>,
) -> ProgramResult {
    let ctx = ManageAuthorizedCreatorsAccounts::context(accounts)?;

    // DoS Protection: Limit vector sizes to prevent excessive computation
    if add.len() > ProgramAuthority::MAX_CREATORS {
        msg!(
            "Too many creators to add: {}. Maximum: {}",
            add.len(),
            ProgramAuthority::MAX_CREATORS
        );
        return Err(StakePoolError::InvalidParameters.into());
    }
    if remove.len() > ProgramAuthority::MAX_CREATORS {
        msg!(
            "Too many creators to remove: {}. Maximum: {}",
            remove.len(),
            ProgramAuthority::MAX_CREATORS
        );
        return Err(StakePoolError::InvalidParameters.into());
    }

    // Load and validate program authority
    let mut program_authority_data = ProgramAuthority::load(ctx.accounts.program_authority)?;

    // Guards
    assert_signer("authority", ctx.accounts.authority)?;
    assert_writable("program_authority", ctx.accounts.program_authority)?;

    // Verify the signer is the program authority
    if ctx.accounts.authority.key != &program_authority_data.authority {
        msg!(
            "Unauthorized: {} is not the program authority",
            ctx.accounts.authority.key
        );
        return Err(StakePoolError::Unauthorized.into());
    }

    // Remove creators first
    for creator in &remove {
        program_authority_data.remove_creator(creator)?;
        msg!("Removed authorized creator: {}", creator);

        // Log event for off-chain indexing
        sol_log_data(&[
            b"AuthorizedCreatorRemoved",
            creator.as_ref(),
            ctx.accounts.authority.key.as_ref(),
        ]);
    }

    // Add new creators
    for creator in &add {
        program_authority_data.add_creator(*creator)?;
        msg!("Added authorized creator: {}", creator);

        // Log event for off-chain indexing
        sol_log_data(&[
            b"AuthorizedCreatorAdded",
            creator.as_ref(),
            ctx.accounts.authority.key.as_ref(),
        ]);
    }

    // Save updated state
    program_authority_data.save(ctx.accounts.program_authority)?;

    msg!(
        "Authorized creators updated. Current count: {}",
        program_authority_data.creator_count
    );

    Ok(())
}
