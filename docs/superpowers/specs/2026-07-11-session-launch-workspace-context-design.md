# Session Launch Workspace Context Design

## Problem

`create_session` launches a new terminal and detects its Git context from the
sidecar process's current directory. The active workspace is already held in
`AppState` and used when persisting session metadata. When those paths differ,
the session's `cwd`, repository root, and branch can describe a different
repository from the active workspace.

## Decision

Resolve the new session's working directory from the active workspace path
when a workspace is selected. If no workspace is selected, retain the existing
best-effort current-directory fallback, then `/` if that lookup fails.

The resolved directory remains the single input to both harness launch
resolution and Git detection. This keeps the live session information and its
persisted metadata aligned without changing the HTTP request, IPC contract, or
metadata schema.

## Alternatives considered

- Reject creation when no workspace is active. This would remove the existing
  fallback behavior and make the app less resilient during startup.
- Change the sidecar process directory globally when the workspace changes.
  This creates process-wide mutable state and can affect unrelated operations.
- Use the active workspace only at session creation. This is the narrowest
  change and aligns with the workspace-scoped session model.

## Testing

Add an async handler test that creates a temporary Git repository, installs it
as the active workspace in test state, creates a session, and asserts that the
response reports the repository as its `cwd` and Git root, with its current
branch. The test must fail against the current behavior because the sidecar's
own directory is used instead.

## Scope

This is a sidecar-only bugfix in `crates/orkworksd/src/http/session_handlers.rs`.
It does not alter workspace selection, session persistence format, harness
commands, or renderer behavior.
