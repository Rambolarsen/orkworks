# Desktop Updater Design

**Status:** Revised after adversarial review; fixture-backed implementation scope

**Date:** 2026-09-17

**Final review wave 2 decision (2026-09-17):** macOS and Windows installation fail closed before
verification, session queries, confirmation, sidecar shutdown, or installer
invocation or recovery. The pinned macOS public API cannot establish native
verification without arming installation. NSIS schedules app quit before an
asynchronous installer failure is known, retains its install latch on that failure,
and exposes no supported recovery/reset handshake. The production adapter reports
installation unavailable and rejects direct install calls without invoking NSIS.
The generic transaction below remains fixture-tested but is not an enabled native
installation path. Repeated blocked requests settle and leave checking usable.

Windows signature verification follows the release pipeline's exact certificate
SimpleName contract: accept the pinned verifier's exact successful CN message,
reject all other verification warnings (including skipped checks), and preserve
its failure verdict. Cached downloads that skip verification provide no current
process proof. Release identity excludes `UpdateDownloadedEvent.downloadedFile`,
so check, download completion, and fresh metadata revalidation compare the same
release metadata. These guards supersede the native install paths described below;
signed artifact and installed-app validation remain external prerequisites.

**Issue:** #511 — Desktop: add user-controlled signed in-app updates

## Goal

Add the main-process and Settings plumbing for user-controlled desktop updates
while signed nightly publication and installed-build verification remain
external prerequisites. Development builds must remain unable to check,
download, or install updates.

This increment proves updater behavior with injected fixtures and deterministic
tests. It does not weaken the signed-only release workflow, publish unsigned
nightlies, or claim that a real installed macOS or Windows build can update.

## Architecture

`apps/desktop/electron/updateService.ts` owns the updater lifecycle. It is the
only component that configures `electron-updater`, selects the fixed public
GitHub provider, performs network/download operations, and requests
installation. The service receives its Electron and backend dependencies
through a narrow options object so tests can use a fake updater, clock,
metadata check, session-state query, and shutdown coordinator without starting
Electron or a sidecar.

The service exposes a read-only status snapshot and a small command surface:

- `getStatus()` returns the current update state.
- `check()` discovers an update without downloading it.
- `download()` downloads the currently discovered update and coalesces
  duplicate requests.
- `requestInstall()` revalidates a downloaded update, verifies the candidate,
  queries live session state from the current backend, constructs a native
  restart confirmation, performs bounded sidecar shutdown, and invokes the
  updater's install method only after shutdown succeeds. All concurrent install
  requests share one promise and one confirmation.

The service uses a monotonically increasing operation sequence. Every async
completion and every singleton `electron-updater` event carries or is checked
against the current sequence before publishing a status, so stale progress,
error, and `update-downloaded` events cannot overwrite a newer operation.
Status subscriptions replay the current snapshot and then receive strictly
increasing sequence numbers, closing the gap between an initial snapshot and
listener registration.

The updater factory has a construction-time `app.isPackaged` gate. In
development it returns an unavailable service without constructing an
`electron-updater` instance, registering event listeners, configuring a
provider, making network requests, or enabling any IPC action. This gate is
tested directly.

The service uses `autoDownload = false`, `autoInstallOnAppQuit = false`, and
disables downgrades. A packaged version with no prerelease uses the stable
`latest` channel. A packaged version whose first prerelease identifier is
exactly `nightly` and whose full version matches the repository's numeric
nightly identity uses the explicit `nightly` channel and allows prereleases.
Other prerelease identifiers and malformed nightly versions are unavailable
errors; they never fall back to stable discovery.

Before install, the service requires a verified candidate identity containing
the channel, version/tag, metadata URL and digest, and updater-payload digest
(`sha512` or the platform-equivalent verified digest). The real updater remains
responsible for platform signature verification: Windows uses the configured
publisher verification and macOS relies on signed/notarized update artifacts.
The service's injected verifier models that result in fixture tests. A missing,
invalid, or mismatched verification result blocks shutdown and installation.

## Electron boundary

`electron/preload.ts`, `src/orkworksWindow.d.ts`, and the main-process service
define the same narrow contract independently on each side of the existing
Electron boundary. The renderer receives status snapshots and update events;
it cannot provide a feed URL, release token, executable path, candidate
identity, or updater configuration. `onUpdateStatus` first delivers the
current snapshot and then only newer sequence-numbered events.

The native menu adds `Check for updates`. The menu command is forwarded to the
renderer only as a request to open Settings on the Updates section and start
the same check command used by the Settings button. There is one updater flow,
not separate menu and Settings implementations.

## Settings UI

Settings gains an `Updates` section showing:

- installed version and stable/nightly channel;
- unavailable, never-checked, checking, up-to-date, update-available,
  downloading, downloaded, installing, and error states;
- available version and release notes when present;
- download progress when available; and
- retry, Download, and Restart and install actions according to the current
  state.

The UI does not promise live terminal continuity. The install confirmation
states that restarting OrkWorks stops the sidecar and interrupts live sessions.
Cancellation leaves the application running.

## Install and failure behavior

The install sequence is strictly:

1. Revalidate the cached candidate against current metadata, comparing channel,
   version/tag, metadata identity/digest, and updater-payload digest.
2. Query the current backend for sessions; `lifecycle === "alive"` is the live
   predicate. A dead or empty session set is safe; an unavailable backend is
   represented as unknown.
3. Construct the native confirmation using that result. Unknown state must say
   that sessions may be interrupted. Cancellation has no shutdown or install
   side effect.
4. Ask the sidecar lifecycle for an awaitable bounded shutdown. The lifecycle
   contract reports success only after process exit, and reports timeout or
   failure without blindly replacing a still-running sidecar. Repeated shutdown
   calls share one promise.
5. Invoke the updater's explicit install operation only after verification and
   shutdown succeed.

The lifecycle gains a focused `stopAndWait(timeoutMs)` seam backed by the
existing process exit/error events. It is bounded, idempotent, and testable for
normal exit, timeout, process error, and repeated callers. If the installer
fails after a successful shutdown, main attempts to restart the sidecar at the
last workspace path before reporting a retryable error. If recovery also
fails, the UI reports that OrkWorks must be restarted; it never replaces a
running sidecar or claims the update was installed.

Ordinary application quit never installs a downloaded update. On a later
launch, a cached update is not installable until the current release metadata
is checked and confirms the same candidate. Offline checks, rate limits, no
eligible release, interrupted downloads, invalid metadata, signature/checksum
failures, and updater errors become readable retryable status; diagnostics omit
credentials and session content.

## Testing strategy

Tests are written before implementation in focused units:

- update-service state transitions, duplicate-action coalescing, generation
  guards, channel configuration, development-build refusal, retryable errors,
  cached-download revalidation, exact candidate identity comparison, signature
  or checksum verification refusal before shutdown, confirmation cancellation,
  live/dead/unavailable session state, duplicate install requests, stale updater
  events and progress, status-subscription replay, shutdown
  timeout/process-error/repeated-call behavior, installer failure recovery, and
  install ordering;
- menu-template coverage for the command and hotkey-capture suppression;
- preload and renderer contract coverage for status/events and commands;
- Settings source/component coverage for the Updates section and action
  enablement; and
- a fixture-backed updater contract that verifies stable/nightly selection
  without contacting GitHub or requiring signed artifacts.

The existing `nightlyUpdateChannel.test.mjs` remains the transport-level
contract for electron-updater's custom channel. A real signed installed-build
update on macOS and Windows remains blocked until #510's credentials and
manual nightly validation are complete. The fixture suite must not be used to
close #511: completion also requires recording one older signed build updating
to a newer signed build on each supported platform, preserving settings and
requiring explicit restart, plus updating the operator/user validation record.

## Non-goals

- No unsigned nightly publication or release-workflow bypass.
- No automatic download, install-on-quit, downgrade, or channel switching.
- No updater implementation in the Rust sidecar or renderer-owned network
  access.
- No automatic harness resume or PTY continuity promise.
- No live artifact verification in this fixture-backed increment.
