import { getRepoContext, paginateGitHub, readEventPayload, githubRequest } from './api';

type PullRequestLabel = {
  name: string;
};

type PullRequestPayload = {
  pull_request?: {
    body?: string | null;
    labels: PullRequestLabel[];
    number: number;
  };
};

const MANAGED_LABELS = [
  'changelog:breaking',
  'changelog:feature',
  'changelog:fix',
  'changelog:refactor',
  'changelog:performance',
  'changelog:docs',
  'changelog:internal'
] as const;

const LABEL_META: Record<string, { color: string; description: string }> = {
  'changelog:breaking': { color: 'd73a4a', description: 'Breaking changes that require user intervention' },
  'changelog:feature': { color: 'a2eeef', description: 'New features and enhancements' },
  'changelog:fix': { color: '34c759', description: 'Bug fixes and stability improvements' },
  'changelog:refactor': { color: 'cca7f1', description: 'Code refactoring without functional changes' },
  'changelog:performance': { color: 'ffcc00', description: 'Performance optimizations' },
  'changelog:docs': { color: '007aff', description: 'Documentation updates and design specifications' },
  'changelog:internal': { color: '8e8e93', description: 'Internal changes excluded from the public changelog' }
};

type RepoLabel = {
  name: string;
};

async function main(): Promise<void> {
  const payload = await readEventPayload<PullRequestPayload>();
  const pr = payload.pull_request;
  if (!pr) {
    return;
  }

  const checked = new Set<string>();
  const body = pr.body ?? '';
  const checkedPattern = /-\s*\[(x|X)\]\s*`([^`]+)`/g;
  let match: RegExpExecArray | null;
  while ((match = checkedPattern.exec(body)) !== null) {
    const label = match[2]?.trim();
    if (label && MANAGED_LABELS.includes(label as (typeof MANAGED_LABELS)[number])) {
      checked.add(label);
    }
  }

  const { owner, repo } = getRepoContext();
  const currentPrLabels = new Set(pr.labels.map((label) => label.name));
  const existingRepoLabels = await paginateGitHub<RepoLabel>(`/repos/${owner}/${repo}/labels`);
  const repoLabelSet = new Set(existingRepoLabels.map((label) => label.name));

  const toAdd = [...checked].filter(
    (label) => !currentPrLabels.has(label)
  );

  for (const label of toAdd) {
    if (!repoLabelSet.has(label)) {
      try {
        const meta = LABEL_META[label] || { color: 'ededed', description: '' };
        await githubRequest({
          method: 'POST',
          path: `/repos/${owner}/${repo}/labels`,
          body: {
            name: label,
            color: meta.color,
            description: meta.description
          }
        });
        console.log(`Created missing repository label: ${label}`);
        repoLabelSet.add(label);
      } catch (error) {
        console.error(`Failed to create repository label: ${label}`, error);
      }
    }
  }

  const toRemove = MANAGED_LABELS.filter(
    (label) => currentPrLabels.has(label) && !checked.has(label)
  );

  if (toAdd.length > 0) {
    await githubRequest({
      method: 'POST',
      path: `/repos/${owner}/${repo}/issues/${pr.number}/labels`,
      body: { labels: toAdd }
    });
  }

  for (const label of toRemove) {
    try {
      await githubRequest({
        method: 'DELETE',
        path: `/repos/${owner}/${repo}/issues/${pr.number}/labels/${encodeURIComponent(label)}`
      });
    } catch (error) {
      const status = (error as { status?: number }).status;
      if (status !== 404) {
        throw error;
      }
    }
  }

  console.log(`changelog labels added=[${toAdd.join(', ')}], removed=[${toRemove.join(', ')}]`);
}

main().catch((error) => {
  console.error('[sync-changelog-labels] Failed:', error);
  process.exitCode = 1;
});
