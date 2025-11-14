# YW Stake Pool - JavaScript Client Examples

This directory contains examples demonstrating how to use the `@yourwallet/stake-pool` JavaScript client library to interact with the YW Stake Pool program on Solana.

## Overview

The examples demonstrate the stake pool program functionality:

1. **Devnet Integration Test** - Live end-to-end test on Solana devnet
2. **List All Pools** - Utility to discover all stake pools on the network

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
# Live integration test on devnet (executes real transactions)
pnpm devnet-test

# List ALL pools on the network (no parameters needed)
pnpm list-all-pools
```

### Development Mode

Run the default test (devnet integration):

```bash
pnpm dev
```

## Examples

### 1. Devnet Integration Test (`src/devnet-integration-test.ts`)

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

### 2. List All Pools (`src/list-all-pools.ts`)

A utility script to discover and display **ALL** stake pools on the network.

**Features:**
- Scans the entire blockchain for all program-owned accounts
- Filters and decodes valid stake pool accounts
- Displays comprehensive information for each pool
- Groups pools by authority
- Shows network-wide statistics:
  - Total number of pools
  - Active vs paused pools
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

**Best for:** Network analysis, monitoring, discovering all pools, and getting ecosystem-wide statistics.

## Code Structure

```
example/
├── src/
│   ├── index.ts                     # Main entry point
│   ├── devnet-integration-test.ts   # Live devnet test
│   ├── list-all-pools.ts            # Network pool scanner
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
- [Security Audit](../audit/SECURITY_AUDIT.md)
- [Solana Documentation](https://docs.solana.com)

## License

MIT - See [LICENSE](../LICENSE) for details.
