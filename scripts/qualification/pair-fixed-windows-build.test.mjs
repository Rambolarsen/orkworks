import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as api from "./pair-fixed-windows-build.mjs";
test("failed source preflight never invokes build or oracle", async () => {
  const calls = [];
  const r = await api.candidateAttempt({
    fetch: async () => {
      calls.push("fetch");
      return 0;
    },
    sources: async () => {
      throw Error("source drift");
    },
    build: async () => {
      calls.push("build");
      return 0;
    },
    oracle: async () => {
      calls.push("oracle");
      return 0;
    },
    inspect: async () => {
      calls.push("inspect");
    },
  });
  assert.equal(r.success, false);
  assert.equal(r.stage, "sources");
  assert.deepEqual(calls, ["fetch"]);
  assert.equal(r.build_invocations, 0);
});
test("build failure is terminal without another build or preprocessing fallback", async () => {
  const calls = [];
  const r = await api.candidateAttempt({
    fetch: async () => 0,
    sources: async () => {},
    build: async () => {
      calls.push("build");
      return 101;
    },
    oracle: async () => {
      calls.push("oracle");
      return 0;
    },
    inspect: async () => {},
  });
  assert.equal(r.success, false);
  assert.equal(r.build_invocations, 1);
  assert.equal(r.oracle_invocations, 0);
  assert.deepEqual(calls, ["build"]);
});
test("successful pipeline still reports zero resulting-program execution", async () => {
  const r = await api.candidateAttempt({
    fetch: async () => 0,
    sources: async () => {},
    build: async () => 0,
    oracle: async () => 0,
    inspect: async () => {},
  });
  assert.equal(r.success, true);
  assert.equal(r.build_invocations, 1);
  assert.equal(r.oracle_invocations, 1);
  assert.equal(r.target_executions, 0);
});
test("candidate environment removes credentials and suppresses system-library discovery", () => {
  const p = {
    rustc: "C:/rust/rustc.exe",
    cargo: "C:/rust/cargo.exe",
    vc: "C:/vc/14.44.35207",
    kit: "C:/kit",
    sdk: "10.0.26100.0",
  };
  const e = api.buildEnvironment(
    {
      SystemRoot: "C:/Windows",
      GH_TOKEN: "secret",
      CL: "bad",
      CFLAGS: "bad",
      VCPKG_ROOT: "foreign",
    },
    p,
    "C:/owned",
  );
  assert.equal(e.GH_TOKEN, undefined);
  assert.equal(e.CL, undefined);
  assert.equal(e.CFLAGS, undefined);
  assert.equal(e.VCPKG_ROOT, undefined);
  assert.equal(e.LIBSQLITE3_FLAGS, "-USQLITE_ENABLE_MEMORY_MANAGEMENT");
  assert.equal(e.LIBGIT2_NO_PKG_CONFIG, "1");
  assert.equal(e.VCPKGRS_DISABLE, "1");
  assert.equal(e.CARGO_ENCODED_RUSTFLAGS.split("\u001f").length, 4);
  assert.equal(e.VSCMD_ARG_TGT_ARCH, "x64");
});
test("macro oracle derives from the actual compiler inputs and removes only output/source options", () => {
  const compiler = "C:\\VC\\cl.exe";
  const args = [
    compiler,
    "-nologo",
    "-MD",
    "-O2",
    "-FoC:\\out\\sqlite3.o",
    "-c",
    "sqlite3/sqlite3.c",
    "-DSQLITE_THREADSAFE=1",
    "-DSQLITE_ENABLE_MEMORY_MANAGEMENT",
    "-USQLITE_ENABLE_MEMORY_MANAGEMENT",
  ];
  const log =
    "[libsqlite3-sys 0.30.1] running: " +
    args.map((x) => JSON.stringify(x)).join(" ");
  const actual = api.sqliteInvocation(log, compiler);
  assert.deepEqual(actual, args);
  assert.deepEqual(api.preprocessArguments(actual, "C:/owned/oracle.c"), [
    "-nologo",
    "-MD",
    "-O2",
    "-DSQLITE_THREADSAFE=1",
    "-DSQLITE_ENABLE_MEMORY_MANAGEMENT",
    "-USQLITE_ENABLE_MEMORY_MANAGEMENT",
    "/EP",
    "/TC",
    "C:/owned/oracle.c",
  ]);
  assert.throws(() => api.sqliteInvocation(log + "\n" + log, compiler));
  assert.throws(() =>
    api.preprocessArguments([...args, "@unknown.rsp"], "oracle.c"),
  );
  assert.throws(() =>
    api.preprocessArguments([...args, "/FIforeign.h"], "oracle.c"),
  );
  assert.throws(() =>
    api.preprocessArguments(
      [...args, "-DSQLITE_ENABLE_MEMORY_MANAGEMENT=0"],
      "oracle.c",
    ),
  );
  const source = api.oracleSource("C:/source/sqlite3.c");
  assert.equal(source.split("#error memory_management_present").length - 1, 2);
  assert.match(source, /#include "C:\/source\/sqlite3.c"/);
});
test("a changed input cannot pass a sealed manifest comparison", (t) => {
  const d = fs.mkdtempSync(path.join(os.tmpdir(), "fixed-build-input-"));
  t.after(() => fs.rmSync(d, { recursive: true, force: true }));
  fs.writeFileSync(path.join(d, "file.h"), "before");
  const m = api.treeManifest({ source: d });
  api.assertManifest(m, { source: d });
  fs.writeFileSync(path.join(d, "file.h"), "after");
  assert.throws(() => api.assertManifest(m, { source: d }));
});
test("oversize staging never publishes an oversized artifact", (t) => {
  const d = fs.mkdtempSync(path.join(os.tmpdir(), "fixed-build-evidence-"));
  t.after(() => fs.rmSync(d, { recursive: true, force: true }));
  const out = path.join(d, "out");
  fs.mkdirSync(out);
  const bad = path.join(d, "large");
  fs.writeFileSync(bad, "123456");
  const result = { success: true };
  api.publishEvidence(out, result, { "diagnostics.txt": bad }, 5);
  assert.equal(result.success, false);
  assert.deepEqual(fs.readdirSync(out), ["result.json"]);
});

test("explicitly disabled automatic build scripts remain disabled", () => {
  assert.equal(
    api.packageBuildScript(
      '[package]\nname = "sample"\nversion = "1"\nbuild = false\n',
      true,
    ),
    undefined,
  );
  assert.equal(
    api.packageBuildScript('[package]\nname = "sample"\nversion = "1"\n', true),
    "build.rs",
  );
});

test("oracle input rejection records zero preprocessing invocations", async () => {
  const r = await api.candidateAttempt({
    fetch: async () => 0,
    sources: async () => {},
    build: async () => 0,
    prepareOracle: async () => {
      throw Error("unknown-input");
    },
    oracle: async () => {
      throw Error("must-not-run");
    },
    inspect: async () => {},
  });
  assert.equal(r.success, false);
  assert.equal(r.build_invocations, 1);
  assert.equal(r.oracle_invocations, 0);
});

test("link custody selects one Cargo artifact and requires its mapped SQLite member", () => {
  const root = "C:\\owned\\target",
    rlib =
      root +
      "\\x86_64-pc-windows-msvc\\release\\deps\\liblibsqlite3_sys-abcd.rlib";
  const record = {
    reason: "compiler-artifact",
    package_id:
      "registry+https://github.com/rust-lang/crates.io-index#libsqlite3-sys@0.30.1",
    target: { name: "libsqlite3_sys", kind: ["lib"] },
    filenames: [rlib],
  };
  assert.equal(api.sqliteRlib(JSON.stringify(record), root), rlib);
  assert.throws(() =>
    api.sqliteRlib(
      JSON.stringify(record) + "\n" + JSON.stringify(record),
      root,
    ),
  );
  const member = "abc-sqlite3.o";
  assert.equal(
    api.assertSqliteMap(
      " 0001:00000000 sqlite3_open 0000000140001000 f liblibsqlite3_sys-abcd.rlib:" +
        member,
      rlib,
      member,
    ).length,
    1,
  );
  assert.throws(() =>
    api.assertSqliteMap("sqlite3_open other.rlib:" + member, rlib, member),
  );
  assert.throws(() =>
    api.assertSqliteMap("sqlite3_open " + member + "\n" + rlib, rlib, member),
  );
});

test("an identically named amalgamation outside the verified crate is rejected", () => {
  const args = ["C:\\vc\\cl.exe", "-c", "sqlite3/sqlite3.c"];
  api.assertSqliteSource(
    args,
    "C:\\cache\\libsqlite",
    "C:\\cache\\libsqlite\\sqlite3\\sqlite3.c",
  );
  assert.throws(() =>
    api.assertSqliteSource(
      [args[0], "-c", "C:\\foreign\\sqlite3\\sqlite3.c"],
      "C:\\cache\\libsqlite",
      "C:\\cache\\libsqlite\\sqlite3\\sqlite3.c",
    ),
  );
});

test("archive extraction uses the exact unique stored member name", () => {
  assert.equal(
    api.archiveMember(
      "Microsoft LIB\nC:\\owned\\abc-sqlite3.o\n",
      "abc-sqlite3.o",
    ),
    "C:\\owned\\abc-sqlite3.o",
  );
  assert.throws(() =>
    api.archiveMember("a/abc-sqlite3.o\nb/abc-sqlite3.o", "abc-sqlite3.o"),
  );
});
test("fixed-build entry denies reruns, a foreign head and a non-dispatch event", () => {
  const sha = "a".repeat(40),
    e = {
      GITHUB_ACTIONS: "true",
      GITHUB_REPOSITORY: "Rambolarsen/orkworks",
      GITHUB_EVENT_NAME: "workflow_dispatch",
      GITHUB_RUN_ATTEMPT: "1",
      EXPECTED_SHA: sha,
      GITHUB_SHA: sha,
      GITHUB_WORKFLOW_REF:
        "Rambolarsen/orkworks/.github/workflows/pair-fixed-windows-build.yml@refs/heads/main",
    };
  api.assertDispatch(e, "win32");
  for (const change of [
    { GITHUB_RUN_ATTEMPT: "2" },
    { GITHUB_SHA: "b".repeat(40) },
    { GITHUB_EVENT_NAME: "pull_request" },
    { GITHUB_WORKFLOW_REF: e.GITHUB_WORKFLOW_REF.replace("main", "other") },
  ])
    assert.throws(() => api.assertDispatch({ ...e, ...change }, "win32"));
  assert.throws(() => api.assertDispatch(e, "darwin"));
});
