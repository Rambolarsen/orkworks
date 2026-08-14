# Resume Stale-Handle Conflict Design

## Problem

The session list derives a non-live, resumable state from persisted metadata
when `lifecycle_phase` is `ended`, even if the in-memory session handle still
has a stale live status. The resume endpoint rejects that same session with
HTTP 409 because it only consults the handle lifecycle. The UI therefore
offers an action that its API cannot perform.

## Decision

Treat persisted `lifecycle_phase: ended` plus the absence of a tracked PTY PID
as the authority for replacing an unattached stale in-memory handle during
resume. Runtime exit clears the session PID before finalization records
`ended`, so these two facts establish that the old runtime has exited. A
handle whose own lifecycle is `ended` continues to be resumable as before.

Terminal attachment remains an unconditional 409 conflict. An unattached
handle with a live lifecycle remains a 409 conflict unless its metadata is
ended and it has no tracked PID. This prevents replacement of a genuinely
running detached PTY, whose driver could otherwise later mutate the newly
inserted handle under the same session ID.

This preserves the existing protection against resuming an attached or truly
running session, while aligning the endpoint with the session-list projection.

## Concurrent Admission

Resume admission is per-session and atomic. `SessionHandle` records an
internal runtime-ownership claim. The endpoint constructs and installs the
claimed replacement under the existing sessions mutex, whether it is replacing
a stale handle or starting from no handle. A second request therefore sees the
claim and receives HTTP 409 even if it read old ended metadata before the first
request persisted `creating`.

The claim remains set for the live runtime and is cleared only when terminal
finalization records its terminal state. That prevents delayed requests holding
a stale metadata snapshot from launching a second PTY. This field is
runtime-only: it does not change lifecycle vocabulary, metadata, or the public
API.

## Data Flow and Error Handling

`resume_session` already reads the session metadata before it checks the
in-memory handle. It will carry whether that metadata is ended into the handle
guard and consult `session_pids` there. If the handle is unattached, the
metadata is ended, and no PID is tracked, the stale handle is replaced by the
resumed runtime; otherwise the endpoint returns 409 unchanged. Unsupported or
missing resume metadata retain their existing 400/404 responses.

The same sessions-mutex critical section rejects an existing claim before
admitting a request, then atomically installs a claimed replacement. This
closes the gap between validation and replacement for both stale-handle and
no-handle sessions.

## Testing

Extend the handler tests with a persisted-ended session paired with an
unattached stale live handle and no tracked PID. Assert that it no longer
returns 409; use a deterministic resume command that can start in the test
environment, then assert the returned session is running/creating as
appropriate. Keep the existing attached-live and detached-live 409 tests
unchanged, and add a tracked-PID case if the existing detached-live fixture
does not already cover it.

The endpoint regression uses a stale handle whose in-memory lifecycle remains
live, matching the reported mismatch. A barrier-controlled concurrent
admission regression makes two callers attempt the same ended, PID-free
session and asserts exactly one succeeds, one conflicts, and only one claimed
runtime is installed.

## Scope

No API-shape, UI, metadata-schema, or harness-command changes. The desktop
toast remains a fallback for genuine resume failures.
