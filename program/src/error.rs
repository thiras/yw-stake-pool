use num_derive::FromPrimitive;
use solana_program::program_error::ProgramError;
use thiserror::Error;

#[allow(non_local_definitions)]
#[derive(Error, Clone, Debug, Eq, PartialEq, FromPrimitive)]
pub enum StakePoolError {
    /// 0 - Error deserializing an account
    #[error("Error deserializing an account")]
    DeserializationError,
    /// 1 - Error serializing an account
    #[error("Error serializing an account")]
    SerializationError,
    /// 2 - Invalid program owner
    #[error("Invalid program owner")]
    InvalidProgramOwner,
    /// 3 - Invalid PDA derivation
    #[error("Invalid PDA derivation")]
    InvalidPda,
    /// 4 - Expected empty account
    #[error("Expected empty account")]
    ExpectedEmptyAccount,
    /// 5 - Expected non empty account
    #[error("Expected non empty account")]
    ExpectedNonEmptyAccount,
    /// 6 - Expected signer account
    #[error("Expected signer account")]
    ExpectedSignerAccount,
    /// 7 - Expected writable account
    #[error("Expected writable account")]
    ExpectedWritableAccount,
    /// 8 - Account mismatch
    #[error("Account mismatch")]
    AccountMismatch,
    /// 9 - Invalid account key
    #[error("Invalid account key")]
    InvalidAccountKey,
    /// 10 - Numerical overflow
    #[error("Numerical overflow")]
    NumericalOverflow,
    /// 11 - Pool is paused
    #[error("Pool is paused")]
    PoolPaused,
    /// 12 - Amount below minimum stake
    #[error("Amount below minimum stake")]
    AmountBelowMinimum,
    /// 13 - Insufficient staked balance
    #[error("Insufficient staked balance")]
    InsufficientStakedBalance,
    /// 14 - Lockup period not expired
    #[error("Lockup period not expired")]
    LockupNotExpired,
    /// 15 - Insufficient rewards in pool
    #[error("Insufficient rewards in pool")]
    InsufficientRewards,
    /// 16 - Unauthorized
    #[error("Unauthorized")]
    Unauthorized,
    /// 17 - Invalid token program
    #[error("Invalid token program")]
    InvalidTokenProgram,
    /// 18 - Invalid mint
    #[error("Invalid mint")]
    InvalidMint,
    /// 19 - Invalid account discriminator
    #[error("Invalid account discriminator")]
    InvalidAccountDiscriminator,
    /// 20 - Pool parameters changed (frontrunning protection)
    #[error("Pool parameters changed (frontrunning protection)")]
    PoolParametersChanged,
    /// 21 - No pending authority transfer
    #[error("No pending authority transfer")]
    NoPendingAuthority,
    /// 22 - Invalid pending authority
    #[error("Invalid pending authority")]
    InvalidPendingAuthority,
    /// 23 - Pool has ended (no new stakes allowed)
    #[error("Pool has ended (no new stakes allowed)")]
    PoolEnded,
    /// 24 - Invalid parameters
    #[error("Invalid parameters")]
    InvalidParameters,
}

impl From<StakePoolError> for ProgramError {
    fn from(e: StakePoolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
