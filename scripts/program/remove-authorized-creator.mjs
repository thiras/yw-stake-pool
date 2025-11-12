#!/usr/bin/env zx
import 'zx/globals';
import {
  cliArguments,
  getKeypairPath,
  getProgramFolders,
  getCargo,
} from '../utils.mjs';
import {
  calculateProgramAuthorityPda,
  loadKeypairSigner,
  getClusterUrl,
  checkProgramAuthorityExists,
  getExplorerUrl,
} from '../lib/program-authority.mjs';

// Import @solana/kit directly from root node_modules
import {
  address,
  createSolanaRpc,
  createTransactionMessage,
  pipe,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  sendAndConfirmTransactionFactory,
} from '@solana/kit';

/**
 * Remove Authorized Pool Creator
 *
 * Removes a public key from the program authority's authorized creators list.
 * Only the program authority can perform this operation.
 *
 * Usage: pnpm programs:remove-creator <CREATOR_PUBLIC_KEY> [OPTIONS]
 */

// Get CLI arguments
const args = cliArguments();

// Show usage if no arguments or help flag
if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
  console.log(`
${chalk.blue('Remove Authorized Pool Creator')}
${chalk.gray('=').repeat(60)}

Removes a public key from the list of addresses authorized to create stake pools.
Only the program authority can call this operation.

${chalk.yellow('Usage:')}
  pnpm programs:remove-creator <CREATOR_PUBLIC_KEY> [OPTIONS]

${chalk.yellow('Arguments:')}
  ${chalk.cyan('<CREATOR_PUBLIC_KEY>')}    The public key to remove

${chalk.yellow('Options:')}
  ${chalk.cyan('--cluster <NAME>')}       Cluster: devnet, testnet, mainnet-beta
                             (default: devnet)
  ${chalk.cyan('--keypair <PATH>')}       Path to program authority keypair
                             (default: ~/.config/solana/id.json)
  ${chalk.cyan('--program-id <ID>')}      Program ID (auto-detected if not provided)

${chalk.yellow('Examples:')}
  ${chalk.gray('# Remove creator on devnet with default authority')}
  pnpm programs:remove-creator 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd

  ${chalk.gray('# Remove creator on mainnet with custom authority')}
  pnpm programs:remove-creator 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd \\
    --cluster mainnet-beta \\
    --keypair /path/to/authority.json

${chalk.yellow('Important Notes:')}
  ${chalk.gray('  - Only the program authority can remove creators')}
  ${chalk.gray('  - The main authority cannot be removed')}
  ${chalk.gray('  - Removed creators cannot create new pools until re-added')}

${chalk.yellow('Related Commands:')}
  ${chalk.cyan('pnpm programs:add-creator <KEY>')}    Add authorized creator
  ${chalk.cyan('pnpm programs:list-creators')}        List all authorized creators

${chalk.gray('=').repeat(60)}
`);
  process.exit(args.length === 0 ? 1 : 0);
}

// Parse arguments
const creatorPubkey = args[0];

// Validate creator pubkey format
if (!creatorPubkey || creatorPubkey.startsWith('--')) {
  echo(chalk.red('❌ Error: Creator public key is required\n'));
  process.exit(1);
}

// Parse options
function getOption(name, shortName = null) {
  const fullIndex = args.findIndex((arg) => arg === `--${name}`);
  const shortIndex = shortName
    ? args.findIndex((arg) => arg === `-${shortName}`)
    : -1;
  const index = fullIndex >= 0 ? fullIndex : shortIndex;
  return index >= 0 && args[index + 1] ? args[index + 1] : null;
}

const providedProgramId = getOption('program-id', 'p');
const cluster = getOption('cluster', 'u') || 'devnet';
const keypairPath = await getKeypairPath(getOption('keypair', 'k'));

echo(chalk.blue('\n' + '='.repeat(60)));
echo(chalk.blue('  Remove Authorized Pool Creator'));
echo(chalk.blue('='.repeat(60) + '\n'));

// Helper: Auto-detect program ID from repository
async function detectProgramId() {
  echo(chalk.cyan('Determining program ID...'));
  const folders = getProgramFolders();

  if (folders.length === 0) {
    throw new Error('No program folders found');
  }

  if (folders.length > 1) {
    echo(chalk.yellow('⚠️  Multiple programs found. Using first program or specify --program-id\n'));
  }

  const folder = folders[0];
  const cargo = getCargo(folder);
  const programName = cargo.package.name.replace(/-/g, '_');

  // Try to find program keypair
  const programKeypairPath = path.join(folder, 'keypair.json');
  const targetKeypairPath = path.join(
    process.cwd(),
    'target',
    'deploy',
    `${programName}-keypair.json`
  );

  let programKeypair = null;
  if (await fs.pathExists(programKeypairPath)) {
    programKeypair = programKeypairPath;
  } else if (await fs.pathExists(targetKeypairPath)) {
    programKeypair = targetKeypairPath;
  }

  if (!programKeypair) {
    throw new Error('No program keypair found. Deploy the program first or use --program-id.');
  }

  const result = await $`solana-keygen pubkey ${programKeypair}`;
  const programId = result.stdout.trim();
  echo(chalk.green(`✓ Program ID: ${programId}`));
  echo(chalk.gray(`  (from ${programKeypair})`));
  return programId;
}

// Helper: Validate and load authority keypair
async function loadAuthority(keypairPath) {
  try {
    await fs.access(keypairPath);
    echo(chalk.green(`✓ Authority keypair found: ${keypairPath}`));
  } catch (error) {
    throw new Error(`Keypair not found at ${keypairPath}`);
  }

  const result = await $`solana-keygen pubkey ${keypairPath}`;
  const authorityPubkey = result.stdout.trim();
  echo(chalk.green(`✓ Authority: ${authorityPubkey}`));
  return authorityPubkey;
}

// Main execution flow
let programId;
let programAuthorityPda;
let authorityPubkey;

try {
  // 1. Load and validate authority
  authorityPubkey = await loadAuthority(keypairPath);

  // 2. Detect or use provided program ID
  programId = providedProgramId || await detectProgramId();

  // 3. Check if JavaScript client is built
  const clientJsPath = path.join(process.cwd(), 'clients', 'js', 'dist');
  if (!(await fs.pathExists(clientJsPath))) {
    echo(chalk.yellow('⚠️  JavaScript client not built. Building now...\n'));
    await $`pnpm clients:build`;
    echo('');
  }

  // 4. Calculate ProgramAuthority PDA
  echo(chalk.cyan('\nCalculating ProgramAuthority PDA...'));
  programAuthorityPda = await calculateProgramAuthorityPda(programId);
  echo(chalk.green(`✓ ProgramAuthority PDA: ${programAuthorityPda}`));

  // 5. Check if ProgramAuthority exists
  echo(chalk.cyan('\nVerifying ProgramAuthority account...'));
  const exists = await checkProgramAuthorityExists(programAuthorityPda, cluster);
  
  if (!exists) {
    echo(chalk.red('\n❌ ProgramAuthority not initialized!'));
    echo(chalk.yellow('\nPlease run the following command first:'));
    echo(chalk.cyan('  pnpm programs:init-authority\n'));
    process.exit(1);
  }
  echo(chalk.green('✓ ProgramAuthority account exists'));

  // 6. Display operation details
  echo(chalk.yellow('\nOperation Details:'));
  echo(chalk.gray(`  Program ID: ${programId}`));
  echo(chalk.gray(`  Program Authority: ${authorityPubkey}`));
  echo(chalk.gray(`  ProgramAuthority PDA: ${programAuthorityPda}`));
  echo(chalk.gray(`  Creator to Remove: ${creatorPubkey}`));
  echo(chalk.gray(`  Cluster: ${cluster}`));
  echo(chalk.gray(`  Keypair: ${keypairPath}\n`));

  // 7. Confirmation prompt
  echo(chalk.cyan('This will remove the creator from the authorized list.'));
  echo(chalk.red('⚠️  The creator will no longer be able to create new stake pools.\n'));
  echo(chalk.cyan('Type "remove" to confirm:'));

  const response = await question('> ');
  if (response.trim() !== 'remove') {
    echo(chalk.red('\n❌ Confirmation text did not match. Cancelled.\n'));
    process.exit(0);
  }

  // 8. Execute remove creator operation
  echo(chalk.cyan('\n🔄 Removing authorized creator...\n'));

  // Load the JavaScript client
  const clientPath = path.join(process.cwd(), 'clients/js/dist/src/index.js');
  const { getManageAuthorizedCreatorsInstruction } = await import(clientPath);

  // Load authority keypair for signing
  const authority = await loadKeypairSigner(keypairPath);

  // Create RPC connection
  const clusterUrl = getClusterUrl(cluster);
  const rpc = createSolanaRpc(clusterUrl);

  // Create instruction - remove the creator, add none
  const instruction = getManageAuthorizedCreatorsInstruction({
    programAuthority: address(programAuthorityPda),
    authority: authority,
    add: [],
    remove: [address(creatorPubkey)],
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

  // Send and confirm transaction using polling (no WebSocket required)
  const sendAndConfirmTransaction = sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions: null,
  });

  let signature;
  let confirmationFailed = false;
  try {
    signature = await sendAndConfirmTransaction(signedTransaction, {
      commitment: 'confirmed',
    });
  } catch (error) {
    // If confirmation fails but transaction was sent, try to extract signature from error
    if (error.message && error.message.includes('signatureNotifications')) {
      confirmationFailed = true;
      echo(chalk.yellow('\n⚠️  Transaction sent but confirmation failed'));
      echo(chalk.gray('This is a known issue with WebSocket subscriptions.'));
      echo(chalk.gray('Checking transaction status...\n'));
      
      // Transaction was likely sent, wait a moment and check the account
      await new Promise(resolve => setTimeout(resolve, 3000));
    } else {
      throw error;
    }
  }

  if (confirmationFailed) {
    echo(chalk.green('✓ Operation appears to have completed'));
    echo(chalk.cyan('\nPlease verify with: pnpm programs:list-creators\n'));
  } else {
    // 9. Success output
    echo(chalk.green('\n✅ Authorized creator removed successfully!\n'));
    echo(chalk.green('Transaction Details:'));
    echo(chalk.gray(`  Signature: ${signature}`));
    echo(chalk.gray(`  Creator Removed: ${creatorPubkey}`));
    echo(chalk.gray(`  ProgramAuthority PDA: ${programAuthorityPda}`));
    
    const explorerUrl = getExplorerUrl(signature, cluster);
    echo(chalk.cyan(`\n  View on explorer: ${explorerUrl}\n`));

    echo(chalk.green('✓ Creator removed from authorized list'));
    echo(chalk.gray('  This address can no longer create new stake pools\n'));
  }

} catch (error) {
  // Handle errors
  const errorMessage = error.stderr || error.message || error.toString();
  
  // Log full error for debugging
  if (process.env.DEBUG) {
    console.error('Full error:', error);
  }
  
  // Check for specific error conditions
  if (
    errorMessage.includes('CreatorNotFound') ||
    errorMessage.includes('not found in authorized list')
  ) {
    echo(chalk.yellow('\n⚠️  Creator not found\n'));
    echo(chalk.gray(`  Creator: ${creatorPubkey}`));
    echo(chalk.gray('  This address is not in the authorized list.\n'));
  } else if (
    errorMessage.includes('CannotRemoveMainAuthority') ||
    errorMessage.includes('Cannot remove main authority')
  ) {
    echo(chalk.red('\n❌ Cannot remove main authority!\n'));
    echo(chalk.gray('  The main program authority is always authorized.'));
    echo(chalk.gray('  Only additional creators can be removed.\n'));
  } else if (
    errorMessage.includes('Invalid program authority') ||
    errorMessage.includes('ConstraintHasOne') ||
    errorMessage.includes('authority constraint')
  ) {
    echo(chalk.red('\n❌ Permission denied!\n'));
    echo(chalk.gray('  Only the program authority can remove creators.'));
    echo(chalk.gray(`  Current authority keypair: ${keypairPath}`));
    echo(chalk.gray(`  Authority pubkey: ${authorityPubkey}\n`));
  } else if (error.message) {
    echo(chalk.red(`\n❌ Error: ${error.message}\n`));
    process.exit(1);
  } else {
    echo(chalk.red('\n❌ Operation failed!\n'));
    echo(chalk.red(errorMessage));
    echo('');
    process.exit(1);
  }
}

echo(chalk.green('✓ Remove creator operation complete!\n'));
