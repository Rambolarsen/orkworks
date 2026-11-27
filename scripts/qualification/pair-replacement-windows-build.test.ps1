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
    $archiveStream=[IO.File]::Create($archive);$gzip=[IO.Compression.GZipStream]::new($archiveStream,[IO.Compression.CompressionLevel]::Fastest)
    $tar=[System.Formats.Tar.TarWriter]::new($gzip,$true)
    try {
        foreach($item in @{ 'Cargo.toml'="[package]`nname = `"fixture`"`nversion = `"1.0.0`"`nbuild = false`n"; 'src/lib.rs'='pub fn fixture() {}' }.GetEnumerator()) {
            $bytes=[Text.Encoding]::UTF8.GetBytes($item.Value);$entry=[System.Formats.Tar.PaxTarEntry]::new([System.Formats.Tar.TarEntryType]::RegularFile,($package+'/'+$item.Key))
            $entry.DataStream=[IO.MemoryStream]::new($bytes);$tar.WriteEntry($entry);$entry.DataStream.Dispose()
            $file=Join-Path $src $item.Key;[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($file))|Out-Null;[IO.File]::WriteAllBytes($file,$bytes)
        }
    } finally {$tar.Dispose();$gzip.Dispose();$archiveStream.Dispose()}
    [IO.File]::WriteAllText((Join-Path $src '.cargo-ok'),'{"v":1}',[Text.UTF8Encoding]::new($false))
    $h=(Read-SealedFile $archive (New-ReadBudget 1048576)).sha256
    $policy=@{packages=@{'fixture@1.0.0'=$h};scripts=@{};proc_macros=@{}}
    $verified=Assert-CrateArchives (Join-Path $registry 'src') (New-ReadBudget 1048576) $policy
    Assert ($verified.crates.Count -eq 1 -and $verified.crates[0].files.Count -eq 2) 'archive association'
    $badPolicy=@{packages=@{'fixture@1.0.0'=('0'*64)};scripts=@{};proc_macros=@{}}
    $badBudget=New-ReadBudget 1048576;$denied=$false
    try{Assert-CrateArchives (Join-Path $registry 'src') $badBudget $badPolicy|Out-Null}catch{$denied=$_.Exception.Message -ceq 'package-source-drift'}
    Assert ($denied -and $badBudget.hashed -eq [IO.FileInfo]::new($archive).Length) 'deny bad archive before decompression'
    [IO.File]::WriteAllText((Join-Path $src 'unlisted.rs'),'extra',[Text.UTF8Encoding]::new($false))
    $denied=$false;try{Assert-CrateArchives (Join-Path $registry 'src') (New-ReadBudget 1048576) $policy|Out-Null}catch{$denied=$_.Exception.Message -ceq 'unlisted-crate-file'};Assert $denied 'unlisted extracted member'
    [IO.File]::Delete((Join-Path $src 'unlisted.rs'))
    [IO.File]::WriteAllText((Join-Path $src 'src/lib.rs'),'changed',[Text.UTF8Encoding]::new($false))
    $denied=$false;try{Assert-CrateArchives (Join-Path $registry 'src') (New-ReadBudget 1048576) $policy|Out-Null}catch{$denied=$true};Assert $denied 'tampered extracted member'
    # Malformed archives are owned synthetic bytes with fixture-specific lock hashes.
    function Assert-ArchiveDenial($Names,[string]$Kind,[string]$Reason) {
        $archiveStream=[IO.File]::Create($archive);$gzip=[IO.Compression.GZipStream]::new($archiveStream,[IO.Compression.CompressionLevel]::Fastest)
        $tar=[System.Formats.Tar.TarWriter]::new($gzip,$true)
        try{foreach($n in $Names){$type=if($Kind -ceq 'link'){[System.Formats.Tar.TarEntryType]::SymbolicLink}else{[System.Formats.Tar.TarEntryType]::RegularFile};$entry=[System.Formats.Tar.PaxTarEntry]::new($type,$n);if($Kind -ceq 'link'){$entry.LinkName='target'}else{$entry.DataStream=[IO.MemoryStream]::new([byte[]]@(1))};try{$tar.WriteEntry($entry)}finally{if($null -ne $entry.DataStream){$entry.DataStream.Dispose()}}}}finally{$tar.Dispose();$gzip.Dispose();$archiveStream.Dispose()}
        $badHash=(Read-SealedFile $archive (New-ReadBudget 1048576)).sha256
        $badPolicy=@{packages=@{'fixture@1.0.0'=$badHash};scripts=@{};proc_macros=@{}}
        $denied=$false;try{Assert-CrateArchives (Join-Path $registry 'src') (New-ReadBudget 1048576) $badPolicy|Out-Null}catch{$denied=$_.Exception.Message -ceq $Reason};Assert $denied ('archive refusal: '+$Reason)
    }
    foreach($n in @($package+'/../escape',$package+'/a/./b',$package+'/a:b',$package+'/a\b','/absolute')){Assert-ArchiveDenial @($n) 'file' 'archive-path'}
    Assert-ArchiveDenial @($package+'/same',$package+'/same') 'file' 'archive-path'
    Assert-ArchiveDenial @($package+'/link') 'link' 'archive-entry-type'
    # A USTAR header declares a 512 MiB+1 member without allocating its body.
    $header=[byte[]]::new(512)
    function Field([int]$Offset,[string]$Value){[Text.Encoding]::ASCII.GetBytes($Value).CopyTo($header,$Offset)}
    Field 0 ($package+'/oversize');Field 100 "0000644`0";Field 108 "0000000`0";Field 116 "0000000`0";Field 124 "04000000001`0";Field 136 "00000000000`0";Field 148 '        ';$header[156]=48;Field 257 "ustar`0";Field 263 '00'
    $sum=0;foreach($v in $header){$sum+=$v};Field 148 ([Convert]::ToString($sum,8).PadLeft(6,'0')+"`0 ")
    $archiveStream=[IO.File]::Create($archive);$gzip=[IO.Compression.GZipStream]::new($archiveStream,[IO.Compression.CompressionLevel]::Fastest)
    try{$gzip.Write($header,0,512);$gzip.Write([byte[]]::new(1024),0,1024)}finally{$gzip.Dispose();$archiveStream.Dispose()}
    $badPolicy=@{packages=@{'fixture@1.0.0'=(Read-SealedFile $archive (New-ReadBudget 1048576)).sha256};scripts=@{};proc_macros=@{}}
    $denied=$false;try{Assert-CrateArchives (Join-Path $registry 'src') (New-ReadBudget 1048576) $badPolicy|Out-Null}catch{$denied=$_.Exception.Message -ceq 'archive-entry-limit'};Assert $denied 'oversize archive member before body read'
    Write-Output 'Bounded Windows reader fixtures passed' 
} finally {Remove-Item -LiteralPath $dir -Recurse -Force}
