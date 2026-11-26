import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { spawn } from "node:child_process";
import { pathToFileURL } from "node:url";
import { assertFirstRun } from "./pair-windows-metadata-evidence.mjs";
const need = (v, m) => {
  if (!v) throw Error(m);
};
const sha = (v) => crypto.createHash("sha256").update(v).digest("hex");
const workflow = "pair-windows-toolchain-inventory.yml";
const cap = 4 * 1024 * 1024,
  maxHash = 1024 ** 3;
export const pins = {
  "cargo.exe":
    "f381bae27c59588eacf69208e115b2c001a2998afed83a2e9084ea93620737b1",
  "rustc.exe":
    "d4f8a0283de2dba0dc458cd13c333337d2b76c3649eb39e91c4acce618e661a1",
  "cl.exe": "e87e78b3ef6580b75aebda843f8a22e874612d9f766344046551399f898eb226",
};
export function targets(profile) {
  need(
    /^[A-Za-z]:\\/.test(profile ?? "") &&
      !profile.split(/[\\/]/).includes(".."),
    "profile-identity",
  );
  const vc = "C:/Program Files/Microsoft Visual Studio/2022/Enterprise";
  const groups = [
    [
      path.win32.join(
        profile,
        ".rustup/toolchains/stable-x86_64-pc-windows-msvc/bin",
      ),
      ["cargo.exe", "rustc.exe"],
    ],
    [
      path.win32.join(vc, "VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64"),
      [
        "cl.exe",
        "link.exe",
        "lib.exe",
        "dumpbin.exe",
        "c1.dll",
        "c1xx.dll",
        "c2.dll",
        "mspdbcore.dll",
      ],
    ],
    [
      "C:/Program Files (x86)/Windows Kits/10/bin/10.0.26100.0/x64",
      ["rc.exe", "mt.exe"],
    ],
    [path.win32.join(vc, "Common7/IDE"), ["devenv.exe"]],
  ];
  return groups.flatMap(([root, names]) =>
    names.map((name) => ({ name, path: path.win32.join(root, name) })),
  );
}
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
        "Rambolarsen/orkworks/.github/workflows/" +
          workflow +
          "@refs/heads/main",
    "dispatch-denied",
  );
}
export function childEnvironment(parent, budget) {
  const e = {};
  for (const k of ["SystemRoot", "WINDIR", "TEMP", "TMP", "USERPROFILE"])
    if (parent[k]) e[k] = parent[k];
  return {
    ...e,
    PATH: path.win32.join(e.SystemRoot ?? "", "System32"),
    ORK_PAIR_HASH_BUDGET: String(budget),
    GITHUB_ACTIONS: "true",
    GITHUB_EVENT_NAME: "workflow_dispatch",
    GITHUB_RUN_ATTEMPT: "1",
    EXPECTED_SHA: parent.EXPECTED_SHA,
    ORK_PAIR_INVENTORY_CHILD: "1",
  };
}
export function problems(data, profile) {
  const errors = [],
    add = (v, m) => {
      if (!v) errors.push(m);
    };
  const selected = targets(profile),
    files = data?.files;
  add(Array.isArray(files) && files.length === 13, "file-inventory-incomplete");
  if (Array.isArray(files))
    for (const target of selected) {
      const matches = files.filter((f) => f?.name === target.name);
      add(matches.length === 1, "ambiguous-file");
      const f = matches[0];
      if (!f) continue;
      add(
        typeof f.path === "string" &&
          path.win32.normalize(f.path).toLowerCase() ===
            target.path.toLowerCase(),
        "foreign-file",
      );
      add(
        f.status === "ok" &&
          Number.isSafeInteger(f.bytes) &&
          f.bytes > 0 &&
          f.bytes <= 256 * 1024 * 1024 &&
          /^[a-f0-9]{64}$/.test(f.sha256 ?? "") &&
          f.machine === "8664",
        "file-unavailable",
      );
      add(f.final_sha256 === f.sha256, "file-drift");
      if (pins[f.name]) add(f.sha256 === pins[f.name], "tool-byte-mismatch");
      for (const k of ["file_version", "product_version"])
        add(
          f[k] === null ||
            (typeof f[k] === "string" && /^[\x20-\x7e]{1,128}$/.test(f[k])),
          "invalid-version-resource",
        );
      if (f.name === "cl.exe")
        add(
          /^[0-9]+(?:\.[0-9]+){1,3}$/.test(f.file_version ?? ""),
          "compiler-version-unavailable",
        );
    }
  add(
    Number.isSafeInteger(data?.hashed_bytes) &&
      data.hashed_bytes >= 0 &&
      data.hashed_bytes <= maxHash,
    "hash-budget",
  );
  if (Array.isArray(files))
    add(
      data?.hashed_bytes >=
        files.reduce(
          (n, f) =>
            n +
            (Number.isSafeInteger(f?.bytes) && f.bytes > 0 ? 2 * f.bytes : 0),
          0,
        ),
      "hash-work-unaccounted",
    );
  add(
    Number.isSafeInteger(data?.hash_passes) && data.hash_passes >= 26,
    "hash-recheck-incomplete",
  );
  const sdk = data?.sdk;
  add(
    sdk?.status === "ok" &&
      sdk.root?.replaceAll("/", "\\").replace(/\\$/, "").toLowerCase() ===
        "c:\\program files (x86)\\windows kits\\10",
    "sdk-root",
  );
  add(
    Array.isArray(sdk?.directories) &&
      sdk.directories.length === 3 &&
      sdk.directories.every((v) => v === true),
    "sdk-directories",
  );
  add(
    Number.isSafeInteger(sdk?.scanned_entries) &&
      sdk.scanned_entries >= 0 &&
      sdk.scanned_entries <= 256,
    "registry-budget",
  );
  add(
    Array.isArray(sdk?.products) &&
      sdk.products.length <= 256 &&
      sdk.products.some(
        (p) =>
          p.version === "10.1.26100.9457" &&
          typeof p.name === "string" &&
          p.name.includes("Software Development Kit"),
      ),
    "sdk-service",
  );
  add(
    ["ok", "missing"].includes(data?.default_toolset?.status),
    "default-text-unavailable",
  );
  if (data?.default_toolset?.status === "ok")
    add(
      data.default_toolset.sha256 === data.default_toolset.final_sha256,
      "default-text-drift",
    );
  return [...new Set(errors)];
}
export async function collectOnce(run, profile) {
  const r = {
    success: false,
    diagnostics: "",
    controller_invocations: 1,
    inspected_tool_executions: 0,
    fetch_invocations: 0,
    build_invocations: 0,
    target_executions: 0,
  };
  try {
    const x = await run();
    r.controller_exit = x.code;
    r.diagnostics = x.stderr;
    r.inventory = JSON.parse(x.stdout);
    r.problems = problems(r.inventory, profile);
    r.success = x.code === 0 && r.problems.length === 0;
    r.reason = r.success ? undefined : (r.problems[0] ?? "controller-failed");
  } catch {
    r.reason = "inventory-unavailable";
  }
  return r;
}
export function publish(
  directory,
  result,
  inventory,
  diagnostics,
  limit = cap,
) {
  let files = {
    "result.json": JSON.stringify(result, null, 2),
    "diagnostics.txt": diagnostics,
  };
  if (inventory !== undefined) files["inventory.json"] = inventory;
  if (
    Object.values(files).reduce((n, s) => n + Buffer.byteLength(s), 0) > limit
  ) {
    result.success = false;
    result.reason = "evidence-limit";
    files = {
      "result.json": JSON.stringify({
        schema: 1,
        success: false,
        reason: "evidence-limit",
        run_id: result.run_id,
        implementation_sha: result.implementation_sha,
        controller_invocations: result.controller_invocations,
        inspected_tool_executions: 0,
        fetch_invocations: 0,
        build_invocations: 0,
        target_executions: 0,
        cleanup: result.cleanup,
      }),
    };
  }
  for (const [name, value] of Object.entries(files))
    fs.writeFileSync(path.join(directory, name), value);
}
function controllerRun(executable, script, env, root) {
  return new Promise((resolve) => {
    const child = spawn(
      executable,
      [
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-File",
        script,
        "-Inventory",
      ],
      { cwd: root, env, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] },
    );
    const out = [],
      err = [];
    let stdoutBytes = 0,
      stderrBytes = 0,
      failed = false;
    const stop = () => {
      failed = true;
      child.kill("SIGKILL");
    };
    const timer = setTimeout(stop, 6 * 60 * 1000);
    child.stdout.on("data", (b) => {
      stdoutBytes += b.length;
      if (stdoutBytes > 3 * 1024 * 1024) stop();
      else if (!failed) out.push(b);
    });
    child.stderr.on("data", (b) => {
      stderrBytes += b.length;
      if (stderrBytes > 65536) stop();
      else if (!failed) err.push(b);
    });
    child.on("error", () => {
      failed = true;
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({
        code: failed ? -1 : code,
        stdout: failed ? "" : Buffer.concat(out).toString("utf8"),
        stderr: Buffer.concat(err).toString("utf8"),
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
    "ork-pair-toolchain-inventory",
  );
  fs.mkdirSync(scratch);
  const work = path.join(scratch, "working"),
    evidence = path.join(scratch, "evidence");
  fs.mkdirSync(work);
  fs.mkdirSync(evidence);
  const result = {
    schema: 1,
    proposal_merge: "d67135d00cb764fb23ac9126ecaff5152e4837f0",
    implementation_sha: process.env.EXPECTED_SHA,
    run_id: process.env.GITHUB_RUN_ID,
    started_utc: new Date().toISOString(),
    stage: "preflight",
    success: false,
    controller_invocations: 0,
    inspected_tool_executions: 0,
    fetch_invocations: 0,
    build_invocations: 0,
    target_executions: 0,
    runtime_qualified: false,
    replacement_pin_accepted: false,
  };
  let inventory,
    diagnostics = "";
  try {
    const response = await fetch(
      "https://api.github.com/repos/Rambolarsen/orkworks/actions/workflows/" +
        workflow +
        "/runs?event=workflow_dispatch&per_page=100",
      {
        headers: {
          Authorization: "Bearer " + process.env.GITHUB_TOKEN,
          Accept: "application/vnd.github+json",
        },
        signal: AbortSignal.timeout(30000),
      },
    );
    need(response.ok, "history-unavailable");
    assertFirstRun(
      await response.json(),
      process.env.GITHUB_RUN_ID,
      process.env.EXPECTED_SHA,
    );
    result.dispatch_history_count = 1;
    need(
      process.env.RUNNER_OS === "Windows" &&
        process.env.RUNNER_ARCH === "X64" &&
        /^[a-zA-Z0-9_-]+$/.test(process.env.ImageOS ?? "") &&
        /^[0-9]+(?:\.[0-9]+){1,3}$/.test(process.env.ImageVersion ?? ""),
      "runner-identity",
    );
    result.runner = {
      os: process.env.RUNNER_OS,
      arch: process.env.RUNNER_ARCH,
      image_os: process.env.ImageOS,
      image_version: process.env.ImageVersion,
    };
    targets(process.env.USERPROFILE);
    const script = path.join(
        root,
        "scripts/qualification/pair-windows-toolchain-inventory.ps1",
      ),
      self = path.join(
        root,
        "scripts/qualification/pair-windows-toolchain-inventory.mjs",
      );
    const sources = [self, script].map((f) => ({
      file: path.relative(root, f),
      bytes: fs.statSync(f).size,
      sha256: sha(fs.readFileSync(f)),
    }));
    result.controllers = sources;
    const sourceBytes = sources.reduce((n, s) => n + s.bytes, 0);
    const ps = process.env.ORK_PAIR_PWSH;
    need(
      ps &&
        path.win32.isAbsolute(ps) &&
        path.win32.basename(ps).toLowerCase() === "pwsh.exe",
      "controller-identity",
    );
    result.controller_program = ps;
    result.stage = "inventory";
    result.controller_invocations = 1;
    const out = await collectOnce(
      () =>
        controllerRun(
          ps,
          script,
          childEnvironment(process.env, maxHash - cap - 2 * sourceBytes),
          work,
        ),
      process.env.USERPROFILE,
    );
    diagnostics = out.diagnostics;
    delete out.diagnostics;
    if (out.inventory) inventory = JSON.stringify(out.inventory);
    delete out.inventory;
    Object.assign(result, out);
    if (inventory !== undefined) {
      need(
        JSON.parse(inventory).hashed_bytes +
          2 * sourceBytes +
          Buffer.byteLength(inventory) <=
          maxHash,
        "hash-budget",
      );
      result.inventory_sha256 = sha(inventory);
      result.inventory_bytes = Buffer.byteLength(inventory);
    }
    result.stage = result.success ? "complete" : "inventory";
    for (const source of sources)
      need(
        sha(fs.readFileSync(path.join(root, source.file))) === source.sha256,
        "controller-drift",
      );
  } catch (e) {
    result.success = false;
    result.reason =
      String(e.message)
        .replace(/[^a-z-]/g, "")
        .slice(0, 80) || "unavailable";
  } finally {
    try {
      fs.rmSync(work, { recursive: true, force: true });
      result.cleanup = "owned-working-scratch-removed";
    } catch {
      result.success = false;
      result.cleanup = "unavailable";
    }
    result.ended_utc = new Date().toISOString();
    for (const value of [root, scratch, process.env.USERPROFILE])
      if (value) diagnostics = diagnostics.split(value).join("[runner]");
    publish(evidence, result, inventory, diagnostics);
  }
  if (!result.success) process.exitCode = 1;
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href
)
  main().catch(() => {
    console.error("Inventory unavailable; no retry authorized");
    process.exitCode = 1;
  });
