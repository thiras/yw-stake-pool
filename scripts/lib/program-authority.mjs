#!/usr/bin/env node
/**
 * Program Authority Management Library
 * 
 * Shared utilities for managing the ProgramAuthority account.
 * Used by deployment scripts and can be imported by other tools.
 */

import { readFileSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const workspaceRoot = join(__dirname, '../..');

// Import @solana/kit directly from root node_modules
import {
  address,
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  createTransactionMessage,
  getProgramDerivedAddress,
  pipe,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
} from '@solana/kit';

/**
 * Get cluster RPC URL
 */
export function getClusterUrl(cluster) {
  const urls = {
    devnet: 'https://api.devnet.solana.com',
    testnet: 'https://api.testnet.solana.com',
    'mainnet-beta': 'https://api.mainnet-beta.solana.com',
  };
  return urls[cluster] || cluster;
}

/**
 * Calculate the ProgramAuthority PDA for a given program ID
 * 
 * @param {string} programId - The program ID
 * @returns {Promise<string>} The ProgramAuthority PDA address
 */
export async function calculateProgramAuthorityPda(programId) {
  const seeds = [Buffer.from('program_authority')];
  const [pda] = await getProgramDerivedAddress({
    programAddress: address(programId),
    seeds,
  });
  
  return pda;
}

/**
 * Load a keypair from a file path
 * 
 * @param {string} keypairPath - Path to the keypair JSON file
 * @returns {Promise<Object>} Keypair signer object
 */
export async function loadKeypairSigner(keypairPath) {
  const keypairData = JSON.parse(readFileSync(keypairPath, 'utf-8'));
  return await createKeyPairSignerFromBytes(new Uint8Array(keypairData));
}

/**
 * Helper to send and wait for transaction confirmation
 * @param {Object} rpc - RPC connection
 * @param {Object} signedTransaction - Signed transaction
 * @returns {Promise<string>} Transaction signature
 */
export async function sendAndWaitForTransaction(rpc, signedTransaction) {
  // Import getBase64EncodedWireTransaction from @solana/kit
  const { getBase64EncodedWireTransaction } = await import('@solana/kit');
  
  // Encode transaction
  const base64Transaction = getBase64EncodedWireTransaction(signedTransaction);
  
  // Send transaction
  const signature = await rpc
    .sendTransaction(base64Transaction, {
      encoding: 'base64',
      preflightCommitment: 'confirmed',
    })
    .send();
  
  // Poll for confirmation
  let confirmed = false;
  for (let i = 0; i < 30; i++) {
    await new Promise(resolve => setTimeout(resolve, 1000));
    try {
      const status = await rpc.getSignatureStatuses([signature]).send();
      if (status.value[0]?.confirmationStatus === 'confirmed' || 
          status.value[0]?.confirmationStatus === 'finalized') {
        confirmed = true;
        break;
      }
      if (status.value[0]?.err) {
        throw new Error(`Transaction failed: ${JSON.stringify(status.value[0].err)}`);
      }
    } catch (e) {
      if (e.message.includes('Transaction failed')) {
        throw e;
      }
      // Continue polling on other errors
    }
  }
  
  if (!confirmed) {
    throw new Error('Transaction confirmation timeout');
  }
  
  return signature;
}

/**
 * Initialize the ProgramAuthority account
 * 
 * @param {Object} options - Initialization options
 * @param {string} options.programId - Program ID
 * @param {string} options.programAuthorityPda - ProgramAuthority PDA address
 * @param {string} options.authorityKeypairPath - Path to authority keypair
 * @param {string} options.cluster - Cluster name (devnet, testnet, mainnet-beta)
 * @returns {Promise<string>} Transaction signature
 */
export async function initializeProgramAuthority({
  programId,
  programAuthorityPda,
  authorityKeypairPath,
  cluster = 'devnet',
}) {
  const clientPath = join(workspaceRoot, 'clients/js/dist/src/index.js');
  const { getInitializeProgramAuthorityInstruction } = await import(clientPath);
  
  const clusterUrl = getClusterUrl(cluster);
  
  // Load authority keypair
  const authority = await loadKeypairSigner(authorityKeypairPath);
  
  // Create RPC connection
  const rpc = createSolanaRpc(clusterUrl);
  
  // Create instruction
  const instruction = getInitializeProgramAuthorityInstruction({
    programAuthority: address(programAuthorityPda),
    initialAuthority: authority.address,
    payer: authority.address,
    systemProgram: address('11111111111111111111111111111111'),
  });
  
  // Get recent blockhash
  const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();
  
  // Create and sign transaction
  const transactionMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(authority, tx),
    (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
    (tx) => ({
      ...tx,
      instructions: [instruction],
    })
  );
  
  const signedTransaction = await signTransactionMessageWithSigners(transactionMessage);
  
  // Send and wait for confirmation
  return await sendAndWaitForTransaction(rpc, signedTransaction);
}

/**
 * Check if a ProgramAuthority account exists
 * 
 * @param {string} programAuthorityPda - ProgramAuthority PDA address
 * @param {string} cluster - Cluster name
 * @returns {Promise<boolean>} True if account exists
 */
export async function checkProgramAuthorityExists(programAuthorityPda, cluster = 'devnet') {
  try {
    const clusterUrl = getClusterUrl(cluster);
    const rpc = createSolanaRpc(clusterUrl);
    
    const accountInfo = await rpc.getAccountInfo(address(programAuthorityPda), {
      encoding: 'base64',
    }).send();
    
    return accountInfo.value !== null && accountInfo.value.data.length > 0;
  } catch (error) {
    return false;
  }
}

/**
 * Get explorer URL for a transaction
 * 
 * @param {string} signature - Transaction signature
 * @param {string} cluster - Cluster name
 * @returns {string} Explorer URL
 */
export function getExplorerUrl(signature, cluster = 'devnet') {
  const baseUrl = 'https://explorer.solana.com/tx';
  if (cluster === 'mainnet-beta') {
    return `${baseUrl}/${signature}`;
  }
  return `${baseUrl}/${signature}?cluster=${cluster}`;
}
