import assert from "node:assert/strict";
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { once } from "node:events";
import { existsSync, mkdtempSync, readFileSync, readdirSync, renameSync, rmSync, statSync, truncateSync, writeFileSync } from "node:fs";
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

test("shell layout returns a diagnostic for a FIFO record without blocking", (t) => {
  if (process.platform === "win32") return t.skip("FIFOs are unavailable on Windows");
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-fifo-"));
  try {
    const path = shellLayoutMemoryPath(directory);
    try { execFileSync("mkfifo", [path]); }
    catch { return t.skip("mkfifo is unavailable"); }
    const probe = spawnSync(process.execPath, ["--experimental-strip-types", "--input-type=module", "-e", `
      import { createShellLayoutMemory } from "./electron/shellLayoutMemory.ts";
      const result = createShellLayoutMemory(process.argv[1]).read();
      if (result.diagnostic !== "corrupt_record") process.exit(1);
    `, directory], { encoding: "utf8", timeout: 2_000 });
    assert.equal(probe.error, undefined, "reading the FIFO record must not block");
    assert.equal(probe.status, 0, probe.stderr || "child process failed");
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("confirmed shell-layout rebuild can replace a FIFO record", async (t) => {
  if (process.platform === "win32") return t.skip("FIFOs are unavailable on Windows");
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-fifo-rebuild-"));
  try {
    const path = shellLayoutMemoryPath(directory);
    try { execFileSync("mkfifo", [path]); }
    catch { return t.skip("mkfifo is unavailable"); }
    const memory = createShellLayoutMemory(directory);
    assert.equal(memory.read().diagnostic, "corrupt_record");

    assert.deepEqual(await memory.rebuild(true), { ok: true });

    assert.equal(statSync(path).isFile(), true);
    assert.equal(memory.read().diagnostic, null);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("failed confirmed rebuild restores the prior FIFO inode", async (t) => {
  if (process.platform === "win32") return t.skip("FIFOs are unavailable on Windows");
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-fifo-rollback-"));
  try {
    const path = shellLayoutMemoryPath(directory);
    try { execFileSync("mkfifo", [path]); }
    catch { return t.skip("mkfifo is unavailable"); }
    const prior = statSync(path);
    const memory = createShellLayoutMemory(directory, (temporary, target) => {
      renameSync(temporary, target);
      throw new Error("injected read-back failure");
    });

    assert.deepEqual(await memory.rebuild(true), { ok: false, diagnostic: "write_failed" });

    const restored = statSync(path);
    assert.equal(restored.isFIFO(), true);
    assert.equal(restored.dev, prior.dev);
    assert.equal(restored.ino, prior.ino);
    assert.equal(readdirSync(directory).some((name) => name.endsWith(".bak")), false);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("failed read-back restores a prior FIFO when target reads keep failing", (t) => {
  if (process.platform === "win32") return t.skip("FIFOs are unavailable on Windows");
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-fifo-readback-"));
  try {
    const target = shellLayoutMemoryPath(directory);
    try { execFileSync("mkfifo", [target]); }
    catch { return t.skip("mkfifo is unavailable"); }
    const moduleUrl = new URL("../electron/shellLayoutMemory.ts", import.meta.url).href;
    const script = `
      import fs from 'node:fs';
      import { syncBuiltinESMExports } from 'node:module';
      const { createShellLayoutMemory } = await import(${JSON.stringify(moduleUrl)});
      const target = process.argv[1];
      const directory = process.argv[2];
      const prior = fs.statSync(target);
      const originalOpen = fs.openSync;
      let failedReads = 0;
      fs.openSync = (path, ...args) => {
        if (path === target && failedReads > 0) { failedReads -= 1; throw new Error('injected target read failure'); }
        return originalOpen(path, ...args);
      };
      syncBuiltinESMExports();
      const memory = createShellLayoutMemory(directory, (temporary, path) => {
        fs.renameSync(temporary, path);
        failedReads = 2;
      });
      const result = await memory.rebuild(true);
      const restored = fs.statSync(target);
      if (result.diagnostic !== 'write_failed' || !restored.isFIFO()
        || restored.dev !== prior.dev || restored.ino !== prior.ino
        || fs.readdirSync(directory).some((name) => name.endsWith('.bak'))) {
        console.error(JSON.stringify({ result, restored: { fifo: restored.isFIFO(), dev: restored.dev, ino: restored.ino }, prior: { dev: prior.dev, ino: prior.ino } }));
        process.exitCode = 1;
      }
    `;
    const child = spawnSync(process.execPath, ["--experimental-strip-types", "--input-type=module", "--eval", script, target, directory], { encoding: "utf8" });
    assert.equal(child.status, 0, child.stderr);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("failed read-back restores prior record bytes when target reads keep failing", () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-readback-bytes-"));
  try {
    const target = shellLayoutMemoryPath(directory);
    const moduleUrl = new URL("../electron/shellLayoutMemory.ts", import.meta.url).href;
    const script = `
      import fs from 'node:fs';
      import { syncBuiltinESMExports } from 'node:module';
      const { createShellLayoutMemory } = await import(${JSON.stringify(moduleUrl)});
      const target = process.argv[1];
      const directory = process.argv[2];
      const originalOpen = fs.openSync;
      let failedReads = 0;
      let injectFailure = false;
      fs.openSync = (path, ...args) => {
        if (path === target && failedReads > 0) { failedReads -= 1; throw new Error('injected target read failure'); }
        return originalOpen(path, ...args);
      };
      syncBuiltinESMExports();
      const memory = createShellLayoutMemory(directory, (temporary, path) => {
        fs.renameSync(temporary, path);
        if (injectFailure) failedReads = 2;
      });
      const initial = memory.read();
      if (initial.diagnostic !== null || !fs.existsSync(target)) throw new Error('initial shell record was not published');
      const prior = fs.readFileSync(target);
      injectFailure = true;
      const result = await memory.rebuild(true);
      const restored = fs.existsSync(target) ? fs.readFileSync(target) : null;
      if (result.diagnostic !== 'write_failed' || !restored?.equals(prior)) {
        console.error(JSON.stringify({ result, restored: restored?.toString() ?? null, prior: prior.toString() }));
        process.exitCode = 1;
      }
    `;
    const child = spawnSync(process.execPath, ["--experimental-strip-types", "--input-type=module", "--eval", script, target, directory], { encoding: "utf8" });
    assert.equal(child.status, 0, child.stderr);
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

test("first-use candidate published before replacer throws restores prior absence", () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const memory = createShellLayoutMemory(directory, (temporary, target) => {
      writeFileSync(target, readFileSync(temporary));
      throw new Error("injected post-publication failure");
    });
    assert.equal(memory.read().diagnostic, "write_failed");
    assert.equal(existsSync(shellLayoutMemoryPath(directory)), false);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("oversized shell record is diagnosed and rebuilt without unbounded target reads", () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const target = shellLayoutMemoryPath(directory);
    writeFileSync(target, "old record");
    truncateSync(target, 4 * 1024 * 1024);
    const moduleUrl = new URL("../electron/shellLayoutMemory.ts", import.meta.url).href;
    const script = `
      import fs from 'node:fs';
      import { syncBuiltinESMExports } from 'node:module';
      const target = process.argv[1];
      const originalRead = fs.readFileSync;
      let targetReads = 0;
      fs.readFileSync = (...args) => {
        if (args[0] === target) targetReads += 1;
        return originalRead(...args);
      };
      syncBuiltinESMExports();
      const { createShellLayoutMemory } = await import(${JSON.stringify(moduleUrl)});
      const memory = createShellLayoutMemory(process.argv[2]);
      const diagnostic = memory.read().diagnostic;
      const rebuilt = await memory.rebuild(true);
      if (diagnostic !== 'corrupt_record' || !rebuilt.ok || targetReads !== 0) {
        console.error(JSON.stringify({ diagnostic, rebuilt, targetReads }));
        process.exitCode = 1;
      }
    `;
    const child = spawnSync(process.execPath, ["--experimental-strip-types", "--input-type=module", "--eval", script, target, directory], { encoding: "utf8" });
    assert.equal(child.status, 0, child.stderr);
    assert.ok(statSync(target).size <= 16 * 1024);
    assert.equal(readdirSync(directory).some((name) => name.endsWith(".bak")), false);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("a record that grows after size preflight is read only to the bound plus one", () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const target = shellLayoutMemoryPath(directory);
    writeFileSync(target, "old record");
    truncateSync(target, 4 * 1024 * 1024);
    const moduleUrl = new URL("../electron/shellLayoutMemory.ts", import.meta.url).href;
    const script = `
      import fs from 'node:fs';
      import { syncBuiltinESMExports } from 'node:module';
      const { createShellLayoutMemory } = await import(${JSON.stringify(moduleUrl)});
      const originalStat = fs.fstatSync;
      const originalRead = fs.readSync;
      let bytesRead = 0;
      fs.fstatSync = (...args) => new Proxy(originalStat(...args), {
        get(target, property, receiver) { return property === 'size' ? 0 : Reflect.get(target, property, receiver); },
      });
      fs.readSync = (...args) => {
        const count = originalRead(...args);
        bytesRead += count;
        return count;
      };
      syncBuiltinESMExports();
      const diagnostic = createShellLayoutMemory(process.argv[1]).read().diagnostic;
      if (diagnostic !== 'corrupt_record' || bytesRead !== 16 * 1024 + 1) {
        console.error(JSON.stringify({ diagnostic, bytesRead }));
        process.exitCode = 1;
      }
    `;
    const child = spawnSync(process.execPath, ["--experimental-strip-types", "--input-type=module", "--eval", script, directory], { encoding: "utf8" });
    assert.equal(child.status, 0, child.stderr);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("failed confirmed rebuild restores an oversized prior inode", async () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const target = shellLayoutMemoryPath(directory);
    writeFileSync(target, "old record");
    truncateSync(target, 4 * 1024 * 1024);
    const prior = statSync(target);
    const memory = createShellLayoutMemory(directory, (temporary, destination) => {
      const malformed = `${temporary}.malformed`;
      writeFileSync(malformed, "{mismatch");
      renameSync(malformed, destination);
    });
    assert.equal(memory.read().diagnostic, "corrupt_record");
    assert.deepEqual(await memory.rebuild(true), { ok: false, diagnostic: "write_failed" });
    const restored = statSync(target);
    assert.equal(restored.ino, prior.ino);
    assert.equal(restored.size, prior.size);
    assert.equal(readdirSync(directory).some((name) => name.endsWith(".bak")), false);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("failed rebuild retains an oversized backup when prior restoration cannot be verified", async () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const target = shellLayoutMemoryPath(directory);
    const source = Buffer.from("prior record ".repeat(2_000));
    writeFileSync(target, source);
    const memory = createShellLayoutMemory(directory, (temporary, path) => {
      renameSync(temporary, path);
      throw new Error("injected publication failure");
    }, () => { throw new Error("injected restoration failure"); });

    assert.deepEqual(await memory.rebuild(true), { ok: false, diagnostic: "restore_failed" });

    const backups = readdirSync(directory).filter((name) => name.endsWith(".bak"));
    assert.equal(backups.length, 1);
    assert.deepEqual(readFileSync(join(directory, backups[0])), source);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("unsupported oversized backup leaves the record untouched", () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    const target = shellLayoutMemoryPath(directory);
    writeFileSync(target, "old record");
    truncateSync(target, 4 * 1024 * 1024);
    const prior = statSync(target);
    const moduleUrl = new URL("../electron/shellLayoutMemory.ts", import.meta.url).href;
    const script = `
      import fs from 'node:fs';
      import { syncBuiltinESMExports } from 'node:module';
      fs.linkSync = () => { throw new Error('hardlinks unavailable'); };
      syncBuiltinESMExports();
      const { createShellLayoutMemory } = await import(${JSON.stringify(moduleUrl)});
      const result = await createShellLayoutMemory(process.argv[1]).rebuild(true);
      if (result.ok || result.diagnostic !== 'write_failed') process.exitCode = 1;
    `;
    const child = spawnSync(process.execPath, ["--experimental-strip-types", "--input-type=module", "--eval", script, directory], { encoding: "utf8" });
    assert.equal(child.status, 0, child.stderr);
    const after = statSync(target);
    assert.equal(after.ino, prior.ino);
    assert.equal(after.size, prior.size);
    assert.equal(readdirSync(directory).some((name) => name.endsWith(".bak")), false);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("malformed UTF-8 read-back still restores the prior bounded record", async () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-layout-"));
  try {
    let replacements = 0;
    const memory = createShellLayoutMemory(directory, (temporary, target) => {
      replacements += 1;
      writeFileSync(target, replacements === 2 ? Buffer.from([0xff]) : readFileSync(temporary));
    });
    memory.read();
    const target = shellLayoutMemoryPath(directory);
    const prior = readFileSync(target);
    const result = await memory.save({ sessionsWidth: 250, inspectorWidth: 320, sessionsVisible: true, density: "low" });
    assert.deepEqual(result, { ok: false, diagnostic: "write_failed" });
    assert.deepEqual(readFileSync(target), prior);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
