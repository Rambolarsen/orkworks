# Runtime Recovery Fix-Wave Design

## Goal

Close the actionable findings from the whole-branch review of the runtime
recovery work without changing the existing workspace/session recovery
contract or touching the pre-existing `.superpowers/sdd/task-1-report.md`
edit.

## Design

### Renderer diagnostics

`main.ts` will pass renderer events through a pure projection that returns only
the stable diagnostic fields needed by operators: event type, numeric load
error code, sanitized reason, URL origin, process reason, and numeric exit
code. The console-message event will record only the event type, not the
renderer message. Tests will assert both the exact allowlisted shape and the
absence of prompt, token, path, and arbitrary payload data.

### CenterPanel recovery

The terminal attach effect will own a local `unavailable` state for failures
while obtaining the backend URL. It will use an effect-local cancellation
flag, handle rejected URL lookup, and ignore stale success or failure
continuations after cleanup. While unavailable it will render the shared
`EmptyState` with a Retry action that calls
`window.orkworks.retryBackend()`, matching the app-level backend recovery
bridge. A pure attach-result helper will make the cancellation and fallback
decision behavior-testable without mounting Electron or xterm.

### Sidecar retry accounting

The attempt counter represents the launch that can fail, including a launch
that was stable before it failed. Stability reset will therefore retain one
attempt rather than setting the counter to zero. The next automatic recovery
will select the first configured backoff delay, and subsequent failures will
continue through the bounded attempt budget. Fake-timer tests will assert the
actual scheduled delays before and after stability reset.

### Provider process spawning

The existing plain `Command::spawn()` provider runner remains the safe
cross-platform implementation. The architecture prose will describe piped
stdio and standard child descriptor handling instead of claiming Unix
`setsid()` or inherited-FD closure. The provider test module will retain the
missing-executable invariant and add a macOS-only test that invokes the real
runner concurrently from multiple threads, confirming successful completion
and no parent-process abort. The same real-runner behavior remains covered on
other platforms by the existing platform-neutral test.

### Behavior-level lifecycle coverage

The lifecycle tests will exercise public module behavior—state transitions,
readiness, retry delays, and generation invalidation—rather than inspecting
source text. Pure diagnostic and renderer attach helpers will be tested
directly. Electron registration and React rendering source checks will remain
only where the current test environment cannot instantiate those runtime
objects; lifecycle assertions that can use the pure seams will move out of
regex tests.

### Small cleanup

The unused `fetch` dependency will be removed from the sidecar lifecycle
options and its callers/tests. The recovery document wording will explicitly
describe its `location.replace(originalUrl)` action, and the touched files
will be normalized to pass `git diff --check`.

## Verification

Use the Node test runner for the focused renderer, diagnostic, lifecycle, and
wiring tests; run the desktop type-check and build; run the full desktop test
suite; run the focused and full Rust test suites; run `cargo fmt --check`, the
repository doc-currency check, the worktree check, and `git diff --check`.

## Self-review

- No placeholders or unresolved choices remain.
- All six requested findings and the valid suggestions have a named design
  response.
- The design keeps the existing preload retry bridge and bounded recovery
  state machine intact.
- The pre-existing task report is explicitly outside the change scope.
