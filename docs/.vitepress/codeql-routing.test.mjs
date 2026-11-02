import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, relative, resolve } from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';
import { selectCodeqlLanguages, detectCodeqlChanges } from '../../scripts/codeql-routing.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const all = ['actions', 'c-cpp', 'javascript-typescript', 'python', 'rust'];
const four = all.slice(0, -1);
const select = paths => selectCodeqlLanguages(paths, { complete: true });

test('only complete unrelated renderer/prose inventories skip Rust', () => {
  for (const path of ['apps/desktop/src/App.tsx', 'apps/desktop/src/theme.css', 'docs/agents/project-context.md', 'specs/taskmaster.md', 'skills/example/SKILL.md']) {
    assert.deepEqual(select([path]).language, four, path);
  }
  for (const paths of [[], null, [''], ['../docs/readme.md'], ['docs/a.md', 'unknown.data']]) {
    assert.deepEqual(select(paths).language, all);
  }
  assert.deepEqual(selectCodeqlLanguages(['docs/a.md']).language, all);
  assert.deepEqual(selectCodeqlLanguages(['docs/a.md'], { complete: true, forceFull: true }).language, all);
});

test('Rust inputs and unknown paths take precedence over the skip allowlist', () => {
  for (const path of ['crates/new-crate/src/main.rs', 'docs/example.rs', 'apps/desktop/src/example.rs', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo/config.toml', '.github/workflows/codeql.yml', 'scripts/codeql-routing.mjs', 'apps/desktop/resources/knowledge/starter.json', 'apps/desktop/tests/fixtures/helper.mjs', 'tests/fixtures/shared.json', 'docs/validation/fixtures/example.md', 'apps/desktop/src/fixtures/example.ts']) {
    assert.deepEqual(select(['docs/readme.md', path]).language, all, path);
  }
});

test('every literal embedded input in every Rust crate runs Rust', () => {
  let checked = 0;
  function walk(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = resolve(directory, entry.name);
      if (entry.isDirectory() && entry.name !== 'target') walk(path);
      else if (entry.isFile() && entry.name.endsWith('.rs')) {
        for (const match of readFileSync(path, 'utf8').matchAll(/include_(?:str|bytes)!\(\s*"([^"]+)"/g)) {
          const input = relative(root, resolve(dirname(path), match[1]));
          assert.deepEqual(select([input]).language, all, input); checked++;
        }
      }
    }
  }
  walk(resolve(root, 'crates'));
  assert.ok(checked > 0);
});

test('Git detection uses cumulative PR diff, immutable push SHAs, and both rename paths', () => {
  const cwd = mkdtempSync(resolve(tmpdir(), 'codeql-routing-'));
  const git = (...args) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
  const write = (path, text) => { mkdirSync(dirname(resolve(cwd, path)), { recursive: true }); writeFileSync(resolve(cwd, path), text); };
  const commit = () => { git('add', '.'); git('commit', '-qm', 'fixture'); return git('rev-parse', 'HEAD'); };
  try {
    git('init', '-q'); git('config', 'user.email', 'test@example.invalid'); git('config', 'user.name', 'Test');
    write('crates/a.rs', 'baseline'); const base = commit();
    write('crates/a.rs', 'changed'); const rust = commit();
    write('docs/a.md', 'docs'); const head = commit();
    const pr = detectCodeqlChanges('pull_request', { pull_request: { base: { sha: base }, head: { sha: head } } }, head, cwd);
    assert.deepEqual(selectCodeqlLanguages(pr.paths, pr).language, all);
    assert.deepEqual(detectCodeqlChanges('push', { before: rust }, head, cwd).paths, ['docs/a.md']);
    const push = detectCodeqlChanges('push', { before: rust }, head, cwd);
    assert.deepEqual(selectCodeqlLanguages(push.paths, push).language, four);
    git('mv', 'crates/a.rs', 'docs/a.md.rs'); const renamed = commit();
    const rename = detectCodeqlChanges('push', { before: head }, renamed, cwd);
    assert.ok(rename.paths.includes('crates/a.rs')); assert.ok(rename.paths.includes('docs/a.md.rs'));
    assert.equal(detectCodeqlChanges('push', { before: '0'.repeat(40) }, head, cwd).forceFull, true);
    assert.equal(detectCodeqlChanges('push', { before: 'f'.repeat(40) }, head, cwd).complete, false);
    for (const event of ['schedule', 'workflow_dispatch']) assert.equal(detectCodeqlChanges(event, {}, head, cwd).forceFull, true);
  } finally { rmSync(cwd, { recursive: true, force: true }); }
});

test('workflow preserves publication, category and cancellation boundaries', () => {
  const workflow = parse(readFileSync(resolve(root, '.github/workflows/codeql.yml'), 'utf8'));
  assert.equal(workflow.on.pull_request_target, undefined);
  assert.deepEqual(workflow.on.pull_request.branches, ['main']);
  assert.deepEqual(workflow.on.push.branches, ['main']);
  assert.ok(workflow.on.schedule.length > 0);
  assert.ok(Object.hasOwn(workflow.on, 'workflow_dispatch'));
  assert.equal(workflow.concurrency['cancel-in-progress'], "${{ github.event_name == 'pull_request' }}");
  assert.match(workflow.concurrency.group, /github.run_id/);
  assert.match(workflow.jobs.route.if, /head.repo.full_name == github.repository/);
  assert.equal(workflow.permissions.contents, 'read');
  assert.equal(workflow.jobs.analyze.permissions['security-events'], 'write');
  const steps = workflow.jobs.analyze.steps;
  const init = steps.find(s => s.id === 'init');
  assert.equal(init.with['build-mode'], 'none');
  assert.equal(init.with.queries, undefined);
  assert.equal(init.with['config-file'], undefined);
  const analyze = steps.find(s => s.name === 'Analyze');
  assert.equal(analyze.with.category, '/language:${{ matrix.language }}');
  assert.match(analyze.with.upload, /CODEQL_ADVANCED_UPLOAD == 'true'/);
  assert.match(analyze.with.upload, /'always' \|\| 'never'/);
  for (const [enabled, ref, event, expected] of [
    ['', 'refs/heads/main', 'push', 'never'],
    ['true', 'refs/heads/main', 'push', 'always'],
    ['true', 'refs/pull/1/merge', 'pull_request', 'always'],
    ['true', 'refs/heads/staging', 'workflow_dispatch', 'never'],
  ]) {
    const expression = analyze.with.upload.slice(3, -2);
    const actual = new Function('vars', 'github', `return (${expression})`)({ CODEQL_ADVANCED_UPLOAD: enabled }, { ref, event_name: event });
    assert.equal(actual, expected);
  }
});

test('qualification metadata follows canonical CodeQL database locations for aliases', () => {
  const workflow = parse(readFileSync(resolve(root, '.github/workflows/codeql.yml'), 'utf8'));
  const script = workflow.jobs.analyze.steps.find(s => s.name === 'Record qualification inputs').run;
  const fixture = mkdtempSync(resolve(tmpdir(), 'codeql-metadata-'));
  try {
    for (const [alias, canonical] of [['c-cpp', 'cpp'], ['javascript-typescript', 'javascript']]) {
      const cwd = resolve(fixture, alias); const database = resolve(fixture, 'codeql-databases', canonical);
      mkdirSync(cwd, { recursive: true }); mkdirSync(database, { recursive: true });
      writeFileSync(resolve(database, 'codeql-database.yml'), `language: ${canonical}\n`);
      execFileSync('bash', ['-e'], { cwd, input: script, env: { ...process.env, GIT_DIR: execFileSync('git', ['rev-parse', '--absolute-git-dir'], { cwd: root, encoding: 'utf8' }).trim(), CODEQL_BIN: '/usr/bin/true', RUNNER_TEMP: fixture, LANGUAGE: alias, DATABASE_LOCATIONS: JSON.stringify({ [canonical]: database }) }, stdio: ['pipe', 'pipe', 'pipe'] });
      assert.equal(readFileSync(resolve(cwd, 'evidence/database.yml'), 'utf8'), `language: ${canonical}\n`);
    }
  } finally { rmSync(fixture, { recursive: true, force: true }); }
});
