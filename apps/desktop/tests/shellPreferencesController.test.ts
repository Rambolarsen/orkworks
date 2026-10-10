import test from "node:test";
import assert from "node:assert/strict";
import { createShellPreferencesController, DEFAULT_SHELL_PREFERENCES } from "../src/shellPreferencesController.ts";

function fixture() {
  let resolveRead!: (value: any) => void;
  const reads = new Promise<any>(resolve => { resolveRead = resolve; });
  const writes: any[] = [];
  const changes: any[] = [];
  let pending: (() => void) | null = null;
  const controller = createShellPreferencesController({
    read: () => reads,
    save: async value => { writes.push(value); return { ok: true }; },
    reset: async () => { writes.push("reset"); return { ok: true }; },
    onChange: value => changes.push(value),
    onError: () => {},
    schedule: callback => { pending = callback; return () => { pending = null; }; },
  });
  return { controller, writes, changes, resolveRead, flush: () => { const callback = pending; pending = null; callback?.(); } };
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
