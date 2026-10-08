import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { createShellLayoutMemory, shellLayoutMemoryPath } from "../electron/shellLayoutMemory.ts";

test("shell layout initializes defaults, saves bounded preferences, and resets presentation", async () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    writeFileSync(join(directory, "layout.json"), "legacy docking bytes");
    const memory = createShellLayoutMemory(directory);
    assert.deepEqual(memory.read().preferences, {
      sessionsWidth: 240, inspectorWidth: 320, sessionsVisible: true, density: "low",
    });
    const saved = await memory.save({ sessionsWidth: 300, inspectorWidth: 400, sessionsVisible: false, density: "high" });
    assert.equal(saved.ok, true);
    assert.equal(memory.read().preferences.sessionsWidth, 300);
    assert.equal((await memory.reset()).ok, true);
    assert.equal(memory.read().preferences.sessionsWidth, 240);
    assert.equal(readFileSync(join(directory, "layout.json"), "utf8"), "legacy docking bytes");
    assert.ok(readFileSync(shellLayoutMemoryPath(directory)).byteLength <= 16 * 1024);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("invalid and future shell layout records preserve bytes until confirmed rebuild", async () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const path = shellLayoutMemoryPath(directory);
    const source = JSON.stringify({ version: 2, epoch: "future", revision: 4, sessionsWidth: 240 });
    writeFileSync(path, source);
    const memory = createShellLayoutMemory(directory);
    assert.equal(memory.read().diagnostic, "unsupported_version");
    assert.equal((await memory.save({ sessionsWidth: 200, inspectorWidth: 280, sessionsVisible: true, density: "low" })).ok, false);
    assert.equal((await memory.reset()).ok, false);
    assert.equal(readFileSync(path, "utf8"), source);
    assert.equal((await memory.rebuild(true)).ok, true);
    assert.notEqual(readFileSync(path, "utf8"), source);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("shell layout rejects out-of-range, non-finite, and unknown preference fields", async () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const memory = createShellLayoutMemory(directory);
    memory.read();
    const source = readFileSync(shellLayoutMemoryPath(directory), "utf8");
    for (const preferences of [
      { sessionsWidth: 199, inspectorWidth: 320, sessionsVisible: true, density: "low" },
      { sessionsWidth: Infinity, inspectorWidth: 320, sessionsVisible: true, density: "low" },
      { sessionsWidth: 240, inspectorWidth: 421, sessionsVisible: true, density: "low" },
      { sessionsWidth: 240, inspectorWidth: 320, sessionsVisible: true, density: "low", workflowPresentation: "tree" },
    ]) {
      assert.equal((await memory.save(preferences)).ok, false);
      assert.equal(readFileSync(shellLayoutMemoryPath(directory), "utf8"), source);
    }
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("unprepared presentation updates coalesce to the latest enqueue-time intent", async () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const memory = createShellLayoutMemory(directory);
    memory.read();
    const earlier = memory.save({ sessionsWidth: 250, inspectorWidth: 320, sessionsVisible: true, density: "low" });
    const later = memory.save({ sessionsWidth: 260, inspectorWidth: 320, sessionsVisible: true, density: "low" });
    assert.equal((await earlier).ok, false);
    assert.equal((await later).ok, true);
    assert.equal(memory.read().revision, 1);
    assert.equal(memory.read().preferences.sessionsWidth, 260);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("reset barrier cancels an unprepared save", async () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const memory = createShellLayoutMemory(directory);
    memory.read();
    const first = memory.save({ sessionsWidth: 250, inspectorWidth: 320, sessionsVisible: true, density: "low" });
    const reset = memory.reset();
    assert.equal((await first).ok, false);
    assert.equal((await reset).ok, true);
    assert.equal(memory.read().revision, 1);
    assert.equal(memory.read().preferences.sessionsWidth, 240);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("a live retained shell lock blocks writes without evicting its inode", async () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  const lockPath = join(directory, ".shell-memory.lock");
  const child = spawn(process.execPath, ["--input-type=commonjs", "--eval", `
    const fs = require('node:fs');
    const { flockSync } = require('fs-ext');
    const fd = fs.openSync(process.argv[1], 'a+', 0o600);
    flockSync(fd, 'exnb');
    process.send('locked');
    setInterval(() => {}, 1000);
  `, lockPath], { stdio: ["ignore", "pipe", "pipe", "ipc"] });
  const exited = once(child, "exit");
  try {
    const [message] = await once(child, "message", { signal: AbortSignal.timeout(10_000) });
    assert.equal(message, "locked");
    const memory = createShellLayoutMemory(directory);
    assert.equal(memory.read().diagnostic, "lock_timeout");
    assert.equal((await memory.save({ sessionsWidth: 240, inspectorWidth: 320, sessionsVisible: true, density: "low" })).ok, false);
    assert.equal(readFileSync(lockPath).byteLength, 0);
  } finally {
    child.kill("SIGKILL");
    await exited;
    rmSync(directory, { recursive: true, force: true });
  }
});

test("failed first-use read-back restores prior absence for malformed output", () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const memory = createShellLayoutMemory(directory, (_temporary, target) => writeFileSync(target, "{mismatch"));
    assert.equal(memory.read().diagnostic, "write_failed");
    assert.equal(existsSync(shellLayoutMemoryPath(directory)), false);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("failed first-use read-back leaves an unexpected valid future record intact", () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const future = JSON.stringify({ version: 2, epoch: "f".repeat(32), revision: 7, payload: {} });
    const memory = createShellLayoutMemory(directory, (_temporary, target) => writeFileSync(target, future));
    assert.equal(memory.read().diagnostic, "write_failed");
    assert.equal(readFileSync(shellLayoutMemoryPath(directory), "utf8"), future);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
