# Independent workspace instances

Status: proposed; written-spec review required before implementation
Date: 2026-09-17
Tracking: [issue #541](https://github.com/Rambolarsen/orkworks/issues/541)

## Purpose and authority

Let an OrkWorks installation remember several workspace locations and let a
user run separate workspaces concurrently without making one application
process coordinate them. Each OrkWorks instance owns at most one workspace,
one sidecar lifecycle, and one visible session context, preserving the
single-active-context rule in
[ADR 0013](../docs/adr/0013-single-active-context-primitive.md).

This is a proposed extension to the [MVP](orkworks-mvp.md). The active
architecture decision is [ADR 0060](../docs/adr/0060-independent-workspace-instances.md),
which supersedes the multi-sidecar proposal in ADR 0056. The executable
acceptance contract is the [validation plan](../docs/validation/multi-workspace.md).
Nothing in this proposal is implemented merely because it is documented here.

## Product model

Concurrent workspaces use independent OrkWorks application instances. An
instance never discovers, registers, focuses, monitors, or controls another
instance. It does not publish its port, process identity, active workspace,
health, attention, or ownership into shared history.

An instance has five visible states:

- **Picker:** no workspace or sidecar is adopted.
- **Opening:** a selected destination is being validated, leased, started, or
  restored.
- **Ready:** one sidecar owns the selected workspace and restoration completed.
- **Closing:** new work is gated while this instance cleans up its owned runtime.
- **Unresolved:** cleanup or ownership proof failed, so the instance cannot
  claim the workspace closed or start a replacement.

There is no global focused-workspace identity. The ready workspace in each
instance is that instance's sole context. A second instance may have another
workspace open, but neither instance knows or cares whether the other exists.

## Installation-scoped workspace history

Recent workspace history belongs to the OrkWorks installation. It is stored in
the installation's persistent application-data directory and survives process
exit and restart. It is shared path data, not live instance state.

The history record contains:

```json
{
  "version": 2,
  "revision": 43,
  "lastWorkspacePath": "<canonical path or null>",
  "recentWorkspacePaths": ["<canonical path>"],
  "pinnedWorkspacePaths": ["<canonical path>"]
}
```

It contains no process IDs, ports, leases, health flags, ownership claims,
peer identifiers, or open/closed state. `lastWorkspacePath` is only a startup
hint. Two instances may receive the same hint and independently attempt to open
it; the workspace lease resolves ownership without either instance finding or
activating the other.

History follows these rules:

- Persist canonical paths only. A display path may exist in current UI state,
  but is not peer or ownership data.
- Add a path only after its sidecar acquires the workspace lease, reaches
  readiness, and completes restoration.
- Move a successfully opened path to the front without duplication.
- Retain at most 20 recent entries and at most 50 pinned entries. Pinned
  entries never count toward the recent-entry cap and are never evicted by
  recency logic. A path appears in at most one of the two lists at a time.
- Retain at most 64 KiB of serialized history, evicting oldest recent entries
  to satisfy the byte limit. Pinned entries count toward the byte budget.
- Removing an entry deletes only that path shortcut. It never deletes project
  files, workspace metadata, sessions, or settings.
- Every mutation takes a short-lived advisory lock, rereads the current
  revision while holding it, increments the revision, writes and flushes a
  same-directory temporary file, and atomically replaces the history file.
  This lock protects one file update; it is not a running-instance registry or
  workspace-ownership mechanism.
- Missing history produces an empty picker. Corrupt or unreadable history is
  left untouched, loaded as empty in memory, and surfaced as a diagnostic. It
  is not overwritten until the user explicitly chooses to rebuild it.
- After an ambiguous replacement result, read back and compare the expected
  revision and record. Accept a confirmed new record. An old or unreadable
  result leaves runtime state unchanged and reports a storage diagnostic.
- History persistence is convenience state. Failure to write it never
  terminates or rolls back a successfully opened workspace.

Rendering or mutating history must not enumerate processes, probe ports, query
workspace leases for status, or contact another OrkWorks instance.

## Workspace identity and lease adoption

Resolve and validate the selected directory before changing the current runtime.
Canonical filesystem identity coalesces equivalent aliases. Different Git
worktrees remain different workspaces even when they share a Git common
directory. This feature does not create, delete, or manage Git worktrees.

The proposed [master-session parallel runner](../docs/superpowers/specs/2026-09-25-master-session-parallel-runner-design.md)
is a separately gated exception. Its parent/child broker may start dedicated
one-workspace child runtimes for worktrees allocated by an explicitly approved
plan. This bounded association is not a peer-instance registry: the master
sidecar retains ownership of only its own workspace metadata, each child
sidecar owns only its assigned worktree workspace, and no instance gains
cross-workspace focus, attention, or general control. The runner's worktree
creation and cleanup limits are defined by that design; child edits remain
for manual user integration, and cleanup requires a state-valid recorded user
disposition plus a clean, quiescent, plan-owned worktree. Each allocated
worktree is attached to a unique plan-owned branch that is preserved at cleanup.
Implementation remains gated on review of the separate runner implementation
plan and native proof of generation-specific child process ownership and
confinement.

On Windows, use filesystem semantics for junctions, symlinks, drive-letter,
separator, case, UNC, and extended-path spellings; do not lowercase paths or
assume distinct shares are aliases. Preserve distinctions on case-sensitive
filesystems. If identity resolution fails or changes during an open attempt,
report the error instead of falling back to the raw spelling.

The destination sidecar starts unadopted. Electron passes expected resolved
identity separately from presentation data. Before metadata loading,
reconciliation, session adoption, or other workspace work, the sidecar retains
a directory handle, verifies identity, and acquires the existing
workspace-scoped metadata lease from ADR 0052. Native validation covers
replacement between Electron's check and sidecar adoption.

A conflicting owner is an opaque external owner. Report a visible, retryable
conflict and clean up only the requesting instance's attempted runtime. Do not
inspect, activate, wait for, take over, reconcile, or terminate the owner. A
released lease or PID record is never descendant-cleanup proof.

Remembered paths retain existing path-keyed metadata semantics. Reopening a
deliberately replaced directory can therefore expose that path's existing
history and settings. Do not silently migrate keys or imply that a closed
shortcut tracks durable filesystem-object identity.

## Deterministic close-then-open lifecycle

Selecting a new workspace in one instance is a serial close-then-open operation:

1. Resolve and validate the destination while the current workspace remains
   ready and usable.
2. Gate new session creation, resume, generated foreground submissions, and
   Taskmaster analysis for the current workspace.
3. Take a final generation-bound snapshot of non-terminal sessions and owned
   inference. If confirmation is required, name this workspace and its known or
   uncertain work.
4. Stop owned sessions and inference, await bounded finalization, flush durable
   terminal state, terminate the sidecar, and obtain generation-specific proof
   that all owned descendants exited.
5. Only after acknowledged cleanup, clear the active workspace and enter
   **Opening** for the validated destination.
6. Acquire and verify the destination lease, await sidecar readiness, and
   complete restoration before publishing **Ready**.
7. Update installation history after successful restoration.

Failure outcomes are deterministic:

- Destination validation failure leaves the current workspace unchanged and
  ready.
- Cancellation before cleanup leaves the current workspace and its processes
  unchanged.
- Cleanup or ownership-proof failure leaves the current workspace
  **Unresolved** and starts no destination process.
- After successful cleanup, a destination lease conflict, spawn failure,
  readiness timeout, or restoration failure cleans up only that attempted
  runtime and leaves the instance in **Picker**. The previous workspace is not
  silently reopened.
- If attempted-runtime cleanup leaves an owned descendant, the instance is
  **Unresolved**, not **Picker**, and cannot start another replacement.
- A late event from an obsolete generation cannot publish readiness, mutate
  history, restore input authority, or replace current state.

Serialize switch, close, quit, and retry operations in the instance. Coalesce
repeated identical requests. No operation may acquire a second set of gates,
launch a replacement, or clear another operation's gates.

## Session, terminal, and Taskmaster behavior

The active sidecar retains existing session, terminal, Peon, review, and
Taskmaster contracts. One terminal attachment is visible. This feature does not
keep the old workspace's PTYs running inside the same instance after a switch.

Every asynchronous request captures workspace identity and runtime generation.
Late responses from a closing or replaced generation cannot update the ready
workspace. Generated prompt handoffs are admitted or rejected at the owning
sidecar's PTY boundary before cleanup acknowledgement; uncertain writes remain
visibly unresolved and are never blindly retried. Ordinary bytes already
admitted to a terminal remain bound to that original session.

Taskmaster analysis, evidence, recommendations, settings overrides, and usage
state remain scoped according to existing workspace and global contracts. This
feature introduces no cross-instance recommendation, attention, analysis,
quota, or focus authority. Global settings, usage-ledger rules, coding-tool
definitions, executable trust, and reporter publication retain their existing
atomicity and ownership contracts; independent instances do not make concurrent
global mutations safe by implication.

## Startup, close, quit, and crash recovery

On startup, read `lastWorkspacePath` as a hint. If it is absent, inaccessible,
invalid, or lease-conflicted, show the picker and start no adopted sidecar. Do
not automatically open another remembered path, resume coding sessions, or fall
back to the development repository or home directory.

Closing the active workspace uses the same admission gate and bounded cleanup,
then shows the picker. Graceful quit applies that sequence only to the current
instance. Repeated quit requests coalesce. Cancel preserves the instance,
workspace, terminal attachment, and controls.

Forced termination and OS crashes cannot promise a history flush or graceful
session finalization. Relaunch may adopt a remembered workspace only after the
platform ownership mechanism proves the previous generation's descendants have
exited or remain in a generation-bound container the new instance is authorized
to adopt. Persisted PIDs, process names, executable paths, and released metadata
leases are insufficient. Without proof, startup remains **Picker** or
**Unresolved** with a visible cleanup error.

## Process-ownership prerequisite

Issue [#545](https://github.com/Rambolarsen/orkworks/issues/545) is closed, but
its closure does not itself provide the native evidence required for
unavailable-sidecar cleanup and replacement or relaunch adoption. That evidence
remains a hard prerequisite. The production ownership boundary must enumerate
every process family that can outlive its launch request, including PTY and
long-lived inference children. Discovery, version-probe, Git, shell, and
coding-tool helpers may be excluded only when native evidence proves they
finish within bounded operations and cannot detach or leave descendants.

Each included child must be registered before execution or fail closed. Cleanup
must enumerate and terminate only that generation's registered descendants,
preserve a foreign-owner sentinel, and acknowledge complete exit within bounded
deadlines. Windows, macOS, and portable Linux need native evidence for selected
mechanisms. Unsupported platforms remain fail-closed.

The 2026-09-15 amendment retained in ADR 0056 proves a private Windows Job
fixture only. It does not prove production integration, PTY containment, or a
Unix mechanism. No implementation may generalize it into cleanup or replacement
authority.

## Validation prerequisites and non-goals

The [validation plan](../docs/validation/multi-workspace.md) is a release gate,
not evidence of completion. Close the reproduced Windows lease-classification
defect ([#543](https://github.com/Rambolarsen/orkworks/issues/543)) and
recommendation-replacement defect ([#544](https://github.com/Rambolarsen/orkworks/issues/544))
with native evidence before relying on those paths. Coordinate generation work
with [#360](https://github.com/Rambolarsen/orkworks/issues/360) and
[#361](https://github.com/Rambolarsen/orkworks/issues/361), and native fixtures
with [#525](https://github.com/Rambolarsen/orkworks/issues/525).

This proposal does not manage several live sidecars from one process, publish or
discover live instance state, provide a global dashboard/focus switcher, or
roll up attention across instances, automatically launch or resume another workspace,
coordinate Taskmaster across instances, manage Git worktrees, or use PIDs,
process names, paths, ports, or released leases as ownership proof. It also does
not add parallel terminal rendering, account management, native audio capture,
or coding after an instance exits.
