# Single-writer workspace lease for the sidecar

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-09-07

## Context

The sidecar persists workspace and session metadata globally, but a replacement
sidecar previously had no way to distinguish an active owner from a stale
process. It could reconcile sessions written as running by the first sidecar
as orphaned, even while their harness processes were still alive. A later
restore then collided with those surviving harnesses.

## Decision

`orkworksd` acquires an exclusive operating-system advisory lease on
`.sidecar.lock` in the workspace's global metadata directory before loading,
migrating, or reconciling persisted session state. The lease is owned by the
in-memory `WorkspaceState` and is released automatically when that state or
the sidecar exits. The lock file remains on disk; ownership is represented by
the OS lock, not by file existence or a stale PID.

If another sidecar owns the workspace, opening or switching to it returns a
conflict and does not reconcile its persisted sessions.

## Consequences

Only one sidecar may mutate or reconcile a workspace's metadata at a time,
preventing replacement races and making the failure actionable to the caller.
An ungraceful sidecar exit does not strand the workspace because the OS
releases the advisory lock. The lease does not prove that individual harness
processes survived a sidecar crash; process-level reconciliation remains a
separate concern.
