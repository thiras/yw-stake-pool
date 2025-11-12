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
  sendAndWaitForTransaction,
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

/**
 * Transfer Program Authority Script
 * 
 * Nominates a new authority for the ProgramAuthority account (Step 1 of 2).
 * The new authority must then call accept-program-authority to complete the transfer.
 * 
 * Usage: pnpm programs:transfer-authority <NEW_AUTHORITY_PUBKEY> [OPTIONS]
 */

// Get CLI arguments
const args = cliArguments();

// Show usage if no arguments or help flag
if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
  console.log(`
${chalk.blue('Transfer Program Authority (Step 1 of 2)')}
${chalk.gray('=').repeat(60)}

Nominates a new authority for the ProgramAuthority account.
This is the first step of a two-step authority transfer process.

${chalk.yellow('Usage:')}
  pnpm programs:transfer-authority <NEW_AUTHORITY_PUBKEY> [OPTIONS]

${chalk.yellow('Arguments:')}
  ${chalk.cyan('<NEW_AUTHORITY_PUBKEY>')}  The public key of the new authority to nominate

${chalk.yellow('Options:')}
  ${chalk.cyan('--program-id <ADDRESS>')}     Program ID (defaults to deployed program)
  ${chalk.cyan('--cluster <NAME>')}           Cluster: devnet, testnet, mainnet-beta (default: devnet)
  ${chalk.cyan('--keypair <PATH>')}           Path to current authority keypair (default: ~/.config/solana/id.json)

${chalk.yellow('Examples:')}
  ${chalk.gray('# Nominate new authority on devnet')}
  pnpm programs:transfer-authority 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd

  ${chalk.gray('# Nominate on mainnet with custom keypair')}
  pnpm programs:transfer-authority 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd \\
    --cluster mainnet-beta \\
    --keypair /path/to/current-authority.json

${chalk.yellow('Important Notes:')}
  ${chalk.red('⚠️  This is a TWO-STEP process:')}
  ${chalk.gray('  1. Current authority calls this script to nominate new authority')}
  ${chalk.gray('  2. New authority calls accept-program-authority to complete transfer')}
  
  ${chalk.gray('  • Current authority retains control until new authority accepts')}
  ${chalk.gray('  • Transfer can be cancelled by current authority before acceptance')}

${chalk.gray('=').repeat(60)}
`);
  process.exit(args.length === 0 ? 1 : 0);
}

// Parse arguments
const newAuthorityPubkey = args[0];

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

// Validate new authority pubkey
if (!newAuthorityPubkey || newAuthorityPubkey.startsWith('--')) {
  echo(chalk.red('❌ Error: New authority public key is required\n'));
  process.exit(1);
}

echo(chalk.blue('\n' + '='.repeat(60)));
echo(chalk.blue('  Transfer Program Authority (Step 1)'));
echo(chalk.blue('='.repeat(60) + '\n'));

// Determine program ID
let programId = providedProgramId;

if (!programId) {
  echo(chalk.cyan('Determining program ID from repository...'));
  const folders = getProgramFolders();

  if (folders.length === 0) {
    echo(chalk.red('❌ No program folders found\n'));
    process.exit(1);
  }

  if (folders.length > 1 && !providedProgramId) {
    echo(
      chalk.yellow('⚠️  Multiple programs found. Please specify --program-id\n')
    );
    echo('Available programs:');
    for (const folder of folders) {
      const cargo = getCargo(folder);
      echo(chalk.gray(`  - ${cargo.package.name}`));
    }
    echo('');
    process.exit(1);
  }

  const folder = folders[0];
  
  // Try to find program keypair
  const programKeypairPath = path.join(folder, 'keypair.json');

  if (await fs.pathExists(programKeypairPath)) {
    try {
      const keypairData = JSON.parse(await fs.readFile(programKeypairPath, 'utf-8'));
      const programKeypair = await createKeyPairSignerFromBytes(new Uint8Array(keypairData));
      programId = programKeypair.address;
      echo(chalk.green(`✓ Found program keypair: ${programKeypairPath}`));
      echo(chalk.green(`✓ Program ID: ${programId}\n`));
    } catch (error) {
      echo(chalk.red('❌ Failed to read program keypair\n'));
      process.exit(1);
    }
  } else {
    echo(
      chalk.red('❌ No program keypair found. Use --program-id to specify.\n')
    );
    process.exit(1);
  }
}

// Calculate ProgramAuthority PDA
echo(chalk.cyan('Calculating ProgramAuthority PDA...'));
const programAuthorityPda = await calculateProgramAuthorityPda(programId);
echo(chalk.green(`✓ ProgramAuthority PDA: ${programAuthorityPda}\n`));

// Load current authority keypair
echo(chalk.cyan('Loading current authority keypair...'));
let currentAuthority;
try {
  currentAuthority = await loadKeypairSigner(keypairPath);
  echo(chalk.green(`✓ Current Authority: ${currentAuthority.address}\n`));
} catch (error) {
  echo(chalk.red(`❌ Error: Failed to load keypair from ${keypairPath}`));
  echo(chalk.red(error.message));
  process.exit(1);
}

// Display transfer details
echo(chalk.yellow('Transfer Details:'));
echo(chalk.gray(`  Program ID: ${programId}`));
echo(chalk.gray(`  ProgramAuthority PDA: ${programAuthorityPda}`));
echo(chalk.gray(`  Current Authority: ${currentAuthority.address}`));
echo(chalk.gray(`  New Authority (pending): ${newAuthorityPubkey}`));
echo(chalk.gray(`  Cluster: ${cluster}\n`));

echo(chalk.red('⚠️  After this transaction:'));
echo(chalk.gray('  • Current authority retains control'));
echo(chalk.gray(`  • New authority (${newAuthorityPubkey}) must accept the transfer`));
echo(chalk.gray('  • Transfer can be cancelled before acceptance\n'));

// Confirm
echo(chalk.cyan('Proceed with authority transfer nomination? (y/n)'));

let confirmed = false;
try {
  const response = await question('> ');
  confirmed = response.trim().toLowerCase() === 'y' || response.trim().toLowerCase() === 'yes';
} catch (error) {
  echo(chalk.red('\n❌ Transfer cancelled\n'));
  process.exit(0);
}

if (!confirmed) {
  echo(chalk.red('\n❌ Transfer cancelled\n'));
  process.exit(0);
}

// Import client library
const clientPath = path.join(process.cwd(), 'clients/js/dist/src/index.js');
const { getTransferProgramAuthorityInstruction } = await import(clientPath);

// Create RPC connection
const clusterUrl = getClusterUrl(cluster);
const rpc = createSolanaRpc(clusterUrl);

// Create instruction
echo(chalk.cyan('\n🔄 Creating transfer instruction...'));
const instruction = getTransferProgramAuthorityInstruction({
  programAuthority: address(programAuthorityPda),
  currentAuthority: currentAuthority,
  newAuthority: address(newAuthorityPubkey),
});

// Get recent blockhash
echo(chalk.cyan('📡 Fetching recent blockhash...'));
const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

// Create and sign transaction
echo(chalk.cyan('✍️  Signing transaction...'));
const transactionMessage = pipe(
  createTransactionMessage({ version: 0 }),
  (tx) => setTransactionMessageFeePayerSigner(currentAuthority, tx),
  (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
  (tx) => ({
    ...tx,
    instructions: [instruction],
  })
);

const signedTransaction = await signTransactionMessageWithSigners(transactionMessage);

// Send transaction
echo(chalk.cyan('📤 Sending transaction...'));
const signature = await sendAndWaitForTransaction(rpc, signedTransaction);

echo(chalk.green('\n✅ Authority transfer nominated successfully!\n'));
echo(chalk.yellow('Transaction Details:'));
echo(chalk.gray(`  Signature: ${signature}`));
echo(chalk.gray(`  Explorer: ${getExplorerUrl(signature, cluster)}\n`));

echo(chalk.yellow('Next Steps:'));
echo(chalk.gray(`  1. The new authority (${newAuthorityPubkey}) must run:`));
echo(chalk.cyan(`     pnpm programs:accept-authority --cluster ${cluster}`));
echo(chalk.gray('  2. Or, current authority can cancel with:'));
echo(chalk.cyan(`     pnpm programs:cancel-authority-transfer --cluster ${cluster}\n`));
