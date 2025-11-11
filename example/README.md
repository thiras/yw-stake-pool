# YW Stake Pool - JavaScript Client Examples

This directory contains comprehensive examples demonstrating how to use the `@yourwallet/stake-pool` JavaScript client library to interact with the YW Stake Pool program on Solana.

## Overview

The examples demonstrate the stake pool program functionality:

1. **Simple Example** - Basic instruction creation (no network calls)
2. **Pool Administration** - Pool management operations (instruction demos)
3. **User Staking** - Staking workflow examples (instruction demos)
4. **Devnet Integration Test** - Live end-to-end test on Solana devnet

## Prerequisites

- Node.js >= 20.0.0
- pnpm (recommended) or npm
- A local Solana validator or access to devnet/mainnet
- SOL for transaction fees
- SPL tokens for staking

## Installation

From the repository root:

```bash
# Install dependencies
pnpm install

# Build the client library
pnpm clients:js:build

# Install example dependencies
cd example
pnpm install
```

## Running the Examples

### Quick Start

```bash
# Simple example (no network calls - instruction creation only)
pnpm simple

# Pool administration examples (instruction demos)
pnpm pool-admin

# User staking examples (instruction demos)
pnpm user-staking

# Live integration test on devnet (executes real transactions)
pnpm devnet-test

# Check if a pool exists for an authority + stake mint
pnpm list-pools <authority> <stake_mint>

# List ALL pools on the network (no parameters needed)
pnpm list-all-pools
```

### Development Mode

Run the default test (devnet integration):

```bash
pnpm dev
```

### Production Mode

Build and run:

```bash
pnpm build
pnpm start
```

## Examples

### 1. Simple Example (`src/simple-example.ts`)

A minimal example showing basic library usage without network calls. Demonstrates instruction creation for:

- Initialize Pool
- Stake tokens
- Claim rewards

**Best for:** Understanding the API without needing a running network.

```bash
pnpm simple
```

### 2. Pool Administration (`src/pool-admin.ts`)

Demonstrates pool operator/admin operations with instruction creation:

- Initialize new pools with custom parameters
- Fund reward vaults
- Update pool settings (reward rate, lockup, minimum stake)
- Pause and unpause pools
- Set pool end dates
- Transfer authority (two-step process)

**Best for:** Pool operators and administrators.

```bash
pnpm pool-admin
```

### 3. User Staking (`src/user-staking.ts`)

Demonstrates typical user interactions with instruction creation:

- Stake tokens (basic and with frontrunning protection)
  - Automatically creates stake accounts if they don't exist
- Claim accrued rewards
- Partial and full unstaking
- Close empty stake accounts to recover rent
- Multiple stake accounts with different indices

**Best for:** Users and frontend developers.

```bash
pnpm user-staking
```

### 4. List Pools (`src/list-pools.ts`)

A utility script to check if a stake pool exists for a given authority and stake mint combination.

**Features:**
- Derives the correct pool PDA address
- Checks if the pool account exists on-chain
- Displays detailed pool information including:
  - Pool configuration (reward rate, lockup period, min stake)
  - Current state (total staked, rewards owed)
  - Pool status (active/paused)
  - Authority and vault addresses

**Usage:**

```bash
# Check a specific pool
pnpm list-pools <authority_pubkey> <stake_mint_address>

# Example
pnpm list-pools 7xYz3pZD6qvL8... TokenkegQfeZyiNwAJ...
```

**Note:** In the current implementation, each authority + stake_mint combination can have only **one** pool. This script helps you verify if a pool already exists before trying to create one.

**Best for:** Pool operators checking existing pools, debugging, and monitoring.

### 5. List All Pools (`src/list-all-pools.ts`)

A utility script to discover and display **ALL** stake pools on the network, regardless of authority or stake mint.

**Features:**
- Scans the entire blockchain for all program-owned accounts
- Filters and decodes valid stake pool accounts
- Displays comprehensive information for each pool
- Groups pools by authority
- Shows network-wide statistics:
  - Total number of pools
  - Active vs paused pools
  - Number of unique authorities
  - Total value locked across all pools
  - Total rewards owed

**Usage:**

```bash
# Scan all pools on the configured network
pnpm list-all-pools
```

**Note:** This script uses `getProgramAccounts` RPC call which:
- May take several seconds to complete
- Might not be available on all public RPC endpoints
- Works best with local validators or premium RPC services

**Output Example:**
```
Found 5 Stake Pool(s)

1. Pool ABC123...
   Authority: XYZ...
   Total Staked: 1,250.50 tokens
   Reward Rate: 15.00%
   Status: ✅ ACTIVE

[... more pools ...]

Summary by Authority:
XYZ... → 3 pool(s), Total Staked: 2,500.00 tokens

Overall Statistics:
Total Pools: 5
Active Pools: 4
Paused Pools: 1
Total Value Staked: 5,000.00 tokens
```

**Best for:** Network analysis, monitoring, discovering all pools, and getting ecosystem-wide statistics.

### 6. Devnet Integration Test (`src/devnet-integration-test.ts`)

A **live end-to-end test** that executes real transactions on Solana devnet:

- Creates real SPL tokens (stake and reward mints)
- Initializes a stake pool on devnet
- Funds the reward vault
- Stakes tokens with automatic account creation
- Claims rewards after lockup
- Unstakes tokens
- Updates pool parameters

**Best for:** Validating the complete workflow on a live network.

**⚠️ Note:** This test requires devnet SOL and executes actual transactions.

```bash
pnpm devnet-test
```

## Code Structure

```
example/
├── src/
│   ├── index.ts                     # Main entry point
│   ├── simple-example.ts            # Basic instruction creation
│   ├── pool-admin.ts                # Admin operation demos
│   ├── user-staking.ts              # User staking demos  
│   ├── devnet-integration-test.ts   # Live devnet test
│   ├── setup-tokens.ts              # Token creation helper
│   ├── utils.ts                     # Common utilities
│   └── config.ts                    # Configuration
├── package.json
├── tsconfig.json
└── README.md
```

## Key Concepts

### Program Derived Addresses (PDAs)

The stake pool uses PDAs for deterministic account addressing:

```typescript
// Pool PDA (includes poolId for multi-pool support)
const poolId = 0n; // First pool (use 1n, 2n, etc. for additional pools)
const [poolAddress] = await findPoolPda(
  stakeMint,
  poolId,
  programId
);

// Stake Account PDA
const [stakeAccountAddress] = await findStakeAccountPda(
  poolAddress,
  owner.address,
  index,
  programId
);
```

### Reward Calculation

Rewards are calculated based on:
- **Amount Staked**: The amount of tokens the user has staked
- **Reward Rate**: Expressed as basis points (e.g., 100_000_000 = 10%)
- **Lockup Period**: Time that must pass before rewards are earned

Formula: `rewards = (amount_staked * reward_rate) / 1e9`

### Transaction Building

All examples use @solana/kit for modern transaction building:

```typescript
import { 
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  pipe
} from '@solana/kit';

const message = pipe(
  createTransactionMessage({ version: 0 }),
  tx => setTransactionMessageFeePayerSigner(payer, tx),
  tx => setTransactionMessageLifetimeUsingBlockhash(blockhash, tx),
  tx => ({ ...tx, instructions: [ix1, ix2] })
);
```

## Configuration

Edit `src/config.ts` to customize:

```typescript
export const config = {
  // RPC endpoint
  rpcUrl: 'https://solana-devnet.g.alchemy.com/v2/YOUR_KEY', // Devnet
  
  // Program ID (deployed on devnet)
  programId: '8PtjrGvKNeZt2vCmRkSPGjss7TAFhvxux2N8r67UMKBx',
  
  // Pool parameters
  defaultPoolConfig: {
    rewardRate: 100_000_000n,      // 10% APY
    minStakeAmount: 1_000_000n,    // 1 token (6 decimals)
    lockupPeriod: 86400n,          // 1 day in seconds
  },
  
  // Keypair configuration
  useLocalKeypair: true,         // Use your local Solana keypair
  
  // Rate limiting (for devnet)
  rateLimitDelay: 10000,         // 10 seconds between operations
};
```

### Rate Limiting

**Public RPC endpoints** (like devnet) have rate limits. The devnet integration test includes 10-second delays between operations to prevent `429 Too Many Requests` errors.

- **Devnet**: 10000ms (10 seconds) recommended
- **Local validator**: Set to `0` to disable
- **Premium RPC**: Adjust based on your provider's limits

### Using Your Local Keypair

**By default**, examples use your local Solana keypair at `~/.config/solana/id.json` (same as Solana CLI).

**To use a different keypair:**

```bash
# Option 1: Environment variable
export SOLANA_KEYPAIR_PATH=/path/to/your/keypair.json

# Option 2: Set in config.ts
customKeypairPath: '/path/to/your/keypair.json'

# Option 3: Disable (generates new keypairs)
useLocalKeypair: false
```

**Check your current keypair:**
```bash
solana address
solana balance
```

## Testing on Devnet

The devnet integration test is pre-configured for Solana devnet:

1. Ensure you have devnet SOL:
   ```bash
   solana airdrop 2 --url devnet
   ```

2. Run the integration test:
   ```bash
   pnpm devnet-test
   ```

The test will:
- Create real SPL tokens on devnet
- Execute all stake pool operations
- Display transaction signatures
- Complete in ~2 minutes (with rate limiting)

## Common Operations

### Initialize a Pool

```typescript
import { getInitializePoolInstruction } from '@yourwallet/stake-pool';

const instruction = getInitializePoolInstruction({
  pool: poolAddress,
  authority: authority,
  stakeMint,
  rewardMint,
  stakeVault,
  rewardVault,
  payer: authority,
  tokenProgram: TOKEN_PROGRAM_ID,
  systemProgram: SYSTEM_PROGRAM_ID,
  rent: SYSVAR_RENT_PUBKEY,
  poolId: 0n, // First pool (use 1n, 2n, etc. for additional pools)
  rewardRate: 100_000_000n,     // 10%
  minStakeAmount: 1_000_000n,   // 1 token
  lockupPeriod: 86400n,         // 1 day
  poolEndDate: null,
});
```

### Stake Tokens

```typescript
import { getStakeInstruction } from '@yourwallet/stake-pool';

// Stake instruction automatically creates the stake account if it doesn't exist
const instruction = getStakeInstruction({
  pool: poolAddress,
  stakeAccount: stakeAccountAddress,
  owner: user,
  userTokenAccount,
  stakeVault,
  rewardVault,
  stakeMint,
  tokenProgram: TOKEN_PROGRAM_ID,
  payer: user,
  systemProgram: SYSTEM_PROGRAM_ID,
  amount: 100_000_000n, // 100 tokens
  index: 0n,             // Increment for multiple deposits per user
  expectedRewardRate: null,      // Optional frontrunning protection
  expectedLockupPeriod: null,    // Optional frontrunning protection
});
```

### Claim Rewards

```typescript
import { getClaimRewardsInstruction } from '@yourwallet/stake-pool';

const instruction = getClaimRewardsInstruction({
  pool: poolAddress,
  stakeAccount: stakeAccountAddress,
  owner: user,
  userRewardAccount,
  rewardVault,
  rewardMint,
  tokenProgram: TOKEN_PROGRAM_ID,
  clock: SYSVAR_CLOCK_PUBKEY,
});
```

### Close Stake Account

After unstaking all tokens, you can close the stake account to recover rent:

```typescript
import { getCloseStakeAccountInstruction } from '@yourwallet/stake-pool';

// Can only close when staked_amount is 0
const instruction = getCloseStakeAccountInstruction({
  stakeAccount: stakeAccountAddress,
  owner: user,
  receiver: user.address, // Where to send the rent SOL
});
```

## Error Handling

The client exports all program errors:

```typescript
import { 
  STAKE_POOL_ERROR__POOL_IS_PAUSED,
  STAKE_POOL_ERROR__LOCKUP_NOT_EXPIRED,
  STAKE_POOL_ERROR__INSUFFICIENT_REWARDS
} from '@yourwallet/stake-pool';

try {
  await sendTransaction(instruction);
} catch (error) {
  if (error.code === STAKE_POOL_ERROR__POOL_IS_PAUSED) {
    console.error('Cannot stake: pool is paused');
  }
}
```

## Security Features

The examples demonstrate:

1. **Frontrunning Protection** - Optional parameter verification for stake/unstake
2. **Two-Step Authority Transfer** - Nominate and accept pattern
3. **Account Validation** - Proper PDA derivation and verification
4. **Token-2022 Support** - Compatible with transfer fee tokens

## Troubleshooting

### "Account not found" errors

- Ensure accounts are initialized before use
- Verify PDA derivations match program expectations
- Check that addresses are correct

### "Insufficient funds" errors

- Airdrop SOL for transaction fees
- Ensure token accounts have sufficient balance
- Fund reward vault before staking

### "Custom program error" messages

- Check the error code against exported error constants
- Review program logs for detailed error messages
- Verify all account addresses and parameters

## Resources

- [Program Documentation](../README.md)
- [Client Library Documentation](../clients/js/README.md)
- [Security Audit](../SECURITY_AUDIT.md)
- [Solana Documentation](https://docs.solana.com)

## License

MIT - See [LICENSE](../LICENSE) for details.
