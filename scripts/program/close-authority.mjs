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
  closeProgramAuthority,
  getExplorerUrl,
} from '../lib/program-authority.mjs';

/**
 * Close Program Authority
 *
 * Closes the ProgramAuthority account and recovers its rent lamports.
 * This is primarily intended for devnet/testnet cleanup when you need
 * to reinitialize the authority after an upgrade that changed the account structure.
 *
 * ⚠️  WARNING: This removes global program authority control!
 * Only use this for testing/development scenarios.
 *
 * Usage: pnpm programs:close-authority [OPTIONS]
 */

// Get CLI arguments
const args = cliArguments();

// Show usage if help flag
if (args.includes('--help') || args.includes('-h')) {
  console.log(`
${chalk.blue('Close Program Authority')}
${chalk.gray('=').repeat(60)}

Close the ProgramAuthority account and recover its rent lamports.
This is intended for devnet/testnet cleanup scenarios.

${chalk.yellow('Usage:')}
  pnpm programs:close-authority [OPTIONS]

${chalk.yellow('Options:')}
  ${chalk.cyan('--cluster <NAME>')}     Cluster: devnet, testnet, mainnet-beta
                           (default: devnet)
  ${chalk.cyan('--keypair <PATH>')}     Path to authority keypair
                           (default: ~/.config/solana/id.json)
  ${chalk.cyan('--program-id <ID>')}    Program ID (auto-detected if not provided)
  ${chalk.cyan('--receiver <PUBKEY>')}  Receiver for lamports (default: authority)

${chalk.yellow('Examples:')}
  ${chalk.gray('# Close on devnet with default keypair')}
  pnpm programs:close-authority

  ${chalk.gray('# Close and send lamports to specific address')}
  pnpm programs:close-authority \\
    --receiver 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd

${chalk.yellow('Important Notes:')}
  ${chalk.red('⚠️  This removes global program authority!')}
  ${chalk.gray('  - Only the current authority can close')}
  ${chalk.gray('  - All lamports are transferred to receiver')}
  ${chalk.gray('  - Cannot close if authority transfer is pending')}
  ${chalk.gray('  - After closing, you can re-initialize with programs:init-authority')}

${chalk.red('⚠️  USE WITH CAUTION ON MAINNET!')}

${chalk.gray('=').repeat(60)}
`);
  process.exit(0);
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

const cluster = getOption('cluster', 'u') || 'devnet';
const keypairPath = await getKeypairPath(getOption('keypair', 'k'));
const providedProgramId = getOption('program-id', 'p');
const receiverPubkey = getOption('receiver', 'r');

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

echo(chalk.blue('\n' + '='.repeat(60)));
echo(chalk.blue('  Close Program Authority'));
echo(chalk.blue('='.repeat(60) + '\n'));

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

  // 5. Verify account exists on-chain
  echo(chalk.cyan('\nVerifying ProgramAuthority account...'));
  const clusterUrl = cluster === 'devnet' 
    ? 'https://api.devnet.solana.com'
    : cluster === 'testnet'
    ? 'https://api.testnet.solana.com'
    : 'https://api.mainnet-beta.solana.com';
  
  const accountInfoResponse = await fetch(clusterUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: '0',
      method: 'getAccountInfo',
      params: [
        programAuthorityPda,
        { encoding: 'base64', commitment: 'confirmed' }
      ]
    })
  });

  const accountInfo = await accountInfoResponse.json();
  
  if (!accountInfo.result || !accountInfo.result.value) {
    echo(chalk.yellow('⚠️  ProgramAuthority account does not exist'));
    echo(chalk.gray(`  PDA: ${programAuthorityPda}`));
    echo(chalk.gray('  Nothing to close.\n'));
    process.exit(0);
  }

  echo(chalk.green(`✓ ProgramAuthority account exists`));

  // 6. Determine receiver (default to authority)
  const receiver = receiverPubkey || authorityPubkey;

  // 7. Display close details
  echo(chalk.yellow('\nOperation Details:'));
  echo(chalk.gray(`  Program ID: ${programId}`));
  echo(chalk.gray(`  Program Authority: ${authorityPubkey}`));
  echo(chalk.gray(`  ProgramAuthority PDA: ${programAuthorityPda}`));
  echo(chalk.gray(`  Receiver: ${receiver}`));
  echo(chalk.gray(`  Cluster: ${cluster}`));
  echo(chalk.gray(`  Keypair: ${keypairPath}\n`));

  // 8. Confirmation prompt
  echo(chalk.red('⚠️  WARNING: This will close the ProgramAuthority account!'));
  echo(chalk.yellow('After closing, no one can manage authorized creators until re-initialized.\n'));
  echo(chalk.cyan('Type "close" to confirm:'));

  const response = await question('> ');
  if (response.trim() !== 'close') {
    echo(chalk.red('\n❌ Confirmation text did not match. Cancelled.\n'));
    process.exit(0);
  }

  // 9. Execute close
  echo(chalk.cyan('\n🔄 Closing ProgramAuthority...\n'));
  
  const signature = await closeProgramAuthority({
    programId,
    programAuthorityPda,
    authorityKeypairPath: keypairPath,
    receiver,
    cluster,
  });

  // 10. Success output
  echo(chalk.green('\n✅ ProgramAuthority closed successfully!\n'));
  echo(chalk.green('Transaction Details:'));
  echo(chalk.gray(`  Signature: ${signature}`));
  echo(chalk.gray(`  Closed PDA: ${programAuthorityPda}`));
  echo(chalk.gray(`  Lamports sent to: ${receiver}`));
  
  const explorerUrl = getExplorerUrl(signature, cluster);
  echo(chalk.cyan(`\n  View on explorer: ${explorerUrl}\n`));

  echo(chalk.green('✓ Account closed and rent recovered'));
  echo(chalk.gray('  You can now re-initialize with: pnpm programs:init-authority\n'));

} catch (error) {
  // Handle errors
  const errorMessage = error.stderr || error.message || error.toString();
  
  // Log full error for debugging
  if (process.env.DEBUG) {
    console.error('Full error:', error);
  }
  
  // Check common error scenarios
  if (
    errorMessage.includes('Unauthorized') ||
    errorMessage.includes('is not the program authority')
  ) {
    echo(chalk.red('\n❌ Authorization failed\n'));
    echo(chalk.gray('  Only the current authority can close the ProgramAuthority.\n'));
    echo(chalk.gray(`  PDA: ${programAuthorityPda}\n`));
  } else if (
    errorMessage.includes('pending') ||
    errorMessage.includes('transfer is pending')
  ) {
    echo(chalk.red('\n❌ Cannot close with pending authority transfer\n'));
    echo(chalk.gray('  Cancel the pending transfer first or wait for it to complete.\n'));
  } else if (error.message) {
    echo(chalk.red(`\n❌ Error: ${error.message}\n`));
  } else {
    echo(chalk.red('\n❌ Close operation failed!\n'));
    echo(chalk.red(errorMessage));
    echo('');
  }
  
  process.exit(1);
}

echo(chalk.green('✓ Program authority close complete!\n'));
