/**
 * Token Setup Utilities
 * 
 * Creates real SPL tokens and token accounts for testing
 * Uses only @solana/kit - no web3.js
 */

import {
  address,
  Address,
  KeyPairSigner,
  TransactionSigner,
  generateKeyPairSigner,
  getAddressEncoder,
} from '@solana/kit';
import { config } from './config.js';
import {
  createRpc,
  buildAndSendTransaction,
  waitForRateLimit,
} from './utils.js';

export interface TokenSetup {
  stakeMint: Address;
  rewardMint: Address;
  stakeVault: Address;
  rewardVault: Address;
  authorityRewardAccount: Address;
  userStakeAccount: Address;
  userRewardAccount: Address;
}

// SPL Token Program Instructions
const TOKEN_PROGRAM_ID = config.tokenProgramId;

// Instruction discriminators
const INITIALIZE_MINT = 0;
const INITIALIZE_ACCOUNT = 1;
const MINT_TO = 7;

/**
 * Create SPL Token Mint Instruction (manual construction)
 */
function createInitializeMintInstruction(
  mint: Address,
  decimals: number,
  mintAuthority: Address,
  freezeAuthority: Address | null
): any {
  const data = new Uint8Array(67);
  data[0] = INITIALIZE_MINT;
  data[1] = decimals;
  
  const encoder = getAddressEncoder();
  const mintAuthorityBytes = encoder.encode(mintAuthority);
  data.set(mintAuthorityBytes, 2);
  
  data[34] = freezeAuthority ? 1 : 0;
  if (freezeAuthority) {
    const freezeAuthorityBytes = encoder.encode(freezeAuthority);
    data.set(freezeAuthorityBytes, 35);
  }

  return {
    programAddress: TOKEN_PROGRAM_ID,
    accounts: [
      { address: mint, role: 1 }, // writable
      { address: address('SysvarRent111111111111111111111111111111111'), role: 0 }, // readonly
    ],
    data,
  };
}

/**
 * Create a new SPL token mint
 */
export async function createMint(
  rpc: any,
  payer: KeyPairSigner & TransactionSigner,
  mintAuthority: Address,
  decimals: number = 6
): Promise<Address> {
  console.log(`\n🪙 Creating SPL Token Mint (${decimals} decimals)...`);

  const mintKeypair = await generateKeyPairSigner();
  const mintAddress = mintKeypair.address;

  console.log(`   Mint Address: ${mintAddress}`);

  // Mint account size: 82 bytes
  const space = BigInt(82);
  const rentLamports = await rpc.getMinimumBalanceForRentExemption(space).send();

  const encoder = getAddressEncoder();

  // Create account instruction
  const createAccountData = new Uint8Array(52);
  const view = new DataView(createAccountData.buffer);
  view.setUint32(0, 0, true); // CreateAccount instruction
  view.setBigUint64(4, BigInt(rentLamports), true); // lamports
  view.setBigUint64(12, space, true); // space
  createAccountData.set(encoder.encode(TOKEN_PROGRAM_ID), 20); // owner

  const createAccountIx = {
    programAddress: config.systemProgramId,
    accounts: [
      { address: payer.address, role: 3, signer: payer }, // WRITABLE_SIGNER
      { address: mintAddress, role: 3, signer: mintKeypair }, // WRITABLE_SIGNER
    ],
    data: createAccountData,
  };

  // Initialize mint instruction
  const initMintIx = createInitializeMintInstruction(
    mintAddress,
    decimals,
    mintAuthority,
    null
  );

  await buildAndSendTransaction(rpc, [createAccountIx, initMintIx], payer, [mintKeypair]);

  console.log(`✅ Mint created: ${mintAddress}`);
  
  return mintAddress;
}

/**
 * Create Associated Token Account (simplified - uses PDA derivation)
 */
export async function createTokenAccount(
  rpc: any,
  payer: KeyPairSigner & TransactionSigner,
  mint: Address,
  owner: Address
): Promise<Address> {
  console.log(`\n💼 Creating Token Account...`);
  console.log(`   Owner: ${owner}`);
  console.log(`   Mint: ${mint}`);

  // For simplicity, create a regular token account owned by the owner
  // In production, you'd use Associated Token Accounts
  const tokenAccountKeypair = await generateKeyPairSigner();
  const tokenAddress = tokenAccountKeypair.address;

  console.log(`   Token Account: ${tokenAddress}`);

  // Token account size: 165 bytes
  const space = BigInt(165);
  const rentLamports = await rpc.getMinimumBalanceForRentExemption(space).send();

  const encoder = getAddressEncoder();

  // Create account
  const createAccountData = new Uint8Array(52);
  const view = new DataView(createAccountData.buffer);
  view.setUint32(0, 0, true); // CreateAccount
  view.setBigUint64(4, BigInt(rentLamports), true);
  view.setBigUint64(12, space, true);
  createAccountData.set(encoder.encode(TOKEN_PROGRAM_ID), 20);

  const createAccountIx = {
    programAddress: config.systemProgramId,
    accounts: [
      { address: payer.address, role: 3, signer: payer },
      { address: tokenAddress, role: 3, signer: tokenAccountKeypair },
    ],
    data: createAccountData,
  };

  // Initialize account
  const initAccountData = new Uint8Array(66);
  initAccountData[0] = INITIALIZE_ACCOUNT;
  initAccountData.set(encoder.encode(mint), 1);
  initAccountData.set(encoder.encode(owner), 33);

  const initAccountIx = {
    programAddress: TOKEN_PROGRAM_ID,
    accounts: [
      { address: tokenAddress, role: 1 }, // writable
      { address: mint, role: 0 }, // readonly
      { address: owner, role: 0 }, // readonly
      { address: address('SysvarRent111111111111111111111111111111111'), role: 0 },
    ],
    data: initAccountData,
  };

  await buildAndSendTransaction(rpc, [createAccountIx, initAccountIx], payer, [tokenAccountKeypair]);

  console.log(`✅ Token account created: ${tokenAddress}`);
  
  return tokenAddress;
}

/**
 * Mint tokens to an account
 */
export async function mintTokensTo(
  rpc: any,
  payer: KeyPairSigner & TransactionSigner,
  mint: Address,
  destination: Address,
  amount: bigint
): Promise<void> {
  console.log(`\n🏦 Minting tokens...`);
  console.log(`   Amount: ${amount.toString()}`);
  console.log(`   Destination: ${destination}`);

  const mintToData = new Uint8Array(9);
  mintToData[0] = MINT_TO;
  const view = new DataView(mintToData.buffer);
  view.setBigUint64(1, amount, true);

  const mintToIx = {
    programAddress: TOKEN_PROGRAM_ID,
    accounts: [
      { address: mint, role: 1 }, // writable
      { address: destination, role: 1 }, // writable
      { address: payer.address, role: 2 }, // signer (mint authority)
    ],
    data: mintToData,
  };

  await buildAndSendTransaction(rpc, [mintToIx], payer);

  console.log(`✅ Minted ${amount} tokens`);
}

/**
 * Setup all required tokens and accounts for stake pool testing
 * @param authority - The authority keypair (mint authority and payer)
 * @param user - The user keypair
 * @param poolPda - The pool PDA address (to be used as vault owner)
 * @param stakeMintKeypair - Pre-generated stake mint keypair (needed for PDA derivation)
 */
export async function setupTestTokens(
  authority: KeyPairSigner & TransactionSigner,
  user: KeyPairSigner & TransactionSigner,
  poolPda: Address,
  stakeMintKeypair: KeyPairSigner & TransactionSigner
): Promise<TokenSetup> {
  const rpc = createRpc();

  console.log('\n' + '='.repeat(60));
  console.log('  Setting Up Test Tokens');
  console.log('='.repeat(60));

  // Use the provided stake mint keypair
  const stakeMint = stakeMintKeypair.address;
  console.log(`\n🪙 Using pre-generated Stake Mint: ${stakeMint}`);
  
  // Create the stake mint account
  const space = BigInt(82);
  const rentLamports = await rpc.getMinimumBalanceForRentExemption(space).send();
  const encoder = getAddressEncoder();

  const createAccountData = new Uint8Array(52);
  const view = new DataView(createAccountData.buffer);
  view.setUint32(0, 0, true);
  view.setBigUint64(4, BigInt(rentLamports), true);
  view.setBigUint64(12, space, true);
  createAccountData.set(encoder.encode(TOKEN_PROGRAM_ID), 20);

  const createAccountIx = {
    programAddress: config.systemProgramId,
    accounts: [
      { address: authority.address, role: 3, signer: authority },
      { address: stakeMint, role: 3, signer: stakeMintKeypair },
    ],
    data: createAccountData,
  };

  const initMintIx = createInitializeMintInstruction(stakeMint, 6, authority.address, null);

  await buildAndSendTransaction(rpc, [createAccountIx, initMintIx], authority, [stakeMintKeypair]);
  console.log(`✅ Stake mint created: ${stakeMint}`);
  await waitForRateLimit();

  // Create reward token mint
  const rewardMint = await createMint(rpc, authority, authority.address, 6);
  await waitForRateLimit();

  // Create stake vault (pool's stake token account) - OWNED BY POOL PDA
  const stakeVault = await createTokenAccount(rpc, authority, stakeMint, poolPda);
  await waitForRateLimit();

  // Create reward vault (pool's reward token account) - OWNED BY POOL PDA
  const rewardVault = await createTokenAccount(rpc, authority, rewardMint, poolPda);
  await waitForRateLimit();

  // Create authority's reward token account (for funding the vault)
  const authorityRewardAccount = await createTokenAccount(rpc, authority, rewardMint, authority.address);
  await waitForRateLimit();

  // Create user's stake token account
  const userStakeAccount = await createTokenAccount(rpc, authority, stakeMint, user.address);
  await waitForRateLimit();

  // Create user's reward token account
  const userRewardAccount = await createTokenAccount(rpc, authority, rewardMint, user.address);
  await waitForRateLimit();

  // Mint some tokens to user for testing
  console.log('\n💰 Funding accounts for testing...');
  
  await mintTokensTo(rpc, authority, stakeMint, userStakeAccount, 1_000_000_000n); // 1000 tokens
  await waitForRateLimit();

  await mintTokensTo(rpc, authority, rewardMint, authorityRewardAccount, 10_000_000_000n); // 10000 tokens
  await waitForRateLimit();

  console.log('\n✅ Token setup complete!\n');

  return {
    stakeMint,
    rewardMint,
    stakeVault,
    rewardVault,
    authorityRewardAccount,
    userStakeAccount,
    userRewardAccount,
  };
}
