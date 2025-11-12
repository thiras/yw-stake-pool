# YW Stake Pool - Architecture

## Overview

The YW Stake Pool is a Solana program that enables secure token staking with fixed reward rates and flexible lockup periods. This document provides a high-level architectural overview of the system.

## Core Components

### State Accounts

```mermaid
graph TB
    subgraph "Program Accounts"
        PA[ProgramAuthority]
        SP[StakePool]
        SA[StakeAccount]
    end
    
    subgraph "Token Accounts"
        SV[Stake Vault]
        RV[Reward Vault]
    end
    
    PA -->|authorizes| SP
    SP -->|owns| SV
    SP -->|owns| RV
    SP -->|tracks| SA
    
    style PA fill:#e1f5ff
    style SP fill:#fff4e1
    style SA fill:#ffe1f5
```

- **ProgramAuthority**: Global authority that controls who can create new pools (398 bytes)
  - Single authority address
  - List of up to 10 authorized creator addresses
  - Pending authority for two-step transfer
- **StakePool**: Individual pool configuration (reward rate, lockup period, vaults) - 223 bytes
  - No per-pool authority (managed globally via ProgramAuthority)
- **StakeAccount**: User's stake position (amount, timestamp, claimed rewards)
- **Stake Vault**: Token account holding all staked tokens for a pool
- **Reward Vault**: Token account holding reward tokens for distribution

### Key Operations Flow

```mermaid
sequenceDiagram
    participant User
    participant Program
    participant StakePool
    participant StakeAccount
    participant Vaults
    
    Note over User,Vaults: Pool Initialization
    User->>Program: InitializePool
    Program->>ProgramAuthority: Verify creator authorized
    Program->>StakePool: Create pool account
    Program->>Vaults: Validate vault ownership
    StakePool-->>User: Pool created
    
    Note over User,Vaults: Staking Flow
    User->>Program: Stake(amount)
    Program->>StakePool: Check pool not paused
    Program->>StakePool: Verify reward solvency
    Program->>StakeAccount: Create/update stake account
    User->>Vaults: Transfer tokens to stake vault
    Program->>StakePool: Update total_staked
    StakePool-->>User: Stake successful
    
    Note over User,Vaults: Claim Rewards
    User->>Program: ClaimRewards
    Program->>StakeAccount: Load stake data
    Program->>StakePool: Calculate rewards
    alt Lockup complete
        Program->>Vaults: Transfer from reward vault
        Program->>StakeAccount: Update claimed_rewards
        Vaults-->>User: Rewards transferred
    else Lockup incomplete
        Program-->>User: No rewards yet
    end
    
    Note over User,Vaults: Unstaking
    User->>Program: Unstake(amount)
    Program->>StakeAccount: Check balance
    alt enforce_lockup = true
        Program->>StakePool: Check lockup expired
    end
    Program->>StakePool: Calculate forfeited rewards
    Program->>Vaults: Transfer from stake vault
    Program->>StakeAccount: Update amount_staked
    Vaults-->>User: Tokens returned
```

## Detailed Operational Workflows

### Authority Transfer Flow

Two-step process prevents accidental authority loss. This transfers **global program authority** (control over entire system):

```mermaid
sequenceDiagram
    participant OldAuth as Current Authority
    participant PA as ProgramAuthority
    participant NewAuth as New Authority
    
    Note over PA: authority = OldAuth<br/>pending_authority = None
    
    OldAuth->>PA: TransferProgramAuthority(NewAuth)
    Note over PA: authority = OldAuth<br/>pending_authority = NewAuth
    
    NewAuth->>PA: AcceptProgramAuthority()
    Note over PA: authority = NewAuth<br/>pending_authority = None
    
    Note over PA: Transfer Complete!<br/>NewAuth now controls ALL pools
```

**Important**: This is a **global operation** that transfers control of the entire program, not individual pools. The new authority will control:
- Who can create new pools (via ManageAuthorizedCreators)
- Global program settings
- All future administrative operations

### Reward Rate Change Flow (Time-Locked)

Seven-day delay gives users time to exit if they disagree:

```mermaid
sequenceDiagram
    participant Auth as Authority
    participant Pool as Pool State
    participant Anyone as Anyone (Permissionless)
    participant Users as Users
    
    Note over Pool: reward_rate = 10%<br/>pending_reward_rate = None
    
    Auth->>Pool: UpdatePool(reward_rate: 50%)
    Note over Pool: reward_rate = 10%<br/>pending_reward_rate = 50%<br/>reward_rate_change_timestamp = Now
    
    Note over Users: 7-day notice period<br/>Users can unstake if they disagree
    
    rect rgb(255, 240, 200)
        Note over Anyone: Wait 7 days...
    end
    
    Anyone->>Pool: FinalizeRewardRateChange()
    Note over Pool: reward_rate = 50%<br/>pending_reward_rate = None<br/>last_rate_change = Now
    
    Note over Pool: Cooldown enforced<br/>Cannot propose new rate<br/>for another 7 days
```

### Staking Workflow

Complete flow from user action to state updates:

```mermaid
sequenceDiagram
    participant User
    participant StakeAccount as Stake Account (PDA)
    participant Pool as Pool State
    participant StakeVault as Stake Vault
    participant RewardVault as Reward Vault
    
    User->>Pool: Stake(amount, index)
    
    Note over Pool: Validate:<br/>- Pool not paused<br/>- Pool not ended<br/>- Amount >= min_stake
    
    Pool->>RewardVault: Check sufficient rewards
    Note over RewardVault: balance >= total_owed + new_rewards
    
    Pool->>StakeAccount: Create PDA (if new)
    Note over StakeAccount: owner = User<br/>index = index<br/>amount_staked = 0
    
    User->>StakeVault: Transfer tokens
    Note over StakeVault: balance += amount
    
    Pool->>Pool: Update state
    Note over Pool: total_staked += amount<br/>total_rewards_owed += expected_rewards
    
    Pool->>StakeAccount: Initialize/Update
    Note over StakeAccount: amount_staked = amount<br/>stake_timestamp = Now<br/>claimed_rewards = 0
    
    Note over User: Staking complete!<br/>Rewards accrue after lockup period
```

### Claiming Rewards Workflow

Rewards are only available after lockup period completes:

```mermaid
sequenceDiagram
    participant User
    participant StakeAccount as Stake Account
    participant Pool as Pool State
    participant RewardVault as Reward Vault
    participant UserRewardAccount as User Reward Account
    
    User->>Pool: ClaimRewards()
    
    Pool->>StakeAccount: Load stake data
    Note over StakeAccount: amount_staked<br/>stake_timestamp<br/>claimed_rewards
    
    Pool->>Pool: Calculate rewards
    alt Lockup period complete
        Note over Pool: rewards = (amount * rate) / 1e9
    else Lockup incomplete
        Note over Pool: rewards = 0
    end
    
    Note over Pool: unclaimed = total_rewards - claimed_rewards
    
    alt unclaimed > 0
        Pool->>RewardVault: Check balance
        RewardVault->>UserRewardAccount: Transfer unclaimed rewards
        Note over UserRewardAccount: balance += actual_amount<br/>(after transfer fees)
        
        Pool->>Pool: Update state
        Note over Pool: total_rewards_owed -= unclaimed
        
        Pool->>StakeAccount: Update claimed
        Note over StakeAccount: claimed_rewards += unclaimed
        
        Note over User: Rewards claimed!
    else unclaimed == 0
        Note over User: No rewards available yet
    end
```

### Unstaking Workflow

Partial or full unstake with optional lockup enforcement:

```mermaid
sequenceDiagram
    participant User
    participant StakeAccount as Stake Account
    participant Pool as Pool State
    participant StakeVault as Stake Vault
    participant UserTokenAccount as User Token Account
    
    User->>Pool: Unstake(amount)
    
    Pool->>StakeAccount: Load stake data
    Note over StakeAccount: amount_staked<br/>stake_timestamp
    
    Pool->>Pool: Check lockup
    alt enforce_lockup = true AND lockup not complete
        Note over User: Error: LockupNotExpired
    else enforce_lockup = false OR lockup complete
        Note over Pool: Continue unstaking
    end
    
    Pool->>Pool: Calculate forfeited rewards
    alt Partial unstake
        Note over Pool: forfeit = proportional_unclaimed_rewards
    else Full unstake
        Note over Pool: forfeit = all_unclaimed_rewards
    end
    
    StakeVault->>UserTokenAccount: Transfer tokens
    Note over UserTokenAccount: balance += actual_amount<br/>(after transfer fees)
    
    Pool->>Pool: Update state
    Note over Pool: total_staked -= amount<br/>total_rewards_owed -= forfeited_rewards
    
    Pool->>StakeAccount: Update or reset
    alt Partial unstake
        Note over StakeAccount: amount_staked -= amount
    else Full unstake
        Note over StakeAccount: amount_staked = 0<br/>stake_timestamp = 0<br/>claimed_rewards = 0
    end
    
    Note over User: Unstaking complete!
```

## Account Relationships

```mermaid
graph LR
    subgraph "PDA Derivation"
        SP_SEEDS["['stake_pool', stake_mint, pool_id]"]
        SA_SEEDS["['stake_account', pool, owner, index]"]
        PA_SEEDS["['program_authority']"]
    end
    
    SP_SEEDS -->|derives| SP[StakePool PDA]
    SA_SEEDS -->|derives| SA[StakeAccount PDA]
    PA_SEEDS -->|derives| PA[ProgramAuthority PDA]
    
    PA -->|authority: Pubkey| MAIN_AUTH[Main Authority]
    PA -->|authorized_creators: [Pubkey; 10]| CREATORS[Authorized Creators]
    PA -->|controls creation| SP
    
    SP -->|stake_vault: Pubkey| SV[Stake Vault Token Account]
    SP -->|reward_vault: Pubkey| RV[Reward Vault Token Account]
    
    SA -->|pool: Pubkey| SP
    SA -->|owner: Pubkey| USER[User Wallet]
    
    style SP fill:#fff4e1
    style SA fill:#ffe1f5
    style PA fill:#e1f5ff
```

**Key Changes from Per-Pool Authority:**
- StakePool no longer has `authority` or `pending_authority` fields
- All administrative control flows through global ProgramAuthority
- UpdatePool instruction requires both `admin` signer AND `programAuthority` validation

## Security Model

### Type Safety
- All accounts use discriminators (Type Cosplay protection)
- Account ownership validated before deserialization
- PDA validation ensures correct derivation

### Authorization
- Signer checks on all sensitive operations
- Two-step program authority transfer (TransferProgramAuthority + AcceptProgramAuthority)
- Global admin model: ProgramAuthority controls pool creation and updates
- Authorized creators list allows delegation (up to 10 addresses)
- Admin verification required for UpdatePool operations

### Economic Security
- Pre-flight reward solvency checks
- Checked arithmetic (no overflow/underflow)
- Transfer fee support for Token-2022
- Freeze authority validation

### Time-Locked Operations
```mermaid
graph LR
    A[Authority proposes<br/>reward rate change] -->|UpdatePool| B[Pending for 7 days]
    B -->|Users can exit| C[FinalizeRewardRateChange]
    C -->|Anyone can call| D[New rate active]
    
    style B fill:#fff4e1
    style D fill:#e1ffe1
```

## Data Flow

### Reward Calculation
```
Expected Rewards = (amount_staked × reward_rate) ÷ REWARD_SCALE
Where REWARD_SCALE = 1_000_000_000 (1e9)

Example:
- Stake: 1000 tokens
- Rate: 100_000_000 (10% when divided by 1e9)
- Rewards: (1000 × 100_000_000) ÷ 1_000_000_000 = 100 tokens
```

### Pool Solvency Tracking
```
total_staked: Sum of all user stakes
total_rewards_owed: Sum of all expected rewards
reward_vault.balance: Actual tokens available

Invariant: reward_vault.balance ≥ total_rewards_owed
```

## Extension Points

### Multiple Pools
Pools are identified by `(stake_mint, pool_id)` allowing:
- Multiple reward rates for the same token
- Different lockup periods
- Separate user segments (VIP vs standard)

**Authorization**: Only addresses authorized by ProgramAuthority can create pools:
- Main authority (always authorized)
- Up to 10 additional addresses in authorized_creators list
- Managed via ManageAuthorizedCreators instruction

### Token-2022 Support
- Transfer fee handling via balance checking
- Extension validation (blocks dangerous extensions)
- Forward-compatible with new token standards

## Error Handling

The program uses custom error types for clear failure modes:
- `PoolPaused`: Operations blocked when pool is paused
- `InsufficientRewards`: Not enough rewards in vault
- `LockupNotExpired`: Early withdrawal when enforce_lockup=true
- `PoolParametersChanged`: Frontrunning protection triggered

## Performance Considerations

- **Account Size**: ProgramAuthority = 398 bytes, StakePool = 223 bytes, StakeAccount = 98 bytes
- **Compute Units**: Typical operations < 200k CU
- **Rent-Exempt Minimum**: 
  - ProgramAuthority: ~3.2M lamports (created once per program deployment)
  - StakePool: ~2.2M lamports
  - StakeAccount: ~1.2M lamports
- **Concurrent Operations**: Lock-free design allows parallel stakes

## Upgrade Path

The program uses reserved fields for future upgrades:
```rust
pub struct StakePool {
    // ... existing fields
    pub reserved: [u8; 7],  // For future use
}

pub struct ProgramAuthority {
    // ... existing fields  
    pub reserved: [u8; 32],  // For future use
}
```

**Breaking Changes**: Require full migration (drain → close → redeploy)

**Global Admin Model**: The ProgramAuthority account persists across program upgrades, maintaining administrative control and authorized creator lists.

## Monitoring & Observability

Events emitted for off-chain indexing:
- `InitializePool`: New pool created
- `Stake`: User stakes tokens
- `Unstake`: User withdraws tokens
- `ClaimRewards`: User claims rewards
- `FundRewards`: Pool funded with rewards

These events enable real-time notifications and analytics via Helius, TheGraph, or custom indexers.
