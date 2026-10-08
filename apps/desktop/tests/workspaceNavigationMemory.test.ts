import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { createWorkspaceNavigationMemory, forgetRememberedWorkspaceWithNavigation, workspaceNavigationMemoryPath } from "../electron/workspaceNavigationMemory.ts";
import { readWorkspaceMemory, rememberWorkspacePath, workspaceMemoryPath } from "../electron/workspaceMemory.ts";

function fixture() {
  const directory = mkdtempSync(join(tmpdir(), "ork-navigation-"));
  let generation = 1;
  let ready = true;
  const current = () => ({ workspacePath: "/canonical/current", generation, ready });
  return { directory, current, switchGeneration: () => { generation += 1; }, setReady: (value: boolean) => { ready = value; }, close: () => rmSync(directory, { recursive: true, force: true }) };
}

test("successful navigation orders entries and evicts only least recent entries", async () => {
  const f = fixture();
  try {
    const memory = createWorkspaceNavigationMemory(f.directory);
    memory.read();
    for (let index = 0; index < 21; index += 1) {
      assert.equal((await memory.complete(`/canonical/${index}`, 1, () => true, "terminal")).ok, true);
    }
    const snapshot = memory.read();
    assert.equal(snapshot.entries.length, 20);
    assert.equal(snapshot.entries[0].workspaceIdentity, "/canonical/20");
    assert.equal(snapshot.entries.some((entry) => entry.workspaceIdentity === "/canonical/0"), false);
    const before = readFileSync(workspaceNavigationMemoryPath(f.directory), "utf8");
    memory.read();
    assert.equal(readFileSync(workspaceNavigationMemoryPath(f.directory), "utf8"), before);
    assert.equal((await memory.complete("/canonical/1", 1, () => true, "review")).ok, true);
    assert.deepEqual(memory.read().entries[0], { workspaceIdentity: "/canonical/1", lastCentralSurface: "review" });
  } finally { f.close(); }
});

test("two rapid local writes advance only through verified local commits", async () => {
  const f = fixture();
  try {
    const memory = createWorkspaceNavigationMemory(f.directory);
    memory.read();
    const first = memory.complete("/canonical/a", 1, () => true, "terminal");
    const second = memory.complete("/canonical/b", 1, () => true, "review");
    assert.equal((await first).ok, true);
    assert.equal((await second).ok, true);
    assert.deepEqual(memory.read().entries.map((entry) => entry.workspaceIdentity), ["/canonical/b", "/canonical/a"]);
  } finally { f.close(); }
});

test("cross-instance intervening write rejects stale intent without replay", async () => {
  const f = fixture();
  try {
    const first = createWorkspaceNavigationMemory(f.directory);
    const second = createWorkspaceNavigationMemory(f.directory);
    first.read(); second.read();
    assert.equal((await second.complete("/canonical/b", 1, () => true, "terminal")).ok, true);
    assert.equal((await first.complete("/canonical/a", 1, () => true, "review")).ok, false);
    assert.deepEqual(second.read().entries.map((entry) => entry.workspaceIdentity), ["/canonical/b"]);
  } finally { f.close(); }
});

test("queued navigation is dropped when workspace generation becomes stale", async () => {
  const f = fixture();
  try {
    const memory = createWorkspaceNavigationMemory(f.directory);
    memory.read();
    const pending = memory.complete("/canonical/current", 1, () => f.current().generation === 1 && f.current().ready, "review");
    f.switchGeneration();
    assert.equal((await pending).ok, false);
    assert.deepEqual(memory.read().entries, []);
  } finally { f.close(); }
});

test("deletion barrier cancels pre-barrier saves and preserves a revisioned container", async () => {
  const f = fixture();
  try {
    const memory = createWorkspaceNavigationMemory(f.directory);
    memory.read();
    const save = memory.complete("/canonical/a", 1, () => true, "terminal");
    const deletion = memory.delete("/canonical/a", 1, () => true);
    assert.equal((await save).ok, false);
    assert.equal((await deletion).ok, true);
    assert.deepEqual(memory.read().entries, []);
    assert.ok(memory.read().revision > 0);
  } finally { f.close(); }
});

test("protected oversize entry rejects save without deleting prior bytes or entries", async () => {
  const f = fixture();
  try {
    const memory = createWorkspaceNavigationMemory(f.directory);
    memory.read();
    await memory.complete("/canonical/a", 1, () => true, "terminal");
    const source = readFileSync(workspaceNavigationMemoryPath(f.directory));
    assert.equal((await memory.complete(`/canonical/${"x".repeat(70_000)}`, 1, () => true, "review")).ok, false);
    assert.deepEqual(readFileSync(workspaceNavigationMemoryPath(f.directory)), source);
    assert.deepEqual(memory.read().entries, [{ workspaceIdentity: "/canonical/a", lastCentralSurface: "terminal" }]);
  } finally { f.close(); }
});

test("corrupt navigation bytes remain until separately confirmed rebuild", async () => {
  const f = fixture();
  try {
    const path = workspaceNavigationMemoryPath(f.directory);
    writeFileSync(path, "{corrupt");
    const memory = createWorkspaceNavigationMemory(f.directory);
    assert.equal(memory.read().diagnostic, "corrupt_record");
    assert.equal((await memory.complete("/canonical/a", 1, () => true, "terminal")).ok, false);
    assert.equal(readFileSync(path, "utf8"), "{corrupt");
    assert.equal((await memory.rebuild(true)).ok, true);
    assert.deepEqual(memory.read().entries, []);
  } finally { f.close(); }
});

test("failed save invalidates a queued successor without changing prior bytes", async () => {
  const f = fixture();
  try {
    let replacements = 0;
    const memory = createWorkspaceNavigationMemory(f.directory, (temporary, target) => {
      replacements += 1;
      if (replacements === 2) throw new Error("injected failure");
      writeFileSync(target, readFileSync(temporary));
    });
    memory.read();
    const source = readFileSync(workspaceNavigationMemoryPath(f.directory));
    const first = memory.complete("/canonical/a", 1, () => true, "terminal");
    const second = memory.complete("/canonical/b", 1, () => true, "review");
    assert.equal((await first).ok, false);
    assert.equal((await second).ok, false);
    assert.deepEqual(readFileSync(workspaceNavigationMemoryPath(f.directory)), source);
  } finally { f.close(); }
});

test("read-back mismatch restores prior bytes and invalidates a queued successor", async () => {
  const f = fixture();
  try {
    let replacements = 0;
    const memory = createWorkspaceNavigationMemory(f.directory, (temporary, target) => {
      replacements += 1;
      writeFileSync(target, replacements === 2 ? "{mismatch" : readFileSync(temporary));
    });
    memory.read();
    const source = readFileSync(workspaceNavigationMemoryPath(f.directory));
    const first = memory.complete("/canonical/a", 1, () => true, "terminal");
    const second = memory.complete("/canonical/b", 1, () => true, "review");
    assert.equal((await first).ok, false);
    assert.equal((await second).ok, false);
    assert.deepEqual(readFileSync(workspaceNavigationMemoryPath(f.directory)), source);
    assert.deepEqual(memory.read().entries, []);
  } finally { f.close(); }
});

test("failed mismatch rollback reports unconfirmed preservation", async () => {
  const f = fixture();
  try {
    let replacements = 0;
    const memory = createWorkspaceNavigationMemory(f.directory, (temporary, target) => {
      replacements += 1;
      writeFileSync(target, replacements === 2 ? "{mismatch" : readFileSync(temporary));
    }, () => { throw new Error("injected rollback failure"); });
    memory.read();
    const result = await memory.complete("/canonical/a", 1, () => true, "terminal");
    assert.deepEqual(result, { ok: false, diagnostic: "restore_failed" });
    assert.equal(readFileSync(workspaceNavigationMemoryPath(f.directory), "utf8"), "{mismatch");
  } finally { f.close(); }
});

test("mismatch recovery does not overwrite a newer future-version record", async () => {
  const f = fixture();
  try {
    let replacements = 0;
    const future = JSON.stringify({ version: 2, epoch: "f".repeat(32), revision: 99, payload: { entries: [] } });
    const memory = createWorkspaceNavigationMemory(f.directory, (temporary, target) => {
      replacements += 1;
      writeFileSync(target, replacements === 2 ? future : readFileSync(temporary));
    });
    memory.read();
    assert.equal((await memory.complete("/canonical/a", 1, () => true, "review")).ok, false);
    assert.equal(readFileSync(workspaceNavigationMemoryPath(f.directory), "utf8"), future);
  } finally { f.close(); }
});

test("two instances share the first creation epoch instead of reinitializing", () => {
  const f = fixture();
  try {
    const first = createWorkspaceNavigationMemory(f.directory);
    const second = createWorkspaceNavigationMemory(f.directory);
    first.read();
    const source = readFileSync(workspaceNavigationMemoryPath(f.directory), "utf8");
    second.read();
    assert.equal(readFileSync(workspaceNavigationMemoryPath(f.directory), "utf8"), source);
  } finally { f.close(); }
});

test("external reconstruction changes epoch and rejects an older queued intent", async () => {
  const f = fixture();
  try {
    const first = createWorkspaceNavigationMemory(f.directory);
    const second = createWorkspaceNavigationMemory(f.directory);
    first.read();
    const pending = first.complete("/canonical/a", 1, () => true, "review");
    assert.equal((await second.rebuild(true)).ok, true);
    assert.equal((await pending).ok, false);
    assert.deepEqual(first.read().entries, []);
  } finally { f.close(); }
});

test("two processes racing first use converge on one initialized record", async () => {
  const f = fixture();
  try {
    const moduleUrl = new URL("../electron/workspaceNavigationMemory.ts", import.meta.url).href;
    const script = `import { createWorkspaceNavigationMemory } from ${JSON.stringify(moduleUrl)};
const result = createWorkspaceNavigationMemory(process.argv[1]).read();
if (result.diagnostic || result.revision !== 0) process.exitCode = 1;`;
    const children = [0, 1].map(() => spawn(process.execPath, ["--experimental-strip-types", "--input-type=module", "--eval", script, f.directory]));
    const codes = await Promise.all(children.map(async (child) => (await once(child, "close"))[0]));
    assert.deepEqual(codes, [0, 0]);
    const record = JSON.parse(readFileSync(workspaceNavigationMemoryPath(f.directory), "utf8"));
    assert.equal(record.revision, 0);
    assert.match(record.epoch, /^[0-9a-f]{32}$/);
  } finally { f.close(); }
});

test("byte-budget eviction keeps the written entry and removes older entries", async () => {
  const f = fixture();
  try {
    const memory = createWorkspaceNavigationMemory(f.directory);
    memory.read();
    const older = `/canonical/${"a".repeat(38_000)}`;
    const newer = `/canonical/${"b".repeat(38_000)}`;
    assert.equal((await memory.complete(older, 1, () => true, "terminal")).ok, true);
    assert.equal((await memory.complete(newer, 1, () => true, "review")).ok, true);
    assert.deepEqual(memory.read().entries, [{ workspaceIdentity: newer, lastCentralSurface: "review" }]);
    assert.ok(readFileSync(workspaceNavigationMemoryPath(f.directory)).byteLength <= 64 * 1024);
  } finally { f.close(); }
});

test("another instance's deletion creates a revision gap for an older intent", async () => {
  const f = fixture();
  try {
    const first = createWorkspaceNavigationMemory(f.directory);
    const second = createWorkspaceNavigationMemory(f.directory);
    first.read(); second.read();
    const deletion = second.delete("/canonical/a", 1, () => true);
    const stale = first.complete("/canonical/a", 1, () => true, "terminal");
    assert.equal((await deletion).ok, true);
    assert.equal((await stale).ok, false);
    assert.deepEqual(first.read().entries, []);
  } finally { f.close(); }
});

test("revision overflow preserves the record", async () => {
  const f = fixture();
  try {
    const path = workspaceNavigationMemoryPath(f.directory);
    const memory = createWorkspaceNavigationMemory(f.directory);
    memory.read();
    const record = JSON.parse(readFileSync(path, "utf8"));
    record.revision = Number.MAX_SAFE_INTEGER;
    const source = JSON.stringify(record);
    writeFileSync(path, source);
    memory.read();
    assert.equal((await memory.complete("/canonical/a", 1, () => true, "terminal")).ok, false);
    assert.equal(readFileSync(path, "utf8"), source);
  } finally { f.close(); }
});

test("forgetting a non-current canonical workspace removes only its navigation entry", async () => {
  const f = fixture();
  try {
    const older = join(f.directory, "older");
    const current = join(f.directory, "current");
    mkdirSync(older); mkdirSync(current);
    const olderIdentity = realpathSync.native(older);
    const currentIdentity = realpathSync.native(current);
    rememberWorkspacePath(f.directory, olderIdentity);
    rememberWorkspacePath(f.directory, currentIdentity);
    const navigation = createWorkspaceNavigationMemory(f.directory);
    navigation.read();
    await navigation.complete(olderIdentity, 1, () => true, "review");
    await navigation.complete(currentIdentity, 1, () => true, "terminal");

    const result = await forgetRememberedWorkspaceWithNavigation(f.directory, olderIdentity, navigation);

    assert.equal(result.history.diagnostic, null);
    assert.equal(result.navigation?.ok, true);
    assert.deepEqual(navigation.read().entries, [{ workspaceIdentity: currentIdentity, lastCentralSurface: "terminal" }]);
    assert.deepEqual(readWorkspaceMemory(f.directory).recentWorkspacePaths, [currentIdentity]);
  } finally { f.close(); }
});

test("unrecognized, alias, and failed history forgets preserve navigation entries", async () => {
  const f = fixture();
  try {
    const workspace = join(f.directory, "workspace");
    const alias = join(f.directory, "alias");
    mkdirSync(workspace); symlinkSync(workspace, alias, "dir");
    const identity = realpathSync.native(workspace);
    rememberWorkspacePath(f.directory, identity);
    const navigation = createWorkspaceNavigationMemory(f.directory);
    navigation.read();
    await navigation.complete(identity, 1, () => true, "review");
    const source = readFileSync(workspaceNavigationMemoryPath(f.directory));

    assert.equal((await forgetRememberedWorkspaceWithNavigation(f.directory, alias, navigation)).navigation, null);
    assert.equal((await forgetRememberedWorkspaceWithNavigation(f.directory, join(f.directory, "unknown"), navigation)).navigation, null);
    assert.deepEqual(readFileSync(workspaceNavigationMemoryPath(f.directory)), source);
    writeFileSync(workspaceMemoryPath(f.directory), "{corrupt");
    assert.equal((await forgetRememberedWorkspaceWithNavigation(f.directory, identity, navigation)).history.diagnostic?.code, "corrupt_history");
    assert.deepEqual(readFileSync(workspaceNavigationMemoryPath(f.directory)), source);
  } finally { f.close(); }
});

test("missing but retained canonical workspace identity can be pruned after invalid destination", async () => {
  const f = fixture();
  try {
    const workspace = join(f.directory, "removed");
    mkdirSync(workspace);
    const identity = realpathSync.native(workspace);
    rememberWorkspacePath(f.directory, identity);
    const navigation = createWorkspaceNavigationMemory(f.directory);
    navigation.read();
    await navigation.complete(identity, 1, () => true, "review");
    rmSync(workspace, { recursive: true });
    const result = await forgetRememberedWorkspaceWithNavigation(f.directory, identity, navigation);
    assert.equal(result.navigation?.ok, true);
    assert.deepEqual(navigation.read().entries, []);
  } finally { f.close(); }
});
