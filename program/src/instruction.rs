use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, ShankInstruction)]
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
        reward_rate_per_second: u64,
        min_stake_amount: u64,
        lockup_period: i64,
    },

    /// Initialize a stake account for a user
    #[account(0, writable, name="stake_account", desc = "The stake account PDA")]
    #[account(1, name="pool", desc = "The stake pool")]
    #[account(2, signer, name="owner", desc = "The stake account owner")]
    #[account(3, writable, signer, name="payer", desc = "The account paying for rent")]
    #[account(4, name="system_program", desc = "The system program")]
    InitializeStakeAccount,

    /// Stake tokens into the pool
    #[account(0, writable, name="pool", desc = "The stake pool")]
    #[account(1, writable, name="stake_account", desc = "The user's stake account")]
    #[account(2, signer, name="owner", desc = "The stake account owner")]
    #[account(3, writable, name="user_token_account", desc = "User's token account")]
    #[account(4, writable, name="stake_vault", desc = "Pool's stake vault")]
    #[account(5, name="token_program", desc = "The token program (Token or Token-2022)")]
    Stake { amount: u64 },

    /// Unstake tokens from the pool
    #[account(0, writable, name="pool", desc = "The stake pool")]
    #[account(1, writable, name="stake_account", desc = "The user's stake account")]
    #[account(2, signer, name="owner", desc = "The stake account owner")]
    #[account(3, writable, name="user_token_account", desc = "User's token account")]
    #[account(4, writable, name="stake_vault", desc = "Pool's stake vault")]
    #[account(5, name="token_program", desc = "The token program")]
    #[account(6, name="clock", desc = "Clock sysvar")]
    Unstake { amount: u64 },

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
        reward_rate_per_second: Option<u64>,
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

// Manual context implementations
pub mod accounts {
    use solana_program::{account_info::AccountInfo, program_error::ProgramError};

    pub struct Context<T> {
        pub accounts: T,
    }

    pub struct InitializePoolAccounts<'a> {
        pub pool: &'a AccountInfo<'a>,
        pub authority: &'a AccountInfo<'a>,
        pub stake_mint: &'a AccountInfo<'a>,
        pub reward_mint: &'a AccountInfo<'a>,
        pub stake_vault: &'a AccountInfo<'a>,
        pub reward_vault: &'a AccountInfo<'a>,
        pub payer: &'a AccountInfo<'a>,
        pub token_program: &'a AccountInfo<'a>,
        pub system_program: &'a AccountInfo<'a>,
        pub rent: &'a AccountInfo<'a>,
    }

    impl<'a> InitializePoolAccounts<'a> {
        pub fn context(accounts: &'a [AccountInfo<'a>]) -> Result<Context<Self>, ProgramError> {
            if accounts.len() < 10 {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            Ok(Context {
                accounts: Self {
                    pool: &accounts[0],
                    authority: &accounts[1],
                    stake_mint: &accounts[2],
                    reward_mint: &accounts[3],
                    stake_vault: &accounts[4],
                    reward_vault: &accounts[5],
                    payer: &accounts[6],
                    token_program: &accounts[7],
                    system_program: &accounts[8],
                    rent: &accounts[9],
                },
            })
        }
    }

    pub struct InitializeStakeAccountAccounts<'a> {
        pub stake_account: &'a AccountInfo<'a>,
        pub pool: &'a AccountInfo<'a>,
        pub owner: &'a AccountInfo<'a>,
        pub payer: &'a AccountInfo<'a>,
        pub system_program: &'a AccountInfo<'a>,
    }

    impl<'a> InitializeStakeAccountAccounts<'a> {
        pub fn context(accounts: &'a [AccountInfo<'a>]) -> Result<Context<Self>, ProgramError> {
            if accounts.len() < 5 {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            Ok(Context {
                accounts: Self {
                    stake_account: &accounts[0],
                    pool: &accounts[1],
                    owner: &accounts[2],
                    payer: &accounts[3],
                    system_program: &accounts[4],
                },
            })
        }
    }

    pub struct StakeAccounts<'a> {
        pub pool: &'a AccountInfo<'a>,
        pub stake_account: &'a AccountInfo<'a>,
        pub owner: &'a AccountInfo<'a>,
        pub user_token_account: &'a AccountInfo<'a>,
        pub stake_vault: &'a AccountInfo<'a>,
        pub token_program: &'a AccountInfo<'a>,
    }

    impl<'a> StakeAccounts<'a> {
        pub fn context(accounts: &'a [AccountInfo<'a>]) -> Result<Context<Self>, ProgramError> {
            if accounts.len() < 6 {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            Ok(Context {
                accounts: Self {
                    pool: &accounts[0],
                    stake_account: &accounts[1],
                    owner: &accounts[2],
                    user_token_account: &accounts[3],
                    stake_vault: &accounts[4],
                    token_program: &accounts[5],
                },
            })
        }
    }

    pub struct UnstakeAccounts<'a> {
        pub pool: &'a AccountInfo<'a>,
        pub stake_account: &'a AccountInfo<'a>,
        pub owner: &'a AccountInfo<'a>,
        pub user_token_account: &'a AccountInfo<'a>,
        pub stake_vault: &'a AccountInfo<'a>,
        pub token_program: &'a AccountInfo<'a>,
        pub clock: &'a AccountInfo<'a>,
    }

    impl<'a> UnstakeAccounts<'a> {
        pub fn context(accounts: &'a [AccountInfo<'a>]) -> Result<Context<Self>, ProgramError> {
            if accounts.len() < 7 {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            Ok(Context {
                accounts: Self {
                    pool: &accounts[0],
                    stake_account: &accounts[1],
                    owner: &accounts[2],
                    user_token_account: &accounts[3],
                    stake_vault: &accounts[4],
                    token_program: &accounts[5],
                    clock: &accounts[6],
                },
            })
        }
    }

    pub struct ClaimRewardsAccounts<'a> {
        pub pool: &'a AccountInfo<'a>,
        pub stake_account: &'a AccountInfo<'a>,
        pub owner: &'a AccountInfo<'a>,
        pub user_reward_account: &'a AccountInfo<'a>,
        pub reward_vault: &'a AccountInfo<'a>,
        pub token_program: &'a AccountInfo<'a>,
        pub clock: &'a AccountInfo<'a>,
    }

    impl<'a> ClaimRewardsAccounts<'a> {
        pub fn context(accounts: &'a [AccountInfo<'a>]) -> Result<Context<Self>, ProgramError> {
            if accounts.len() < 7 {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            Ok(Context {
                accounts: Self {
                    pool: &accounts[0],
                    stake_account: &accounts[1],
                    owner: &accounts[2],
                    user_reward_account: &accounts[3],
                    reward_vault: &accounts[4],
                    token_program: &accounts[5],
                    clock: &accounts[6],
                },
            })
        }
    }

    pub struct UpdatePoolAccounts<'a> {
        pub pool: &'a AccountInfo<'a>,
        pub authority: &'a AccountInfo<'a>,
    }

    impl<'a> UpdatePoolAccounts<'a> {
        pub fn context(accounts: &'a [AccountInfo<'a>]) -> Result<Context<Self>, ProgramError> {
            if accounts.len() < 2 {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            Ok(Context {
                accounts: Self {
                    pool: &accounts[0],
                    authority: &accounts[1],
                },
            })
        }
    }

    pub struct FundRewardsAccounts<'a> {
        pub pool: &'a AccountInfo<'a>,
        pub funder: &'a AccountInfo<'a>,
        pub funder_token_account: &'a AccountInfo<'a>,
        pub reward_vault: &'a AccountInfo<'a>,
        pub token_program: &'a AccountInfo<'a>,
    }

    impl<'a> FundRewardsAccounts<'a> {
        pub fn context(accounts: &'a [AccountInfo<'a>]) -> Result<Context<Self>, ProgramError> {
            if accounts.len() < 5 {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            Ok(Context {
                accounts: Self {
                    pool: &accounts[0],
                    funder: &accounts[1],
                    funder_token_account: &accounts[2],
                    reward_vault: &accounts[3],
                    token_program: &accounts[4],
                },
            })
        }
    }
}
