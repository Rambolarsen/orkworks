import test from "node:test";
import assert from "node:assert/strict";
import { createStreamState, startWorkspaceUpdates } from "../src/workspaceUpdates.ts";

const row = (id: string, label = id) => ({ id, label, lifecycle: "alive", status: "running", cwd: "/tmp", created_at: "2026-10-24", memoryState: "live", resumeStrategy: "none" });
const header = { version: 1, instance: "sidecar", workspace: "/workspace", generation: 1 };
const snapshot = (sequence = 0) => ({ ...header, type: "snapshot", sequence, sessions: [row("a"), row("b")] });

test("snapshot and changed rows/removals share a cursor with recommendation invalidations", () => {
  const state = createStreamState("/workspace");
  assert.deepEqual(state.apply(snapshot()).sessions?.map(x => x.id), ["a", "b"]);
  assert.equal(state.apply({ ...header, type: "recommendations-invalidated", base: 0, sequence: 1 }).recommendationsInvalidated, true);
  const result = state.apply({ ...header, type: "sessions-delta", base: 1, sequence: 2, changed: [row("a", "new"), row("c")], removed: ["b"], order: ["c", "a"] });
  assert.deepEqual(result.sessions?.map(x => [x.id, x.label]), [["c", "c"], ["a", "new"]]);
});

test("gaps, wrong identity, malformed deltas and duplicate IDs never partially apply", () => {
  for (const mutation of [
    { base: 9 }, { instance: "old" }, { workspace: "/old" }, { generation: 2 },
    { changed: [{ id: "bad" }] }, { removed: ["missing"] }, { order: ["a", "a"] },
  ]) {
    const state = createStreamState("/workspace"); state.apply(snapshot());
    assert.throws(() => state.apply({ ...header, type: "sessions-delta", base: 0, sequence: 1, changed: [], removed: [], ...mutation }));
    const valid = state.apply({ ...header, type: "sessions-delta", base: 0, sequence: 1, changed: [row("a", "valid")], removed: [] });
    assert.equal(valid.sessions?.[0].label, "valid");
  }
  assert.throws(() => createStreamState("/workspace").apply({ ...snapshot(), sessions: [row("a"), row("a")] }));
});

test("delta before snapshot and unsupported version require resynchronization", () => {
  assert.throws(() => createStreamState("/workspace").apply({ ...header, type: "sessions-delta", base: 0, sequence: 1, changed: [], removed: [] }));
  assert.throws(() => createStreamState("/workspace").apply({ ...snapshot(), version: 2 }));
});

test("transport adopts only a valid snapshot and restores fallback on a gap; disposal fences old socket", async () => {
  const sockets: Array<{ onmessage: ((e: {data:string}) => void) | null; onclose: (() => void) | null; onerror: (()=>void)|null; close():void; send(s:string):void }> = [];
  const timers = new Map<unknown, () => void>(); let timerId = 0;
  const health: boolean[] = []; const lists: string[][] = [];
  const stream = startWorkspaceUpdates({
    workspacePath: "/workspace", getBackendUrl: async () => "http://localhost:1234",
    createSocket: () => { const socket = { onmessage: null, onclose: null, onerror: null, close() {}, send(_s:string) {} }; sockets.push(socket); return socket as unknown as WebSocket; },
    scheduler: { set: callback => { timers.set(++timerId, callback); return timerId; }, clear: handle => { timers.delete(handle); } },
    onHealthy: value => health.push(value), onSessions: rows => lists.push(rows.map(x => x.id)), onRecommendationsInvalidated: () => {},
  });
  await Promise.resolve(); await Promise.resolve();
  sockets[0].onmessage?.({ data: JSON.stringify(snapshot()) });
  assert.deepEqual(health, [false, true]); assert.deepEqual(lists, [["a", "b"]]);
  sockets[0].onmessage?.({ data: JSON.stringify({ ...header, type: "sessions-delta", base: 99, sequence: 100, changed: [], removed: [] }) });
  assert.equal(health.at(-1), false);
  stream.dispose(); const count = lists.length;
  sockets[0].onmessage?.({ data: JSON.stringify(snapshot()) });
  assert.equal(lists.length, count); assert.equal(timers.size, 0);
});
