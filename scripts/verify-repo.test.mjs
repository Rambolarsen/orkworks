import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const scripts = dirname(fileURLToPath(import.meta.url));

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'orkworks-verifier-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const write = (path, contents) => {
    const target = join(root, path);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, contents);
  };
  mkdirSync(join(root, 'scripts'));
  for (const name of ['verify-repo.sh', 'verify-prerequisites.cjs']) {
    if (existsSync(join(scripts, name))) copyFileSync(join(scripts, name), join(root, 'scripts', name));
  }
  write('.nvmrc', process.versions.node.split('.')[0] + '\n');
  for (const [surface, dependencies] of [
    ['apps/desktop', { 'fs-ext': '*', typescript: '*' }],
    ['docs', { yaml: '*' }],
  ]) {
    write(`${surface}/package.json`, JSON.stringify({ devDependencies: dependencies }));
    for (const name of Object.keys(dependencies)) {
      write(`${surface}/node_modules/${name}/package.json`, JSON.stringify({ name, main: 'index.js' }));
      write(`${surface}/node_modules/${name}/index.js`, 'module.exports = {};\n');
    }
  }
  // Stop at the first expensive operation, so continuation is observable without
  // compiling the product or modifying dependency caches.
  write('bin/cargo', '#!/bin/sh\nprintf reached > "$VERIFIER_CARGO_MARKER"\nexit 77\n');
  chmodSync(join(root, 'bin/cargo'), 0o755);
  const result = (...args) => spawnSync('bash', [join(root, 'scripts/verify-repo.sh'), ...args], {
    encoding: 'utf8',
    env: { ...process.env, PATH: join(root, 'bin') + ':' + process.env.PATH, VERIFIER_CARGO_MARKER: join(root, 'cargo-reached') },
  });
  return { root, write, result, reached: () => existsSync(join(root, 'cargo-reached')) };
}

test('wrong Node major stops verification before Cargo with the pinned version', t => {
  const f = fixture(t);
  const expected = String(Number(process.versions.node.split('.')[0]) + 1);
  f.write('.nvmrc', expected + '\n');
  const result = f.result();
  assert.notEqual(result.status, 0);
  assert.equal(f.reached(), false, result.stdout + result.stderr);
  assert.match(result.stderr, new RegExp(`Node ${expected}`));
  assert.match(result.stderr, /\.nvmrc/);
});

for (const [surface, name] of [['apps/desktop', 'typescript'], ['docs', 'yaml']]) {
  test(`missing ${surface} dependency stops verification with scoped install guidance`, t => {
    const f = fixture(t);
    rmSync(join(f.root, surface, 'node_modules', name), { recursive: true });
    const result = f.result();
    assert.notEqual(result.status, 0);
    assert.equal(f.reached(), false, result.stdout + result.stderr);
    assert.ok(result.stderr.includes(name), result.stderr);
    assert.ok(result.stderr.includes(`pnpm --dir ${surface} install --frozen-lockfile`), result.stderr);
  });
}

test('unloadable host native module stops verification without rebuilding it', t => {
  const f = fixture(t);
  const contents = "throw Object.assign(new Error('incompatible native fixture'), {code: 'ERR_DLOPEN_FAILED'});\n";
  f.write('apps/desktop/node_modules/fs-ext/index.js', contents);
  const result = f.result();
  assert.notEqual(result.status, 0);
  assert.equal(f.reached(), false, result.stdout + result.stderr);
  assert.match(result.stderr, /fs-ext/);
  assert.match(result.stderr, /host Node/);
  assert.equal(readFileSync(join(f.root, 'apps/desktop/node_modules/fs-ext/index.js'), 'utf8'), contents);
});

test('installed prerequisites continue into the unchanged verifier failure path', t => {
  const f = fixture(t);
  const result = f.result();
  assert.equal(result.status, 77, result.stdout + result.stderr);
  assert.equal(f.reached(), true);
  assert.match(result.stdout, /Rust formatting/);
  assert.doesNotMatch(result.stdout, /Rust build/);
  assert.doesNotMatch(result.stdout, /all repository checks passed/);
});

test('dry-run needs no installed dependencies and performs no expensive work', t => {
  const f = fixture(t);
  rmSync(join(f.root, 'apps'), { recursive: true });
  rmSync(join(f.root, 'docs'), { recursive: true });
  f.write('.nvmrc', '0\n');
  const result = f.result('--dry-run');
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.equal(f.reached(), false);
  assert.match(result.stdout, /Docs build/);
});
