param([ValidateSet('checks','pipeline','refusals')][string]$Case='checks')
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$script=Join-Path $PSScriptRoot 'pair-root-diagnostic-windows-build.ps1'
if (-not (Test-Path -LiteralPath $script)) { throw 'Replacement bounded data reader is not implemented' }
. $script -Nonce ('a'*32)
function Assert($v,$message) { $script:Assertions++;if (-not $v) { throw $message } }
$script:Assertions=0
$script:FixtureBudgets=[Collections.Generic.List[object]]::new()
$script:FixtureHashLimit=if($Case -ceq 'pipeline'){2097152}else{16777216}
$script:MakeBudget=${function:New-ReadBudget}
function New-ReadBudget([long]$Maximum) {
    $used=[long]0;foreach($b in $script:FixtureBudgets){$used+=$b.hashed}
    return & $script:MakeBudget ([Math]::Min($Maximum,$script:FixtureHashLimit-$used))
}

# One pre-effect storage ledger per worker; the Node parent partitions the global caps.
$script:FixtureStorage=@{files=0L;dirs=0L;inputs=0L;scratch=0L;file_cap=512L;dir_cap=64L;input_cap=$(if($Case -ceq 'checks'){2097152L}else{524288L});scratch_cap=2097152L}
function Reserve-FixtureStorage($Ledger,[long]$Files=0,[long]$Dirs=0,[long]$InputBytes=0,[long]$ScratchBytes=0) {
    foreach($pair in @(@('files','file_cap',$Files),@('dirs','dir_cap',$Dirs),@('inputs','input_cap',$InputBytes),@('scratch','scratch_cap',$ScratchBytes))){
        if($pair[2] -lt 0 -or $pair[2] -gt $Ledger[$pair[1]]-$Ledger[$pair[0]]){throw 'fixture-storage-limit'}
    }
    $Ledger.files+=$Files;$Ledger.dirs+=$Dirs;$Ledger.inputs+=$InputBytes;$Ledger.scratch+=$ScratchBytes
}
function Assert-FixturePath([string]$Name) {
    $full=[IO.Path]::GetFullPath($Name);$parent=[IO.Path]::GetFullPath($env:TEMP)
    if(-not $full.StartsWith($parent+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'foreign-fixture-path'}
    return $full
}
function New-FixtureDirectory([string]$Name) {
    $full=Assert-FixturePath $Name;$parent=[IO.Path]::GetFullPath($env:TEMP);$missing=0
    for($p=$full;$p -cne $parent -and -not [IO.Directory]::Exists($p);$p=[IO.Path]::GetDirectoryName($p)){$missing++}
    Reserve-FixtureStorage $script:FixtureStorage -Dirs $missing
    return [IO.Directory]::CreateDirectory($full)
}
function Write-FixtureBytes([string]$Name,[byte[]]$Bytes) {
    $full=Assert-FixturePath $Name;$new=if([IO.File]::Exists($full)){0}else{1}
    Reserve-FixtureStorage $script:FixtureStorage -Files $new -InputBytes $Bytes.Length -ScratchBytes $Bytes.Length
    [IO.File]::WriteAllBytes($full,$Bytes)
}
function Write-FixtureText([string]$Name,[string]$Text,$Encoding=([Text.UTF8Encoding]::new($false))) {Write-FixtureBytes $Name ($Encoding.GetBytes($Text))}
function New-FixtureArchive([string]$Name) {
    $full=Assert-FixturePath $Name;$new=if([IO.File]::Exists($full)){0}else{1}
    # Fixed fixture archive writes are headers/one-byte members, bounded by 64KiB.
    Reserve-FixtureStorage $script:FixtureStorage -Files $new -InputBytes 65536 -ScratchBytes 65536
    return [IO.File]::Create($full)
}
$dir=Join-Path ([IO.Path]::GetTempPath()) ('progress-reader-'+[guid]::NewGuid().ToString('N'))
New-FixtureDirectory ($dir)|Out-Null
try {
  if($Case -ceq 'checks'){
    $golden=Join-Path $dir 'golden';Write-FixtureText ($golden) ('abc') ([Text.UTF8Encoding]::new($false))
    Assert ((Read-SealedFile $golden (New-ReadBudget 3)).sha256 -ceq 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad') 'native owned file sealing golden'
    [IO.File]::Delete($golden)
    $request='[3,"pinned-windows-root-diagnostic-v1","before","'+('a'*32)+'",1000000,"1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6"]'
    $parsed=Read-LiveRequest $request ('a'*32)
    Assert ($parsed[2] -ceq 'before') 'canonical live request'
    foreach($bad in @($request.Replace('[3,','[1,'),$request.Replace('[3,','["2",'),(' '+$request),$request.Replace('before','reference'),$request.Replace('1f5c','0f5c'))){$denied=$false;try{Read-LiveRequest $bad ('a'*32)|Out-Null}catch{$denied=$true};Assert $denied 'request binding refusal'}
    $roots=@{source=$dir;msvc=(Join-Path $dir 'absent');registry=(Join-Path $dir 'missing/src')}
    Assert-ReaderRoots $roots 'preflight'
    $denied=$false;try{Assert-ReaderRoots $roots 'before'}catch{$denied=$true};Assert $denied 'missing live roots denied without repair'
    $f=Join-Path $dir 'a.txt';Write-FixtureText ($f) ('abc') ([Text.UTF8Encoding]::new($false))
    $b=New-ReadBudget 3
    $x=Read-SealedFile $f $b
    Assert ($x.sha256 -ceq 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad') 'file hash'
    Assert ($b.hashed -eq 3) 'precharged bytes'
    $denied=$false;try { Read-SealedFile $f $b|Out-Null }catch{$denied=$true};Assert $denied 'hash overflow'
    $a=Get-SealedManifest @{source=$dir} (New-ReadBudget 1048576)
    Assert ($a.files.Count -eq 1 -and $a.files[0].file -ceq 'a.txt') 'membership'
    Write-FixtureText ($f) ('abd') ([Text.UTF8Encoding]::new($false))
    $c=Get-SealedManifest @{source=$dir} (New-ReadBudget 1048576)
    Assert ($a.files[0].sha256 -cne $c.files[0].sha256) 'content drift'
    $denied=$false;try {Get-SealedManifest @{source=$dir} (New-ReadBudget 1048576) -FileLimit 0|Out-Null}catch{$denied=$true};Assert $denied 'file cap'
    $denied=$false;try {Get-SealedManifest @{source=$dir} (New-ReadBudget 1048576) -EncodedLimit 8|Out-Null}catch{$denied=$true};Assert $denied 'encoded cap'
    $link=Join-Path $dir 'linked';Reserve-FixtureStorage $script:FixtureStorage -Dirs 1;New-Item -ItemType Junction -Path $link -Target $dir|Out-Null
    try {$denied=$false;try {Get-SealedManifest @{source=$dir} (New-ReadBudget 1048576)|Out-Null}catch{$denied=$true};Assert $denied 'reparse refusal'}finally{[IO.Directory]::Delete($link)}
    Add-Type -AssemblyName System.Formats.Tar
    $registry=Join-Path $dir 'registry';$index='fixture-index';$package='fixture-1.0.0'
    $src=Join-Path $registry "src/$index/$package";$cache=Join-Path $registry "cache/$index"
    New-FixtureDirectory ($src)|Out-Null;New-FixtureDirectory ($cache)|Out-Null
    $archive=Join-Path $cache ($package+'.crate')
    $archiveStream=New-FixtureArchive ($archive);$gzip=[IO.Compression.GZipStream]::new($archiveStream,[IO.Compression.CompressionLevel]::Fastest)
    $tar=[System.Formats.Tar.TarWriter]::new($gzip,$true)
    try {
        foreach($item in @{ 'Cargo.toml'="[package]`nname = `"fixture`"`nversion = `"1.0.0`"`nbuild = false`n"; 'src/lib.rs'='pub fn fixture() {}' }.GetEnumerator()) {
            $bytes=[Text.Encoding]::UTF8.GetBytes($item.Value);$entry=[System.Formats.Tar.PaxTarEntry]::new([System.Formats.Tar.TarEntryType]::RegularFile,($package+'/'+$item.Key))
            $entry.DataStream=[IO.MemoryStream]::new($bytes);$tar.WriteEntry($entry);$entry.DataStream.Dispose()
            $file=Join-Path $src $item.Key;New-FixtureDirectory ([IO.Path]::GetDirectoryName($file))|Out-Null;Write-FixtureBytes ($file) ($bytes)
        }
    } finally {$tar.Dispose();$gzip.Dispose();$archiveStream.Dispose()}
    Write-FixtureText ((Join-Path $src '.cargo-ok')) ('{"v":1}') ([Text.UTF8Encoding]::new($false))
    $h=(Read-SealedFile $archive (New-ReadBudget 1048576)).sha256
    $policy=@{packages=@{'fixture@1.0.0'=$h};scripts=@{};proc_macros=@{}}
    $verified=Assert-CrateArchives (Join-Path $registry 'src') (New-ReadBudget 1048576) $policy
    Assert ($verified.crates.Count -eq 1 -and $verified.crates[0].files.Count -eq 2) 'archive association'
    $badPolicy=@{packages=@{'fixture@1.0.0'=('0'*64)};scripts=@{};proc_macros=@{}}
    $badBudget=New-ReadBudget 1048576;$denied=$false
    try{Assert-CrateArchives (Join-Path $registry 'src') $badBudget $badPolicy|Out-Null}catch{$denied=$_.Exception.Message -ceq 'package-source-drift'}
    Assert ($denied -and $badBudget.hashed -eq [IO.FileInfo]::new($archive).Length) 'deny bad archive before decompression'
    Write-FixtureText ((Join-Path $src 'unlisted.rs')) ('extra') ([Text.UTF8Encoding]::new($false))
    $denied=$false;try{Assert-CrateArchives (Join-Path $registry 'src') (New-ReadBudget 1048576) $policy|Out-Null}catch{$denied=$_.Exception.Message -ceq 'unlisted-crate-file'};Assert $denied 'unlisted extracted member'
    [IO.File]::Delete((Join-Path $src 'unlisted.rs'))
    Write-FixtureText ((Join-Path $src 'src/lib.rs')) ('changed') ([Text.UTF8Encoding]::new($false))
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
        $archiveStream=New-FixtureArchive ($archive);$gzip=[IO.Compression.GZipStream]::new($archiveStream,[IO.Compression.CompressionLevel]::Fastest)
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
    $large=Join-Path $dir 'large';Write-FixtureBytes ($large) ([byte[]]::new(1048576))
    $largeHash=(Read-SealedFile $large (New-ReadBudget 2097152)).sha256
    Assert ($largeHash -ceq '30e14955ebf1352266dc2ff8067e68104607e750abb9d3b36582b8af909fcb58') 'large zero golden'
    Write-FixtureBytes ($large) ([byte[]]::new(0))
    Assert ((Read-SealedFile $large (New-ReadBudget 0)).sha256 -ceq 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855') 'empty after large'
    Assert ([OrkRootDiagnostic.Payload]::Allocations -eq 1 -and [OrkRootDiagnostic.Payload]::MaximumLeases -eq 1) 'exclusive reuse metrics'
    $lease=[OrkRootDiagnostic.Payload]::Acquire();$denied=$false;try{[OrkRootDiagnostic.Payload]::Acquire()|Out-Null}catch{$denied=$true}finally{$lease.Dispose()};Assert $denied 'nested lease denial'

    foreach($dimension in @('files','dirs','inputs','scratch')){
      $model=@{files=0L;dirs=0L;inputs=0L;scratch=0L;file_cap=2048L;dir_cap=256L;input_cap=4194304L;scratch_cap=8388608L}
      switch($dimension){
        files {Reserve-FixtureStorage $model -Files 2048;$overflow=@{Files=1}}
        dirs {Reserve-FixtureStorage $model -Dirs 256;$overflow=@{Dirs=1}}
        inputs {Reserve-FixtureStorage $model -InputBytes 4194304;$overflow=@{InputBytes=1}}
        scratch {Reserve-FixtureStorage $model -ScratchBytes 8388608;$overflow=@{ScratchBytes=1}}
      }
      $denied=$false;try{Reserve-FixtureStorage $model @overflow}catch{$denied=$_.Exception.Message -ceq 'fixture-storage-limit'}
      Assert $denied ('pre-effect storage boundary '+$dimension)
    }
    $stderr=[IO.StringWriter]::new();$originalError=[Console]::Error
    try{[Console]::SetError($stderr);& $script -Nonce ('c'*32);$exitCode=$LASTEXITCODE}finally{[Console]::SetError($originalError)}
    Assert ($exitCode -eq 1 -and $stderr.ToString().Trim() -ceq 'live-entry-disabled') 'reader CLI refuses without a child or installed access'
  }
  $cases=[Collections.Generic.List[object]]::new()
  if($Case -cne 'checks'){
    $profile=Join-Path $dir 'profile';New-FixtureDirectory ($profile)|Out-Null
    $owned=@{};foreach($label in @('msvc','registry','rust','sdk_bin','sdk_include','sdk_lib','source')){$owned[$label]=Join-Path $profile $label;New-FixtureDirectory ($owned[$label])|Out-Null}
    $owned.registry=Join-Path $profile 'registry/src';New-FixtureDirectory ($owned.registry)|Out-Null
    New-FixtureDirectory ((Join-Path $owned.registry 'fixture-index'))|Out-Null
    New-FixtureDirectory ((Join-Path $profile 'registry/cache/fixture-index'))|Out-Null
    $nested=Join-Path $owned.msvc 'nested';New-FixtureDirectory ($nested)|Out-Null
    foreach($i in 1..16){Write-FixtureText ((Join-Path $nested ('f'+$i))) ('abc') ([Text.UTF8Encoding]::new($false))}
    Write-FixtureText ((Join-Path $owned.source 'literal')) ('abc') ([Text.UTF8Encoding]::new($false))
    $policy=@{packages=@{};scripts=@{};proc_macros=@{}}
    function Reset-Reader([long]$Maximum){
      $script:budget=New-ReadBudget $Maximum;$script:LiveSequence=0;$script:LiveSample=0;$script:LiveFiles=0;$script:LiveDirectories=0;$script:LiveArchives=0;$script:LiveProgress=$true
    }
    if($Case -ceq 'pipeline'){
      # The old reader is a guarded library over these exact owned identities only.
      $baseline=& {
        . (Join-Path $PSScriptRoot 'pair-progress-windows-build.ps1') -Nonce ('b'*32)
        $baselineBudget=New-ReadBudget 1048576
        $data=Invoke-ManifestRead $owned $baselineBudget $policy -EncodedLimit 32768
        Charge-Encoding $baselineBudget 32768
        return $data
      }
      $bytes=[Text.UTF8Encoding]::new($false,$true).GetBytes(($baseline|ConvertTo-Json -Depth 14 -Compress))
      Assert ($bytes.Length -le 32768) 'bounded baseline encoding'
      Reserve-FixtureStorage $script:FixtureStorage -Files 1 -ScratchBytes 32768
      $file=[IO.FileStream]::new((Join-Path $env:TEMP 'root-baseline.json'),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
      try{$file.Write($bytes,0,$bytes.Length)}finally{$file.Dispose()}
      $Nonce='a'*32;Reset-Reader 1048576
      Emit-Progress 'start';Assert-ReaderRoots $owned 'before';Emit-Progress 'ready'
      $data=Invoke-ManifestRead $owned $script:budget $policy -EncodedLimit 32768
      Reserve-FixtureStorage $script:FixtureStorage -Files 1 -ScratchBytes 32768
      Write-ReaderOutput (Join-Path $env:TEMP 'root-output.json') $data 'before' $Nonce '1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6' $script:budget -EncodedLimit 32768
      return
    }
    $script:RealEmitSample=${function:Emit-Sample}
    foreach($stop in 1..7){
      $Nonce=('{0:x32}' -f $stop);$script:StopSample=$stop
      function Emit-Sample([int]$Detail){& $script:RealEmitSample $Detail;if($Detail -eq $script:StopSample){throw 'fixture-stopped'}}
      $capture=[IO.StringWriter]::new();$originalOutput=[Console]::Out;$stopped=$false
      $payloadBefore=[OrkRootDiagnostic.Payload]::HashBytes
      try{
        [Console]::SetOut($capture);Reset-Reader 131072
        Emit-Progress 'start';Assert-ReaderRoots $owned 'before';Emit-Progress 'ready'
        try{Invoke-ManifestRead $owned $script:budget $policy -EncodedLimit 32768|Out-Null}catch{
          if($_.Exception.Message -cne 'fixture-stopped'){throw};$stopped=$true;Emit-Progress 'error' '' 'input-denied'
        }
      }finally{[Console]::SetOut($originalOutput);$script:LiveProgress=$false}
      Assert $stopped 'controlled completion boundary refusal'
      $payload=[OrkRootDiagnostic.Payload]::HashBytes-$payloadBefore
      Assert ($payload -eq $(if($stop -ge 5){3}else{0})) 'payload hash occurs only after first held checkpoint'
      Assert ([Text.Encoding]::ASCII.GetByteCount($capture.ToString()) -le 4096) 'bounded refused prefix'
      $cases.Add(@{stop=$stop;nonce=$Nonce;prefix=$capture.ToString();payload_bytes=$payload})
    }
    Set-Item Function:Emit-Sample -Value $script:RealEmitSample
  }
  $total=[long]0;foreach($b in $script:FixtureBudgets){$total+=$b.hashed}
  Assert ($total -le $script:FixtureHashLimit) 'one cumulative fixture work pool'
  Assert ([OrkRootDiagnostic.Payload]::Allocations -eq 1 -and [OrkRootDiagnostic.Payload]::MaximumLeases -eq 1) 'single exclusive payload buffer'
  Reserve-FixtureStorage $script:FixtureStorage -ScratchBytes 65536
  $report=[ordered]@{storage=$script:FixtureStorage;resource_vectors=$(if($Case -ceq 'checks'){4}else{0});tests_passed=$true;case=$Case;powershell_version=$PSVersionTable.PSVersion.ToString();assertions=$script:Assertions;cases=@($cases);hash_bytes=$total;scratch_allocations=1;maximum_leases=1;target_executions=0;runtime_qualified=$false;production_pin_accepted=$false}
  $json=$report|ConvertTo-Json -Depth 14 -Compress
  Assert ([Text.Encoding]::UTF8.GetByteCount($json) -le 65536) 'bounded fixture result'
  [Console]::Out.Write($json+"`n")
}finally{Remove-Item -LiteralPath $dir -Recurse -Force}
