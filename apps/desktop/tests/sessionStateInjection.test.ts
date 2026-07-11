import test from "node:test";
import assert from "node:assert/strict";
import { replaceSessionAfterInjection } from "../src/sessionStateInjection.ts";

test("replaceSessionAfterInjection swaps only the matching session and keeps neighbors intact", () => {
  const sessions = [
    { id: "a", status: "running", memoryState: "live" },
    { id: "b", status: "running", memoryState: "live" },
  ] as any;
  const injected = { id: "b", status: "ended", memoryState: "live" } as any;
  const next = replaceSessionAfterInjection(sessions, injected);

  assert.equal(next.find((session) => session.id === "a")?.status, "running");
  assert.equal(next.find((session) => session.id === "b")?.status, "ended");
});
