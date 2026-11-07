# Compiler-free tests: dot-source definitions only; never enter the attempt.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'pair-sqlite-compile-evidence.ps1')

function Require($Condition, $Message) {
    if (-not $Condition) { throw $Message }
}
$temp = Join-Path ([IO.Path]::GetTempPath()) ("ork-compile-guard-test-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp | Out-Null
try {
    $file = Join-Path $temp 'synthetic.txt'
    [IO.File]::WriteAllText($file, 'synthetic, not SQLite')
    $hash = (Get-FileHash $file -Algorithm SHA256).Hash.ToLowerInvariant()
    Assert-PinnedFile $file $hash
    foreach ($bad in @(('0' * 64), 'not-a-digest')) {
        $denied = $false
        try { Assert-PinnedFile $file $bad } catch { $denied = $true }
        Require $denied 'Mismatched/malformed source identity was accepted'
    }
    $denied = $false
    try { Assert-PinnedFile (Join-Path $temp 'missing') $hash } catch { $denied = $true }
    Require $denied 'Missing source was accepted'
    $arguments = @(Get-CompileArguments)
    $expected = @(
        '/nologo',
        '/c',
        '/TC',
        '/O2',
        '/MD',
        '/W3',
        '/FAsu',
        '/Fasqlite3.asm',
        '/Fosqlite3.obj',
        '/DSQLITE_CORE',
        '/DSQLITE_DEFAULT_FOREIGN_KEYS=1',
        '/DSQLITE_ENABLE_API_ARMOR',
        '/DSQLITE_ENABLE_COLUMN_METADATA',
        '/DSQLITE_ENABLE_DBSTAT_VTAB',
        '/DSQLITE_ENABLE_FTS3',
        '/DSQLITE_ENABLE_FTS3_PARENTHESIS',
        '/DSQLITE_ENABLE_FTS5',
        '/DSQLITE_ENABLE_JSON1',
        '/DSQLITE_ENABLE_LOAD_EXTENSION=1',
        '/DSQLITE_ENABLE_MEMORY_MANAGEMENT',
        '/DSQLITE_ENABLE_RTREE',
        '/DSQLITE_ENABLE_STAT4',
        '/DSQLITE_SOUNDEX',
        '/DSQLITE_THREADSAFE=1',
        '/DSQLITE_USE_URI',
        '/DHAVE_USLEEP=1',
        '/DHAVE_ISNAN',
        '/D_POSIX_THREAD_SAFE_FUNCTIONS',
        'sqlite3.c'
    )
    Require ($arguments.Count -eq $expected.Count -and
        [string]::Join([char]0, $arguments) -ceq [string]::Join([char]0, $expected)) 'Exact approved compiler argument list changed'
    $secretRoot = 'C:\private\sentinel'
    $text = ConvertTo-SafeCompilerText "source: $secretRoot\sqlite3.c" @($secretRoot)
    Require (-not $text.Contains($secretRoot)) 'Runner path leaked'
    Require ($text.Contains('[runner]\sqlite3.c')) 'Public diagnostic suffix was lost'
    foreach ($valid in @('20261101.1.0', '19.44.35217.0', '10.0.26100.0')) {
        Assert-NumericIdentity $valid
    }
    foreach ($invalid in @('', 'secret-token', "19.44.1\nsecret")) {
        $denied = $false
        try { Assert-NumericIdentity $invalid } catch { $denied = $true }
        Require $denied 'Unstructured toolchain identity was accepted'
    }
    Write-Output 'Compile-only guard tests passed; zero compiler invocations.'
} finally {
    Remove-Item -LiteralPath $temp -Recurse -Force
}
