# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
