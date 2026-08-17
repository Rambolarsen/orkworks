import test from "node:test";
import assert from "node:assert/strict";

import { resolvePendingCreates, trackPendingCreate } from "../src/pendingCreate.ts";
import type { SessionInfo } from "../src/api.ts";

function session(id: string, status: string): SessionInfo {
  return {
    id,
    label: id,
    status,
    cwd: "/tmp",
    created_at: "2026-08-17T07:00:00Z",
  };
}

test("trackPendingCreate adds an id without mutating the previous set", () => {
  const prev: ReadonlySet<string> = new Set();
  const next = trackPendingCreate(prev, "a");
  assert.equal(prev.size, 0);
  assert.ok(next.has("a"));
});

test("a tracked id still creating stays tracked and produces no toast", () => {
  const pending = trackPendingCreate(new Set(), "a");
  const result = resolvePendingCreates(pending, [session("a", "creating")]);
  assert.ok(result.ids.has("a"));
  assert.deepEqual(result.erroredIds, []);
});

test("a tracked id that resolves to running drops silently", () => {
  const pending = trackPendingCreate(new Set(), "a");
  const result = resolvePendingCreates(pending, [session("a", "running")]);
  assert.equal(result.ids.has("a"), false);
  assert.deepEqual(result.erroredIds, []);
});

test("a tracked id that resolves to error drops and is reported", () => {
  const pending = trackPendingCreate(new Set(), "a");
  const result = resolvePendingCreates(pending, [session("a", "error")]);
  assert.equal(result.ids.has("a"), false);
  assert.deepEqual(result.erroredIds, ["a"]);
});

test("a tracked id missing from the poll (deleted mid-create) drops without a toast", () => {
  const pending = trackPendingCreate(new Set(), "a");
  const result = resolvePendingCreates(pending, []);
  assert.equal(result.ids.has("a"), false);
  assert.deepEqual(result.erroredIds, []);
});

test("an unrelated session's error never reports a toast for an untracked id", () => {
  const pending = trackPendingCreate(new Set(), "a");
  const result = resolvePendingCreates(pending, [
    session("a", "creating"),
    session("unrelated", "error"),
  ]);
  assert.ok(result.ids.has("a"));
  assert.deepEqual(result.erroredIds, []);
});

test("multiple tracked ids resolve independently in the same poll", () => {
  let pending = trackPendingCreate(new Set(), "a");
  pending = trackPendingCreate(pending, "b");
  pending = trackPendingCreate(pending, "c");
  const result = resolvePendingCreates(pending, [
    session("a", "running"),
    session("b", "error"),
    session("c", "creating"),
  ]);
  assert.equal(result.ids.size, 1);
  assert.ok(result.ids.has("c"));
  assert.deepEqual(result.erroredIds, ["b"]);
});

test("an empty pending set is a no-op, returning the same set instance", () => {
  const prev: ReadonlySet<string> = new Set();
  const result = resolvePendingCreates(prev, [session("a", "error")]);
  assert.equal(result.ids, prev);
  assert.deepEqual(result.erroredIds, []);
});
