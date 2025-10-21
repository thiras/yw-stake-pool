/**
 * Configuration constants for the stake pool examples
 */

import { address } from '@solana/kit';
import { STAKE_POOL_PROGRAM_ADDRESS } from '@yourwallet/stake-pool';

export const config = {
  // RPC endpoint - change based on your environment
  // rpcUrl: process.env.RPC_URL || 'http://127.0.0.1:8899', // Local validator
  rpcUrl: process.env.RPC_URL || 'https://api.devnet.solana.com', // Devnet

  // Program ID - automatically imported from the generated client
  programId: STAKE_POOL_PROGRAM_ADDRESS,

  // Token program IDs
  tokenProgramId: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
  token2022ProgramId: address('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb'),
  systemProgramId: address('11111111111111111111111111111111'),

  // Pool configuration defaults
  defaultPoolConfig: {
    rewardRate: 100_000_000n, // 10% reward rate (100_000_000 / 1e9 = 0.10 = 10%)
    minStakeAmount: 1_000_000n, // 1 token with 6 decimals
    lockupPeriod: 86400n, // 1 day in seconds (24 * 60 * 60)
  },

  // Staking amounts for examples
  exampleAmounts: {
    stake: 100_000_000n, // 100 tokens
    unstake: 50_000_000n, // 50 tokens (partial)
    fund: 10_000_000_000n, // 10,000 tokens for reward vault
  },

  // Airdrop amount for testing (in lamports)
  airdropAmount: 2_000_000_000n, // 2 SOL

  // Keypair configuration
  // Set to true to use your local Solana keypair (~/.config/solana/id.json)
  // Set to false to generate new keypairs for testing
  useLocalKeypair: true,

  // Optional: specify custom keypair path
  // customKeypairPath: '/path/to/your/keypair.json',

  // Rate limiting
  // Delay between operations to avoid RPC rate limits (in milliseconds)
  // Set to 0 to disable delays (not recommended for public RPC endpoints)
  rateLimitDelay: 10000, // 10 seconds between operations
} as const;

// Time constants
export const TIME = {
  SECOND: 1000,
  MINUTE: 60 * 1000,
  HOUR: 60 * 60 * 1000,
  DAY: 24 * 60 * 60 * 1000,
} as const;

// Display helpers
export const DECIMALS = {
  SOL: 9,
  TOKEN: 6,
} as const;

export function formatAmount(amount: bigint, decimals: number = DECIMALS.TOKEN): string {
  const divisor = BigInt(10 ** decimals);
  const whole = amount / divisor;
  const fraction = amount % divisor;
  return `${whole}.${fraction.toString().padStart(decimals, '0')}`;
}

export function formatRewardRate(rate: bigint): string {
  const percentage = Number(rate) / 1e7; // rate is scaled by 1e9, so divide by 1e7 to get percentage
  return `${percentage.toFixed(2)}%`;
}

export function formatDuration(seconds: bigint): string {
  const s = Number(seconds);
  if (s < 60) return `${s} seconds`;
  if (s < 3600) return `${Math.floor(s / 60)} minutes`;
  if (s < 86400) return `${Math.floor(s / 3600)} hours`;
  return `${Math.floor(s / 86400)} days`;
}
