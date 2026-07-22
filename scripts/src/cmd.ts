import { Command } from 'commander';

import { setVersion } from './github/set-version';
import { syncChangelogLabels } from './github/sync-changelog-labels';
import { validatePrPolicy } from './github/validate-pr-policy';
import { buildPlayground } from './playground/build';
import { setupExtension } from './setup/extension';

const program = new Command();
const github = program
  .name('galfus-scripts')
  .description('Galfus repository automation commands')
  .command('github')
  .description('GitHub workflow commands');

const setup = program.command('setup').description('Local development setup commands');
const playground = program
  .command('playground')
  .description('Playground development and distribution commands');

github
  .command('set-version')
  .description('Apply an artifact version or derive it from a release branch')
  .option('-c, --component <component>', 'Artifact component: all, editor, or playground')
  .option('-t, --tag <tag>', 'Release channel tag')
  .option('-v, --version <version>', 'Semantic version')
  .action(setVersion);

github
  .command('sync-changelog-labels')
  .description('Synchronize changelog labels selected in a pull request')
  .action(syncChangelogLabels);

github
  .command('validate-pr-policy')
  .description('Validate pull request title, links, labels, and promotion rules')
  .action(validatePrPolicy);

setup
  .command('extension')
  .description('Install the local editor extension')
  .option('-v, --vscode', 'Install to VS Code and VS Code Insiders')
  .option('-a, --antigravity', 'Install to Antigravity IDE')
  .option('--all', 'Install to all editors (default)')
  .action(setupExtension);

playground
  .command('build')
  .description('Build the playground WebAssembly module and generate bindings')
  .option('-t, --target <target>', 'wasm-bindgen target (web, bundler, nodejs, etc)', 'web')
  .option('-o, --out-dir <path>', 'Output directory relative to the repository root')
  .action(buildPlayground);

program.parseAsync(process.argv).catch((error) => {
  console.error('[galfus-scripts] Failed:', error);
  process.exitCode = 1;
});
