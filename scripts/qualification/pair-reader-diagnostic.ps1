param([Parameter(Mandatory=$true)][string]$Nonce)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$ProgressPreference='SilentlyContinue'
# Only the nonce is caller supplied; all data paths are derived below owned cwd.
# No legacy reader is imported or invoked.
if (-not $IsWindows -or $env:ORK_READER_SYNTHETIC -cne 'synthetic-v1' -or $Nonce -cnotmatch '\A[a-f0-9]{32}\z') { exit 64 }
$owned=[IO.Path]::GetFullPath((Get-Location).Path)
if ([IO.Path]::GetFileName($owned) -cnotmatch '\Aork-reader-diagnostic-[a-f0-9]{32}\z' -or $env:TEMP -cne $owned -or $env:TMP -cne $owned) {exit 65}
$bound=$false
$failureCode=70
try {
    $requestPath=Join-Path $owned ('requests/'+$Nonce+'.json')
    # Framing precedes native setup. Held-file identity admission follows ready.
    $prefile=[IO.FileStream]::new($requestPath,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
    $failureCode=71
    try {
        if($prefile.Length -gt 4096) {throw 'bad-request'}
        $raw=[byte[]]::new([int]$prefile.Length)
        $prefile.ReadExactly($raw,0,$raw.Length)
        if($prefile.Length -ne $raw.Length) {throw 'bad-request'}
        $requestText=[Text.UTF8Encoding]::new($false,$true).GetString($raw)
    } finally {$prefile.Dispose()}
    $failureCode=72
    $request=ConvertFrom-Json -InputObject $requestText -NoEnumerate
    $failureCode=73
    if ($request.Count -ne 5 -or $request[0] -ne 1 -or $request[1] -cne 'synthetic-v1' -or $request[2] -cnotin @('reference','reuse','guards') -or $request[3] -cne $Nonce -or $request[4] -isnot [long] -and $request[4] -isnot [int]) {throw 'bad-request'}
    $mode=[string]$request[2];$grant=[long]$request[4]
    $failureCode=74
    if ($grant -le 0 -or $grant -gt 67108864 -or $requestText -cne ('[1,"synthetic-v1","'+$mode+'","'+$Nonce+'",'+$grant.ToString([Globalization.CultureInfo]::InvariantCulture)+']')) {throw 'bad-request'}
    $bound=$true
    [Console]::Out.Write('[1,"'+$Nonce+'",1,"start","",0,0,0,0]'+"`n");[Console]::Out.Flush()
$failureCode=75
Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.IO.Compression;
using System.Formats.Tar;
using System.Text;
using System.Text.Json;
using System.Text.Encodings.Web;
using System.Security.Cryptography;
using System.Runtime.InteropServices;
using System.Collections.Generic;
using System.Linq;
using Microsoft.Win32.SafeHandles;
namespace OrkReaderDiagnostic {
 [StructLayout(LayoutKind.Sequential)] public struct Time { public uint Low,High; }
 [StructLayout(LayoutKind.Sequential)] public struct Info {
  public uint Attributes; public Time Creation,Access,Write; public uint Volume,SizeHigh,SizeLow,Links,IndexHigh,IndexLow;
 }
 static class Native {
  [DllImport("kernel32.dll",SetLastError=true)] public static extern bool GetFileInformationByHandle(SafeFileHandle h,out Info info);
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] public static extern uint GetFinalPathNameByHandleW(SafeFileHandle h,StringBuilder text,uint size,uint flags);
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] public static extern SafeFileHandle CreateFileW(string name,uint access,uint share,IntPtr security,uint creation,uint flags,IntPtr template);
 }
 public sealed class Budget {
  public long Used {get;private set;} public readonly long Maximum;
  public Budget(long maximum){if(maximum<0||maximum>67108864)throw new IOException("hash-limit");Maximum=maximum;}
  public void Charge(long n){if(n<0||n>Maximum-Used)throw new IOException("hash-limit");Used+=n;}
 }
 public sealed class Meter : Stream {
  readonly Stream inner; readonly Budget budget; readonly long cap; long used;
  public Meter(Stream s,Budget b,long maximum){inner=s;budget=b;cap=maximum;}
  public override int Read(byte[] b,int o,int n){budget.Charge(n);int count=inner.Read(b,o,n);used+=count;if(used>cap)throw new IOException("input-limit");return count;}
  public override int Read(Span<byte> b){budget.Charge(b.Length);int count=inner.Read(b);used+=count;if(used>cap)throw new IOException("input-limit");return count;}
  public override bool CanRead=>true;public override bool CanSeek=>false;public override bool CanWrite=>false;
  public override long Length=>throw new NotSupportedException();public override long Position {get=>used;set=>throw new NotSupportedException();}
  public override void Flush(){} public override long Seek(long a,SeekOrigin b)=>throw new NotSupportedException();
  public override void SetLength(long n)=>throw new NotSupportedException();public override void Write(byte[] b,int o,int n)=>throw new NotSupportedException();
 }
 public sealed class Reader : IDisposable {
  const int MiB=1048576;
  public readonly Budget Work; readonly string root,nonce,mode; readonly byte[] reusable;
  public long Allocations,Capacity,HashBytes; public int MaximumLeases; int leases,sequence=1,files,dirs,archives;
  long inputBytes,encoded=4096; readonly List<SafeFileHandle> held=new List<SafeFileHandle>();
  readonly Dictionary<string,string> expected=new Dictionary<string,string>(StringComparer.Ordinal);
  readonly List<object> fileRows=new List<object>(),dirRows=new List<object>(),archiveRows=new List<object>();
  string catalogHash;
  static readonly JsonSerializerOptions Json=new JsonSerializerOptions {Encoder=JavaScriptEncoder.UnsafeRelaxedJsonEscaping};
  static readonly string[] Labels={"msvc","registry","rust","sdk_bin","sdk_include","sdk_lib","source"};
  public static string Encode(object x)=>JsonSerializer.Serialize(x,Json);
  static void Deny(string why){throw new IOException(why);}
  public Reader(string owned,string n,string strategy,long grant){root=Path.GetFullPath(owned);nonce=n;mode=strategy;Work=new Budget(grant);if(mode=="reuse"){reusable=new byte[MiB];Allocations=1;Capacity=MiB;}}
  public static string Plain(string path) {
   if(path.StartsWith("\\\\",StringComparison.Ordinal))Deny("input-denied");
   string full=Path.GetFullPath(path);
   if(full.Length<3||full[1]!=':'||full[2]!='\\'||full.Substring(3).Contains(':'))Deny("input-denied");
   for(string p=full;!String.IsNullOrEmpty(p);p=Path.GetDirectoryName(p))if((File.GetAttributes(p)&FileAttributes.ReparsePoint)!=0)Deny("input-denied");
   return full;
  }
  string Owned(string path){string p=Plain(path);if(!p.StartsWith(root+Path.DirectorySeparatorChar,StringComparison.OrdinalIgnoreCase)&&!String.Equals(p,root,StringComparison.OrdinalIgnoreCase))Deny("input-denied");return p;}
  static string Id(Info i)=>$"{i.Volume:x8}:{i.IndexHigh:x8}{i.IndexLow:x8}";
  static long Length(Info i)=>(long)i.SizeHigh*4294967296+i.SizeLow;
  static string Stamp(Info i)=>$"{i.Write.High:x8}{i.Write.Low:x8}";
  static Info Identity(SafeFileHandle h,string path) {
   if(h.IsInvalid||!Native.GetFileInformationByHandle(h,out Info info)||(info.Attributes&1024)!=0)throw new IOException("identity-drift");
   var text=new StringBuilder(4096);uint n=Native.GetFinalPathNameByHandleW(h,text,4096,0);
   string actual=text.ToString();if(actual.StartsWith("\\\\?\\"))actual=actual.Substring(4);
   if(n==0||n>=4096||!String.Equals(actual,path,StringComparison.OrdinalIgnoreCase))Deny("identity-drift");
   return info;
  }
  static void Same(Info a,Info b){if(Id(a)!=Id(b)||Length(a)!=Length(b)||Stamp(a)!=Stamp(b)||a.Attributes!=b.Attributes)Deny("identity-drift");}
  string DirectoryId(string path,bool keep=false) {
   string p=Owned(path);var h=Native.CreateFileW(p,128,1,IntPtr.Zero,3,35651584,IntPtr.Zero);
   bool retained=false;try{var i=Identity(h,p);if((i.Attributes&16)==0)Deny("input-denied");if(keep){held.Add(h);retained=true;}return Id(i);}finally{if(!retained)h.Dispose();}
  }
  FileStream Open(string path)=>new FileStream(Owned(path),FileMode.Open,FileAccess.Read,FileShare.Read);
  sealed class Lease : IDisposable {
   readonly Reader owner; public readonly byte[] Bytes; bool released;
   public Lease(Reader r){owner=r;if(r.leases!=0)Deny("fixture-assertion");r.leases=1;r.MaximumLeases=Math.Max(r.MaximumLeases,1);if(r.reusable!=null)Bytes=r.reusable;else{Bytes=new byte[MiB];r.Allocations++;r.Capacity+=MiB;}}
   public void Dispose(){if(!released){owner.leases=0;released=true;}}
  }
  string Hash(Stream stream,long size,bool charged=false) {
   if(size<0||size>MiB)Deny("input-limit");if(!charged)Work.Charge(size);
   using(var lease=new Lease(this))using(var h=IncrementalHash.CreateHash(HashAlgorithmName.SHA256)){
    long left=size;while(left>0){int n=stream.Read(lease.Bytes,0,(int)Math.Min(MiB,left));if(n<=0)Deny("identity-drift");h.AppendData(lease.Bytes,0,n);HashBytes+=n;left-=n;}
    return Convert.ToHexString(h.GetHashAndReset()).ToLowerInvariant();
   }
  }
  public Dictionary<string,object> ReadFile(string path,long cap=MiB,long expectedSize=-1) {
   string p=Owned(path);long size=new FileInfo(p).Length;if(size<0||size>Math.Min(cap,MiB))Deny("input-limit");Work.Charge(size);
   using(var s=Open(p)){
    Info before=Identity(s.SafeFileHandle,p);if(Length(before)!=size||(expectedSize>=0&&Length(before)!=expectedSize)||(before.Attributes&16)!=0)Deny("identity-drift");
    string digest=Hash(s,size,true);Same(before,Identity(s.SafeFileHandle,p));using(var again=Open(p))Same(before,Identity(again.SafeFileHandle,p));
    return new Dictionary<string,object>{{"bytes",size},{"sha256",digest},{"id",Id(before)}};
   }
  }
  byte[] Control(string path,int cap,bool charge=true) {
   string p=Owned(path);using(var s=Open(p)){
    Info b=Identity(s.SafeFileHandle,p);long length=Length(b);if(length<0||length>cap)Deny("input-limit");if(charge)Work.Charge(length);
    byte[] bytes=new byte[(int)length];s.ReadExactly(bytes);Same(b,Identity(s.SafeFileHandle,p));using(var again=Open(p))Same(b,Identity(again.SafeFileHandle,p));return bytes;
   }
  }
  void Row(List<object> rows,Dictionary<string,object> row,long cap=MiB) {
   encoded+=Encoding.UTF8.GetByteCount(Encode(row))+1;if(encoded>cap)Deny("output-limit");rows.Add(row);
  }
  public void Emit(string phase,string label="",string reason=null) {
   if(leases!=0)Deny("fixture-assertion");
   object[] tuple=reason==null?new object[]{1,nonce,++sequence,phase,label,Work.Used,files,dirs,archives}:new object[]{1,nonce,++sequence,phase,"",Work.Used,files,dirs,archives,reason};
   string line=Encode(tuple)+"\n";if(Encoding.UTF8.GetByteCount(line)>192||sequence>21)Deny("output-limit");Console.Out.Write(line);Console.Out.Flush();
  }
  public void Setup() {
   DirectoryId(root,true);foreach(string p in new[]{"requests","outputs","fixtures","fixtures/registry/cache","fixtures/registry/cache/synthetic-index"})DirectoryId(Path.Combine(root,p),true);
   foreach(string label in Labels)DirectoryId(Root(label),true);
   var request=Control(Path.Combine(root,"requests",nonce+".json"),4096,false);
   if(Encoding.UTF8.GetString(request)!=Encode(new object[]{1,"synthetic-v1",mode,nonce,Work.Maximum}))Deny("bad-request");
   string catalog=Path.Combine(root,"catalog.json");
   byte[] bytes=Control(catalog,262144);Work.Charge(bytes.Length);catalogHash=Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();using(var doc=JsonDocument.Parse(bytes)){
    var a=doc.RootElement;if(a.ValueKind!=JsonValueKind.Array||a.GetArrayLength()!=6||a[0].GetInt32()!=1||a[1].GetString()!="fixture@1.0.0"||a[4].ValueKind!=JsonValueKind.Null||a[5].ValueKind!=JsonValueKind.Null||Encoding.UTF8.GetString(bytes)!=Encode(a))Deny("format-denied");
    archiveDigest=a[2].GetString();if(!Hex(archiveDigest))Deny("format-denied");string prior="";
    foreach(var pair in a[3].EnumerateArray()){
     if(pair.ValueKind!=JsonValueKind.Array||pair.GetArrayLength()!=2)Deny("format-denied");string name=pair[0].GetString(),sha=pair[1].GetString();MemberName("fixture-1.0.0/"+name);if(String.CompareOrdinal(prior,name)>=0||!Hex(sha))Deny("format-denied");expected.Add(name,sha);prior=name;
    }
    if(expected.Count!=1027||!expected.ContainsKey("Cargo.toml"))Deny("format-denied");
   }
   AuditInputs();Emit("ready");
  }
  string archiveDigest;
  static bool Hex(string s)=>s!=null&&s.Length==64&&s.All(c=>(c>='0'&&c<='9')||(c>='a'&&c<='f'));
  string Root(string label)=>Path.GetFullPath(Path.Combine(root,"fixtures",label=="registry"?"registry/src":label));
  void AuditInputs(){
   int fc=0,dc=0;long total=0;var pending=new Stack<string>();pending.Push(Path.Combine(root,"fixtures"));
   while(pending.Count>0){string p=Owned(pending.Pop());if(++dc>256)Deny("input-limit");foreach(string x in Directory.EnumerateFileSystemEntries(p)){
    Owned(x);if((File.GetAttributes(x)&FileAttributes.Directory)!=0)pending.Push(x);else{long n=new FileInfo(x).Length;if(n>MiB||++fc>2048||n<0||(total+=n)>4L*MiB)Deny("input-limit");}
   }}
  }
  public static string MemberName(string full) {
   const string prefix="fixture-1.0.0/";
   if(full==null||full.Length>1024||full.Contains('\\')||full.Contains(':')||full.Contains('\0')||!full.StartsWith(prefix,StringComparison.Ordinal))throw new IOException("format-denied");
   string n=full.Substring(prefix.Length);
   if(n.Length==0||n.Split('/').Any(s=>s==""||s=="."||s=="..")||n.Any(c=>c<32||c>126)||n.Split('/').Any(s=>s.EndsWith('.')||s.EndsWith(' ')))Deny("format-denied");
   return n;
  }
  Dictionary<string,string> InspectArchive(string path,string expectedDigest,Dictionary<string,string> members,long memberLimit=MiB,long expandedLimit=4L*MiB) {
   var identity=ReadFile(path);if((string)identity["sha256"]!=expectedDigest)Deny("identity-drift");
   var found=new Dictionary<string,string>(StringComparer.Ordinal);var folded=new HashSet<string>(StringComparer.OrdinalIgnoreCase);long expandedMembers=0;
   Work.Charge((long)identity["bytes"]);
   using(var input=Open(path)){
    Info before=Identity(input.SafeFileHandle,Owned(path));if(Id(before)!=(string)identity["id"]||Length(before)!=(long)identity["bytes"])Deny("identity-drift");
    using(var gzip=new GZipStream(input,CompressionMode.Decompress,true))using(var meter=new Meter(gzip,Work,expandedLimit))using(var tar=new TarReader(meter,true)){
     TarEntry e;while((e=tar.GetNextEntry(false))!=null){
      string n=MemberName(e.Name);if(e.EntryType!=TarEntryType.RegularFile&&e.EntryType!=TarEntryType.V7RegularFile)Deny("format-denied");
      if(!folded.Add(n)||found.Count>=2048||e.Length<0||e.Length>memberLimit||(expandedMembers+=e.Length)>4L*MiB)Deny("input-limit");
      if(!members.TryGetValue(n,out string golden))Deny("format-denied");string digest=Hash(e.DataStream??Stream.Null,e.Length);
      if(digest!=golden)Deny("identity-drift");found.Add(n,digest);
     }
    }
    Same(before,Identity(input.SafeFileHandle,Owned(path)));using(var named=Open(path))Same(before,Identity(named.SafeFileHandle,Owned(path)));
   }
   if(found.Count!=members.Count)Deny("format-denied");
   return found;
  }
  public void Archive() {
   string dir=Path.Combine(Root("registry"),"synthetic-index","fixture-1.0.0");
   if(Directory.EnumerateFileSystemEntries(Root("registry")).Count()!=1||Directory.EnumerateFileSystemEntries(Path.GetDirectoryName(dir)).Count()!=1)Deny("input-denied");
   string path=Path.Combine(root,"fixtures/registry/cache/synthetic-index/fixture-1.0.0.crate");
   if(Directory.EnumerateFileSystemEntries(Path.GetDirectoryName(path)).Count()!=1)Deny("input-denied");
   var identity=ReadFile(path);
   var found=InspectArchive(path,archiveDigest,expected);
   if(Encode(identity)!=Encode(ReadFile(path)))Deny("identity-drift");
   foreach(var pair in found.OrderBy(x=>x.Key,StringComparer.Ordinal)){
    var row=ReadFile(Path.Combine(dir,pair.Key));if((string)row["sha256"]!=pair.Value)Deny("identity-drift");
   }
   var seen=new HashSet<string>(StringComparer.Ordinal);var todo=new Stack<string>();todo.Push(dir);int dc=0;
   while(todo.Count>0){string d=todo.Pop();DirectoryId(d);if(++dc>256)Deny("input-limit");foreach(string p in Directory.EnumerateFileSystemEntries(d)){
    Owned(p);if((File.GetAttributes(p)&FileAttributes.Directory)!=0){todo.Push(p);continue;}
    string n=Path.GetRelativePath(dir,p).Replace('\\','/');if(!expected.ContainsKey(n)||!seen.Add(n))Deny("format-denied");
   }}
   if(seen.Count!=found.Count)Deny("format-denied");
   string toml=Encoding.UTF8.GetString(Control(Path.Combine(dir,"Cargo.toml"),4096));
   CheckToml(toml);
   var members=found.OrderBy(x=>x.Key,StringComparer.Ordinal).Select(x=>(object)new object[]{x.Key,x.Value}).ToArray();
   Row(archiveRows,new Dictionary<string,object>{{"package","fixture@1.0.0"},{"id",identity["id"]},{"bytes",identity["bytes"]},{"sha256",archiveDigest},{"members",members},{"build_script",null},{"proc_macro",null}});
   archives=1;
  }
  void Manifest(string label) {
   var pending=new Stack<string>();pending.Push(Root(label));
   while(pending.Count>0){string dir=Owned(pending.Pop());if(dirs+(archives>0?2:0)>=256)Deny("input-limit");
    Row(dirRows,new Dictionary<string,object>{{"root",label},{"file",(dir==Root(label)?"":Path.GetRelativePath(Root(label),dir).Replace('\\','/'))},{"id",DirectoryId(dir)}});dirs++;
    foreach(string path in Directory.EnumerateFileSystemEntries(dir).OrderBy(x=>x,StringComparer.Ordinal)){
     Owned(path);if((File.GetAttributes(path)&FileAttributes.Directory)!=0){pending.Push(path);continue;}
     if(files+archives>=2048)Deny("input-limit");var value=ReadFile(path);inputBytes+=(long)value["bytes"];if(inputBytes>4L*MiB)Deny("input-limit");
     value.Add("root",label);value.Add("file",Path.GetRelativePath(Root(label),path).Replace('\\','/'));Row(fileRows,value);files++;
    }
   }
  }
  static object[] Sorted(List<object> rows)=>rows.Cast<Dictionary<string,object>>().OrderBy(x=>(string)x["root"],StringComparer.Ordinal).ThenBy(x=>(string)x["file"],StringComparer.Ordinal).Cast<object>().ToArray();
  public void Positive() {
   Emit("archives-begin");Archive();Emit("archives-end");
   foreach(string label in Labels){Emit("root-begin",label);Manifest(label);Emit("root-end",label);}
   Emit("serialize-begin");
   // Re-open every retained name before publication; IDs/digests cannot drift.
   foreach(var row in fileRows.Cast<Dictionary<string,object>>()){
    var current=ReadFile(Path.Combine(Root((string)row["root"]),(string)row["file"]));
    if(Encode(current)!=Encode(row.Where(x=>x.Key!="root"&&x.Key!="file").ToDictionary(x=>x.Key,x=>x.Value)))Deny("identity-drift");
   }
   foreach(var row in dirRows.Cast<Dictionary<string,object>>())if(DirectoryId(Path.Combine(Root((string)row["root"]),(string)row["file"]))!=(string)row["id"])Deny("identity-drift");
   var result=new Dictionary<string,object>{{"files",Sorted(fileRows)},{"directories",Sorted(dirRows)},{"archives",archiveRows.ToArray()},
    {"file_count",files},{"directory_count",dirs},{"file_bytes",inputBytes},{"hash_bytes",Work.Used},
    {"metrics",new Dictionary<string,object>{{"strategy",mode},{"scratch_allocations",Allocations},{"scratch_capacity_bytes",Capacity},{"maximum_leases",MaximumLeases},{"hash_bytes",HashBytes},{"catalog_sha256",catalogHash}}}};
   Publish(result,MiB);Emit("serialize-end");
  }
  public void Publish(object value,int cap) {
   byte[] b=Encoding.UTF8.GetBytes(Encode(value));if(b.Length>cap)Deny("output-limit");
   string p=Path.Combine(root,"outputs",nonce+".json");Owned(Path.GetDirectoryName(p));
   using(var s=new FileStream(p,FileMode.CreateNew,FileAccess.Write,FileShare.None)){s.Write(b);s.Flush(true);}
  }
  static void CheckToml(string toml){if(toml!="[package]\nname = \"fixture\"\nversion = \"1.0.0\"\nbuild = false\n")Deny("format-denied");}
  public object GuardResult(string[] dispositions,string runtimeVersion)=>new Dictionary<string,object>{{"vectors",dispositions},{"h",Work.Used},{"catalog_sha256",catalogHash},{"runtime_version",runtimeVersion}};
  public string Guard(string name) {
   string p=Path.Combine(root,"guards","tiny");
   switch(name){
    case "traversal":MemberName("fixture-1.0.0/../x");break;
    case "absolute":MemberName("/fixture-1.0.0/x");break;
    case "backslash":MemberName("fixture-1.0.0/a\\b");break;
    case "ads":MemberName("fixture-1.0.0/a:b");break;
    case "empty":MemberName("fixture-1.0.0/");break;
    case "dot":MemberName("fixture-1.0.0/./x");break;
    case "double-slash":MemberName("fixture-1.0.0/a//b");break;
    case "nul":MemberName("fixture-1.0.0/a\0");break;
    case "trailing-dot":MemberName("fixture-1.0.0/a.");break;
    case "reparse":{string link=Path.Combine(root,"guards","link");try{File.CreateSymbolicLink(link,p);Owned(link);}finally{File.Delete(link);}break;}
    case "path-limit":MemberName("fixture-1.0.0/"+new string('a',1025));break;
    case "foreign-root":Owned(Path.GetPathRoot(root));break;
    case "unc":Plain("\\\\localhost\\c$\\fixture");break;
    case "file-limit":ReadFile(p,0);break;
    case "short-stream":Hash(new MemoryStream(new byte[]{1}),2);break;
    case "hash-limit":{long used=Work.Used;try{Work.Charge(Work.Maximum+1);}finally{if(Work.Used!=used)Deny("fixture-assertion");}break;}
    case "growth":ReadFile(p,MiB,0);break;
    case "nested-lease":using(var lease=new Lease(this)){Hash(Stream.Null,0);}break;
    case "encoded-limit":Row(new List<object>(),new Dictionary<string,object>{{"oversize",new string('a',64)}},1);break;
    case "wrong-path-identity":using(var s=Open(p)){Identity(s.SafeFileHandle,Path.Combine(root,"guards","other"));}break;
    case "helper":CheckToml("[package]\nname = \"fixture\"\nversion = \"1.0.0\"\nbuild = \"evil.rs\"\n");break;
    case "extra-file":if(Directory.EnumerateFiles(Path.Combine(root,"guards")).Any(x=>!expected.ContainsKey(Path.GetFileName(x)) ))Deny("format-denied");break;
    case "duplicate":case "case-fold":case "type":case "tar-traversal":case "extra":case "content":case "missing":case "wrong-archive":case "member-limit":case "expanded-limit":{
     string fixture=name=="tar-traversal"?"traversal":(name=="wrong-archive"||name=="member-limit"||name=="expanded-limit")?"valid":name;
     string archive=Path.Combine(root,"guards",fixture+".crate");var sha=(string)ReadFile(archive)["sha256"];
     Work.Charge(1);var golden=Convert.ToHexString(SHA256.HashData(new byte[]{97})).ToLowerInvariant();
     var members=new Dictionary<string,string>(StringComparer.Ordinal){{"x",golden}};
     InspectArchive(archive,name=="wrong-archive"?new string('0',64):sha,members,name=="member-limit"?0:MiB,name=="expanded-limit"?1:4L*MiB);break;
    }
    default:Deny("fixture-assertion");break;
   }
   return "accepted";
  }
  public void Dispose(){foreach(var h in held)h.Dispose();held.Clear();if(leases!=0)Deny("fixture-assertion");}
 }
}
'@
    $failureCode=76
    $reader=[OrkReaderDiagnostic.Reader]::new($owned,$Nonce,$mode,$grant)
    try {
        $reader.Setup()
        if($mode -ceq 'guards') {
            . (Join-Path $PSScriptRoot 'pair-reader-diagnostic.test.ps1')
            $dispositions=Invoke-DiagnosticGuards $reader
            $reader.Publish($reader.GuardResult($dispositions,$PSVersionTable.PSVersion.ToString()),4096)
        } else {$reader.Positive()}
        $reader.Dispose()
        $reader.Emit('ok')
        exit 0
    } catch {
        $reason=$_.Exception.GetBaseException().Message
        if($reason -cnotin @('bad-request','input-denied','hash-limit','input-limit','identity-drift','format-denied','output-limit','fixture-assertion','reader-unavailable')) {$reason='reader-unavailable'}
        $reader.Dispose()
        $reader.Emit('error','',$reason)
        exit 1
    }
} catch {
    # Fixed stage status only; no raw exception or invented work report.
    exit $failureCode
}
