import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { confirmShellMemoryRebuild } from "../electron/shellMemoryRebuild.ts";
import { createShellLayoutMemory, shellLayoutMemoryPath } from "../electron/shellLayoutMemory.ts";
import { createWorkspaceNavigationMemory, workspaceNavigationMemoryPath } from "../electron/workspaceNavigationMemory.ts";

test("cancelled native confirmation skips rebuild for each shell record", async () => {
  for (const kind of ["layout", "navigation"] as const) {
    const window = { isDestroyed: () => false };
    let rebuilt = 0;
    const dialogs: Array<{ title: string; type: string; buttons: string[]; defaultId: number; cancelId: number }> = [];
    const result = await confirmShellMemoryRebuild(kind, window, async (owner, options) => {
      assert.equal(owner, window);
      dialogs.push(options);
      return { response: 0 };
    }, async () => { rebuilt += 1; return { ok: true }; });
    assert.deepEqual(result, { ok: false, diagnostic: "user_cancelled" });
    assert.equal(rebuilt, 0);
    assert.equal(dialogs.length, 1);
    assert.equal(dialogs[0]?.type, "warning");
    assert.deepEqual(dialogs[0]?.buttons, ["Cancel", "Rebuild"]);
    assert.equal(dialogs[0]?.defaultId, 0);
    assert.equal(dialogs[0]?.cancelId, 0);
    assert.match(dialogs[0]?.title ?? "", kind === "layout" ? /shell layout/i : /workspace navigation/i);
  }
});

test("cancelling a rebuild preserves the existing bytes of both records", async () => {
  const directory = mkdtempSync(join(tmpdir(), "ork-shell-rebuild-"));
  try {
    const layoutPath = shellLayoutMemoryPath(directory);
    const navigationPath = workspaceNavigationMemoryPath(directory);
    writeFileSync(layoutPath, "{old layout bytes");
    writeFileSync(navigationPath, "{old navigation bytes");
    const cases = [
      { kind: "layout" as const, path: layoutPath, rebuild: () => createShellLayoutMemory(directory).rebuild(true) },
      { kind: "navigation" as const, path: navigationPath, rebuild: () => createWorkspaceNavigationMemory(directory).rebuild(true) },
    ];
    for (const entry of cases) {
      const previous = readFileSync(entry.path);
      assert.deepEqual(await confirmShellMemoryRebuild(entry.kind, { isDestroyed: () => false },
        async () => ({ response: 0 }), entry.rebuild), { ok: false, diagnostic: "user_cancelled" });
      assert.deepEqual(readFileSync(entry.path), previous);
    }
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("accepted native confirmation invokes the selected rebuild once", async () => {
  for (const kind of ["layout", "navigation"] as const) {
    const window = { isDestroyed: () => false };
    let dialogs = 0;
    let rebuilt = 0;
    const result = await confirmShellMemoryRebuild(kind, window, async () => {
      dialogs += 1;
      return { response: 1 };
    }, async () => { rebuilt += 1; return { ok: true }; });
    assert.deepEqual(result, { ok: true });
    assert.equal(dialogs, 1);
    assert.equal(rebuilt, 1);
  }
});

test("missing or destroyed owner window fails closed, including during confirmation", async () => {
  for (const kind of ["layout", "navigation"] as const) {
    let rebuilt = 0;
    let dialogs = 0;
    const confirm = async () => { dialogs += 1; return { response: 1 }; };
    const rebuild = async () => { rebuilt += 1; return { ok: true } as const; };
    assert.deepEqual(await confirmShellMemoryRebuild(kind, null, confirm, rebuild),
      { ok: false, diagnostic: "user_cancelled" });
    assert.deepEqual(await confirmShellMemoryRebuild(kind, { isDestroyed: () => true }, confirm, rebuild),
      { ok: false, diagnostic: "user_cancelled" });
    let destroyed = false;
    const window = { isDestroyed: () => destroyed };
    assert.deepEqual(await confirmShellMemoryRebuild(kind, window, async () => {
      destroyed = true;
      return { response: 1 };
    }, rebuild), { ok: false, diagnostic: "user_cancelled" });
    assert.equal(dialogs, 0);
    assert.equal(rebuilt, 0);
  }
});
