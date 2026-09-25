# Independent workspace instances

- Status: accepted
- Deciders: owner
- Date: 2026-09-17

## Context

OrkWorks needs to remember and reopen more than one workspace without turning a
single desktop process into a coordinator for many live workspace runtimes. ADR
0056 proposed one Electron-owned registry containing one sidecar per open
workspace, with background execution, aggregate attention, and application-wide
close and quit coordination. That model added live peer state and cross-workspace
authority that are not required for deliberate workspace switching.

The existing sidecar, metadata lease, terminal ownership, and Taskmaster state
are already scoped to one workspace. Keeping that boundary at the application
instance level preserves failure isolation and makes each instance independently
understandable.

## Decision

Concurrent workspaces are represented by independent OrkWorks application
instances. Each instance owns at most one active workspace, one sidecar
lifecycle, and one visible session context. It does not discover, register,
focus, monitor, or clean up another instance. There is no live instance registry,
peer-activation protocol, cross-instance attention rollup, or app-wide workspace
shutdown coordinator.

Selecting another workspace in one instance is a deterministic close-then-open
operation:

1. Resolve and validate the destination while leaving the current workspace
   usable.
2. Gate new work and completely clean up the current instance's owned runtime.
3. If cleanup or ownership proof is unresolved, report the instance as
   unresolved and do not start the destination.
4. Start and restore the destination only after the old runtime's cleanup is
   acknowledged. Acquire the destination workspace lease before metadata
   adoption or reconciliation.
5. If destination startup, lease acquisition, readiness, or restoration fails,
   clean up only that attempted runtime and show the picker. Do not silently
   reopen the previous workspace.
6. Record the destination in installation history only after a successful open.
   A history-write failure does not roll back a ready workspace.

The installation keeps a bounded, revisioned history containing canonical
workspace paths only: a nullable last-workspace hint and a recent-path list.
This durable history survives application exits and is shared through a
short-lived storage lock and atomic replacement. It contains no process IDs,
ports, leases, health, ownership, peer identity, or open/closed state. Reading
or updating it never queries another instance. A remembered path is a startup
hint, not proof that a workspace is available or owned.

Existing workspace metadata leases remain authoritative. If another instance or
external sidecar owns the selected workspace, opening fails with an opaque,
retryable ownership conflict. The requesting instance does not inspect, focus,
take over, wait for, or terminate the owner. Equivalent filesystem aliases must
resolve to the same workspace identity; distinct Git worktrees remain distinct.

Graceful switch, close, and quit operations clean up only the current instance's
owned runtime. Forced termination and crash recovery cannot infer descendant
exit from persisted PIDs, process names, executable paths, or a released metadata
lease.

Issue [#545](https://github.com/Rambolarsen/orkworks/issues/545) is closed, but
its closure does not itself provide the native evidence required for
unavailable-sidecar cleanup and replacement or relaunch adoption. That
evidence remains a hard prerequisite. Every process family that can outlive its
launch request must be registered with a crash-surviving native ownership
boundary, or be proven by native evidence unable to detach or leave descendants.
Unsupported platforms fail closed. No runtime implementation may claim
successful cleanup or launch a replacement until the applicable mechanism
proves bounded, generation-specific exit while preserving foreign processes.

The detailed proposed behavior and executable acceptance contract live in the
[independent workspace specification](../../specs/multi-workspace.md) and its
[validation plan](../validation/multi-workspace.md).

## Alternatives considered

- The ADR 0056 model, with one desktop registry and one sidecar per open
  workspace, would preserve background execution in one window but requires
  shared focus authority, cross-workspace routing, attention aggregation, and
  coordinated cleanup. Those responsibilities are deliberately removed.
- One sidecar hosting several workspaces would weaken the current
  workspace-scoped lease, metadata, request, and failure boundaries.
- A live peer-instance registry would allow activation of an existing instance,
  but would turn path history into process-discovery and ownership state. The
  installation history is intentionally path-only.
- Automatically reopening the previous workspace after a failed destination
  open would hide the close-then-open result and could overlap an unresolved
  generation. Failure instead returns to the picker.

## Consequences

Task 2 storage implementation uses `fs-ext` (`flock` on Unix, `LockFileEx` on
Windows) on a retained installation-local lock file. The OS releases the lock
when its descriptor closes or its process exits. A five-second age threshold
is not evidence of abandonment and cannot authorize eviction. The native addon
requires an Electron ABI rebuild for desktop execution; Node tests use the host
Node build. This implements the existing short-lived advisory-lock decision.

Multiple workspaces can remain active only by running multiple independent
OrkWorks instances. One instance has no consolidated dashboard, attention count,
resource policy, focus control, or quit behavior for the others. Closing or
crashing one instance does not authorize operations against another.

The implementation can retain the existing one-workspace sidecar and lease
model. Workspace switching becomes a serial state machine with explicit picker,
opening, ready, closing, and unresolved states. Installation history needs
concurrency-safe writes because independent instances can update the same path
list, but its lock is never an instance coordinator or ownership mechanism.

ADR 0013's single-active-context rule, ADR 0022's runtime-owned PTY lifetime,
and ADR 0052's single-writer workspace lease remain in force. ADR 0056 is
superseded as implementation authority but retained, including its 2026-09-15
evidence amendment, as the historical record of the rejected multi-sidecar
proposal and the incomplete ownership evidence that still motivates #545.

## Amendment — 2026-09-25: bounded master-plan child runtimes

This amendment records the narrow parent/child control-plane exception in the
[master-session parallel runner design](../superpowers/specs/2026-09-25-master-session-parallel-runner-design.md).
The one-workspace-per-instance decision remains in force: a master sidecar
continues to own only its workspace metadata, and each dedicated child sidecar
owns only its assigned worktree workspace. A master plan may maintain a
durable, plan-scoped association with child runtimes it launched after the
user approved one immutable plan revision. That association is not a peer
registry and grants no discovery, focus, attention, or control over unrelated
OrkWorks instances.

The parent/child broker may authorize only the launch, report, pause, recovery,
and cleanup operations declared by the approved plan. It uses distinct
master and child capabilities and server-owned runtime/allocation identities;
it does not reuse workspace metadata authority across sidecars. Before any
child launch, pause acknowledgement, recovery, or cleanup can claim success,
the applicable platform must provide enforceable worktree confinement and
generation-specific proof that the complete owned process tree has exited.
Unsupported or ambiguous ownership fails closed. This amendment does not
authorize multi-workspace metadata ownership, a peer-instance registry,
cross-workspace focus, or general workflow control.
