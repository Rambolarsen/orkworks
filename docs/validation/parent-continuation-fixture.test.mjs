import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

test('synthetic continuation peer satisfies its direct-call oracles and pinned evidence', () => {
  const directory = fileURLToPath(new URL('../../crates/orkworksd/tests/fixtures/parent-continuation/', import.meta.url));
  const run = spawnSync('python3', ['-I', '-B', '-m', 'unittest', 'discover', '-s', directory, '-p', 'test_peer.py', '-v'], {
    encoding: 'utf8', timeout: 10000,
  });
  assert.equal(run.status, 0, run.error?.message ?? run.stderr);
});
