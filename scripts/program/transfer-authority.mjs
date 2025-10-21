#!/usr/bin/env zx
import 'zx/globals';
import { cliArguments, getProgramFolders, getCargo } from '../utils.mjs';

/**
 * Transfer Program Upgrade Authority
 *
 * Uses Solana CLI to transfer the upgrade authority of a deployed program
 * to a new public key. This is a direct one-step transfer.
 *
 * Usage: pnpm programs:transfer-authority <NEW_AUTHORITY_PUBLIC_KEY> [OPTIONS]
 */

// Get CLI arguments
const args = cliArguments();

// Show usage if no arguments or help flag
if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
  console.log(`
${chalk.blue('Transfer Program Upgrade Authority')}
${chalk.gray('=').repeat(60)}

Transfers the upgrade authority of a Solana program to a new address.
Uses Solana CLI's direct authority transfer (one-step process).

${chalk.yellow('Usage:')}
  pnpm programs:transfer-authority <NEW_AUTHORITY_PUBLIC_KEY> [OPTIONS]

${chalk.yellow('Arguments:')}
  ${chalk.cyan('<NEW_AUTHORITY_PUBLIC_KEY>')}  The public key of the new upgrade authority

${chalk.yellow('Options:')}
  ${chalk.cyan('--program-id <ADDRESS>')}     Program ID to transfer (defaults to deployed program)
  ${chalk.cyan('--cluster <NAME>')}           Cluster: devnet, testnet, mainnet-beta
                                 (default: devnet)
  ${chalk.cyan('--keypair <PATH>')}           Path to current upgrade authority keypair
                                 (default: ~/.config/solana/id.json)

${chalk.yellow('Examples:')}
  ${chalk.gray('# Transfer to a new authority on devnet')}
  pnpm programs:transfer-authority 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd

  ${chalk.gray('# Transfer specific program on mainnet')}
  pnpm programs:transfer-authority 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd \\
    --program-id 8PtjrGvKNeZt2vCmRkSPGjss7TAFhvxux2N8r67UMKBx \\
    --cluster mainnet-beta

  ${chalk.gray('# Transfer using custom authority keypair')}
  pnpm programs:transfer-authority 6tuAX4SL4bXdiFaMKqaVBkGG1vZnUnhVippuVdEUGyJd \\
    --keypair /path/to/current-authority.json

  ${chalk.gray('# Make program immutable (no upgrade authority)')}
  pnpm programs:transfer-authority --none

${chalk.yellow('Important Notes:')}
  ${chalk.red('⚠️  This is a ONE-STEP, IRREVERSIBLE process!')}
  ${chalk.gray('  - The current authority immediately loses control')}
  ${chalk.gray('  - The new authority gains immediate control')}
  ${chalk.gray('  - Use --none to make the program immutable (cannot be upgraded)')}
  ${chalk.gray('  - Double-check addresses before confirming')}

${chalk.gray('=').repeat(60)}
`);
  process.exit(args.length === 0 ? 1 : 0);
}

// Parse arguments
const newAuthorityKey = args[0];
const makeImmutable = newAuthorityKey === '--none';

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
const keypairPath =
  getOption('keypair', 'k') ||
  path.join(os.homedir(), '.config', 'solana', 'id.json');

echo(chalk.blue('\n' + '='.repeat(60)));
echo(chalk.blue('  Transfer Program Upgrade Authority'));
echo(chalk.blue('='.repeat(60) + '\n'));

// Validate new authority if not making immutable
if (!makeImmutable && !newAuthorityKey) {
  echo(chalk.red('❌ Error: New authority public key is required\n'));
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
echo(chalk.cyan('Loading current upgrade authority...'));
let currentAuthority;
try {
  const result = await $`solana-keygen pubkey ${keypairPath}`;
  currentAuthority = result.stdout.trim();
  echo(chalk.green(`✓ Current Authority: ${currentAuthority}\n`));
} catch (error) {
  echo(chalk.red('❌ Failed to read keypair\n'));
  process.exit(1);
}

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
  const cargo = getCargo(folder);
  const programName = cargo.package.name.replace(/-/g, '_');

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
      echo(chalk.green(`✓ Found program keypair: ${programKeypair}`));
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

// Display transfer details
echo(chalk.yellow('Transfer Details:'));
echo(chalk.gray(`  Program ID: ${programId}`));
echo(chalk.gray(`  Cluster: ${cluster}`));
echo(chalk.gray(`  Current Authority: ${currentAuthority}`));
if (makeImmutable) {
  echo(chalk.gray(`  New Authority: ${chalk.red('NONE (immutable)')}`));
} else {
  echo(chalk.gray(`  New Authority: ${newAuthorityKey}`));
}
echo(chalk.gray(`  Authority Keypair: ${keypairPath}\n`));

// Confirmation prompt
echo(chalk.red('⚠️  WARNING: This action is IMMEDIATE and IRREVERSIBLE!\n'));

if (makeImmutable) {
  echo(chalk.red('Making the program IMMUTABLE means:'));
  echo(chalk.gray('  ✗ No one can upgrade the program ever again'));
  echo(chalk.gray('  ✗ Bugs cannot be fixed'));
  echo(chalk.gray('  ✗ Features cannot be added'));
  echo(chalk.gray('  ✗ This action CANNOT be undone\n'));
} else {
  echo(chalk.yellow('After this transfer:'));
  echo(chalk.gray(`  ✗ ${currentAuthority} will lose upgrade authority`));
  echo(chalk.gray(`  ✓ ${newAuthorityKey} will gain immediate control`));
  echo(chalk.gray('  ℹ️  Double-check the new authority address!\n'));
}

// Ask for confirmation
const confirmText = makeImmutable ? 'make-immutable' : 'transfer';
echo(chalk.cyan(`Type "${confirmText}" to confirm:`));

let confirmed = false;
try {
  const response = await question('> ');
  confirmed = response.trim() === confirmText;
} catch (error) {
  echo(chalk.red('\n❌ Cancelled\n'));
  process.exit(0);
}

if (!confirmed) {
  echo(chalk.red('\n❌ Confirmation text did not match. Cancelled.\n'));
  process.exit(0);
}

// Execute the transfer
echo(chalk.cyan('\n🔄 Transferring upgrade authority...\n'));

try {
  const transferArgs = ['program', 'set-upgrade-authority', programId];

  if (makeImmutable) {
    transferArgs.push('--final');
  } else {
    transferArgs.push('--new-upgrade-authority', newAuthorityKey);
  }

  transferArgs.push('--url', cluster, '--keypair', keypairPath);

  await $`solana ${transferArgs}`;

  echo(chalk.green('\n✅ Transfer successful!\n'));

  if (makeImmutable) {
    echo(chalk.yellow('Program is now IMMUTABLE:'));
    echo(chalk.gray(`  Program ID: ${programId}`));
    echo(chalk.gray('  Upgrade Authority: None'));
    echo(chalk.gray('  Status: Cannot be upgraded\n'));
  } else {
    echo(chalk.green('New authority details:'));
    echo(chalk.gray(`  Program ID: ${programId}`));
    echo(chalk.gray(`  Previous Authority: ${currentAuthority}`));
    echo(chalk.gray(`  New Authority: ${newAuthorityKey}`));
    echo(chalk.gray(`  Cluster: ${cluster}\n`));
  }

  // Show how to verify
  echo(chalk.cyan('Verify the transfer:'));
  echo(chalk.gray(`  solana program show ${programId} --url ${cluster}\n`));
} catch (error) {
  echo(chalk.red('\n❌ Transfer failed!\n'));
  echo(chalk.red(error.stderr || error.message));
  process.exit(1);
}

echo(chalk.green('✓ Authority transfer complete!\n'));
