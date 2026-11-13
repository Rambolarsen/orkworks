import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { assertShellSubprocess } from './fixtures/shellSubprocess.mjs';

const run = (source, options = {}) => spawnSync(process.execPath, ['-e', source], {
  encoding: 'utf8', ...options,
});

test('shell timeout retains child progress, both streams and process outcome', () => {
  const result = run(`
    require('node:fs').writeSync(1, 'output before stall');
    require('node:fs').writeSync(2, '[shell-fixture] waiting for renderer');
    setInterval(() => {}, 1000);
  `, { timeout: 2000 });
  assert.equal(result.error?.code, 'ETIMEDOUT');
  assert.throws(() => assertShellSubprocess(result), error => {
    assert.match(error.message, /output before stall/);
    assert.match(error.message, /\[shell-fixture\] waiting for renderer/);
    assert.ok(error.message.includes(`"pid":${result.pid}`));
    assert.ok(error.message.includes(`"status":${result.status}`));
    assert.ok(error.message.includes(`"signal":${JSON.stringify(result.signal)}`));
    assert.equal(error.cause, result.error);
    return true;
  });
});

test('shell nonzero exit retains both streams and exit status', () => {
  const result = run(`console.log('child output'); console.error('child error'); process.exit(7);`);
  assert.throws(() => assertShellSubprocess(result), error => {
    assert.match(error.message, /child output/);
    assert.match(error.message, /child error/);
    assert.ok(error.message.includes('"status":7'));
    return true;
  });
});

test('shell successful exit remains successful', () => {
  assert.doesNotThrow(() => assertShellSubprocess(run('console.log("passed");')));
});
