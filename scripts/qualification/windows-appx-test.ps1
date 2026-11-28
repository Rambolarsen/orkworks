# This fixture uses a disposable runner and a short-lived test certificate.
# The private key stays in the runner's certificate store.
$ErrorActionPreference = 'Stop'
if (-not $env:RUNNER_TEMP -or $env:GITHUB_ACTIONS -ne 'true') {
    throw 'Run this qualification only on a disposable GitHub Actions Windows runner.'
}
$identity = 'OrkWorks.AppX.Test'
$subject = 'CN=OrkWorks AppX Test'
$output = Join-Path $env:RUNNER_TEMP 'orkworks-appx-test'
New-Item -Path $output -ItemType Directory | Out-Null

function Invoke-Checked {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Program failed with exit code $LASTEXITCODE"
    }
}

function Install-TestPackage {
    param([string]$Path)
    Add-AppxPackage -Path $Path
    $installed = Get-AppxPackage -Name $identity
    if ($null -eq $installed) { throw "AppX identity $identity was not installed" }
    return $installed
}

function Assert-BundledFiles {
    param($Installation)
    $resources = Join-Path $Installation.InstallLocation 'app/resources'
    foreach ($relative in @('app.asar', 'orkworksd.exe', 'scripts/report-harness-event.ps1', 'knowledge/starter.json')) {
        if (-not (Test-Path -LiteralPath (Join-Path $resources $relative) -PathType Leaf)) {
            throw "Installed resource is missing: $relative"
        }
    }
}

$kit = Get-ChildItem -Path "${env:ProgramFiles(x86)}/Windows Kits/10/bin/*/x64/signtool.exe" |
    Sort-Object FullName | Select-Object -Last 1
if (-not $kit) { throw 'Windows SDK SignTool is unavailable' }

$keyParameters = @{
    Type = 'CodeSigningCert'
    Subject = $subject
    CertStoreLocation = 'Cert:\CurrentUser\My'
    KeyExportPolicy = 'NonExportable'
    NotAfter = (Get-Date).AddDays(14)
}
$certificate = New-SelfSignedCertificate @keyParameters
$publicCertificate = Join-Path $output 'OrkWorks-AppX-Test.cer'
$trustedPath = "Cert:\LocalMachine\TrustedPeople\$($certificate.Thumbprint)"

try {
    Export-Certificate -Cert $certificate -FilePath $publicCertificate | Out-Null
    Import-Certificate -FilePath $publicCertificate -CertStoreLocation 'Cert:\LocalMachine\TrustedPeople' | Out-Null

    Push-Location "$PSScriptRoot/../../apps/desktop"
    try {
        # Reuse the release layout with a disposable test identity and output.
        $env:ORKWORKS_APPX_TEST_OUTPUT = $output
        $env:ORKWORKS_APPX_TEST_SUBJECT = $subject
        $env:ORKWORKS_APPX_TEST_IDENTITY = $identity
        $config = @'
const fs = require("node:fs");
const yaml = require("js-yaml");
const config = yaml.load(fs.readFileSync("electron-builder.yml", "utf8"));
config.publish = null;
config.win.publish = null;
config.win.forceCodeSigning = false;
config.npmRebuild = false;
config.directories.output = process.env.ORKWORKS_APPX_TEST_OUTPUT;
config.appx = {
  identityName: process.env.ORKWORKS_APPX_TEST_IDENTITY,
  applicationId: "OrkWorksTest",
  displayName: "OrkWorks AppX Test",
  publisher: process.env.ORKWORKS_APPX_TEST_SUBJECT,
  publisherDisplayName: "OrkWorks AppX Test",
  artifactName: "OrkWorks-AppX-Test.appx",
  electronUpdaterAware: false,
  setBuildNumber: true,
};
fs.writeFileSync("appx-test.ci.json", JSON.stringify(config));
'@
        Set-Content -LiteralPath 'appx-test.ci.cjs' -Value $config -Encoding ASCII
        Invoke-Checked node @('appx-test.ci.cjs')
        foreach ($build in @(1, 2)) {
            $env:BUILD_NUMBER = "$build"
            Invoke-Checked pnpm @('exec', 'electron-builder', '--win', 'appx', '--x64', '--publish', 'never', '--config', 'appx-test.ci.json')
            $built = Join-Path $output 'OrkWorks-AppX-Test.appx'
            $package = Join-Path $output "OrkWorks-AppX-Test-$build.appx"
            Invoke-Checked $kit.FullName @('sign', '/fd', 'SHA256', '/sha1', $certificate.Thumbprint, $built)
            Move-Item -LiteralPath $built -Destination $package
        }
    } finally {
        Pop-Location
    }

    $first = Install-TestPackage (Join-Path $output 'OrkWorks-AppX-Test-1.appx')
    Assert-BundledFiles $first
    Write-Host "Installed AppX $($first.Version) with expected resources."

    $stdout = Join-Path $env:RUNNER_TEMP 'orkworks-appx-sidecar.stdout'
    $rendererToken = [guid]::NewGuid().ToString('N')
    $workspace = Join-Path $env:RUNNER_TEMP 'orkworks-appx-workspace'
    New-Item -Path $workspace -ItemType Directory | Out-Null
    $start = @{
        FilePath = (Join-Path $first.InstallLocation 'app/resources/orkworksd.exe')
        WorkingDirectory = $env:RUNNER_TEMP
        PassThru = $true
        RedirectStandardOutput = $stdout
    }
    $env:ORKWORKS_RENDERER_TOKEN = $rendererToken
    $env:PEON_ENABLED = 'false'
    try {
        $sidecar = Start-Process @start
    } finally {
        Remove-Item Env:ORKWORKS_RENDERER_TOKEN, Env:PEON_ENABLED
    }
    try {
        $port = $null
        for ($attempt = 0; $attempt -lt 50 -and -not $port; $attempt++) {
            if ($sidecar.HasExited) { throw 'Installed sidecar exited before binding' }
            if ((Get-Content -LiteralPath $stdout -Raw) -match 'ORKWORKSD_PORT=(\d+)') { $port = $Matches[1] }
            if (-not $port) { Start-Sleep -Milliseconds 200 }
        }
        if (-not $port) { throw 'Installed sidecar did not announce a port' }
        $health = Invoke-WebRequest -Uri "http://127.0.0.1:$port/health" -TimeoutSec 5
        if ($health.StatusCode -ne 200) { throw 'Installed sidecar did not respond to its health probe' }
        Write-Host 'Installed sidecar answered its health endpoint.'

        $api = "http://127.0.0.1:$port"
        $headers = @{ 'X-OrkWorks-Renderer-Token' = $rendererToken }
        $workspaceBody = @{ path = $workspace; workspaceIdentity = $workspace } | ConvertTo-Json
        Invoke-RestMethod -Method Post -Uri "$api/workspace" -Headers $headers -ContentType 'application/json' -Body $workspaceBody | Out-Null
        $sessionBody = @{ harnessId = 'generic-shell'; expectedWorkspacePath = $workspace } | ConvertTo-Json
        $session = Invoke-RestMethod -Method Post -Uri "$api/sessions" -Headers $headers -ContentType 'application/json' -Body $sessionBody
        try {
            $ready = $false
            for ($attempt = 0; $attempt -lt 50 -and -not $ready; $attempt++) {
                $sessions = Invoke-RestMethod -Uri "$api/sessions" -Headers $headers -TimeoutSec 5
                $ready = [bool]($sessions | Where-Object { $_.id -eq $session.id -and $_.lifecycle -eq 'alive' })
                if (-not $ready) { Start-Sleep -Milliseconds 200 }
            }
            if (-not $ready) { throw 'Installed sidecar did not start the test shell session' }
            Write-Host 'Installed sidecar opened the disposable workspace and started a shell session.'
        } finally {
            Invoke-RestMethod -Method Delete -Uri "$api/sessions/$($session.id)" -Headers $headers -TimeoutSec 5 | Out-Null
        }
    } finally {
        if (-not $sidecar.HasExited) { Stop-Process -Id $sidecar.Id }
        $sidecar.WaitForExit(5000) | Out-Null
    }

    $statePath = Join-Path $env:LOCALAPPDATA "Packages/$($first.PackageFamilyName)/LocalState"
    New-Item -Path $statePath -ItemType Directory -Force | Out-Null
    $marker = Join-Path $statePath 'test-upgrade-marker.txt'
    Set-Content -LiteralPath $marker -Value 'preserve this state'
    $second = Install-TestPackage (Join-Path $output 'OrkWorks-AppX-Test-2.appx')
    if ($second.PackageFamilyName -ne $first.PackageFamilyName -or $second.Version -le $first.Version) {
        throw 'The second package did not upgrade the same AppX identity'
    }
    Assert-BundledFiles $second
    if ((Get-Content -LiteralPath $marker -Raw).Trim() -ne 'preserve this state') {
        throw 'Package LocalState did not survive the upgrade'
    }
    Write-Host "Upgraded to $($second.Version) and preserved package LocalState."

    $installedApp = Join-Path $second.InstallLocation 'app/OrkWorks.exe'
    Start-Process explorer.exe -ArgumentList "shell:AppsFolder\$($second.PackageFamilyName)!OrkWorksTest"
    $window = $null
    try {
        for ($attempt = 0; $attempt -lt 50 -and -not $window; $attempt++) {
            $window = Get-Process -Name OrkWorks -ErrorAction SilentlyContinue |
                Where-Object { $_.Path -eq $installedApp -and $_.MainWindowHandle -ne 0 } |
                Select-Object -First 1
            if (-not $window) { Start-Sleep -Milliseconds 500 }
        }
        if (-not $window) { throw 'The runner did not observe an installed AppX desktop window' }
        Write-Host 'The upgraded AppX opened an OrkWorks desktop window.'
    } finally {
        if ($window) {
            $window.CloseMainWindow() | Out-Null
            if (-not $window.WaitForExit(5000)) { Stop-Process -Id $window.Id }
        }
    }
} finally {
    Get-AppxPackage -Name $identity | Remove-AppxPackage -ErrorAction Continue
    if (Test-Path -LiteralPath $trustedPath) { Remove-Item -LiteralPath $trustedPath }
    Remove-Item -LiteralPath ("Cert:\CurrentUser\My\" + $certificate.Thumbprint)
}
