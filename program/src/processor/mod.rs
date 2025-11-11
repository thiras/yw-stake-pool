use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instruction::StakePoolInstruction;

mod admin;
mod close;
mod helpers;
mod initialize;
mod program_authority;
mod rewards;
mod stake;

// Re-export handler functions
pub use admin::{
    accept_authority, finalize_reward_rate_change, nominate_new_authority, update_pool,
};
pub use close::close_stake_account;
pub use initialize::initialize_pool;
pub use program_authority::{initialize_program_authority, manage_authorized_creators};
pub use rewards::{claim_rewards, fund_rewards};
pub use stake::{stake, unstake};

pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    // Validate instruction data before deserialization to prevent type cosplay attacks
    if instruction_data.is_empty() {
        msg!("Instruction data is empty");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Deserialize instruction with explicit error handling
    let instruction: StakePoolInstruction = StakePoolInstruction::try_from_slice(instruction_data)
        .map_err(|_| {
            msg!("Failed to deserialize instruction");
            ProgramError::InvalidInstructionData
        })?;

    match instruction {
        StakePoolInstruction::InitializePool {
            pool_id,
            reward_rate,
            min_stake_amount,
            lockup_period,
            enforce_lockup,
            pool_end_date,
        } => {
            msg!("Instruction: InitializePool");
            initialize_pool(
                accounts,
                pool_id,
                reward_rate,
                min_stake_amount,
                lockup_period,
                enforce_lockup,
                pool_end_date,
            )
        }
        StakePoolInstruction::Stake {
            amount,
            index,
            expected_reward_rate,
            expected_lockup_period,
        } => {
            msg!("Instruction: Stake");
            stake(
                accounts,
                amount,
                index,
                expected_reward_rate,
                expected_lockup_period,
            )
        }
        StakePoolInstruction::Unstake {
            amount,
            expected_reward_rate,
        } => {
            msg!("Instruction: Unstake");
            unstake(accounts, amount, expected_reward_rate)
        }
        StakePoolInstruction::ClaimRewards => {
            msg!("Instruction: ClaimRewards");
            claim_rewards(accounts)
        }
        StakePoolInstruction::UpdatePool {
            reward_rate,
            min_stake_amount,
            lockup_period,
            is_paused,
            enforce_lockup,
            pool_end_date,
        } => {
            msg!("Instruction: UpdatePool");
            update_pool(
                accounts,
                reward_rate,
                min_stake_amount,
                lockup_period,
                is_paused,
                enforce_lockup,
                pool_end_date,
            )
        }
        StakePoolInstruction::FundRewards { amount } => {
            msg!("Instruction: FundRewards");
            fund_rewards(accounts, amount)
        }
        StakePoolInstruction::NominateNewAuthority => {
            msg!("Instruction: NominateNewAuthority");
            nominate_new_authority(accounts)
        }
        StakePoolInstruction::AcceptAuthority => {
            msg!("Instruction: AcceptAuthority");
            accept_authority(accounts)
        }
        StakePoolInstruction::CloseStakeAccount => {
            msg!("Instruction: CloseStakeAccount");
            close_stake_account(accounts)
        }
        StakePoolInstruction::FinalizeRewardRateChange => {
            msg!("Instruction: FinalizeRewardRateChange");
            finalize_reward_rate_change(accounts)
        }
        StakePoolInstruction::InitializeProgramAuthority => {
            msg!("Instruction: InitializeProgramAuthority");
            initialize_program_authority(accounts)
        }
        StakePoolInstruction::ManageAuthorizedCreators { add, remove } => {
            msg!("Instruction: ManageAuthorizedCreators");
            manage_authorized_creators(accounts, add, remove)
        }
    }
}
