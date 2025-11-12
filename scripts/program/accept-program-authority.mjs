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
 * Accept Program Authority Script
 * 
 * Accepts a pending authority transfer for the ProgramAuthority account (Step 2 of 2).
 * Must be called by the nominated pending authority to complete the transfer.
 * 
 * Usage: pnpm programs:accept-authority [OPTIONS]
 */

// Get CLI arguments
const args = cliArguments();

// Show usage if help flag
if (args.includes('--help') || args.includes('-h')) {
  console.log(`
${chalk.blue('Accept Program Authority (Step 2 of 2)')}
${chalk.gray('=').repeat(60)}

Accepts a pending authority transfer for the ProgramAuthority account.
This completes the two-step authority transfer process.

${chalk.yellow('Usage:')}
  pnpm programs:accept-authority [OPTIONS]

${chalk.yellow('Options:')}
  ${chalk.cyan('--program-id <ADDRESS>')}     Program ID (defaults to deployed program)
  ${chalk.cyan('--cluster <NAME>')}           Cluster: devnet, testnet, mainnet-beta (default: devnet)
  ${chalk.cyan('--keypair <PATH>')}           Path to pending authority keypair (default: ~/.config/solana/id.json)

${chalk.yellow('Examples:')}
  ${chalk.gray('# Accept authority transfer on devnet')}
  pnpm programs:accept-authority

  ${chalk.gray('# Accept on mainnet with custom keypair')}
  pnpm programs:accept-authority \\
    --cluster mainnet-beta \\
    --keypair /path/to/new-authority.json

${chalk.yellow('Important Notes:')}
  ${chalk.red('⚠️  You must be the nominated pending authority to call this')}
  ${chalk.red('⚠️  After acceptance, you become the new authority immediately')}
  ${chalk.red('⚠️  Previous authority loses all control')}

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
echo(chalk.blue('  Accept Program Authority (Step 2)'));
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

// Load pending authority keypair
echo(chalk.cyan('Loading pending authority keypair...'));
let pendingAuthority;
try {
  pendingAuthority = await loadKeypairSigner(keypairPath);
  echo(chalk.green(`✓ Pending Authority: ${pendingAuthority.address}\n`));
} catch (error) {
  echo(chalk.red(`❌ Error: Failed to load keypair from ${keypairPath}`));
  echo(chalk.red(error.message));
  process.exit(1);
}

// Fetch current program authority state to verify pending authority
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
  echo(chalk.red('❌ Error: No pending authority transfer found'));
  echo(chalk.gray('   Current authority must first nominate you with:'));
  echo(chalk.cyan('   pnpm programs:transfer-authority <YOUR_PUBKEY>\n'));
  process.exit(1);
}

// Verify that the pending authority matches
if (programAuthorityAccount.pendingAuthority !== pendingAuthority.address) {
  echo(chalk.red('❌ Error: Your address does not match the pending authority'));
  echo(chalk.gray(`   Your address: ${pendingAuthority.address}`));
  echo(chalk.gray(`   Pending authority: ${programAuthorityAccount.pendingAuthority}\n`));
  process.exit(1);
}

// Display transfer details
echo(chalk.green('✓ You are the nominated pending authority!\n'));
echo(chalk.yellow('Transfer Details:'));
echo(chalk.gray(`  Program ID: ${programId}`));
echo(chalk.gray(`  ProgramAuthority PDA: ${programAuthorityPda}`));
echo(chalk.gray(`  Previous Authority: ${programAuthorityAccount.authority}`));
echo(chalk.gray(`  New Authority (you): ${pendingAuthority.address}`));
echo(chalk.gray(`  Cluster: ${cluster}\n`));

echo(chalk.red('⚠️  After acceptance:'));
echo(chalk.gray('  • You become the new authority immediately'));
echo(chalk.gray(`  • Previous authority (${programAuthorityAccount.authority}) loses all control`));
echo(chalk.gray('  • This action is IRREVERSIBLE\n'));

// Confirm
echo(chalk.cyan('Accept authority transfer? (y/n)'));

let confirmed = false;
try {
  const response = await question('> ');
  confirmed = response.trim().toLowerCase() === 'y' || response.trim().toLowerCase() === 'yes';
} catch (error) {
  echo(chalk.red('\n❌ Transfer acceptance cancelled\n'));
  process.exit(0);
}

if (!confirmed) {
  echo(chalk.red('\n❌ Transfer acceptance cancelled\n'));
  process.exit(0);
}

// Import client library
const { getAcceptProgramAuthorityInstruction } = await import(clientPath);

// Create instruction
echo(chalk.cyan('\n🔄 Creating accept instruction...'));
const instruction = getAcceptProgramAuthorityInstruction({
  programAuthority: address(programAuthorityPda),
  pendingAuthority: pendingAuthority,
});

// Get recent blockhash
echo(chalk.cyan('📡 Fetching recent blockhash...'));
const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

// Create and sign transaction
echo(chalk.cyan('✍️  Signing transaction...'));
const transactionMessage = pipe(
  createTransactionMessage({ version: 0 }),
  (tx) => setTransactionMessageFeePayerSigner(pendingAuthority, tx),
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

echo(chalk.green('\n✅ Authority transfer accepted successfully!\n'));
echo(chalk.yellow('Transaction Details:'));
echo(chalk.gray(`  Signature: ${signature}`));
echo(chalk.gray(`  Explorer: ${getExplorerUrl(signature, cluster)}\n`));

echo(chalk.yellow('Authority Change Summary:'));
echo(chalk.gray(`  Previous Authority: ${programAuthorityAccount.authority}`));
echo(chalk.gray(`  New Authority: ${pendingAuthority.address}\n`));

echo(chalk.green('✓ You are now the program authority!\n'));
