import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
const require = createRequire(new URL('../../docs/package.json', import.meta.url));
const { parse } = require('yaml');
const workflowPath = new URL('../../.github/workflows/pair-sqlite-compile-evidence.yml', import.meta.url);
const driverPath = new URL('./pair-sqlite-compile-evidence.ps1', import.meta.url);

test('qualification is manual-only with one bounded read-only Windows job', () => {
  assert.ok(existsSync(workflowPath), 'approved compile-only workflow is missing');
  const workflow = parse(readFileSync(workflowPath, 'utf8'));
  assert.deepEqual(Object.keys(workflow.on), ['workflow_dispatch']);
  assert.deepEqual(workflow.permissions, { contents: 'read' });
  assert.deepEqual(Object.keys(workflow.jobs), ['compile']);
  const job = workflow.jobs.compile;
  assert.equal(job['runs-on'], 'windows-2022');
  assert.equal(job['timeout-minutes'], 15);
  assert.equal(job.strategy, undefined);
  const steps = job.steps;
  const checkout = steps.find(s => s.uses?.startsWith('actions/checkout@'));
  assert.equal(checkout.with['persist-credentials'], false);
  assert.ok(steps.some(s => s.run?.includes('GITHUB_RUN_ATTEMPT')));
  assert.ok(steps.some(s => s.run?.includes('git rev-parse HEAD') && s.run.includes('EXPECTED_SHA')));
  assert.equal(steps.filter(s => s.run?.includes('pair-sqlite-compile-evidence.ps1')).length, 1);
  assert.equal(steps.filter(s => s.uses?.includes('cache')).length, 0);
});

test('artifact upload has a literal allowlist and one-day retention', () => {
  const workflow = parse(readFileSync(workflowPath, 'utf8'));
  const upload = workflow.jobs.compile.steps.find(s => s.uses?.startsWith('actions/upload-artifact@'));
  assert.equal(upload.if, 'always()');
  assert.equal(upload.with['retention-days'], 1);
  const paths = upload.with.path.trim().split(/\r?\n/).map(s => s.trim());
  assert.deepEqual(paths.map(s => s.split('/').at(-1)), ['result.json', 'diagnostics.txt', 'sqlite3.asm']);
  assert.ok(paths.every(s => !/[?*]/.test(s)));
});

test('driver contains one literal compiler invocation, no linker and no run target', () => {
  const source = readFileSync(driverPath, 'utf8');
  assert.equal((source.match(/& \$compiler @arguments/g) ?? []).length, 1);
  assert.doesNotMatch(source, /(?:Start-Process|Invoke-Expression|Add-Type|link\.exe|cargo (?:test|run|build))/i);
  assert.match(source, /Remove-Item Env:CL, Env:_CL_/);
  const archiveCheck = source.indexOf('Assert-PinnedFile $archive');
  const extract = source.indexOf('& tar.exe');
  const sourceCheck = source.indexOf('Assert-PinnedFile $source');
  const compile = source.indexOf('& $compiler @arguments');
  assert.ok(archiveCheck >= 0 && archiveCheck < extract && extract < sourceCheck && sourceCheck < compile);
});

test('literal driver argv preserves the exact approved candidate configuration', () => {
  const source = readFileSync(driverPath, 'utf8');
  const body = source.match(/function Get-CompileArguments {[\s\S]*?return @\(([\s\S]*?)\n    \)/)?.[1];
  assert.ok(body, 'compiler arguments must remain a literal array');
  const actual = [...body.matchAll(/'([^']*)'/g)].map(m => m[1]);
  assert.deepEqual(actual, ["/nologo","/c","/TC","/O2","/MD","/W3","/FAsu","/Fasqlite3.asm","/Fosqlite3.obj","/DSQLITE_CORE","/DSQLITE_DEFAULT_FOREIGN_KEYS=1","/DSQLITE_ENABLE_API_ARMOR","/DSQLITE_ENABLE_COLUMN_METADATA","/DSQLITE_ENABLE_DBSTAT_VTAB","/DSQLITE_ENABLE_FTS3","/DSQLITE_ENABLE_FTS3_PARENTHESIS","/DSQLITE_ENABLE_FTS5","/DSQLITE_ENABLE_JSON1","/DSQLITE_ENABLE_LOAD_EXTENSION=1","/DSQLITE_ENABLE_MEMORY_MANAGEMENT","/DSQLITE_ENABLE_RTREE","/DSQLITE_ENABLE_STAT4","/DSQLITE_SOUNDEX","/DSQLITE_THREADSAFE=1","/DSQLITE_USE_URI","/DHAVE_USLEEP=1","/DHAVE_ISNAN","/D_POSIX_THREAD_SAFE_FUNCTIONS","sqlite3.c"]);
});
