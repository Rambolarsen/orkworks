import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const reporter = fileURLToPath(new URL('../crates/orkworksd/scripts/report-harness-event.ps1', import.meta.url));
const retained = JSON.parse(readFileSync(new URL('../docs/validation/antigravity-resume-1518-2026-11-27.json', import.meta.url), 'utf8'));
const cwd = '/fixture/workspace';
const identity = { harnessSessionId: 'fixture-conversation', source: 'antigravity_hook', confidence: 0.98, cwd };
const working = { status: 'working', source: 'antigravity_hook', event: 'PreInvocation', cwd };
const idle = { status: 'idle', source: 'antigravity_hook', event: 'Stop', cwd };

// This is reporter execution with intercepted HTTP, not a Windows agy session.
test('native Windows PowerShell replays retained Antigravity hooks silently', {
  skip: process.platform !== 'win32',
  timeout: 45_000,
}, (t) => {
  const cases = retained.phases.flatMap((phase) => phase.hooks.map((hook) => ({
    name: `${phase.result.phase}/${hook.event}`, event: hook.event, payload: hook.payload,
  })));
  assert.deepEqual(cases.map(({ name }) => name), [
    'fresh/SessionStart', 'fresh/PreInvocation', 'fresh/Stop',
    'continue/PreInvocation', 'continue/Stop',
    'exact-print/PreInvocation', 'exact-print/Stop',
  ]);
  const expected = [
    [identity], [identity, working], [idle],
    [identity, working], [idle], [identity, working], [idle],
  ];
  for (const [name, event, payload, reports] of [
    ['later invocation', 'PreInvocation', { conversationId: 'fixture-conversation', cwd, invocationNum: 1 }, [working]],
    ['busy stop', 'Stop', { conversationId: 'fixture-conversation', cwd, fullyIdle: false }, []],
    ['unknown event', 'PostInvocation', { conversationId: 'fixture-conversation', cwd }, []],
    ['unsafe identity', 'SessionStart', { conversationId: 'fixture\u001fforged', cwd }, []],
    ['Windows cwd', 'SessionStart', { conversationId: 'fixture-conversation', cwd: 'C:\\fixture\\workspace' }, [{ ...identity, cwd: 'C:\\fixture\\workspace' }]],
  ]) {
    cases.push({ name, event, payload });
    expected.push(reports);
  }

  const directory = mkdtempSync(join(tmpdir(), 'orkworks-antigravity-ps1-'));
  try {
    const casesPath = join(directory, 'cases.json');
    const capture = join(directory, 'requests.jsonl');
    const runtime = join(directory, 'runtime.json');
    const wrapper = join(directory, 'fixture.ps1');
    const progress = join(directory, 'progress.txt');
    writeFileSync(casesPath, JSON.stringify(cases));
    writeFileSync(capture, '');
    writeFileSync(progress, '');
    writeFileSync(wrapper, String.raw`
$ErrorActionPreference = 'Stop'
function Write-FixtureStage([string]$Stage) {
    [IO.File]::AppendAllText($env:ORKWORKS_PROGRESS_CAPTURE, ($Stage + [Environment]::NewLine))
}
Write-FixtureStage 'wrapper-start'
function Invoke-RestMethod {
    param($Method, $Uri, $Headers, $ContentType, $Body, $TimeoutSec)
    Write-FixtureStage "http-$global:fixtureIndex"
    $record = @{ index=$global:fixtureIndex; method=[string]$Method; uri=[string]$Uri; headers=$Headers; contentType=$ContentType; timeout=$TimeoutSec; body=($Body | ConvertFrom-Json) }
    [IO.File]::AppendAllText($env:ORKWORKS_REQUEST_CAPTURE, (($record | ConvertTo-Json -Depth 10 -Compress) + [Environment]::NewLine))
}
@{ version=$PSVersionTable.PSVersion.ToString(); edition=$PSVersionTable.PSEdition; is64Bit=[Environment]::Is64BitProcess } | ConvertTo-Json -Compress | Set-Content -LiteralPath $env:ORKWORKS_RUNTIME_CAPTURE -Encoding UTF8
$cases = Get-Content -LiteralPath $env:ORKWORKS_CASES -Raw -Encoding UTF8 | ConvertFrom-Json
Write-FixtureStage 'cases-loaded'
for ($i=0; $i -lt $cases.Count; $i++) {
    $global:fixtureIndex=$i
    $payload = $cases[$i].payload | ConvertTo-Json -Depth 10 -Compress
    Write-FixtureStage "stdin-$i"
    [Console]::SetIn([IO.StringReader]::new($payload))
    Write-FixtureStage "invoke-$i"
    & $env:ORKWORKS_REPORTER -Marker 'orkworks:harness-integration:v2:antigravity' -Event $cases[$i].event
    if ($LASTEXITCODE -ne 0) { throw 'reporter failed' }
    Write-FixtureStage "done-$i"
}
Write-FixtureStage 'wrapper-done'
`);
    // Keep OS startup paths, replacing every reporting capability with fixture data.
    const env = Object.fromEntries(Object.entries(process.env).filter(([key]) =>
      ['systemroot', 'windir', 'path', 'temp', 'tmp'].includes(key.toLowerCase())));
    Object.assign(env, {
      ORKWORKS_CASES: casesPath,
      ORKWORKS_REPORTER: reporter,
      ORKWORKS_REQUEST_CAPTURE: capture,
      ORKWORKS_RUNTIME_CAPTURE: runtime,
      ORKWORKS_PROGRESS_CAPTURE: progress,
      ORKWORKS_SESSION_ID: 'fixture-session',
      ORKWORKS_PORT: '1',
      ORKWORKS_REPORT_TOKEN: 'fixture-token',
    });
    const result = spawnSync('powershell.exe', [
      '-NoLogo', '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', wrapper,
    ], { env, encoding: 'utf8', timeout: 30_000, maxBuffer: 64 * 1024, windowsHide: true });
    const stages = existsSync(progress) ? readFileSync(progress, 'utf8').slice(-4096)
      .split(/\r?\n/).filter((line) => /^(wrapper-start|cases-loaded|wrapper-done|(?:stdin|invoke|http|done)-[0-9]{1,2})$/.test(line)) : [];
    t.diagnostic(`PowerShell stages: ${stages.join(', ') || 'none before process outcome'}`);
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout, '', 'hooks must not print agent instructions');
    assert.equal(result.stderr, '', 'hooks must remain silent');
    const native = JSON.parse(readFileSync(runtime, 'utf8').replace(/^\uFEFF/, ''));
    assert.equal(native.edition, 'Desktop', 'exercise native Windows PowerShell');
    t.diagnostic(`Windows PowerShell ${native.version}; 64-bit=${native.is64Bit}; 7 retained payloads and 5 controls`);

    const rows = readFileSync(capture, 'utf8').split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line));
    assert.equal(rows.length, expected.flat().length, 'no extra HTTP requests');
    for (const row of rows) {
      assert.ok(Number.isInteger(row.index) && row.index >= 0 && row.index < cases.length);
      assert.equal(row.method, 'Post');
      assert.deepEqual(row.headers, { Authorization: 'Bearer fixture-token' });
      assert.equal(row.contentType, 'application/json');
      assert.equal(row.timeout, 5);
      const route = row.body.status ? 'attention' : 'harness-session';
      assert.equal(row.uri, `http://127.0.0.1:1/sessions/fixture-session/${route}`);
      if (route === 'attention') {
        assert.match(row.body.observedAt, /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{6}Z$/);
        assert.ok(Number.isFinite(Date.parse(row.body.observedAt)));
        delete row.body.observedAt;
      }
    }
    for (let index = 0; index < cases.length; index++) {
      assert.deepEqual(rows.filter((row) => row.index === index).map((row) => row.body), expected[index], cases[index].name);
    }
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
