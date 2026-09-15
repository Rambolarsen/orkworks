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
Acceptance also coordinates the release-pipeline shutdown contract, ADR 0048's
foreground prompt-submission boundary, and ADR 0034/specs/session-plan-review.md.
One lifecycle-operation coordinator owns focus/close/quit/install and registry
admission; stale generated-prompt handoffs cannot write after focus
revocation. Focus storage requires atomic publication and uncertain-outcome recovery.
Clear durable last focus before terminating an explicitly closed focused workspace;
failed replacement persistence cannot restore that closed workspace on restart.
Failed quit cleanup retains unresolved ownership but releases global coordination
after reconciling unaffected runtimes, allowing continued use or explicit retry.

An OS advisory coordinator lease scopes one desktop registry/memory writer to
one application-global data root, including launches with different userData.
Electron single-instance delivery raises the existing window; a second registry
cannot grant focus. Foreground prompt grants are explicit and independently
acknowledged, even when Taskmaster analysis is disabled or unavailable. Ordinary
already-sent keystrokes retain their original terminal target across a switch.

Directory identity coalesces aliases and validates each open attempt. Remembered
locations retain canonical-path metadata semantics; no persistent object-identity
migration or atomic sandbox against concurrent root replacement is included.
Changing the root while open is unsupported and detected changes require reopen.

Analysis handoff requires destination readiness, confirmed old-generation
revocation (or process exit), and generation/epoch-bound commands. Commit and
publish focus before activating the destination; failed activation keeps that
destination selected and is reconciled without rollback. Pre-commit failure
preserves prior focus once storage outcome is confirmed. Neither path can grant
a second analysis owner. Initial focus uses adoption/activation without source
revocation; crash recovery also waits for old inference exit proof before activation.
Existing sidecar-owned PTY handles do not prove cleanup after a sidecar crash.
Native crash-surviving containment/supervision must be selected, demonstrated,
and recorded here before implementing unavailable-runtime cleanup or recovery;
the specification defines the required registration and exit acknowledgements,
including cleanup after unexpected Electron exit before relaunch adoption.

## Amendment — 2026-09-15: process-ownership proof remains an open gate

Task 7 records the evidence required by the proposed multi-workspace decision.
The fixture demonstrated a private Windows Job Object with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, suspended registration before target
execution, retained creation-time identities, independent Job Object census,
breakaway rejection, non-inheritable authority handles, and actual
`TerminateProcess` termination of the Electron-like parent. The hosted native
run passed all 8 of 8 `windows_job` tests on Windows Server 2025 x64 MSVC.

This is fixture evidence, not production integration and not a cross-platform
selection. The Task 4 ProcessGroup, RegisteredRoot, and launchd candidates did
not produce a native macOS proof; portable Linux native runtime and descriptor
launch evidence are also absent. The Task 4 candidate tests are
vacuous/re-scoped because they fail closed before native launch; their labels
must not be generalized into a Unix ownership claim. Admission-substitution
isolation/reliability remains fixture-bounded. Task 5 retained its 14 skipped
launch-dependent rows, the acknowledgement-concurrency gap (the paused-launch
latch test does not close it), and the escaped-JSON 64 KiB framing gap. Task 6
found the production Electron, PTY, provider, inference, discovery, and
harness roots outside this boundary.

Accordingly, the ADR remains `proposed`. No production recovery, replacement,
or adoption behavior is authorized by this amendment. Before this proposal can
support those behaviors, a native Unix mechanism must pass the complete matrix
and every production root must route through the demonstrated owner protocol;
otherwise the result remains unresolved rather than an empty generation.
