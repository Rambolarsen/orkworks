# Runtime Recovery Latest-Fix Design

## Goal

Close the final-review findings on `fix-runtime-recovery` without changing the
existing renderer recovery document or the user's pre-existing report.

## Design

Electron main will publish the fully restored workspace snapshot as part of the
validated `ready` lifecycle event. The preload boundary will validate the
workspace shape and port range. The renderer will synchronously hand that
snapshot to the workspace/session controller before setting backend status to
connected; the controller will invalidate stale work, publish the new identity,
clear stale sessions, and refresh the new backend under its existing generation
guards. The normal workspace-switch result will use the same adoption path, so
the renderer will not POST the workspace a second time.

Automatic recovery will count the stable generation's eventual failure as the
first launch in its sequence. It will retain the first configured backoff delay
and permit only two replacement launches, for three launches total. Generation
replacement and cancellation behavior remain unchanged.

The sidecar lifecycle will reject an announced port unless it is an integer in
`1..=65535`, and will cap pre-readiness stdout retention while preserving marker
detection. Renderer diagnostic messages will redact arbitrary absolute POSIX
paths while retaining non-path metadata, URLs, and useful error text.

CenterPanel will own a local attach-unavailable state. A readiness rejection
will show an `EmptyState` Retry action and will not escalate a stale renderer
lookup into the global recovery overlay. Retry will call the existing
`retryBackend` bridge, re-run attachment only after successful readiness, and
ignore stale results after cancellation/unmount.

The macOS provider regression will synchronize several real `ProcessRunner`
invocations with a barrier, asserting every invocation completes successfully.

## Testing

Each behavior starts with a failing focused test: controller adoption after a
failed switch and retry, lifecycle launch budget and invalid ports, diagnostic
redaction, CenterPanel recovery helper behavior, and concurrent macOS spawning.
Existing generation, preload validation, terminal, and provider tests remain in
the focused and full suites.
