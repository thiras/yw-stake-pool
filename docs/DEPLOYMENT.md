# YW Stake Pool - Deployment Guide

This guide walks you through deploying the YW Stake Pool program to Solana clusters (devnet, testnet, or mainnet-beta).

## Table of Contents

- [Prerequisites](#prerequisites)
- [Deployment Steps](#deployment-steps)
- [Post-Deployment Setup](#post-deployment-setup)
- [Verification](#verification)
- [Managing Authorities](#managing-authorities)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Tools

- **Solana CLI** (v1.18.22 or compatible)
- **Rust toolchain** (edition 2021)
- **Node.js** (v20+)
- **pnpm** (v9.1.0)

### Required Accounts

1. **Deployer Keypair**: Account with sufficient SOL for deployment
   - Devnet: ~5 SOL (use `solana airdrop`)
   - Testnet: ~5 SOL (use `solana airdrop`)
   - Mainnet: ~5-10 SOL (for program deployment and authority initialization)

2. **Authority Keypair**: Will control:
   - Program upgrades (upgrade authority) - separate from program authority
   - Pool creation permissions (program authority) - global admin for entire system
   - Authorized creator management (can delegate pool creation to other addresses)
   - Can be the same as deployer or separate for security

### Install Dependencies

```bash
# Install all dependencies and build client
pnpm install:all

# Verify Solana version
pnpm solana:check
```

---

## Deployment Steps

### Step 1: Build the Program

Build the Solana program:

```bash
pnpm programs:build
```

This compiles the Rust program and generates the `.so` binary in `target/deploy/`.

**Verify Build:**
```bash
ls -lh target/deploy/*.so
# Should see: your_wallet_stake_pool.so
```

### Step 2: Configure Cluster

Set your target cluster:

```bash
# For devnet (recommended for testing)
solana config set --url devnet

# For testnet
solana config set --url testnet

# For mainnet-beta (production)
solana config set --url mainnet-beta
```

**Verify Configuration:**
```bash
solana config get
```

### Step 3: Prepare Keypairs

#### Option A: Use Default Keypair

```bash
# Check your default keypair
solana address
solana balance
```

#### Option B: Generate New Keypair

```bash
# Generate program keypair (if not exists)
solana-keygen new -o program/keypair.json

# Generate authority keypair (optional, for security)
solana-keygen new -o ~/.config/solana/authority.json
```

**Important:** Back up your keypairs securely!

### Step 4: Deploy the Program

Deploy to your configured cluster:

```bash
# Deploy to devnet (default)
pnpm programs:deploy

# Deploy to specific cluster
pnpm programs:deploy -- --cluster mainnet-beta

# Deploy with custom upgrade authority
pnpm programs:deploy -- --keypair ~/.config/solana/authority.json --cluster devnet
```

**Expected Output:**
```
Deploying your_wallet_stake_pool to devnet...
✓ Successfully deployed your_wallet_stake_pool to devnet
  Program ID: <PROGRAM_ID>
```

**Save the Program ID!** You'll need it for the next steps.

---

## Post-Deployment Setup

### Step 5: Initialize Program Authority

**This is a critical one-time setup step!**

After deploying, you must initialize the `ProgramAuthority` account, which controls who can create stake pools:

```bash
# Initialize on devnet
pnpm programs:init-authority

# Initialize on specific cluster
pnpm programs:init-authority -- --cluster mainnet-beta

# Initialize with custom authority
pnpm programs:init-authority -- --keypair ~/.config/solana/authority.json
```

**What This Does:**
- Creates the global `ProgramAuthority` PDA account (one per program deployment)
- Sets your keypair as the main authority (controls entire system)
- Initializes the authorized_creators list (empty initially)
- Main authority can immediately create pools or delegate to others

**Expected Output:**
```
✅ ProgramAuthority initialized successfully!

Transaction Details:
  Signature: <TX_SIGNATURE>
  Authority: <YOUR_PUBKEY>
  PDA: <PROGRAM_AUTHORITY_PDA>

✓ The authority is now authorized to create pools
```

**Important Notes:**
- ⚠️ This command can only succeed once per program deployment
- Running it again will show "already initialized" (expected)
- The authority controls ALL pool creation and can delegate to others (max 10 additional addresses)
- This is a GLOBAL admin role, not per-pool

---

## Verification

### Verify Program Deployment

Check that your program is deployed:

```bash
# Get program info
solana program show <PROGRAM_ID>

# Expected output includes:
# - Program Id
# - Owner
# - ProgramData Address
# - Authority (your upgrade authority)
# - Last Deployed Slot
# - Data Length
```

### Verify Program Authority

Check that the ProgramAuthority was initialized:

```bash
# Use Solana Explorer
# Navigate to: https://explorer.solana.com/address/<PROGRAM_AUTHORITY_PDA>?cluster=devnet

# Or use the TypeScript client
cd example
pnpm devnet-test
```

### Test Pool Creation

Create a test pool to verify everything works:

```bash
# Run the integration test
cd example
pnpm devnet-test
```

This will:
1. ✅ Verify program authority exists
2. ✅ Create a test stake pool
3. ✅ Perform stake operations
4. ✅ Test reward claims

---

## Managing Authorities

### Program Upgrade Authority

The upgrade authority controls program upgrades (bug fixes, new features).

#### View Current Authority

```bash
solana program show <PROGRAM_ID>
# Look for "Authority: <PUBKEY>"
```

#### Transfer Upgrade Authority

```bash
# Transfer to new authority
pnpm programs:transfer-authority <NEW_AUTHORITY_PUBKEY>

# Make program immutable (IRREVERSIBLE!)
pnpm programs:transfer-authority --none
```

**Warning:** Transferring authority is irreversible. Double-check the address!

### Program Authority (Pool Creation)

The program authority controls **all pool creation globally**.

#### Add Authorized Pool Creators

Delegate pool creation rights to other addresses (max 10 additional):

```bash
# Using the CLI script
pnpm programs:add-creator <CREATOR_ADDRESS>

# Add multiple creators
pnpm programs:add-creator <CREATOR_1> <CREATOR_2>
```

Or use the TypeScript client directly:

```typescript
import { getManageAuthorizedCreatorsInstruction } from '@yourwallet/stake-pool';

// Add creators (batched operation)
const instruction = getManageAuthorizedCreatorsInstruction({
  programAuthority: programAuthorityPda,
  authority: mainAuthority,
  creatorsToAdd: [creator1, creator2],
  creatorsToRemove: [],
});

// Send transaction...
```

See `example/src/pool-admin.ts` for full examples.

#### Remove Authorized Creators

```bash
# Using the CLI script
pnpm programs:remove-creator <CREATOR_ADDRESS>
```

Or use TypeScript:

```typescript
const instruction = getManageAuthorizedCreatorsInstruction({
  programAuthority: programAuthorityPda,
  authority: mainAuthority,
  creatorsToAdd: [],
  creatorsToRemove: [creatorToRemove],
});
```

#### Transfer Program Authority

To transfer GLOBAL control (all pools, entire system):

```typescript
import { 
  getTransferProgramAuthorityInstruction,
  getAcceptProgramAuthorityInstruction 
} from '@yourwallet/stake-pool';

// Step 1: Current authority nominates new authority
const transferIx = getTransferProgramAuthorityInstruction({
  programAuthority: programAuthorityPda,
  currentAuthority: currentAuth,
  newAuthority: newAuthAddress,
});

// Step 2: New authority accepts (must be signed by new authority)
const acceptIx = getAcceptProgramAuthorityInstruction({
  programAuthority: programAuthorityPda,
  pendingAuthority: newAuth,
});
```

**Warning:** This transfers control of the ENTIRE SYSTEM, not individual pools!

**Note:** Main authority cannot be removed from authorized creators list.

---

## Implementation Details

### Initialize Authority Script

The `programs:init-authority` command uses a modular design for maintainability:

**Architecture:**
- **Main Script**: `scripts/program/initialize-authority.mjs` (245 lines)
  - CLI interface with help, options, and user confirmation
  - Auto-detects program ID from repository structure
  - Validates authority keypairs
  - Streamlined execution flow

- **Shared Library**: `scripts/lib/program-authority.mjs` (169 lines)
  - `calculateProgramAuthorityPda()` - PDA calculation
  - `loadKeypairSigner()` - Keypair loading
  - `initializeProgramAuthority()` - Transaction execution
  - `getClusterUrl()`, `getExplorerUrl()` - Utilities

**Benefits:**
- Code reusability across deployment scripts
- Clean separation of concerns
- Easy to test and maintain
- Direct ES6 imports (no dynamic resolution)

**Dependencies:**
- `@solana/kit` (from root node_modules)
- `@yourwallet/stake-pool` (generated JavaScript client)
- `zx` (for shell scripting)

See `scripts/lib/program-authority.mjs` for implementation details.

---

## Deployment Checklist

Use this checklist for production deployments:

### Pre-Deployment

- [ ] All tests passing (`pnpm programs:test`)
- [ ] Code audited (see `audit/SECURITY_AUDIT.md`)
- [ ] Tested on devnet/testnet
- [ ] Backup all keypairs securely
- [ ] Sufficient SOL in deployer account
- [ ] Cluster configured correctly

### Deployment

- [ ] Build program (`pnpm programs:build`)
- [ ] Deploy program (`pnpm programs:deploy`)
- [ ] Save Program ID
- [ ] Initialize program authority (`pnpm programs:init-authority`)
- [ ] Verify deployment (explorer + tests)

### Post-Deployment

- [ ] Document Program ID and authority addresses
- [ ] Set up monitoring (transaction history, account changes)
- [ ] Create first production pool (if needed)
- [ ] Update frontend/client configuration
- [ ] Announce deployment (if public)

### Security

- [ ] Transfer upgrade authority to multisig (recommended for mainnet)
- [ ] Secure authority keypairs (hardware wallet, multisig)
- [ ] Set up emergency procedures
- [ ] Document access control
- [ ] Plan upgrade procedures

---

## Troubleshooting

### Common Issues

#### 1. Insufficient Funds

**Error:** `insufficient funds for transaction`

**Solution:**
```bash
# Check balance
solana balance

# Request airdrop (devnet/testnet only)
solana airdrop 2

# For mainnet: fund your account
```

#### 2. Program Already Deployed

**Error:** `Error: Deploying program failed: Program <ID> already deployed`

**Solution:**
```bash
# Either:
# A) Use existing program with --program-id flag
pnpm programs:deploy -- --program-id <EXISTING_ID>

# B) Generate new program keypair
solana-keygen new -o program/keypair.json --force
```

#### 3. Program Authority Already Initialized

**Error:** `AccountNotEmpty` or `already in use`

**Solution:** This is expected! The program authority can only be initialized once.

#### 4. Wrong Cluster

**Error:** Program deployed to wrong cluster

**Solution:**
```bash
# Check current cluster
solana config get

# Change cluster
solana config set --url <CORRECT_CLUSTER>

# Redeploy if necessary
```

#### 5. JavaScript Client Not Built

**Error:** `Cannot find module './clients/js/dist/index.js'`

**Solution:**
```bash
# Build the JavaScript client
pnpm clients:build

# Or rebuild everything
pnpm clean && pnpm install:all
```

### Getting Help

If you encounter issues:

1. Check the [README.md](../README.md) for basic setup
2. Review [ARCHITECTURE.md](./ARCHITECTURE.md) for system design
3. See [example/src/](../example/src/) for usage examples
4. Check program logs: `solana logs <PROGRAM_ID>`
5. Open an issue on GitHub with:
   - Error message
   - Steps to reproduce
   - Cluster (devnet/testnet/mainnet)
   - Program ID

---

## Production Recommendations

For mainnet deployments:

### Security

1. **Use Multisig for Authority**
   - Use Squads Protocol or similar for upgrade authority
   - Require 2-of-3 or 3-of-5 signatures for upgrades

2. **Hardware Wallets**
   - Store authority keys on hardware wallets (Ledger, etc.)

3. **Time-Locked Upgrades**
   - Consider governance/timelock for upgrades

### Operations

1. **Monitoring**
   - Set up transaction monitoring
   - Track program account changes
   - Monitor reward pool balances

2. **Testing**
   - Deploy to devnet for 2-4 weeks first
   - Run integration tests regularly
   - Simulate all operations

3. **Documentation**
   - Document all authority addresses
   - Create runbooks for common operations
   - Plan emergency procedures

4. **Backups**
   - Backup all keypairs (encrypted, offline)
   - Document recovery procedures
   - Test recovery process

### Before Going Live

- [ ] Complete security audit by professional firm
- [ ] Test all features on devnet/testnet
- [ ] Verify all authority configurations
- [ ] Set up monitoring and alerts
- [ ] Prepare emergency procedures
- [ ] Document everything
- [ ] Have upgrade plan ready

---

## Quick Reference

### Common Commands

```bash
# Build program
pnpm programs:build

# Deploy to devnet
pnpm programs:deploy

# Initialize program authority
pnpm programs:init-authority

# Transfer upgrade authority
pnpm programs:transfer-authority <NEW_AUTHORITY>

# Run tests
pnpm programs:test

# Run integration test
cd example && pnpm devnet-test
```

### Important Files

- `program/keypair.json` - Program keypair (if exists)
- `target/deploy/*.so` - Compiled program binary
- `~/.config/solana/id.json` - Default Solana keypair
- `idl.json` - Program IDL (for clients)

### Important Accounts

- **Program ID**: The deployed program address
- **ProgramAuthority PDA**: Derived from `["program_authority"]`
- **Upgrade Authority**: Controls program upgrades
- **Main Authority**: Controls pool creation

---

## Next Steps

After successful deployment:

1. **Create Your First Pool**: See [example/src/pool-admin.ts](../example/src/pool-admin.ts)
2. **Integrate with Frontend**: Use the JavaScript client
3. **Set Up Monitoring**: Track pool activity
4. **Plan Maintenance**: Schedule regular reviews

For API documentation, see the [JavaScript client README](../clients/js/README.md).

---

**Need Help?** Check our [documentation](../README.md) or open an issue!
