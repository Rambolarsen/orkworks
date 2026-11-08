import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

test('unapproved native continuation packet passes only synthetic clients and process checks', () => {
  const directory = fileURLToPath(new URL('../../crates/orkworksd/tests/fixtures/parent-native-packet/', import.meta.url));
  const run = spawnSync('python3', ['-I', '-B', '-m', 'unittest', 'discover', '-s', directory, '-p', 'test_packet.py', '-v'], {
    encoding: 'utf8', timeout: 30000,
  });
  assert.equal(run.status, 0, run.error?.message ?? run.stderr);
});
