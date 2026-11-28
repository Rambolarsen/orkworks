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

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class AppxTestActivation
{
    [ComImport, Guid("2E941141-7F97-4756-BA1D-9DECDE894A3D"),
     InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IApplicationActivationManager
    {
        [PreserveSig]
        int ActivateApplication(
            [MarshalAs(UnmanagedType.LPWStr)] string appUserModelId,
            [MarshalAs(UnmanagedType.LPWStr)] string arguments,
            uint options, out uint processId);
    }

    public static int Start(string appUserModelId, string arguments)
    {
        var type = Type.GetTypeFromCLSID(new Guid("45BA127D-10A8-46EA-8AB7-56EA9078943C"));
        var manager = (IApplicationActivationManager)Activator.CreateInstance(type);
        try
        {
            uint processId;
            Marshal.ThrowExceptionForHR(manager.ActivateApplication(appUserModelId, arguments, 0, out processId));
            return checked((int)processId);
        }
        finally
        {
            Marshal.ReleaseComObject(manager);
        }
    }
}
'@

function Test-InstalledDesktop {
    param($Installation, [string]$Workspace, [string]$RetainedSessionId)

    $listener = New-Object System.Net.Sockets.TcpListener ([Net.IPAddress]::Loopback, 0)
    $listener.Start()
    try {
        $port = $listener.LocalEndpoint.Port
    } finally {
        $listener.Stop()
    }

    $appUserModelId = "$($Installation.PackageFamilyName)!OrkWorksTest"
    $appProcessId = [AppxTestActivation]::Start($appUserModelId, "--remote-debugging-address=127.0.0.1 --remote-debugging-port=$port")
    $process = Get-Process -Id $appProcessId
    try {
        for ($attempt = 0; $attempt -lt 60; $attempt++) {
            $process.Refresh()
            if ($process.HasExited) { throw 'The installed AppX exited before showing its desktop window' }
            if ($process.MainWindowHandle -ne 0) { break }
            Start-Sleep -Milliseconds 500
        }
        if ($process.MainWindowHandle -eq 0) { throw 'The installed AppX did not show a desktop window' }
        Write-Host "AppX $($Installation.Version) opened an OrkWorks desktop window."

        $arguments = @("$PSScriptRoot/windows-appx-renderer-probe.mjs", "$port", $Workspace)
        if ($RetainedSessionId) { $arguments += $RetainedSessionId }
        Invoke-Checked node $arguments
    } finally {
        if (-not $process.HasExited) {
            $process.CloseMainWindow() | Out-Null
            if (-not $process.WaitForExit(10000)) {
                Stop-Process -Id $appProcessId
                $process.WaitForExit()
                throw 'The installed AppX did not shut down cleanly'
            }
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

    $workspace = Join-Path $env:RUNNER_TEMP 'orkworks-appx-workspace'
    New-Item -Path $workspace -ItemType Directory | Out-Null
    $workspace = (Get-Item -LiteralPath $workspace).FullName
    $retainedSessionId = Test-InstalledDesktop $first $workspace
    if (-not $retainedSessionId) { throw 'The desktop probe did not return a retained session ID' }
    Write-Host 'The installed desktop opened the workspace and started and ended a shell session.'

    $second = Install-TestPackage (Join-Path $output 'OrkWorks-AppX-Test-2.appx')
    if ($second.PackageFamilyName -ne $first.PackageFamilyName -or $second.Version -le $first.Version) {
        throw 'The second package did not upgrade the same AppX identity'
    }
    Assert-BundledFiles $second
    Write-Host "Upgraded the same AppX identity to $($second.Version)."
    Test-InstalledDesktop $second $workspace $retainedSessionId
    Write-Host 'The upgraded desktop remembered the workspace and recovered the ended session.'
} finally {
    Get-AppxPackage -Name $identity | Remove-AppxPackage -ErrorAction Continue
    if (Test-Path -LiteralPath $trustedPath) { Remove-Item -LiteralPath $trustedPath }
    Remove-Item -LiteralPath ("Cert:\CurrentUser\My\" + $certificate.Thumbprint)
}
