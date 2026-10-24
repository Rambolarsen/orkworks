import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import * as opener from "../electron/sessionAppOpener.ts";
import { inspectSessionApp, openSessionApp, type SessionAppDependencies } from "../electron/sessionAppOpener.ts";

const builtins = JSON.parse(readFileSync(new URL("../../../crates/orkworksd/resources/harnesses-v2.json", import.meta.url), "utf8")).builtins;
// Mirror the HTTP HarnessConfigEntry envelope, not the renderer's flattened mapping.
const harnessResponse = { documentRevision: null, harnesses: builtins.map((definition: Record<string, unknown>) => ({
  definition: { retired: false, minVersion: null, ...definition }, origin: "builtin",
  compatibility: { profile: null, sessionSignals: definition.sessionSignals, integration: definition.integration },
})) };
const nativeId = "12345678-1234-4234-8234-123456789abc";
function fixture(harnessId = "codex") {
  const opened: string[] = [];
  const commands: Array<{ command: string; args: string[]; cwd: string }> = [];
  let sessions: unknown = [{ id: "selected", harnessId, cwd: "/project", lifecycle: "dead", resume: { harnessSessionId: nativeId } }];
  const controller = new AbortController();
  const deps: SessionAppDependencies = {
    platform: "darwin", arch: "arm64", signal: controller.signal,
    readSessions: async () => sessions,
    readHarnesses: async () => harnessResponse,
    applicationForProtocol: async () => "Installed app",
    checkDirectory: async () => {},
    run: async (command, args, cwd) => { commands.push({ command, args, cwd }); return args[0] === "--help" ? "--desktop --resume <id>" : "Opening session"; },
    openExternal: async url => { opened.push(url); },
  };
  return { deps, opened, commands, controller, setSessions(value: unknown) { sessions = value; } };
}

test("opens only the selected native Codex conversation and inspection has no opening side effect", async () => {
  const f = fixture();
  assert.deepEqual(await inspectSessionApp("selected", f.deps), { label: "ChatGPT", available: true });
  assert.deepEqual(f.opened, []);
  await openSessionApp("selected", f.deps);
  assert.deepEqual(f.opened, ["codex://threads/12345678-1234-4234-8234-123456789abc"]);
  assert.deepEqual(f.commands, []);
});

test("rejects missing or URL/argument-shaped native identity without opening", async () => {
  for (const id of [undefined, "new", "--continue", nativeId + "?prompt=bad", "../settings"]) {
    const f = fixture();
    f.setSessions([{ id: "selected", harnessId: "codex", cwd: "/project", lifecycle: "dead", resume: { harnessSessionId: id } }]);
    assert.equal((await inspectSessionApp("selected", f.deps)).available, false);
    await assert.rejects(openSessionApp("selected", f.deps));
    assert.deepEqual(f.opened, []);
  }
});

test("Claude handoff uses the configured command, exact ID and session directory", async () => {
  const f = fixture("claude-code");
  await openSessionApp("selected", f.deps);
  assert.deepEqual(f.commands, [
    { command: "claude", args: ["--help"], cwd: "/project" },
    { command: "claude", args: ["--desktop", "--resume", nativeId], cwd: "/project" },
  ]);
  assert.deepEqual(f.opened, []);
});

test("rechecks current Claude lifecycle and never closes or transfers a live session", async () => {
  const f = fixture("claude-code");
  assert.equal((await inspectSessionApp("selected", f.deps)).available, true);
  f.setSessions([{ id: "selected", harnessId: "claude-code", cwd: "/project", lifecycle: "alive", resume: { harnessSessionId: nativeId } }]);
  await assert.rejects(openSessionApp("selected", f.deps), /Close.*first/);
  assert.equal(f.commands.length, 1);
});

test("unsupported tools, platform, old Claude, absent app and disappeared session are unavailable", async () => {
  const unsupported = fixture("generic-shell");
  assert.equal((await inspectSessionApp("selected", unsupported.deps)).available, false);
  const linux = fixture("claude-code"); linux.deps.platform = "linux";
  assert.equal((await inspectSessionApp("selected", linux.deps)).available, false);
  const old = fixture("claude-code"); old.deps.run = async () => "--resume <id>";
  assert.equal((await inspectSessionApp("selected", old.deps)).available, false);
  const absent = fixture(); absent.deps.applicationForProtocol = async () => "";
  assert.equal((await inspectSessionApp("selected", absent.deps)).available, false);
  const removed = fixture(); removed.setSessions([]);
  await assert.rejects(openSessionApp("selected", removed.deps), /no longer available/);
});

test("aborting during capability lookup prevents native dispatch", async () => {
  const f = fixture();
  f.deps.applicationForProtocol = async () => { f.controller.abort(); return "ChatGPT"; };
  await assert.rejects(openSessionApp("selected", f.deps));
  assert.deepEqual(f.opened, []);
});

test("propagates launch failure instead of reporting success", async () => {
  const f = fixture(); f.deps.openExternal = async () => { throw new Error("App refused"); };
  await assert.rejects(openSessionApp("selected", f.deps), /App refused/);
});

test("custom definitions cannot impersonate a built-in desktop destination", async () => {
  const f = fixture();
  f.deps.readHarnesses = async () => ({ harnesses: [{ definition: { id: "codex" }, origin: "custom", compatibility: { profile: null, sessionSignals: null, integration: null } }] });
  assert.equal((await inspectSessionApp("selected", f.deps)).available, false);
  await assert.rejects(openSessionApp("selected", f.deps));
  assert.deepEqual(f.opened, []);
});

test("Claude rejects an unavailable directory before any executable runs", async () => {
  const f = fixture("claude-code");
  f.deps.checkDirectory = async () => { throw new Error("Missing directory"); };
  await assert.rejects(openSessionApp("selected", f.deps), /Missing directory/);
  assert.deepEqual(f.commands, []);
});

test("Claude resumed during capability probing cannot transfer after the probe completes", async () => {
  const f = fixture("claude-code");
  const run = f.deps.run;
  f.deps.run = async (command, args, cwd) => {
    const result = await run(command, args, cwd);
    if (args[0] === "--help") f.setSessions([{ id: "selected", harnessId: "claude-code", cwd: "/project", lifecycle: "alive", resume: { harnessSessionId: nativeId } }]);
    return result;
  };
  await assert.rejects(openSessionApp("selected", f.deps), /changed|Close/);
  assert.equal(f.commands.length, 1, "capability probing never becomes a handoff after Resume");
});

test("native identity changed during capability lookup cannot open an obsolete conversation", async () => {
  const f = fixture();
  f.deps.applicationForProtocol = async () => {
    f.setSessions([{ id: "selected", harnessId: "codex", cwd: "/project", lifecycle: "alive", resume: { harnessSessionId: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee" } }]);
    return "ChatGPT";
  };
  await assert.rejects(openSessionApp("selected", f.deps), /changed/);
  assert.deepEqual(f.opened, []);
});

test("Windows built-in Claude uses a bounded cmd invocation for native executables and npm shims", () => {
  assert.deepEqual(opener.sessionAppInvocation("win32", "claude", ["--help"], "C:\\Windows\\System32\\cmd.exe"), {
    env: { NoDefaultCurrentDirectoryInExePath: "1" }, file: "C:\\Windows\\System32\\cmd.exe", args: ["/d", "/v:off", "/s", "/c", "claude --help"],
  });
  assert.deepEqual(opener.sessionAppInvocation("win32", "claude", ["--desktop", "--resume", nativeId], "C:\\Windows\\System32\\cmd.exe"), {
    env: { NoDefaultCurrentDirectoryInExePath: "1" }, file: "C:\\Windows\\System32\\cmd.exe", args: ["/d", "/v:off", "/s", "/c", "claude --desktop --resume 12345678-1234-4234-8234-123456789abc"],
  });
  assert.deepEqual(opener.sessionAppInvocation("darwin", "claude", ["--help"]), { file: "claude", args: ["--help"] });
});

test("Windows invocation never interpolates untrusted commands, flags or native IDs", () => {
  for (const [command, args] of [
    ["claude & whoami", ["--help"]], ["claude", ["--help", "&", "whoami"]],
    ["claude", ["--desktop", "--resume", nativeId + " & whoami"]],
    ["claude", ["--desktop", "--resume", "%SECRET%"]],
  ] as Array<[string, string[]]>) assert.throws(() => opener.sessionAppInvocation("win32", command, args));
});

test("Windows runs the installed shim without a repository-local command shadowing it", { skip: process.platform !== "win32" }, async () => {
  const root = await mkdtemp(join(tmpdir(), "orkworks-native-app-"));
  try {
    const tools = join(root, "tools");
    const cwd = join(root, "repository");
    await mkdir(tools); await mkdir(cwd);
    await writeFile(join(tools, "claude.cmd"), "@echo off\r\n@echo %*\r\n");
    await writeFile(join(cwd, "claude.cmd"), "@echo off\r\n@echo WRONG-REPOSITORY-COMMAND\r\n");
    for (const args of [["--help"], ["--desktop", "--resume", nativeId]]) {
      const invocation = opener.sessionAppInvocation("win32", "claude", args, process.env.ComSpec);
      const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => key.toUpperCase() !== "PATH"));
      const result = await promisify(execFile)(invocation.file, invocation.args, {
        cwd, env: { ...env, PATH: tools, ...invocation.env }, timeout: 5_000, windowsHide: true,
      });
      assert.equal(result.stdout.trim(), args.join(" "));
    }
  } finally { await rm(root, { recursive: true, force: true }); }
});
