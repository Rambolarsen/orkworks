param([switch]$Inventory)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows -or $env:GITHUB_ACTIONS -cne 'true' -or $env:GITHUB_EVENT_NAME -cne 'workflow_dispatch' -or $env:GITHUB_RUN_ATTEMPT -cne '1' -or $env:EXPECTED_SHA -cnotmatch '\A[0-9a-f]{40}\z') {
    throw 'Only the approved first Windows fixed-build attempt may run'
}
if ($Inventory) {
    $vc = 'C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.44.35207'
    $kit = 'C:\Program Files (x86)\Windows Kits\10'
    $installed = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots').KitsRoot10.TrimEnd([char]92)
    if ($installed -cne $kit) { throw 'Named SDK root mismatch' }
    $products = @(Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object {
        $_.PSObject.Properties.Name -contains 'DisplayName' -and $_.PSObject.Properties.Name -contains 'DisplayVersion' -and
        $_.DisplayName -like '*Software Development Kit*' -and $_.DisplayVersion -eq '10.1.26100.9457'
    })
    if ($products.Count -eq 0) { throw 'Named SDK servicing package unavailable' }
    foreach ($name in @('cl.exe','link.exe','lib.exe','dumpbin.exe')) { if (-not (Test-Path -LiteralPath "$vc\bin\Hostx64\x64\$name" -PathType Leaf)) { throw 'Named MSVC helper unavailable' } }
    @{ sdk_service = '10.1.26100.9457'; sdk = '10.0.26100.0'; compiler_version = (Get-Item -LiteralPath "$vc\bin\Hostx64\x64\cl.exe").VersionInfo.FileVersion; linker_version = (Get-Item -LiteralPath "$vc\bin\Hostx64\x64\link.exe").VersionInfo.FileVersion; sdk_products = @($products | ForEach-Object { @{ name = $_.DisplayName; version = $_.DisplayVersion } }) } | ConvertTo-Json -Depth 5 -Compress
    exit 0
}
$actual = (git rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $actual -cne $env:EXPECTED_SHA) { throw 'Reviewed fixed-build checkout mismatch' }
$env:ORK_PAIR_PWSH = Join-Path $PSHOME 'pwsh.exe'
& node.exe "$PSScriptRoot/pair-fixed-windows-build.mjs"
if ($LASTEXITCODE -ne 0) { throw 'Fixed build unavailable; no retry authorized' }
