import { appendFile, readFile, writeFile } from 'fs/promises';
import { resolve } from 'path';

const COMPONENTS = new Set(['editor', 'playground']);
const CHANNELS = new Set(['stable', 'alpha', 'beta', 'next', 'latest']);
const VERSION_PATTERN =
  /^(?<channel>[^/]+)\/(?<component>[^/]+)\/(?<version>\d+\.\d+\.\d+)$/;

type Component = 'editor' | 'playground';

function fail(message: string): never {
  throw new Error(message);
}

function parseComponent(): Component {
  const component = Bun.argv[2];
  if (!component || !COMPONENTS.has(component)) {
    fail('Expected component argument: editor or playground');
  }
  return component as Component;
}

function parseBranch(component: Component): {
  channel: string;
  version: string;
} {
  const branch = process.env.GITHUB_HEAD_REF || process.env.GITHUB_REF_NAME;
  if (!branch) {
    fail('GITHUB_HEAD_REF or GITHUB_REF_NAME is required');
  }

  const match = VERSION_PATTERN.exec(branch);
  if (!match?.groups) {
    fail(
      `Invalid release branch "${branch}"; expected <channel>/${component}/<version>`,
    );
  }

  const { channel, component: branchComponent, version } = match.groups;
  if (!CHANNELS.has(channel) || branchComponent !== component) {
    fail(
      `Invalid release branch "${branch}"; expected <channel>/${component}/<version>`,
    );
  }

  return { channel, version };
}

async function updateEditorVersion(
  root: string,
  version: string,
): Promise<void> {
  const path = resolve(root, 'editors/vscode/package.json');
  const manifest = JSON.parse(await readFile(path, 'utf8')) as Record<
    string,
    unknown
  >;
  manifest.version = version;
  await writeFile(path, `${JSON.stringify(manifest, null, 2)}\n`);
}

async function updatePlaygroundVersion(
  root: string,
  version: string,
): Promise<void> {
  const path = resolve(root, 'Cargo.toml');
  const manifest = await readFile(path, 'utf8');
  const updated = manifest.replace(
    /(\[workspace\.package\][\s\S]*?^version = ")[^"]+("$)/m,
    `$1${version}$2`,
  );
  if (updated === manifest) {
    fail('Unable to update workspace package version');
  }
  await writeFile(path, updated);
}

async function exportVersion(
  channel: string,
  component: Component,
  version: string,
): Promise<void> {
  const output = process.env.GITHUB_OUTPUT;
  if (!output) {
    console.log(JSON.stringify({ channel, component, version }));
    return;
  }
  await appendFile(
    output,
    `channel=${channel}\ncomponent=${component}\nversion=${version}\n`,
  );
}

async function main(): Promise<void> {
  const component = parseComponent();
  const { channel, version } = parseBranch(component);
  const root = resolve(import.meta.dir, '../..');

  if (component === 'editor') {
    await updateEditorVersion(root, version);
  } else {
    await updatePlaygroundVersion(root, version);
  }

  await exportVersion(channel, component, version);
}

main().catch((error) => {
  console.error('[prepare-component-version] Failed:', error);
  process.exitCode = 1;
});
