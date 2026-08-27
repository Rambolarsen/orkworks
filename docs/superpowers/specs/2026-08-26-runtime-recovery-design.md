# Runtime Recovery Design

## Context

The live OrkWorks instance produced repeated `orkworksd` aborts on macOS while
Peon spawned a provider process. The crash report identifies
`std::process::Command::spawn` and the provider runner's Unix `pre_exec` hook,
which performs fork-time operations that are not safe in a multithreaded
process. Electron then keeps the renderer attached to the dead sidecar. A
white window also needs renderer-level diagnostics and a bounded recovery path.

## Goals

- Prevent provider subprocess startup from aborting `orkworksd` on macOS.
- Preserve provider invocation behavior and PTY isolation as far as the
  platform safely permits.
- Detect sidecar exit/error in Electron main and make the backend state
  recoverable without an infinite restart loop.
- Detect renderer load/process failures and present a recoverable state instead
  of silently leaving a blank window.
- Add focused regression coverage for the failure boundaries.

## Non-goals

- Rework provider fallback policy or Peon inference behavior.
- Add a general-purpose process supervisor.
- Change the renderer layout or product UI beyond failure/recovery states.
- Automatically restart indefinitely or discard user workspace/session data.

## Design

### Safe provider spawning

The provider runner will remove the Unix `pre_exec` hook entirely. Its
`setsid`, `sysconf`, and descriptor sweep all force a fork-time callback in a
multithreaded daemon; the macOS crash report shows that path aborting before
the provider starts. Provider invocations already use piped stdin/stdout/
stderr, and `Command` owns the normal close-on-exec handling for its child
process descriptors. This preserves the relevant invariant for this runner:
providers do not receive the user's PTY or arbitrary parent descriptors.

Provider spawn failures remain ordinary `InvocationResult` failures, allowing
fallback providers and keeping the sidecar alive.

The existing runner abstraction remains the test seam. Tests will exercise the
real process runner with a successful platform-neutral command and a missing
executable. A macOS-specific subprocess test will run the real runner from a
multithreaded parent and verify that the parent remains alive; on other
platforms the equivalent real-runner tests still protect fallback behavior.

### Sidecar lifecycle recovery

Electron main will centralize sidecar startup and exit handling. Every sidecar
start receives a monotonically increasing generation. Only the current
generation may publish a port, clear the active process, or notify the
renderer; late events from a replaced process are ignored.

Readiness is a generation-specific promise. Spawn errors, pre-ready exits,
and a readiness timeout reject it and clear the active port. Post-ready exits
invalidate the port and send a renderer lifecycle event that changes the
renderer to an unavailable/retryable state. No API handler waits forever.

Automatic recovery uses a small state machine: `starting → ready → failed →
retrying → exhausted`. It makes at most three attempts with increasing delays,
allows only one recovery sequence at a time, resets the attempt counter after
a stable ready period, and stops at `exhausted` until the user explicitly
retries. A replacement sidecar restores the remembered `workspacePath` and
reapplies persisted provider/retention settings before publishing ready.
Workspace switches cancel the old generation and use the same centralized
startup path.

### Renderer failure recovery

The `BrowserWindow` will record `did-fail-load` and `render-process-gone`
events, plus short, structured console diagnostics without arbitrary message
payloads. The renderer will handle the sidecar lifecycle event and show an
unavailable/retry state while the document is alive.

For failures before React mounts, or after the renderer process is gone,
Electron main will load a minimal local recovery document with a single
user-triggered reload/retry action. It will not automatically reload forever.
The fallback is deliberately independent of React and the preload bridge so
it can render when the normal application document cannot.

## Error handling

- Provider subprocess failures are non-fatal and participate in existing
  fallback behavior.
- Sidecar failures are logged with exit/error details and transition the
  renderer to an unavailable/retryable backend state.
- Renderer failures are logged with the failure reason and surfaced through a
  recovery UI when the document can still render.
- Console diagnostics are truncated/structured and do not log arbitrary
  renderer payloads, prompts, workspace contents, or tokens.
- All retries are bounded or user-triggered; no loop may continue indefinitely
  without delay or a state change.

## Verification

- Rust provider tests and the complete sidecar test suite.
- Desktop type-check, renderer/Electron tests, and production build, including
  lifecycle tests for pre-ready exit, post-ready exit, spawn error, stale
  generation events, workspace restoration, bounded retry exhaustion,
  `did-fail-load`, and `render-process-gone`.
- Manual/reproducible process-failure checks where the environment permits.
- Update `docs/agents/architecture.md` for the new sidecar lifecycle and
  recovery contract.
- Repository doc-currency and worktree checks before handoff.
