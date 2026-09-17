import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  forgetWorkspacePath,
  canonicalWorkspacePath,
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

test("workspace history recovers an abandoned lock only after the explicit five-second threshold", () =>
  withTemporaryUserData((directory) => {
    const lockPath = join(directory, ".workspace-memory.lock");
    mkdirSync(lockPath);
    const staleAt = new Date(Date.now() - 5_001);
    utimesSync(lockPath, staleAt, staleAt);

    const memory = rememberWorkspacePath(directory, "/repo/recovered");

    assert.equal(memory.diagnostic, null);
    assert.equal(memory.lastWorkspacePath, "/repo/recovered");
  }));

test("workspace history does not recover a lock younger than five seconds", () =>
  withTemporaryUserData((directory) => {
    const lockPath = join(directory, ".workspace-memory.lock");
    mkdirSync(lockPath);
    const recentAt = new Date(Date.now() - 4_000);
    utimesSync(lockPath, recentAt, recentAt);

    const memory = rememberWorkspacePath(directory, "/repo/not-written");

    assert.equal(memory.diagnostic?.code, "history_lock_timeout");
    assert.equal(existsSync(workspaceMemoryPath(directory)), false);
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
      Buffer.byteLength(`${JSON.stringify(baseRecord(padding, 0), null, 2)}\n`, "utf8") <= 64 * 1024
      && Buffer.byteLength(`${JSON.stringify(baseRecord(padding, revision + 1), null, 2)}\n`, "utf8") <= 64 * 1024
    ) {
      padding += 1;
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
