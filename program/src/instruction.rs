use borsh::{BorshDeserialize, BorshSerialize};
use shank::{ShankContext, ShankInstruction};
use solana_program::pubkey::Pubkey;

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, ShankContext, ShankInstruction)]
#[rustfmt::skip]
pub enum StakePoolInstruction {
    /// Initialize a new stake pool
    #[account(0, writable, name="pool", desc = "The stake pool PDA")]
    #[account(1, name="stake_mint", desc = "The token mint being staked")]
    #[account(2, name="reward_mint", desc = "The reward token mint")]
    #[account(3, writable, name="stake_vault", desc = "The pool's stake token vault")]
    #[account(4, writable, name="reward_vault", desc = "The pool's reward token vault")]
    #[account(5, writable, signer, name="payer", desc = "The account paying for rent (must be authorized admin)")]
    #[account(6, name="token_program", desc = "The token program")]
    #[account(7, name="system_program", desc = "The system program")]
    #[account(8, name="rent", desc = "Rent sysvar")]
    #[account(9, name="program_authority", desc = "The program authority account (validates creator permission)")]
    InitializePool {
        /// Unique identifier to allow multiple pools for same authority + stake_mint (typically 0 for first pool, 1 for second, etc.)
        pool_id: u64,
        reward_rate: u64,
        min_stake_amount: u64,
        lockup_period: i64,
        /// Whether to enforce lockup period (prevent early withdrawals). Default: false (allows early unstake with penalty)
        enforce_lockup: bool,
        /// Optional pool end date (Unix timestamp). If set, no new stakes allowed after this time.
        pool_end_date: Option<i64>,
    },

    /// Stake tokens into the pool (creates a new stake account for this deposit)
    /// Each stake account has independent lockup period and reward tracking
    /// Multiple deposits create separate accounts (index 0, 1, 2, etc.)
    #[account(0, writable, name="pool", desc = "The stake pool")]
    #[account(1, writable, name="stake_account", desc = "The stake account PDA (will be created)")]
    #[account(2, signer, name="owner", desc = "The stake account owner")]
    #[account(3, writable, name="user_token_account", desc = "User's token account")]
    #[account(4, writable, name="stake_vault", desc = "Pool's stake vault")]
    #[account(5, name="reward_vault", desc = "Pool's reward vault (for checking available rewards)")]
    #[account(6, name="stake_mint", desc = "The token mint being staked")]
    #[account(7, name="token_program", desc = "The token program (Token or Token-2022)")]
    #[account(8, writable, signer, name="payer", desc = "The account paying for rent")]
    #[account(9, name="system_program", desc = "The system program")]
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
    #[account(5, name="stake_mint", desc = "The token mint being staked")]
    #[account(6, name="token_program", desc = "The token program")]
    #[account(7, name="clock", desc = "Clock sysvar")]
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
    #[account(5, name="reward_mint", desc = "The reward token mint")]
    #[account(6, name="token_program", desc = "The token program")]
    #[account(7, name="clock", desc = "Clock sysvar")]
    ClaimRewards,

    /// Update pool settings (global admin only)
    #[account(0, writable, name="pool", desc = "The stake pool")]
    #[account(1, signer, name="admin", desc = "The global admin (authorized in ProgramAuthority)")]
    #[account(2, name="program_authority", desc = "The program authority account (validates admin permission)")]
    UpdatePool {
        reward_rate: Option<u64>,
        min_stake_amount: Option<u64>,
        lockup_period: Option<i64>,
        is_paused: Option<bool>,
        /// Whether to enforce lockup period (prevent early withdrawals)
        enforce_lockup: Option<bool>,
        /// Optional pool end date (Unix timestamp). Set to extend/shorten pool duration.
        pool_end_date: Option<Option<i64>>,
    },

    /// Fund the reward pool (anyone can fund)
    #[account(0, writable, name="pool", desc = "The stake pool")]
    #[account(1, signer, name="funder", desc = "The account funding rewards")]
    #[account(2, writable, name="funder_token_account", desc = "Funder's reward token account")]
    #[account(3, writable, name="reward_vault", desc = "Pool's reward vault")]
    #[account(4, name="reward_mint", desc = "The reward token mint")]
    #[account(5, name="token_program", desc = "The token program")]
    FundRewards { amount: u64 },

    /// Close an empty stake account and recover rent
    #[account(0, writable, name="stake_account", desc = "The stake account to close")]
    #[account(1, signer, name="owner", desc = "The stake account owner")]
    #[account(2, writable, name="receiver", desc = "Account to receive the rent lamports")]
    CloseStakeAccount,

    /// Finalize a pending reward rate change after the delay period
    /// This completes the two-step process for changing reward rates.
    /// After authority proposes a rate change via UpdatePool, anyone can
    /// call this after 7 days to apply the change.
    #[account(0, writable, name="pool", desc = "The stake pool")]
    FinalizeRewardRateChange,

    /// Initialize the program authority (one-time setup)
    /// This creates the global authority account that controls who can create pools
    #[account(0, writable, name="program_authority", desc = "The program authority PDA to create")]
    #[account(1, signer, name="initial_authority", desc = "The initial authority who will control authorized creators")]
    #[account(2, writable, signer, name="payer", desc = "The account paying for rent")]
    #[account(3, name="system_program", desc = "The system program")]
    InitializeProgramAuthority,

    /// Manage authorized pool creators (add or remove)
    /// Only the program authority can call this
    #[account(0, writable, name="program_authority", desc = "The program authority PDA")]
    #[account(1, signer, name="authority", desc = "The program authority signer")]
    ManageAuthorizedCreators {
        /// Addresses to add to authorized creators list
        add: Vec<Pubkey>,
        /// Addresses to remove from authorized creators list
        remove: Vec<Pubkey>,
    },

    /// Transfer the global program authority to a new admin
    /// This is a two-step process: nominate + accept
    /// Only the current program authority can call this
    #[account(0, writable, name="program_authority", desc = "The program authority PDA")]
    #[account(1, signer, name="current_authority", desc = "The current program authority")]
    #[account(2, name="new_authority", desc = "The new authority to nominate")]
    TransferProgramAuthority,

    /// Accept the transfer of program authority
    /// Second step of the authority transfer process
    #[account(0, writable, name="program_authority", desc = "The program authority PDA")]
    #[account(1, signer, name="pending_authority", desc = "The pending authority accepting the transfer")]
    AcceptProgramAuthority,
}
