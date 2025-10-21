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
const clusterIndex = args.findIndex((arg) => arg === '--cluster' || arg === '-u');
let cluster = 'devnet';
if (clusterIndex >= 0 && args[clusterIndex + 1]) {
  cluster = args[clusterIndex + 1];
  args.splice(clusterIndex, 2);
}

// Check for keypair argument
const keypairIndex = args.findIndex((arg) => arg === '--keypair' || arg === '-k');
let keypairPath = null;
if (keypairIndex >= 0 && args[keypairIndex + 1]) {
  keypairPath = args[keypairIndex + 1];
  args.splice(keypairIndex, 2);
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
  if (keypairPath) {
    echo(chalk.gray(`Using keypair: ${keypairPath}`));
  }

  try {
    const deployArgs = keypairPath ? ['--keypair', keypairPath, ...args] : args;
    await $`solana program deploy ${soPath} --url ${cluster} ${deployArgs}`;
    echo(chalk.green(`✓ Successfully deployed ${packageName} to ${cluster}`));
  } catch (error) {
    echo(chalk.red(`✗ Failed to deploy ${packageName}`));
    throw error;
  }
}

echo(chalk.green('\n✓ All programs deployed successfully!'));
