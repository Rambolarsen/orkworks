param([ValidateSet('Preflight','Manifest')][string]$Mode='Preflight',[string]$Request,[string]$Output)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
# In-process declarations are part of this sealed own controller, not an inspected-tool child.
if ($IsWindows -and -not ('OrkReplacement.Native' -as [type])) {
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
namespace OrkReplacement {
 [StructLayout(LayoutKind.Sequential)] public struct Time { public uint Low,High; }
 [StructLayout(LayoutKind.Sequential)] public struct Info {
  public uint Attributes; public Time Creation,Access,Write; public uint Volume,SizeHigh,SizeLow,Links,IndexHigh,IndexLow;
 }
 public sealed class CappedStream : System.IO.Stream {
  readonly System.IO.Stream inner; readonly long limit; public long Used {get;private set;}
  public CappedStream(System.IO.Stream source,long maximum){inner=source;limit=maximum;}
  public override int Read(byte[] buffer,int offset,int count){if(count>limit-Used)throw new System.IO.IOException("hash-limit");int n=inner.Read(buffer,offset,count);Used+=n;return n;}
  public override int Read(Span<byte> buffer){if(buffer.Length>limit-Used)throw new System.IO.IOException("hash-limit");int n=inner.Read(buffer);Used+=n;return n;}
  public override bool CanRead=>true;public override bool CanSeek=>false;public override bool CanWrite=>false;
  public override long Length=>throw new NotSupportedException();public override long Position {get=>Used;set=>throw new NotSupportedException();}
  public override void Flush(){} public override long Seek(long a,System.IO.SeekOrigin b)=>throw new NotSupportedException();
  public override void SetLength(long n)=>throw new NotSupportedException();public override void Write(byte[] b,int o,int n)=>throw new NotSupportedException();
 }
 public static class Native {
  [DllImport("kernel32.dll",SetLastError=true)] public static extern bool GetFileInformationByHandle(SafeFileHandle h,out Info info);
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] public static extern uint GetFinalPathNameByHandleW(SafeFileHandle h,StringBuilder text,uint size,uint flags);
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] public static extern SafeFileHandle CreateFileW(string name,uint access,uint share,IntPtr security,uint creation,uint flags,IntPtr template);
 }
}
'@
}
function New-ReadBudget([long]$Maximum) {
    if ($Maximum -lt 0 -or $Maximum -gt 34359738368) {throw 'hash-limit'}
    return @{maximum=$Maximum;hashed=[long]0}
}
function Assert-PlainPath([string]$File) {
    $full=[IO.Path]::GetFullPath($File)
    if ($full -notmatch '\A[A-Za-z]:\\' -or $full.Substring(3).Contains(':')) {throw 'foreign-input'}
    for($p=$full;;$p=[IO.Path]::GetDirectoryName($p)) {
        $attr=[IO.File]::GetAttributes($p)
        if (($attr -band [IO.FileAttributes]::ReparsePoint) -ne 0) {throw 'linked-input'}
        $parent=[IO.Path]::GetDirectoryName($p)
        if ([string]::IsNullOrEmpty($parent) -or $parent -ceq $p) {break}
    }
    return $full
}
function Get-HandleInfo($Handle,[string]$Expected) {
    $info=[OrkReplacement.Info]::new()
    if (-not [OrkReplacement.Native]::GetFileInformationByHandle($Handle,[ref]$info)) {throw 'file-identity'}
    if (($info.Attributes -band 1024) -ne 0) {throw 'linked-input'}
    $text=[Text.StringBuilder]::new(4096)
    $length=[OrkReplacement.Native]::GetFinalPathNameByHandleW($Handle,$text,4096,0)
    if ($length -eq 0 -or $length -ge 4096) {throw 'file-identity'}
    $actual=$text.ToString()
    if ($actual.StartsWith('\\?\')) {$actual=$actual.Substring(4)}
    if (-not [string]::Equals($actual,$Expected,[StringComparison]::OrdinalIgnoreCase)) {throw 'foreign-input'}
    return @{id=('{0:x8}:{1:x8}{2:x8}' -f $info.Volume,$info.IndexHigh,$info.IndexLow);bytes=([long]$info.SizeHigh*4294967296+[long]$info.SizeLow);write=('{0:x8}{1:x8}' -f $info.Write.High,$info.Write.Low);attributes=$info.Attributes}
}
function Get-DirectoryIdentity([string]$Directory) {
    $full=Assert-PlainPath $Directory
    $h=[OrkReplacement.Native]::CreateFileW($full,128,1,[IntPtr]::Zero,3,35651584,[IntPtr]::Zero)
    try {if ($h.IsInvalid) {throw 'directory-identity'};return (Get-HandleInfo $h $full).id}finally{$h.Dispose()}
}
function Read-SealedFile([string]$File,$Budget,[switch]$PE) {
    $full=Assert-PlainPath $File
    $named=[IO.FileInfo]::new($full)
    if (-not $named.Exists -or $named.Length -gt 536870912) {throw 'file-limit'}
    $expected=[long]$named.Length
    if ($Budget.hashed+$expected -gt $Budget.maximum) {throw 'hash-limit'}
    $Budget.hashed+=$expected
    $stream=[IO.FileStream]::new($full,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
    try {
        $before=Get-HandleInfo $stream.SafeFileHandle $full
        if ($before.bytes -ne $expected -or ($before.attributes -band 16) -ne 0) {throw 'file-drift'}
        $hash=[Security.Cryptography.IncrementalHash]::CreateHash([Security.Cryptography.HashAlgorithmName]::SHA256)
        try {
            $buffer=[byte[]]::new(1048576);$offset=[long]0
            while($offset -lt $expected) {
                $n=$stream.Read($buffer,0,[int][Math]::Min(1048576,$expected-$offset))
                if ($n -le 0) {throw 'file-short-read'}
                $hash.AppendData($buffer,0,$n);$offset+=$n
            }
            $digest=[Convert]::ToHexString($hash.GetHashAndReset()).ToLowerInvariant()
        }finally{$hash.Dispose()}
        $result=@{bytes=$expected;sha256=$digest;id=$before.id}
        if ($PE) {
            if ($expected -lt 64) {throw 'pe-format'}
            $reader=[IO.BinaryReader]::new($stream,[Text.Encoding]::UTF8,$true)
            try {
                $stream.Position=0;if ($reader.ReadUInt16() -ne 23117) {throw 'pe-format'}
                $stream.Position=60;$nt=$reader.ReadUInt32()
                if ($nt -gt $expected-24) {throw 'pe-format'}
                $stream.Position=$nt;if ($reader.ReadUInt32() -ne 17744) {throw 'pe-format'}
                $result.machine=$reader.ReadUInt16()
            }finally{$reader.Dispose()}
            $v=[Diagnostics.FileVersionInfo]::GetVersionInfo($full)
            $result.version=if ([string]::IsNullOrEmpty($v.FileVersion)) {$null}else{('{0}.{1}.{2}.{3}' -f $v.FileMajorPart,$v.FileMinorPart,$v.FileBuildPart,$v.FilePrivatePart)}
        }
        $after=Get-HandleInfo $stream.SafeFileHandle $full
        $again=[IO.FileStream]::new((Assert-PlainPath $full),[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
        try {$current=Get-HandleInfo $again.SafeFileHandle $full}finally{$again.Dispose()}
        if ($before.id -cne $after.id -or $before.id -cne $current.id -or $before.bytes -ne $after.bytes -or $before.bytes -ne $current.bytes -or $before.write -cne $after.write -or $before.write -cne $current.write) {throw 'file-drift'}
        return $result
    }finally{$stream.Dispose()}
}
function Get-SealedManifest($Roots,$Budget,[int]$FileLimit=200000,[int]$DirectoryLimit=50000,[long]$EncodedLimit=67108864) {
    $files=[Collections.Generic.List[object]]::new();$dirs=[Collections.Generic.List[object]]::new();$bytes=[long]0;$encoded=[long]4096
    foreach($label in @($Roots.Keys|Sort-Object)) {
        $root=Assert-PlainPath ([string]$Roots[$label])
        $pending=[Collections.Generic.Stack[object]]::new();$pending.Push(@{full=$root;relative=''})
        while($pending.Count -gt 0) {
            $item=$pending.Pop()
            if ($dirs.Count -ge $DirectoryLimit) {throw 'directory-limit'}
            $row=@{root=$label;file=$item.relative;id=(Get-DirectoryIdentity $item.full)}
            $encoded += [Text.Encoding]::UTF8.GetByteCount(($row|ConvertTo-Json -Compress))+1
            if ($encoded -gt $EncodedLimit) {throw 'manifest-limit'}
            $dirs.Add($row)
            foreach($entry in [IO.Directory]::EnumerateFileSystemEntries($item.full)) {
                $name=[IO.Path]::GetFileName($entry)
                $relative=if($item.relative) {$item.relative+'/'+$name}else{$name}
                if ($label -ceq 'source' -and $relative -ceq '.git') {continue}
                if ($relative.Length -gt 1024) {throw 'path-limit'}
                $attr=[IO.File]::GetAttributes($entry)
                if (($attr -band [IO.FileAttributes]::ReparsePoint) -ne 0) {throw 'linked-input'}
                if (($attr -band [IO.FileAttributes]::Directory) -ne 0) {
                    if($pending.Count+$dirs.Count -ge $DirectoryLimit) {throw 'directory-limit'}
                    $pending.Push(@{full=$entry;relative=$relative})
                } else {
                    if ($files.Count -ge $FileLimit) {throw 'file-count-limit'}
                    $length=[IO.FileInfo]::new($entry).Length
                    if ($bytes+$length -gt 12884901888) {throw 'manifest-bytes-limit'}
                    $sealed=Read-SealedFile $entry $Budget
                    $row=@{root=$label;file=$relative;bytes=$sealed.bytes;sha256=$sealed.sha256;id=$sealed.id}
                    $encoded += [Text.Encoding]::UTF8.GetByteCount(($row|ConvertTo-Json -Compress))+1
                    if ($encoded -gt $EncodedLimit) {throw 'manifest-limit'}
                    $files.Add($row);$bytes+=$sealed.bytes
                }
            }
        }
    }
    return @{files=@($files|Sort-Object root,file);directories=@($dirs|Sort-Object root,file);file_bytes=$bytes;file_count=$files.Count;directory_count=$dirs.Count;hash_bytes=$Budget.hashed;success=$true}
}
function Assert-CrateArchives([string]$Source,$Budget,$Policy) {
    Add-Type -AssemblyName System.Formats.Tar
    $sourceRoot=Assert-PlainPath $Source
    $indices=[Collections.Generic.List[string]]::new()
    foreach($entry in [IO.Directory]::EnumerateDirectories($sourceRoot)) {if($indices.Count -ge 1){throw 'registry-identity'};$indices.Add((Assert-PlainPath $entry))}
    if($indices.Count -ne 1){throw 'registry-identity'}
    $index=[IO.Path]::GetFileName($indices[0]);$cache=Join-Path ([IO.Path]::GetDirectoryName($sourceRoot)) ('cache/'+$index)
    $cache=Assert-PlainPath $cache
    $crates=[Collections.Generic.List[object]]::new();$archiveBytes=[long]0;$members=0;$encoded=[long]4096
    foreach($directory in [IO.Directory]::EnumerateDirectories($indices[0])) {
        if($crates.Count -ge $Policy.packages.Count){throw 'package-count-limit'}
        $directory=Assert-PlainPath $directory;$leaf=[IO.Path]::GetFileName($directory)
        $archive=Join-Path $cache ($leaf+'.crate');$identity=Read-SealedFile $archive $Budget
        if($Budget.hashed+$identity.bytes -gt $Budget.maximum){throw 'hash-limit'}
        $Budget.hashed+=$identity.bytes
        $archiveBytes+=$identity.bytes;if($archiveBytes -gt 12884901888){throw 'manifest-bytes-limit'}
        $files=[Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
        $folded=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        $stream=[IO.FileStream]::new((Assert-PlainPath $archive),[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
        try {
            $opened=Get-HandleInfo $stream.SafeFileHandle ([IO.Path]::GetFullPath($archive))
            if($opened.id -cne $identity.id -or $opened.bytes -ne $identity.bytes){throw 'archive-drift'}
            $gzip=[IO.Compression.GZipStream]::new($stream,[IO.Compression.CompressionMode]::Decompress,$true)
            $meter=[OrkReplacement.CappedStream]::new($gzip,($Budget.maximum-$Budget.hashed))
            $tar=[System.Formats.Tar.TarReader]::new($meter,$true)
            try {
                while($null -ne ($entry=$tar.GetNextEntry($false))) {
                    $name=$entry.Name
                    if($name.Length -gt 1024 -or $name.Contains('\') -or $name.Contains(':') -or $name.Contains([char]0) -or $name.StartsWith('/')){throw 'archive-path'}
                    if($entry.EntryType -eq [System.Formats.Tar.TarEntryType]::Directory){continue}
                    if($entry.EntryType -ne [System.Formats.Tar.TarEntryType]::RegularFile -and $entry.EntryType -ne [System.Formats.Tar.TarEntryType]::V7RegularFile){throw 'archive-entry-type'}
                    if(-not $name.StartsWith($leaf+'/',[StringComparison]::Ordinal)){throw 'archive-path'}
                    $relative=$name.Substring($leaf.Length+1)
                    if([string]::IsNullOrEmpty($relative) -or @($relative.Split('/')|Where-Object{$_ -ceq '..' -or $_ -ceq '.' -or $_ -ceq ''}).Count -gt 0 -or -not $folded.Add($relative)){throw 'archive-path'}
                    if($members -ge 200000 -or $entry.Length -lt 0 -or $entry.Length -gt 536870912){throw 'archive-entry-limit'}
                    $encoded += [Text.Encoding]::UTF8.GetByteCount((@($relative,('a'*64))|ConvertTo-Json -Compress))+1
                    if($encoded -gt 67108864){throw 'manifest-limit'}
                    $h=[Security.Cryptography.IncrementalHash]::CreateHash([Security.Cryptography.HashAlgorithmName]::SHA256)
                    try{
                        $left=[long]$entry.Length;$buffer=[byte[]]::new(1048576)
                        while($left -gt 0){$n=$entry.DataStream.Read($buffer,0,[int][Math]::Min(1048576,$left));if($n -le 0){throw 'archive-short-read'};$h.AppendData($buffer,0,$n);$left-=$n}
                        $files.Add($relative,[Convert]::ToHexString($h.GetHashAndReset()).ToLowerInvariant());$members++
                    }finally{$h.Dispose()}
                }
                $end=Get-HandleInfo $stream.SafeFileHandle ([IO.Path]::GetFullPath($archive))
                if($end.id -cne $opened.id -or $end.bytes -ne $opened.bytes -or $end.write -cne $opened.write){throw 'archive-drift'}
            }finally{$Budget.hashed+=$meter.Used;$tar.Dispose();$meter.Dispose();$gzip.Dispose()}
        }finally{$stream.Dispose()}
        foreach($name in $files.Keys){$actual=Read-SealedFile (Join-Path $directory $name) $Budget;if($actual.sha256 -cne $files[$name]){throw 'crate-file-drift'}}
        $tomlPath=Join-Path $directory 'Cargo.toml'
        if(-not $files.ContainsKey('Cargo.toml')){throw 'crate-manifest'}
        $toml=[IO.File]::ReadAllText((Assert-PlainPath $tomlPath),[Text.UTF8Encoding]::new($false,$true))
        $section=[regex]::Match($toml,'(?ms)^\[package\]\r?\n(.*?)(?=^\[|\z)').Groups[1].Value
        $name=[regex]::Match($section,'(?m)^name = "([^"\r\n]+)"\r?$').Groups[1].Value
        $version=[regex]::Match($section,'(?m)^version = "([^"\r\n]+)"\r?$').Groups[1].Value
        $key=$name+'@'+$version
        if($leaf -cne ($name+'-'+$version) -or -not $Policy.packages.ContainsKey($key) -or $Policy.packages[$key] -cne $identity.sha256){throw 'package-source-drift'}
        $build=$null;$m=[regex]::Match($section,'(?m)^build = (.+?)\r?$')
        if($m.Success){if($m.Groups[1].Value -ceq 'false'){}elseif($m.Groups[1].Value -match '\A"([^"\r\n]+)"\z'){$build=$Matches[1]}else{throw 'unknown-build-script'}}elseif($files.ContainsKey('build.rs')){$build='build.rs'}
        if($null -ne $build){if(-not $Policy.scripts.ContainsKey($key) -or $Policy.scripts[$key].file -cne $build -or -not $files.ContainsKey($build) -or $files[$build] -cne $Policy.scripts[$key].sha256){throw 'unaudited-build-script'}}
        $lib=[regex]::Match($toml,'(?ms)^\[lib\]\r?\n(.*?)(?=^\[|\z)').Groups[1].Value;$proc=$null
        if($lib -match '(?m)^proc-macro = true\r?$'){
            $m=[regex]::Match($lib,'(?m)^path = "([^"\r\n]+)"\r?$');$proc=if($m.Success){$m.Groups[1].Value}else{'src/lib.rs'}
            if(-not $Policy.proc_macros.ContainsKey($key) -or $Policy.proc_macros[$key].file -cne $proc -or -not $files.ContainsKey($proc) -or $files[$proc] -cne $Policy.proc_macros[$key].sha256){throw 'unaudited-proc-macro'}
        }
        # Extra executable/module source cannot hide beside authenticated archive members.
        $pending=[Collections.Generic.Stack[string]]::new();$pending.Push($directory);$seen=0;$dirCount=0
        while($pending.Count -gt 0){$d=$pending.Pop();$dirCount++;if($dirCount -gt 50000){throw 'directory-limit'};foreach($p in [IO.Directory]::EnumerateFileSystemEntries($d)){
            $p=Assert-PlainPath $p
            if(([IO.File]::GetAttributes($p) -band [IO.FileAttributes]::Directory) -ne 0){$pending.Push($p);continue}
            $seen++;if($seen -gt 200000){throw 'file-count-limit'};$relative=[IO.Path]::GetRelativePath($directory,$p).Replace('\','/')
            if($relative -ceq '.cargo-ok'){if([IO.FileInfo]::new($p).Length -gt 32 -or [IO.File]::ReadAllText($p) -cne '{"v":1}'){throw 'cargo-marker'}}
            elseif(-not $files.ContainsKey($relative)){throw 'unlisted-crate-file'}
        }}
        $crates.Add(@{package=$key;directory=($index+'/'+$leaf);archive=@{file=($index+'/'+$leaf+'.crate');bytes=$identity.bytes;sha256=$identity.sha256;id=$identity.id};files=$files;build_script=$build;proc_macro=$proc})
    }
    return @{crates=@($crates|Sort-Object package);archive_bytes=$archiveBytes;archive_count=$crates.Count;encoded_bytes=$encoded;cache_id=(Get-DirectoryIdentity $cache)}
}

if ($MyInvocation.InvocationName -eq '.') {return}
if (-not $IsWindows -or $env:ORK_REPLACEMENT_READER -cne '1') {throw 'reader-denied'}
$budget=$null
try {
    $scratch=Assert-PlainPath $env:ORK_REPLACEMENT_SCRATCH
    if ([IO.Path]::GetFullPath($Request) -cne (Join-Path $scratch 'request.json') -or [IO.Path]::GetFullPath($Output) -cne (Join-Path $scratch 'reader.json')) {throw 'reader-path'}
    if ([IO.FileInfo]::new($Request).Length -gt 1048576) {throw 'request-limit'}
    $r=[IO.File]::ReadAllText((Assert-PlainPath $Request))|ConvertFrom-Json -AsHashtable
    $budget=New-ReadBudget ([long]$r.hash_remaining)
    $policy=[IO.File]::ReadAllText((Join-Path $PSScriptRoot 'pair-replacement-windows-build.inputs.json'))|ConvertFrom-Json -AsHashtable
    if ($Mode -ceq 'Preflight') {
        $tools=@{}
        foreach($name in @($policy.paths.Keys|Sort-Object)) {$tools[$name]=Read-SealedFile $policy.paths[$name] $budget -PE}
        $result=@{success=$true;tools=$tools;hash_bytes=$budget.hashed}
    }else{
        $expected=@{source=$env:ORK_REPLACEMENT_WORKSPACE;registry=(Join-Path $scratch 'working/cargo-home/registry/src')}
        foreach($key in $policy.roots.Keys) {$expected[$key]=$policy.roots[$key]}
        if ($r.roots.Count -ne $expected.Count) {throw 'manifest-roots'}
        foreach($key in $expected.Keys) {if (-not $r.roots.ContainsKey($key) -or -not [string]::Equals([IO.Path]::GetFullPath($r.roots[$key]),[IO.Path]::GetFullPath($expected[$key]),[StringComparison]::OrdinalIgnoreCase)) {throw 'manifest-roots'}}
        $verified=Assert-CrateArchives $expected.registry $budget $policy
        $result=Get-SealedManifest $r.roots $budget -FileLimit (200000-$verified.archive_count) -DirectoryLimit 49998 -EncodedLimit (67108864-$verified.encoded_bytes)
        $result.file_bytes+=$verified.archive_bytes;$result.file_count+=$verified.archive_count;$result.directory_count+=2
        if($result.file_bytes -gt 12884901888){throw 'manifest-bytes-limit'}
        $result.crates=$verified.crates;$result.archive_cache_id=$verified.cache_id
        $result.hash_bytes=$budget.hashed
    }
    $json=$result|ConvertTo-Json -Depth 12 -Compress
    if ([Text.Encoding]::UTF8.GetByteCount($json) -gt 67108864) {throw 'manifest-limit'}
    [IO.File]::WriteAllText($Output,$json,[Text.UTF8Encoding]::new($false))
    @{success=$true;hash_bytes=$budget.hashed}|ConvertTo-Json -Compress
} catch {
    $reason=$_.Exception.Message
    if ($reason -notmatch '\A[a-z-]{1,60}\z') {$reason='data-reader-unavailable'}
    @{success=$false;reason=$reason;hash_bytes=$(if($null -eq $budget){0}else{$budget.hashed})}|ConvertTo-Json -Compress
    exit 1
}
