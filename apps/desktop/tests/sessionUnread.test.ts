import test from "node:test";
import assert from "node:assert/strict";

import { EMPTY_UNREAD_STATE, clearUnread, trackUnread } from "../src/sessionUnread.ts";
import type { SessionInfo } from "../src/api.ts";

function session(id: string, overrides: Partial<SessionInfo> = {}): SessionInfo {
  return {
    id,
    label: id,
    status: "running",
    lifecycle: "alive",
    attention: "working",
    cwd: "/tmp",
    created_at: "2026-07-01T10:00:00Z",
    memoryState: "live",
    resumeStrategy: "none",
    ...overrides,
  };
}

test("first snapshot marks nothing unread", () => {
  const state = trackUnread(EMPTY_UNREAD_STATE, [session("a"), session("b")], null);
  assert.equal(state.unreadIds.size, 0);
});

for (const result of ["idle", "needs_you", "blocked", "failed", "capped"] as const) {
  test(`working to ${result} on an inactive live session marks it unread`, () => {
    const first = trackUnread(EMPTY_UNREAD_STATE, [session("a")], null);
    const next = trackUnread(first, [session("a", { attention: result })], null);
    assert.ok(next.unreadIds.has("a"));
  });
}

test("attention change on the active session never marks it unread", () => {
  const first = trackUnread(EMPTY_UNREAD_STATE, [session("a")], "a");
  const next = trackUnread(first, [session("a", { attention: "needs_you" })], "a");
  assert.equal(next.unreadIds.has("a"), false);
});

test("non-working to non-working changes do not mark an inactive session unread", () => {
  const first = trackUnread(
    EMPTY_UNREAD_STATE,
    [session("a", { attention: "idle" })],
    null,
  );
  const next = trackUnread(first, [session("a", { attention: "needs_you" })], null);
  assert.equal(next.unreadIds.has("a"), false);
});

test("working to dead does not mark a remembered session unread", () => {
  const first = trackUnread(EMPTY_UNREAD_STATE, [session("a")], null);
  const next = trackUnread(
    first,
    [session("a", { status: "ended", lifecycle: "dead", attention: undefined, memoryState: "remembered" })],
    null,
  );
  assert.equal(next.unreadIds.has("a"), false);
});

test("unread persists across polls until cleared", () => {
  const first = trackUnread(EMPTY_UNREAD_STATE, [session("a")], null);
  const changed = trackUnread(first, [session("a", { attention: "idle" })], null);
  const later = trackUnread(changed, [session("a", { attention: "idle" })], null);
  assert.ok(later.unreadIds.has("a"));
});

test("clearUnread removes the flag and it stays clear while the status holds", () => {
  const first = trackUnread(EMPTY_UNREAD_STATE, [session("a")], null);
  const changed = trackUnread(first, [session("a", { attention: "idle" })], null);
  const cleared = clearUnread(changed, "a");
  assert.equal(cleared.unreadIds.has("a"), false);
  const later = trackUnread(cleared, [session("a", { attention: "idle" })], null);
  assert.equal(later.unreadIds.has("a"), false);
});

test("selecting a session clears it even while its status keeps changing", () => {
  const first = trackUnread(EMPTY_UNREAD_STATE, [session("a")], null);
  const changed = trackUnread(first, [session("a", { attention: "idle" })], null);
  assert.ok(changed.unreadIds.has("a"));
  const selected = trackUnread(clearUnread(changed, "a"), [session("a", { attention: "needs_you" })], "a");
  assert.equal(selected.unreadIds.has("a"), false);
});

test("an unread latch survives an unexpected return to working while inactive", () => {
  const first = trackUnread(EMPTY_UNREAD_STATE, [session("a")], null);
  const result = trackUnread(first, [session("a", { attention: "idle" })], null);
  const workingAgain = trackUnread(result, [session("a")], null);
  assert.ok(workingAgain.unreadIds.has("a"));
});

test("a session appearing mid-run starts read, not unread", () => {
  const first = trackUnread(EMPTY_UNREAD_STATE, [session("a")], null);
  const next = trackUnread(first, [session("a"), session("b")], null);
  assert.equal(next.unreadIds.has("b"), false);
});

test("sessions that disappear are dropped from the state", () => {
  const first = trackUnread(EMPTY_UNREAD_STATE, [session("a")], null);
  const changed = trackUnread(first, [session("a", { attention: "idle" })], null);
  const next = trackUnread(changed, [], null);
  assert.equal(next.unreadIds.size, 0);
  assert.equal(next.signatures.size, 0);
});

test("raw activity ticks without an attention change do not mark unread", () => {
  const first = trackUnread(EMPTY_UNREAD_STATE, [session("a", { lastActivityAt: "2026-07-01T10:00:00Z" })], null);
  const next = trackUnread(first, [session("a", { lastActivityAt: "2026-07-01T10:00:02Z" })], null);
  assert.equal(next.unreadIds.has("a"), false);
});

test("an unchanged snapshot returns the same state object so React can bail out", () => {
  const first = trackUnread(EMPTY_UNREAD_STATE, [session("a"), session("b")], null);
  // Fresh SessionInfo objects, same content — mirrors the 2s poll.
  const next = trackUnread(first, [session("a"), session("b")], null);
  assert.equal(next, first);
});

test("hitting the usage cap counts as an attention change", () => {
  const first = trackUnread(EMPTY_UNREAD_STATE, [session("a")], null);
  const next = trackUnread(first, [session("a", { attention: "capped" })], null);
  assert.ok(next.unreadIds.has("a"));
});
