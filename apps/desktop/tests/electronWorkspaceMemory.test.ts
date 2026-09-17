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
  workspaceMemoryPath,
} from "../electron/workspaceMemory.ts";

const EMPTY_MEMORY = {
  version: 1,
  revision: 0,
  lastWorkspacePath: null,
  recentWorkspacePaths: [],
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

    assert.equal(first.version, 1);
    assert.equal(first.revision, 1);
    assert.equal(second.revision, 2);
    assert.deepEqual(readWorkspaceMemory(directory), second);
    assert.deepEqual(JSON.parse(readFileSync(workspaceMemoryPath(directory), "utf8")), {
      version: 1,
      revision: 2,
      lastWorkspacePath: "/repo/b",
      recentWorkspacePaths: ["/repo/b", "/repo/a"],
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
      version: 1,
      revision: 2,
      lastWorkspacePath: "/repo/b",
      recentWorkspacePaths: ["/repo/b", "/repo/a"],
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

test("corrupt workspace history is diagnosed and preserved across mutation attempts", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const corrupt = "{ definitely not valid workspace history";
    writeFileSync(historyPath, corrupt);

    const loaded = readWorkspaceMemory(directory);
    const remembered = rememberWorkspacePath(directory, "/repo/a");
    const forgotten = forgetWorkspacePath(directory, "/repo/a");

    assert.deepEqual(loaded, {
      ...EMPTY_MEMORY,
      diagnostic: {
        code: "corrupt_history",
        message: "Workspace history is corrupt or unreadable and was left unchanged.",
      },
    });
    assert.deepEqual(remembered, loaded);
    assert.deepEqual(forgotten, loaded);
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
      version: 1,
      revision: 1,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a"],
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
        version: 1,
        revision: 2,
        lastWorkspacePath: "/repo/a",
        recentWorkspacePaths: Array.from({ length: 21 }, (_, index) => `/repo/${index}`),
      },
      {
        version: 1,
        revision: 3,
        lastWorkspacePath: "/repo/a",
        recentWorkspacePaths: ["/repo/a", "/repo/a"],
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

test("revision overflow rejects both mutations before writing and preserves history bytes", () =>
  withTemporaryUserData((directory) => {
    const original = JSON.stringify({
      version: 1,
      revision: Number.MAX_SAFE_INTEGER,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a"],
    });
    writeFileSync(workspaceMemoryPath(directory), original);
    for (const mutate of [rememberWorkspacePath, forgetWorkspacePath]) {
      const result = mutate(directory, "/repo/a");
      assert.equal(result.diagnostic?.code, "history_write_failed");
      assert.equal(result.revision, Number.MAX_SAFE_INTEGER);
      assert.equal(readFileSync(workspaceMemoryPath(directory), "utf8"), original);
      assert.equal(readWorkspaceMemory(directory).diagnostic, null);
    }
    assert.equal(forgetWorkspacePath(directory, "/unknown").diagnostic, null);
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
