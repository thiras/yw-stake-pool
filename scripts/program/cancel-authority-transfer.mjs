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
 * Cancel Authority Transfer Script
 * 
 * Cancels a pending authority transfer for the ProgramAuthority account.
 * Must be called by the current authority before the pending authority accepts.
 * 
 * Usage: pnpm programs:cancel-authority-transfer [OPTIONS]
 */

// Get CLI arguments
const args = cliArguments();

// Show usage if help flag
if (args.includes('--help') || args.includes('-h')) {
  console.log(`
${chalk.blue('Cancel Authority Transfer')}
${chalk.gray('=').repeat(60)}

Cancels a pending authority transfer for the ProgramAuthority account.
Only the current authority can call this before the transfer is accepted.

${chalk.yellow('Usage:')}
  pnpm programs:cancel-authority-transfer [OPTIONS]

${chalk.yellow('Options:')}
  ${chalk.cyan('--program-id <ADDRESS>')}     Program ID (defaults to deployed program)
  ${chalk.cyan('--cluster <NAME>')}           Cluster: devnet, testnet, mainnet-beta (default: devnet)
  ${chalk.cyan('--keypair <PATH>')}           Path to current authority keypair (default: ~/.config/solana/id.json)

${chalk.yellow('Examples:')}
  ${chalk.gray('# Cancel pending transfer on devnet')}
  pnpm programs:cancel-authority-transfer

  ${chalk.gray('# Cancel on mainnet with custom keypair')}
  pnpm programs:cancel-authority-transfer \\
    --cluster mainnet-beta \\
    --keypair /path/to/current-authority.json

${chalk.yellow('Important Notes:')}
  ${chalk.red('⚠️  Only the current authority can cancel the transfer')}
  ${chalk.red('⚠️  Cannot cancel after the pending authority has accepted')}
  ${chalk.gray('  • Resets pending_authority to None')}

${chalk.gray('=').repeat(60)}
`);
  process.exit(0);
}

// Parse arguments
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
echo(chalk.blue('  Cancel Authority Transfer'));
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

// Fetch current program authority state
const clusterUrl = getClusterUrl(cluster);
const rpc = createSolanaRpc(clusterUrl);

echo(chalk.cyan('Fetching ProgramAuthority account state...'));
const accountInfo = await rpc.getAccountInfo(address(programAuthorityPda), {
  encoding: 'base64',
}).send();

if (!accountInfo.value || accountInfo.value.data.length === 0) {
  echo(chalk.red('❌ Error: ProgramAuthority account does not exist'));
  echo(chalk.gray('   Initialize it first with: pnpm programs:init-authority\n'));
  process.exit(1);
}

// Import account decoder
const clientPath = path.join(process.cwd(), 'clients/js/dist/src/index.js');
const { getProgramAuthorityDecoder } = await import(clientPath);

// Decode account data
const accountData = Buffer.from(accountInfo.value.data[0], 'base64');
const programAuthorityAccount = getProgramAuthorityDecoder().decode(accountData);

echo(chalk.yellow('Current ProgramAuthority state:'));
echo(chalk.gray(`  Current Authority: ${programAuthorityAccount.authority}`));
if (programAuthorityAccount.pendingAuthority) {
  echo(chalk.gray(`  Pending Authority: ${programAuthorityAccount.pendingAuthority}\n`));
} else {
  echo(chalk.gray(`  Pending Authority: None\n`));
  echo(chalk.red('❌ Error: No pending authority transfer to cancel\n'));
  process.exit(1);
}

// Verify that the caller is the current authority
if (programAuthorityAccount.authority !== currentAuthority.address) {
  echo(chalk.red('❌ Error: You are not the current authority'));
  echo(chalk.gray(`   Your address: ${currentAuthority.address}`));
  echo(chalk.gray(`   Current authority: ${programAuthorityAccount.authority}\n`));
  process.exit(1);
}

// Display cancellation details
echo(chalk.green('✓ You are the current authority\n'));
echo(chalk.yellow('Cancellation Details:'));
echo(chalk.gray(`  Program ID: ${programId}`));
echo(chalk.gray(`  ProgramAuthority PDA: ${programAuthorityPda}`));
echo(chalk.gray(`  Current Authority: ${currentAuthority.address}`));
echo(chalk.gray(`  Pending Authority (to cancel): ${programAuthorityAccount.pendingAuthority}`));
echo(chalk.gray(`  Cluster: ${cluster}\n`));

echo(chalk.red('⚠️  After cancellation:'));
echo(chalk.gray('  • Pending authority will be set to None'));
echo(chalk.gray(`  • ${programAuthorityAccount.pendingAuthority} will no longer be able to accept`));
echo(chalk.gray('  • You remain the current authority\n'));

// Confirm
echo(chalk.cyan('Cancel the pending authority transfer? (y/n)'));

let confirmed = false;
try {
  const response = await question('> ');
  confirmed = response.trim().toLowerCase() === 'y' || response.trim().toLowerCase() === 'yes';
} catch (error) {
  echo(chalk.red('\n❌ Cancellation aborted\n'));
  process.exit(0);
}

if (!confirmed) {
  echo(chalk.red('\n❌ Cancellation aborted\n'));
  process.exit(0);
}

// Import client library
const { getCancelAuthorityTransferInstruction } = await import(clientPath);

// Create instruction
echo(chalk.cyan('\n🔄 Creating cancel instruction...'));
const instruction = getCancelAuthorityTransferInstruction({
  programAuthority: address(programAuthorityPda),
  currentAuthority: currentAuthority,
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

echo(chalk.green('\n✅ Authority transfer cancelled successfully!\n'));
echo(chalk.yellow('Transaction Details:'));
echo(chalk.gray(`  Signature: ${signature}`));
echo(chalk.gray(`  Explorer: ${getExplorerUrl(signature, cluster)}\n`));

echo(chalk.yellow('Authority Status:'));
echo(chalk.gray(`  Current Authority: ${currentAuthority.address}`));
echo(chalk.gray(`  Pending Authority: None\n`));

echo(chalk.green('✓ No pending authority transfer\n'));
