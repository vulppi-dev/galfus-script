import { appendFile, readFile, writeFile } from 'fs/promises';
import { resolve } from 'path';

const COMPONENTS = new Set(['all', 'editor', 'playground']);
const TAGS = new Set(['stable', 'alpha', 'beta', 'next', 'latest']);
const VERSION_PATTERN =
  /^(?<tag>[^/]+)\/(?<component>[^/]+)\/(?<version>\d+\.\d+\.\d+)$/;
const SEMVER_PATTERN = /^\d+\.\d+\.\d+$/;

type Component = 'all' | 'editor' | 'playground';

export type SetVersionOptions = {
  component?: string;
  tag?: string;
  version?: string;
};

function fail(message: string): never {
  throw new Error(message);
}

function parseComponent(component: string | undefined): Component {
  if (!component) {
    return 'all';
  }
  if (!COMPONENTS.has(component)) {
    fail('Expected component: all, editor, or playground');
  }
  return component as Component;
}

function parseBranch(component: Exclude<Component, 'all'>): {
  tag: string;
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

  const { tag, component: branchComponent, version } = match.groups;
  if (!tag || !branchComponent || !version) {
    fail(`Invalid release branch "${branch}"; expected <channel>/${component}/<version>`);
  }
  if (!TAGS.has(tag) || branchComponent !== component) {
    fail(
      `Invalid release branch "${branch}"; expected <channel>/${component}/<version>`,
    );
  }

  return { tag, version };
}

function parseVersion(options: SetVersionOptions, component: Component): {
  tag: string;
  version: string;
} {
  if (options.tag || options.version) {
    if (!options.tag || !options.version) {
      fail('Both --tag and --version are required when setting a version explicitly');
    }
    if (!TAGS.has(options.tag) || !SEMVER_PATTERN.test(options.version)) {
      fail('Expected a supported --tag and a semantic --version (for example: alpha 0.0.1)');
    }
    return { tag: options.tag, version: options.version };
  }

  if (component === 'all') {
    fail('--component is required when --tag and --version are not provided');
  }
  return parseBranch(component);
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
  tag: string,
  component: Component,
  version: string,
): Promise<void> {
  const output = process.env.GITHUB_OUTPUT;
  if (!output) {
    console.log(JSON.stringify({ tag, component, version }));
    return;
  }
  await appendFile(
    output,
    `tag=${tag}\ncomponent=${component}\nversion=${version}\n`,
  );
}

export async function setVersion(options: SetVersionOptions): Promise<void> {
  const component = parseComponent(options.component);
  const { tag, version } = parseVersion(options, component);
  const root = resolve(import.meta.dir, '../../..');

  if (component === 'all' || component === 'editor') {
    await updateEditorVersion(root, version);
  }
  if (component === 'all' || component === 'playground') {
    await updatePlaygroundVersion(root, version);
  }

  await exportVersion(tag, component, version);
}
