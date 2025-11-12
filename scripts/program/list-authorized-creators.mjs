#!/usr/bin/env zx
import 'zx/globals';
import {
  cliArguments,
  getProgramFolders,
  getCargo,
} from '../utils.mjs';
import {
  calculateProgramAuthorityPda,
  getClusterUrl,
  checkProgramAuthorityExists,
} from '../lib/program-authority.mjs';

// Import @solana/kit directly from root node_modules
import {
  address,
  createSolanaRpc,
} from '@solana/kit';

/**
 * List Authorized Pool Creators
 *
 * Displays the program authority and all authorized creators who can create stake pools.
 *
 * Usage: pnpm programs:list-creators [OPTIONS]
 */

// Get CLI arguments
const args = cliArguments();

// Show usage if help flag
if (args.includes('--help') || args.includes('-h')) {
  console.log(`
${chalk.blue('List Authorized Pool Creators')}
${chalk.gray('=').repeat(60)}

Displays all addresses authorized to create stake pools, including
the main program authority and any additional authorized creators.

${chalk.yellow('Usage:')}
  pnpm programs:list-creators [OPTIONS]

${chalk.yellow('Options:')}
  ${chalk.cyan('--cluster <NAME>')}       Cluster: devnet, testnet, mainnet-beta
                             (default: devnet)
  ${chalk.cyan('--program-id <ID>')}      Program ID (auto-detected if not provided)
  ${chalk.cyan('--json')}                 Output in JSON format

${chalk.yellow('Examples:')}
  ${chalk.gray('# List creators on devnet')}
  pnpm programs:list-creators

  ${chalk.gray('# List creators on mainnet')}
  pnpm programs:list-creators --cluster mainnet-beta

  ${chalk.gray('# List creators with specific program ID')}
  pnpm programs:list-creators \\
    --program-id 8PtjrGvKNeZt2vCmRkSPGjss7TAFhvxux2N8r67UMKBx

  ${chalk.gray('# Output as JSON')}
  pnpm programs:list-creators --json

${chalk.yellow('Related Commands:')}
  ${chalk.cyan('pnpm programs:add-creator <KEY>')}       Add authorized creator
  ${chalk.cyan('pnpm programs:remove-creator <KEY>')}    Remove authorized creator
  ${chalk.cyan('pnpm programs:init-authority')}          Initialize program authority

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

const providedProgramId = getOption('program-id', 'p');
const cluster = getOption('cluster', 'u') || 'devnet';
const jsonOutput = args.includes('--json');

if (!jsonOutput) {
  echo(chalk.blue('\n' + '='.repeat(60)));
  echo(chalk.blue('  Authorized Pool Creators'));
  echo(chalk.blue('='.repeat(60) + '\n'));
}

// Helper: Auto-detect program ID from repository
async function detectProgramId() {
  if (!jsonOutput) {
    echo(chalk.cyan('Determining program ID...'));
  }
  
  const folders = getProgramFolders();

  if (folders.length === 0) {
    throw new Error('No program folders found');
  }

  if (folders.length > 1 && !jsonOutput) {
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
  
  if (!jsonOutput) {
    echo(chalk.green(`✓ Program ID: ${programId}`));
    echo(chalk.gray(`  (from ${programKeypair})`));
  }
  
  return programId;
}

// Main execution flow
let programId;
let programAuthorityPda;

try {
  // 1. Detect or use provided program ID
  programId = providedProgramId || await detectProgramId();

  // 2. Calculate ProgramAuthority PDA
  if (!jsonOutput) {
    echo(chalk.cyan('\nCalculating ProgramAuthority PDA...'));
  }
  
  programAuthorityPda = await calculateProgramAuthorityPda(programId);
  
  if (!jsonOutput) {
    echo(chalk.green(`✓ ProgramAuthority PDA: ${programAuthorityPda}`));
  }

  // 3. Check if ProgramAuthority exists
  if (!jsonOutput) {
    echo(chalk.cyan('\nVerifying ProgramAuthority account...'));
  }
  
  const exists = await checkProgramAuthorityExists(programAuthorityPda, cluster);
  
  if (!exists) {
    if (jsonOutput) {
      console.log(JSON.stringify({ error: 'ProgramAuthority not initialized' }, null, 2));
    } else {
      echo(chalk.red('\n❌ ProgramAuthority not initialized!'));
      echo(chalk.yellow('\nPlease run the following command first:'));
      echo(chalk.cyan('  pnpm programs:init-authority\n'));
    }
    process.exit(1);
  }
  
  if (!jsonOutput) {
    echo(chalk.green('✓ ProgramAuthority account exists'));
  }

  // 4. Fetch and decode account data using the client library
  if (!jsonOutput) {
    echo(chalk.cyan('\nFetching account data...\n'));
  }

  // Load the JavaScript client
  const clientPath = path.join(process.cwd(), 'clients/js/dist/src/index.js');
  const { fetchProgramAuthority } = await import(clientPath);

  const clusterUrl = getClusterUrl(cluster);
  const rpc = createSolanaRpc(clusterUrl);

  // Use the generated fetch helper
  const programAuthorityAccount = await fetchProgramAuthority(rpc, address(programAuthorityPda));
  const programAuthority = programAuthorityAccount.data;

  // Extract authority as string
  const authorityPubkey = programAuthority.authority;

  // Parse authorized creators - they're already decoded as Option<Address> types
  const authorizedCreators = [];
  for (const creator of programAuthority.authorizedCreators) {
    if (creator.__option === 'Some') {
      authorizedCreators.push(creator.value);
    }
  }

  // 5. Display results
  if (jsonOutput) {
    const output = {
      programId,
      programAuthorityPda,
      cluster,
      authority: authorityPubkey,
      authorizedCreators,
      totalCreators: authorizedCreators.length,
      maxCreators: 10,
    };
    console.log(JSON.stringify(output, null, 2));
  } else {
    echo(chalk.yellow('Program Authority Details:'));
    echo(chalk.gray(`  PDA: ${programAuthorityPda}`));
    echo(chalk.gray(`  Main Authority: ${authorityPubkey}`));
    echo(chalk.gray(`  Cluster: ${cluster}`));
    echo(chalk.gray(`  Program ID: ${programId}\n`));

    echo(chalk.yellow('Authorized Creators:'));
    echo(chalk.green(`  Main Authority: ${authorityPubkey}`));
    echo(chalk.gray('    (always authorized)\n'));

    if (authorizedCreators.length > 0) {
      echo(chalk.cyan(`  Additional Creators (${authorizedCreators.length}/10):`));
      authorizedCreators.forEach((creator, index) => {
        echo(chalk.gray(`    ${index + 1}. ${creator}`));
      });
      echo('');
    } else {
      echo(chalk.gray('  No additional creators authorized\n'));
    }

    echo(chalk.yellow('Summary:'));
    echo(chalk.gray(`  Total authorized: ${authorizedCreators.length + 1} (including main authority)`));
    echo(chalk.gray(`  Remaining slots: ${10 - authorizedCreators.length}/10\n`));
  }

} catch (error) {
  if (jsonOutput) {
    console.error(JSON.stringify({ error: error.message || error.toString() }, null, 2));
  } else {
    echo(chalk.red('\n❌ Failed to list creators!\n'));
    echo(chalk.red(error.message || error.toString()));
    echo('');
  }
  process.exit(1);
}

if (!jsonOutput) {
  echo(chalk.green('✓ List creators complete!\n'));
}
