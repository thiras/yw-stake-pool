#!/usr/bin/env zx
import 'zx/globals';
import {
  cliArguments,
  getProgramFolders,
  getToolchainArgument,
  partitionArguments,
  popArgument,
  workingDirectory,
} from '../utils.mjs';

// Configure additional arguments here, e.g.:
// ['--arg1', '--arg2', ...cliArguments()]
let formatArgs = cliArguments();

const fix = popArgument(formatArgs, '--fix');

// Filter out file paths that may be passed by lint-staged
// cargo fmt doesn't accept individual file paths
formatArgs = formatArgs.filter(arg => !arg.endsWith('.rs') && !arg.includes('/'));

const [cargoArgs, fmtArgs] = partitionArguments(formatArgs, '--');
const toolchain = getToolchainArgument('format');

// Format the programs.
for (const folder of getProgramFolders()) {
  const manifestPath = path.join(workingDirectory, folder, 'Cargo.toml');

  if (fix) {
    await $`cargo ${toolchain} fmt --manifest-path ${manifestPath} ${cargoArgs} -- ${fmtArgs}`;
  } else {
    await $`cargo ${toolchain} fmt --manifest-path ${manifestPath} ${cargoArgs} -- --check ${fmtArgs}`;
  }
}
