import test from "node:test";
import assert from "node:assert/strict";
import { createShellPreferencesController, DEFAULT_SHELL_PREFERENCES } from "../src/shellPreferencesController.ts";

function fixture(options: { rebuildResult?: any; legacy?: string | null } = {}) {
  let resolveRead!: (value: any) => void;
  const reads = new Promise<any>(resolve => { resolveRead = resolve; });
  const writes: any[] = [];
  const changes: any[] = [];
  const diagnostics: any[] = [];
  let notices = 0;
  let pending: (() => void) | null = null;
  const controller = createShellPreferencesController({
    read: () => reads,
    save: async value => { writes.push(value); return { ok: true }; },
    reset: async () => { writes.push("reset"); return { ok: true }; },
    rebuild: async () => { writes.push("rebuild"); return options.rebuildResult ?? { ok: true }; },
    readLegacy: async () => options.legacy ?? null,
    onMigrationNotice: () => { notices++; },
    onDiagnostic: value => diagnostics.push(value),
    onChange: value => changes.push(value),
    onError: () => {},
    schedule: callback => { pending = callback; return () => { pending = null; }; },
  });
  return { controller, writes, changes, diagnostics, notices: () => notices, resolveRead, flush: () => { const callback = pending; pending = null; callback?.(); } };
}

test("hydration never writes and a delayed read cannot replace a user's change", async () => {
  const f = fixture();
  const ready = f.controller.load();
  f.controller.change({ ...DEFAULT_SHELL_PREFERENCES, sessionsVisible: false });
  f.resolveRead({ preferences: { ...DEFAULT_SHELL_PREFERENCES, sessionsWidth: 300 }, diagnostic: null });
  await ready;
  assert.equal(f.changes.at(-1).sessionsVisible, false);
  assert.deepEqual(f.writes, []);
  f.flush();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.writes.at(-1).sessionsVisible, false);
});

test("reset cancels a pending resize save and disposal prevents later writes", async () => {
  const f = fixture();
  const ready = f.controller.load();
  f.resolveRead({ preferences: DEFAULT_SHELL_PREFERENCES, diagnostic: null });
  await ready;
  f.controller.change({ ...DEFAULT_SHELL_PREFERENCES, sessionsWidth: 300 });
  await f.controller.reset();
  f.flush();
  assert.deepEqual(f.writes, ["reset"]);
  f.controller.change({ ...DEFAULT_SHELL_PREFERENCES, sessionsWidth: 280 });
  f.controller.dispose();
  f.flush();
  assert.deepEqual(f.writes, ["reset"]);
});

test("an unresolved load cannot publish after disposal", async () => {
  const f = fixture();
  const ready = f.controller.load();
  f.controller.dispose();
  f.resolveRead({ preferences: DEFAULT_SHELL_PREFERENCES, diagnostic: null });
  await ready;
  assert.deepEqual(f.changes, []);
  assert.deepEqual(f.writes, []);
});

test("reset stays an ordering barrier when a later resize arrives before hydration", async () => {
  const f = fixture();
  const ready = f.controller.load();
  const reset = f.controller.reset();
  f.controller.change({ ...DEFAULT_SHELL_PREFERENCES, sessionsWidth: 280 });
  f.resolveRead({ preferences: DEFAULT_SHELL_PREFERENCES, diagnostic: null });
  await ready;
  await reset;
  f.flush();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.writes[0], "reset");
  assert.equal(f.writes[1].sessionsWidth, 280);
});

for (const diagnostic of ["corrupt_record", "unsupported_version"]) test(`reset offers confirmed rebuild for ${diagnostic}`, async () => {
  const f = fixture();
  f.resolveRead({ preferences: DEFAULT_SHELL_PREFERENCES, diagnostic });
  await f.controller.load();
  assert.equal(f.diagnostics.at(-1), diagnostic);
  await f.controller.reset();
  assert.deepEqual(f.writes, ["rebuild"]);
  assert.equal(f.diagnostics.at(-1), null);
  f.controller.change({ ...DEFAULT_SHELL_PREFERENCES, sessionsWidth: 280 });
  f.flush();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.writes.at(-1).sessionsWidth, 280);
});

test("cancelled rebuild preserves the current view and recovery diagnostic", async () => {
  const f = fixture({ rebuildResult: { ok: false, diagnostic: "user_cancelled" } });
  f.resolveRead({ preferences: DEFAULT_SHELL_PREFERENCES, diagnostic: "corrupt_record" });
  await f.controller.load();
  f.controller.change({ ...DEFAULT_SHELL_PREFERENCES, sessionsWidth: 280 });
  await f.controller.reset();
  assert.equal(f.changes.at(-1).sessionsWidth, 280);
  assert.equal(f.diagnostics.at(-1), "corrupt_record");
  f.flush();
  assert.deepEqual(f.writes, ["rebuild"]);
});

test("first valid legacy-layout use announces retained arrangement without hydration writes", async () => {
  const f = fixture({ legacy: '{"legacy":true}' });
  f.resolveRead({ preferences: DEFAULT_SHELL_PREFERENCES, revision: 0, diagnostic: null });
  await f.controller.load();
  await f.controller.load();
  assert.equal(f.notices(), 1);
  assert.deepEqual(f.writes, []);
});

test("already-used shell preferences do not repeat the legacy notice", async () => {
  const f = fixture({ legacy: '{"legacy":true}' });
  f.resolveRead({ preferences: DEFAULT_SHELL_PREFERENCES, revision: 1, diagnostic: null });
  await f.controller.load();
  assert.equal(f.notices(), 0);
});
