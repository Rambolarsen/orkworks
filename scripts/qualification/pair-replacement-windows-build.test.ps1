Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$script=Join-Path $PSScriptRoot 'pair-replacement-windows-build.ps1'
if (-not (Test-Path -LiteralPath $script)) { throw 'Replacement bounded data reader is not implemented' }
. $script
function Assert($v,$message) { if (-not $v) { throw $message } }
$dir=Join-Path ([IO.Path]::GetTempPath()) ('replacement-reader-'+[guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($dir)|Out-Null
try {
    $f=Join-Path $dir 'a.txt';[IO.File]::WriteAllText($f,'abc',[Text.UTF8Encoding]::new($false))
    $b=New-ReadBudget 3
    $x=Read-SealedFile $f $b
    Assert ($x.sha256 -ceq 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad') 'file hash'
    Assert ($b.hashed -eq 3) 'precharged bytes'
    $denied=$false;try { Read-SealedFile $f $b|Out-Null }catch{$denied=$true};Assert $denied 'hash overflow'
    $a=Get-SealedManifest @{source=$dir} (New-ReadBudget 1024)
    Assert ($a.files.Count -eq 1 -and $a.files[0].file -ceq 'a.txt') 'membership'
    [IO.File]::WriteAllText($f,'abd',[Text.UTF8Encoding]::new($false))
    $c=Get-SealedManifest @{source=$dir} (New-ReadBudget 1024)
    Assert ($a.files[0].sha256 -cne $c.files[0].sha256) 'content drift'
    $denied=$false;try {Get-SealedManifest @{source=$dir} (New-ReadBudget 1024) -FileLimit 0|Out-Null}catch{$denied=$true};Assert $denied 'file cap'
    $denied=$false;try {Get-SealedManifest @{source=$dir} (New-ReadBudget 1024) -EncodedLimit 8|Out-Null}catch{$denied=$true};Assert $denied 'encoded cap'
    $link=Join-Path $dir 'linked';New-Item -ItemType Junction -Path $link -Target $dir|Out-Null
    try {$denied=$false;try {Get-SealedManifest @{source=$dir} (New-ReadBudget 1024)|Out-Null}catch{$denied=$true};Assert $denied 'reparse refusal'}finally{[IO.Directory]::Delete($link)}
    Add-Type -AssemblyName System.Formats.Tar
    $registry=Join-Path $dir 'registry';$index='fixture-index';$package='fixture-1.0.0'
    $src=Join-Path $registry "src/$index/$package";$cache=Join-Path $registry "cache/$index"
    [IO.Directory]::CreateDirectory($src)|Out-Null;[IO.Directory]::CreateDirectory($cache)|Out-Null
    $archive=Join-Path $cache ($package+'.crate')
    $output=[IO.File]::Create($archive);$gzip=[IO.Compression.GZipStream]::new($output,[IO.Compression.CompressionLevel]::Fastest)
    $tar=[System.Formats.Tar.TarWriter]::new($gzip,$true)
    try {
        foreach($item in @{ 'Cargo.toml'="[package]`nname = `"fixture`"`nversion = `"1.0.0`"`nbuild = false`n"; 'src/lib.rs'='pub fn fixture() {}' }.GetEnumerator()) {
            $bytes=[Text.Encoding]::UTF8.GetBytes($item.Value);$entry=[System.Formats.Tar.PaxTarEntry]::new([System.Formats.Tar.TarEntryType]::RegularFile,($package+'/'+$item.Key))
            $entry.DataStream=[IO.MemoryStream]::new($bytes);$tar.WriteEntry($entry);$entry.DataStream.Dispose()
            $file=Join-Path $src $item.Key;[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($file))|Out-Null;[IO.File]::WriteAllBytes($file,$bytes)
        }
    } finally {$tar.Dispose();$gzip.Dispose();$output.Dispose()}
    [IO.File]::WriteAllText((Join-Path $src '.cargo-ok'),'{"v":1}',[Text.UTF8Encoding]::new($false))
    $h=(Read-SealedFile $archive (New-ReadBudget 1048576)).sha256
    $policy=@{packages=@{'fixture@1.0.0'=$h};scripts=@{};proc_macros=@{}}
    $verified=Assert-CrateArchives (Join-Path $registry 'src') (New-ReadBudget 1048576) $policy
    Assert ($verified.crates.Count -eq 1 -and $verified.crates[0].files.Count -eq 2) 'archive association'
    [IO.File]::WriteAllText((Join-Path $src 'src/lib.rs'),'changed',[Text.UTF8Encoding]::new($false))
    $denied=$false;try{Assert-CrateArchives (Join-Path $registry 'src') (New-ReadBudget 1048576) $policy|Out-Null}catch{$denied=$true};Assert $denied 'tampered extracted member'
    Write-Output 'Bounded Windows reader fixtures passed' 
} finally {Remove-Item -LiteralPath $dir -Recurse -Force}
