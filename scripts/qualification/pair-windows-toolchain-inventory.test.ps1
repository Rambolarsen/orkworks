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
    # Hand-built PE resource data. This is never compiled, loaded or executed.
    $versioned = [byte[]]::new(2048)
    function Put16([int]$Offset, [uint16]$Value) { [BitConverter]::GetBytes($Value).CopyTo($versioned, $Offset) }
    function Put32([int]$Offset, [uint32]$Value) { [BitConverter]::GetBytes($Value).CopyTo($versioned, $Offset) }
    Put16 0 0x5a4d; Put32 60 64; Put32 64 0x4550
    Put16 68 0x8664; Put16 70 1; Put16 84 240; Put16 88 0x20b
    Put32 196 16; Put32 216 4096; Put32 220 512
    Put32 336 512; Put32 340 4096; Put32 344 512; Put32 348 512
    Put16 526 1; Put32 528 16; Put32 532 2147483672
    Put16 550 1; Put32 552 1; Put32 556 2147483696
    Put16 574 1; Put32 576 1033; Put32 580 72
    Put32 584 4352; Put32 588 92
    Put16 768 92; Put16 770 52; Put16 772 0
    ([Text.Encoding]::Unicode.GetBytes("VS_VERSION_INFO" + [char]0)).CopyTo($versioned, 774)
    Put32 808 4277077181; Put32 812 65536
    Put32 816 ((19L -shl 16) + 44); Put32 820 (35231L -shl 16)
    Put32 824 ((14L -shl 16) + 44); Put32 828 (35231L -shl 16)
    $memory = [IO.MemoryStream]::new($versioned, $false)
    try {
        $embedded = Get-EmbeddedVersion $memory 64
        Check ($embedded.file_version -ceq '19.44.35231.0' -and $embedded.product_version -ceq '14.44.35231.0') 'Version numbers must come from embedded fixed file info'
        $versionFile = Join-Path $temp 'version-data.bin'
        [IO.File]::WriteAllBytes($versionFile, $versioned)
        $fromFile = Read-InspectionFile 'fixture' $versionFile $budget
        Check ($fromFile.status -eq 'ok' -and $fromFile.file_version -ceq '19.44.35231.0') 'File reader must use embedded version data'
        Put32 584 4294967295
        $malformed = Get-EmbeddedVersion $memory 64
        Check ($null -eq $malformed) 'Out-of-range resource RVA must not be followed'
    }
    finally { $memory.Dispose() }
    $env:GITHUB_EVENT_NAME = 'pull_request'
    $denied = $false
    try { & $script } catch { $denied = $_.Exception.Message -eq 'Only the approved first Windows inventory attempt may run' }
    Check $denied 'Non-dispatch entry must deny before inspecting installed files'
    Write-Output 'Data-only Windows fixture checks passed; zero installed-tool inspection or execution.'
}
finally { [IO.Directory]::Delete($temp, $true) }
