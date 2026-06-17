import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  readWorkspaceMemory,
  writeWorkspaceMemory,
  rememberWorkspacePath,
} from "../electron/workspaceMemory.ts";

test("workspace memory round-trips last workspace and recent paths", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-memory-"));
  try {
    writeWorkspaceMemory(dir, {
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a"],
    });

    const memory = readWorkspaceMemory(dir);

    assert.equal(memory.lastWorkspacePath, "/repo/a");
    assert.deepEqual(memory.recentWorkspacePaths, ["/repo/a"]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("rememberWorkspacePath deduplicates and keeps newest first", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-memory-"));
  try {
    rememberWorkspacePath(dir, "/repo/a");
    rememberWorkspacePath(dir, "/repo/b");
    rememberWorkspacePath(dir, "/repo/a");

    const memory = readWorkspaceMemory(dir);

    assert.equal(memory.lastWorkspacePath, "/repo/a");
    assert.deepEqual(memory.recentWorkspacePaths, ["/repo/a", "/repo/b"]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
