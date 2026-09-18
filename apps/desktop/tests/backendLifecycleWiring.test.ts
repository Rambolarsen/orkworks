import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const appSource = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

test("startup waits in picker without health probes or recovery UI until a lifecycle starts", () => {
  assert.match(appSource, /useState<BackendStatus>\("picker"\)/);
  assert.match(appSource, /event.state === "picker"[\s\S]*?setBackendStatus\("picker"\)/);
  assert.match(appSource, /if \(backendStatus !== "connecting…"\) return;/);
  assert.match(appSource, /backendStatus !== "picker" &&/);
});

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

test("App makes the coordinator lifecycle the renderer admission authority", () => {
  assert.match(appSource, /workspaceSessionController\.setAdmissionEnabled\(event\.state === "ready"\)/);
  assert.match(appSource, /workspaceSessionController\.submitActiveSession\(sid\)/);
  assert.match(appSource, /sessionAdmissionEnabled/);
  assert.match(appSource, /canFixWithAi=\{sessionAdmissionEnabled && activeSession\?\.lifecycle === "alive"\}/);
});

test("App exposes a retry action that resets status and invokes the lifecycle bridge", () => {
  assert.match(appSource, /setBackendStatus\("connecting…"\)/);
  assert.match(appSource, /window\.orkworks\.retryBackend\(\)/);
  assert.match(appSource, /backend-recovery/);
  assert.match(appSource, />\s*Retry\s*</);
});

test("a stale retry rejection is guarded so it cannot clobber a newer retry — issue #356", () => {
  const start = appSource.indexOf("const handleRetryBackend = useCallback");
  const end = appSource.indexOf("const handleBackendUnavailable = useCallback");
  assert.ok(start !== -1 && end !== -1 && start < end, "handleRetryBackend block not found");
  const handler = appSource.slice(start, end);

  assert.match(handler, /backendRetryGuardRef\.current\.begin\(\)/);
  // The unreachable transition on rejection must be gated by isCurrent(token),
  // not applied unconditionally — otherwise a superseded retry's rejection can
  // overwrite a newer retry's success (double-click Retry race, issue #356).
  assert.match(handler, /catch\(\(\) => \{\s*[\s\S]*?if \(backendRetryGuardRef\.current\.isCurrent\(token\)\)\s*\{\s*setBackendStatus\("unreachable"\);/);
});

test("a retry result preserves an unresolved cleanup diagnostic instead of mapping it to unreachable", () => {
  const start = appSource.indexOf("const handleRetryBackend = useCallback");
  const end = appSource.indexOf("const handleBackendUnavailable = useCallback");
  assert.ok(start !== -1 && end !== -1 && start < end, "handleRetryBackend block not found");
  const handler = appSource.slice(start, end);

  assert.match(handler, /\.then\(\(result\) => \{/);
  assert.match(handler, /if \(!result\.ok\)/);
  assert.match(handler, /setWorkspaceSwitchDiagnostic\(result\.failure\.message\)/);
  assert.match(handler, /setBackendStatus\(result\.state\);/);
  assert.doesNotMatch(handler, /result\.state === "unresolved" \? "unresolved" : "unreachable"/);
});

test("a failed destination retry preserves the picker state from the typed IPC result", () => {
  const start = appSource.indexOf("const handleRetryBackend = useCallback");
  const end = appSource.indexOf("const handleBackendUnavailable = useCallback");
  assert.ok(start !== -1 && end !== -1 && start < end, "handleRetryBackend block not found");
  const handler = appSource.slice(start, end);

  assert.match(handler, /if \(!result\.ok\) \{/);
  assert.match(handler, /setWorkspaceSwitchDiagnostic\(result\.failure\.message\)/);
  assert.match(handler, /setBackendStatus\(result\.state\);/);
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

test("a delayed initial workspace snapshot cannot overwrite a newer live lifecycle diagnostic", () => {
  assert.match(appSource, /const workspaceLifecycleRef = useRef\(\{ generation: 0, readyGeneration: null as number \| null \}\)/);
  assert.match(appSource, /const generation = workspaceLifecycleRef\.current\.generation \+ 1/);
  const start = appSource.indexOf("async function loadInitialWorkspace");
  const end = appSource.indexOf("void loadInitialWorkspace", start);
  assert.ok(start !== -1 && end > start, "initial workspace loader block not found");
  const loader = appSource.slice(start, end);
  assert.match(loader, /generation: workspaceLifecycleRef\.current\.generation/);
  assert.match(loader, /lifecycle\.generation !== snapshotRequest\.generation/);
  assert.match(loader, /setWorkspaceHistoryDiagnostic\(snapshot\.historyDiagnostic\)/);
});
