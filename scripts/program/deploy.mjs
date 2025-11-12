#!/usr/bin/env zx
import 'zx/globals';
import {
  cliArguments,
  getProgramFolders,
  workingDirectory,
  popArgument,
  getCargo,
} from '../utils.mjs';

// Get the CLI arguments
const args = cliArguments();

// Check for cluster/network argument
const clusterIndex = args.findIndex(
  (arg) => arg === '--cluster' || arg === '-u'
);
let cluster = 'devnet';
if (clusterIndex >= 0 && args[clusterIndex + 1]) {
  cluster = args[clusterIndex + 1];
  args.splice(clusterIndex, 2);
}

// Check for keypair argument (upgrade authority)
const keypairIndex = args.findIndex(
  (arg) => arg === '--keypair' || arg === '-k'
);
let keypairPath = null;
if (keypairIndex >= 0 && args[keypairIndex + 1]) {
  keypairPath = args[keypairIndex + 1];
  args.splice(keypairIndex, 2);
}

// Check for program-id argument (to use existing deployed program)
const programIdIndex = args.findIndex((arg) => arg === '--program-id');
let existingProgramId = null;
if (programIdIndex >= 0 && args[programIdIndex + 1]) {
  existingProgramId = args[programIdIndex + 1];
  args.splice(programIdIndex, 2);
}

// Build the programs first if --skip-build is not provided
const skipBuild = popArgument(args, '--skip-build');
if (!skipBuild) {
  echo(chalk.blue('Building programs...'));
  await $`pnpm programs:build`;
}

// Deploy the programs
for (const folder of getProgramFolders()) {
  // Get the actual package name from Cargo.toml
  const cargo = getCargo(folder);
  const packageName = cargo.package.name;
  const programName = packageName.replace(/-/g, '_'); // Convert hyphens to underscores for .so filename

  const soPath = path.join(
    workingDirectory,
    'target',
    'deploy',
    `${programName}.so`
  );

  echo(chalk.blue(`\nDeploying ${packageName} to ${cluster}...`));
  echo(chalk.gray(`Binary: ${soPath}`));

  // Determine program ID to use
  let programId = existingProgramId;
  let programKeypair = null;

  if (!programId) {
    // Try to find program keypair - check program folder first, then target/deploy
    const programKeypairPath = path.join(folder, 'keypair.json');
    const targetKeypairPath = path.join(
      workingDirectory,
      'target',
      'deploy',
      `${programName}-keypair.json`
    );

    if (await fs.pathExists(programKeypairPath)) {
      programKeypair = programKeypairPath;
    } else if (await fs.pathExists(targetKeypairPath)) {
      programKeypair = targetKeypairPath;
    }

    // Get program ID from keypair if found
    if (programKeypair) {
      try {
        const result = await $`solana-keygen pubkey ${programKeypair}`;
        programId = result.stdout.trim();
        echo(chalk.gray(`Program ID: ${programId} (from keypair)`));
        echo(chalk.gray(`Program Keypair: ${programKeypair}`));
      } catch (error) {
        echo(
          chalk.yellow(
            `⚠️  Could not derive program ID from keypair: ${error.message}`
          )
        );
      }
    }
  } else {
    echo(chalk.gray(`Program ID: ${programId} (provided)`));
  }

  if (!programId && !programKeypair) {
    echo(
      chalk.yellow(
        `⚠️  No program keypair found - Solana will generate a new program ID`
      )
    );
  }

  if (keypairPath) {
    echo(chalk.gray(`Upgrade Authority: ${keypairPath}`));
  } else {
    echo(chalk.gray(`Upgrade Authority: <default Solana keypair>`));
  }

  try {
    const deployArgs = [];

    // Add program ID/keypair if available
    if (programKeypair) {
      deployArgs.push('--program-id', programKeypair);
    } else if (programId) {
      deployArgs.push('--program-id', programId);
    }

    // Add upgrade authority keypair if provided
    if (keypairPath) {
      deployArgs.push('--upgrade-authority', keypairPath);
    }

    // Add any additional args
    deployArgs.push(...args);

    await $`solana program deploy ${soPath} --url ${cluster} ${deployArgs}`;
    echo(chalk.green(`✓ Successfully deployed ${packageName} to ${cluster}`));

    // Try to get the actual program ID after deployment
    if (!programId) {
      try {
        const showResult =
          await $`solana program show ${soPath} --url ${cluster}`;
        const programIdMatch = showResult.stdout.match(/Program Id: (\w+)/);
        if (programIdMatch) {
          echo(chalk.green(`  Program ID: ${programIdMatch[1]}`));
        }
      } catch (error) {
        // Ignore - just means we can't fetch the ID
      }
    } else {
      echo(chalk.green(`  Program ID: ${programId}`));
    }
  } catch (error) {
    echo(chalk.red(`✗ Failed to deploy ${packageName}`));
    throw error;
  }
}

echo(chalk.green('\n✓ All programs deployed successfully!'));

// Provide helpful next steps
echo(chalk.blue('\n' + '═'.repeat(60)));
echo(chalk.blue('  Next Steps'));
echo(chalk.blue('═'.repeat(60) + '\n'));

echo(chalk.yellow('📋 Important: Initialize Program Authority'));
echo(chalk.gray('   Before creating any stake pools, you must initialize the'));
echo(chalk.gray('   ProgramAuthority account (one-time setup):\n'));
echo(chalk.cyan('   pnpm programs:init-authority'));
if (cluster !== 'devnet') {
  echo(chalk.cyan(`   pnpm programs:init-authority --cluster ${cluster}`));
}
echo('');

echo(chalk.yellow('📋 Verify Deployment:'));
echo(chalk.gray('   Check the program on Solana Explorer or use:\n'));
for (const folder of getProgramFolders()) {
  const cargo = getCargo(folder);
  const packageName = cargo.package.name;
  const programName = packageName.replace(/-/g, '_');
  
  let programId = existingProgramId;
  if (!programId) {
    const programKeypairPath = path.join(folder, 'keypair.json');
    const targetKeypairPath = path.join(
      workingDirectory,
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
      } catch (error) {
        // Ignore
      }
    }
  }
  
  if (programId) {
    echo(chalk.cyan(`   solana program show ${programId} --url ${cluster}`));
  }
}
echo(chalk.blue('\n' + '═'.repeat(60) + '\n'));
