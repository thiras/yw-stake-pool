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
  initializeProgramAuthority,
  getExplorerUrl,
} from '../lib/program-authority.mjs';

/**
 * Initialize Program Authority
 *
 * This is a one-time setup step that creates the ProgramAuthority account,
 * which controls who can create new stake pools. Must be run after deploying
 * the program for the first time.
 *
 * Usage: pnpm programs:init-authority [OPTIONS]
 */

// Get CLI arguments
const args = cliArguments();

// Show usage if help flag
if (args.includes('--help') || args.includes('-h')) {
  console.log(`
${chalk.blue('Initialize Program Authority')}
${chalk.gray('=').repeat(60)}

One-time setup to initialize the ProgramAuthority account after deploying
the stake pool program. This account controls who can create new pools.

${chalk.yellow('Usage:')}
  pnpm programs:init-authority [OPTIONS]

${chalk.yellow('Options:')}
  ${chalk.cyan('--cluster <NAME>')}     Cluster: devnet, testnet, mainnet-beta
                           (default: devnet)
  ${chalk.cyan('--keypair <PATH>')}     Path to authority keypair
                           (default: ~/.config/solana/id.json)
  ${chalk.cyan('--program-id <ID>')}    Program ID (auto-detected if not provided)

${chalk.yellow('Examples:')}
  ${chalk.gray('# Initialize on devnet with default keypair')}
  pnpm programs:init-authority

  ${chalk.gray('# Initialize on mainnet with custom authority')}
  pnpm programs:init-authority \\
    --cluster mainnet-beta \\
    --keypair /path/to/authority.json

  ${chalk.gray('# Initialize with specific program ID')}
  pnpm programs:init-authority \\
    --program-id 8PtjrGvKNeZt2vCmRkSPGjss7TAFhvxux2N8r67UMKBx

${chalk.yellow('Important Notes:')}
  ${chalk.red('⚠️  This should only be run ONCE after deploying the program')}
  ${chalk.gray('  - Creates the ProgramAuthority PDA account')}
  ${chalk.gray('  - The authority can manage who creates pools')}
  ${chalk.gray('  - Running this again will fail if already initialized')}
  ${chalk.gray('  - The authority is automatically authorized to create pools')}

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
echo(chalk.blue('  Initialize Program Authority'));
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

  // 5. Display initialization details
  echo(chalk.yellow('\nInitialization Details:'));
  echo(chalk.gray(`  Program ID: ${programId}`));
  echo(chalk.gray(`  Authority: ${authorityPubkey}`));
  echo(chalk.gray(`  ProgramAuthority PDA: ${programAuthorityPda}`));
  echo(chalk.gray(`  Cluster: ${cluster}`));
  echo(chalk.gray(`  Keypair: ${keypairPath}\n`));

  // 6. Confirmation prompt
  echo(chalk.cyan('This will initialize the ProgramAuthority account.'));
  echo(chalk.yellow('⚠️  This should only be done once per program deployment.\n'));
  echo(chalk.cyan('Type "initialize" to confirm:'));

  const response = await question('> ');
  if (response.trim() !== 'initialize') {
    echo(chalk.red('\n❌ Confirmation text did not match. Cancelled.\n'));
    process.exit(0);
  }

  // 7. Execute initialization
  echo(chalk.cyan('\n🔄 Initializing ProgramAuthority...\n'));
  
  const signature = await initializeProgramAuthority({
    programId,
    programAuthorityPda,
    authorityKeypairPath: keypairPath,
    cluster,
  });

  // 8. Success output
  echo(chalk.green('\n✅ ProgramAuthority initialized successfully!\n'));
  echo(chalk.green('Transaction Details:'));
  echo(chalk.gray(`  Signature: ${signature}`));
  echo(chalk.gray(`  Authority: ${authorityPubkey}`));
  echo(chalk.gray(`  PDA: ${programAuthorityPda}`));
  
  const explorerUrl = getExplorerUrl(signature, cluster);
  echo(chalk.cyan(`\n  View on explorer: ${explorerUrl}\n`));

  echo(chalk.green('✓ The authority is now authorized to create pools'));
  echo(chalk.gray('  Use manage_authorized_creators to add more pool creators\n'));

} catch (error) {
  // Handle errors
  const errorMessage = error.stderr || error.message || error.toString();
  
  // Log full error for debugging
  if (process.env.DEBUG) {
    console.error('Full error:', error);
  }
  
  // Check if it's an "already initialized" error
  if (
    errorMessage.includes('already in use') ||
    errorMessage.includes('must be empty') ||
    errorMessage.includes('AccountNotEmpty') ||
    errorMessage.includes('Transaction simulation failed')
  ) {
    echo(chalk.yellow('\n⚠️  ProgramAuthority already initialized\n'));
    echo(chalk.gray('  This is expected if you have run this before.\n'));
    echo(chalk.gray(`  PDA: ${programAuthorityPda}\n`));
  } else if (error.message) {
    echo(chalk.red(`\n❌ Error: ${error.message}\n`));
    process.exit(1);
  } else {
    echo(chalk.red('\n❌ Initialization failed!\n'));
    echo(chalk.red(errorMessage));
    echo('');
    process.exit(1);
  }
}

echo(chalk.green('✓ Program authority initialization complete!\n'));
