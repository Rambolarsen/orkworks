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

const splitSnapshot = () => ({ ...snapshot(), capability: "status-enrichment-v1", sessions: [
  { ...row("a"), branch: "main", enrichmentRevision: "r1", enrichment: { state: "ready", checkedAt: 1000 } },
] });
test("negotiated status preserves enrichment and enrichment cannot roll back newer status", () => {
  const state = createStreamState("/workspace"); state.apply(splitSnapshot());
  const { cwd: _, ...status } = row("a", "new status");
  const next = state.apply({ ...header, type: "session-status-delta", base: 0, sequence: 1,
    changed: [{ ...status, enrichmentRevision: "r1" }], removed: [] });
  assert.equal(next.sessions?.[0].branch, "main");
  const enriched = state.apply({ ...header, type: "session-enrichment-delta", base: 1, sequence: 2,
    changed: [{ id: "a", revision: "r1", data: { cwd: "/new", branch: "feature" }, state: "ready", checkedAt: 2000 }] });
  assert.equal(enriched.sessions?.[0].label, "new status");
  assert.equal(enriched.sessions?.[0].branch, "feature");
});
test("negotiated status clears enrichment on replacement and rejects late results atomically", () => {
  const state = createStreamState("/workspace"); state.apply(splitSnapshot());
  const { cwd: _, ...status } = row("a");
  const next = state.apply({ ...header, type: "session-status-delta", base: 0, sequence: 1,
    changed: [{ ...status, enrichmentRevision: "r2" }], removed: [] });
  assert.equal(next.sessions?.[0].branch, undefined);
  assert.throws(() => state.apply({ ...header, type: "session-enrichment-delta", base: 1, sequence: 2,
    changed: [{ id: "a", revision: "r1", data: { cwd: "/stale" }, state: "ready", checkedAt: 2000 }] }));
  assert.throws(() => state.apply({ ...header, type: "session-status-delta", base: 1, sequence: 2,
    changed: [{ ...status, branch: "injected", enrichmentRevision: "r2" }], removed: [] }));
});
test("unnegotiated clients reject status messages and freshness expires without network traffic", () => {
  const legacy = createStreamState("/workspace"); legacy.apply(snapshot());
  assert.throws(() => legacy.apply({ ...header, type: "session-status-delta", base: 0, sequence: 1, changed: [], removed: [] }));
  const split = createStreamState("/workspace"); split.apply(splitSnapshot());
  assert.equal(split.expire(5999).sessions, undefined);
  assert.equal(split.expire(6000).sessions?.[0].enrichment?.state, "stale");
});

test("status replacement clears optional fields and enrichment clears old document values", () => {
  const state = createStreamState("/workspace");
  const initial = splitSnapshot();
  state.apply({ ...initial, sessions: [{ ...initial.sessions[0], summary: "old", hasOpenablePlan: true }] });
  const { cwd: _, ...status } = row("a");
  const result = state.apply({ ...header, type: "session-status-delta", base: 0, sequence: 1,
    changed: [{ ...status, enrichmentRevision: "r1" }], removed: [] });
  assert.equal(result.sessions?.[0].summary, undefined);
  assert.equal(result.sessions?.[0].hasOpenablePlan, true);
  const enriched = state.apply({ ...header, type: "session-enrichment-delta", base: 1, sequence: 2,
    changed: [{ id: "a", revision: "r1", data: { cwd: "/tmp" }, state: "ready", checkedAt: 2000 }] });
  assert.equal(enriched.sessions?.[0].hasOpenablePlan, undefined);
  assert.equal(enriched.sessions?.[0].branch, undefined);
});
test("an invalid enrichment batch cannot partly update valid rows or advance its cursor", () => {
  const state = createStreamState("/workspace"); state.apply(splitSnapshot());
  const good = { id: "a", revision: "r1", data: { cwd: "/valid", branch: "new" }, state: "ready", checkedAt: 2000 };
  assert.throws(() => state.apply({ ...header, type: "session-enrichment-delta", base: 0, sequence: 1,
    changed: [good, { ...good, id: "missing" }] }));
  const result = state.apply({ ...header, type: "session-enrichment-delta", base: 0, sequence: 1, changed: [good] });
  assert.equal(result.sessions?.[0].branch, "new");
});
