import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(fileURLToPath(new URL('../../..', import.meta.url)));
const settings = JSON.parse(readFileSync(resolve(repoRoot, '.claude/settings.json'), 'utf8'));

function walk(value, visit) {
  visit(value);
  if (Array.isArray(value)) {
    for (const item of value) walk(item, visit);
  } else if (value && typeof value === 'object') {
    for (const item of Object.values(value)) walk(item, visit);
  }
}

test('Claude project hooks use supported shell configuration', () => {
  const hookEntries = [];
  walk(settings.hooks, (value) => {
    if (value && typeof value === 'object' && value.type === 'command') hookEntries.push(value);
  });

  assert.ok(hookEntries.length > 0);
  for (const hook of hookEntries) {
    assert.equal('commandWindows' in hook, false);
  }
});
