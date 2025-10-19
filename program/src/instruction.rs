use borsh::{BorshDeserialize, BorshSerialize};
use shank::{ShankContext, ShankInstruction};

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, ShankContext, ShankInstruction)]
#[rustfmt::skip]
pub enum StakePoolInstruction {
    /// Initialize a new stake pool
    #[account(0, writable, name="pool", desc = "The stake pool PDA")]
    #[account(1, signer, name="authority", desc = "The pool authority")]
    #[account(2, name="stake_mint", desc = "The token mint being staked")]
    #[account(3, name="reward_mint", desc = "The reward token mint")]
    #[account(4, writable, name="stake_vault", desc = "The pool's stake token vault")]
    #[account(5, writable, name="reward_vault", desc = "The pool's reward token vault")]
    #[account(6, writable, signer, name="payer", desc = "The account paying for rent")]
    #[account(7, name="token_program", desc = "The token program")]
    #[account(8, name="system_program", desc = "The system program")]
    #[account(9, name="rent", desc = "Rent sysvar")]
    InitializePool {
        reward_rate: u64,
        min_stake_amount: u64,
        lockup_period: i64,
    },

    /// Initialize a stake account for a user with a specific index
    #[account(0, writable, name="stake_account", desc = "The stake account PDA")]
    #[account(1, name="pool", desc = "The stake pool")]
    #[account(2, signer, name="owner", desc = "The stake account owner")]
    #[account(3, writable, signer, name="payer", desc = "The account paying for rent")]
    #[account(4, name="system_program", desc = "The system program")]
    InitializeStakeAccount { index: u64 },

    /// Stake tokens into the pool (creates a new stake account for each deposit)
    #[account(0, writable, name="pool", desc = "The stake pool")]
    #[account(1, writable, name="stake_account", desc = "The user's new stake account")]
    #[account(2, signer, name="owner", desc = "The stake account owner")]
    #[account(3, writable, name="user_token_account", desc = "User's token account")]
    #[account(4, writable, name="stake_vault", desc = "Pool's stake vault")]
    #[account(5, name="reward_vault", desc = "Pool's reward vault (for checking available rewards)")]
    #[account(6, name="token_program", desc = "The token program (Token or Token-2022)")]
    #[account(7, writable, signer, name="payer", desc = "The account paying for rent")]
    #[account(8, name="system_program", desc = "The system program")]
    Stake {
        amount: u64,
        index: u64,
        /// Frontrunning protection: expected reward rate (optional)
        expected_reward_rate: Option<u64>,
        /// Frontrunning protection: expected lockup period (optional)
        expected_lockup_period: Option<i64>,
    },

    /// Unstake tokens from the pool
    #[account(0, writable, name="pool", desc = "The stake pool")]
    #[account(1, writable, name="stake_account", desc = "The user's stake account")]
    #[account(2, signer, name="owner", desc = "The stake account owner")]
    #[account(3, writable, name="user_token_account", desc = "User's token account")]
    #[account(4, writable, name="stake_vault", desc = "Pool's stake vault")]
    #[account(5, name="token_program", desc = "The token program")]
    #[account(6, name="clock", desc = "Clock sysvar")]
    Unstake {
        amount: u64,
        /// Frontrunning protection: expected reward rate (optional)
        expected_reward_rate: Option<u64>,
    },

    /// Claim rewards
    #[account(0, writable, name="pool", desc = "The stake pool")]
    #[account(1, writable, name="stake_account", desc = "The user's stake account")]
    #[account(2, signer, name="owner", desc = "The stake account owner")]
    #[account(3, writable, name="user_reward_account", desc = "User's reward token account")]
    #[account(4, writable, name="reward_vault", desc = "Pool's reward vault")]
    #[account(5, name="token_program", desc = "The token program")]
    #[account(6, name="clock", desc = "Clock sysvar")]
    ClaimRewards,

    /// Update pool settings (authority only)
    #[account(0, writable, name="pool", desc = "The stake pool")]
    #[account(1, signer, name="authority", desc = "The pool authority")]
    UpdatePool {
        reward_rate: Option<u64>,
        min_stake_amount: Option<u64>,
        lockup_period: Option<i64>,
        is_paused: Option<bool>,
    },

    /// Fund the reward pool (anyone can fund)
    #[account(0, writable, name="pool", desc = "The stake pool")]
    #[account(1, signer, name="funder", desc = "The account funding rewards")]
    #[account(2, writable, name="funder_token_account", desc = "Funder's reward token account")]
    #[account(3, writable, name="reward_vault", desc = "Pool's reward vault")]
    #[account(4, name="token_program", desc = "The token program")]
    FundRewards { amount: u64 },
}
