# One approved compile-only attempt. Dot-sourcing loads compiler-free test helpers.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-PinnedFile([string]$Path, [string]$ExpectedHash) {
    if ($ExpectedHash -cnotmatch '\A[0-9a-f]{64}\z' -or
        -not (Test-Path -LiteralPath $Path -PathType Leaf) -or
        (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $ExpectedHash) {
        throw 'Pinned file identity mismatch'
    }
}

function Assert-NumericIdentity([string]$Value) {
    if ($Value -cnotmatch '\A[0-9]+(?:\.[0-9]+){1,3}\z') { throw 'Toolchain identity unavailable' }
}

function ConvertTo-SafeCompilerText([string]$Text, [string[]]$Roots) {
    foreach ($root in $Roots) {
        if (-not [string]::IsNullOrEmpty($root)) {
            $Text = $Text.Replace($root, '[runner]', [StringComparison]::OrdinalIgnoreCase)
        }
    }
    return $Text
}

function Get-CompileArguments {
    # Literal approved configuration; never consume CFLAGS, CL, _CL_ or input flags.
    return @(
        '/nologo', '/c', '/TC', '/O2', '/MD', '/W3', '/FAsu',
        '/Fasqlite3.asm', '/Fosqlite3.obj',
        '/DSQLITE_CORE', '/DSQLITE_DEFAULT_FOREIGN_KEYS=1', '/DSQLITE_ENABLE_API_ARMOR',
        '/DSQLITE_ENABLE_COLUMN_METADATA', '/DSQLITE_ENABLE_DBSTAT_VTAB', '/DSQLITE_ENABLE_FTS3',
        '/DSQLITE_ENABLE_FTS3_PARENTHESIS', '/DSQLITE_ENABLE_FTS5', '/DSQLITE_ENABLE_JSON1',
        '/DSQLITE_ENABLE_LOAD_EXTENSION=1', '/DSQLITE_ENABLE_MEMORY_MANAGEMENT',
        '/DSQLITE_ENABLE_RTREE', '/DSQLITE_ENABLE_STAT4', '/DSQLITE_SOUNDEX',
        '/DSQLITE_THREADSAFE=1', '/DSQLITE_USE_URI', '/DHAVE_USLEEP=1',
        '/DHAVE_ISNAN', '/D_POSIX_THREAD_SAFE_FUNCTIONS', 'sqlite3.c'
    )
}

function Invoke-PairSqliteCompilation {
    if (-not $IsWindows -or $env:GITHUB_ACTIONS -cne 'true' -or
        $env:GITHUB_RUN_ATTEMPT -cne '1' -or $env:EXPECTED_SHA -cnotmatch '\A[0-9a-f]{40}\z') {
        throw 'Only the approved first Windows workflow attempt may compile'
    }
    # New-Item fails if this same disposable runner already entered the attempt.
    $scratch = Join-Path $env:RUNNER_TEMP 'ork-pair-sqlite-compile'
    New-Item -ItemType Directory -Path $scratch -ErrorAction Stop | Out-Null
    $evidence = Join-Path $scratch 'evidence'
    New-Item -ItemType Directory -Path $evidence | Out-Null
    $result = [ordered]@{
        schema = 1; implementation_sha = $env:EXPECTED_SHA
        plan_merge = '5474a192c13760ee8052bb201c7fc2b1f9aad4fd'
        runner_class = 'windows-2022'; started_utc = [DateTime]::UtcNow.ToString('o')
        compiler_invocations = 0; target_executions = 0; success = $false; stage = 'preflight'
        archive_sha256 = '2e99fb7a497b1e3339bc746195567ed8d3e24945ecd636e3619d20b9de9e9149'
        source_sha256 = 'c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d'
    }
    try {
        Assert-NumericIdentity $env:ImageVersion
        $result.runner_image_version = $env:ImageVersion
        $result.stage = 'toolchain'
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        $installations = @(& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)
        if ($LASTEXITCODE -ne 0 -or $installations.Count -ne 1) { throw 'MSVC installation unavailable' }
        $install = $installations[0].Trim()
        # Installed Microsoft helper sets developer variables without dumping the environment.
        & (Join-Path $install 'Common7\Tools\Launch-VsDevShell.ps1') -Arch amd64 -HostArch amd64 -SkipAutomaticLocation *> $null
        $compiler = (Get-Command cl.exe -CommandType Application -ErrorAction Stop).Source
        if ($compiler -notmatch '[\\/]Hostx64[\\/]x64[\\/]cl\.exe$' -or $env:VSCMD_ARG_TGT_ARCH -ne 'x64') {
            throw 'Expected x64-host x64-target compiler'
        }
        $version = (Get-Item -LiteralPath $compiler).VersionInfo.FileVersion
        $sdk = $env:WindowsSDKVersion.TrimEnd([char]92)
        Assert-NumericIdentity $version
        Assert-NumericIdentity $sdk
        $result.compiler_version = $version
        $result.compiler_sha256 = (Get-FileHash -LiteralPath $compiler -Algorithm SHA256).Hash.ToLowerInvariant()
        $result.windows_sdk_version = $sdk

        $result.stage = 'source'
        $archive = Join-Path $scratch 'libsqlite3-sys-0.30.1.crate'
        Invoke-WebRequest -Uri 'https://static.crates.io/crates/libsqlite3-sys/libsqlite3-sys-0.30.1.crate' -OutFile $archive -MaximumRetryCount 0 -TimeoutSec 120
        Assert-PinnedFile $archive $result.archive_sha256
        & tar.exe -xzf $archive -C $scratch
        if ($LASTEXITCODE -ne 0) { throw 'Pinned crate extraction failed' }
        $sourceDirectory = Join-Path $scratch 'libsqlite3-sys-0.30.1\sqlite3'
        $source = Join-Path $sourceDirectory 'sqlite3.c'
        Assert-PinnedFile $source $result.source_sha256

        $arguments = @(Get-CompileArguments)
        $result.arguments = $arguments
        Remove-Item Env:CL, Env:_CL_ -ErrorAction SilentlyContinue
        $rawLog = Join-Path $scratch 'compiler-raw.txt'
        $result.stage = 'compile'
        Push-Location $sourceDirectory
        try {
            # The sole driver invocation. Compiler children stay inside this one job.
            $result.compiler_invocations = 1
            & $compiler @arguments *> $rawLog
            $result.exit_code = $LASTEXITCODE
        } finally { Pop-Location }
        $result.stage = 'collect'
        $roots = @($scratch, $env:GITHUB_WORKSPACE, $env:USERPROFILE, $install)
        $diagnostics = ConvertTo-SafeCompilerText (Get-Content -LiteralPath $rawLog -Raw) $roots
        [IO.File]::WriteAllText((Join-Path $evidence 'diagnostics.txt'), $diagnostics)
        $assembly = Join-Path $sourceDirectory 'sqlite3.asm'
        $object = Join-Path $sourceDirectory 'sqlite3.obj'
        if ($result.exit_code -ne 0 -or $diagnostics -match '\b(?:warning|error) D[0-9]{4}\b' -or
            -not (Test-Path -LiteralPath $assembly -PathType Leaf) -or
            -not (Test-Path -LiteralPath $object -PathType Leaf)) {
            throw 'Compilation/listing unavailable or compiler options changed'
        }
        $result.object_sha256 = (Get-FileHash -LiteralPath $object -Algorithm SHA256).Hash.ToLowerInvariant()
        $result.object_bytes = (Get-Item -LiteralPath $object).Length
        $result.raw_listing_sha256 = (Get-FileHash -LiteralPath $assembly -Algorithm SHA256).Hash.ToLowerInvariant()
        $listing = ConvertTo-SafeCompilerText (Get-Content -LiteralPath $assembly -Raw) $roots
        [IO.File]::WriteAllText((Join-Path $evidence 'sqlite3.asm'), $listing)
        foreach ($name in @('sqlite3.asm', 'diagnostics.txt')) {
            $path = Join-Path $evidence $name
            $result[$name] = [ordered]@{
                sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
                bytes = (Get-Item -LiteralPath $path).Length
            }
        }
        # The object stays disposable and is never included in the artifact allowlist.
        $result.stage = 'complete'
        $result.success = $true
    } catch {
        # Stage is a fixed tag. Do not print arbitrary exception/environment text.
        $result.success = $false
    } finally {
        $result.ended_utc = [DateTime]::UtcNow.ToString('o')
        [IO.File]::WriteAllText((Join-Path $evidence 'result.json'), ($result | ConvertTo-Json -Depth 5))
    }
    if (-not $result.success) { throw 'Compile-only evidence unavailable; no retry authorized' }
}

if ($MyInvocation.InvocationName -ne '.') { Invoke-PairSqliteCompilation }
