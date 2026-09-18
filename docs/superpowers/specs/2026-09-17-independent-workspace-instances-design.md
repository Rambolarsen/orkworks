# Independent OrkWorks Workspace Instances

Status: proposed design; written-spec review required before implementation
Date: 2026-09-17
Tracking: [issue #541](https://github.com/Rambolarsen/orkworks/issues/541)
Related: [multi-workspace specification](../../../specs/multi-workspace.md),
[ADR 0056](../../adr/0056-one-sidecar-per-open-workspace.md),
[issue #545](https://github.com/Rambolarsen/orkworks/issues/545)

## Decision direction

Concurrent workspaces are represented by independent OrkWorks instances, not
one OrkWorks process managing multiple workspace sidecars.

Each instance owns at most one workspace and one sidecar lifecycle. An instance
does not discover, register, focus, monitor, or coordinate with other
instances. There is no cross-instance workspace registry, aggregate attention
count, shared focus coordinator, or peer-instance cleanup protocol.

The installation may run multiple independent instances concurrently. They use
the installation's persistent application-data root only for ordinary global
settings and path history; they do not use it to publish live instance state.
No global single-instance lock is used to collapse these independent
instances, and an instance never inspects processes to discover whether another
instance is active.

Selecting a different workspace in an instance closes the current workspace and
opens the selected workspace in that same instance. A second instance may happen
to have another workspace open, but the first instance neither knows nor cares.

## Installation workspace history

Recent-workspace history is tied to the OrkWorks installation and is persisted
in its application-data directory, surviving process exits and application
restarts. It is shared path history, not a live-instance registry.

The installation-scoped history file contains only:

```json
{
  "version": 1,
  "revision": 42,
  "lastWorkspacePath": "<canonical path or null>",
  "recentWorkspacePaths": ["<canonical path>"]
}
```

The file contains no process IDs, ports, leases, health flags, ownership
claims, peer identifiers, or open/closed state. `lastWorkspacePath` is only a
startup hint for the picker; it is not proof that a newly launched instance
owns the directory. Until the native process-tree ownership prerequisite is
proven for the target platform, startup must remain in the picker and require
an explicit workspace selection. If two instances receive the same hint, each
independently attempts to open it and an ordinary workspace-lease conflict is
shown; neither instance searches for or activates the other.

The history contract is:

- Canonical paths are used for identity; display paths are retained separately
  for presentation.
- A path is added only after the destination has completed a successful open.
- The selected path moves to the front without duplication.
- The list is bounded to 20 entries and 64 KiB serialized size; oldest entries
  are evicted when either bound is reached.
- Existing workspace metadata and settings are preserved when reopening a path.
- Explicit removal deletes only the path-history entry, never project files or
  workspace metadata.
- Every read-modify-write takes a short-lived advisory lock on the history
  store, rereads the current revision while holding the lock, increments the
  revision, writes a same-directory temporary file, flushes/closes it, and
  atomically replaces the history file. The lock protects only history-file
  updates; it is not a peer-instance or workspace-ownership mechanism.
- A missing history file means an empty picker. A corrupt or unreadable file is
  left untouched, loaded as empty in memory, and surfaced as a visible
  diagnostic. It is not silently overwritten until the user explicitly chooses
  to rebuild history.
- If an atomic write has an ambiguous result, read back and compare the
  expected revision and record. A confirmed write is accepted; a confirmed old
  record or an unreadable result leaves runtime state unchanged and shows a
  storage diagnostic. Path history is convenience state, not a reason to
  terminate a successfully opened workspace.
- Rendering or mutating this list never queries another OrkWorks instance.

## Instance and workspace state

An instance has these visible states:

- **Picker:** no active workspace or no sidecar has been adopted.
- **Opening:** a destination is being validated, leased, started, or restored.
- **Ready:** the sidecar owns the selected workspace and restoration completed.
- **Closing:** new work is gated while owned cleanup runs.
- **Unresolved:** cleanup or ownership proof failed; the instance cannot claim
  the workspace closed or start a replacement.

The instance owns at most one active state at a time. A failed or missing
destination never causes another remembered path to open automatically.

## Open and switch lifecycle

The switch is close-then-open within one instance, not a handoff between
workspaces:

1. Resolve and validate the selected directory before changing the current
   runtime. Validate canonical identity and preserve the display path.
2. Gate new session creation, resume, foreground submission, and analysis for
   the current workspace.
3. Perform bounded graceful cleanup, then bounded termination of processes
   owned by the current runtime.
4. Continue only after cleanup is acknowledged complete. If cleanup or
   ownership proof is unresolved, remain on the current workspace in the
   **Unresolved** state and do not start the destination.
5. Enter **Opening** with no active workspace. The destination sidecar may
   start in an unadopted state, but it must acquire the destination workspace
   lease before metadata loading, reconciliation, session adoption, or other
   workspace work. A lease conflict is reported as an external-owner conflict;
   the instance never inspects, takes over, or terminates that owner.
6. Restore the destination workspace. A successful open means that the lease
   was acquired, the sidecar reached readiness, and restoration completed.
7. After successful open, update installation history. A history-write failure
   keeps the workspace **Ready** and shows a storage diagnostic; it does not
   roll back a valid runtime.

The deterministic failure contract is:

- Validation failure leaves the current workspace unchanged.
- Cleanup failure leaves the current workspace **Unresolved**; no destination
  sidecar is started.
- Destination lease conflict, spawn failure, readiness timeout, or restoration
  failure leaves the instance in **Picker** with no adopted workspace. The
  failed destination runtime is cleaned up using only its owned handles. The
  previous workspace is never silently reopened.
- If failed cleanup leaves an owned descendant, the instance remains
  **Unresolved** and cannot report a successful switch or launch another
  replacement over it.
- A user may retry the destination or select another path from the picker only
  after the failed attempt has reached a terminal cleanup state.

On startup, read the installation's last selected path as a hint. If it is
missing, inaccessible, or conflicts with an external lease owner, show the
picker and start no adopted sidecar until the user selects a location. Do not
resume coding sessions automatically and do not fall back to the development
repository or home directory.

## Instance isolation

The design deliberately removes these requirements from multi-workspace scope:

- one Electron registry containing multiple sidecars;
- global focused-workspace identity;
- cross-workspace attention aggregation;
- background workspace terminal draining;
- peer-instance discovery or focus activation;
- shared Taskmaster analysis coordination across workspaces;
- cross-workspace quit, close, or resource-pressure orchestration.

Existing workspace leases still prevent two sidecars from owning the same
workspace metadata concurrently. That is an opaque ownership conflict for the
selected workspace, not peer-instance coordination. Canonical identity and
alias rules remain authoritative: equivalent path spellings must resolve to the
same workspace lease, while distinct Git worktrees remain distinct workspaces.
Lease acquisition must precede metadata adoption. A conflict is visible and
retryable, but never triggers takeover, peer inspection, or forced termination.

Global application settings and shared reporter installation retain their
existing ownership rules. This design does not make simultaneous mutation of
shared global settings safe by implication; those writes continue to use their
existing atomic/revision contracts and are not part of instance coordination.

## Quit, crash, and relaunch

Graceful quit of one instance gates new work, performs the same bounded cleanup
and ownership acknowledgement as switching, and reports failure if owned
processes survive. Repeated quit requests coalesce. A cancelled quit leaves the
instance and its current workspace unchanged.

Forced application termination and OS crashes cannot promise a history flush or
graceful session finalization. On relaunch, the new instance must not treat a
released metadata lease, persisted PID, process name, or path match as proof
that descendants are gone. It may adopt the remembered workspace only after
the platform ownership mechanism proves that the previous generation's owned
descendants have exited or remain contained by a generation that the new
instance is authorized to adopt. Otherwise startup remains in **Picker** or
**Unresolved** with an explicit cleanup error.

The implementation plan must include the #545 crash, immediate relaunch,
surviving-descendant, foreign-owner, and unsupported-platform cases.

## Process ownership prerequisite

Independent instances simplify orchestration but do not remove the
crash-surviving process-ownership prerequisite in issue #545. A sidecar crash
can still leave PTY, inference, or helper descendants unless a surviving owner
or OS containment boundary proves registration, bounded termination, and
complete exit.

The ownership plan must enumerate every process family that can outlive the
request that launched it. PTY and long-lived inference children are in scope.
Model discovery, version probes, Git, shell, and harness helpers may be
excluded only after native evidence proves both that they exit within their
bounded operation and that they cannot detach or spawn an unowned descendant.
Without that evidence, they use the same containment boundary or cause the
operation to fail closed.

No runtime implementation may claim successful unavailable-sidecar cleanup or
replacement adoption until the applicable platform mechanism is selected and
proven with native evidence. Unsupported platforms remain fail-closed.

## Authority transition and follow-up work

This document is the proposed replacement for the single-Electron,
multi-sidecar direction currently recorded by `specs/multi-workspace.md`, ADR
0056, and the original acceptance criteria for issue #541. Before any runtime
implementation begins, the implementation plan must:

- supersede or amend ADR 0056;
- update `specs/multi-workspace.md` and its validation plan so they no longer
  require a registry, background sidecars, aggregate attention, or cross-workspace
  quit orchestration;
- rewrite issue #541's acceptance criteria to match independent instances;
- retain #545 as a prerequisite for crash-surviving ownership, not as evidence
  that a released lease proves descendant cleanup.

Until that authority transition is recorded, the older multi-sidecar documents
remain historical proposals and must not be used as implementation authority.

## Acceptance examples

- Open workspace A in instance 1, then select workspace B: instance 1 closes A
  and opens B; it never checks whether instance 2 has A or B open.
- Restart the OrkWorks installation: its recent list and last selected path
  remain available from the persistent application-data directory.
- Launch instance 2 while instance 1 is running: both may read and update path
  history through the short-lived history lock, but neither learns the other's
  live workspace or process state.
- Open workspace A in instance 2 while instance 1 owns A: the workspace lease
  reports a conflict; instance 2 does not terminate or inspect instance 1.
- Open A, B, and C sequentially in one instance: the installation history
  remembers all three within the 20-entry bound while only the selected
  workspace has a live sidecar.
- A cleanup survivor keeps the instance **Unresolved** and prevents B from
  starting.
- A destination that reaches readiness but fails restoration is cleaned up and
  leaves the instance in **Picker**; A is not silently reopened.
- A corrupt history file remains available for recovery and produces a visible
  diagnostic rather than being overwritten automatically.
- A crashed sidecar followed by immediate relaunch cannot adopt the workspace
  until native ownership evidence proves the previous generation is gone or
  safely contained.

## Non-goals

- Managing multiple sidecars from one Electron registry.
- Providing a global workspace switcher or dashboard.
- Automatically resuming sessions in another instance or workspace.
- Creating, deleting, or managing Git worktrees.
- Using persisted PIDs, process names, executable paths, or released metadata
  locks as proof of process ownership.
