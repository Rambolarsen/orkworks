Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot 'pair-windows-toolchain-inventory.ps1'
if (-not (Test-Path -LiteralPath $script)) { throw 'Data-only inventory reader is not implemented' }
. $script
function Check($Value, [string]$Message) { if (-not $Value) { throw $Message } }
$targets = @(Get-InspectionTargets 'C:\Users\fixture')
Check ($targets.Count -eq 13) 'Inspection target count'
Check ($targets[2].path -ceq 'C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\cl.exe') 'Fixed compiler selection'
$temp = Join-Path ([IO.Path]::GetTempPath()) ('inventory-fixture-' + [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($temp) | Out-Null
try {
    $data = [byte[]]::new(128)
    $data[0] = 0x4d; $data[1] = 0x5a; $data[60] = 64
    $data[64] = 0x50; $data[65] = 0x45; $data[68] = 0x64; $data[69] = 0x86
    $file = Join-Path $temp 'data.bin'
    [IO.File]::WriteAllBytes($file, $data)
    $budget = @{ maximum = 4096L; hashed_bytes = 0L; hash_passes = 0 }
    $record = Read-InspectionFile 'fixture' $file $budget
    Check ($record.status -eq 'ok' -and $record.machine -eq '8664' -and $record.bytes -eq 128) 'PE metadata must be read as data'
    Check ($budget.hashed_bytes -eq 128 -and $budget.hash_passes -eq 1) 'Hash charge'
    $initial = $record.sha256
    Update-FileRecheck $record $budget
    Check ($record.final_sha256 -eq $initial -and $budget.hashed_bytes -eq 256) 'Identity recheck'
    $data[100] = 1; [IO.File]::WriteAllBytes($file, $data)
    Update-FileRecheck $record $budget
    Check ($record.final_sha256 -ne $initial -and $record.status -eq 'drift') 'Drift must be visible'
    $small = @{ maximum = 127L; hashed_bytes = 0L; hash_passes = 0 }
    $limited = Read-InspectionFile 'fixture' $file $small
    Check ($limited.status -eq 'hash-budget' -and $small.hashed_bytes -eq 0) 'Over-budget input must not be hashed'
    $missing = Read-InspectionFile 'missing' (Join-Path $temp 'absent') $budget
    Check ($missing.status -eq 'missing') 'Missing file observation'
    $bad = Join-Path $temp 'not-pe.txt'; [IO.File]::WriteAllText($bad, 'data only')
    $badRecord = Read-InspectionFile 'fixture' $bad $budget
    Check ($badRecord.status -eq 'invalid-pe') 'Malformed PE cannot supply machine metadata'
    $text = Join-Path $temp 'default.txt'; [IO.File]::WriteAllText($text, '14.44.35207')
    $default = Read-DefaultToolset $text $budget
    Check ($default.status -eq 'ok' -and $default.value -ceq '14.44.35207') 'Bounded default text'
    [IO.File]::WriteAllText($text, ('x' * 4097))
    Check ((Read-DefaultToolset $text $budget).status -eq 'text-limit') 'Oversize text must be denied'
    $env:GITHUB_EVENT_NAME = 'pull_request'
    $denied = $false
    try { & $script } catch { $denied = $_.Exception.Message -eq 'Only the approved first Windows inventory attempt may run' }
    Check $denied 'Non-dispatch entry must deny before inspecting installed files'
    Write-Output 'Data-only Windows fixture checks passed; zero installed-tool inspection or execution.'
}
finally { [IO.Directory]::Delete($temp, $true) }
