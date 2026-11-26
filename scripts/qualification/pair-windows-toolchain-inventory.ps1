param([switch]$Inventory)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-InspectionTargets([string]$Profile) {
    $vs = 'C:\Program Files\Microsoft Visual Studio\2022\Enterprise'
    $groups = @(
        @{ root = "$Profile\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin"; names = @('cargo.exe','rustc.exe') },
        @{ root = "$vs\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64"; names = @('cl.exe','link.exe','lib.exe','dumpbin.exe','c1.dll','c1xx.dll','c2.dll','mspdbcore.dll') },
        @{ root = 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64'; names = @('rc.exe','mt.exe') },
        @{ root = "$vs\Common7\IDE"; names = @('devenv.exe') }
    )
    foreach ($group in $groups) { foreach ($name in $group.names) { @{ name = $name; path = [IO.Path]::Combine($group.root, $name) } } }
}
function Assert-PlainPath([string]$Path, [bool]$Directory = $false) {
    $item = if ($Directory) { [IO.DirectoryInfo]::new($Path) } else { [IO.FileInfo]::new($Path) }
    if (-not $item.Exists) { throw 'missing' }
    $current = $item
    while ($null -ne $current) {
        if (($current.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'reparse' }
        $current = if ($current -is [IO.FileInfo]) { $current.Directory } else { $current.Parent }
    }
    return $item
}
function Convert-ResourceText($Value) {
    if ([string]::IsNullOrEmpty($Value)) { return $null }
    $text = $Value.Trim()
    if ($text.Length -gt 128 -or $text -cnotmatch '\A[\x20-\x7e]+\z') { throw 'resource-limit' }
    return $text
}
function Read-DataFingerprint([string]$Name, [string]$Path, [hashtable]$Budget, [string]$Mode = 'Hash') {
    $record = @{ name = $Name; path = $Path; status = 'unavailable'; bytes = $null; sha256 = $null; final_sha256 = $null; machine = $null; file_version = $null; product_version = $null; version_status = 'not-read' }
    $stream = $null; $algorithm = $null; $reader = $null
    try {
        $item = Assert-PlainPath $Path
        $record.bytes = $item.Length
        $maximum = if ($Mode -eq 'Text') { 4096L } else { 268435456L }
        if ($item.Length -gt $maximum) { if ($Mode -eq 'Text') { throw 'text-limit' } else { throw 'file-limit' } }
        $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
        $record.bytes = $stream.Length
        if ($stream.Length -gt $maximum) { throw 'file-limit' }
        if ($Budget.hashed_bytes + $stream.Length -gt $Budget.maximum) { throw 'hash-budget' }
        $algorithm = [Security.Cryptography.SHA256]::Create()
        $record.sha256 = [Convert]::ToHexString($algorithm.ComputeHash($stream)).ToLowerInvariant()
        $Budget.hashed_bytes += $stream.Length
        $Budget.hash_passes++
        $stream.Position = 0
        if ($Mode -eq 'PE') {
            if ($stream.Length -lt 70) { throw 'invalid-pe' }
            $reader = [IO.BinaryReader]::new($stream, [Text.Encoding]::UTF8, $true)
            if ($reader.ReadUInt16() -ne 0x5a4d) { throw 'invalid-pe' }
            $stream.Position = 60
            $offset = $reader.ReadUInt32()
            if ($offset -lt 64 -or $offset -gt $stream.Length - 6) { throw 'invalid-pe' }
            $stream.Position = $offset
            if ($reader.ReadUInt32() -ne 0x00004550) { throw 'invalid-pe' }
            $record.machine = '{0:x4}' -f $reader.ReadUInt16()
            try {
                $version = [Diagnostics.FileVersionInfo]::GetVersionInfo($Path)
                $record.file_version = Convert-ResourceText $version.FileVersion
                $record.product_version = Convert-ResourceText $version.ProductVersion
                $record.version_status = 'observed'
            }
            catch { $record.file_version = $null; $record.product_version = $null; $record.version_status = 'unavailable' }
        }
        elseif ($Mode -eq 'Text') {
            $textReader = [IO.StreamReader]::new($stream, [Text.Encoding]::UTF8, $true, 1024, $true)
            try { $value = $textReader.ReadToEnd().Trim() } finally { $textReader.Dispose() }
            if ($value -cnotmatch '\A[0-9]+(?:\.[0-9]+){1,3}\z' -or $value.Length -gt 128) { throw 'text-format' }
            $record.value = $value
        }
        $record.status = 'ok'
    }
    catch {
        $message = $_.Exception.Message
        $record.status = if ($message -cmatch '\A(?:missing|reparse|text-limit|file-limit|hash-budget|invalid-pe|text-format)\z') { $message } else { 'io-unavailable' }
    }
    finally {
        if ($null -ne $reader) { $reader.Dispose() }
        if ($null -ne $algorithm) { $algorithm.Dispose() }
        if ($null -ne $stream) { $stream.Dispose() }
    }
    return $record
}
function Read-InspectionFile([string]$Name, [string]$Path, [hashtable]$Budget) { Read-DataFingerprint $Name $Path $Budget 'PE' }
function Read-DefaultToolset([string]$Path, [hashtable]$Budget) { Read-DataFingerprint 'default-toolset' $Path $Budget 'Text' }
function Update-FileRecheck([hashtable]$Record, [hashtable]$Budget) {
    if ($null -eq $Record.sha256) { return }
    $again = Read-DataFingerprint $Record.name $Record.path $Budget
    $Record.final_sha256 = $again.sha256
    if ($again.status -ne 'ok') { $Record.status = $again.status }
    elseif ($again.sha256 -cne $Record.sha256) { $Record.status = 'drift' }
}
function Read-SdkInventory {
    $root = 'C:\Program Files (x86)\Windows Kits\10'
    $result = @{ status = 'unavailable'; root = $null; directories = @(); scanned_entries = 0; products = @(); available_entries = 0 }
    $machine = $null; $keys = @()
    try {
        $machine = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry64)
        $installed = $machine.OpenSubKey('SOFTWARE\Microsoft\Windows Kits\Installed Roots')
        if ($null -eq $installed) { throw 'sdk-root' }
        try { $result.root = Convert-ResourceText $installed.GetValue('KitsRoot10') } finally { $installed.Dispose() }
        foreach ($dir in @("$root\Include\10.0.26100.0", "$root\Lib\10.0.26100.0", "$root\bin\10.0.26100.0")) {
            try { $null = Assert-PlainPath $dir $true; $result.directories += $true } catch { $result.directories += $false }
        }
        foreach ($name in @('SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall','SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall')) {
            $key = $machine.OpenSubKey($name)
            if ($null -eq $key) { throw 'registry-root' }
            $keys += $key
            $result.available_entries += $key.SubKeyCount
        }
        if ($result.available_entries -gt 256) { throw 'registry-budget' }
        foreach ($key in $keys) {
            foreach ($name in $key.GetSubKeyNames()) {
                if ($result.scanned_entries -ge 256) { throw 'registry-budget' }
                $result.scanned_entries++
                $product = $key.OpenSubKey($name)
                if ($null -eq $product) { throw 'registry-drift' }
                try {
                    $display = $product.GetValue('DisplayName')
                    if ($display -is [string] -and $display -like '*Software Development Kit*') {
                        $result.products += @{ name = Convert-ResourceText $display; version = Convert-ResourceText $product.GetValue('DisplayVersion') }
                    }
                }
                finally { $product.Dispose() }
            }
        }
        $result.status = 'ok'
    }
    catch {
        $message = $_.Exception.Message
        $result.status = if ($message -cmatch '\A(?:sdk-root|registry-root|registry-budget|registry-drift|resource-limit)\z') { $message } else { 'registry-unavailable' }
    }
    finally { foreach ($key in $keys) { $key.Dispose() }; if ($null -ne $machine) { $machine.Dispose() } }
    return $result
}
function Assert-InventoryEntry {
    if (-not $IsWindows -or $env:GITHUB_ACTIONS -cne 'true' -or $env:GITHUB_EVENT_NAME -cne 'workflow_dispatch' -or $env:GITHUB_RUN_ATTEMPT -cne '1' -or $env:EXPECTED_SHA -cnotmatch '\A[0-9a-f]{40}\z') {
        throw 'Only the approved first Windows inventory attempt may run'
    }
}
function Invoke-DataInventory {
    Assert-InventoryEntry
    if ($env:ORK_PAIR_INVENTORY_CHILD -cne '1' -or $env:ORK_PAIR_HASH_BUDGET -cnotmatch '\A[0-9]{1,10}\z') { throw 'Controller-only inventory entry' }
    $maximum = [long]$env:ORK_PAIR_HASH_BUDGET
    if ($maximum -lt 0 -or $maximum -gt 1073741824L) { throw 'Invalid hash allowance' }
    $budget = @{ maximum = $maximum; hashed_bytes = 0L; hash_passes = 0 }
    $records = @()
    foreach ($target in @(Get-InspectionTargets $env:USERPROFILE)) {
        $records += Read-InspectionFile $target.name $target.path $budget
    }
    $default = Read-DefaultToolset 'C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\Microsoft.VCToolsVersion.default.txt' $budget
    $sdk = Read-SdkInventory
    foreach ($record in $records) { Update-FileRecheck $record $budget }
    Update-FileRecheck $default $budget
    @{ schema = 1; files = $records; default_toolset = $default; sdk = $sdk; hashed_bytes = $budget.hashed_bytes; hash_passes = $budget.hash_passes } | Microsoft.PowerShell.Utility\ConvertTo-Json -Depth 8 -Compress
}
if ($MyInvocation.InvocationName -ne '.') {
    Assert-InventoryEntry
    if ($Inventory) { Invoke-DataInventory }
    else {
        $actual = (git rev-parse HEAD).Trim()
        if ($LASTEXITCODE -ne 0 -or $actual -cne $env:EXPECTED_SHA) { throw 'Reviewed inventory checkout mismatch' }
        $env:ORK_PAIR_PWSH = [IO.Path]::Combine($PSHOME, 'pwsh.exe')
        & node.exe ([IO.Path]::Combine($PSScriptRoot, 'pair-windows-toolchain-inventory.mjs'))
        if ($LASTEXITCODE -ne 0) { throw 'Inventory unavailable; no retry authorized' }
    }
}
