# One sidecar per open workspace

- Status: proposed
- Deciders: owner; written-spec review pending
- Date: 2026-09-13

## Context

The owner wants infrequent, deliberate switching among remembered workspaces,
while sessions in other opened workspaces keep running. The current Electron
integration replaces its only sidecar on workspace switch. The sidecar already
owns one workspace's metadata lease and detached session runtimes. The desktop
must preserve the single-active-context rule from ADR 0013.

## Decision

Propose an Electron-owned registry with one sidecar lifecycle per open canonical
workspace and a separate focused-workspace identity. Reuse existing lifecycle
and restoration logic. Bind requests, credentials and recovery to the owning
workspace generation. Background workspaces continue session execution and
observation, while only the focused workspace holds Taskmaster analysis
permission. Share installed definitions, executable trust, reference knowledge,
and the durable analysis budget; retain workspace-specific workflow state and
settings overrides.

Startup opens only the last focused workspace. Closing a workspace with live
sessions requires confirmation; explicit app quit confirms once across affected
workspaces. Shutdown must cover every owned runtime without affecting external
owners. The [proposed specification](../../specs/multi-workspace.md) is the
complete behavior and validation contract. This ADR does not claim implementation.

## Alternatives considered

- One sidecar hosting multiple workspaces would share some resources directly,
  but changes the existing workspace/session ownership and request assumptions
  throughout the backend and increases the impact of a process failure.
- Separate application windows would fragment the focused switching experience
  and provide no aggregate switcher in the user's current context.

## Consequences

Per-workspace failure isolation and metadata ownership reuse existing mechanisms.
The application must coordinate lifecycle routing, settings refresh, shared
reporter writes, cancellation and all-process shutdown. Background memory and
inference load grow with open workspaces and require measurement. A sidecar
process boundary does not make shared global settings or caches coherent.

ADR 0022 (runtime-owned PTYs), ADR 0013 (single active context), and ADR 0052
(one metadata owner per workspace) remain in force. When this proposal is
accepted, coordinate the focused-workspace qualification in specs/taskmaster.md,
specs/taskmaster-knowledge.md, and ADR 0042's deterministic correlation clause,
following the ADR amendment/supersession sequence. ADR 0054 remains the
managed-CLI-policy decision.

Analysis handoff requires destination readiness, confirmed old-generation
revocation (or process exit), and generation/epoch-bound commands. Commit and
publish focus before activating the destination; failed activation keeps that
destination selected and is reconciled without rollback. Pre-commit failure
preserves prior focus. Neither path can grant a second analysis owner.
Existing sidecar-owned PTY handles do not prove cleanup after a sidecar crash.
Native crash-surviving containment/supervision must be selected, demonstrated,
and recorded here before implementing unavailable-runtime cleanup or recovery;
the specification defines the required registration and exit acknowledgements,
including cleanup after unexpected Electron exit before relaunch adoption.
