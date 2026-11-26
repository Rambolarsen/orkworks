import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import {
  assertFirstRun,
  assertConfiguration,
  assertLock,
} from "./pair-windows-metadata-evidence.mjs";
const policy = JSON.parse(
  fs.readFileSync(
    new URL("./pair-fixed-windows-build.inputs.json", import.meta.url),
    "utf8",
  ),
);
const need = (v, m) => {
  if (!v) throw Error(m);
};
const digest = (v) => crypto.createHash("sha256").update(v).digest("hex");
function fileHash(file) {
  const fd = fs.openSync(file, "r"),
    h = crypto.createHash("sha256"),
    b = Buffer.alloc(1024 * 1024);
  try {
    let n;
    while ((n = fs.readSync(fd, b, 0, b.length, null)))
      h.update(b.subarray(0, n));
    return h.digest("hex");
  } finally {
    fs.closeSync(fd);
  }
}
const win = (v) => path.win32.normalize(v).toLowerCase();
const isSqlite = (v) => v.replaceAll("\\", "/").endsWith("sqlite3/sqlite3.c");
const names = [
  "inputs.json",
  "cargo.jsonl",
  "diagnostics.txt",
  "oracle.c",
  "oracle.i",
  "sqlite3.lib",
  "orkworksd.exe",
  "candidate.map",
  "dependencies.txt",
  "archive-members.txt",
  "compiler.json",
  "libsqlite3_sys.rlib",
  "link-custody.json",
];
export function assertDispatch(e, platform) {
  need(
    platform === "win32" &&
      e.GITHUB_ACTIONS === "true" &&
      e.GITHUB_REPOSITORY === "Rambolarsen/orkworks" &&
      e.GITHUB_EVENT_NAME === "workflow_dispatch" &&
      e.GITHUB_RUN_ATTEMPT === "1" &&
      /^[a-f0-9]{40}$/.test(e.EXPECTED_SHA ?? "") &&
      e.GITHUB_SHA === e.EXPECTED_SHA &&
      e.GITHUB_WORKFLOW_REF ===
        "Rambolarsen/orkworks/.github/workflows/pair-fixed-windows-build.yml@refs/heads/main",
    "dispatch-denied",
  );
}
export function buildEnvironment(inherited, p, scratch) {
  const e = {};
  for (const k of ["SystemRoot", "WINDIR", "TEMP", "TMP"])
    if (inherited[k]) e[k] = inherited[k];
  const vcbin = path.join(p.vc, "bin/Hostx64/x64"),
    sdkbin = path.join(p.kit, "bin", p.sdk, "x64");
  return {
    ...e,
    PATH: [
      path.dirname(p.rustc),
      vcbin,
      sdkbin,
      path.join(e.SystemRoot ?? "", "System32"),
    ].join(path.delimiter),
    RUSTC: p.rustc,
    CARGO_HOME: path.join(scratch, "cargo-home"),
    CARGO_TARGET_DIR: path.join(scratch, "target"),
    CARGO_NET_RETRY: "0",
    CARGO_HTTP_TIMEOUT: "30",
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL: "sparse",
    CARGO_TERM_COLOR: "never",
    CC: path.join(vcbin, "cl.exe"),
    CXX: path.join(vcbin, "cl.exe"),
    AR: path.join(vcbin, "lib.exe"),
    CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER: path.join(vcbin, "link.exe"),
    CARGO_ENCODED_RUSTFLAGS: [
      "-C",
      "link-arg=/MAP:" + path.join(scratch, "candidate.map"),
      "-C",
      "link-arg=/VERBOSE:LIB",
    ].join(""),
    LIBSQLITE3_FLAGS: "-USQLITE_ENABLE_MEMORY_MANAGEMENT",
    CC_ENABLE_DEBUG_OUTPUT: "1",
    LIBGIT2_NO_PKG_CONFIG: "1",
    VCPKGRS_DISABLE: "1",
    VCINSTALLDIR: path.dirname(path.dirname(path.dirname(p.vc))) + path.sep,
    VCToolsInstallDir: p.vc + path.sep,
    VCToolsVersion: "14.44.35207",
    VSCMD_ARG_TGT_ARCH: "x64",
    WindowsSdkDir: p.kit + path.sep,
    WindowsSDKVersion: p.sdk + path.sep,
    UniversalCRTSdkDir: p.kit + path.sep,
    UCRTVersion: p.sdk,
    VSLANG: "1033",
    INCLUDE: [
      path.join(p.vc, "include"),
      ...["ucrt", "shared", "um", "winrt", "cppwinrt"].map((d) =>
        path.join(p.kit, "Include", p.sdk, d),
      ),
    ].join(";"),
    LIB: [
      path.join(p.vc, "lib/x64"),
      ...["ucrt", "um"].map((d) => path.join(p.kit, "Lib", p.sdk, d, "x64")),
    ].join(";"),
  };
}
export function treeManifest(roots) {
  const entries = [];
  for (const [label, root] of Object.entries(roots)) {
    const walk = (dir, relative = "") => {
      for (const n of fs.readdirSync(dir).sort()) {
        if (n === ".git") continue;
        const f = path.join(dir, n),
          r = relative ? relative + "/" + n : n,
          s = fs.lstatSync(f);
        need(!s.isSymbolicLink(), "linked-input");
        if (s.isDirectory()) walk(f, r);
        else {
          need(s.isFile(), "unknown-input");
          entries.push({
            root: label,
            file: r,
            bytes: s.size,
            sha256: fileHash(f),
          });
        }
      }
    };
    walk(root);
  }
  return entries;
}
export function assertManifest(before, roots) {
  need(
    JSON.stringify(before) === JSON.stringify(treeManifest(roots)),
    "input-drift",
  );
}
function tokens(s) {
  const out = [];
  let p = 0;
  while (p < s.length) {
    while (s[p] === " ") p++;
    if (p === s.length) break;
    const re = /"(?:[^"\\]|\\.)*"/y;
    re.lastIndex = p;
    const m = re.exec(s);
    need(m, "unknown-command-format");
    out.push(JSON.parse(m[0]));
    p = re.lastIndex;
  }
  return out;
}
export function sqliteInvocation(log, compiler) {
  const hits = [];
  for (const line of log.split(/\r?\n/)) {
    const n = line.indexOf("running: ");
    if (n < 0 || !line.includes("sqlite3.c")) continue;
    const a = tokens(line.slice(n + 9).trim());
    if (a.some(isSqlite)) {
      need(
        win(a[0]) === win(compiler) && a.includes("-c"),
        "foreign-sqlite-compiler",
      );
      hits.push(a);
    }
  }
  need(hits.length === 1, "ambiguous-sqlite-command");
  return hits[0];
}
export function assertSqliteSource(argv, cwd, expected) {
  const sources = argv.slice(1).filter(isSqlite);
  need(
    sources.length === 1 &&
      win(path.win32.resolve(cwd, sources[0])) === win(expected),
    "foreign-sqlite-source",
  );
}
export function preprocessArguments(argv, oracle) {
  let source = 0,
    compile = 0;
  const out = [],
    defs = new Map();
  const allowed = new Set([
    "SQLITE_CORE",
    "SQLITE_DEFAULT_FOREIGN_KEYS=1",
    "SQLITE_ENABLE_API_ARMOR",
    "SQLITE_ENABLE_COLUMN_METADATA",
    "SQLITE_ENABLE_DBSTAT_VTAB",
    "SQLITE_ENABLE_FTS3",
    "SQLITE_ENABLE_FTS3_PARENTHESIS",
    "SQLITE_ENABLE_FTS5",
    "SQLITE_ENABLE_JSON1",
    "SQLITE_ENABLE_LOAD_EXTENSION=1",
    "SQLITE_ENABLE_MEMORY_MANAGEMENT",
    "SQLITE_ENABLE_RTREE",
    "SQLITE_ENABLE_STAT4",
    "SQLITE_SOUNDEX",
    "SQLITE_THREADSAFE=1",
    "SQLITE_USE_URI",
    "HAVE_USLEEP=1",
    "HAVE_ISNAN",
    "_POSIX_THREAD_SAFE_FUNCTIONS",
  ]);
  for (const a of argv.slice(1)) {
    if (isSqlite(a)) {
      source++;
      continue;
    }
    if (/^[-/]c$/i.test(a)) {
      compile++;
      continue;
    }
    if (/^[-/]Fo.+/.test(a) || /^[-/]Fd.+/.test(a)) continue;
    const d = /^[-/]D(.+)$/.exec(a),
      u = /^[-/]U(.+)$/.exec(a);
    if (d) {
      need(allowed.has(d[1]), "unknown-define");
      const [k, v = "1"] = d[1].split("=");
      defs.set(k, v);
      out.push(a);
      continue;
    }
    if (u) {
      need(u[1] === "SQLITE_ENABLE_MEMORY_MANAGEMENT", "unknown-undefine");
      defs.delete(u[1]);
      out.push(a);
      continue;
    }
    need(/^[-/](?:nologo|MD|O2|Brepro|W0)$/.test(a), "unknown-compiler-option");
    out.push(a);
  }
  need(
    source === 1 &&
      compile === 1 &&
      !defs.has("SQLITE_ENABLE_MEMORY_MANAGEMENT") &&
      defs.get("SQLITE_THREADSAFE") === "1",
    "macro-input-mismatch",
  );
  return [...out, "/EP", "/TC", oracle];
}
export function sqliteRlib(log, target) {
  const hits = [];
  for (const line of log.split(/\r?\n/)) {
    let record;
    try {
      record = JSON.parse(line);
    } catch {
      continue;
    }
    if (
      record.reason === "compiler-artifact" &&
      record.package_id ===
        "registry+https://github.com/rust-lang/crates.io-index#libsqlite3-sys@0.30.1" &&
      record.target?.name === "libsqlite3_sys" &&
      record.target.kind?.includes("lib")
    )
      hits.push(...record.filenames.filter((f) => f.endsWith(".rlib")));
  }
  need(
    hits.length === 1 && win(hits[0]).startsWith(win(target) + path.win32.sep),
    "ambiguous-sqlite-rlib",
  );
  return hits[0];
}
export function archiveMember(listing, expected) {
  const found = listing
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => path.win32.basename(l) === expected);
  need(found.length === 1, "ambiguous-archive-member");
  return found[0];
}
export function assertSqliteMap(map, rlib, member) {
  const library = path.win32.basename(rlib).toLowerCase(),
    object = path.win32.basename(member).toLowerCase();
  const origins = new Set([
    library + ":" + object,
    library.replace(/\.rlib$/, "") + ":" + object,
  ]);
  const rows = map
    .split(/\r?\n/)
    .filter((l) =>
      /^\s*[a-f0-9]{4}:[a-f0-9]{8,16}\s+_?sqlite3_[^\s]+\s+/i.test(l),
    );
  need(
    rows.length > 0 &&
      rows.every((l) =>
        origins.has(l.trim().split(/\s+/).at(-1).toLowerCase()),
      ),
    "sqlite-map-association-unavailable",
  );
  return rows;
}
export function oracleSource(source) {
  const check =
    "#if defined(SQLITE_ENABLE_MEMORY_MANAGEMENT)\n#error memory_management_present\n#endif\n#if !defined(SQLITE_THREADSAFE) || SQLITE_THREADSAFE != 1\n#error threadsafe_mismatch\n#endif\n";
  need(!/["\r\n]/.test(source), "invalid-source-path");
  return check + '#include "' + source.replaceAll("\\", "/") + '"\n' + check;
}
export async function candidateAttempt(io) {
  const r = {
    success: false,
    stage: "fetch",
    fetch_invocations: 0,
    build_invocations: 0,
    oracle_invocations: 0,
    target_executions: 0,
  };
  try {
    r.fetch_invocations++;
    r.fetch_exit = await io.fetch();
    need(r.fetch_exit === 0, "fetch-failed");
    r.stage = "sources";
    await io.sources();
    r.stage = "build";
    r.build_invocations++;
    r.build_exit = await io.build();
    need(r.build_exit === 0, "build-failed");
    r.stage = "oracle-inputs";
    await io.prepareOracle?.();
    r.stage = "oracle";
    r.oracle_invocations++;
    r.oracle_exit = await io.oracle();
    need(r.oracle_exit === 0, "oracle-failed");
    r.stage = "inspect";
    await io.inspect();
    r.stage = "complete";
    r.success = true;
  } catch (e) {
    r.reason =
      String(e.message)
        .replace(/[^a-z-]/g, "")
        .slice(0, 80) || "unavailable";
  }
  return r;
}
export function publishEvidence(
  directory,
  result,
  files,
  limit = 256 * 1024 * 1024,
) {
  const text = JSON.stringify(result, null, 2);
  const entries = Object.entries(files);
  if (
    entries.some(([n, f]) => !names.includes(n) || !fs.statSync(f).isFile()) ||
    entries.reduce(
      (n, [, f]) => n + fs.statSync(f).size,
      Buffer.byteLength(text) + 4096,
    ) > limit
  ) {
    result.success = false;
    result.reason = "evidence-limit";
    fs.writeFileSync(
      path.join(directory, "result.json"),
      JSON.stringify({
        schema: 1,
        success: false,
        reason: "evidence-limit",
        run_id: result.run_id,
        implementation_sha: result.implementation_sha,
        fetch_invocations: result.fetch_invocations,
        build_invocations: result.build_invocations,
        oracle_invocations: result.oracle_invocations,
      }),
    );
    return;
  }
  for (const [n, f] of entries) fs.copyFileSync(f, path.join(directory, n));
  fs.writeFileSync(path.join(directory, "result.json"), text);
}
function checkPins(root) {
  for (const [f, h] of Object.entries(policy.source_pins))
    need(fileHash(path.join(root, f)) === h, "source-input-drift");
}
function readPackage(t) {
  const s = t.split(/^\[package\]\r?$/m)[1]?.split(/^\[/m)[0];
  need(s, "unknown-manifest");
  const get = (k) =>
    new RegExp("^" + k + ' = "([^"\\r\\n]+)"$', "m").exec(s)?.[1];
  return {
    name: get("name"),
    version: get("version"),
    script: /^build = false$/m.test(s) ? false : get("build"),
  };
}
export function packageBuildScript(text, hasDefault) {
  const { script } = readPackage(text);
  return script === false
    ? undefined
    : (script ?? (hasDefault ? "build.rs" : undefined));
}
function verifiedSources(cache, lock) {
  const locks = new Map();
  for (const b of lock.split("[[package]]").slice(1)) {
    const get = (k) =>
      new RegExp("^" + k + ' = "([^"\\r\\n]+)"$', "m").exec(b)?.[1];
    if (get("source"))
      locks.set(get("name") + "@" + get("version"), get("checksum"));
  }
  const sourceBase = path.join(cache, "registry/src");
  const roots = fs.readdirSync(sourceBase);
  need(roots.length === 1, "registry-identity");
  const manifests = [],
    dirs = {};
  for (const dir of fs.readdirSync(path.join(sourceBase, roots[0])).sort()) {
    const root = path.join(sourceBase, roots[0], dir);
    const info = readPackage(
        fs.readFileSync(path.join(root, "Cargo.toml"), "utf8"),
      ),
      key = info.name + "@" + info.version;
    const checksum = JSON.parse(
      fs.readFileSync(path.join(root, ".cargo-checksum.json"), "utf8"),
    );
    need(locks.get(key) === checksum.package, "package-source-drift");
    for (const [f, h] of Object.entries(checksum.files)) {
      need(
        !path.isAbsolute(f) && !f.split(/[\\/]/).includes(".."),
        "escaped-crate-input",
      );
      const full = path.join(root, f);
      need(
        fs.lstatSync(full).isFile() &&
          !fs.lstatSync(full).isSymbolicLink() &&
          fileHash(full) === h,
        "crate-file-drift",
      );
    }
    const build = packageBuildScript(
      fs.readFileSync(path.join(root, "Cargo.toml"), "utf8"),
      fs.existsSync(path.join(root, "build.rs")),
    );
    if (build) {
      const a = policy.scripts[key];
      need(
        a && a.file === build && fileHash(path.join(root, build)) === a.sha256,
        "unaudited-build-script",
      );
    }
    const lib = fs
      .readFileSync(path.join(root, "Cargo.toml"), "utf8")
      .split(/^\[lib\]\r?$/m)[1]
      ?.split(/^\[/m)[0];
    if (lib && /^proc-macro = true$/m.test(lib)) {
      const f = /^path = "([^"]+)"$/m.exec(lib)?.[1] ?? "src/lib.rs",
        a = policy.proc_macros[key];
      need(
        a && a.file === f && fileHash(path.join(root, f)) === a.sha256,
        "unaudited-proc-macro",
      );
    }
    manifests.push({
      package: key,
      archive_sha256: checksum.package,
      files: checksum.files,
    });
    dirs[key] = root;
  }
  for (const [key, sub] of [
    ["libgit2-sys@0.18.8+1.9.7", "libgit2/src"],
    ["libssh2-sys@0.3.1", "libssh2/src"],
    ["ring@0.17.14", "pregenerated"],
  ])
    need(
      dirs[key] && fs.statSync(path.join(dirs[key], sub)).isDirectory(),
      "missing-packaged-source",
    );
  need(
    !fs.existsSync(path.join(dirs["ring@0.17.14"], ".git")),
    "unpackaged-ring",
  );
  const sqlite = dirs["libsqlite3-sys@0.30.1"];
  need(
    sqlite &&
      fileHash(path.join(sqlite, "sqlite3/sqlite3.c")) ===
        "c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d",
    "sqlite-source-drift",
  );
  return { manifests, dirs, sqlite };
}
function runProcess(
  exe,
  args,
  { cwd, env, timeout = 30000, cap = 64 * 1024 * 1024 },
) {
  return new Promise((resolve) => {
    const c = spawn(exe, args, {
        cwd,
        env,
        stdio: ["ignore", "pipe", "pipe"],
        windowsHide: true,
      }),
      stdout = [],
      stderr = [];
    let bytes = 0,
      failed = false;
    const stop = () => {
      if (failed) return;
      failed = true;
      if (c.pid)
        spawnSync(
          path.join(env.SystemRoot, "System32/taskkill.exe"),
          ["/PID", String(c.pid), "/T", "/F"],
          { stdio: "ignore", timeout: 10000, windowsHide: true },
        );
      c.kill();
    };
    const timer = setTimeout(stop, timeout);
    for (const [stream, parts] of [
      [c.stdout, stdout],
      [c.stderr, stderr],
    ])
      stream.on("data", (b) => {
        bytes += b.length;
        if (bytes > cap) stop();
        else if (!failed) parts.push(b);
      });
    c.on("error", () => {
      failed = true;
    });
    c.on("close", (code) => {
      clearTimeout(timer);
      resolve({
        code: failed ? -1 : code,
        stdout: Buffer.concat(stdout).toString("utf8"),
        stderr: Buffer.concat(stderr).toString("utf8"),
      });
    });
  });
}
async function main() {
  assertDispatch(process.env, process.platform);
  const root = fs.realpathSync(process.cwd());
  need(
    root === fs.realpathSync(process.env.GITHUB_WORKSPACE),
    "wrong-checkout",
  );
  const scratch = path.join(
    process.env.RUNNER_TEMP,
    "ork-pair-fixed-windows-build",
  );
  fs.mkdirSync(scratch);
  const evidence = path.join(scratch, "evidence");
  fs.mkdirSync(evidence);
  const result = {
    schema: 1,
    proposal_merge: "cd12001b37a661232195f68def753b51a0d21c61",
    implementation_sha: process.env.EXPECTED_SHA,
    run_id: process.env.GITHUB_RUN_ID,
    started_utc: new Date().toISOString(),
    success: false,
    stage: "preflight",
    fetch_invocations: 0,
    build_invocations: 0,
    oracle_invocations: 0,
    target_executions: 0,
  };
  const files = {},
    save = (name, text) => {
      const f = path.join(scratch, name);
      fs.writeFileSync(f, text);
      files[name] = f;
      return f;
    };
  let roots,
    seal,
    src,
    env,
    p,
    cargoLog = "",
    cargoOut = "",
    logs = "";
  const redact = (t) =>
    [root, scratch, process.env.USERPROFILE]
      .filter(Boolean)
      .reduce((s, r) => s.split(r).join("[runner]"), t);
  const exec = async (exe, args, options = {}) => {
    result.commands ??= [];
    result.commands.push({ exe, args, cwd: options.cwd ?? root });
    const x = await runProcess(exe, args, { cwd: root, env, ...options });
    logs += x.stderr + "\n";
    return x;
  };
  try {
    checkPins(root);
    assertConfiguration(root);
    const lock = fs.readFileSync(
      path.join(root, "crates/orkworksd/Cargo.lock"),
      "utf8",
    );
    assertLock(lock);
    const response = await fetch(
      "https://api.github.com/repos/Rambolarsen/orkworks/actions/workflows/pair-fixed-windows-build.yml/runs?event=workflow_dispatch&per_page=100",
      {
        headers: {
          Authorization: `Bearer ${process.env.GITHUB_TOKEN}`,
          Accept: "application/vnd.github+json",
        },
        signal: AbortSignal.timeout(30000),
      },
    );
    need(response.ok, "run-history-unavailable");
    assertFirstRun(
      await response.json(),
      process.env.GITHUB_RUN_ID,
      process.env.EXPECTED_SHA,
    );
    result.dispatch_history_count = 1;
    result.image = {
      os: process.env.ImageOS,
      version: process.env.ImageVersion,
    };
    result.stage = "toolchain";
    const rust = path.join(
      process.env.USERPROFILE,
      ".rustup/toolchains/stable-x86_64-pc-windows-msvc",
    );
    p = {
      cargo: path.join(rust, "bin/cargo.exe"),
      rustc: path.join(rust, "bin/rustc.exe"),
      vc: "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.44.35207",
      kit: "C:\\Program Files (x86)\\Windows Kits\\10",
      sdk: "10.0.26100.0",
    };
    const cl = path.join(p.vc, "bin/Hostx64/x64/cl.exe");
    for (const [name, f] of [
      ["cargo.exe", p.cargo],
      ["rustc.exe", p.rustc],
      ["cl.exe", cl],
    ])
      need(fileHash(f) === policy.tools[name], "tool-byte-mismatch");
    fs.mkdirSync(path.join(scratch, "cargo-home"));
    env = buildEnvironment(process.env, p, scratch);
    const ps = process.env.ORK_PAIR_PWSH;
    need(ps && fs.statSync(ps).isFile(), "powershell-identity");
    const info = await exec(
      ps,
      [
        "-NoProfile",
        "-NonInteractive",
        "-File",
        path.join(root, "scripts/qualification/pair-fixed-windows-build.ps1"),
        "-Inventory",
      ],
      {
        env: {
          ...env,
          GITHUB_ACTIONS: "true",
          GITHUB_EVENT_NAME: "workflow_dispatch",
          GITHUB_RUN_ATTEMPT: "1",
          EXPECTED_SHA: process.env.EXPECTED_SHA,
        },
      },
    );
    need(info.code === 0, "sdk-selection-unavailable");
    result.selection = JSON.parse(info.stdout);
    need(
      result.selection.compiler_version === "19.44.35229.0" &&
        result.selection.sdk_service === "10.1.26100.9457",
      "named-toolchain-mismatch",
    );
    roots = {
      source: root,
      rust,
      msvc: p.vc,
      sdk_include: path.join(p.kit, "Include", p.sdk),
      sdk_lib: path.join(p.kit, "Lib", p.sdk),
      sdk_bin: path.join(p.kit, "bin", p.sdk),
    };
    seal = treeManifest(roots);
    const input = {
      source_pins: policy.source_pins,
      environment: env,
      selection: result.selection,
      tools: {
        cargo: fileHash(p.cargo),
        rustc: fileHash(p.rustc),
        cl: fileHash(cl),
        node: fileHash(process.execPath),
        powershell: fileHash(ps),
      },
      files: seal,
    };
    save("inputs.json", JSON.stringify(input));
    for (const [exe, args] of [
      [p.cargo, ["-Vv"]],
      [p.rustc, ["-vV"]],
    ]) {
      const x = await exec(exe, args);
      need(
        x.code === 0 &&
          /^release: 1\.100\.0$/m.test(x.stdout) &&
          /^host: x86_64-pc-windows-msvc$/m.test(x.stdout),
        "rust-version-mismatch",
      );
    }
    let invocation, archive, object, oracleArgs;
    const outcome = await candidateAttempt({
      fetch: async () => {
        const x = await exec(
          p.cargo,
          [
            "fetch",
            "--locked",
            "--target",
            "x86_64-pc-windows-msvc",
            "--manifest-path",
            "crates/orkworksd/Cargo.toml",
            "--config",
            "net.retry=0",
          ],
          { timeout: 4 * 60 * 1000 },
        );
        return x.code;
      },
      sources: async () => {
        src = verifiedSources(env.CARGO_HOME, lock);
        roots.registry = path.join(env.CARGO_HOME, "registry/src");
        seal = [...seal, ...treeManifest({ registry: roots.registry })];
        input.files = seal;
        input.crates = src.manifests;
        save("inputs.json", JSON.stringify(input));
        result.input_manifest_sha256 = fileHash(files["inputs.json"]);
      },
      build: async () => {
        const x = await exec(
          p.cargo,
          [
            "build",
            "--locked",
            "--offline",
            "--release",
            "--target",
            "x86_64-pc-windows-msvc",
            "--bin",
            "orkworksd",
            "--jobs",
            "2",
            "--manifest-path",
            "crates/orkworksd/Cargo.toml",
            "-vv",
            "--message-format=json",
          ],
          { timeout: 35 * 60 * 1000 },
        );
        cargoLog = x.stderr;
        cargoOut = x.stdout;
        save("cargo.jsonl", redact(cargoOut));
        return x.code;
      },
      prepareOracle: async () => {
        invocation = sqliteInvocation(cargoLog + "\n" + cargoOut, cl);
        assertSqliteSource(
          invocation,
          src.sqlite,
          path.join(src.sqlite, "sqlite3/sqlite3.c"),
        );
        const oracle = save(
            "oracle.c",
            oracleSource(path.join(src.sqlite, "sqlite3/sqlite3.c")),
          ),
          args = preprocessArguments(invocation, oracle);
        save(
          "compiler.json",
          JSON.stringify(
            {
              program: cl,
              argv: invocation.slice(1),
              cwd: src.sqlite,
              environment: env,
              oracle_argv: args,
            },
            null,
            2,
          ),
        );
        const fo = invocation.find((a) => /^[-/]Fo/.test(a));
        need(fo, "sqlite-object-unavailable");
        object = fo.slice(3);
        need(
          win(object).startsWith(win(env.CARGO_TARGET_DIR) + path.win32.sep),
          "foreign-object",
        );
        archive = path.join(path.dirname(object), "sqlite3.lib");
        files["sqlite3.lib"] = archive;
        result.sqlite = {
          source_sha256: fileHash(path.join(src.sqlite, "sqlite3/sqlite3.c")),
          object_sha256: fileHash(object),
          archive_sha256: fileHash(archive),
        };
        oracleArgs = args;
      },
      oracle: async () => {
        const x = await exec(cl, oracleArgs, {
          cwd: src.sqlite,
          timeout: 2 * 60 * 1000,
          cap: 32 * 1024 * 1024,
        });
        save("oracle.i", redact(x.stdout));
        return /warning D9002/.test(x.stderr) ? -1 : x.code;
      },
      inspect: async () => {
        const candidate = path.join(
          env.CARGO_TARGET_DIR,
          "x86_64-pc-windows-msvc/release/orkworksd.exe",
        );
        need(fs.statSync(candidate).isFile(), "candidate-unavailable");
        files["orkworksd.exe"] = candidate;
        files["candidate.map"] = path.join(scratch, "candidate.map");
        const libs = await exec(path.join(p.vc, "bin/Hostx64/x64/lib.exe"), [
          "/LIST",
          archive,
        ]);
        save("archive-members.txt", redact(libs.stdout + libs.stderr));
        need(
          libs.code === 0 &&
            (libs.stdout + libs.stderr)
              .toLowerCase()
              .includes(path.basename(object).toLowerCase()),
          "archive-association-unavailable",
        );
        const deps = await exec(
          path.join(p.vc, "bin/Hostx64/x64/dumpbin.exe"),
          ["/DEPENDENTS", candidate],
        );
        save("dependencies.txt", redact(deps.stdout + deps.stderr));
        need(
          deps.code === 0 &&
            !/\b(?:sqlite3?|sqlcipher)\.dll\b/i.test(deps.stdout),
          "external-sqlite-import",
        );
        const rlib = sqliteRlib(cargoOut, env.CARGO_TARGET_DIR),
          member = path.basename(object),
          lib = path.join(p.vc, "bin/Hostx64/x64/lib.exe");
        files["libsqlite3_sys.rlib"] = rlib;
        const custody = {
          object_sha256: fileHash(object),
          native_archive_sha256: fileHash(archive),
          rlib_sha256: fileHash(rlib),
          rlib,
          member,
          extractions: [],
        };
        for (const [label, input] of [
          ["native", archive],
          ["rust", rlib],
        ]) {
          const listing =
            label === "native" ? libs : await exec(lib, ["/LIST", input]);
          need(listing.code === 0, "archive-list-unavailable");
          const selectedMember = archiveMember(listing.stdout, member);
          if (label === "rust")
            save(
              "archive-members.txt",
              redact(
                libs.stdout +
                  libs.stderr +
                  "\n" +
                  listing.stdout +
                  listing.stderr,
              ),
            );
          const extracted = path.join(scratch, label + "-sqlite.obj");
          const x = await exec(lib, [
            input,
            "/EXTRACT:" + selectedMember,
            "/OUT:" + extracted,
          ]);
          need(
            x.code === 0 && fileHash(extracted) === custody.object_sha256,
            "sqlite-member-byte-mismatch",
          );
          custody.extractions.push({
            archive: input,
            member: selectedMember,
            sha256: fileHash(extracted),
          });
          fs.rmSync(extracted);
        }
        const map = fs.readFileSync(files["candidate.map"], "utf8");
        need(
          /^\s*orkworksd(?:\.exe)?\s*$/m.test(map.split(/Timestamp/i)[0]),
          "candidate-map-module-mismatch",
        );
        custody.map_rows = assertSqliteMap(map, rlib, member);
        save("link-custody.json", JSON.stringify(custody, null, 2));
        result.candidate = {
          sha256: fileHash(candidate),
          bytes: fs.statSync(candidate).size,
        };
        result.linkage = {
          archive_sha256: fileHash(archive),
          map_sha256: fileHash(files["candidate.map"]),
          dependency_report_sha256: fileHash(files["dependencies.txt"]),
        };
      },
    });
    Object.assign(result, outcome);
  } catch (e) {
    result.success = false;
    result.reason =
      String(e.message)
        .replace(/[^a-z-]/g, "")
        .slice(0, 80) || "unavailable";
  } finally {
    try {
      checkPins(root);
      if (seal) assertManifest(seal, roots);
    } catch {
      result.success = false;
      result.reason = "input-drift";
    }
    // Files not yet produced are not evidence; preserve all available bounded failure data.
    for (const n of Object.keys(files))
      if (!fs.existsSync(files[n])) delete files[n];
    save("diagnostics.txt", redact(logs));
    result.ended_utc = new Date().toISOString();
    result.runtime_qualified = false;
    publishEvidence(evidence, result, files);
    try {
      for (const name of ["cargo-home", "target"])
        fs.rmSync(path.join(scratch, name), { recursive: true, force: true });
      result.cleanup = "owned-scratch-removed";
    } catch {
      result.success = false;
      result.cleanup = "unavailable";
    }
    const receipt = JSON.parse(
      fs.readFileSync(path.join(evidence, "result.json"), "utf8"),
    );
    receipt.cleanup = result.cleanup;
    if (result.cleanup === "unavailable") receipt.success = false;
    fs.writeFileSync(
      path.join(evidence, "result.json"),
      JSON.stringify(receipt),
    );
  }
  if (!result.success) process.exitCode = 1;
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href
)
  main().catch(() => {
    console.error("Fixed build unavailable; no retry authorized");
    process.exitCode = 1;
  });
