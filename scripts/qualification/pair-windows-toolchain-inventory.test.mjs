import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as api from "./pair-windows-toolchain-inventory.mjs";
const profile = "C:\\Users\\runneradmin";
test("inventory selects exactly the thirteen approved data-only inspection targets", () => {
  assert.equal(typeof api.targets, "function");
  const rows = api.targets(profile);
  assert.deepEqual(
    rows.map((x) => x.name),
    [
      "cargo.exe",
      "rustc.exe",
      "cl.exe",
      "link.exe",
      "lib.exe",
      "dumpbin.exe",
      "c1.dll",
      "c1xx.dll",
      "c2.dll",
      "mspdbcore.dll",
      "rc.exe",
      "mt.exe",
      "devenv.exe",
    ],
  );
  assert.equal(
    rows.find((x) => x.name === "cl.exe").path,
    "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.44.35207\\bin\\Hostx64\\x64\\cl.exe",
  );
  assert.throws(() => api.targets("\\\\foreign\\profile"));
});
const fixturePins = {
  "cargo.exe":
    "f381bae27c59588eacf69208e115b2c001a2998afed83a2e9084ea93620737b1",
  "rustc.exe":
    "d4f8a0283de2dba0dc458cd13c333337d2b76c3649eb39e91c4acce618e661a1",
  "cl.exe": "e87e78b3ef6580b75aebda843f8a22e874612d9f766344046551399f898eb226",
};
const complete = () => ({
  files: api.targets(profile).map((t, i) => ({
    ...t,
    status: "ok",
    bytes: 100,
    sha256: fixturePins[t.name] ?? "a".repeat(64),
    final_sha256: fixturePins[t.name] ?? "a".repeat(64),
    machine: "8664",
    file_version: "19.44.35229.0",
    product_version: null,
  })),
  hashed_bytes: 2600,
  hash_passes: 26,
  sdk: {
    status: "ok",
    root: "C:\\Program Files (x86)\\Windows Kits\\10",
    directories: [true, true, true],
    scanned_entries: 100,
    products: [
      { name: "Windows Software Development Kit", version: "10.1.26100.9457" },
    ],
  },
  default_toolset: { status: "missing" },
});
test("a complete inventory supplies metadata only, never build or execution authority", () => {
  assert.deepEqual(api.problems(complete(), profile), []);
});
test("a third compiler hash remains visible while co-availability is denied", async () => {
  const data = complete();
  data.files[2].sha256 = data.files[2].final_sha256 = "b".repeat(64);
  const r = await api.collectOnce(
    async () => ({ code: 0, stdout: JSON.stringify(data), stderr: "" }),
    profile,
  );
  assert.equal(r.success, false);
  assert.equal(r.inventory.files[2].sha256, "b".repeat(64));
  assert.equal(r.inspected_tool_executions, 0);
  assert.equal(r.controller_invocations, 1);
});
test("missing input preserves all other observations and never invokes a fallback", async () => {
  const data = complete();
  data.files[4] = { ...data.files[4], status: "missing" };
  let count = 0;
  const r = await api.collectOnce(async () => {
    count++;
    return { code: 0, stdout: JSON.stringify(data), stderr: "" };
  }, profile);
  assert.equal(r.success, false);
  assert.equal(r.inventory.files.length, 13);
  assert.equal(count, 1);
});
test("post-collection drift and exceeded hash/registry limits cannot pass", () => {
  let data = complete();
  data.files[2].final_sha256 = "b".repeat(64);
  assert.ok(api.problems(data, profile).length);
  data = complete();
  data.hashed_bytes = 1024 ** 3 + 1;
  assert.ok(api.problems(data, profile).length);
  data = complete();
  data.sdk.scanned_entries = 257;
  assert.ok(api.problems(data, profile).length);
});
test("unapproved target paths cannot be substituted under an approved filename", () => {
  const d = complete();
  d.files[2].path = "C:\\foreign\\cl.exe";
  assert.ok(api.problems(d, profile).length);
});
test("controller failures or invalid JSON are terminal and do not report success", async () => {
  for (const x of [
    { code: 1, stdout: "{}", stderr: "" },
    { code: 0, stdout: "truncated", stderr: "" },
  ]) {
    let calls = 0;
    const r = await api.collectOnce(async () => {
      calls++;
      return x;
    }, profile);
    assert.equal(r.success, false);
    assert.equal(calls, 1);
    assert.equal(r.target_executions, 0);
  }
});
test("the collector environment omits credentials, configuration and executable search paths", () => {
  const e = api.childEnvironment(
    {
      SystemRoot: "C:\\Windows",
      USERPROFILE: profile,
      TEMP: "C:\\temp",
      GITHUB_TOKEN: "secret",
      CL: "bad",
      PATH: "foreign",
    },
    1234,
  );
  assert.equal(e.GITHUB_TOKEN, undefined);
  assert.equal(e.CL, undefined);
  assert.equal(e.PATH, "C:\\Windows\\System32");
  assert.equal(e.ORK_PAIR_HASH_BUDGET, "1234");
});
test("oversize evidence publishes only a bounded failure receipt", (t) => {
  const d = fs.mkdtempSync(path.join(os.tmpdir(), "inventory-evidence-"));
  t.after(() => fs.rmSync(d, { recursive: true, force: true }));
  const result = { success: true, run_id: "1" };
  api.publish(d, result, "x".repeat(1000), "", 500);
  assert.equal(result.success, false);
  assert.deepEqual(fs.readdirSync(d), ["result.json"]);
  assert.equal(
    JSON.parse(fs.readFileSync(path.join(d, "result.json"))).reason,
    "evidence-limit",
  );
});
test("foreign dispatches and reruns are denied before collection", () => {
  const sha = "a".repeat(40),
    e = {
      GITHUB_ACTIONS: "true",
      GITHUB_REPOSITORY: "Rambolarsen/orkworks",
      GITHUB_EVENT_NAME: "workflow_dispatch",
      GITHUB_RUN_ATTEMPT: "1",
      EXPECTED_SHA: sha,
      GITHUB_SHA: sha,
      GITHUB_WORKFLOW_REF:
        "Rambolarsen/orkworks/.github/workflows/pair-windows-toolchain-inventory.yml@refs/heads/main",
    };
  api.assertDispatch(e, "win32");
  for (const c of [
    { GITHUB_RUN_ATTEMPT: "2" },
    { GITHUB_EVENT_NAME: "pull_request" },
    { GITHUB_SHA: "b".repeat(40) },
  ])
    assert.throws(() => api.assertDispatch({ ...e, ...c }, "win32"));
});

test("a thrown controller launch remains one terminal attempt with zero target execution", async () => {
  const r = await api.collectOnce(async () => {
    throw Error("spawn denied");
  }, profile);
  assert.equal(r.success, false);
  assert.equal(r.controller_invocations, 1);
  assert.equal(r.target_executions, 0);
});
test("reported hash work cannot omit the required two file passes", () => {
  const rows = api.targets(profile).map((t) => ({
    ...t,
    status: "ok",
    bytes: 100,
    sha256: "a".repeat(64),
    final_sha256: "a".repeat(64),
    machine: "8664",
    file_version: "19.44.35229.0",
    product_version: null,
  }));
  assert.ok(
    api
      .problems(
        {
          files: rows,
          hashed_bytes: 0,
          hash_passes: 26,
          sdk: {
            status: "ok",
            root: "C:/Program Files (x86)/Windows Kits/10",
            directories: [true, true, true],
            scanned_entries: 0,
            products: [],
          },
        },
        profile,
      )
      .includes("hash-work-unaccounted"),
  );
});

test("an over-limit default metadata file denies the inventory even though it cannot select a compiler", () => {
  const d = complete();
  d.default_toolset = { status: "text-limit" };
  assert.ok(api.problems(d, profile).includes("default-text-unavailable"));
});

test("a controller that never finishes is terminated and drained within its timeout", async () => {
  const started = Date.now();
  const r = await api.runBounded(
    process.execPath,
    ["-e", "setTimeout(()=>{},10000)"],
    {
      timeout: 30,
      env: { SystemRoot: process.env.SystemRoot, WINDIR: process.env.WINDIR },
      cwd: process.cwd(),
    },
  );
  assert.equal(r.code, -1);
  assert.equal(r.failure, "time-limit");
  assert.ok(Date.now() - started < 5000);
});
test("controller output overflow cannot be returned as a truncated successful inventory", async () => {
  const r = await api.runBounded(
    process.execPath,
    ["-e", "process.stdout.write('x'.repeat(10000));setTimeout(()=>{},10000)"],
    {
      timeout: 3000,
      stdoutLimit: 1000,
      env: { SystemRoot: process.env.SystemRoot, WINDIR: process.env.WINDIR },
      cwd: process.cwd(),
    },
  );
  assert.equal(r.code, -1);
  assert.equal(r.failure, "stdout-limit");
  assert.equal(r.stdout, "");
});

test("a registry budget failure is retained without falsely reporting a changed SDK root", () => {
  const d = complete();
  d.sdk.status = "registry-budget";
  d.sdk.scanned_entries = 0;
  const errors = api.problems(d, profile);
  assert.ok(errors.includes("sdk-unavailable"));
  assert.ok(!errors.includes("sdk-root"));
});
