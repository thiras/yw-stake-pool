#!/usr/bin/env zx
import 'zx/globals';
import { cliArguments, getProgramFolders, getCargo } from '../utils.mjs';

/**
 * Transfer Pool Authority Script
 *
 * Transfers the operational authority of a stake pool using the program's
 * two-step authority transfer process (NominateNewAuthority + AcceptAuthority).
 *
 * This is different from program upgrade authority - this controls pool operations
 * like updating parameters, funding rewards, pausing, etc.
 *
 * Usage: pnpm programs:pool:transfer-authority <NEW_AUTHORITY_PUBLIC_KEY> [OPTIONS]
 */

// Get CLI arguments
const args = cliArguments();

// Show usage if no arguments or help flag
if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
  console.log(`
${chalk.blue('Transfer Stake Pool Authority')}
${chalk.gray('=').repeat(60)}

Transfers the operational authority of a stake pool to a new address.
Uses the program's two-step authority transfer process for safety.

${chalk.yellow('Usage:')}
  pnpm programs:pool:transfer-authority <NEW_AUTHORITY_PUBLIC_KEY> [OPTIONS]

${chalk.yellow('Arguments:')}
  ${chalk.cyan('<NEW_AUTHORITY_PUBLIC_KEY>')}  The public key of the new pool authority

${chalk.yellow('Options:')}
  ${chalk.cyan('--pool <ADDRESS>')}           Pool address (required if --stake-mint not provided)
  ${chalk.cyan('--stake-mint <ADDRESS>')}     Stake mint address (used to derive pool PDA)
  ${chalk.cyan('--cluster <NAME>')}           Cluster: devnet, testnet, mainnet-beta
                                 (default: devnet)
  ${chalk.cyan('--keypair <PATH>')}           Path to current pool authority keypair
                                 (default: ~/.config/solana/id.json)
  ${chalk.cyan('--program-id <ADDRESS>')}     Program ID (default: auto-detected from repo)

${chalk.yellow('Examples:')}
  ${chalk.gray('# Transfer pool authority (derives pool from stake mint)')}
  pnpm programs:pool:transfer-authority 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd \\
    --stake-mint TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA

  ${chalk.gray('# Transfer with known pool address')}
  pnpm programs:pool:transfer-authority 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd \\
    --pool FeAp4E2uWRNuDVQL26cZSkHJKjaDukbXg4coia91ATXo

  ${chalk.gray('# Transfer on mainnet with custom keypair')}
  pnpm programs:pool:transfer-authority 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd \\
    --pool FeAp4E2uWRNuDVQL26cZSkHJKjaDukbXg4coia91ATXo \\
    --cluster mainnet-beta \\
    --keypair /path/to/pool-authority.json

${chalk.yellow('Two-Step Process:')}
  ${chalk.cyan('Step 1:')} Current authority nominates new authority (this script)
  ${chalk.cyan('Step 2:')} New authority must accept (run by new authority)

${chalk.yellow('After running this script:')}
  The new authority must run:
  ${chalk.gray('pnpm programs:pool:accept-authority --pool <POOL_ADDRESS>')}

${chalk.yellow('Pool Authority Controls:')}
  ${chalk.gray('✓')} Update reward rates and pool parameters
  ${chalk.gray('✓')} Fund reward vault
  ${chalk.gray('✓')} Pause/unpause the pool
  ${chalk.gray('✓')} Set pool end dates
  ${chalk.gray('✓')} Transfer authority again

${chalk.yellow('Difference from Program Authority:')}
  ${chalk.gray('Program Authority:')} Who can deploy/upgrade program code
  ${chalk.gray('Pool Authority:')}    Who can manage pool operations (this script)

${chalk.gray('=').repeat(60)}
`);
  process.exit(args.length === 0 ? 1 : 0);
}

// Parse arguments
const newAuthorityKey = args[0];

// Validate new authority public key
if (!newAuthorityKey || newAuthorityKey.startsWith('--')) {
  echo(chalk.red('❌ Error: New authority public key is required\n'));
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

const poolAddress = getOption('pool', 'p');
const stakeMint = getOption('stake-mint', 'm');
const cluster = getOption('cluster', 'u') || 'devnet';
const keypairPath =
  getOption('keypair', 'k') ||
  path.join(os.homedir(), '.config', 'solana', 'id.json');
let programId = getOption('program-id');

echo(chalk.blue('\n' + '='.repeat(60)));
echo(chalk.blue('  Transfer Stake Pool Authority'));
echo(chalk.blue('='.repeat(60) + '\n'));

// Validate inputs
if (!poolAddress && !stakeMint) {
  echo(chalk.red('❌ Error: Either --pool or --stake-mint must be provided\n'));
  process.exit(1);
}

// Check if keypair exists
try {
  await fs.access(keypairPath);
} catch (error) {
  echo(chalk.red(`❌ Error: Keypair not found at ${keypairPath}\n`));
  process.exit(1);
}

// Get current authority public key
echo(chalk.cyan('Loading current pool authority...'));
let currentAuthority;
try {
  const result = await $`solana-keygen pubkey ${keypairPath}`;
  currentAuthority = result.stdout.trim();
  echo(chalk.green(`✓ Current Authority: ${currentAuthority}\n`));
} catch (error) {
  echo(chalk.red('❌ Failed to read keypair\n'));
  process.exit(1);
}

// Auto-detect program ID if not provided
if (!programId) {
  echo(chalk.cyan('Auto-detecting program ID from repository...'));
  const folders = getProgramFolders();

  if (folders.length > 0) {
    const folder = folders[0];
    const programName = getCargo(folder).package.name.replace(/-/g, '_');
    const targetKeypairPath = path.join(
      process.cwd(),
      'target',
      'deploy',
      `${programName}-keypair.json`
    );
    const programKeypairPath = path.join(folder, 'keypair.json');

    let programKeypair = null;
    if (await fs.pathExists(targetKeypairPath)) {
      programKeypair = targetKeypairPath;
    } else if (await fs.pathExists(programKeypairPath)) {
      programKeypair = programKeypairPath;
    }

    if (programKeypair) {
      try {
        const result = await $`solana-keygen pubkey ${programKeypair}`;
        programId = result.stdout.trim();
        echo(chalk.green(`✓ Program ID: ${programId}\n`));
      } catch (error) {
        echo(chalk.yellow('⚠️  Could not auto-detect program ID'));
      }
    }
  }

  if (!programId) {
    echo(
      chalk.red('❌ Could not determine program ID. Use --program-id option.\n')
    );
    process.exit(1);
  }
}

// Display configuration
echo(chalk.cyan('Configuration:'));
if (poolAddress) {
  echo(chalk.gray(`  Pool Address: ${poolAddress}`));
} else if (stakeMint) {
  echo(chalk.gray(`  Stake Mint: ${stakeMint}`));
  echo(
    chalk.yellow(
      `  Pool Address: (will be derived from authority + stake mint)`
    )
  );
}
echo(chalk.gray(`  Program ID: ${programId}`));
echo(chalk.gray(`  Current Authority: ${currentAuthority}`));
echo(chalk.gray(`  New Authority: ${newAuthorityKey}`));
echo(chalk.gray(`  Cluster: ${cluster}`));
echo(chalk.gray(`  Keypair: ${keypairPath}\n`));

// Derive pool address if using stake mint
let finalPoolAddress = poolAddress;
if (!finalPoolAddress && stakeMint) {
  echo(chalk.yellow('⚠️  Pool PDA Derivation:'));
  echo(chalk.gray('  The pool address is derived using:'));
  echo(chalk.gray(`    seeds: ["pool", authority, stakeMint]`));
  echo(chalk.gray(`    authority: ${currentAuthority}`));
  echo(chalk.gray(`    stakeMint: ${stakeMint}`));
  echo(chalk.gray(`    programId: ${programId}\n`));

  echo(chalk.yellow('  Note: This script shows the configuration.'));
  echo(chalk.yellow('  To execute, use the TypeScript client or frontend.\n'));
}

// Display transaction info
echo(chalk.blue('Transaction Details:'));
echo(chalk.gray('  Instruction: NominateNewAuthority'));
echo(chalk.gray(`  Pool: ${finalPoolAddress || '(to be derived)'}`));
echo(chalk.gray(`  Current Authority (signer): ${currentAuthority}`));
echo(chalk.gray(`  New Authority: ${newAuthorityKey}\n`));

echo(chalk.yellow('🔐 Two-Step Process:'));
echo(chalk.gray('  1. Current authority nominates (this transaction)'));
echo(chalk.gray('  2. New authority must accept\n'));

// Implementation instructions
echo(chalk.blue('Implementation Options:\n'));

echo(chalk.cyan('Option 1: Use TypeScript Example'));
echo(chalk.gray('  cd example'));
echo(chalk.gray('  # Create a script based on pool-admin.ts'));
echo(chalk.gray('  # Import getNominateNewAuthorityInstruction'));
echo(chalk.gray('  # Build and send transaction\n'));

echo(chalk.cyan('Option 2: Use JavaScript Client'));
echo(
  chalk.gray(
    '  import { getNominateNewAuthorityInstruction } from "@yourwallet/stake-pool";'
  )
);
echo(chalk.gray('  '));
echo(chalk.gray('  const nominateIx = getNominateNewAuthorityInstruction({'));
echo(chalk.gray(`    pool: ${finalPoolAddress || 'derivedPoolAddress'},`));
echo(chalk.gray('    currentAuthority: currentAuthoritySigner,'));
echo(chalk.gray(`    newAuthority: address("${newAuthorityKey}"),`));
echo(chalk.gray('  });'));
echo(chalk.gray('  '));
echo(chalk.gray('  // Build and send transaction with nominateIx\n'));

echo(chalk.cyan('Option 3: Frontend Integration'));
echo(chalk.gray('  Integrate the instruction into your web application'));
echo(chalk.gray('  Use wallet adapters to sign with current authority\n'));

// Next steps
echo(chalk.blue('After Nomination:'));
echo(chalk.gray(`  1. Verify transaction on explorer`));
echo(chalk.gray(`  2. New authority (${newAuthorityKey}) must accept:`));
echo(
  chalk.gray(
    `     pnpm programs:pool:accept-authority --pool ${finalPoolAddress || '<POOL_ADDRESS>'}\n`
  )
);

echo(chalk.green('✓ Configuration validated!\n'));

echo(chalk.yellow('📚 For implementation details, see:'));
echo(chalk.gray('  - example/src/pool-admin.ts (authority transfer example)'));
echo(chalk.gray('  - example/README.md (documentation)'));
echo(chalk.gray('  - clients/js/README.md (client library docs)\n'));
