import assert from "node:assert/strict";
import { test } from "node:test";
import type { SessionInfo } from "../src/api.ts";
import {
  autoArchiveWarningKey,
  collectNewAutoArchiveWarnings,
  formatAutoArchiveRemaining,
  nextAutoArchiveCountdownRefreshMs,
} from "../src/autoArchiveWarning.ts";

function activeSession(
  id: string,
  overrides: Partial<SessionInfo> = {},
): SessionInfo {
  return {
    id,
    label: id,
    harness: "codex",
    status: "running",
    lifecycle: "alive",
    lifecyclePhase: "active",
    sessionState: "active",
    cwd: "/tmp/ws",
    created_at: "2026-11-20T00:00:00Z",
    ...overrides,
  };
}

test("formatAutoArchiveRemaining formats hours, minutes, sub-minute, and invalid timestamps", () => {
  const now = new Date("2026-11-26T12:00:00Z");
  assert.equal(formatAutoArchiveRemaining("2026-11-27T12:00:00Z", now), "24h");
  assert.equal(formatAutoArchiveRemaining("2026-11-26T14:00:00Z", now), "2h");
  assert.equal(formatAutoArchiveRemaining("2026-11-26T12:45:00Z", now), "45m");
  assert.equal(formatAutoArchiveRemaining("2026-11-26T12:00:30Z", now), "<1m");
  assert.equal(formatAutoArchiveRemaining("2026-11-26T11:55:00Z", now), "<1m");
  assert.equal(formatAutoArchiveRemaining("not-a-date", now), "soon");
});

test("nextAutoArchiveCountdownRefreshMs aligns to hour and minute boundaries", () => {
  const now = new Date("2026-11-26T12:00:00Z");
  assert.equal(nextAutoArchiveCountdownRefreshMs("2026-11-26T14:15:00Z", now), 15 * 60_000);
  assert.equal(nextAutoArchiveCountdownRefreshMs("2026-11-26T14:00:00Z", now), 60 * 60_000);
  assert.equal(nextAutoArchiveCountdownRefreshMs("2026-11-26T12:10:20Z", now), 20_000);
  assert.equal(nextAutoArchiveCountdownRefreshMs("2026-11-26T12:00:15Z", now), 15_000);
  assert.equal(nextAutoArchiveCountdownRefreshMs("2026-11-26T11:59:00Z", now), null);
  assert.equal(nextAutoArchiveCountdownRefreshMs(null, now), null);
});

test("collectNewAutoArchiveWarnings deduplicates per (scope, sessionId, warnedAt) cycle", () => {
  const s1 = activeSession("s1", {
    autoArchiveWarning: {
      warnedAt: "2026-11-26T00:00:00Z",
      scheduledAt: "2026-11-27T00:00:00Z",
    },
  });
  const favorited = activeSession("fav", {
    favorited: true,
    autoArchiveWarning: {
      warnedAt: "2026-11-26T00:00:00Z",
      scheduledAt: "2026-11-27T00:00:00Z",
    },
  });
  const closed = activeSession("closed", {
    sessionState: "closed",
    lifecycle: "dead",
    autoArchiveWarning: {
      warnedAt: "2026-11-26T00:00:00Z",
      scheduledAt: "2026-11-27T00:00:00Z",
    },
  });

  // First poll emits s1 once, skipping favorited and non-active rows.
  const first = collectNewAutoArchiveWarnings(null, "ws-1", [s1, favorited, closed]);
  assert.deepEqual(first.toNotify.map((s) => s.id), ["s1"]);
  assert.ok(first.state?.emittedKeys.has(autoArchiveWarningKey(s1)!));

  // Repeated 5-minute polls for the same cycle must not re-notify.
  const repeat = collectNewAutoArchiveWarnings(first.state, "ws-1", [s1]);
  assert.deepEqual(repeat.toNotify, []);

  // A renewed deadline keeping the same warnedAt must not re-notify.
  const renewed = activeSession("s1", {
    autoArchiveWarning: {
      warnedAt: "2026-11-26T00:00:00Z",
      scheduledAt: "2026-11-28T00:00:00Z",
    },
  });
  const afterRenew = collectNewAutoArchiveWarnings(repeat.state, "ws-1", [renewed]);
  assert.deepEqual(afterRenew.toNotify, []);

  // A brand-new warning cycle (new warnedAt) emits once.
  const nextCycle = activeSession("s1", {
    autoArchiveWarning: {
      warnedAt: "2026-12-03T00:00:00Z",
      scheduledAt: "2026-12-04T00:00:00Z",
    },
  });
  const afterReset = collectNewAutoArchiveWarnings(afterRenew.state, "ws-1", [nextCycle]);
  assert.deepEqual(afterReset.toNotify.map((s) => s.id), ["s1"]);
  assert.deepEqual(collectNewAutoArchiveWarnings(afterReset.state, "ws-1", [nextCycle]).toNotify, []);

  // Null scope clears tracking cleanly.
  assert.deepEqual(collectNewAutoArchiveWarnings(afterReset.state, null, [nextCycle]), {
    state: null,
    toNotify: [],
  });
});
