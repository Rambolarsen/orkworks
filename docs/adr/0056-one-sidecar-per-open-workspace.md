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
accepted, clarify ADR 0054's active-workspace analysis wording to mean focused
workspace; it does not change that ADR's managed-policy decision.
