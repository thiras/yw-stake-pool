#!/usr/bin/env zx
import 'zx/globals';
import {
  cliArguments,
  getProgramFolders,
  getCargo,
  getKeypairPath,
} from '../utils.mjs';

/**
 * Accept Pool Authority Script
 *
 * Accepts a pending pool authority transfer. This is step 2 of the two-step
 * authority transfer process. Must be run by the nominated new authority.
 *
 * Usage: pnpm programs:pool:accept-authority --pool <POOL_ADDRESS> [OPTIONS]
 */

// Get CLI arguments
const args = cliArguments();

// Show usage if help flag
if (args.includes('--help') || args.includes('-h')) {
  console.log(`
${chalk.blue('Accept Stake Pool Authority Transfer')}
${chalk.gray('=').repeat(60)}

Accepts a pending pool authority transfer (Step 2 of 2).
Must be run by the nominated new authority.

${chalk.yellow('Usage:')}
  pnpm programs:pool:accept-authority --pool <POOL_ADDRESS> [OPTIONS]

${chalk.yellow('Required Options:')}
  ${chalk.cyan('--pool <ADDRESS>')}           Pool address where authority transfer is pending

${chalk.yellow('Other Options:')}
  ${chalk.cyan('--cluster <NAME>')}           Cluster: devnet, testnet, mainnet-beta
                                 (default: devnet)
  ${chalk.cyan('--keypair <PATH>')}           Path to new authority keypair (pending authority)
                                 (default: ~/.config/solana/id.json)
  ${chalk.cyan('--program-id <ADDRESS>')}     Program ID (default: auto-detected from repo)

${chalk.yellow('Examples:')}
  ${chalk.gray('# Accept authority transfer on devnet')}
  pnpm programs:pool:accept-authority \\
    --pool FeAp4E2uWRNuDVQL26cZSkHJKjaDukbXg4coia91ATXo

  ${chalk.gray('# Accept on mainnet with custom keypair')}
  pnpm programs:pool:accept-authority \\
    --pool FeAp4E2uWRNuDVQL26cZSkHJKjaDukbXg4coia91ATXo \\
    --cluster mainnet-beta \\
    --keypair /path/to/new-authority.json

${chalk.yellow('Prerequisites:')}
  ${chalk.gray('1. Current authority must have nominated you first')}
  ${chalk.gray('2. You must use the keypair that was nominated')}
  ${chalk.gray('3. The nomination must still be pending (not cancelled)')}

${chalk.yellow('After Acceptance:')}
  ${chalk.gray('✓ You become the new pool authority')}
  ${chalk.gray('✓ You can update pool parameters')}
  ${chalk.gray('✓ You can fund rewards, pause/unpause, etc.')}
  ${chalk.gray('✓ Previous authority loses all control')}

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

const poolAddress = getOption('pool', 'p');
const cluster = getOption('cluster', 'u') || 'devnet';
const keypairPath = await getKeypairPath(getOption('keypair', 'k'));
let programId = getOption('program-id');

echo(chalk.blue('\n' + '='.repeat(60)));
echo(chalk.blue('  Accept Stake Pool Authority Transfer'));
echo(chalk.blue('='.repeat(60) + '\n'));

// Validate required pool address
if (!poolAddress) {
  echo(chalk.red('❌ Error: --pool <ADDRESS> is required\n'));
  echo(
    chalk.gray(
      'Usage: pnpm programs:pool:accept-authority --pool <POOL_ADDRESS>\n'
    )
  );
  process.exit(1);
}

// Check if keypair exists
try {
  await fs.access(keypairPath);
} catch (error) {
  echo(chalk.red(`❌ Error: Keypair not found at ${keypairPath}\n`));
  process.exit(1);
}

// Get pending authority public key (must match nominated address)
echo(chalk.cyan('Loading pending authority keypair...'));
let pendingAuthority;
try {
  const result = await $`solana-keygen pubkey ${keypairPath}`;
  pendingAuthority = result.stdout.trim();
  echo(chalk.green(`✓ Pending Authority: ${pendingAuthority}\n`));
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
    // Try to find program keypair - check program folder first, then target/deploy
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
echo(chalk.gray(`  Pool Address: ${poolAddress}`));
echo(chalk.gray(`  Program ID: ${programId}`));
echo(chalk.gray(`  Pending Authority: ${pendingAuthority}`));
echo(chalk.gray(`  Cluster: ${cluster}`));
echo(chalk.gray(`  Keypair: ${keypairPath}\n`));

// Display transaction info
echo(chalk.blue('Transaction Details:'));
echo(chalk.gray('  Instruction: AcceptAuthority'));
echo(chalk.gray(`  Pool: ${poolAddress}`));
echo(chalk.gray(`  Pending Authority (signer): ${pendingAuthority}\n`));

echo(chalk.yellow('⚠️  Important:'));
echo(chalk.gray('  - You must be the nominated authority'));
echo(chalk.gray('  - This completes the transfer immediately'));
echo(chalk.gray('  - Previous authority loses control'));
echo(chalk.gray('  - You gain full pool authority\n'));

// Implementation instructions
echo(chalk.blue('Implementation Options:\n'));

echo(chalk.cyan('Option 1: Use TypeScript Example'));
echo(chalk.gray('  cd example'));
echo(chalk.gray('  # Create a script based on pool-admin.ts'));
echo(chalk.gray('  # Import getAcceptAuthorityInstruction'));
echo(chalk.gray('  # Build and send transaction\n'));

echo(chalk.cyan('Option 2: Use JavaScript Client'));
echo(
  chalk.gray(
    '  import { getAcceptAuthorityInstruction } from "@yourwallet/stake-pool";'
  )
);
echo(chalk.gray('  '));
echo(chalk.gray('  const acceptIx = getAcceptAuthorityInstruction({'));
echo(chalk.gray(`    pool: address("${poolAddress}"),`));
echo(chalk.gray('    pendingAuthority: pendingAuthoritySigner,'));
echo(chalk.gray('  });'));
echo(chalk.gray('  '));
echo(chalk.gray('  // Build and send transaction with acceptIx\n'));

echo(chalk.cyan('Option 3: Frontend Integration'));
echo(chalk.gray('  Integrate the instruction into your web application'));
echo(chalk.gray('  Use wallet adapters to sign with pending authority\n'));

// After acceptance
echo(chalk.blue('After Acceptance:'));
echo(chalk.gray('  ✓ You become the new pool authority'));
echo(chalk.gray('  ✓ Update parameters: pnpm example:pool-admin'));
echo(chalk.gray('  ✓ Fund rewards, pause/unpause pool'));
echo(chalk.gray('  ✓ Transfer authority again if needed\n'));

echo(chalk.green('✓ Configuration validated!\n'));

echo(chalk.yellow('📚 For implementation details, see:'));
echo(chalk.gray('  - example/src/pool-admin.ts (authority transfer example)'));
echo(chalk.gray('  - example/README.md (documentation)'));
echo(chalk.gray('  - clients/js/README.md (client library docs)\n'));
