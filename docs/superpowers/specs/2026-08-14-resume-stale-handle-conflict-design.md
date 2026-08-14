# Resume Stale-Handle Conflict Design

## Problem

The session list derives a non-live, resumable state from persisted metadata
when `lifecycle_phase` is `ended`, even if the in-memory session handle still
has a stale live status. The resume endpoint rejects that same session with
HTTP 409 because it only consults the handle lifecycle. The UI therefore
offers an action that its API cannot perform.

## Decision

Treat persisted `lifecycle_phase: ended` as the authority for replacing a
stale in-memory handle during resume. A handle whose own lifecycle is `ended`
continues to be resumable as before. A handle with a live lifecycle remains a
409 conflict unless the metadata for this exact session is already ended.

This preserves the existing protection against resuming an attached or truly
running session, while aligning the endpoint with the session-list projection.

## Data Flow and Error Handling

`resume_session` already reads the session metadata before it checks the
in-memory handle. It will carry whether that metadata is ended into the handle
guard. If the metadata is ended, the stale handle is replaced by the resumed
runtime; otherwise the endpoint returns 409 unchanged. Unsupported or missing
resume metadata retain their existing 400/404 responses.

## Testing

Extend the handler tests with a persisted-ended session paired with a stale
live handle. Assert that it no longer returns 409; use a deterministic resume
command that can start in the test environment, then assert the returned
session is running/creating as appropriate. Keep the existing attached-live
and detached-live 409 tests unchanged.

## Scope

No API-shape, UI, metadata-schema, or harness-command changes. The desktop
toast remains a fallback for genuine resume failures.
