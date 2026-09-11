import { test } from 'node:test';
import assert from 'node:assert/strict';
import { validateGuideChanges } from './docs-audit-scope.mjs';
test('audit accepts up to three modified existing guides and reports no-change', () => {
  assert.equal(validateGuideChanges('', ''), 0);
  assert.equal(validateGuideChanges('M\tdocs/user/sessions.md\nM\tdocs/user/taskmaster.md\n', ''), 2);
});
test('audit rejects out-of-scope paths, new files, removals, renames and excess changes', () => {
  for (const changes of [
    'M\t.github/workflows/docs.yml', 'M\tdocs/user/../index.md',
    'A\tdocs/user/new.md', 'D\tdocs/user/sessions.md',
    'R100\tdocs/user/sessions.md\tdocs/user/new.md',
    ['a','b','c','d'].map(n => `M\tdocs/user/${n}.md`).join('\n'),
  ]) assert.throws(() => validateGuideChanges(changes, ''), /scope/);
  assert.throws(() => validateGuideChanges('', 'new-script.sh\n'), /untracked/);
});
