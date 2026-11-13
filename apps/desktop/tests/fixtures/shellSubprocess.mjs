import assert from 'node:assert/strict';

export function assertShellSubprocess(result) {
  const details = JSON.stringify({
    pid: result.pid, status: result.status, signal: result.signal,
    error: result.error?.code ?? null,
  });
  const diagnostic = `Shell subprocess failed: ${details}\nstdout:\n${result.stdout ?? ''}\nstderr:\n${result.stderr ?? ''}`;
  if (result.error) throw new Error(diagnostic, { cause: result.error });
  assert.equal(result.status, 0, diagnostic);
}
