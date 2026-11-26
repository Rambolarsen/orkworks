# A thin entry point: all policy/process tests run against the same Node helper.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows -or $env:GITHUB_EVENT_NAME -cne 'workflow_dispatch' -or
    $env:GITHUB_RUN_ATTEMPT -cne '1' -or $env:EXPECTED_SHA -cnotmatch '\A[0-9a-f]{40}\z') {
    throw 'Only the approved first Windows metadata attempt may run'
}
$actual = (git rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $actual -cne $env:EXPECTED_SHA) { throw 'Reviewed checkout mismatch' }
& node.exe "$PSScriptRoot/pair-windows-metadata-evidence.mjs"
if ($LASTEXITCODE -ne 0) { throw 'Metadata unavailable; no retry authorized' }
