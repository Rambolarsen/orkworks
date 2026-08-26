import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const appSource = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

test("App subscribes to backend lifecycle events and maps failures to visible status", () => {
  assert.match(appSource, /window\.orkworks\.onBackendLifecycle/);
  assert.match(appSource, /state === "ready"/);
  assert.match(appSource, /state === "failed"/);
  assert.match(appSource, /state === "exhausted"/);
  assert.match(appSource, /setBackendStatus\("unreachable"\)/);
  assert.match(appSource, /setBackendStatus\("exhausted"\)/);
});

test("App stops session polling unless the backend is connected", () => {
  assert.match(appSource, /const enabled = backendStatus === "connected" && workspace !== null/);
  assert.match(appSource, /workspaceSessionController\.setPollingEnabled\(enabled\)/);
});

test("App exposes a retry action that resets status and invokes the lifecycle bridge", () => {
  assert.match(appSource, /setBackendStatus\("connecting…"\)/);
  assert.match(appSource, /window\.orkworks\.retryBackend\(\)/);
  assert.match(appSource, /backend-recovery/);
  assert.match(appSource, />\s*Retry\s*</);
});
