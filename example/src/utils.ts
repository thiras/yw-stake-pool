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
  sendAndConfirmTransactionFactory,
  getSignatureFromTransaction,
  getProgramDerivedAddress,
  pipe,
  KeyPairSigner,
  TransactionSigner,
  getAddressEncoder,
  lamports,
  createKeyPairSignerFromBytes,
} from '@solana/kit';
import type { Rpc, SolanaRpcApi, Commitment } from '@solana/kit';
import { config } from './config.js';
import { readFileSync, existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';

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
  
  const [pda, bump] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [
      'stake_pool',
      encoder.encode(authority),
      encoder.encode(stakeMint),
    ],
  });
  
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
  const indexBuffer = new Uint8Array(8);
  new DataView(indexBuffer.buffer).setBigUint64(0, index, true); // little-endian

  const [pda, bump] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [
      'stake_account',
      encoder.encode(pool),
      encoder.encode(owner),
      indexBuffer,
    ],
  });

  return [pda, bump];
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

  // Get signature before sending
  const signature = getSignatureFromTransaction(signedTransaction);
  console.log(`📝 Transaction signature: ${signature}`);

  // Send and confirm transaction
  console.log('📡 Sending and confirming transaction...');
  const rpcSubscriptions = createRpcSubscriptions();
  
  await sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions })(
    signedTransaction as any, // Type assertion to bypass complex lifetime constraint
    { commitment: 'confirmed' }
  );

  console.log(`✅ Transaction confirmed!`);

  return signature;
}

/**
 * Sleep for a duration
 */
export async function sleep(ms: number): Promise<void> {
  console.log(`⏸️  Sleeping for ${ms}ms...`);
  await new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Wait between operations to avoid RPC rate limits
 * Uses configured delay from config
 */
export async function waitForRateLimit(): Promise<void> {
  const delay = config.rateLimitDelay;
  if (delay > 0) {
    console.log(`⏱️  Waiting ${delay}ms for rate limit...`);
    await new Promise((resolve) => setTimeout(resolve, delay));
  }
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
 * Load keypair from filesystem
 * Tries the following locations in order:
 * 1. Path specified in SOLANA_KEYPAIR_PATH env var
 * 2. ~/.config/solana/id.json (default Solana CLI location)
 * 3. ./keypair.json (local file)
 */
export async function loadKeypair(customPath?: string): Promise<KeyPairSigner> {
  const paths = [
    customPath,
    process.env.SOLANA_KEYPAIR_PATH,
    join(homedir(), '.config', 'solana', 'id.json'),
    './keypair.json',
  ].filter(Boolean) as string[];

  for (const path of paths) {
    if (existsSync(path)) {
      try {
        const keypairData = JSON.parse(readFileSync(path, 'utf-8'));
        const secretKey = new Uint8Array(keypairData);
        const keypair = await createKeyPairSignerFromBytes(secretKey);
        console.log(`🔑 Loaded keypair from: ${path}`);
        console.log(`   Address: ${keypair.address}`);
        return keypair;
      } catch (error) {
        console.warn(`⚠️  Failed to load keypair from ${path}: ${(error as Error).message}`);
      }
    }
  }

  throw new Error(
    'No keypair found. Please ensure you have a Solana keypair at:\n' +
    '  - ~/.config/solana/id.json (default), or\n' +
    '  - Set SOLANA_KEYPAIR_PATH environment variable, or\n' +
    '  - Create ./keypair.json in the example directory\n' +
    'You can generate one with: solana-keygen new'
  );
}

/**
 * Create or load a keypair with optional airdrop
 * If useLocalKeypair is true, loads from filesystem, otherwise generates new one
 */
export async function createFundedKeypair(
  rpc: any,
  name: string = 'keypair',
  useLocalKeypair: boolean = true,
  customKeypairPath?: string
): Promise<KeyPairSigner> {
  let keypair: KeyPairSigner;

  if (useLocalKeypair) {
    try {
      keypair = await loadKeypair(customKeypairPath);
      console.log(`📝 Using local keypair as ${name}`);
    } catch (error) {
      console.log(`⚠️  ${(error as Error).message}`);
      console.log(`🔄 Falling back to generating new keypair for ${name}...`);
      keypair = await generateKeyPairSigner();
      console.log(`🔑 Generated ${name}: ${keypair.address}`);
      await airdrop(rpc, keypair.address);
      return keypair;
    }
  } else {
    keypair = await generateKeyPairSigner();
    console.log(`🔑 Generated ${name}: ${keypair.address}`);
    await airdrop(rpc, keypair.address);
    return keypair;
  }

  // Check balance and airdrop if needed
  try {
    const balance = await (rpc as any).getBalance(keypair.address).send();
    console.log(`   Balance: ${balance.value} lamports`);
    
    if (balance.value < 100_000_000n) { // Less than 0.1 SOL
      console.log(`   💰 Balance low, requesting airdrop...`);
      await airdrop(rpc, keypair.address);
    }
  } catch (error) {
    console.warn(`⚠️  Could not check balance: ${(error as Error).message}`);
  }

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
