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

Each instance owns exactly one workspace and one sidecar lifecycle. An instance
does not discover, register, focus, monitor, or coordinate with other instances.
There is no cross-instance workspace registry, aggregate attention count, shared
focus coordinator, or peer-instance cleanup protocol.

Selecting a different workspace in an instance closes the current workspace and
opens the selected workspace in that same instance. A second instance may happen
to have another workspace open, but the first instance neither knows nor cares.

## Remembered workspaces

The instance keeps a recent-workspace list containing canonicalized location
records and display paths. The list is path history only; it contains no live
instance state, process IDs, ports, leases, health flags, or ownership claims.

- Add a location after a successful open.
- Move the selected location to the front without duplicating it.
- Preserve existing workspace metadata and settings when reopening a location.
- Allow explicit removal of a remembered location without deleting project files
  or workspace metadata.
- Use atomic bounded persistence and tolerate a missing or corrupt history file
  by showing an empty picker with a visible diagnostic.
- Do not query other OrkWorks instances while rendering or mutating this list.

The recent list may be stored in the existing application-level settings root,
but it must remain independent of workspace metadata and sidecar ownership. A
second instance reading the same history is only reading paths; it does not
create a coordination relationship.

## Open and switch lifecycle

The current instance follows the existing single-workspace lifecycle:

1. Resolve and validate the selected directory before spawning a sidecar.
2. Gate new session, resume, foreground submission, and analysis admission for
   the current workspace.
3. Perform bounded graceful cleanup and owned-process termination for the current
   sidecar.
4. Continue only after the current sidecar reports acknowledged cleanup, or
   remain on the current workspace with an explicit unresolved-cleanup state.
5. Persist the new recent-workspace entry atomically.
6. Start the new sidecar and restore only that workspace's state.

The switch is not a handoff between workspaces. It is close-then-open within one
instance. A failed close never silently starts the destination over surviving
owned processes. A failed destination open leaves the instance with no active
workspace or returns to the previously confirmed workspace according to the
existing lifecycle contract; it never adopts another instance's state.

On startup, open only the last selected workspace for that instance. If it is
missing or inaccessible, show the remembered-workspace picker and start no
sidecar until the user selects a location. Do not resume coding sessions
automatically.

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
workspace metadata concurrently. That is an ownership conflict for the selected
workspace, not peer-instance coordination. The instance reports the existing
conflict and does not take over or terminate the external owner.

Global application settings and shared reporter installation retain their
existing ownership rules. This design does not make simultaneous mutation of
shared global settings safe by implication; those writes continue to use their
existing atomic/revision contracts.

## Process ownership prerequisite

Independent instances simplify multi-workspace orchestration but do not remove
the crash-surviving process-ownership prerequisite in issue #545. A sidecar
crash can still leave PTY or inference descendants unless a surviving owner or
OS containment boundary proves registration, bounded termination, and complete
exit.

The next process-ownership plan therefore covers only long-lived session and
inference children. Short-lived model-discovery, version-probe, Git, and shell
helpers remain bounded operations with explicit local timeouts; they are not
treated as persistent workspace descendants unless later evidence shows that
they can outlive their operation.

No runtime implementation should claim successful unavailable-sidecar cleanup or
replacement adoption until the applicable platform mechanism is selected and
proven with native evidence. Unsupported platforms remain fail-closed.

## Acceptance examples

- Open workspace A in instance 1, then select workspace B: instance 1 closes A
  and opens B; it never checks whether instance 2 has A or B open.
- Open workspace A in instance 2 while instance 1 owns A: the existing
  workspace lease reports a conflict; instance 2 does not terminate or inspect
  instance 1.
- Open A, B, and C sequentially in one instance: the recent picker remembers
  all three locations while only the selected workspace has a live sidecar.
- Close A with a surviving owned descendant: the instance remains unresolved and
  does not claim that B was opened successfully.
- Start a second OrkWorks process with an independent application root: it may
  have an independent recent list and workspace lifecycle; neither process gains
  knowledge of the other.

## Non-goals

- Managing multiple sidecars from one Electron registry.
- Providing a global workspace switcher or dashboard.
- Automatically resuming sessions in another instance or workspace.
- Creating, deleting, or managing Git worktrees.
- Using persisted PIDs, process names, executable paths, or released metadata
  locks as proof of process ownership.
