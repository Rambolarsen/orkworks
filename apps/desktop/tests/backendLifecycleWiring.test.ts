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
  assert.match(appSource, /shouldEnableSessionPolling\(backendStatus, workspace !== null, isSwitchingWorkspace\)/);
  assert.match(appSource, /workspaceSessionController\.setPollingEnabled\(enabled\)/);
});

test("App exposes a retry action that resets status and invokes the lifecycle bridge", () => {
  assert.match(appSource, /setBackendStatus\("connecting…"\)/);
  assert.match(appSource, /window\.orkworks\.retryBackend\(\)/);
  assert.match(appSource, /backend-recovery/);
  assert.match(appSource, />\s*Retry\s*</);
});

test("opening a workspace adopts the restoration once — from the ready handler, not the dialog handler", () => {
  // One restoration (main publishes the same restored workspace via the
  // ready lifecycle event and as open-workspace/get-initial-workspace IPC
  // results) must trigger exactly one adoptRestoredWorkspace. A second
  // adopt clears the just-populated session list and refetches — the
  // double /sessions round-trip and visible list flash from issue #357.
  const start = appSource.indexOf("const handleOpenWorkspace = useCallback");
  const end = appSource.indexOf("const openSettings = useCallback");
  assert.ok(start !== -1 && end !== -1 && start < end, "handleOpenWorkspace block not found");
  const dialogHandler = appSource.slice(start, end);
  assert.doesNotMatch(dialogHandler, /adoptRestoredWorkspace/);
  assert.match(appSource, /state === "ready"[\s\S]{0,200}adoptRestoredWorkspace\(event\.workspace\)/);
});
