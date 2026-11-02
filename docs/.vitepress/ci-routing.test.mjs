import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, matchesGlob, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const root = fileURLToPath(new URL('../../', import.meta.url));
const filters = parse(readFileSync(resolve(root, '.github/ci-paths.yml'), 'utf8'));
const routed = (surface, path) => filters[surface].some(pattern => matchesGlob(path, pattern));

function* rustFiles(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) yield* rustFiles(path);
    else if (entry.name.endsWith('.rs')) yield path;
  }
}

test('Rust embedded inputs outside the crate always trigger Rust validation', () => {
  const external = new Set();
  for (const directory of ['src', 'tests']) {
    for (const file of rustFiles(resolve(root, 'crates/orkworksd', directory))) {
      const source = readFileSync(file, 'utf8');
      for (const match of source.matchAll(/include_(?:str|bytes)!\(\s*"([^"]+)"/g)) {
        const path = relative(root, resolve(dirname(file), match[1]));
        if (!path.startsWith('crates/orkworksd/')) external.add(path);
      }
    }
  }
  assert.ok(external.size > 0, 'must discover real external inputs');
  const missing = [...external].filter(path => !routed('rust', path));
  assert.deepEqual(missing, [], 'external Rust inputs missing from CI routing');
});

test('Rust runtime terminal replay helper triggers Rust validation', () => {
  assert.ok(routed('rust', 'apps/desktop/tests/fixtures/terminalReplayModes.mjs'));
});

test('ordinary documentation skips product suites but behavioral fixtures do not', () => {
  for (const path of ['docs/home.md', 'docs/agents/project-context.md', 'specs/taskmaster.md']) {
    assert.equal(routed('desktop', path), false, path);
    assert.equal(routed('rust', path), false, path);
  }
  assert.ok(routed('rust', 'docs/validation/fixtures/score-directed-readiness-v4-2026-10-31/skipped-tests-sources.json'));
});
