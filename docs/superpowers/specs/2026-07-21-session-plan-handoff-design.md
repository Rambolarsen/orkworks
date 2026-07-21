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

Add an optional `plan_path` to the session metadata and its API projection. It
is a path relative to the open workspace, for example:

```json
{
  "attention": "needs_you",
  "plan_path": "docs/superpowers/plans/2026-07-21-example.md"
}
```

The field is advisory session context, not a workflow command. A session can
provide it before or after it transitions to `needs_you`, but the product
expectation is that an agent adds it when asking the user to review its plan.

### User experience

The existing session `needs you` state remains the sole review prompt. In that
session's Details panel, show **Open plan** only when `plan_path` resolves to a
valid Markdown file within the active workspace. Selecting it hands the file
to the operating system, allowing the user to read it in their configured
editor or browser.

There is no standalone panel or notification. The user deliberately selects
the session first, preserving its terminal, branch, and activity context.

### Safety and failures

The sidecar resolves `plan_path` against the workspace root and rejects paths
that escape it, are not regular files, or are not Markdown. The renderer never
opens a user-provided path directly; it requests the sidecar/Electron handoff
through a narrow IPC/API action.

If the file has moved, been deleted, or cannot be opened, the action reports a
clear non-destructive error and the session remains unchanged. The action is
hidden for missing or invalid paths when known in advance; a race after render
is handled on click.

## Data flow

```text
agent/session metadata writes plan_path + needs_you
  -> sidecar loads and projects plan_path with SessionInfo
  -> Details validates availability and renders Open plan
  -> user selects Open plan
  -> narrow IPC/API resolves and validates workspace-relative path
  -> Electron delegates file to the OS default handler
```

## Testing

- Metadata/API tests retain and expose a valid optional `plan_path`.
- Path-validation tests reject absolute paths, workspace escapes, non-Markdown
  files, and missing files.
- Renderer tests show **Open plan** only for a valid session plan and invoke
  the bridge with the session identifier.
- Electron/bridge tests verify that only the validated handoff path reaches
  the OS opener; tests stub the opener rather than launching an external app.

## Acceptance criteria

- [ ] A session can report one repo-relative Markdown plan path.
- [ ] A session with a valid path exposes **Open plan** in Details.
- [ ] Opening a plan uses the default OS editor/browser handler.
- [ ] Invalid, missing, or workspace-escaping paths are never opened.
- [ ] No review queue, Markdown viewer, automatic discovery, or review-state
      model is introduced.
