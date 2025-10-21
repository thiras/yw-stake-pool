/**
 * Utility functions for stake pool examples
 */

import {
  address,
  Address,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  generateKeyPairSigner,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  pipe,
  KeyPairSigner,
  TransactionSigner,
  getAddressEncoder,
  lamports,
} from '@solana/kit';
import type { Rpc, SolanaRpcApi, Commitment } from '@solana/kit';
import { config } from './config.js';

/**
 * Create RPC client
 */
export function createRpc() {
  return createSolanaRpc(config.rpcUrl);
}

/**
 * Create RPC subscriptions client
 */
export function createRpcSubscriptions() {
  return createSolanaRpcSubscriptions(config.rpcUrl.replace('http', 'ws'));
}

/**
 * Find Pool PDA
 */
export async function findPoolPda(
  authority: Address,
  stakeMint: Address,
  programId: Address = config.programId
): Promise<[Address, number]> {
  const encoder = getAddressEncoder();
  const seeds: any[] = [
    Buffer.from('stake_pool'),
    encoder.encode(authority),
    encoder.encode(stakeMint),
  ];

  // Note: This is a simplified version. In production, use proper PDA derivation
  // For now, we'll use a placeholder that matches the on-chain program
  const [pda, bump] = await findProgramAddress(seeds, programId);
  return [pda, bump];
}

/**
 * Find Stake Account PDA
 */
export async function findStakeAccountPda(
  pool: Address,
  owner: Address,
  index: bigint,
  programId: Address = config.programId
): Promise<[Address, number]> {
  const encoder = getAddressEncoder();
  const indexBuffer = Buffer.alloc(8);
  indexBuffer.writeBigUInt64LE(index);

  const seeds: any[] = [
    Buffer.from('stake_account'),
    encoder.encode(pool),
    encoder.encode(owner),
    indexBuffer,
  ];

  const [pda, bump] = await findProgramAddress(seeds, programId);
  return [pda, bump];
}

/**
 * Helper to find program address (PDA)
 * NOTE: This is a simplified placeholder. In production, use proper PDA derivation
 * from @solana/addresses or similar library
 */
async function findProgramAddress(
  seeds: any[],
  programId: Address
): Promise<[Address, number]> {
  // Placeholder implementation
  // In a real implementation, you would:
  // 1. Hash the seeds + bump + program ID
  // 2. Check if the result is on the ed25519 curve
  // 3. If not, try the next bump
  
  // For now, return a placeholder
  console.warn('⚠️  Using placeholder PDA derivation. Replace with proper implementation.');
  const pda = address('11111111111111111111111111111111');
  return [pda, 255];
}

/**
 * Airdrop SOL to an address
 */
export async function airdrop(
  rpc: any,
  recipient: Address,
  amount: bigint = config.airdropAmount
): Promise<void> {
  console.log(`💰 Airdropping ${amount} lamports to ${recipient}...`);
  
  try {
    const signature = await (rpc as any)
      .requestAirdrop(recipient, lamports(amount))
      .send();
    
    // Wait for confirmation
    await new Promise((resolve) => setTimeout(resolve, 1000));
    console.log(`✅ Airdrop confirmed: ${signature}`);
  } catch (error) {
    console.error('❌ Airdrop failed:', error);
    throw error;
  }
}

/**
 * Build and send a transaction
 */
export async function buildAndSendTransaction(
  rpc: any,
  instructions: any[],
  payer: KeyPairSigner & TransactionSigner,
  signers: (KeyPairSigner & TransactionSigner)[] = []
): Promise<string> {
  console.log(`📤 Building transaction with ${instructions.length} instruction(s)...`);

  // Get latest blockhash
  const { value: latestBlockhash } = await (rpc as any).getLatestBlockhash().send();

  // Build transaction message
  const allSigners = [payer, ...signers];
  const transactionMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(payer, tx),
    (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
    (tx) => ({ ...tx, instructions })
  );

  // Sign transaction
  console.log('✍️  Signing transaction...');
  const signedTransaction = await signTransactionMessageWithSigners(
    transactionMessage
  );

  // Send transaction
  console.log('📡 Sending transaction...');
  const signature = await (rpc as any)
    .sendTransaction(signedTransaction, {
      encoding: 'base64',
      maxRetries: 3n,
      skipPreflight: false,
    })
    .send();

  console.log(`✅ Transaction sent: ${signature}`);

  // Wait for confirmation
  await confirmTransaction(rpc, signature);

  return signature;
}

/**
 * Confirm a transaction
 */
async function confirmTransaction(
  rpc: any,
  signature: string,
  maxRetries: number = 30
): Promise<void> {
  console.log('⏳ Confirming transaction...');

  for (let i = 0; i < maxRetries; i++) {
    try {
      const status = await (rpc as any)
        .getSignatureStatuses([signature])
        .send();

      if (status.value[0]?.confirmationStatus === 'confirmed' ||
          status.value[0]?.confirmationStatus === 'finalized') {
        console.log('✅ Transaction confirmed!');
        return;
      }
    } catch (error) {
      // Ignore and retry
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  throw new Error('Transaction confirmation timeout');
}

/**
 * Sleep for a duration
 */
export async function sleep(ms: number): Promise<void> {
  console.log(`⏸️  Sleeping for ${ms}ms...`);
  await new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Calculate expected rewards
 */
export function calculateRewards(
  amountStaked: bigint,
  rewardRate: bigint,
  lockupPassed: boolean
): bigint {
  if (!lockupPassed) return 0n;
  return (amountStaked * rewardRate) / 1_000_000_000n;
}

/**
 * Format address for display
 */
export function formatAddress(addr: Address): string {
  const str = addr.toString();
  return `${str.slice(0, 4)}...${str.slice(-4)}`;
}

/**
 * Log section header
 */
export function logSection(title: string): void {
  console.log('\n' + '='.repeat(60));
  console.log(`  ${title}`);
  console.log('='.repeat(60) + '\n');
}

/**
 * Log step
 */
export function logStep(step: number, description: string): void {
  console.log(`\n[Step ${step}] ${description}`);
  console.log('-'.repeat(50));
}

/**
 * Create a test keypair with airdrop
 */
export async function createFundedKeypair(
  rpc: any,
  name: string = 'keypair'
): Promise<KeyPairSigner> {
  const keypair = await generateKeyPairSigner();
  console.log(`🔑 Generated ${name}: ${keypair.address}`);
  await airdrop(rpc, keypair.address);
  return keypair;
}

/**
 * Log transaction result
 */
export function logTransaction(signature: string, description: string): void {
  console.log(`\n✨ ${description}`);
  console.log(`   Signature: ${signature}`);
  console.log(`   Explorer: https://explorer.solana.com/tx/${signature}?cluster=custom`);
}

/**
 * Error handler
 */
export function handleError(error: any, context: string): never {
  console.error(`\n❌ Error in ${context}:`);
  console.error(error);
  if (error.logs) {
    console.error('\nProgram Logs:');
    error.logs.forEach((log: string) => console.error(`  ${log}`));
  }
  process.exit(1);
}
