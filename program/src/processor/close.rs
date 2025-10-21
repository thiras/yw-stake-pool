use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg};

use crate::assertions::*;
use crate::error::StakePoolError;
use crate::instruction::accounts::*;
use crate::state::{Key, StakeAccount};
use crate::utils::close_account;

pub fn close_stake_account<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    // Parse accounts using ShankContext-generated struct
    let ctx = CloseStakeAccountAccounts::context(accounts)?;

    // Verify stake account discriminator before loading (Type Cosplay protection)
    assert_account_key(
        "stake_account",
        ctx.accounts.stake_account,
        Key::StakeAccount,
    )?;

    // Verify program ownership
    assert_program_owner("stake_account", ctx.accounts.stake_account, &crate::ID)?;

    // Load stake account
    let stake_account_data = StakeAccount::load(ctx.accounts.stake_account)?;

    // Guards
    assert_signer("owner", ctx.accounts.owner)?;
    assert_writable("stake_account", ctx.accounts.stake_account)?;
    assert_writable("receiver", ctx.accounts.receiver)?;
    assert_same_pubkeys("owner", ctx.accounts.owner, &stake_account_data.owner)?;

    // Ensure stake account is empty (no staked amount)
    if stake_account_data.amount_staked != 0 {
        msg!(
            "Cannot close stake account with non-zero balance. Amount staked: {}",
            stake_account_data.amount_staked
        );
        return Err(StakePoolError::ExpectedEmptyAccount.into());
    }

    // Close the account and recover rent
    close_account(ctx.accounts.stake_account, ctx.accounts.receiver)?;

    msg!(
        "Closed stake account {} and returned rent to {}",
        ctx.accounts.stake_account.key,
        ctx.accounts.receiver.key
    );

    Ok(())
}
