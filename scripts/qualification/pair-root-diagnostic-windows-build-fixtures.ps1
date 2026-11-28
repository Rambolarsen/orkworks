Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$script=Join-Path $PSScriptRoot 'pair-root-diagnostic-windows-build.ps1'
if (-not (Test-Path -LiteralPath $script)) { throw 'Replacement bounded data reader is not implemented' }
. $script -Nonce ('a'*32)
function Assert($v,$message) { if (-not $v) { throw $message } }
$script:FixtureBudgets=[Collections.Generic.List[object]]::new()
$dir=Join-Path ([IO.Path]::GetTempPath()) ('progress-reader-'+[guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($dir)|Out-Null
try {
    $golden=Join-Path $dir 'golden';[IO.File]::WriteAllText($golden,'abc',[Text.UTF8Encoding]::new($false))
    Assert ((Read-SealedFile $golden (New-ReadBudget 3)).sha256 -ceq 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad') 'native owned file sealing golden'
    [IO.File]::Delete($golden)
    $request='[2,"pinned-windows-candidate-v1","before","'+('a'*32)+'",1000000,"1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6"]'
    $parsed=Read-LiveRequest $request ('a'*32)
    Assert ($parsed[2] -ceq 'before') 'canonical live request'
    foreach($bad in @($request.Replace('[2,','[1,'),$request.Replace('[2,','["2",'),(' '+$request),$request.Replace('before','reference'),$request.Replace('1f5c','0f5c'))){$denied=$false;try{Read-LiveRequest $bad ('a'*32)|Out-Null}catch{$denied=$true};Assert $denied 'request binding refusal'}
    $roots=@{source=$dir;msvc=(Join-Path $dir 'absent');registry=(Join-Path $dir 'missing/src')}
    Assert-ReaderRoots $roots 'preflight'
    $denied=$false;try{Assert-ReaderRoots $roots 'before'}catch{$denied=$true};Assert $denied 'missing live roots denied without repair'
    $f=Join-Path $dir 'a.txt';[IO.File]::WriteAllText($f,'abc',[Text.UTF8Encoding]::new($false))
    $b=New-ReadBudget 3
    $x=Read-SealedFile $f $b
    Assert ($x.sha256 -ceq 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad') 'file hash'
    Assert ($b.hashed -eq 3) 'precharged bytes'
    $denied=$false;try { Read-SealedFile $f $b|Out-Null }catch{$denied=$true};Assert $denied 'hash overflow'
    $a=Get-SealedManifest @{source=$dir} (New-ReadBudget 1048576)
    Assert ($a.files.Count -eq 1 -and $a.files[0].file -ceq 'a.txt') 'membership'
    [IO.File]::WriteAllText($f,'abd',[Text.UTF8Encoding]::new($false))
    $c=Get-SealedManifest @{source=$dir} (New-ReadBudget 1048576)
    Assert ($a.files[0].sha256 -cne $c.files[0].sha256) 'content drift'
    $denied=$false;try {Get-SealedManifest @{source=$dir} (New-ReadBudget 1048576) -FileLimit 0|Out-Null}catch{$denied=$true};Assert $denied 'file cap'
    $denied=$false;try {Get-SealedManifest @{source=$dir} (New-ReadBudget 1048576) -EncodedLimit 8|Out-Null}catch{$denied=$true};Assert $denied 'encoded cap'
    $link=Join-Path $dir 'linked';New-Item -ItemType Junction -Path $link -Target $dir|Out-Null
    try {$denied=$false;try {Get-SealedManifest @{source=$dir} (New-ReadBudget 1048576)|Out-Null}catch{$denied=$true};Assert $denied 'reparse refusal'}finally{[IO.Directory]::Delete($link)}
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
    # Raw USTAR headers preserve unsafe bytes; TarWriter may normalize names.
    function New-FixtureHeader([string]$Name,[long]$Length,[char]$Type='0') {
        $header=[byte[]]::new(512)
        function Field([int]$Offset,[string]$Value){[Text.Encoding]::ASCII.GetBytes($Value).CopyTo($header,$Offset)}
        Field 0 $Name;Field 100 "0000644`0";Field 108 "0000000`0";Field 116 "0000000`0";Field 124 ([Convert]::ToString($Length,8).PadLeft(11,'0')+"`0");Field 136 "00000000000`0";Field 148 '        ';$header[156]=[byte]$Type;Field 257 "ustar`0";Field 263 '00'
        if($Type -eq '2'){Field 157 'target'}
        $sum=0;foreach($v in $header){$sum+=$v};Field 148 ([Convert]::ToString($sum,8).PadLeft(6,'0')+"`0 ")
        return ,$header
    }
    function Assert-ArchiveDenial($Names,[string]$Kind,[string]$Reason) {
        $archiveStream=[IO.File]::Create($archive);$gzip=[IO.Compression.GZipStream]::new($archiveStream,[IO.Compression.CompressionLevel]::Fastest)
        try{foreach($n in $Names){$type=if($Kind -ceq 'link'){'2'}else{'0'};$length=if($Kind -ceq 'large'){536870913}elseif($Kind -ceq 'link'){0}else{1};$header=New-FixtureHeader $n $length $type;$gzip.Write($header,0,512);if($Kind -ceq 'file'){$gzip.Write([byte[]]::new(512),0,512)}};$gzip.Write([byte[]]::new(1024),0,1024)}finally{$gzip.Dispose();$archiveStream.Dispose()}
        $badHash=(Read-SealedFile $archive (New-ReadBudget 1048576)).sha256
        $badPolicy=@{packages=@{'fixture@1.0.0'=$badHash};scripts=@{};proc_macros=@{}}
        $actual='accepted';try{Assert-CrateArchives (Join-Path $registry 'src') (New-ReadBudget 1048576) $badPolicy|Out-Null}catch{$actual=$_.Exception.Message}
        Assert ($actual -ceq $Reason) ('archive refusal '+($Names -join ',')+': expected '+$Reason+'; actual '+$actual)
    }
    foreach($n in @(($package+'/../escape'),($package+'/a/./b'),($package+'/a:b'),($package+'/a\b'),'/absolute')){Assert-ArchiveDenial @($n) 'file' 'archive-path'}
    Assert-ArchiveDenial @(($package+'/same'),($package+'/same')) 'file' 'archive-path'
    Assert-ArchiveDenial @($package+'/link') 'link' 'archive-entry-type'
    Assert-ArchiveDenial @($package+'/oversize') 'large' 'archive-entry-limit'
    $large=Join-Path $dir 'large';[IO.File]::WriteAllBytes($large,[byte[]]::new(1048576))
    $largeHash=(Read-SealedFile $large (New-ReadBudget 2097152)).sha256
    Assert ($largeHash -ceq '30e14955ebf1352266dc2ff8067e68104607e750abb9d3b36582b8af909fcb58') 'large zero golden'
    [IO.File]::WriteAllBytes($large,[byte[]]::new(0))
    Assert ((Read-SealedFile $large (New-ReadBudget 0)).sha256 -ceq 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855') 'empty after large'
    Assert ([OrkProgress.Payload]::Allocations -eq 1 -and [OrkProgress.Payload]::MaximumLeases -eq 1) 'exclusive reuse metrics'
    $lease=[OrkProgress.Payload]::Acquire();$denied=$false;try{[OrkProgress.Payload]::Acquire()|Out-Null}catch{$denied=$true}finally{$lease.Dispose()};Assert $denied 'nested lease denial'
    # Run the same request decoder, manifest pipeline and writer on seven owned roots.
    $profile=Join-Path $dir 'profile';[IO.Directory]::CreateDirectory($profile)|Out-Null
    $owned=@{};foreach($label in @('msvc','registry','rust','sdk_bin','sdk_include','sdk_lib','source')){$owned[$label]=Join-Path $profile $label;[IO.Directory]::CreateDirectory($owned[$label])|Out-Null}
    $owned.registry=Join-Path $profile 'registry/src';[IO.Directory]::CreateDirectory($owned.registry)|Out-Null
    [IO.Directory]::CreateDirectory((Join-Path $profile 'registry/cache'))|Out-Null
    # The archive verifier requires one index; an empty fixed index admits zero packages.
    [IO.Directory]::CreateDirectory((Join-Path $owned.registry 'fixture-index'))|Out-Null
    [IO.Directory]::CreateDirectory((Join-Path $profile 'registry/cache/fixture-index'))|Out-Null
    [IO.File]::WriteAllText((Join-Path $owned.source 'literal'),'abc',[Text.UTF8Encoding]::new($false))
    $script:budget=New-ReadBudget 4194304;$script:LiveSequence=0;$script:LiveFiles=0;$script:LiveDirectories=0;$script:LiveArchives=0;$script:LiveProgress=$true
    Emit-Progress 'start';Assert-ReaderRoots $owned 'before';Emit-Progress 'ready'
    $data=Invoke-ManifestRead $owned $script:budget @{packages=@{};scripts=@{};proc_macros=@{}} -EncodedLimit 32768
    $output=Join-Path $dir 'bound-output.json';Write-ReaderOutput $output $data 'before' ('a'*32) '1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6' $script:budget -EncodedLimit 32768
    $script:LiveProgress=$false
    $envelope=[IO.File]::ReadAllText($output)|ConvertFrom-Json -AsHashtable
    Assert ($envelope.mode -ceq 'before' -and $envelope.data.files.Count -eq 1 -and $envelope.data.files[0].sha256 -ceq 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad') 'complete producer envelope'
    Assert ($envelope.data.hash_bytes -eq $script:budget.hashed -and $script:LiveSequence -eq 21) 'terminal settled encoding debit'
    $denied=$false;try{Write-ReaderOutput $output $data 'before' ('a'*32) $envelope.policy_sha256 $script:budget -EncodedLimit 32768}catch{$denied=$true};Assert $denied 'exclusive output'
    $total=0;foreach($b in $script:FixtureBudgets){$total+=$b.hashed};Assert ($total -le 67108864) 'suite cumulative work'
    [ordered]@{tests_passed=$true;powershell_version=$PSVersionTable.PSVersion.ToString();envelope=$envelope;hash_bytes=$total;scratch_allocations=[OrkProgress.Payload]::Allocations;maximum_leases=[OrkProgress.Payload]::MaximumLeases;target_executions=0;runtime_qualified=$false;production_pin_accepted=$false}|ConvertTo-Json -Depth 14 -Compress
} finally {Remove-Item -LiteralPath $dir -Recurse -Force}
