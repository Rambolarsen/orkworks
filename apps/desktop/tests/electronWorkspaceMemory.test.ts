import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { setTimeout as delay } from "node:timers/promises";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  forgetWorkspacePath,
  canonicalWorkspacePath,
  accessibleWorkspaceDirectoryPath,
  readWorkspaceMemory,
  rememberWorkspacePath,
  pinWorkspacePath,
  unpinWorkspacePath,
  workspaceMemoryPath,
} from "../electron/workspaceMemory.ts";

const EMPTY_MEMORY = {
  version: 2,
  revision: 0,
  lastWorkspacePath: null,
  recentWorkspacePaths: [],
  pinnedWorkspacePaths: [],
  diagnostic: null,
} as const;

function withTemporaryUserData(run: (directory: string) => void | Promise<void>): Promise<void> {
  const directory = mkdtempSync(join(tmpdir(), "orkworks-memory-"));
  return Promise.resolve(run(directory)).finally(() => {
    rmSync(directory, { recursive: true, force: true });
  });
}

test("workspace memory round-trips the version and increasing revision", () =>
  withTemporaryUserData((directory) => {
    assert.deepEqual(readWorkspaceMemory(directory), EMPTY_MEMORY);

    const first = rememberWorkspacePath(directory, "/repo/a");
    const second = rememberWorkspacePath(directory, "/repo/b");

    assert.equal(first.version, 2);
    assert.equal(first.revision, 1);
    assert.equal(second.revision, 2);
    assert.deepEqual(readWorkspaceMemory(directory), second);
    assert.deepEqual(JSON.parse(readFileSync(workspaceMemoryPath(directory), "utf8")), {
      version: 2,
      revision: 2,
      lastWorkspacePath: "/repo/b",
      recentWorkspacePaths: ["/repo/b", "/repo/a"],
      pinnedWorkspacePaths: [],
    });
  }));

test("workspace history replacement seam handles a second write over an existing target", () =>
  withTemporaryUserData((directory) => {
    const replacements: boolean[] = [];
    const replace = (source: string, target: string, targetExists: boolean): void => {
      replacements.push(targetExists);
      writeFileSync(target, readFileSync(source));
      rmSync(source, { force: true });
    };

    const first = rememberWorkspacePath(directory, "/repo/a", replace);
    const second = rememberWorkspacePath(directory, "/repo/b", replace);

    assert.equal(first.diagnostic, null);
    assert.equal(second.diagnostic, null);
    assert.deepEqual(replacements, [false, true]);
    assert.deepEqual(readWorkspaceMemory(directory), second);
  }));

test("Windows workspace history replaces an existing target on the second write", {
  skip: process.platform !== "win32",
}, () => withTemporaryUserData((directory) => {
  rememberWorkspacePath(directory, "/repo/a");
  const second = rememberWorkspacePath(directory, "/repo/b");

  assert.equal(second.diagnostic, null);
  assert.equal(second.revision, 2);
  assert.equal(readWorkspaceMemory(directory).lastWorkspacePath, "/repo/b");
}));

test("workspace memory survives a fresh process using the same application-data directory", () =>
  withTemporaryUserData(async (directory) => {
    rememberWorkspacePath(directory, "/repo/a");
    rememberWorkspacePath(directory, "/repo/b");

    const moduleUrl = new URL("../electron/workspaceMemory.ts", import.meta.url).href;
    const script = `import { readWorkspaceMemory } from ${JSON.stringify(moduleUrl)};
process.stdout.write(JSON.stringify(readWorkspaceMemory(process.argv[1])));`;
    const child = spawn(process.execPath, [
      "--experimental-strip-types",
      "--input-type=module",
      "--eval",
      script,
      directory,
    ]);
    let stdout = "";
    child.stdout.on("data", (chunk: Buffer) => { stdout += chunk.toString(); });
    const [code] = await once(child, "close");

    assert.equal(code, 0);
    assert.deepEqual(JSON.parse(stdout), {
      version: 2,
      revision: 2,
      lastWorkspacePath: "/repo/b",
      recentWorkspacePaths: ["/repo/b", "/repo/a"],
      pinnedWorkspacePaths: [],
      diagnostic: null,
    });
  }));

test("rememberWorkspacePath keeps at most 20 newest paths", () =>
  withTemporaryUserData((directory) => {
    for (let index = 0; index < 25; index += 1) {
      rememberWorkspacePath(directory, `/repo/${index}`);
    }

    const memory = readWorkspaceMemory(directory);
    assert.equal(memory.revision, 25);
    assert.equal(memory.lastWorkspacePath, "/repo/24");
    assert.deepEqual(
      memory.recentWorkspacePaths,
      Array.from({ length: 20 }, (_, index) => `/repo/${24 - index}`),
    );
  }));

test("rememberWorkspacePath evicts oldest entries until serialized history is at most 64 KiB", () =>
  withTemporaryUserData((directory) => {
    for (let index = 0; index < 20; index += 1) {
      rememberWorkspacePath(directory, `/repo/${index}-${"x".repeat(4_000)}`);
    }

    const serialized = readFileSync(workspaceMemoryPath(directory), "utf8");
    const memory = readWorkspaceMemory(directory);
    assert.ok(Buffer.byteLength(serialized, "utf8") <= 64 * 1024);
    assert.ok(memory.recentWorkspacePaths.length < 20);
    assert.equal(memory.recentWorkspacePaths[0], `/repo/19-${"x".repeat(4_000)}`);
  }));

test("rememberWorkspacePath deduplicates canonical paths and keeps the newest first", () =>
  withTemporaryUserData((directory) => {
    rememberWorkspacePath(directory, "/repo/a");
    rememberWorkspacePath(directory, "/repo/b");
    const memory = rememberWorkspacePath(directory, "/repo/a");

    assert.equal(memory.revision, 3);
    assert.equal(memory.lastWorkspacePath, "/repo/a");
    assert.deepEqual(memory.recentWorkspacePaths, ["/repo/a", "/repo/b"]);
  }));

test("forgetWorkspacePath removes only the selected shortcut without deleting its workspace", () =>
  withTemporaryUserData((directory) => {
    const workspace = join(directory, "workspace-a");
    const marker = join(workspace, "keep.txt");
    mkdirSync(workspace);
    writeFileSync(marker, "keep");
    rememberWorkspacePath(directory, workspace);
    rememberWorkspacePath(directory, "/repo/other");
    rememberWorkspacePath(directory, workspace);

    const memory = forgetWorkspacePath(directory, workspace);

    assert.equal(memory.revision, 4);
    assert.equal(memory.lastWorkspacePath, null);
    assert.deepEqual(memory.recentWorkspacePaths, ["/repo/other"]);
    assert.equal(readFileSync(marker, "utf8"), "keep");
    assert.equal(existsSync(workspace), true);
  }));

test("forgetWorkspacePath leaves memory and revision untouched for an unknown path", () =>
  withTemporaryUserData((directory) => {
    const remembered = rememberWorkspacePath(directory, "/repo/a");
    const result = forgetWorkspacePath(directory, "/repo/other");

    assert.deepEqual(result, remembered);
    assert.deepEqual(readWorkspaceMemory(directory), remembered);
  }));

test("forgetWorkspacePath removes a pinned path from pinnedWorkspacePaths", () =>
  withTemporaryUserData((directory) => {
    pinWorkspacePath(directory, "/repo/pinned");
    const memory = forgetWorkspacePath(directory, "/repo/pinned");

    assert.equal(memory.diagnostic, null);
    assert.deepEqual(memory.pinnedWorkspacePaths, []);
    assert.deepEqual(memory.recentWorkspacePaths, []);
    assert.equal(memory.lastWorkspacePath, null);
  }));

test("pinWorkspacePath moves a path out of recentWorkspacePaths and unpinWorkspacePath reinserts it at the front", () =>
  withTemporaryUserData((directory) => {
    rememberWorkspacePath(directory, "/repo/a");
    rememberWorkspacePath(directory, "/repo/b");

    const pinned = pinWorkspacePath(directory, "/repo/a");
    assert.equal(pinned.diagnostic, null);
    assert.deepEqual(pinned.pinnedWorkspacePaths, ["/repo/a"]);
    assert.deepEqual(pinned.recentWorkspacePaths, ["/repo/b"]);
    // Pinning does not open the workspace, so lastWorkspacePath is
    // unchanged from the last remember call.
    assert.equal(pinned.lastWorkspacePath, "/repo/b");

    const unpinned = unpinWorkspacePath(directory, "/repo/a");
    assert.equal(unpinned.diagnostic, null);
    assert.deepEqual(unpinned.pinnedWorkspacePaths, []);
    assert.deepEqual(unpinned.recentWorkspacePaths, ["/repo/a", "/repo/b"]);
  }));

test("pinWorkspacePath is a no-op when the path is already pinned", () =>
  withTemporaryUserData((directory) => {
    const first = pinWorkspacePath(directory, "/repo/a");
    const second = pinWorkspacePath(directory, "/repo/a");
    assert.deepEqual(second, first);
  }));

test("unpinWorkspacePath is a no-op for a path that is not pinned", () =>
  withTemporaryUserData((directory) => {
    const remembered = rememberWorkspacePath(directory, "/repo/a");
    const result = unpinWorkspacePath(directory, "/repo/a");
    assert.deepEqual(result, remembered);
  }));

test("pinWorkspacePath rejects a 51st pin with a distinct diagnostic instead of evicting", () =>
  withTemporaryUserData((directory) => {
    for (let index = 0; index < 50; index += 1) {
      const result = pinWorkspacePath(directory, `/repo/${index}`);
      assert.equal(result.diagnostic, null);
    }
    const overflow = pinWorkspacePath(directory, "/repo/overflow");
    assert.equal(overflow.diagnostic?.code, "pin_limit_reached");
    assert.equal(overflow.pinnedWorkspacePaths.length, 50);
    assert.equal(overflow.pinnedWorkspacePaths.includes("/repo/overflow"), false);
  }));

test("rememberWorkspacePath updates lastWorkspacePath without duplicating an already-pinned path into recentWorkspacePaths", () =>
  withTemporaryUserData((directory) => {
    rememberWorkspacePath(directory, "/repo/other");
    const pinned = pinWorkspacePath(directory, "/repo/pinned");
    assert.deepEqual(pinned.pinnedWorkspacePaths, ["/repo/pinned"]);
    assert.deepEqual(pinned.recentWorkspacePaths, ["/repo/other"]);

    const remembered = rememberWorkspacePath(directory, "/repo/pinned");

    assert.equal(remembered.diagnostic, null);
    assert.equal(remembered.lastWorkspacePath, "/repo/pinned");
    assert.deepEqual(remembered.pinnedWorkspacePaths, ["/repo/pinned"]);
    assert.deepEqual(remembered.recentWorkspacePaths, ["/repo/other"]);
    assert.equal(remembered.revision, pinned.revision + 1);
  }));

test("a v2 file whose lastWorkspacePath is pinned-only (not in recentWorkspacePaths) reloads without a corrupt diagnostic", () =>
  withTemporaryUserData((directory) => {
    pinWorkspacePath(directory, "/repo/pinned");
    const remembered = rememberWorkspacePath(directory, "/repo/pinned");
    assert.equal(remembered.diagnostic, null);
    assert.equal(remembered.lastWorkspacePath, "/repo/pinned");

    const reloaded = readWorkspaceMemory(directory);
    assert.equal(reloaded.diagnostic, null);
    assert.equal(reloaded.lastWorkspacePath, "/repo/pinned");
    assert.deepEqual(reloaded.pinnedWorkspacePaths, ["/repo/pinned"]);
    assert.deepEqual(reloaded.recentWorkspacePaths, []);
  }));

test("legacy workspace history without version/revision fields is migrated instead of diagnosed as corrupt", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const legacy = {
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a", "/repo/b"],
    };
    writeFileSync(historyPath, JSON.stringify(legacy, null, 2));

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic, null);
    assert.equal(loaded.version, 2);
    assert.equal(loaded.lastWorkspacePath, "/repo/a");
    // Secondary entries are dropped during migration (see the dedicated
    // "drops secondary entries" test below for why).
    assert.deepEqual(loaded.recentWorkspacePaths, ["/repo/a"]);
    assert.deepEqual(loaded.pinnedWorkspacePaths, []);

    const remembered = rememberWorkspacePath(directory, "/repo/c");

    assert.equal(remembered.diagnostic, null);
    assert.equal(remembered.lastWorkspacePath, "/repo/c");
    assert.deepEqual(remembered.recentWorkspacePaths, ["/repo/c", "/repo/a"]);
    assert.deepEqual(JSON.parse(readFileSync(historyPath, "utf8")), {
      version: 2,
      revision: remembered.revision,
      lastWorkspacePath: "/repo/c",
      recentWorkspacePaths: ["/repo/c", "/repo/a"],
      pinnedWorkspacePaths: [],
    });
  }));

test("legacy workspace history with a cleared lastWorkspacePath still migrates", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const legacy = {
      lastWorkspacePath: null,
      recentWorkspacePaths: ["/repo/a", "/repo/b"],
    };
    writeFileSync(historyPath, JSON.stringify(legacy, null, 2));

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic, null);
    assert.equal(loaded.lastWorkspacePath, null);
    assert.deepEqual(loaded.recentWorkspacePaths, []);
    assert.deepEqual(loaded.pinnedWorkspacePaths, []);
  }));

test("legacy workspace history canonicalizes lastWorkspacePath and drops secondary entries", () =>
  withTemporaryUserData((directory) => {
    const realPath = join(directory, "real-workspace");
    const aliasPath = join(directory, "alias-workspace");
    mkdirSync(realPath);
    symlinkSync(realPath, aliasPath, "dir");
    const expectedPath = canonicalWorkspacePath(realPath);

    const historyPath = workspaceMemoryPath(directory);
    const legacy = {
      lastWorkspacePath: aliasPath,
      recentWorkspacePaths: [aliasPath, "/repo/other", "/repo/missing"],
    };
    writeFileSync(historyPath, JSON.stringify(legacy, null, 2));

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic, null);
    assert.equal(loaded.lastWorkspacePath, expectedPath);
    assert.deepEqual(loaded.recentWorkspacePaths, [expectedPath]);
  }));

test("legacy workspace history over the old 64 KiB gate but under the legacy ceiling still migrates", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const paths = Array.from({ length: 5 }, (_, index) => `/repo/${index}-${"x".repeat(15_000)}`);
    const legacy = { lastWorkspacePath: paths[0], recentWorkspacePaths: paths };
    const raw = JSON.stringify(legacy);
    assert.ok(Buffer.byteLength(raw, "utf8") > 64 * 1024);
    assert.ok(Buffer.byteLength(raw, "utf8") <= 2 * 1024 * 1024);
    writeFileSync(historyPath, raw);

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic, null);
    assert.equal(loaded.lastWorkspacePath, paths[0]);
  }));

test("legacy workspace history with more entries than the old writer's 10-entry cap is diagnosed as corrupt", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const legacy = {
      lastWorkspacePath: "/repo/0",
      recentWorkspacePaths: Array.from({ length: 11 }, (_, index) => `/repo/${index}`),
    };
    const raw = JSON.stringify(legacy);
    writeFileSync(historyPath, raw);

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic?.code, "corrupt_history");
    assert.equal(readFileSync(historyPath, "utf8"), raw);
  }));

test("legacy workspace history whose migrated record cannot fit is diagnosed, not silently truncated", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const oversizedPath = `/repo/${"x".repeat(50_000)}`;
    const legacy = {
      lastWorkspacePath: oversizedPath,
      recentWorkspacePaths: [] as string[],
    };
    const raw = JSON.stringify(legacy);
    assert.ok(Buffer.byteLength(raw, "utf8") <= 64 * 1024);
    writeFileSync(historyPath, raw);

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic?.code, "corrupt_history");
    assert.equal(loaded.lastWorkspacePath, null);
    assert.deepEqual(loaded.recentWorkspacePaths, []);
    assert.equal(readFileSync(historyPath, "utf8"), raw);
  }));

test("a v1 file with no pinnedWorkspacePaths key migrates with an empty pinned list", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const v1 = {
      version: 1,
      revision: 5,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a", "/repo/b"],
    };
    writeFileSync(historyPath, JSON.stringify(v1));

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic, null);
    assert.equal(loaded.version, 2);
    assert.equal(loaded.revision, 5);
    assert.equal(loaded.lastWorkspacePath, "/repo/a");
    assert.deepEqual(loaded.recentWorkspacePaths, ["/repo/a", "/repo/b"]);
    assert.deepEqual(loaded.pinnedWorkspacePaths, []);

    const pinned = pinWorkspacePath(directory, "/repo/a");
    assert.equal(pinned.diagnostic, null);
    assert.deepEqual(JSON.parse(readFileSync(historyPath, "utf8")), {
      version: 2,
      revision: 6,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/b"],
      pinnedWorkspacePaths: ["/repo/a"],
    });
  }));

test("corrupt workspace history is diagnosed and preserved across mutation attempts", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const corrupt = "{ definitely not valid workspace history";
    writeFileSync(historyPath, corrupt);

    const loaded = readWorkspaceMemory(directory);
    const remembered = rememberWorkspacePath(directory, "/repo/a");
    const forgotten = forgetWorkspacePath(directory, "/repo/a");
    const pinned = pinWorkspacePath(directory, "/repo/a");
    const unpinned = unpinWorkspacePath(directory, "/repo/a");

    assert.deepEqual(loaded, {
      ...EMPTY_MEMORY,
      diagnostic: {
        code: "corrupt_history",
        message: "Workspace history is corrupt or unreadable and was left unchanged.",
      },
    });
    assert.deepEqual(remembered, loaded);
    assert.deepEqual(forgotten, loaded);
    assert.deepEqual(pinned, loaded);
    assert.deepEqual(unpinned, loaded);
    assert.equal(readFileSync(historyPath, "utf8"), corrupt);
  }));

test("malformed UTF-8 workspace history is diagnosed and preserved byte-for-byte", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const corrupt = Buffer.concat([
      Buffer.from('{"version":1,"revision":1,"lastWorkspacePath":"/repo/'),
      Buffer.from([0x80]),
      Buffer.from('","recentWorkspacePaths":["/repo/'),
      Buffer.from([0x80]),
      Buffer.from('"]}\n'),
    ]);
    writeFileSync(historyPath, corrupt);

    const memory = readWorkspaceMemory(directory);

    assert.equal(memory.diagnostic?.code, "corrupt_history");
    assert.equal(memory.revision, 0);
    assert.deepEqual(readFileSync(historyPath), corrupt);
  }));

test("workspace history rejects unknown persisted fields without overwriting the source", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const corrupt = JSON.stringify({
      version: 1,
      revision: 0,
      lastWorkspacePath: null,
      recentWorkspacePaths: [],
      unexpected: true,
    });
    writeFileSync(historyPath, corrupt);

    const memory = readWorkspaceMemory(directory);

    assert.equal(memory.diagnostic?.code, "corrupt_history");
    assert.equal(readFileSync(historyPath, "utf8"), corrupt);
  }));

test("workspace history rejects a valid-looking file whose serialized bytes exceed the bound", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const record = `${JSON.stringify({
      version: 2,
      revision: 1,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a"],
      pinnedWorkspacePaths: [],
    })}${" ".repeat(64 * 1024)}\n`;
    assert.ok(Buffer.byteLength(record, "utf8") > 64 * 1024);
    writeFileSync(historyPath, record);

    const memory = readWorkspaceMemory(directory);

    assert.equal(memory.diagnostic?.code, "corrupt_history");
    assert.equal(memory.revision, 0);
    assert.equal(readFileSync(historyPath, "utf8"), record);
  }));

test("workspace history rejects oversized and duplicate records without normalizing them", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const cases = [
      {
        version: 2,
        revision: 2,
        lastWorkspacePath: "/repo/a",
        recentWorkspacePaths: Array.from({ length: 21 }, (_, index) => `/repo/${index}`),
        pinnedWorkspacePaths: [],
      },
      {
        version: 2,
        revision: 3,
        lastWorkspacePath: "/repo/a",
        recentWorkspacePaths: ["/repo/a", "/repo/a"],
        pinnedWorkspacePaths: [],
      },
      {
        version: 2,
        revision: 4,
        lastWorkspacePath: "/repo/a",
        recentWorkspacePaths: ["/repo/a"],
        pinnedWorkspacePaths: ["/repo/a"],
      },
    ];

    for (const record of cases) {
      const source = `${JSON.stringify(record)}\n`;
      writeFileSync(historyPath, source);
      const memory = readWorkspaceMemory(directory);
      assert.equal(memory.diagnostic?.code, "corrupt_history");
      assert.equal(memory.revision, 0);
      assert.equal(readFileSync(historyPath, "utf8"), source);
    }
  }));

test("startup workspace validation accepts only accessible directories", () =>
  withTemporaryUserData((directory) => {
    const workspace = join(directory, "workspace");
    const regularFile = join(directory, "workspace.txt");
    mkdirSync(workspace);
    writeFileSync(regularFile, "not a workspace");

    assert.equal(accessibleWorkspaceDirectoryPath(workspace), realpathSync(workspace));
    assert.equal(accessibleWorkspaceDirectoryPath(regularFile), null);
    assert.equal(statSync(regularFile).isDirectory(), false);
  }));

test("a live OS lock is never evicted, even after five seconds; process exit releases it", () =>
  withTemporaryUserData(async (directory) => {
    const lockPath = join(directory, ".workspace-memory.lock");
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
      const fresh = rememberWorkspacePath(directory, "/repo/not-written");
      assert.equal(fresh.diagnostic?.code, "history_lock_timeout");
      await delay(5_100);
      const old = rememberWorkspacePath(directory, "/repo/still-not-written");
      assert.equal(old.diagnostic?.code, "history_lock_timeout");
      assert.equal(existsSync(workspaceMemoryPath(directory)), false);
    } finally {
      child.kill("SIGKILL");
      await exited;
    }
    assert.equal(existsSync(lockPath), true, "the lock inode must be retained");
    const recovered = rememberWorkspacePath(directory, "/repo/recovered");
    assert.equal(recovered.diagnostic, null);
    assert.equal(recovered.revision, 1);
  }));

test("revision overflow rejects mutations before writing and preserves history bytes", () =>
  withTemporaryUserData((directory) => {
    const original = JSON.stringify({
      version: 1,
      revision: Number.MAX_SAFE_INTEGER,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a"],
    });
    writeFileSync(workspaceMemoryPath(directory), original);
    // pinWorkspacePath("/repo/a") is a genuine change here (the migrated v1
    // file has an empty pinnedWorkspacePaths, so pinning is not a no-op) and
    // so reaches the revision-exhausted check like remember/forget do.
    // unpinWorkspacePath is deliberately NOT included in this loop: since
    // "/repo/a" is never pinned in this v1-migrated fixture, unpinning it
    // would return the noChange sentinel and return early *before* the
    // revision check runs at all — see the dedicated pinned-fixture test
    // below for unpin's revision-exhaustion behavior instead.
    for (const mutate of [rememberWorkspacePath, forgetWorkspacePath, pinWorkspacePath]) {
      const result = mutate(directory, "/repo/a");
      assert.equal(result.diagnostic?.code, "history_write_failed");
      assert.equal(result.revision, Number.MAX_SAFE_INTEGER);
      assert.equal(readFileSync(workspaceMemoryPath(directory), "utf8"), original);
      assert.equal(readWorkspaceMemory(directory).diagnostic, null);
    }
    assert.equal(forgetWorkspacePath(directory, "/unknown").diagnostic, null);
  }));

test("revision overflow rejects unpinWorkspacePath before writing and preserves history bytes", () =>
  withTemporaryUserData((directory) => {
    // unpinWorkspacePath only reaches the revision-exhausted check when
    // unpinning is a genuine change, which requires the path to already be
    // pinned — not expressible in a v1 fixture (v1 has no pinned list), so
    // this uses a v2 fixture directly instead of migrating one.
    const original = JSON.stringify({
      version: 2,
      revision: Number.MAX_SAFE_INTEGER,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: [],
      pinnedWorkspacePaths: ["/repo/a"],
    });
    writeFileSync(workspaceMemoryPath(directory), original);

    const result = unpinWorkspacePath(directory, "/repo/a");

    assert.equal(result.diagnostic?.code, "history_write_failed");
    assert.equal(result.revision, Number.MAX_SAFE_INTEGER);
    assert.equal(readFileSync(workspaceMemoryPath(directory), "utf8"), original);
    assert.equal(readWorkspaceMemory(directory).diagnostic, null);
  }));

test("workspace history canonicalizes an alias before it is remembered", () =>
  withTemporaryUserData((directory) => {
    const realPath = join(directory, "real-workspace");
    const aliasPath = join(directory, "alias-workspace");
    mkdirSync(realPath);
    symlinkSync(realPath, aliasPath, "dir");

    const canonicalPath = canonicalWorkspacePath(aliasPath);
    const expectedPath = canonicalWorkspacePath(realPath);
    assert.equal(canonicalPath, expectedPath);
    const memory = rememberWorkspacePath(directory, canonicalPath!);

    assert.deepEqual(memory.recentWorkspacePaths, [expectedPath]);
    assert.doesNotMatch(readFileSync(workspaceMemoryPath(directory), "utf8"), /alias-workspace/);
  }));

test("workspace history evicts using the actual next revision size", () =>
  withTemporaryUserData((directory) => {
    const revision = 999_999_999;
    const prefix = "/repo/";
    const baseRecord = (padding: number, candidateRevision: number) => ({
      version: 1,
      revision: candidateRevision,
      lastWorkspacePath: `${prefix}${"x".repeat(padding)}`,
      recentWorkspacePaths: [
        `${prefix}${"x".repeat(padding)}`,
        "/repo/oldest",
      ],
    });
    let padding = 0;
    while (
      Buffer.byteLength(`${JSON.stringify(baseRecord(padding, revision), null, 2)}\n`, "utf8") <= 64 * 1024
      && Buffer.byteLength(`${JSON.stringify(baseRecord(padding, revision + 1), null, 2)}\n`, "utf8") <= 64 * 1024
    ) {
      padding += 1;
    }
    while (Buffer.byteLength(`${JSON.stringify(baseRecord(padding, revision), null, 2)}\n`, "utf8") > 64 * 1024) {
      padding -= 1;
    }
    writeFileSync(workspaceMemoryPath(directory), `${JSON.stringify(baseRecord(padding, revision), null, 2)}\n`);

    const memory = rememberWorkspacePath(directory, "/repo/new");

    assert.equal(memory.diagnostic, null);
    assert.equal(memory.revision, revision + 1);
    assert.equal(memory.recentWorkspacePaths[0], "/repo/new");
    assert.ok(Buffer.byteLength(readFileSync(workspaceMemoryPath(directory), "utf8"), "utf8") <= 64 * 1024);
  }));

test("a single path that cannot fit returns a diagnostic without a silent no-op", () =>
  withTemporaryUserData((directory) => {
    const memory = rememberWorkspacePath(directory, `/repo/${"x".repeat(70_000)}`);

    assert.equal(memory.diagnostic?.code, "history_write_failed");
    assert.equal(existsSync(workspaceMemoryPath(directory)), false);
  }));

test("two process writers merge under the history lock without losing revisions", () =>
  withTemporaryUserData(async (directory) => {
    const moduleUrl = new URL("../electron/workspaceMemory.ts", import.meta.url).href;
    const script = `import { rememberWorkspacePath } from ${JSON.stringify(moduleUrl)};
const directory = process.argv[1];
const prefix = process.argv[2];
const startAt = Number(process.argv[3]);
while (Date.now() < startAt) {}
for (let index = 0; index < 40; index += 1) {
  rememberWorkspacePath(directory, \`/repo/\${prefix}-\${index}\`);
}`;
    const startAt = Date.now() + 250;
    const children = ["a", "b"].map((prefix) => spawn(process.execPath, [
      "--experimental-strip-types",
      "--input-type=module",
      "--eval",
      script,
      directory,
      prefix,
      String(startAt),
    ]));
    const codes = await Promise.all(children.map(async (child) => {
      const [code] = await once(child, "close");
      return code;
    }));

    assert.deepEqual(codes, [0, 0]);
    const memory = readWorkspaceMemory(directory);
    assert.equal(memory.revision, 80);
    assert.equal(memory.recentWorkspacePaths.length, 20);
    assert.equal(new Set(memory.recentWorkspacePaths).size, 20);
    assert.equal(memory.diagnostic, null);
  }));

test("two process writers pinning under the history lock without losing revisions", () =>
  withTemporaryUserData(async (directory) => {
    const moduleUrl = new URL("../electron/workspaceMemory.ts", import.meta.url).href;
    const script = `import { pinWorkspacePath } from ${JSON.stringify(moduleUrl)};
const directory = process.argv[1];
const prefix = process.argv[2];
const startAt = Number(process.argv[3]);
while (Date.now() < startAt) {}
for (let index = 0; index < 10; index += 1) {
  pinWorkspacePath(directory, \`/repo/\${prefix}-\${index}\`);
}`;
    const startAt = Date.now() + 250;
    const children = ["a", "b"].map((prefix) => spawn(process.execPath, [
      "--experimental-strip-types",
      "--input-type=module",
      "--eval",
      script,
      directory,
      prefix,
      String(startAt),
    ]));
    const codes = await Promise.all(children.map(async (child) => {
      const [code] = await once(child, "close");
      return code;
    }));

    assert.deepEqual(codes, [0, 0]);
    const memory = readWorkspaceMemory(directory);
    assert.equal(memory.revision, 20);
    assert.equal(memory.pinnedWorkspacePaths.length, 20);
    assert.equal(new Set(memory.pinnedWorkspacePaths).size, 20);
    assert.equal(memory.diagnostic, null);
  }));
