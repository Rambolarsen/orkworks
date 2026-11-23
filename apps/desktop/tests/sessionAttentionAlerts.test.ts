import assert from "node:assert/strict";
import test from "node:test";
import type { SessionInfo } from "../src/api.ts";
import { observeSessionAttention, sessionAlertDelivery } from "../src/sessionAttentionAlerts.ts";

function session(id: string, attention: SessionInfo["attention"], lifecycle: SessionInfo["lifecycle"] = "alive"): SessionInfo {
  return { id, attention, lifecycle } as SessionInfo;
}

const working = session("agent", "working");
const waiting = session("agent", "needs_you");

test("a live session alerts once for each observed wait", () => {
  const baseline = observeSessionAttention(null, "workspace:1", [working]);
  const firstWait = observeSessionAttention(baseline.snapshot, "workspace:1", [waiting]);
  assert.deepEqual(firstWait.newWaits, [waiting]);

  const poll = observeSessionAttention(firstWait.snapshot, "workspace:1", [
    { ...waiting, label: "New summary", attentionUpdatedAt: "2026-11-23T06:00:00Z" },
  ]);
  assert.deepEqual(poll.newWaits, []);

  const resolved = observeSessionAttention(poll.snapshot, "workspace:1", [working]);
  const nextWait = observeSessionAttention(resolved.snapshot, "workspace:1", [waiting]);
  assert.deepEqual(nextWait.newWaits, [waiting]);
});

test("initial, switched, and recovered snapshots establish a quiet baseline", () => {
  const initial = observeSessionAttention(null, "workspace:1", [waiting]);
  assert.deepEqual(initial.newWaits, []);
  assert.equal(initial.snapshot?.waitingIds.has(waiting.id), true);

  const switched = observeSessionAttention(initial.snapshot, "workspace:2", [session("other", "needs_you")]);
  assert.deepEqual(switched.newWaits, []);

  const disconnected = observeSessionAttention(switched.snapshot, null, [waiting]);
  assert.equal(disconnected.snapshot, null);
  assert.deepEqual(disconnected.newWaits, []);

  const recovered = observeSessionAttention(disconnected.snapshot, "workspace:2", [waiting]);
  assert.deepEqual(recovered.newWaits, []);
});

test("new waits count only for live sessions", () => {
  const baseline = observeSessionAttention(null, "workspace:1", []);
  const result = observeSessionAttention(baseline.snapshot, "workspace:1", [
    waiting,
    session("historical", "needs_you", "dead"),
    session("stopping", "needs_you", "stopping"),
    { ...session("archived", "needs_you"), sessionState: "archived" },
    session("blocked", "blocked"),
  ]);
  assert.deepEqual(result.newWaits, [waiting]);
  assert.deepEqual([...result.snapshot!.waitingIds], [waiting.id]);
});

test("focus, terminal visibility, and opt-in choose one delivery channel", () => {
  assert.equal(sessionAlertDelivery(waiting.id, waiting.id, true, true), null);
  assert.equal(sessionAlertDelivery(waiting.id, null, true, true), "toast");
  assert.equal(sessionAlertDelivery(waiting.id, working.id + "-different", true, false), "toast");
  assert.equal(sessionAlertDelivery(waiting.id, waiting.id, false, true), "desktop");
  assert.equal(sessionAlertDelivery(waiting.id, null, false, false), null);
});
