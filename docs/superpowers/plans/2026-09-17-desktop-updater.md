# Desktop Updater Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a packaged-only, fixture-testable nightly/stable desktop updater with explicit download and install confirmation while keeping unsigned artifacts and development builds unavailable.

**Architecture:** Keep all updater configuration, network access, candidate verification, download, and install calls in Electron main behind a narrow `UpdateEngine` adapter. Expose only typed status/actions through preload to the renderer; the Settings Updates section and application menu consume the same service. Installation revalidates the candidate, queries live sidecar sessions, asks for native confirmation, waits for bounded sidecar shutdown, then calls the updater; a failed installer triggers one sidecar recovery attempt.

**Tech Stack:** Electron 44, `electron-updater` 6.8.9, Electron main TypeScript, preload context bridge, React/TypeScript renderer, Node’s built-in test runner, pnpm.

## Global Constraints

- Use the fixed GitHub provider from `apps/desktop/electron-builder.yml`; renderer code must not supply feed URLs, tokens, channels, paths, or candidates.
- Development builds return `unavailable` and must not construct/configure an updater, attach updater listeners, perform network work, or expose update actions.
- Keep `autoDownload: false`, `autoInstallOnAppQuit: false`, and `allowDowngrade: false`.
- Stable packaged builds use `latest`; prereleases are eligible only when the first prerelease identifier is exactly `nightly` and the nightly version identity is numeric; malformed or other prereleases are unavailable without stable fallback.
- Candidate identity includes channel, version, tag, metadata URL, metadata digest, and updater payload digest; missing, invalid, or mismatched identity blocks shutdown and install.
- Platform signature verification remains enabled by the existing `forceCodeSigning`, `verifyUpdateCodeSignature`, and macOS notarization configuration; no unsigned bypass is added.
- `lifecycle === "alive"` is the only live-session predicate for install confirmation; empty/dead sessions are safe, and an unavailable session query is explicitly unknown.
- Duplicate checks/downloads/install requests coalesce; stale operation completions and stale singleton updater events cannot mutate the current status.
- The fixture-backed increment does not claim real installed updates; closing issue #511 still requires external signed macOS and Windows artifact validation after issue #510 credentials are available.
- Use pnpm commands through `pnpm.cmd` on this Windows checkout and preserve the known unrelated baseline test failures recorded before implementation.

## Files and Responsibilities

- Create `apps/desktop/electron/updateService.ts` for the updater state machine, adapter contract, channel/version policy, sequencing, candidate identity checks, session confirmation, and shutdown/install orchestration.
- Create `apps/desktop/tests/updateService.test.ts` for fixture-backed state-machine, sequencing, verification, install, recovery, and coalescing tests.
- Modify `apps/desktop/electron/sidecarLifecycle.ts` and `apps/desktop/tests/sidecarLifecycle.test.ts` to add `stopAndWait(timeoutMs)` with bounded, idempotent process-exit semantics.
- Modify `apps/desktop/electron/main.ts` and create `apps/desktop/tests/electronUpdaterWiring.test.ts` to construct the packaged updater, connect it to the sidecar/session services, register typed IPC, and handle installer recovery.
- Modify `apps/desktop/electron/preload.ts`, `apps/desktop/src/orkworksWindow.d.ts`, and `apps/desktop/tests/preloadUpdaterContract.test.ts` to expose the independent renderer contract and status subscription.
- Modify `apps/desktop/electron/menuTemplate.ts`, `apps/desktop/src/App.tsx`, and `apps/desktop/tests/menuUpdater.test.ts` so `Check for updates` opens Settings → Updates and invokes the same check action.
- Modify `apps/desktop/src/components/SettingsModal.tsx`, `apps/desktop/src/App.css`, and create `apps/desktop/tests/updaterSettings.test.ts` for the Updates section and all specified states/actions.
- Create `docs/user/updates.md` and modify `docs/.vitepress/config.mts` for the user-facing packaged-only update behavior and signed-artifact limitation.

---

### Task 1: Define the updater state machine and write its failing fixture tests

**Files:**
- Create: `apps/desktop/electron/updateService.ts`
- Create: `apps/desktop/tests/updateService.test.ts`

**Interfaces:**
- Produces `UpdateService`, `UpdateStatus`, `UpdateCandidate`, `UpdateCandidateIdentity`, `UpdateEngine`, and `UpdateServiceDependencies` for Tasks 2–5.
- `UpdateService` methods are `getStatus(): UpdateStatus`, `check(): Promise<UpdateStatus>`, `download(): Promise<UpdateStatus>`, `requestInstall(): Promise<UpdateStatus>`, and `subscribe(listener: (status: UpdateStatus) => void): () => void`.
- `UpdateServiceDependencies` is:

```ts
export interface UpdateServiceDependencies {
  isPackaged: boolean;
  currentVersion: string;
  createEngine: () => UpdateEngine;
  now: () => string;
  querySessions: () => Promise<ReadonlyArray<{ lifecycle?: string }>>;
  verifyCandidate: (candidate: UpdateCandidate) => Promise<boolean>;
  confirmInstall: (input: {
    candidate: UpdateCandidate;
    liveSessionCount: number | null;
  }) => Promise<boolean>;
  stopSidecar: (timeoutMs: number) => Promise<void>;
  restartSidecar: () => Promise<void>;
}
```

- `UpdateEngine` is:

```ts
export type UpdateEngineEvent =
  | { type: "update-available"; candidate: UpdateCandidate }
  | { type: "update-not-available" }
  | { type: "download-progress"; percent: number; transferred: number; total: number }
  | { type: "update-downloaded"; candidate: UpdateCandidate }
  | { type: "error"; operation: "check" | "download"; message: string };

export interface UpdateEngine {
  autoDownload: boolean;
  autoInstallOnAppQuit: boolean;
  allowDowngrade: boolean;
  allowPrerelease: boolean;
  channel: "latest" | "nightly";
  onEvent(listener: (event: UpdateEngineEvent) => void): () => void;
  checkForUpdates(): Promise<void>;
  downloadUpdate(): Promise<void>;
  quitAndInstall(): void;
}
```

- `UpdateCandidateIdentity` is:

```ts
export interface UpdateCandidateIdentity {
  channel: "latest" | "nightly";
  version: string;
  tag: string;
  metadataUrl: string;
  metadataDigest: string;
  payloadDigest: string;
}

export interface UpdateCandidate {
  identity: UpdateCandidateIdentity;
  releaseNotes: string | null;
  publishedAt: string | null;
}
```

- Status states are exactly `unavailable`, `never-checked`, `checking`, `up-to-date`, `available`, `downloading`, `downloaded`, `installing`, and `error`; every status carries a strictly increasing `sequence` number.

- [ ] **Step 1: Write the failing fixture tests for construction policy, channels, and status replay.**

```ts
import assert from "node:assert/strict";
import test from "node:test";
import { createUpdateService, type UpdateCandidate, type UpdateEngine } from "../electron/updateService.ts";

const candidate = (overrides: Partial<UpdateCandidate["identity"]> = {}): UpdateCandidate => ({
  identity: {
    channel: "nightly",
    version: "0.2.0-nightly.20260917.1",
    tag: "v0.2.0-nightly.20260917.1",
    metadataUrl: "https://github.com/Rambolarsen/orkworks/releases/download/v0.2.0-nightly.20260917.1/latest.yml",
    metadataDigest: "sha256:metadata-1",
    payloadDigest: "sha512:payload-1",
    ...overrides,
  },
  releaseNotes: "nightly notes",
  publishedAt: "2026-09-17T00:00:00Z",
});

function engineFixture(): UpdateEngine & { events: Array<(event: any) => void>; calls: string[] } {
  const events: Array<(event: any) => void> = [];
  const calls: string[] = [];
  return {
    autoDownload: true,
    autoInstallOnAppQuit: true,
    allowDowngrade: true,
    allowPrerelease: false,
    channel: "latest",
    events,
    calls,
    onEvent(listener) { events.push(listener); return () => events.splice(events.indexOf(listener), 1); },
    async checkForUpdates() { calls.push("check"); },
    async downloadUpdate() { calls.push("download"); },
    quitAndInstall() { calls.push("install"); },
  };
}

test("development construction never creates an updater and is unavailable", async () => {
  let constructed = false;
  const service = createUpdateService({
    isPackaged: false,
    currentVersion: "0.2.0",
    createEngine: () => { constructed = true; throw new Error("must not construct"); },
    now: () => "2026-09-17T00:00:00Z",
    querySessions: async () => [],
    verifyCandidate: async () => true,
    confirmInstall: async () => true,
    stopSidecar: async () => undefined,
    restartSidecar: async () => undefined,
  });
  assert.equal(constructed, false);
  assert.equal(service.getStatus().state, "unavailable");
  assert.equal((await service.check()).state, "unavailable");
});

test("nightly status is replayed first and then only increasing sequences are delivered", async () => {
  const engine = engineFixture();
  const service = createUpdateService({
    isPackaged: true,
    currentVersion: "0.2.0-nightly.20260916.1",
    createEngine: () => engine,
    now: () => "2026-09-17T00:00:00Z",
    querySessions: async () => [],
    verifyCandidate: async () => true,
    confirmInstall: async () => true,
    stopSidecar: async () => undefined,
    restartSidecar: async () => undefined,
  });
  const seen: number[] = [];
  const unsubscribe = service.subscribe((status) => seen.push(status.sequence));
  engine.events[0]({ type: "update-available", candidate: candidate() });
  engine.events[0]({ type: "download-progress", percent: 50, transferred: 1, total: 2 });
  unsubscribe();
  assert.equal(seen[0], 0);
  assert.deepEqual(seen.slice(1), [1, 2]);
  assert.ok(seen.every((value, index) => index === 0 || value > seen[index - 1]));
});

test("channel selection admits exact nightly prereleases and never falls back to stable", () => {
  const engine = engineFixture();
  createUpdateService({
    isPackaged: true,
    currentVersion: "0.2.0-nightly.20260916.1",
    createEngine: () => engine,
    now: () => "2026-09-17T00:00:00Z",
    querySessions: async () => [],
    verifyCandidate: async () => true,
    confirmInstall: async () => true,
    stopSidecar: async () => undefined,
    restartSidecar: async () => undefined,
  });
  assert.equal(engine.channel, "nightly");
  assert.equal(engine.allowPrerelease, true);
});
```

- [ ] **Step 2: Run the focused test to verify it fails for the missing service.**

Run: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/updateService.test.ts`

Expected: FAIL because `apps/desktop/electron/updateService.ts` does not exist.

- [ ] **Step 3: Implement the minimal service, policy, and status sequencing.**

Implement `createUpdateService` with these exact rules:

1. If `isPackaged` is false, return a service whose status is `{ state: "unavailable", reason: "development", sequence: 0 }` and never call `createEngine`.
2. Parse the first prerelease identifier from `currentVersion`; choose `nightly` only for `nightly` followed by numeric nightly identity, choose `latest` for a stable version, and return `unsupported-version` for malformed or other prereleases.
3. Construct the engine once, set all four safety properties before registering listeners, and reject any engine configuration that does not match the required values.
4. Convert engine events into status snapshots. Store the active operation sequence and ignore completions/events from older checks, downloads, or candidates.
5. `check()` and `download()` return the existing operation promise when one is active. `requestInstall()` returns the existing install promise when one is active.
6. `subscribe()` synchronously invokes the listener with the current snapshot, then emits only snapshots with larger sequence values.

The service must not call `quitAndInstall()` from `check()`, `download()`, ordinary app quit, or any automatic event.

- [ ] **Step 4: Run the focused tests and add the policy/error cases.**

Run: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/updateService.test.ts`

Expected: PASS for the construction, replay, and nightly-channel tests. Add and pass cases for stable `latest`, malformed prerelease unavailable, stable candidate ignored by nightly channel, duplicate check/download coalescing, stale progress ignored, updater error becoming retryable `error`, and `up-to-date` when no candidate is reported.

- [ ] **Step 5: Commit the state-machine deliverable.**

```powershell
git add apps/desktop/electron/updateService.ts apps/desktop/tests/updateService.test.ts
git commit -m "feat: add fixture-backed desktop updater service"
```

### Task 2: Add bounded, awaitable sidecar shutdown and recovery primitives

**Files:**
- Modify: `apps/desktop/electron/sidecarLifecycle.ts:11-18, 74-105, 252-281`
- Modify: `apps/desktop/tests/sidecarLifecycle.test.ts`

**Interfaces:**
- Extend `SidecarLifecycle` with `stopAndWait(timeoutMs: number): Promise<void>`.
- Preserve `stop(): void`, `retry(): Promise<number>`, and `dispose(): void` behavior for existing callers.

- [ ] **Step 1: Add failing tests for normal exit, timeout, process error, and repeated calls.**

Use the existing fake process and timer helpers in `sidecarLifecycle.test.ts` and add these assertions:

```ts
test("stopAndWait resolves after the requested child exits", async () => {
  const { lifecycle, process } = fixture();
  await lifecycle.start("C:\\workspace");
  const stopping = lifecycle.stopAndWait(1000);
  process.emit("exit", 0);
  await assert.doesNotReject(stopping);
});

test("stopAndWait rejects on timeout without starting a replacement", async () => {
  const { lifecycle, timers, process, spawnCount } = fixture();
  await lifecycle.start("C:\\workspace");
  const stopping = lifecycle.stopAndWait(50);
  timers.advance(50);
  await assert.rejects(stopping, /timed out/i);
  assert.equal(spawnCount(), 1);
  assert.equal(process.killCount, 1);
});

test("stopAndWait rejects on process error and repeated callers share one promise", async () => {
  const { lifecycle, process } = fixture();
  await lifecycle.start("C:\\workspace");
  const first = lifecycle.stopAndWait(1000);
  const second = lifecycle.stopAndWait(1000);
  assert.strictEqual(first, second);
  process.emit("error", new Error("kill failed"));
  await assert.rejects(first, /kill failed/);
});
```

- [ ] **Step 2: Run the focused lifecycle test to verify the new API fails.**

Run: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/sidecarLifecycle.test.ts`

Expected: FAIL because `stopAndWait` is not part of the lifecycle interface.

- [ ] **Step 3: Implement one shared stop promise tied to the current generation’s `exit`/`error`.**

Add a `stopWait: Promise<void> | null` field to the current generation. `stopAndWait()` must capture the current generation, attach completion listeners before invalidating it, cancel recovery, invalidate the generation against future automatic restarts, call `stopCurrent()`, return the existing promise for repeated callers, resolve on the captured child’s `exit`, reject on the captured child’s `error`, and reject after `timeoutMs` with `/timed out/i`. A missing current process resolves immediately. Clear the timeout on every completion. Do not spawn a replacement from the stop path. Keep `dispose()` safe after a prior stop and keep `retry()` as the explicit restart path.

- [ ] **Step 4: Run lifecycle tests and the desktop typecheck.**

Run: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/sidecarLifecycle.test.ts`  
Run: `pnpm.cmd --dir apps/desktop exec tsc --noEmit`

Expected: PASS for the focused lifecycle tests and PASS for TypeScript compilation.

- [ ] **Step 5: Commit the lifecycle deliverable.**

```powershell
git add apps/desktop/electron/sidecarLifecycle.ts apps/desktop/tests/sidecarLifecycle.test.ts
git commit -m "feat: add bounded sidecar shutdown"
```

### Task 3: Wire the packaged updater through Electron main and preload

**Files:**
- Modify: `apps/desktop/electron/main.ts:1-80, 540-680, 1245-1275`
- Modify: `apps/desktop/electron/preload.ts:1-145`
- Modify: `apps/desktop/src/orkworksWindow.d.ts:1-130`
- Create: `apps/desktop/tests/electronUpdaterWiring.test.ts`
- Create: `apps/desktop/tests/preloadUpdaterContract.test.ts`

**Interfaces:**
- Add the following preload/main methods with the same names and return types on both sides:

```ts
getUpdateStatus(): Promise<UpdateStatus>;
checkForUpdates(): Promise<UpdateStatus>;
downloadUpdate(): Promise<UpdateStatus>;
requestUpdateInstall(): Promise<UpdateStatus>;
onUpdateStatus(callback: (status: UpdateStatus) => void): () => void;
```

- `main.ts` creates the real `UpdateEngine` only inside the packaged branch. It maps public `electron-updater` events into the `UpdateEngineEvent` adapter and uses the existing fixed GitHub provider/configuration from `electron-builder.yml`. Development does not register updater IPC handlers; preload returns the static development-unavailable snapshot when `process.defaultApp` is true.
- `querySessions` awaits the current backend readiness, calls `GET /sessions` through the existing `listSessions(baseUrl)`, and treats only `lifecycle === "alive"` as live.
- `stopSidecar` calls `sidecarLifecycle.stopAndWait(10_000)`; `restartSidecar` starts the last workspace cwd through the existing lifecycle/restoration path. A failed recovery produces an error status whose message tells the user to restart OrkWorks.

- [ ] **Step 1: Write failing source-contract tests for packaged gating, fixed updater settings, IPC names, and preload parity.**

```ts
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const main = await readFile(new URL("../electron/main.ts", import.meta.url), "utf8");
const preload = await readFile(new URL("../electron/preload.ts", import.meta.url), "utf8");
const windowTypes = await readFile(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");

test("main gates updater construction on app.isPackaged and keeps automatic install disabled", () => {
  assert.match(main, /if \(app\.isPackaged\)[\s\S]*createUpdateService/);
  assert.match(main, /if \(app\.isPackaged\)[\s\S]*registerUpdateIpc/);
  assert.match(main, /autoDownload:\s*false/);
  assert.match(main, /autoInstallOnAppQuit:\s*false/);
  assert.match(main, /allowDowngrade:\s*false/);
  assert.match(main, /stopAndWait\(10_000\)/);
});

test("preload and renderer contracts expose the same four actions and subscription", () => {
  for (const name of ["getUpdateStatus", "checkForUpdates", "downloadUpdate", "requestUpdateInstall", "onUpdateStatus"]) {
    assert.match(preload, new RegExp(name));
    assert.match(windowTypes, new RegExp(name));
  }
});
```

- [ ] **Step 2: Run the wiring tests to verify they fail before integration.**

Run: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/electronUpdaterWiring.test.ts tests/preloadUpdaterContract.test.ts`

Expected: FAIL because the updater service and IPC methods are not wired.

- [ ] **Step 3: Implement the main-process adapter and handlers.**

Add a single `updateService` variable owned by main. On packaged startup, construct the adapter before the first window is created, configure the updater with the fixed provider and required safety flags, subscribe once to public updater events, and register these IPC handlers:

```ts
ipcMain.handle("get-update-status", () => updateService.getStatus());
ipcMain.handle("check-for-updates", () => updateService.check());
ipcMain.handle("download-update", () => updateService.download());
ipcMain.handle("request-update-install", () => updateService.requestInstall());
ipcMain.on("subscribe-update-status", (event) => {
  const unsubscribe = updateService.subscribe((status) => event.sender.send("update-status", status));
  event.sender.once("destroyed", unsubscribe);
});
```

The real `quitAndInstall()` call must occur only after the service has completed candidate revalidation, live-session query, confirmation, and `stopAndWait()`. Keep normal `before-quit` cleanup free of updater install calls.

- [ ] **Step 4: Implement the preload bridge and independent renderer declaration.**

Expose the four `ipcRenderer.invoke` methods and a subscription that returns an unsubscribe function. When `process.defaultApp` is true, return the static development-unavailable snapshot and do not invoke IPC. Keep the declarations in `orkworksWindow.d.ts` independently typed with imported `UpdateStatus`; do not derive the renderer type from the preload implementation.

- [ ] **Step 5: Run focused wiring tests and typecheck.**

Run: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/electronUpdaterWiring.test.ts tests/preloadUpdaterContract.test.ts`  
Run: `pnpm.cmd --dir apps/desktop exec tsc --noEmit`

Expected: PASS for both focused tests and PASS for TypeScript compilation.

- [ ] **Step 6: Commit the Electron wiring deliverable.**

```powershell
git add apps/desktop/electron/main.ts apps/desktop/electron/preload.ts apps/desktop/src/orkworksWindow.d.ts apps/desktop/tests/electronUpdaterWiring.test.ts apps/desktop/tests/preloadUpdaterContract.test.ts
git commit -m "feat: wire packaged updater through Electron"
```

### Task 4: Implement install sequencing, candidate verification, and recovery tests

**Files:**
- Modify: `apps/desktop/electron/updateService.ts`
- Modify: `apps/desktop/tests/updateService.test.ts`

**Interfaces:**
- `UpdateServiceDependencies.verifyCandidate(candidate)` returns true only after electron-updater reports a downloaded candidate whose platform verification and checksum validation succeeded. Fixture engines can return false to model missing/invalid signature, checksum, or identity.

- [ ] **Step 1: Add failing tests for strict install ordering and all abort paths.**

```ts
test("install verifies, queries sessions, confirms, waits, then installs", async () => {
  const order: string[] = [];
  const engine = engineFixture();
  const service = packagedService(engine, {
    verifyCandidate: async () => { order.push("verify"); return true; },
    querySessions: async () => { order.push("sessions"); return [{ lifecycle: "alive" }]; },
    confirmInstall: async ({ liveSessionCount }) => { order.push(`confirm:${liveSessionCount}`); return true; },
    stopSidecar: async () => { order.push("stop"); },
  });
  engine.events[0]({ type: "update-downloaded", candidate: candidate() });
  await service.requestInstall();
  assert.deepEqual(order, ["verify", "sessions", "confirm:1", "stop"]);
  assert.deepEqual(engine.calls, ["install"]);
});

test("invalid verification never stops the sidecar or installs", async () => {
  const engine = engineFixture();
  const service = packagedService(engine, {
    verifyCandidate: async () => false,
    stopSidecar: async () => { throw new Error("must not stop"); },
  });
  engine.events[0]({ type: "update-downloaded", candidate: candidate() });
  const status = await service.requestInstall();
  assert.equal(status.state, "error");
  assert.deepEqual(engine.calls, []);
});

test("cancelled confirmation has no shutdown or install side effect", async () => {
  const engine = engineFixture();
  const service = packagedService(engine, {
    verifyCandidate: async () => true,
    confirmInstall: async () => false,
    stopSidecar: async () => { throw new Error("must not stop"); },
  });
  engine.events[0]({ type: "update-downloaded", candidate: candidate() });
  await service.requestInstall();
  assert.deepEqual(engine.calls, []);
});

test("installer failure attempts sidecar recovery and reports restart guidance if recovery fails", async () => {
  const engine = engineFixture();
  engine.quitAndInstall = () => { throw new Error("installer failed"); };
  let restarted = 0;
  const service = packagedService(engine, {
    verifyCandidate: async () => true,
    stopSidecar: async () => undefined,
    restartSidecar: async () => { restarted += 1; throw new Error("recovery failed"); },
  });
  engine.events[0]({ type: "update-downloaded", candidate: candidate() });
  const status = await service.requestInstall();
  assert.equal(restarted, 1);
  assert.equal(status.state, "error");
  assert.match(status.message, /restart OrkWorks/i);
});
```

- [ ] **Step 2: Run the new install tests to verify they fail.**

Run: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/updateService.test.ts`

Expected: FAIL because verification, live-session sequencing, and recovery are not implemented.

- [ ] **Step 3: Implement the strict install transaction.**

Implement `requestInstall()` in this order:

1. Require a downloaded candidate and compare channel, version, tag, metadata URL, metadata digest, and payload digest with the cached identity.
2. Call `verifyCandidate`; if false or it throws, publish retryable `error` and return without querying sessions, stopping, or installing.
3. Call `querySessions`; count only `lifecycle === "alive"`. If the query fails, use `liveSessionCount: null` and continue only after the confirmation callback receives the unknown state.
4. Await `confirmInstall`; false publishes the unchanged downloaded state and performs no side effects.
5. Publish `installing`, await `stopSidecar(10_000)`, call `quitAndInstall()`, and keep the install promise shared by concurrent callers.
6. If shutdown or installer invocation fails, attempt `restartSidecar()` exactly once. Publish a retryable error that distinguishes recovery success from “restart OrkWorks” recovery failure. Never claim that an update was installed.

Use a monotonically increasing operation token for both service operations and singleton updater events. A stale event must be ignored even if it arrives after a newer operation has started.

- [ ] **Step 4: Add identity mismatch, unknown-session, stale-event, duplicate-install, shutdown-timeout, and recovery-success tests, then run the focused suite.**

Run: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/updateService.test.ts`

Expected: PASS with no call to `quitAndInstall()` on any rejected identity, failed verification, cancelled confirmation, failed shutdown, or stale event.

- [ ] **Step 5: Commit the transaction deliverable.**

```powershell
git add apps/desktop/electron/updateService.ts apps/desktop/tests/updateService.test.ts
git commit -m "feat: gate updater installation on verification and shutdown"
```

### Task 5: Add the menu action and Settings Updates UI

**Files:**
- Modify: `apps/desktop/electron/menuTemplate.ts:1-145`
- Modify: `apps/desktop/src/App.tsx:55-70, 235-255, 450-475, 660-690`
- Modify: `apps/desktop/src/components/SettingsModal.tsx:25-120, 810-1040`
- Modify: `apps/desktop/src/App.css`
- Create: `apps/desktop/tests/menuUpdater.test.ts`
- Create: `apps/desktop/tests/updaterSettings.test.ts`

**Interfaces:**
- Extend `SettingsSection` with `"updates"`.
- Extend `MenuCommand.action` with `"check-for-updates"`.
- `App.tsx` handles the command by opening Settings with `initialSection="updates"`, then calls `window.orkworks.checkForUpdates()`; it does not create a second update flow.

- [ ] **Step 1: Add failing menu and Settings source tests.**

```ts
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const menu = await readFile(new URL("../electron/menuTemplate.ts", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const settings = await readFile(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

test("Help menu exposes the shared update check command", () => {
  assert.match(menu, /check-for-updates/);
  assert.match(app, /check-for-updates[\s\S]*checkForUpdates/);
  assert.match(menu, /isCapturing[\s\S]*check-for-updates|check-for-updates[\s\S]*sendIfNotCapturing/);
});

test("Settings Updates renders all lifecycle states and actions", () => {
  for (const text of ["Updates", "Check for updates", "Download update", "Restart and install", "up-to-date", "downloaded", "unavailable"]) {
    assert.match(settings, new RegExp(text, "i"));
  }
  assert.match(settings, /onUpdateStatus/);
});
```

- [ ] **Step 2: Run the focused UI tests to verify they fail.**

Run: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/menuUpdater.test.ts tests/updaterSettings.test.ts`

Expected: FAIL because the command, section, and update controls do not exist.

- [ ] **Step 3: Add the menu command and shared App handler.**

Add `{ action: "check-for-updates" }` under Help. In `App.tsx`, open Settings on the Updates section, invoke `checkForUpdates()`, subscribe on mount, retain the latest status in state, and remove the subscription on unmount. Keep the existing settings reload behavior intact for other sections.

- [ ] **Step 4: Add the Settings Updates section with explicit state rendering.**

Add a navigation item and render branch for `updates`. Show current version, channel, candidate version/tag, release notes, progress, retryable error text, and the correct action button for each state. Disable buttons while `checking`, `downloading`, or `installing`; wire retry to `checkForUpdates`, download to `downloadUpdate`, and restart/install to `requestUpdateInstall`. Render the confirmation copy that restarting OrkWorks stops the sidecar and can interrupt live sessions. Do not expose feed URLs, credentials, filesystem paths, or candidate mutation controls.

- [ ] **Step 5: Run focused UI tests and typecheck.**

Run: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/menuUpdater.test.ts tests/updaterSettings.test.ts`  
Run: `pnpm.cmd --dir apps/desktop exec tsc --noEmit`

Expected: PASS for the focused UI tests and PASS for TypeScript compilation.

- [ ] **Step 6: Commit the UI deliverable.**

```powershell
git add apps/desktop/electron/menuTemplate.ts apps/desktop/src/App.tsx apps/desktop/src/components/SettingsModal.tsx apps/desktop/src/App.css apps/desktop/tests/menuUpdater.test.ts apps/desktop/tests/updaterSettings.test.ts
git commit -m "feat: add updater controls to Settings"
```

### Task 6: Document the packaged-only update contract and verify the branch

**Files:**
- Create: `docs/user/updates.md`
- Modify: `docs/.vitepress/config.mts:83-103`
- Test: `apps/desktop/tests/updateService.test.ts`, `apps/desktop/tests/sidecarLifecycle.test.ts`, `apps/desktop/tests/electronUpdaterWiring.test.ts`, `apps/desktop/tests/preloadUpdaterContract.test.ts`, `apps/desktop/tests/menuUpdater.test.ts`, `apps/desktop/tests/updaterSettings.test.ts`

**Interfaces:**
- Documentation describes the implemented UI and safety behavior without claiming that fixture tests prove a real installed update.
- The documentation links to `specs/release-pipeline.md` for credential-backed release validation and states that signed macOS/Windows artifact validation remains required for issue #511 completion.

- [ ] **Step 1: Write the user documentation and sidebar link.**

Create `docs/user/updates.md` with these user-visible facts: packaged builds check the fixed OrkWorks GitHub release feed; stable builds use stable releases, nightly builds use exact `nightly` prereleases; development builds show updates unavailable; downloads are manual; installation requires confirmation and may warn about live sessions; native signature/checksum verification must succeed; failed downloads/install attempts are retryable; a failed recovery may require restarting OrkWorks. Add `{ text: 'Updates', link: '/docs/user/updates' }` to the user-guide sidebar.

- [ ] **Step 2: Run focused tests, docs build, typecheck, and diff checks.**

Run: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/updateService.test.ts tests/sidecarLifecycle.test.ts tests/electronUpdaterWiring.test.ts tests/preloadUpdaterContract.test.ts tests/menuUpdater.test.ts tests/updaterSettings.test.ts`  
Run: `pnpm.cmd --dir apps/desktop exec tsc --noEmit`  
Run: `pnpm.cmd --dir docs build`  
Run: `git diff --check`

Expected: all focused updater tests PASS, TypeScript PASS, docs build PASS, and `git diff --check` produces no output. Record the six known unrelated baseline failures from the pre-change full suite if they remain; do not attribute them to this feature without new evidence.

- [ ] **Step 3: Run the full desktop test suite and inspect the diff.**

Run: `pnpm.cmd --dir apps/desktop test`

Expected: updater tests PASS. Compare any remaining failures with the pre-change baseline: Electron binary installation, concurrent settings memory, Windows preload path normalization, recommendation evidence, Taskmaster debug metadata, and Taskmaster settings component were already failing before this plan.

- [ ] **Step 4: Perform the completion gate review.**

Confirm from the diff that no unsigned bypass, automatic install-on-quit, downgrade path, renderer-controlled feed configuration, credential logging, or session-content logging was introduced. Confirm the real signed macOS and Windows validation gate remains documented as outstanding for #511.

- [ ] **Step 5: Commit the documentation and verification deliverable.**

```powershell
git add docs/user/updates.md docs/.vitepress/config.mts
git commit -m "docs: document desktop update behavior"
```
