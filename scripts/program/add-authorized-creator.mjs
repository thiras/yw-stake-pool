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
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  createTransactionMessage,
  pipe,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
} from '@solana/kit';
import { sendAndWaitForTransaction } from '../lib/program-authority.mjs';

/**
 * Add Authorized Pool Creator
 *
 * Adds a public key to the program authority's authorized creators list.
 * Only addresses in this list (plus the main authority) can create new stake pools.
 *
 * Usage: pnpm programs:add-creator <CREATOR_PUBLIC_KEY> [OPTIONS]
 */

// Get CLI arguments
const args = cliArguments();

// Show usage if no arguments or help flag
if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
  console.log(`
${chalk.blue('Add Authorized Pool Creator')}
${chalk.gray('=').repeat(60)}

Adds a public key to the list of addresses authorized to create stake pools.
Only the program authority can call this operation.

${chalk.yellow('Usage:')}
  pnpm programs:add-creator <CREATOR_PUBLIC_KEY> [OPTIONS]

${chalk.yellow('Arguments:')}
  ${chalk.cyan('<CREATOR_PUBLIC_KEY>')}    The public key to authorize

${chalk.yellow('Options:')}
  ${chalk.cyan('--cluster <NAME>')}       Cluster: devnet, testnet, mainnet-beta
                             (default: devnet)
  ${chalk.cyan('--keypair <PATH>')}       Path to program authority keypair
                             (default: ~/.config/solana/id.json)
  ${chalk.cyan('--program-id <ID>')}      Program ID (auto-detected if not provided)

${chalk.yellow('Examples:')}
  ${chalk.gray('# Add creator on devnet with default authority')}
  pnpm programs:add-creator 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd

  ${chalk.gray('# Add creator on mainnet with custom authority')}
  pnpm programs:add-creator 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd \\
    --cluster mainnet-beta \\
    --keypair /path/to/authority.json

  ${chalk.gray('# Add creator with specific program ID')}
  pnpm programs:add-creator 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd \\
    --program-id 8PtjrGvKNeZt2vCmRkSPGjss7TAFhvxux2N8r67UMKBx

${chalk.yellow('Important Notes:')}
  ${chalk.gray('  - Only the program authority can add creators')}
  ${chalk.gray('  - The main authority is always authorized (no need to add)')}
  ${chalk.gray('  - Maximum 10 authorized creators per program')}
  ${chalk.gray('  - ProgramAuthority must be initialized first')}
  ${chalk.gray('  - Run "pnpm programs:init-authority" if not initialized')}

${chalk.yellow('Related Commands:')}
  ${chalk.cyan('pnpm programs:init-authority')}        Initialize program authority
  ${chalk.cyan('pnpm programs:remove-creator <KEY>')}  Remove authorized creator
  ${chalk.cyan('pnpm programs:list-creators')}         List all authorized creators

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

// Basic validation - check if it looks like a base58 pubkey
if (creatorPubkey.length < 32 || creatorPubkey.length > 44) {
  echo(chalk.yellow('⚠️  Warning: Public key length seems unusual (expected 32-44 chars)\n'));
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
echo(chalk.blue('  Add Authorized Pool Creator'));
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

try {
  // 1. Load and validate authority
  const authorityPubkey = await loadAuthority(keypairPath);

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
  echo(chalk.gray(`  Creator to Add: ${creatorPubkey}`));
  echo(chalk.gray(`  Cluster: ${cluster}`));
  echo(chalk.gray(`  Keypair: ${keypairPath}\n`));

  // 7. Confirmation prompt
  echo(chalk.cyan('This will add the creator to the authorized list.'));
  echo(chalk.yellow('⚠️  Only addresses in this list can create new stake pools.\n'));
  echo(chalk.cyan('Type "add" to confirm:'));

  const response = await question('> ');
  if (response.trim() !== 'add') {
    echo(chalk.red('\n❌ Confirmation text did not match. Cancelled.\n'));
    process.exit(0);
  }

  // 8. Execute add creator operation
  echo(chalk.cyan('\n🔄 Adding authorized creator...\n'));

  // Load the JavaScript client
  const clientPath = path.join(process.cwd(), 'clients/js/dist/src/index.js');
  const { getManageAuthorizedCreatorsInstruction } = await import(clientPath);

  // Load authority keypair for signing
  const authority = await loadKeypairSigner(keypairPath);

  // Create RPC connection
  const clusterUrl = getClusterUrl(cluster);
  const rpc = createSolanaRpc(clusterUrl);

  // Create instruction - add the creator, remove none
  const instruction = getManageAuthorizedCreatorsInstruction({
    programAuthority: address(programAuthorityPda),
    authority: authority,
    add: [address(creatorPubkey)],
    remove: [],
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
  const signature = await sendAndWaitForTransaction(rpc, signedTransaction);

  // Success output
  echo(chalk.green('\n✅ Authorized creator added successfully!\n'));
  echo(chalk.green('Transaction Details:'));
  echo(chalk.gray(`  Signature: ${signature}`));
  echo(chalk.gray(`  Creator Added: ${creatorPubkey}`));
  echo(chalk.gray(`  ProgramAuthority PDA: ${programAuthorityPda}`));
  
  const explorerUrl = getExplorerUrl(signature, cluster);
  echo(chalk.cyan(`\n  View on explorer: ${explorerUrl}\n`));

  echo(chalk.green('✓ Creator is now authorized to create stake pools'));
  echo(chalk.gray('  The creator can now call initialize_pool instruction\n'));

} catch (error) {
  // Handle errors
  const errorMessage = error.stderr || error.message || error.toString();
  
  // Log full error for debugging
  if (process.env.DEBUG) {
    console.error('Full error:', error);
  }
  
  // Check for specific error conditions
  if (
    errorMessage.includes('CreatorAlreadyAuthorized') ||
    errorMessage.includes('already authorized')
  ) {
    echo(chalk.yellow('\n⚠️  Creator already authorized\n'));
    echo(chalk.gray(`  Creator: ${creatorPubkey}`));
    echo(chalk.gray('  This address is already in the authorized list.\n'));
  } else if (
    errorMessage.includes('MaxAuthorizedCreatorsReached') ||
    errorMessage.includes('maximum number')
  ) {
    echo(chalk.red('\n❌ Maximum authorized creators reached!\n'));
    echo(chalk.gray('  Maximum 10 creators can be authorized per program.'));
    echo(chalk.gray('  Remove an existing creator to add a new one.\n'));
    echo(chalk.cyan('  Use: pnpm programs:remove-creator <KEY>\n'));
  } else if (
    errorMessage.includes('Invalid program authority') ||
    errorMessage.includes('ConstraintHasOne') ||
    errorMessage.includes('authority constraint')
  ) {
    echo(chalk.red('\n❌ Permission denied!\n'));
    echo(chalk.gray('  Only the program authority can add creators.'));
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

echo(chalk.green('✓ Add creator operation complete!\n'));
