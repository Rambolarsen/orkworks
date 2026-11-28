import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
let m;
try {
  m = await import("./pair-progress-windows-build.mjs");
} catch (e) {
  if (e.code !== "ERR_MODULE_NOT_FOUND") throw e;
}
const need = () => assert.ok(m, "progress controller is not implemented");
const sdk = () => ({
  schema: 3,
  evidence_kind: "sdk-positive-build-bracket",
  status: "complete",
  inventory_complete: false,
  enumerated_entries: 0,
  api_calls: 13,
  read_bytes_charged: 1358,
  open_handles: 0,
  hashed_bytes: 1000,
  before_write: ["134399551354776438", "134399565468708431"],
  after_write: ["134399551354776438", "134399565468708431"],
  kits_root: "C:/Program Files (x86)/Windows Kits/10/",
  record: {
    key: "SOFTWARE/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall/{45e335f3-c807-43b8-9ed1-43666cfcf939}",
    name: "Windows Software Development Kit - Windows 10.0.26100.9457",
    version: "10.1.26100.9457",
  },
  digest: "b".repeat(64),
});
test("bounded hashing charges before reads and denies source growth", () => {
  need();
  const d = fs.realpathSync(
      fs.mkdtempSync(path.join(os.tmpdir(), "replacement-hash-")),
    ),
    f = path.join(d, "source");
  try {
    fs.writeFileSync(f, "abc");
    const b = new m.WorkBudget({ hashLimit: 3 });
    assert.equal(
      m.boundedHash(f, b),
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );
    assert.throws(() => m.boundedHash(f, b), /hash-limit/);
    assert.equal(b.hashed, 3);
    assert.throws(() => m.boundedHash(f, new m.WorkBudget(), 2), /file-limit/);
    const original = fs.readSync;
    try {
      fs.readSync = (...a) => {
        fs.appendFileSync(f, "x");
        return original(...a);
      };
      const budget = new m.WorkBudget();
      assert.throws(() => m.boundedHash(f, budget), /file-drift/);
      assert.equal(budget.hashed, 3);
    } finally {
      fs.readSync = original;
    }
  } finally {
    fs.rmSync(d, { recursive: true, force: true });
  }
});
test("SDK ledger shares remaining API/read/hash quotas and rejects drift or third pass", () => {
  need();
  const b = new m.WorkBudget(),
    s = new m.SdkLedger(b);
  s.add(sdk());
  assert.deepEqual(s.remaining(), { api: 19, read: 2738, hash: 31768 });
  s.add(sdk());
  const out = s.finish();
  assert.equal(out.success, true);
  assert.equal(out.inventory_complete, false);
  assert.equal(out.api_calls, 26);
  assert.equal(out.read_bytes_charged, 2716);
  assert.throws(() => s.add(sdk()), /sdk-pass-count/);
  const bad = new m.SdkLedger(new m.WorkBudget());
  bad.add(sdk());
  const p = sdk();
  p.digest = "c".repeat(64);
  assert.throws(() => bad.add(p), /sdk-drift/);
});
test("foreign SDK fields, old schema, leaked values or outstanding handles cannot admit", () => {
  need();
  for (const mutate of [
    (x) => (x.schema = 2),
    (x) => (x.record.version = "other"),
    (x) => (x.record.key = "foreign"),
    (x) => (x.open_handles = 1),
    (x) => (x.enumerated_entries = 1),
    (x) => (x.inventory_complete = true),
    (x) => (x.api_calls = 14),
    (x) => (x.read_bytes_charged = 1359),
  ]) {
    const x = sdk();
    mutate(x);
    assert.throws(() => new m.SdkLedger(new m.WorkBudget()).add(x));
  }
});
test("ordered stages stop once with no later fetch/build/oracle on denial", async () => {
  need();
  const names = [
    "preflight",
    "sdkBefore",
    "versions",
    "fetch",
    "sources",
    "sealBefore",
    "build",
    "prepareOracle",
    "oracle",
    "inspect",
    "sdkAfter",
    "sealAfter",
  ];
  for (const failed of names) {
    const calls = [];
    const io = Object.fromEntries(
      names.map((n) => [
        n,
        async () => {
          calls.push(n);
          if (n === failed) throw Error("fixture-denial");
        },
      ]),
    );
    const r = await m.runStages(io);
    assert.equal(r.success, false);
    assert.equal(r.candidate_evidence_complete, false);
    assert.deepEqual(calls, names.slice(0, names.indexOf(failed) + 1));
    assert.ok(
      r.fetch_invocations <= 1 &&
        r.build_invocations <= 1 &&
        r.oracle_invocations <= 1,
    );
    assert.equal(r.target_executions, 0);
  }
});
test("completed ordered stages do not qualify runtime or production pins", async () => {
  need();
  const r = await m.runStages(
    Object.fromEntries(
      [
        "preflight",
        "sdkBefore",
        "versions",
        "fetch",
        "sources",
        "sealBefore",
        "build",
        "prepareOracle",
        "oracle",
        "inspect",
        "sdkAfter",
        "sealAfter",
      ].map((n) => [n, async () => {}]),
    ),
  );
  assert.equal(r.success, true);
  assert.equal(r.build_invocations, 1);
  assert.equal(r.oracle_invocations, 1);
  assert.equal(r.runtime_qualified, false);
  assert.equal(r.production_pin_accepted, false);
});

test("evidence overflow publishes only a false receipt within the bound", () => {
  need();
  const d = fs.mkdtempSync(path.join(os.tmpdir(), "replacement-publish-"));
  try {
    const source = path.join(d, "input");
    fs.writeFileSync(source, "x".repeat(1000));
    const out = path.join(d, "out");
    fs.mkdirSync(out);
    const r = { success: true, candidate_evidence_complete: true };
    m.publishEvidence(
      out,
      r,
      { "inputs.json": source },
      new m.WorkBudget(),
      500,
    );
    assert.deepEqual(fs.readdirSync(out), ["result.json"]);
    const p = JSON.parse(fs.readFileSync(path.join(out, "result.json")));
    assert.equal(p.success, false);
    assert.equal(p.candidate_evidence_complete, false);
    assert.equal(p.runtime_qualified, false);
  } finally {
    fs.rmSync(d, { recursive: true, force: true });
  }
});
test("manifest admission rejects foreign membership, excessive work and mutable file identity", () => {
  need();
  const value = () => ({
    success: true,
    hash_bytes: 3,
    file_count: 1,
    directory_count: 1,
    file_bytes: 3,
    files: [
      {
        root: "source",
        file: "a.txt",
        bytes: 3,
        sha256: "a".repeat(64),
        id: "00000000:0000000000000001",
      },
    ],
    directories: [
      { root: "source", file: "", id: "00000000:0000000000000002" },
    ],
    crates: [],
  });
  const a = m.validateManifest(value(), ["source"]);
  assert.equal(a.files.length, 1);
  for (const change of [
    (x) => (x.files[0].file = "../escape"),
    (x) => (x.files[0].root = "foreign"),
    (x) => x.files.push(x.files[0]),
    (x) => (x.file_count = 200001),
    (x) => (x.files[0].bytes = 512 * 1024 ** 2 + 1),
    (x) => (x.files[0].id = "unknown"),
  ]) {
    const x = value();
    change(x);
    assert.throws(() => m.validateManifest(x, ["source"]));
  }
  const b = value();
  b.files[0].id = "00000000:0000000000000003";
  assert.throws(
    () => m.sameManifest(a, m.validateManifest(b, ["source"])),
    /input-drift/,
  );
});
test("delegated hash grants retain all possible work on missing reports", () => {
  need();
  const b = new m.WorkBudget({ hashLimit: 10 }),
    finish = b.reserve(8);
  assert.throws(() => b.hash(3), /hash-limit/);
  finish(5);
  b.hash(5);
  assert.equal(b.hashed, 10);
  assert.throws(() => finish(0), /reservation-invalid/);
});
test("all twelve PE pins and selected numeric versions must match before tool execution", () => {
  need();
  const p = JSON.parse(
    fs.readFileSync(
      new URL("./pair-replacement-windows-build.inputs.json", import.meta.url),
    ),
  );
  const tools = Object.fromEntries(
    Object.entries(p.tools).map(([n, h]) => [
      n,
      {
        sha256: h,
        machine: 34404,
        version: p.versions[n] ?? null,
        bytes: 1,
        id: "00000000:0000000000000001",
      },
    ]),
  );
  m.validateTools({ success: true, tools });
  for (const change of [
    (t) => delete t["c2.dll"],
    (t) => (t["cl.exe"].sha256 = "0".repeat(64)),
    (t) => (t["rustc.exe"].machine = 332),
    (t) => (t["cl.exe"].version = "19.44.35229.0"),
    (t) => (t.extra = {}),
  ]) {
    const t = structuredClone(tools);
    change(t);
    assert.throws(() => m.validateTools({ success: true, tools: t }));
  }
});

test("diagnostic redaction preserves labels in plain and JSON-escaped paths", () => {
  const roots = {
    source: "D:\\a\\orkworks\\orkworks",
    scratch: "D:\\a\\_temp\\owned",
    home: "C:\\Users\\runneradmin",
  };
  const original =
    "D:\\a\\orkworks\\orkworks\\crate C:/Users/runneradmin/tool " +
    JSON.stringify("D:\\a\\_temp\\owned\\target");
  const result = m.redactDiagnostics(original, roots);
  assert.equal(result, '[source]\\crate [home]/tool "[scratch]\\\\target"');
  assert.ok(!result.includes("runneradmin"));
});

test('output binding and closed admission precede known settlement', async () => {
 need();const {Decoder,WorkBudget,phases}=await import('./pair-progress-windows-build-protocol.mjs');
 const nonce='a'.repeat(32), policy='1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6';
 const ledger=new WorkBudget(),d=new Decoder({nonce,mode:'preflight',grant:100,ledger});
 for(const [i,p] of ['start','ready','serialize-begin','serialize-end','ok'].entries())d.feed(Buffer.from(JSON.stringify([2,nonce,i+1,p,'',10,0,0,0])+'\n'));
 const data={success:true,tools:{},powershell_version:'7.6.1',hash_bytes:10};
 const v={schema:1,evidence_kind:'progress-reader-output',mode:'preflight',nonce,policy_sha256:policy,data,metrics:{strategy:'reuse',scratch_allocations:1,scratch_capacity_bytes:1048576,maximum_leases:1,hash_bytes:3}};
 // Full policy tools validation is mandatory, even for a framed ok.
 assert.throws(()=>m.validateOutput(v,d));
 assert.throws(()=>m.validateOutput({...v,nonce:'b'.repeat(32)},d));
 assert.throws(()=>m.validateOutput({...v,extra:1},d));
 for(const changed of [{policy_sha256:'b'.repeat(64)},{mode:'before'},{schema:2},{evidence_kind:'reader-diagnostic'}])assert.throws(()=>m.validateOutput({...v,...changed},d));
});

test('streaming runner retains prefix before close and charges late overflow',async()=>{
 need();const {EventEmitter}=await import('node:events');const {Decoder,WorkBudget}=await import('./pair-progress-windows-build-protocol.mjs');
 const b=new WorkBudget({streamLimit:300}),nonce='a'.repeat(32),d=new Decoder({nonce,mode:'before',grant:20,ledger:b});
 let child;const spawnProcess=()=>{child=new EventEmitter();child.stdout=new EventEmitter();child.stderr=new EventEmitter();child.pid=42;child.exitCode=null;child.signalCode=null;return child;};
 const r=new m.CommandRunner(b,{spawnProcess,terminate:()=>{}}),promise=r.run('owned',[],{decoder:d,timeout:1000});
 child.stdout.emit('data',Buffer.from(JSON.stringify([2,nonce,1,'start','',0,0,0,0])+'\n'));assert.equal(d.trace.length,1);
 child.stderr.emit('data',Buffer.from('denied'));child.stdout.emit('data',Buffer.alloc(400,65));child.emit('close',1);
 await assert.rejects(promise);assert.ok(b.streamed>400);assert.equal(d.trace.length,1);assert.ok(r.drained);assert.equal(r.kills,1);
});
test('spawn crossing its deadline fails and forbids another stage',async()=>{
 need();const {EventEmitter}=await import('node:events');let clock=0;const b=new m.WorkBudget({clock:()=>clock});let child;
 const r=new m.CommandRunner(b,{spawnProcess:()=>{clock=2000;child=new EventEmitter();child.stdout=new EventEmitter();child.stderr=new EventEmitter();child.pid=43;child.exitCode=null;child.signalCode=null;return child;},terminate:()=>queueMicrotask(()=>child.emit('close',1))});
 await assert.rejects(()=>r.run('owned',[],{timeout:1000}),/time-limit/);assert.equal(r.stopped,true);assert.equal(r.drained,true);
});

test('held output rejects duplicate keys, malformed UTF-8 and policy schema aliases',async()=>{
 need();const d=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'progress-output-'))),f=path.join(d,'output');
 try{for(const raw of [Buffer.from('{"x":1,"x":2}'),Buffer.from([123,34,120,34,58,34,255,34,125])]){fs.writeFileSync(f,raw);assert.throws(()=>m.readOutput(f,new m.WorkBudget()));}}
 finally{fs.rmSync(d,{recursive:true});}
});

test('expired publication reserve returns explicit unavailable state',()=>{
 need();let now=0;const b=new m.WorkBudget({clock:()=>now,seconds:1});now=121000;
 const result={success:true,candidate_evidence_complete:true};assert.equal(m.publicationAvailable(b,result),false);
 assert.equal(result.publication,'unavailable-reserve-expired');assert.equal(result.success,false);assert.equal(result.candidate_evidence_complete,false);
});
