# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.7.0]

### Added
- **Program Authority Transfer Scripts**: New CLI scripts for managing program authority transfers
  - `transfer-program-authority.mjs`: Nominate new program authority (step 1 of two-step transfer)
  - `accept-program-authority.mjs`: Accept authority transfer (step 2)
  - `cancel-authority-transfer.mjs`: Cancel pending authority transfers
  - New npm scripts: `programs:transfer-authority`, `programs:accept-authority`, `programs:cancel-authority`
  - Two-step transfer process ensures safer authority management with explicit confirmation
  - Comprehensive documentation added to README.md with comparison table

- **Authorized Creator Management Scripts**: CLI tools for managing pool creators
  - `add-authorized-creator.mjs`: Add addresses to authorized creators list
  - `remove-authorized-creator.mjs`: Remove addresses from authorized creators list
  - `list-authorized-creators.mjs`: List all authorized creators
  - New npm scripts: `programs:add-creator`, `programs:remove-creator`, `programs:list-creators`
  - Use generated client library for account decoding
  - Graceful handling of WebSocket confirmation limitations

- **Global Admin System Instructions**: New instructions for centralized authorization
  - `GetAuthorizedCreators`: Query function to retrieve list of all authorized pool creators
  - `CheckAuthorization`: Query function to verify if an address is authorized
  - `CancelAuthorityTransfer`: Cancel pending program authority transfer before acceptance
  - Enhanced event logging for all parameter updates (`PoolParameterUpdated` events)
  - `RewardRateProposed` event emitted when reward rate change is proposed

- **Security Enhancements**:
  - Added security policy (SECURITY.md) and security.txt for vulnerability reporting
  - Creator count validation to prevent data corruption in ProgramAuthority
  - Size verification test for ProgramAuthority maximal case (398 bytes with pending_authority)

### Changed
- **BREAKING: Global Admin System Refactoring**: Complete overhaul of authority model
  - **Removed per-pool authority**: Authority and pending_authority fields removed from StakePool
  - **StakePool size reduced**: 288 bytes → 223 bytes (65 bytes saved per pool)
  - **Global authorization**: All pool management now requires global admin authorization via ProgramAuthority
  - **InitializePool updated**: No longer requires authority parameter (uses global admin check)
  - **UpdatePool updated**: Now checks ProgramAuthority.is_authorized() instead of per-pool authority
  - **Instruction changes**: 
    - Removed: `NominateNewAuthority` and `AcceptAuthority` (per-pool)
    - Added: `TransferProgramAuthority` and `AcceptProgramAuthority` (global)
  - **ProgramAuthority expanded**: Added pending_authority field (size: 365 → 398 bytes)
  - **Benefits**: Simplified administration, centralized access control, smaller pool accounts
  - **Migration required**: Redeploy program, recreate all pools, update client code

- **Instruction Discriminators Updated**: After global admin system refactor
  - `InitializeProgramAuthority`: 10 → 8
  - `FinalizeRewardRateChange`: 9 → 7
  - `ManageAuthorizedCreators`: 10 → 9
  - `TransferProgramAuthority`: 11 → 10
  - `AcceptProgramAuthority`: 12 → 11
  - `CancelAuthorityTransfer`: 13 → 12 (then reordered to 15→12)
  - `GetAuthorizedCreators`: Added at 13
  - `CheckAuthorization`: Added at 14

- **Instruction Order**: Reordered instructions to group authority transfer operations
  - Moved `CancelAuthorityTransfer` (discriminator 15→12) next to `TransferProgramAuthority` and `AcceptProgramAuthority`
  - Groups complete authority transfer lifecycle together (Transfer → Accept → Cancel)
  - Updated discriminators: `GetAuthorizedCreators` (12→13), `CheckAuthorization` (13→14)
  - Improves code organization and logical grouping

- **Architecture Consolidation**: Merged admin-related modules
  - Consolidated `program_authority.rs` functionality into `admin.rs`
  - All authority management in single module (674 lines)
  - Clear separation: program authority vs pool management
  - Better code organization and maintainability

- **Script Library Refactoring**: Improved script dependencies and maintainability
  - Added `@solana/kit` to root devDependencies
  - Created shared `program-authority.mjs` library (169 lines)
  - Refactored to use direct ES6 imports instead of dynamic resolution
  - Eliminated dependency on example directory structure
  - Simplified `initialize-authority.mjs` script (30% complexity reduction)
  - Better IDE support with static imports and faster import resolution

- **Clarity Improvements**: Renamed scripts and instructions for better understanding
  - Renamed `transfer-authority.mjs` → `transfer-upgrade-authority.mjs`
  - Distinguishes between program upgrade authority (binary redeployment) and program authority (PDA controlling pool creation)
  - Updated npm script: `programs:transfer-authority` → `programs:transfer-upgrade-authority`
  - Updated all documentation and usage examples

### Fixed
- **Transaction Confirmation**: Resolved WebSocket subscription errors in all program authority scripts
  - Replaced `sendAndConfirmTransactionFactory` with custom `sendAndWaitForTransaction` helper
  - Uses REST-only polling with `getSignatureStatuses()` (30-second timeout, 1-second intervals)
  - Eliminates "Cannot read properties of null (reading 'signatureNotifications')" errors
  - Fixed in: `initialize-authority.mjs`, `add-authorized-creator.mjs`, `remove-authorized-creator.mjs`, and all new scripts
  - Improved reliability and eliminated WebSocket dependency

- **StakePool::LEN Calculation**: Improved readability and documentation
  - Restructured constant using intermediate const values: FIXED_FIELDS + OPTIONS_MAX + RESERVED
  - Groups fields logically for easier verification (180 + 36 + 7 = 223 bytes)
  - Fixed incorrect documentation stating 237 bytes → corrected to 223 bytes
  - Updated rent-exempt minimum: ~2.3M → ~2.2M lamports in multiple documentation files

- **Authorization Logic**: Fixed error handling in UpdatePool
  - Use correct `Unauthorized` error when admin check fails
  - Consistent error handling across all admin operations

- **Code Review Fixes**: Multiple improvements from code review
  - Added ProgramAuthority size verification test for maximal case (398 bytes)
  - Enhanced authorization documentation in initialize.rs
  - Removed unused variables in example scripts
  - Improved comment clarity throughout codebase

### Removed
- **CloseProgramAuthority Instruction**: Removed temporary migration instruction
  - Was added temporarily for one-time ProgramAuthority account migration (365→398 bytes)
  - Successfully completed migration on devnet, no longer needed
  - Removed to reduce attack surface and eliminate unnecessary closure path
  - Removed: instruction variant, handler, script (`close-authority.mjs`), npm command, client code
  - Updated discriminators after removal: ManageAuthorizedCreators (10→9), TransferProgramAuthority (11→10), AcceptProgramAuthority (12→11), CancelAuthorityTransfer (13→12)
  - **Rationale**: Migration-only instruction that served its purpose; keeping it would provide unnecessary way to close critical program infrastructure

### Documentation
- **README.md Updates**:
  - Updated pool operator features to mention ProgramAuthority
  - Clarified global vs per-pool authority transfer scope
  - Updated instruction count and account sizes
  - Fixed code examples to remove authority parameter from initializePool
  - Added findProgramAuthorityPda to examples
  - Updated pool ID management section with authorization info
  - Added comprehensive authority management documentation with comparison tables

- **ARCHITECTURE.md Updates**:
  - Updated authority transfer flow to show global ProgramAuthority transfer
  - Added note that ProgramAuthority controls ALL pools (not per-pool)
  - Updated account relationships diagram to show ProgramAuthority structure
  - Removed per-pool authority fields from StakePool diagrams
  - Added admin + programAuthority requirement for UpdatePool
  - Documented authorized creators delegation (up to 10 addresses)
  - Updated performance considerations with ProgramAuthority size
  - Documented upgrade path for ProgramAuthority account

- **DEPLOYMENT.md Updates**:
  - Clarified authority keypair controls entire system
  - Updated ProgramAuthority initialization description
  - Added note about global admin role
  - Documented CLI scripts for managing authorized creators
  - Added TransferProgramAuthority/AcceptProgramAuthority examples
  - Warned that program authority transfer affects entire system
  - Consolidated script documentation from scattered locations

- **Implementation Details**: Updated script architecture section
  - Documented new transaction confirmation approach
  - Listed all authority management scripts
  - Explained benefits of REST-only polling
  - Documented shared library structure

### Tests
- **TypeScript Client Tests**: Updated for global admin model
  - Removed authority and pendingAuthority fields from StakePool tests
  - Updated instruction discriminators after adding new instructions
  - Added pendingAuthority field to ProgramAuthority test cases
  - All 53 TypeScript client tests passing

- **Rust Integration Tests**: Updated for global admin model
  - Removed authority from InitializePool accounts
  - Added payer to authorized creators in pool creation tests
  - Removed pool.authority assertions (now global, not per-pool)
  - Marked per-pool authority transfer test as obsolete
  - All 73 tests passing (4 ignored)

- **Admin System Tests**: Comprehensive coverage
  - All 20 admin system tests passing
  - Added creator count validation tests
  - Added ProgramAuthority size verification tests
  - Added GetAuthorizedCreators and CheckAuthorization tests

### Example Scripts
- **Updated for Global Admin Model**:
  - Removed authority parameter from all initializePool calls
  - Updated updatePool calls to use admin + programAuthority
  - Replaced per-pool authority transfer with global program authority transfer
  - Use getTransferProgramAuthorityInstruction and getAcceptProgramAuthorityInstruction
  - Updated comments to clarify global admin scope
  - All example scripts compile and run successfully
  - Added program authority initialization step to devnet integration test

## [1.6.1]

### Added
- **Admin Event Logging**: Added governance event logs for administrative operations
  - `RewardRateFinalized` event emitted when reward rate changes are finalized (includes old and new rates)
  - `AuthorityNominated` event emitted when new authority is nominated (includes current and new authority)
  - `AuthorityTransferred` event emitted when authority transfer completes (includes old and new authority)
  - Improves off-chain monitoring and governance tracking
  - Enables better audit trails for sensitive operations
  - Follows existing pattern: save state before emitting events

- **Pool Solvency Helper**: Added `verify_solvency()` method to `StakePool` state
  - Checks if reward vault balance can cover all owed rewards
  - Provides detailed error message with deficit amount when insolvent
  - Enables proactive monitoring of pool health
  - Includes usage example in documentation
  - Uses checked arithmetic to prevent overflow in calculations
  - Returns `InsufficientRewards` error if pool is insolvent

### Documentation
- **Enhanced Constant Documentation**: Comprehensive inline documentation for security-critical constants
  - **MIN_LOCKUP_PERIOD** (initialize.rs):
    - Added detailed security rationale explaining H-02 attack vector (reward vault drain prevention)
    - Documented business justification for 1-day minimum requirement
    - Noted configurability for different deployment requirements
    - Improved readability with structured markdown-style format
  - **REWARD_RATE_CHANGE_DELAY** (admin.rs):
    - Added comprehensive documentation for L-01 mitigation (centralized rate change prevention)
    - Explained 7-day delay selection and alignment with industry standards
    - Documented cooldown enforcement mechanism to prevent rapid successive changes
    - Clarified balance between user protection and operational flexibility
  - **MIN_ADDITIONAL_LAMPORTS** (utils.rs):
    - Enhanced with operational guidance section for troubleshooting
    - Added step-by-step resolution for "insufficient lamports" errors
    - Included economic analysis of attack/defense costs (~0.00089 SOL)
    - Provided current SOL value estimates for user reference
    - Clarified purpose as anti-griefing threshold vs dynamic rent calculation

## [1.6.0] - 2025-11-10

### Added
- **Event Logging**: Emit `sol_log_data` events for key operations (InitializePool, Stake, Unstake, ClaimRewards, FundRewards)
  - Enables off-chain indexing via Helius, TheGraph, or custom indexers
  - Improves observability and enables real-time notifications
  - Event data includes pool address, user address, amounts, and operation-specific details

- **Architecture Documentation**: Added comprehensive ARCHITECTURE.md with Mermaid diagrams
  - Account relationship diagrams showing PDA derivation and ownership
  - Detailed operational workflow diagrams (staking, unstaking, claiming, authority transfer, reward rate changes)
  - Security model and data flow documentation
  - Moved workflow diagrams from README.md to centralized architecture doc

### Changed
- **Constants Module**: Extract magic numbers to `constants.rs`
  - Added `MAX_REWARD_RATE` (1000% ceiling) and `REWARD_SCALE` (1e9 precision)
  - Replaced hardcoded literals throughout codebase for better maintainability
  - Improves code clarity and reduces risk of inconsistent values

### Fixed
- **[Q-02] Safe Integer Casting**: Replaced unsafe `u128` to `u64` cast with `try_from` in `calculate_rewards`
  - Prevents potential overflow/truncation issues in reward calculations
  - Uses checked conversion with proper error handling
  - Returns `ArithmeticOverflow` error if conversion would lose data

- **Pool End Date Timestamp Validation**: Fixed validation to allow future timestamps for `pool_end_date`
  - Added `validate_future_allowed_timestamp()` helper for expiration dates
  - `pool_end_date` now accepts future timestamps (as intended for expiration)
  - Historical timestamps (`reward_rate_change_timestamp`, `last_rate_change`) still validated as past-only
  - Maintains security by detecting data corruption (timestamps before Jan 1, 2021)
  - Comprehensive test suite with 11 tests covering all timestamp validation scenarios

- **[L-02] Account Size Validation**: Added defensive checks for account size validation
  - Validates that provided accounts have sufficient data length before deserialization
  - Prevents potential buffer overflow or undefined behavior from undersized accounts
  - Returns `AccountSizeTooSmall` error for insufficient account size
  - Added comprehensive test coverage for size validation edge cases

- **[M-03] Freeze Authority Validation**: Prevents pool initialization with mints that have freeze authority
  - Validates both stake_mint and reward_mint to ensure no freeze authority is set
  - Protects against centralized control where authority could freeze user funds
  - Returns `MintHasFreezeAuthority` error if freeze authority is detected
  - Comprehensive integration test coverage for both SPL Token and Token-2022

- **[M-02] Token-2022 Extension Validation**: Comprehensive Token-2022 extension security
  - Validates all Token-2022 mint extensions for dangerous configurations
  - **Dangerous extensions rejected**: `MintCloseAuthority`, `PermanentDelegate`, `TransferHook`, 
    `MetadataPointer`, `GroupPointer`, `GroupMemberPointer`, `ConfidentialTransferMint`, 
    `ConfidentialTransferFeeConfig`
  - **TransferFeeConfig fully supported**: Uses actual transferred amounts for accurate accounting
  - Balance verification before/after transfers to detect unexpected behavior
  - Fee costs borne by users, not pool - prevents fee-based reward re-claims
  - Returns `UnsafeTokenExtension` error for dangerous extensions
  - Returns `UnexpectedBalanceChange` error for balance verification failures
  - Extensive test coverage for all extension types

- **[M-01] PDA Front-Running DoS Prevention**: Protects PDA account creation from front-running attacks
  - Validates account ownership and data state before initialization
  - Rejects pre-allocated accounts with non-zero data
  - Implements idempotency check with discriminator validation for already-initialized accounts
  - Prevents attackers from blocking PDA creation by pre-funding accounts
  - Returns `AccountAlreadyInitialized` error for pre-existing data
  - Returns `AccountSizeTooSmall` error for insufficient pre-allocated space
  - Comprehensive test coverage for all attack vectors

- **[H-02] Reward Vault Drain Prevention**: Enforces minimum lockup period to prevent reward vault drain
  - Minimum 1-second lockup period required for all pools
  - Prevents zero-lockup exploit where users could stake/unstake rapidly to drain rewards
  - Returns `InvalidLockupPeriod` error if lockup period is zero
  - Maintains backward compatibility - pools can still use short lockups (e.g., 1 second)
  - Critical security fix that prevents complete reward vault drainage

- **[H-01] Vault Ownership Validation**: Validates reward vault ownership in pool initialization
  - Ensures reward_vault is owned by the pool's PDA authority
  - Prevents pools from being initialized with vaults they don't control
  - Returns `InvalidVaultOwner` error if vault ownership is incorrect
  - Protects against misconfiguration and potential fund theft

### Added
- **Admin-Only Pool Creation [Q-01 Security Fix]**: Restricts pool creation to authorized admins
  - `ProgramAuthority` state account (365 bytes) with PDA seed `"program_authority"`
  - Main authority (immutable) + up to 10 authorized pool creators
  - `InitializeProgramAuthority` instruction for one-time authority setup
  - `ManageAuthorizedCreators` instruction to add/remove authorized creators
  - Authorization check in `InitializePool` - only authorized creators can create pools
  - New error codes: `UnauthorizedPoolCreator`, `CreatorAlreadyAuthorized`, 
    `MaxAuthorizedCreatorsReached`, `CannotRemoveMainAuthority`, `CreatorNotFound`, `AlreadyInitialized`
  - Prevents permissionless spam/scam pool creation
  - **Security Enhancements**:
    - Reinitialization attack prevention (checks if account already has data)
    - DoS protection (limits vector sizes in ManageAuthorizedCreators)
    - Array compaction (prevents fragmentation after creator removal)
    - Event logging (enables off-chain tracking of authorization changes)
  - Full test coverage with 18 unit tests

- **Time-Locked Reward Rate Changes [L-01 Security Fix]**: 7-day delay for reward rate changes
  - `pending_reward_rate: Option<u64>` field added to `StakePool` state
  - `reward_rate_change_timestamp: Option<i64>` field added to `StakePool` state
  - `last_rate_change: Option<i64>` field added to `StakePool` state (enforces cooldown)
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
- **BREAKING CHANGE**: Pool PDA derivation no longer includes authority
  - **Old derivation**: `["stake_pool", authority, stake_mint, pool_id]`
  - **New derivation**: `["stake_pool", stake_mint, pool_id]`
  - Pool addresses are now token-scoped instead of authority-scoped
  - Eliminates confusion after authority transfers
  - Pool IDs are now globally unique per token (not per authority)
  - **Migration for TypeScript clients**:
    - Update `findPoolPda()` calls to remove authority parameter
    - `findPoolPda(stakeMint, poolId)` instead of `findPoolPda(authority, stakeMint, poolId)`
  - **No on-chain migration needed** (not deployed to mainnet yet)
  - All examples, tests, and documentation updated
  - Client types regenerated with new PDA derivation

- **BREAKING CHANGE**: `InitializePool` instruction now requires `program_authority` account
  - Account #10 (11th account): `program_authority` PDA (readonly)
  - **MIGRATION REQUIRED**: All clients must update to include this account
  - Use `findProgramAuthorityPda()` helper to derive the PDA address
  - TypeScript client regenerated with new account structure
  - All test suites updated (68 tests passing)

- **BREAKING CHANGE**: `StakePool` account structure modified (incompatible with existing pools)
  - Added `pending_reward_rate`, `reward_rate_change_timestamp`, and `last_rate_change` fields
  - Reduced `_reserved` from 32 bytes to 7 bytes
  - Account size remains 288 bytes (when pending fields are Some)
  - **MIGRATION REQUIRED**: Existing pools MUST be drained, closed, and recreated
  - Old structure: `pool_end_date` + `[u8; 32]` reserved
  - New structure: `pool_end_date` + `Option<u64>` + `Option<i64>` + `Option<i64>` + `[u8; 7]` reserved
  - Deserialization of old accounts will fail or produce corrupted data
  - This is acceptable for devnet deployment with no production pools
  - **DO NOT deploy to clusters with existing pools without proper migration**

- **Dependency Updates**: Updated to latest Solana toolchain versions
  - `solana-program`: 2.3.0
  - `shank`: 0.4.5
  - Ensures compatibility with latest Solana features and security patches

### Documentation
- **MIN_ADDITIONAL_LAMPORTS Clarification**: Improved documentation for anti-griefing threshold
  - Clarified this is NOT a dynamic rent-exempt calculation
  - Explained as anti-griefing threshold to prevent micro-transfer attacks
  - Documented why 890,880 lamports was chosen (typical rent-exempt for ~200 byte account)
  - Value remains unchanged but purpose is now clearly documented

### Added (continued from previous unreleased)
- **Multi-Pool Support Enhancement**: New `pool_id` parameter for better pool management
  - `pool_id: u64` field added to `StakePool` state
  - Enables multiple pools for the same stake_mint using unique IDs
  - Pool PDA derivation: `["stake_pool", stake_mint, pool_id]`
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
  - `TransferProgramAuthority` instruction for current authority to nominate new authority
  - `AcceptProgramAuthority` instruction for new authority to accept transfer
  - `pending_authority` field added to `ProgramAuthority` state
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
