# @yourwallet/stake-pool

[![npm version](https://img.shields.io/npm/v/@yourwallet/stake-pool.svg)](https://www.npmjs.com/package/@yourwallet/stake-pool)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A TypeScript/JavaScript client library for the YW Stake Pool program on Solana. This library provides a complete, type-safe interface for interacting with stake pools, enabling token staking, reward management, and pool administration.

## Features

- ✅ **Full TypeScript Support** - Complete type definitions for all instructions and accounts
- 🔐 **Type-Safe** - Auto-generated from program IDL with strong typing
- 📦 **Tree-Shakeable** - ESM and CommonJS support with optimized bundles
- 🎯 **Solana Kit** - Built with @solana/kit for modern Solana development
- 🧪 **Well Tested** - Comprehensive test coverage
- 📚 **Full API Documentation** - JSDoc comments and TypeDoc generation

## Installation

```bash
npm install @yourwallet/stake-pool @solana/kit
```

```bash
yarn add @yourwallet/stake-pool @solana/kit
```

```bash
pnpm add @yourwallet/stake-pool @solana/kit
```

## Quick Start

```typescript
import { createSolanaRpc, address, generateKeyPairSigner } from '@solana/kit';
import { 
  getInitializePoolInstruction,
  getStakeInstruction,
  getClaimRewardsInstruction 
} from '@yourwallet/stake-pool';

// Create RPC client
const rpc = createSolanaRpc('https://api.devnet.solana.com');

// Initialize a new stake pool
const initPoolIx = getInitializePoolInstruction({
  pool: poolPda,
  authority: authority.address,
  stakeMint: stakeMintAddress,
  rewardMint: rewardMintAddress,
  stakeVault: stakeVaultAddress,
  rewardVault: rewardVaultAddress,
  payer: payer.address,
  tokenProgram: tokenProgramAddress,
  poolId: 0n, // Unique ID (0 for first pool, 1 for second, etc.)
  rewardRate: 100_000_000n, // 10% APY
  minStakeAmount: 1_000_000n, // 1 token (6 decimals)
  lockupPeriod: 86400n, // 1 day in seconds
  poolEndDate: null,
});

// Stake tokens
const stakeIx = getStakeInstruction({
  pool: poolAddress,
  stakeAccount: stakeAccountAddress,
  owner: owner.address,
  userTokenAccount: userTokenAddress,
  stakeVault: stakeVaultAddress,
  rewardVault: rewardVaultAddress,
  stakeMint: stakeMintAddress,
  tokenProgram: tokenProgramAddress,
  payer: payer.address,
  amount: 10_000_000n, // 10 tokens
  index: 0n,
  expectedRewardRate: null,
  expectedLockupPeriod: null,
});

// Claim rewards
const claimIx = getClaimRewardsInstruction({
  pool: poolAddress,
  stakeAccount: stakeAccountAddress,
  owner: owner.address,
  userRewardAccount: userRewardAddress,
  rewardVault: rewardVaultAddress,
  rewardMint: rewardMintAddress,
  tokenProgram: tokenProgramAddress,
});
```

## Available Instructions

### Pool Management

#### `getInitializePoolInstruction()`
Initialize a new stake pool with custom parameters.

```typescript
const initIx = getInitializePoolInstruction({
  pool: poolPda,
  authority: authority.address,
  stakeMint: stakeMintAddress,
  rewardMint: rewardMintAddress,
  stakeVault: stakeVaultAddress,
  rewardVault: rewardVaultAddress,
  payer: payer.address,
  tokenProgram: tokenProgramAddress,
  poolId: 0n, // Unique ID (0 for first pool, 1 for second, etc.)
  rewardRate: 100_000_000n, // 10% APY (basis points * 10^6)
  minStakeAmount: 1_000_000n,
  lockupPeriod: 86400n, // 1 day
  poolEndDate: null, // or some(timestamp)
});
```

#### `getUpdatePoolInstruction()`
Update pool parameters (authority only).

```typescript
import { some, none } from '@solana/kit';

const updateIx = getUpdatePoolInstruction({
  pool: poolAddress,
  authority: authority.address,
  rewardRate: some(150_000_000n), // Update to 15% APY
  minStakeAmount: some(2_000_000n), // Update minimum stake
  lockupPeriod: none(), // Don't change lockup
  isPaused: some(false), // Unpause pool
  poolEndDate: none(),
});
```

#### `getFundRewardsInstruction()`
Add rewards to the pool's reward vault.

```typescript
const fundIx = getFundRewardsInstruction({
  pool: poolAddress,
  funder: funder.address,
  funderTokenAccount: funderTokenAddress,
  rewardVault: rewardVaultAddress,
  rewardMint: rewardMintAddress,
  tokenProgram: tokenProgramAddress,
  amount: 1_000_000_000n, // 1000 tokens
});
```

### Staking Operations

#### `getInitializeStakeAccountInstruction()`
Create a new stake account for a user.

```typescript
const initStakeIx = getInitializeStakeAccountInstruction({
  stakeAccount: stakeAccountPda,
  pool: poolAddress,
  owner: owner.address,
  payer: payer.address,
  index: 0n, // First stake account for this user
});
```

#### `getStakeInstruction()`
Stake tokens into the pool.

```typescript
const stakeIx = getStakeInstruction({
  pool: poolAddress,
  stakeAccount: stakeAccountAddress,
  owner: owner.address,
  userTokenAccount: userTokenAddress,
  stakeVault: stakeVaultAddress,
  rewardVault: rewardVaultAddress,
  stakeMint: stakeMintAddress,
  tokenProgram: tokenProgramAddress,
  payer: payer.address,
  amount: 100_000_000n, // 100 tokens
  index: 0n,
  expectedRewardRate: null, // Optional frontrunning protection
  expectedLockupPeriod: null, // Optional frontrunning protection
});
```

#### `getUnstakeInstruction()`
Unstake tokens from the pool.

```typescript
const unstakeIx = getUnstakeInstruction({
  pool: poolAddress,
  stakeAccount: stakeAccountAddress,
  owner: owner.address,
  userTokenAccount: userTokenAddress,
  stakeVault: stakeVaultAddress,
  stakeMint: stakeMintAddress,
  tokenProgram: tokenProgramAddress,
  amount: 50_000_000n, // Partial unstake (50 tokens)
  expectedRewardRate: null, // Optional frontrunning protection
});
```

#### `getClaimRewardsInstruction()`
Claim accrued rewards.

```typescript
const claimIx = getClaimRewardsInstruction({
  pool: poolAddress,
  stakeAccount: stakeAccountAddress,
  owner: owner.address,
  userRewardAccount: userRewardAddress,
  rewardVault: rewardVaultAddress,
  rewardMint: rewardMintAddress,
  tokenProgram: tokenProgramAddress,
});
```

### Authority Management

#### `getNominateNewAuthorityInstruction()`
Nominate a new authority for the pool (step 1 of 2).

```typescript
const nominateIx = getNominateNewAuthorityInstruction({
  pool: poolAddress,
  authority: currentAuthority.address,
  newAuthority: newAuthority.address,
});
```

#### `getAcceptAuthorityInstruction()`
Accept authority nomination (step 2 of 2).

```typescript
const acceptIx = getAcceptAuthorityInstruction({
  pool: poolAddress,
  newAuthority: newAuthority.address,
});
```

## Account Types

### StakePool
The main pool account containing all pool configuration.

```typescript
import { getStakePoolAccountDataCodec } from '@yourwallet/stake-pool';

const codec = getStakePoolAccountDataCodec();
const poolData = codec.decode(accountData);

// Access pool properties
console.log(poolData.authority);       // Pool authority
console.log(poolData.stakeMint);      // Stake token mint
console.log(poolData.rewardMint);     // Reward token mint
console.log(poolData.totalStaked);    // Total staked amount
console.log(poolData.rewardRate);     // Current reward rate
console.log(poolData.minStakeAmount); // Minimum stake
console.log(poolData.lockupPeriod);   // Lockup period in seconds
console.log(poolData.isPaused);       // Pool pause status
```

### StakeAccount
Individual user stake account.

```typescript
import { getStakeAccountAccountDataCodec } from '@yourwallet/stake-pool';

const codec = getStakeAccountAccountDataCodec();
const stakeData = codec.decode(accountData);

// Access stake properties
console.log(stakeData.pool);          // Pool address
console.log(stakeData.owner);         // Stake owner
console.log(stakeData.stakedAmount);  // Amount staked
console.log(stakeData.rewardRate);    // Rate at stake time
console.log(stakeData.stakeTimestamp);// When staked
console.log(stakeData.lastClaimTime); // Last reward claim
console.log(stakeData.index);         // Stake account index
```

## Error Handling

The library exports all program errors with descriptive names:

```typescript
import { 
  STAKE_POOL_ERROR__POOL_IS_PAUSED,
  STAKE_POOL_ERROR__INVALID_AUTHORITY,
  STAKE_POOL_ERROR__LOCKUP_NOT_EXPIRED 
} from '@yourwallet/stake-pool';

try {
  // Send transaction
} catch (error) {
  if (error.code === STAKE_POOL_ERROR__POOL_IS_PAUSED) {
    console.log('Pool is currently paused');
  }
}
```

## PDA Derivation

Derive Program Derived Addresses (PDAs) for pools and stake accounts:

```typescript
import { getAddressEncoder, getProgramDerivedAddress } from '@solana/kit';

// Pool PDA (includes poolId for multi-pool support)
const poolId = 0n; // First pool

// Encode poolId as little-endian u64
const poolIdBytes = new Uint8Array(8);
new DataView(poolIdBytes.buffer).setBigUint64(0, poolId, true); // true = little-endian

const [poolPda] = await getProgramDerivedAddress({
  programAddress: programId,
  seeds: [
    'stake_pool',
    getAddressEncoder().encode(authority),
    getAddressEncoder().encode(stakeMint),
    poolIdBytes,
  ],
});

// Stake Account PDA
// Encode index as little-endian u64
const indexBytes = new Uint8Array(8);
new DataView(indexBytes.buffer).setBigUint64(0, index, true); // true = little-endian

const [stakeAccountPda] = await getProgramDerivedAddress({
  programAddress: programId,
  seeds: [
    'stake_account',
    getAddressEncoder().encode(pool),
    getAddressEncoder().encode(owner),
    indexBytes,
  ],
});
```

### Multiple Pools Per Token

A single authority can create multiple stake pools for the same token by using different `poolId` values:

```typescript
// Encode poolId as little-endian u64
const poolIdBytes1 = new Uint8Array(8);
new DataView(poolIdBytes1.buffer).setBigUint64(0, 0n, true);

// First pool (standard staking)
const [pool1] = await getProgramDerivedAddress({
  programAddress: programId,
  seeds: [
    'stake_pool',
    getAddressEncoder().encode(authority),
    getAddressEncoder().encode(stakeMint),
    poolIdBytes1,
  ],
});

// Encode second poolId
const poolIdBytes2 = new Uint8Array(8);
new DataView(poolIdBytes2.buffer).setBigUint64(0, 1n, true);

// Second pool (VIP staking with higher rewards)
const [pool2] = await getProgramDerivedAddress({
  programAddress: programId,
  seeds: [
    'stake_pool',
    getAddressEncoder().encode(authority),
    getAddressEncoder().encode(stakeMint),
    poolIdBytes2,
  ],
});
```

## Examples

### Complete Staking Flow

```typescript
import {
  createSolanaRpc,
  generateKeyPairSigner,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
} from '@solana/kit';
import {
  getInitializeStakeAccountInstruction,
  getStakeInstruction,
} from '@yourwallet/stake-pool';

const rpc = createSolanaRpc('https://api.devnet.solana.com');
const owner = await generateKeyPairSigner();

// Step 1: Initialize stake account
const initStakeIx = getInitializeStakeAccountInstruction({
  stakeAccount: stakeAccountPda,
  pool: poolAddress,
  owner: owner.address,
  payer: owner.address,
  index: 0n,
});

// Step 2: Stake tokens
const stakeIx = getStakeInstruction({
  pool: poolAddress,
  stakeAccount: stakeAccountPda,
  owner: owner.address,
  userTokenAccount: userTokenAddress,
  stakeVault: stakeVaultAddress,
  rewardVault: rewardVaultAddress,
  stakeMint: stakeMintAddress,
  tokenProgram: tokenProgramAddress,
  payer: owner.address,
  amount: 100_000_000n,
  index: 0n,
  expectedRewardRate: null,
  expectedLockupPeriod: null,
});

// Build and send transaction
const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

const transactionMessage = pipe(
  createTransactionMessage({ version: 0 }),
  tx => setTransactionMessageFeePayerSigner(owner, tx),
  tx => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
  tx => ({ ...tx, instructions: [initStakeIx, stakeIx] })
);

const signedTx = await signTransactionMessageWithSigners(transactionMessage);
const signature = await rpc.sendTransaction(signedTx).send();

console.log('Transaction:', signature);
```

## Development

### Building the Client

```bash
# From repository root
pnpm clients:js:build

# Or from client directory
cd clients/js
pnpm build
```

### Running Tests

```bash
# From repository root (starts local validator)
pnpm clients:js:test

# Run devnet integration tests
pnpm clients:js:test:devnet
```

### Linting and Formatting

```bash
pnpm clients:js:lint        # Check for issues
pnpm clients:js:lint:fix    # Auto-fix issues
pnpm clients:js:format      # Check formatting
pnpm clients:js:format:fix  # Auto-format code
```

### Generating Documentation

```bash
cd clients/js
pnpm build:docs
```

## API Reference

Full API documentation is available at: [TypeDoc Documentation](https://yourwalletio.github.io/yw-stake-pool/)

## Program Information

- **Program ID**: `8PtjrGvKNeZt2vCmRkSPGjss7TAFhvxux2N8r67UMKBx`
- **Version**: `1.5.0`
- **Network**: Devnet, Mainnet
- **Cluster**: Solana

## Security

This client library interacts with a security-audited program that implements:
- Type cosplay protection
- Frontrunning protection (via optional parameter verification)
- Account validation
- Transfer fee support (Token-2022)
- Two-step authority transfer

## Support

- **Issues**: [GitHub Issues](https://github.com/yourwalletio/yw-stake-pool/issues)
- **Documentation**: [Program README](../../README.md)
- **Security**: See [SECURITY_AUDIT.md](../../SECURITY_AUDIT.md)

## License

MIT License - see [LICENSE](../../LICENSE) for details.

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

---

**Built with** ❤️ **using** [Codama](https://github.com/codama-idl/codama) **and** [@solana/kit](https://github.com/solana-program/kit)
