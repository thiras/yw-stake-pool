# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Time-Locked Reward Rate Changes [L-01 Security Fix]**: 7-day delay for reward rate changes
  - `pending_reward_rate: Option<u64>` field added to `StakePool` state
  - `reward_rate_change_timestamp: Option<i64>` field added to `StakePool` state
  - `FinalizeRewardRateChange` instruction (permissionless, callable after 7-day delay)
  - `REWARD_RATE_CHANGE_DELAY` constant: 604800 seconds (7 days)
  - Cancellation mechanism: proposing current rate clears pending change
  - Defense-in-depth: rate bounds validation in both proposal and finalization
  - Comprehensive timestamp validation with `MIN_VALID_TIMESTAMP` (Jan 1, 2021)
  - New error codes: `RewardRateChangeDelayNotElapsed`, `NoPendingRewardRateChange`, 
    `PendingRewardRateChangeExists`, `InvalidTimestamp`, `DataCorruption`
  - Prevents centralized surprise changes to reward rates
  - Authority transfer documentation: new authority can cancel by reproposing current rate

### Changed
- **BREAKING CHANGE**: `StakePool` account structure modified (incompatible with existing pools)
  - Added `pending_reward_rate` and `reward_rate_change_timestamp` fields
  - Reduced `_reserved` from 32 bytes to 16 bytes
  - Account size remains 288 bytes (when pending fields are Some)
  - **MIGRATION REQUIRED**: Existing pools MUST be drained, closed, and recreated
  - Old structure: `pool_end_date` + `[u8; 32]` reserved
  - New structure: `pool_end_date` + `Option<u64>` + `Option<i64>` + `[u8; 16]` reserved
  - Deserialization of old accounts will fail or produce corrupted data
  - This is acceptable for devnet deployment with no production pools
  - **DO NOT deploy to clusters with existing pools without proper migration**

### Added (continued from previous unreleased)
- **Multi-Pool Support Enhancement**: New `pool_id` parameter for better pool management
  - `pool_id: u64` field added to `StakePool` state
  - Enables multiple pools with same authority and stake_mint using unique IDs
  - Pool PDA derivation now includes pool_id: `["stake_pool", authority, stake_mint, pool_id]`
  - Use `pool_id: 0` for first pool, `1` for second, etc.
  - Built-in validation ensures pool address matches provided pool_id

### Changed (continued from previous unreleased)
- `StakePool` state size increased to accommodate new `pool_id` field
- `InitializePool` instruction now requires `pool_id: u64` parameter
- Pool PDA derivation updated to include pool_id in seed array
- All documentation updated with pool_id examples
- Test suite expanded with multi-pool validation

### Technical Details
- Added `test_multiple_pool_ids()` unit test
- Updated all PDA helper functions to include pool_id parameter
- Client library auto-generated with new pool_id field
- Example code updated to demonstrate pool_id usage
- 14 comprehensive reward rate change tests added
- 17 client tests validating time-lock functionality
- Timestamp validation helpers extracted to `helpers.rs` module
- All `Clock::get()` calls protected with timestamp validation

## [1.5.0] - 2025-10-22

### Added
- **Lockup Enforcement Control**: New `enforce_lockup` boolean field added to pools
  - When `true`: Prevents early withdrawals before lockup period expires (returns `LockupNotExpired` error)
  - When `false`: Allows early withdrawals with forfeited rewards (existing behavior)
  - Pool operators can toggle this setting via `UpdatePool` instruction
  - Provides flexibility for different staking strategies (strict vs. flexible)

### Changed
- `StakePool` state size increased from 277 to 278 bytes to accommodate new field
- `InitializePool` instruction now requires `enforce_lockup: bool` parameter (defaults to `false` recommended)
- `UpdatePool` instruction now accepts optional `enforce_lockup: Option<bool>` parameter
- Improved warning messages in unstake logic to only show when relevant

### Technical Details
- Program version bumped from `1.4.1` to `1.5.0`
- IDL updated with new fields
- Client library auto-generated with new types
- All tests updated and passing
- Example code updated to demonstrate new feature

## [1.4.1] - 2025-10-21

### Initial Release
- Secure staking program for Solana SPL tokens
- Token-2022 support with transfer fee handling
- Fixed reward system with configurable rates
- Flexible lockup periods
- Multi-pool support
- Two-step authority transfer
- Comprehensive security features

## [1.2.0] - 2025-10-19

### Added
- **Two-Step Authority Transfer**: Secure authority transfer mechanism
  - `NominateNewAuthority` instruction for current authority to nominate new authority
  - `AcceptAuthority` instruction for new authority to accept transfer
  - `pending_authority` field added to `StakePool` state
  - Protects against key compromise and misconfiguration scenarios
  - Custom errors: `NoPendingAuthority`, `InvalidPendingAuthority`

### Changed
- `StakePool` state size increased to accommodate `pending_authority` field

## [1.1.0] - 2025-10-19

### Added
- **Frontrunning Protection**: Optional parameters to lock in expected pool conditions
  - `expected_reward_rate` parameter in `Stake` and `Unstake` instructions
  - `expected_lockup_period` parameter in `Stake` instruction
  - `PoolParametersChanged` error to revert transactions when parameters mismatch
  - Backward compatible (protection is optional)

### Changed
- `Stake` instruction signature updated with optional frontrunning protection parameters
- `Unstake` instruction signature updated with optional `expected_reward_rate` parameter

---

## [1.1.0] - 2025-10-19

### Added
- **Frontrunning Protection**: Optional parameters to lock in expected pool conditions
  - `expected_reward_rate` parameter in `Stake` and `Unstake` instructions
  - `expected_lockup_period` parameter in `Stake` instruction
  - `PoolParametersChanged` error to revert transactions when parameters mismatch
  - Backward compatible (protection is optional)

### Changed
- `Stake` instruction signature updated with optional frontrunning protection parameters
- `Unstake` instruction signature updated with optional `expected_reward_rate` parameter
