# Desktop Updater Design

**Status:** Approved architecture; fixture-backed implementation scope

**Date:** 2026-09-17

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
- `requestInstall()` revalidates a downloaded update, asks the main process to
  confirm the restart, queries live session state from the current backend,
  performs bounded sidecar shutdown, and invokes the updater's install method
  only after shutdown succeeds.

The service uses a monotonically increasing operation generation. Every async
completion checks its generation before publishing a status or event, so a
late result from an older check or download cannot overwrite a newer state.
The service uses `autoDownload = false`, `autoInstallOnAppQuit = false`, and
disables downgrades. A packaged version without a nightly prerelease suffix
uses the stable `latest` channel; a packaged version with the repository's
nightly suffix uses the explicit `nightly` channel and allows prereleases.
Development builds return an unavailable status and never touch the updater.

## Electron boundary

`electron/preload.ts`, `src/orkworksWindow.d.ts`, and the main-process service
define the same narrow contract independently on each side of the existing
Electron boundary. The renderer receives status snapshots and update events;
it cannot provide a feed URL, release token, executable path, or updater
configuration.

The native menu adds `Check for updates`. The menu command is forwarded to the
renderer only as a request to open Settings on the Updates section and start
the same check command used by the Settings button. There is one updater flow,
not separate menu and Settings implementations.

## Settings UI

Settings gains an `Updates` section showing:

- installed version and stable/nightly channel;
- unavailable, idle, checking, update-available, downloading, downloaded,
  installing, and error states;
- available version and release notes when present;
- download progress when available; and
- retry, Download, and Restart and install actions according to the current
  state.

The UI does not promise live terminal continuity. The install confirmation
states that restarting OrkWorks stops the sidecar and interrupts live sessions.
Cancellation leaves the application running.

## Install and failure behavior

Before installation, main queries the current backend for live session state.
If the backend is unavailable, confirmation explicitly says that sessions may
be interrupted. Main then uses the existing sidecar lifecycle shutdown path and
waits for bounded completion. A shutdown failure leaves the downloaded update
uninstalled and reports a retryable error.

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
  cached-download revalidation, confirmation cancellation, unavailable session
  state, shutdown failure, and install ordering;
- menu-template coverage for the command and hotkey-capture suppression;
- preload and renderer contract coverage for status/events and commands;
- Settings source/component coverage for the Updates section and action
  enablement; and
- a fixture-backed updater contract that verifies stable/nightly selection
  without contacting GitHub or requiring signed artifacts.

The existing `nightlyUpdateChannel.test.mjs` remains the transport-level
contract for electron-updater's custom channel. A real signed installed-build
update on macOS and Windows remains blocked until #510's credentials and
manual nightly validation are complete.

## Non-goals

- No unsigned nightly publication or release-workflow bypass.
- No automatic download, install-on-quit, downgrade, or channel switching.
- No updater implementation in the Rust sidecar or renderer-owned network
  access.
- No automatic harness resume or PTY continuity promise.
- No live artifact verification in this fixture-backed increment.
