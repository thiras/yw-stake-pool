#!/usr/bin/env zx
import 'zx/globals';
import {
  cliArguments,
  getProgramFolders,
  getToolchainArgument,
  popArgument,
  workingDirectory,
} from '../utils.mjs';

// Configure additional arguments here, e.g.:
// ['--arg1', '--arg2', ...cliArguments()]
let lintArgs = cliArguments();

const fix = popArgument(lintArgs, '--fix');
const toolchain = getToolchainArgument('lint');

// Filter out file paths that may be passed by lint-staged
// cargo clippy doesn't accept individual file paths
lintArgs = lintArgs.filter((arg) => !arg.endsWith('.rs') && !arg.includes('/'));

// Lint the programs using clippy.
for (const folder of getProgramFolders()) {
  const manifestPath = path.join(workingDirectory, folder, 'Cargo.toml');

  if (fix) {
    await $`cargo ${toolchain} clippy --manifest-path ${manifestPath} --fix ${lintArgs}`;
  } else {
    await $`cargo ${toolchain} clippy --manifest-path ${manifestPath} ${lintArgs}`;
  }
}
