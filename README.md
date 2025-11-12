# Your Wallet Stake Pool

<a href="https://github.com/yourwalletio/stake-pool/actions/workflows/main.yml"><img src="https://img.shields.io/github/actions/workflow/status/yourwalletio/stake-pool/main.yml?logo=GitHub" /></a>
<a href="https://github.com/yourwalletio/stake-pool/actions/workflows/main.yml"><img src="https://img.shields.io/badge/security-Sec3%20X--ray-blue?logo=shield" /></a>
<a href="https://explorer.solana.com/address/8PtjrGvKNeZt2vCmRkSPGjss7TAFhvxux2N8r67UMKBx"><img src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fraw.githubusercontent.com%2Fyourwalletio%2Fstake-pool%2Fmain%2Fprogram%2Fidl.json&query=%24.version&label=program&logo=solana&logoColor=white&color=9945FF" /></a>
<a href="https://www.npmjs.com/package/@yourwallet/stake-pool"><img src="https://img.shields.io/npm/v/%40yourwallet%2Fstake-pool?logo=npm&color=377CC0" /></a>

A secure, flexible staking program for Solana that enables token holders to stake their SPL tokens and earn fixed rewards. Built with security-first principles and full support for Token-2022 extensions including transfer fees.

## Overview

YW Stake Pool is a production-ready Solana program that provides:

- **🔒 Secure Staking** - Stake any SPL token or Token-2022 with transfer fee support
- **💰 Fixed Rewards** - Predictable rewards based on configurable reward rates
- **⏱️ Flexible Lockup** - Customizable lockup periods per pool
- **🛡️ Security Features** - Built-in protections against common vulnerabilities:
  - Type Cosplay Protection (account discriminators)
  - Frontrunning Protection (parameter verification)
  - Account Validation (ownership checks)
  - Two-step Authority Transfer
- **🔧 Pool Management** - Full administrative controls for pool operators
- **📊 Multi-Pool Support** - Create unlimited pools with different parameters

## Features

### For Stakers
- **Stake** any supported SPL token
- **Earn** fixed rewards after lockup period
- **Claim** rewards at any time
- **Unstake** partially or fully (with early exit option)
- **Track** multiple stakes with indexed stake accounts

### For Pool Operators
- **Initialize** pools with custom parameters (reward rate, lockup, minimum stake)
- **Create Multiple Pools** - Same authority can manage multiple pools for the same token using unique pool IDs
- **Update** pool settings (pause/unpause, change rates)
- **Fund** reward vaults to ensure liquidity
- **Transfer** authority with two-step verification
- **Set** optional pool end dates

## Security

This program implements multiple security best practices:

1. **Type Cosplay Protection** - Account discriminators prevent type confusion attacks
2. **Frontrunning Protection** - Transactions can verify expected pool parameters
3. **Account Validation** - Comprehensive ownership and state validation
4. **Transfer Fee Support** - Properly handles Token-2022 transfer fees
5. **Numerical Overflow Protection** - All arithmetic uses checked operations
6. **Two-step Authority Transfer** - Prevents accidental authority loss
7. **Admin-Only Pool Creation** - Only authorized addresses can create pools (prevents spam/scam pools)

See [SECURITY_AUDIT.md](./SECURITY_AUDIT.md) for detailed security analysis.

## Architecture

```
Program Structure:
├── State Management (237 bytes pool account, 98 bytes stake account)
├── 9 Instructions (Initialize, Stake, Unstake, Claim, Update, Fund, Authority)
├── Token-2022 Support (Transfer fees, extensions)
└── Comprehensive Error Handling (15 custom error types)
```

**Program ID**: `8PtjrGvKNeZt2vCmRkSPGjss7TAFhvxux2N8r67UMKBx`

### Operational Workflows

For detailed architecture diagrams including authority transfer, reward rate changes, staking, claiming, and unstaking workflows, see **[ARCHITECTURE.md](./docs/ARCHITECTURE.md)**.

## Quick Start

### Installation

```sh
# Clone the repository
git clone https://github.com/yourwalletio/yw-stake-pool.git
cd yw-stake-pool

# Install dependencies
pnpm install
```

### Running Examples

The `example` directory contains comprehensive examples demonstrating all stake pool functionality:

```sh
# Build the example code
cd example
pnpm install
pnpm build

# Run examples:
pnpm simple           # Basic instruction creation (no network calls)
pnpm pool-admin       # Pool management demos
pnpm user-staking     # User staking workflow demos
pnpm devnet-test      # Live integration test on Solana devnet
```

See [example/README.md](./example/README.md) for detailed documentation.

### Development

Build and test the program locally:

```sh
pnpm programs:build
pnpm programs:test
```

### Using the Client Library

Install the JavaScript/TypeScript client:

```sh
npm install @yourwallet/stake-pool
# or
pnpm add @yourwallet/stake-pool
```

Basic usage:

```typescript
import { 
  getInitializePoolInstruction,
  getStakeInstruction,
  getClaimRewardsInstruction,
} from '@yourwallet/stake-pool';

// Initialize a pool
const initIx = getInitializePoolInstruction({
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
  poolId: 0n,                    // Unique ID (0 for first pool, 1 for second, etc.)
  rewardRate: 100_000_000n,      // 10% APY
  minStakeAmount: 1_000_000n,    // 1 token (6 decimals)
  lockupPeriod: 86400n,          // 1 day
  poolEndDate: null,
});

// Stake tokens
const stakeIx = getStakeInstruction({
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
  amount: 100_000_000n,  // 100 tokens
  index: 0n,
});
```

See [clients/js/README.md](./clients/js/README.md) for full API documentation.

### Multiple Pools Per Token

Multiple stake pools can be created for the same token by using different `poolId` values. This allows:

- Running multiple pools with different reward rates and lockup periods
- Segmenting user groups (e.g., VIP vs standard pools)
- Testing new configurations without affecting existing pools
- Creating time-limited promotional pools

Pool PDAs are derived from `(stakeMint, poolId)`:
- `poolId: 0n` for the first pool for this token
- `poolId: 1n` for the second pool for this token
- And so on...

**Built-in Safeguards:**
The program validates that the pool address matches the provided `pool_id`. If you:
- ❌ Use the wrong `pool_id` when deriving the PDA → Transaction fails (address mismatch)
- ❌ Try to reuse an existing `pool_id` for the same token → Transaction fails (account already exists)
- ✅ Always use `findPoolPda()` helper → Correct address is guaranteed

**Pool ID Management:**
Since pool IDs are scoped per token (not per authority), coordinate `pool_id` allocation:
- **Query existing pools**: Check which pool IDs are already in use for a token
- **Increment from highest**: Use `max(pool_id) + 1` when creating new pools
- **Document your pools**: Maintain off-chain records of pool purposes and configurations

**Example**:
```typescript
// First USDC staking pool (10% APY, 7-day lockup)
const pool0 = await findPoolPda(USDC_MINT, 0n)

// Second USDC staking pool (15% APY, 30-day lockup)  
const pool1 = await findPoolPda(USDC_MINT, 1n)

// These are different pools for the same token
// Users can choose which pool suits their needs
```

## Documentation

- **[Architecture](./docs/ARCHITECTURE.md)** - System design, workflows, and diagrams
- **[Example Code](./example/README.md)** - Comprehensive examples with live devnet test
- **[Client Library](./clients/js/README.md)** - JavaScript/TypeScript API documentation
- **[Security Audit](./audit/SECURITY_AUDIT.md)** - Detailed security analysis

## Development

### Project Setup

Install dependencies:

```sh
pnpm install
```

### Managing programs

You'll notice a `program` folder in the root of this repository. This is where your generated Solana program is located.

Whilst only one program gets generated, note that you can have as many programs as you like in this repository.
Whenever you add a new program folder to this repository, remember to add it to the `members` array of your root `Cargo.toml` file.
That way, your programs will be recognized by the following scripts that allow you to build, test, format and lint your programs respectively.

```sh
pnpm programs:build
pnpm programs:test
pnpm programs:format
pnpm programs:lint
```

### Deploying programs

Deploy your program to a Solana cluster:

```sh
# Deploy to devnet (default)
pnpm programs:deploy

# Deploy to specific cluster
pnpm programs:deploy -- --cluster mainnet-beta

# Deploy with custom upgrade authority
pnpm programs:deploy -- --keypair /path/to/authority.json
```

The deploy script automatically detects the program ID from your repository keypairs.

#### Initialize Program Authority (One-Time Setup)

After deploying the program for the first time, you must initialize the ProgramAuthority account. This account controls who can create new stake pools:

```sh
# Initialize on devnet (default)
pnpm programs:init-authority

# Initialize on specific cluster
pnpm programs:init-authority -- --cluster mainnet-beta

# Initialize with custom authority keypair
pnpm programs:init-authority -- --keypair /path/to/authority.json
```

**Important Notes:**
- This is a **one-time setup** that creates the ProgramAuthority PDA account
- Must be run after the program is deployed
- The authority keypair you use will be able to:
  - Create stake pools
  - Add/remove other authorized pool creators
- Running this again will fail if already initialized (which is expected)

To add more authorized pool creators later, use the TypeScript client's `manage_authorized_creators` instruction (see example code in `example/src/pool-admin.ts`).

### Managing authority

The project includes tools for managing program upgrade authority:

#### Program Upgrade Authority

Transfer or revoke program upgrade authority using Solana CLI:

```sh
# Transfer upgrade authority to a new address
pnpm programs:transfer-authority -- --new-authority <ADDRESS>

# Make program immutable (irreversible!)
pnpm programs:transfer-authority -- --none

# View help and options
pnpm programs:transfer-authority -- --help
```

This is a **one-step, immediate transfer** using Solana's native authority management. Use with caution!

**Note:** Pool operational authority (for managing pool parameters, funding rewards, etc.) can be transferred using the program's built-in two-step authority transfer instructions (`NominateNewAuthority` and `AcceptAuthority`). See the [Client Library documentation](./clients/js/README.md) for details on using these instructions.

## Generating IDLs

You may use the following command to generate the IDLs for your programs.

```sh
pnpm generate:idls
```

Depending on your program's framework, this will either use Shank or Anchor to generate the IDLs.
Note that, to ensure IDLs are generated using the correct framework version, the specific version used by the program will be downloaded and used locally.

## Generating clients

Once your programs' IDLs have been generated, you can generate clients for them using the following command.

```sh
pnpm generate:clients
```

Alternatively, you can use the `generate` script to generate both the IDLs and the clients at once.

```sh
pnpm generate
```

## Managing clients

The following clients are available for your programs. You may use the following links to learn more about each client.

- [JS client](./clients/js)

## Starting and stopping the local validator

The following script is available to start your local validator.

```sh
pnpm validator:start
```

By default, if a local validator is already running, the script will be skipped. You may use the `validator:restart` script instead to force the validator to restart.

```sh
pnpm validator:restart
```

Finally, you may stop the local validator using the following command.

```sh
pnpm validator:stop
```

## Using external programs in your validator

If your program requires any external programs to be running, you'll want to in your local validator.

You can do this by adding their program addresses to the `program-dependencies` array in the `Cargo.toml` of your program.

```toml
program-dependencies = [
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
  "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV",
]
```

Next time you build your program and run your validator, these external programs will automatically be fetched from mainnet and used in your local validator.

```sh
pnpm programs:build
pnpm validator:restart
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Apache License 2.0 - See [LICENSE](./LICENSE) for details.
