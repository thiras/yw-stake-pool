/// Maximum allowed reward rate (1000% when scaled by REWARD_SCALE)
/// This prevents misconfiguration of reward rates that could drain the reward vault
pub const MAX_REWARD_RATE: u64 = 1_000_000_000_000; // 1000% * 1e9

/// Scale factor for reward rate calculations (1e9)
/// Reward rates are stored as scaled integers to maintain precision
/// Example: 100_000_000 = 10% reward rate (100_000_000 / 1_000_000_000 = 0.10)
pub const REWARD_SCALE: u128 = 1_000_000_000;

/// Scale factor for reward rate calculations (u64 version)
/// Used in contexts where u64 is required instead of u128
pub const REWARD_SCALE_U64: u64 = 1_000_000_000;
