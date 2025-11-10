# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Multi-Pool Support Enhancement**: New `pool_id` parameter for better pool management
  - `pool_id: u64` field added to `StakePool` state
  - Enables multiple pools with same authority and stake_mint using unique IDs
  - Pool PDA derivation now includes pool_id: `["stake_pool", authority, stake_mint, pool_id]`
  - Use `pool_id: 0` for first pool, `1` for second, etc.
  - Built-in validation ensures pool address matches provided pool_id

### Changed
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
