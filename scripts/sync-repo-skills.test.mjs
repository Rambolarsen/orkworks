import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const testDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(testDir, '..');
const scriptPath = join(repoRoot, 'scripts/sync-repo-skills.mjs');
const targets = ['.claude/skills', '.agents/skills', '.github/skills'];

function createFixtureRoot(t) {
  const root = mkdtempSync(join(tmpdir(), 'orkworks-sync-skills-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  return root;
}

function writeFixtureFile(root, relativePath, content) {
  const fullPath = join(root, relativePath);
  mkdirSync(dirname(fullPath), { recursive: true });
  writeFileSync(fullPath, content, 'utf8');
}

function runSync(root, extraArgs = []) {
  return spawnSync(process.execPath, [scriptPath, '--root', root, ...extraArgs], {
    encoding: 'utf8',
  });
}

test('syncs all repo skills as real copies into empty worktree targets and passes --check', (t) => {
  const root = createFixtureRoot(t);
  writeFixtureFile(root, 'skills/alpha/SKILL.md', '---\nname: alpha\n---\n# Alpha\n');
  writeFixtureFile(root, 'skills/alpha/references/notes.md', 'alpha notes\n');
  writeFixtureFile(root, 'skills/beta/SKILL.md', '---\nname: beta\n---\n# Beta\n');
  // Non-skill entry under skills/ should be ignored.
  writeFixtureFile(root, 'skills/.DS_Store', 'junk');

  const syncResult = runSync(root);
  assert.equal(syncResult.status, 0, syncResult.stderr);

  for (const target of targets) {
    const alphaDir = join(root, target, 'alpha');
    assert.equal(lstatSync(alphaDir).isSymbolicLink(), false);
    assert.equal(lstatSync(alphaDir).isDirectory(), true);
    assert.equal(
      readFileSync(join(alphaDir, 'SKILL.md'), 'utf8'),
      '---\nname: alpha\n---\n# Alpha\n',
    );
    assert.equal(
      readFileSync(join(alphaDir, 'references/notes.md'), 'utf8'),
      'alpha notes\n',
    );
    assert.equal(
      readFileSync(join(root, target, 'beta/SKILL.md'), 'utf8'),
      '---\nname: beta\n---\n# Beta\n',
    );
  }

  const checkResult = runSync(root, ['--check']);
  assert.equal(checkResult.status, 0, checkResult.stderr);
});

test('is idempotent and preserves unrelated external APM skills', (t) => {
  const root = createFixtureRoot(t);
  writeFixtureFile(root, 'skills/alpha/SKILL.md', '# Alpha\n');
  writeFixtureFile(root, '.claude/skills/brainstorming/SKILL.md', '# External Claude\n');
  writeFixtureFile(root, '.agents/skills/brainstorming/SKILL.md', '# External Agents\n');
  writeFixtureFile(root, '.github/skills/custom-ext/SKILL.md', '# External Copilot\n');

  assert.equal(runSync(root).status, 0);
  assert.equal(runSync(root).status, 0);

  assert.equal(
    readFileSync(join(root, '.claude/skills/brainstorming/SKILL.md'), 'utf8'),
    '# External Claude\n',
  );
  assert.equal(
    readFileSync(join(root, '.agents/skills/brainstorming/SKILL.md'), 'utf8'),
    '# External Agents\n',
  );
  assert.equal(
    readFileSync(join(root, '.github/skills/custom-ext/SKILL.md'), 'utf8'),
    '# External Copilot\n',
  );
  assert.equal(runSync(root, ['--check']).status, 0);
});

test('migrates legacy symlinks pointing to skills/<name> and rejects symlinks pointing elsewhere', (t) => {
  const root = createFixtureRoot(t);
  writeFixtureFile(root, 'skills/alpha/SKILL.md', '# Alpha\n');
  mkdirSync(join(root, '.claude/skills'), { recursive: true });
  symlinkSync('../../skills/alpha', join(root, '.claude/skills/alpha'));

  // --check must report legacy symlink as out of sync without mutating it.
  const checkBefore = runSync(root, ['--check']);
  assert.equal(checkBefore.status, 1);
  assert.match(checkBefore.stderr, /symlink/i);
  assert.equal(lstatSync(join(root, '.claude/skills/alpha')).isSymbolicLink(), true);

  const syncResult = runSync(root);
  assert.equal(syncResult.status, 0, syncResult.stderr);
  assert.equal(lstatSync(join(root, '.claude/skills/alpha')).isSymbolicLink(), false);
  assert.equal(readFileSync(join(root, '.claude/skills/alpha/SKILL.md'), 'utf8'), '# Alpha\n');
  assert.equal(runSync(root, ['--check']).status, 0);

  // Symlink pointing outside skills/<name> must fail closed.
  const outsideRoot = createFixtureRoot(t);
  writeFixtureFile(outsideRoot, 'skills/alpha/SKILL.md', '# Alpha\n');
  writeFixtureFile(outsideRoot, 'other/SKILL.md', '# Other\n');
  mkdirSync(join(outsideRoot, '.claude/skills'), { recursive: true });
  symlinkSync('../../other', join(outsideRoot, '.claude/skills/alpha'));

  const foreignResult = runSync(outsideRoot);
  assert.equal(foreignResult.status, 1);
  assert.match(foreignResult.stderr, /collision|unmanaged|symlink/i);
  assert.equal(lstatSync(join(outsideRoot, '.claude/skills/alpha')).isSymbolicLink(), true);
});

test('updates drifted content, removes stale files, and prunes removed repo skills without touching external skills', (t) => {
  const root = createFixtureRoot(t);
  writeFixtureFile(root, 'skills/alpha/SKILL.md', '# Alpha v1\n');
  writeFixtureFile(root, 'skills/alpha/extra.md', 'old extra\n');
  writeFixtureFile(root, 'skills/beta/SKILL.md', '# Beta\n');
  writeFixtureFile(root, '.agents/skills/external/SKILL.md', '# External\n');

  assert.equal(runSync(root).status, 0);

  // Modify alpha, delete alpha/extra.md, and remove beta completely.
  writeFixtureFile(root, 'skills/alpha/SKILL.md', '# Alpha v2\n');
  rmSync(join(root, 'skills/alpha/extra.md'));
  rmSync(join(root, 'skills/beta'), { recursive: true, force: true });

  // --check catches drift and stale managed skill without writing.
  const checkStale = runSync(root, ['--check']);
  assert.equal(checkStale.status, 1);
  assert.match(checkStale.stderr, /alpha|beta/);

  const syncAfter = runSync(root);
  assert.equal(syncAfter.status, 0, syncAfter.stderr);

  for (const target of targets) {
    assert.equal(readFileSync(join(root, target, 'alpha/SKILL.md'), 'utf8'), '# Alpha v2\n');
    assert.deepEqual(readdirSync(join(root, target, 'alpha')), ['SKILL.md']);
    assert.equal(readdirSync(join(root, target)).includes('beta'), false);
  }
  assert.equal(
    readFileSync(join(root, '.agents/skills/external/SKILL.md'), 'utf8'),
    '# External\n',
  );
  assert.equal(runSync(root, ['--check']).status, 0);
});

test('fails closed before writing any target when a repo skill collides with an unmanaged skill', (t) => {
  const root = createFixtureRoot(t);
  writeFixtureFile(root, 'skills/alpha/SKILL.md', '# Repo Alpha\n');
  writeFixtureFile(root, 'skills/writing-skills/SKILL.md', '# Repo Writing Skills\n');
  writeFixtureFile(root, '.agents/skills/writing-skills/SKILL.md', '# Superpowers Writing Skills\n');

  const result = runSync(root);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /writing-skills/);
  assert.match(result.stderr, /collision|unmanaged/i);

  // No target should have been partially written.
  assert.throws(() => lstatSync(join(root, '.claude/skills/alpha')));
  assert.throws(() => lstatSync(join(root, '.github/skills/alpha')));
  assert.equal(
    readFileSync(join(root, '.agents/skills/writing-skills/SKILL.md'), 'utf8'),
    '# Superpowers Writing Skills\n',
  );
});

test('refuses symlinks inside canonical skills/ and exits 2 on unknown flags', (t) => {
  const root = createFixtureRoot(t);
  writeFixtureFile(root, 'skills/alpha/SKILL.md', '# Alpha\n');
  writeFixtureFile(root, 'secret.txt', 'private\n');
  symlinkSync('../../secret.txt', join(root, 'skills/alpha/leak.txt'));

  const symlinkResult = runSync(root);
  assert.equal(symlinkResult.status, 1);
  assert.match(symlinkResult.stderr, /symlink/i);
  assert.throws(() => lstatSync(join(root, '.claude/skills/alpha')));

  const badFlag = runSync(root, ['--unknown']);
  assert.equal(badFlag.status, 2);
});

test('rewrites relative Markdown links that leave the skill directory while preserving in-skill links and code', (t) => {
  const root = createFixtureRoot(t);
  writeFixtureFile(root, 'docs/agents/local-sonarqube.md', '# Runbook\n');
  writeFixtureFile(root, 'docs/agents/development-workflow.md', '# Workflow\n');
  writeFixtureFile(
    root,
    'skills/alpha/SKILL.md',
    [
      'Read [runbook](../../docs/agents/local-sonarqube.md) and',
      '[tie-off](../../docs/agents/development-workflow.md#taskmaster-recommendation-tie-off).',
      'Keep [in-skill](references/notes.md), [anchor](#top), and [url](https://example.com).',
      'Keep inline code `[ignored](../../docs/agents/local-sonarqube.md)` untouched.',
      '```markdown',
      '[fenced](../../docs/agents/local-sonarqube.md)',
      '```',
      '',
    ].join('\n'),
  );
  writeFixtureFile(
    root,
    'skills/alpha/references/notes.md',
    'See [runbook](../../../docs/agents/local-sonarqube.md) and [parent](../SKILL.md).\n',
  );

  const syncResult = runSync(root);
  assert.equal(syncResult.status, 0, syncResult.stderr);

  for (const target of targets) {
    const skillText = readFileSync(join(root, target, 'alpha/SKILL.md'), 'utf8');
    assert.match(skillText, /\[runbook\]\(\.\.\/\.\.\/\.\.\/docs\/agents\/local-sonarqube\.md\)/);
    assert.match(
      skillText,
      /\[tie-off\]\(\.\.\/\.\.\/\.\.\/docs\/agents\/development-workflow\.md#taskmaster-recommendation-tie-off\)/,
    );
    assert.match(skillText, /\[in-skill\]\(references\/notes\.md\)/);
    assert.match(skillText, /\[anchor\]\(#top\)/);
    assert.match(skillText, /\[url\]\(https:\/\/example\.com\)/);
    assert.match(skillText, /`\[ignored\]\(\.\.\/\.\.\/docs\/agents\/local-sonarqube\.md\)`/);
    assert.match(skillText, /```markdown\n\[fenced\]\(\.\.\/\.\.\/docs\/agents\/local-sonarqube\.md\)\n```/);

    // Verify rewritten relative link actually resolves to the canonical file.
    const resolvedRunbook = resolve(
      join(root, target, 'alpha'),
      '../../../docs/agents/local-sonarqube.md',
    );
    assert.equal(readFileSync(resolvedRunbook, 'utf8'), '# Runbook\n');

    const nestedText = readFileSync(join(root, target, 'alpha/references/notes.md'), 'utf8');
    assert.match(nestedText, /\[runbook\]\(\.\.\/\.\.\/\.\.\/\.\.\/docs\/agents\/local-sonarqube\.md\)/);
    assert.match(nestedText, /\[parent\]\(\.\.\/SKILL\.md\)/);
  }

  assert.equal(runSync(root, ['--check']).status, 0);

  // Overwriting a materialized file with the un-rewritten link must fail --check.
  writeFixtureFile(
    root,
    '.claude/skills/alpha/SKILL.md',
    readFileSync(join(root, 'skills/alpha/SKILL.md'), 'utf8'),
  );
  const checkAfterDrift = runSync(root, ['--check']);
  assert.equal(checkAfterDrift.status, 1);
  assert.match(checkAfterDrift.stderr, /drifted/i);
});
