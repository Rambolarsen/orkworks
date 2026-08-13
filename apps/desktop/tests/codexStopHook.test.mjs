import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { execFileSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(fileURLToPath(new URL('../../..', import.meta.url)));
const hooksPath = resolve(repoRoot, '.codex/hooks.json');
const hooks = JSON.parse(readFileSync(hooksPath, 'utf8'));
const expectedSessionStartCommand = '"$HOME/.orkworks/hook-scripts/report-harness-event.sh" --marker \'orkworks:harness-integration:v2:codex\'';
const expectedStopCommand = 'ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd); bash "$ROOT/.codex/hooks/doc-check-stop.sh"';

function collectCommands(value, commands = []) {
  if (Array.isArray(value)) {
    for (const item of value) collectCommands(item, commands);
  } else if (value && typeof value === 'object') {
    for (const [key, item] of Object.entries(value)) {
      if (['command', 'commandWindows', 'bash', 'powershell'].includes(key) && typeof item === 'string') {
        commands.push(item);
      }
      collectCommands(item, commands);
    }
  }
  return commands;
}

function runStopHook(environment = {}) {
  const stopCommand = hooks.hooks.Stop[0].hooks[0].command;
  return execFileSync('bash', ['-lc', stopCommand], {
    cwd: resolve(repoRoot, 'apps/desktop'),
    env: { ...process.env, ...environment },
    encoding: 'utf8',
  });
}

test('Codex hooks define exactly the supported SessionStart and Stop commands', () => {
  assert.deepEqual(Object.keys(hooks.hooks), ['SessionStart', 'Stop']);
  assert.deepEqual(hooks.hooks.SessionStart, [{
    hooks: [{
      type: 'command',
      command: expectedSessionStartCommand,
    }],
  }]);
  assert.equal(hooks.hooks.Stop.length, 1);
  assert.deepEqual(hooks.hooks.Stop[0], {
    hooks: [{
      type: 'command',
      command: expectedStopCommand,
    }],
  });
});

test('Codex hooks contain no unsupported generated hook commands', () => {
  const staleCommand = collectCommands(hooks).find((command) =>
    /ponytail|agent-skills|superpowers/i.test(command),
  );

  assert.equal(staleCommand, undefined, `unsupported generated hook command: ${staleCommand}`);
});

test('Codex Stop hook emits an empty JSON object when doc-check is clean', () => {
  assert.deepEqual(JSON.parse(runStopHook({
    ORKWORKS_DOC_CHECK_OUTPUT: '',
    ORKWORKS_DOC_CHECK_EXIT_CODE: '0',
  })), {});
});

test('Codex Stop hook emits diagnostic output as a system message', () => {
  const message = 'Documentation needs attention';

  assert.deepEqual(JSON.parse(runStopHook({
    ORKWORKS_DOC_CHECK_OUTPUT: message,
    ORKWORKS_DOC_CHECK_EXIT_CODE: '0',
  })), { systemMessage: message });
});

test('Codex Stop hook reports numeric and invalid exit code values', () => {
  assert.deepEqual(JSON.parse(runStopHook({
    ORKWORKS_DOC_CHECK_OUTPUT: 'A numeric failure',
    ORKWORKS_DOC_CHECK_EXIT_CODE: '7',
  })), { systemMessage: '[doc-check] Hook failed with exit 7.\nA numeric failure' });

  assert.deepEqual(JSON.parse(runStopHook({
    ORKWORKS_DOC_CHECK_OUTPUT: 'An invalid status',
    ORKWORKS_DOC_CHECK_EXIT_CODE: 'nope',
  })), { systemMessage: '[doc-check] Hook failed with invalid exit code nope.\nAn invalid status' });
});
