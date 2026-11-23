import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { closeSync, mkdtempSync, openSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const readLog = path => {
  try { return readFileSync(path, 'utf8'); }
  catch { return ''; }
};

/**
 * Run a shell fixture subprocess with file-backed stdio so lingering helper
 * descendants holding inherited descriptors cannot stall completion after the
 * direct child exits. On POSIX, run in an owned process group so clean exit,
 * timeouts, and parent interrupts clean up descendant helpers too.
 */
export async function runShellSubprocess(command, args, options = {}) {
  const { env = process.env, timeout = 30000, logDir, windowsHide = true } = options;
  const ownDir = logDir ? null : mkdtempSync(join(tmpdir(), 'orkworks-shell-io-'));
  const dir = logDir ?? ownDir;
  const stdoutPath = join(dir, 'shell-subprocess-stdout.log');
  const stderrPath = join(dir, 'shell-subprocess-stderr.log');
  const detached = process.platform !== 'win32';

  try {
    let stdoutFd;
    let stderrFd;
    let child;
    try {
      stdoutFd = openSync(stdoutPath, 'w');
      stderrFd = openSync(stderrPath, 'w');
      child = spawn(command, args, {
        env,
        stdio: ['ignore', stdoutFd, stderrFd],
        detached,
        windowsHide,
      });
    } finally {
      if (stdoutFd !== undefined) closeSync(stdoutFd);
      if (stderrFd !== undefined) closeSync(stderrFd);
    }

    const killTree = signal => {
      if (!child.pid) return;
      if (detached) {
        try { process.kill(-child.pid, signal); return; }
        catch (err) { if (err.code === 'ESRCH') return; }
      } else if (process.platform === 'win32') {
        try {
          const killed = spawnSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], {
            stdio: 'ignore', windowsHide: true, timeout: 3000,
          });
          if (!killed.error && killed.status === 0) return;
        } catch { /* fall back to direct kill */ }
      }
      try { child.kill(signal); }
      catch { /* already exited */ }
    };

    return await new Promise(resolve => {
      let settled = false;
      let timeoutError = null;
      let forceTimer = null;

      const forwardSignal = sig => {
        killTree('SIGKILL');
        process.off('SIGTERM', forwardSignal);
        process.off('SIGINT', forwardSignal);
        if (process.listenerCount(sig) === 0) {
          process.kill(process.pid, sig);
        }
      };

      const finish = (status, signal, error) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        if (forceTimer) clearTimeout(forceTimer);
        process.off('SIGTERM', forwardSignal);
        process.off('SIGINT', forwardSignal);
        if (detached && child.pid) {
          killTree(timeoutError ? 'SIGKILL' : 'SIGTERM');
        }
        resolve({
          pid: child.pid ?? null,
          status,
          signal,
          stdout: readLog(stdoutPath),
          stderr: readLog(stderrPath),
          error: error ?? timeoutError,
        });
      };

      process.on('SIGTERM', forwardSignal);
      process.on('SIGINT', forwardSignal);

      const timer = timeout > 0 ? setTimeout(() => {
        timeoutError = Object.assign(new Error(`Shell subprocess timed out after ${timeout}ms`), {
          code: 'ETIMEDOUT',
        });
        killTree('SIGKILL');
        forceTimer = setTimeout(() => {
          child.unref();
          finish(null, 'SIGKILL', timeoutError);
        }, 2000);
        forceTimer.unref();
      }, timeout) : null;

      child.once('error', error => finish(null, null, error));
      child.once('exit', (code, signal) => finish(code, signal, null));
    });
  } finally {
    if (ownDir) {
      try {
        rmSync(ownDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 50 });
      } catch (err) {
        if (process.platform !== 'win32' || !['EBUSY', 'ENOTEMPTY', 'EPERM'].includes(err.code)) {
          throw err;
        }
      }
    }
  }
}

export function assertShellSubprocess(result) {
  const details = JSON.stringify({
    pid: result.pid, status: result.status, signal: result.signal,
    error: result.error?.code ?? null,
  });
  const diagnostic = `Shell subprocess failed: ${details}\nstdout:\n${result.stdout ?? ''}\nstderr:\n${result.stderr ?? ''}`;
  if (result.error) throw new Error(diagnostic, { cause: result.error });
  assert.equal(result.status, 0, diagnostic);
}
