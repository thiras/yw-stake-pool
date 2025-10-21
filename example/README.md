# YW Stake Pool - JavaScript Client Examples

This directory contains comprehensive examples demonstrating how to use the `@yourwallet/stake-pool` JavaScript client library to interact with the YW Stake Pool program on Solana.

## Overview

The examples cover all major functionality of the stake pool program:

1. **Pool Administration** - Initialize and manage stake pools
2. **User Staking** - Stake tokens, claim rewards, and unstake
3. **Complete Flow** - End-to-end example with all operations
4. **Helper Utilities** - Common functions for PDA derivation and setup

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

### Development Mode (with tsx)

Run examples directly without building:

```bash
# Complete flow example (recommended to start)
pnpm dev

# Or run specific examples:
pnpm complete-flow    # Full stake pool lifecycle
pnpm pool-admin       # Pool management operations
pnpm user-staking     # Staking and reward claiming
```

### Production Mode

Build and run:

```bash
pnpm build
pnpm start
```

## Examples

### 1. Complete Flow (`src/complete-flow.ts`)

A comprehensive example that demonstrates the entire stake pool lifecycle:

- Create test tokens (stake and reward tokens)
- Initialize a new stake pool
- Fund the reward vault
- Initialize user stake accounts
- Stake tokens
- Wait for lockup period
- Claim rewards
- Unstake tokens
- Update pool parameters
- Transfer pool authority

This is the **recommended starting point** to understand how all pieces work together.

### 2. Pool Administration (`src/pool-admin.ts`)

Focused on pool operator/admin operations:

- Initialize new pools with custom parameters
- Update pool settings (reward rate, lockup period, minimum stake)
- Pause and unpause pools
- Fund reward vaults
- Transfer authority (two-step process)
- Set pool end dates

### 3. User Staking (`src/user-staking.ts`)

Demonstrates typical user interactions:

- Initialize stake accounts
- Stake tokens
- Check stake status
- Claim accrued rewards
- Partial unstaking
- Full unstaking after lockup

### 4. Helper Utilities (`src/utils.ts`)

Common helper functions used across examples:

- PDA derivation for pools and stake accounts
- Airdrop SOL for testing
- Create test tokens
- Get or create associated token accounts
- Build and send transactions
- Calculate expected rewards

## Code Structure

```
example/
├── src/
│   ├── index.ts              # Main entry point
│   ├── complete-flow.ts      # Full lifecycle example
│   ├── pool-admin.ts         # Admin operations
│   ├── user-staking.ts       # User staking operations
│   ├── utils.ts              # Helper functions
│   └── config.ts             # Configuration constants
├── package.json
├── tsconfig.json
└── README.md
```

## Key Concepts

### Program Derived Addresses (PDAs)

The stake pool uses PDAs for deterministic account addressing:

```typescript
// Pool PDA
const [poolAddress] = await findPoolPda(
  authority.address,
  stakeMint,
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
  rpcUrl: 'http://127.0.0.1:8899', // Local validator
  // rpcUrl: 'https://api.devnet.solana.com', // Devnet
  
  // Program ID
  programId: 'YOUR_PROGRAM_ID',
  
  // Pool parameters
  rewardRate: 100_000_000n,      // 10% APY
  minStakeAmount: 1_000_000n,    // 1 token (6 decimals)
  lockupPeriod: 86400n,          // 1 day in seconds
};
```

## Testing on Local Validator

1. Start the local validator:
   ```bash
   cd .. # Back to repo root
   pnpm validator:start
   ```

2. Build and deploy the program:
   ```bash
   pnpm programs:build
   pnpm programs:deploy
   ```

3. Run the examples:
   ```bash
   cd example
   pnpm dev
   ```

## Testing on Devnet

1. Update RPC URL in `src/config.ts` to devnet
2. Ensure you have devnet SOL
3. Update program ID to deployed devnet program
4. Run examples

## Common Operations

### Initialize a Pool

```typescript
import { getInitializePoolInstruction } from '@yourwallet/stake-pool';

const instruction = getInitializePoolInstruction({
  pool: poolAddress,
  authority: authority.address,
  stakeMint: stakeMintAddress,
  rewardMint: rewardMintAddress,
  stakeVault: stakeVaultAddress,
  rewardVault: rewardVaultAddress,
  payer: payer.address,
  tokenProgram: TOKEN_PROGRAM_ID,
  rewardRate: 100_000_000n,     // 10%
  minStakeAmount: 1_000_000n,   // 1 token
  lockupPeriod: 86400n,         // 1 day
  poolEndDate: null,
});
```

### Stake Tokens

```typescript
import { getStakeInstruction } from '@yourwallet/stake-pool';

const instruction = getStakeInstruction({
  pool: poolAddress,
  stakeAccount: stakeAccountAddress,
  owner: owner.address,
  ownerStakeTokenAccount: userTokenAccount,
  stakeVault: stakeVaultAddress,
  tokenProgram: TOKEN_PROGRAM_ID,
  amount: 100_000_000n, // 100 tokens
  index: 0n,
  expectedRewardRate: null,      // Frontrunning protection (optional)
  expectedLockupPeriod: null,    // Frontrunning protection (optional)
});
```

### Claim Rewards

```typescript
import { getClaimRewardsInstruction } from '@yourwallet/stake-pool';

const instruction = getClaimRewardsInstruction({
  pool: poolAddress,
  stakeAccount: stakeAccountAddress,
  owner: owner.address,
  ownerRewardTokenAccount: userRewardAccount,
  rewardVault: rewardVaultAddress,
  tokenProgram: TOKEN_PROGRAM_ID,
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
