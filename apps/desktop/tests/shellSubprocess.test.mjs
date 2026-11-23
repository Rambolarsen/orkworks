import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { assertShellSubprocess, runShellSubprocess } from './fixtures/shellSubprocess.mjs';

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

test('runShellSubprocess returns on clean child exit even when a descendant inherits stdio', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'orkworks-shell-subprocess-'));
  const pidFile = join(dir, 'descendant.pid');
  try {
    const started = performance.now();
    const result = await runShellSubprocess(process.execPath, ['-e', `
      const { spawn } = require('node:child_process');
      const { writeFileSync } = require('node:fs');
      const descendant = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 10000)'], { stdio: 'inherit' });
      writeFileSync(${JSON.stringify(pidFile)}, String(descendant.pid));
      descendant.unref();
      writeFileSync(1, 'child stdout before exit');
      writeFileSync(2, '[shell-fixture] process exit: 0');
      process.exit(0);
    `], { timeout: 2000, logDir: dir });
    const elapsedMs = performance.now() - started;
    assert.equal(result.error, null);
    assert.equal(result.status, 0);
    assert.equal(result.signal, null);
    assert.match(result.stdout, /child stdout before exit/);
    assert.match(result.stderr, /\[shell-fixture\] process exit: 0/);
    assert.ok(elapsedMs < 1500, `expected prompt exit before 2000ms timeout, took ${Math.round(elapsedMs)}ms`);
    assert.doesNotThrow(() => assertShellSubprocess(result));
    if (process.platform !== 'win32') {
      const descendantPid = Number(readFileSync(pidFile, 'utf8').trim());
      assert.ok(Number.isInteger(descendantPid) && descendantPid > 0, `invalid descendant pid: ${descendantPid}`);
      const deadline = Date.now() + 1000;
      let alive = true;
      while (Date.now() < deadline) {
        try { process.kill(descendantPid, 0); }
        catch (err) { if (err.code === 'ESRCH') { alive = false; break; } }
        await new Promise(resolve => setTimeout(resolve, 20));
      }
      assert.equal(alive, false, `lingering descendant ${descendantPid} survived clean child exit`);
    }
  } finally {
    try {
      const pid = Number(readFileSync(pidFile, 'utf8').trim());
      if (Number.isInteger(pid) && pid > 0) process.kill(pid, 'SIGKILL');
    } catch { /* already exited */ }
    rmSync(dir, { recursive: true, force: true, maxRetries: 5, retryDelay: 50 });
  }
});

test('runShellSubprocess times out stalled child, terminates descendants, and retains both streams', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'orkworks-shell-subprocess-'));
  const pidFile = join(dir, 'stalled-descendant.pid');
  try {
    const result = await runShellSubprocess(process.execPath, ['-e', `
      const { spawn } = require('node:child_process');
      const { writeFileSync } = require('node:fs');
      const descendant = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 10000)'], { stdio: 'inherit' });
      writeFileSync(${JSON.stringify(pidFile)}, String(descendant.pid));
      writeFileSync(1, 'stalled stdout');
      writeFileSync(2, '[shell-fixture] waiting for renderer');
      setInterval(() => {}, 1000);
    `], { timeout: 1500, logDir: dir });
    assert.equal(result.error?.code, 'ETIMEDOUT');
    assert.throws(() => assertShellSubprocess(result), error => {
      assert.match(error.message, /stalled stdout/);
      assert.match(error.message, /\[shell-fixture\] waiting for renderer/);
      assert.ok(error.message.includes(`"pid":${result.pid}`));
      assert.equal(error.cause, result.error);
      return true;
    });
    if (process.platform !== 'win32') {
      const descendantPid = Number(readFileSync(pidFile, 'utf8').trim());
      assert.ok(Number.isInteger(descendantPid) && descendantPid > 0, `invalid descendant pid: ${descendantPid}`);
      const deadline = Date.now() + 1000;
      let alive = true;
      while (Date.now() < deadline) {
        try { process.kill(descendantPid, 0); }
        catch (err) { if (err.code === 'ESRCH') { alive = false; break; } }
        await new Promise(resolve => setTimeout(resolve, 20));
      }
      assert.equal(alive, false, `stalled descendant ${descendantPid} survived timeout cleanup`);
    }
  } finally {
    try {
      const pid = Number(readFileSync(pidFile, 'utf8').trim());
      if (Number.isInteger(pid) && pid > 0) process.kill(pid, 'SIGKILL');
    } catch { /* already exited */ }
    rmSync(dir, { recursive: true, force: true, maxRetries: 5, retryDelay: 50 });
  }
});

test('runShellSubprocess retains streams and exit code on nonzero exit', async () => {
  const result = await runShellSubprocess(process.execPath, ['-e', `
    console.log('child out');
    console.error('child err');
    process.exit(7);
  `], { timeout: 2000 });
  assert.equal(result.error, null);
  assert.equal(result.status, 7);
  assert.throws(() => assertShellSubprocess(result), error => {
    assert.match(error.message, /child out/);
    assert.match(error.message, /child err/);
    assert.ok(error.message.includes('"status":7'));
    return true;
  });
});

test('runShellSubprocess surfaces spawn errors without hanging', async () => {
  const result = await runShellSubprocess('/nonexistent-orkworks-shell-binary', [], { timeout: 2000 });
  assert.equal(result.error?.code, 'ENOENT');
  assert.throws(() => assertShellSubprocess(result), error => {
    assert.equal(error.cause, result.error);
    return true;
  });
});
