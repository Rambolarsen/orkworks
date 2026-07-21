# Session Plan Handoff Design

## Goal

Let a user read a plan that needs review without losing the session context
that produced it. When a session already signals `needs_you`, OrkWorks should
offer a direct way to open that session's plan in the user's normal editor or
browser.

## Scope

In scope:

- An optional, repo-relative `plan_path` reported with session metadata.
- A session-detail **Open plan** action when the session has a valid plan path.
- Opening the plan through the operating system's normal file handler.
- Validation that prevents a reported path from escaping the open workspace.

Out of scope:

- A review queue, inbox, or review history.
- Automatic plan/spec discovery, filesystem watching, or plan digests.
- Rendering or editing Markdown inside OrkWorks.
- Review state, dismissal, re-queueing, or plan revision tracking.
- Changing the existing `needs_you` lifecycle or its presentation.

## Design

### Metadata contract

Add an optional `plan_path` to session metadata. An agent supplies it through
the existing attention report endpoint as `planPath`, alongside the status
that makes the session need review. A supplied string replaces the current
path; an explicit JSON `null` clears it; omission leaves it unchanged. The
metadata merge persists the attention signal and this update atomically.

The attention-report request is, for example:

```json
{
  "status": "waiting_for_input",
  "planPath": "docs/superpowers/plans/2026-07-21-example.md"
}
```

The field is advisory session context, not a workflow command. A session can
provide it before or after it transitions to `needs_you`, but the product
expectation is that an agent adds it when asking the user to review its plan.
Only files whose extension is `.md`, case-insensitively, qualify for v1.

### User experience

The existing session `needs you` state remains the sole review prompt. In that
session's Details panel, show **Open plan** only when the sidecar-derived
`hasOpenablePlan` projection is true. Selecting it requests opening by session
ID; the renderer never receives or submits a file path. Electron delegates the
validated file to `shell.openPath`, which uses the operating system's configured
file handler (normally the user's editor or browser association).

There is no standalone panel or notification. The user deliberately selects
the session first, preserving its terminal, branch, and activity context.

### Safety and failures

The sidecar resolves `plan_path` against the workspace root and rejects paths
that escape it, are not regular files, or are not Markdown. It canonicalizes
both the workspace root and candidate file before checking containment, so an
in-workspace symlink cannot target a file outside the workspace. The check is
repeated immediately before opening to limit time-of-check/time-of-use races.

The renderer invokes a narrow `openPlan(sessionId)` preload bridge. Electron
calls a sidecar `POST /sessions/:id/open-plan` handoff endpoint, which resolves
and validates that session ID and returns only the freshly validated canonical
file path. Electron then calls `shell.openPath`. A non-empty error returned by
`shell.openPath` is shown as a user-facing failure. The renderer never opens a
user-provided path directly.

If the file has moved, been deleted, or cannot be opened, the action reports a
clear non-destructive error and the session remains unchanged. The action is
hidden for missing or invalid paths when known in advance; a race after render
is handled on click.

## Data flow

```text
agent reports needs_you + planPath to existing attention endpoint
  -> sidecar atomically persists plan_path and projects hasOpenablePlan
  -> Details renders Open plan when hasOpenablePlan is true
  -> user selects Open plan
  -> preload sends only session ID to Electron
  -> sidecar canonicalizes and validates path immediately before handoff
  -> Electron delegates file to the OS configured handler
```

## Testing

- Attention-report tests cover set, replace, clear, and omit semantics for
  `planPath`, including atomic persistence with the attention update.
- Path-validation tests reject absolute paths, workspace escapes, symlink
  escapes, non-Markdown files, and missing files.
- API/renderer tests expose `hasOpenablePlan`, show **Open plan** only when it
  is true, and invoke the bridge with the session identifier alone.
- Electron/bridge tests verify that a freshly validated canonical path reaches
  `shell.openPath`; tests stub the opener rather than launching an external app.

## Acceptance criteria

- [ ] A session can report one repo-relative Markdown plan path.
- [ ] A session with a valid path exposes **Open plan** in Details.
- [ ] Opening a plan uses the default OS editor/browser handler.
- [ ] Invalid, missing, or workspace-escaping paths are never opened.
- [ ] No review queue, Markdown viewer, automatic discovery, or review-state
      model is introduced.
