# Concurrent workspaces

Status: proposed; written-spec review required before implementation
Date: 2026-09-13
Tracking: [issue #541](https://github.com/Rambolarsen/orkworks/issues/541)

## Purpose and scope

Let a user work deliberately in one workspace while coding sessions continue
in other opened workspaces. Switching is infrequent and represents a mental
context change. Remember locations so returning does not require another
folder search. Preserve one window and one visible session context, following
[ADR 0013](../docs/adr/0013-single-active-context-primitive.md).

This document is a proposed extension to the [MVP](orkworks-mvp.md) and
[Taskmaster knowledge spec](taskmaster-knowledge.md), not implemented behavior.
The process ownership decision is in [ADR 0056](../docs/adr/0056-one-sidecar-per-open-workspace.md).
The executable validation requirements are described in the
[validation plan](../docs/superpowers/plans/2026-09-13-multi-workspace-validation.md).

## Workspace states and identity

- **Remembered:** a saved location, with no requirement for a running sidecar.
- **Open:** owns a sidecar lifecycle, including starting, ready, recovering,
  unavailable, and closing states. Opening alone never starts a coding session.
- **Focused:** the single open workspace whose sessions and details are shown.
  Other open workspaces are background workspaces.
- **Closed:** has no owned running processes; its location and durable history
  remain remembered. Closing does not delete session history or project files.

Canonical filesystem identity determines whether a workspace is already open.
Aliases must reuse the same runtime. Different Git worktrees remain separate
workspaces even when they share a Git common directory. This feature does not
create, delete, or manage Git worktrees. Existing workspace metadata leases
remain authoritative: a conflicting external owner produces a visible conflict,
never takeover, metadata reconciliation, or forced termination of that owner.

Resolve and validate the selected directory before registry lookup or spawning.
Use OS-resolved directory identity to coalesce aliases and one canonical path
for that identity throughout sidecar restoration, settings keys, metadata, and
session ownership. Keep display/input paths separate from identity. On Windows,
resolve junctions/symlinks and equivalent drive-letter, separator, case and
extended-path spellings using filesystem semantics; do not lowercase paths or
assume that distinct UNC shares/server names are aliases. Preserve distinctions
on case-sensitive filesystems. Revalidate identity before opening; if resolution
fails or the directory was replaced, report it rather than falling back to the
raw spelling. Reuse existing durable metadata/settings for the resolved workspace;
normalization must not silently create a second history or discard overrides.

## Switcher and focus

The workspace-name control opens a keyboard-accessible switcher. Show open
workspaces and remembered closed locations, with enough path information to
distinguish equal folder names. Migrate the existing recent-location store into
the remembered-location list, removing its ten-entry truncation. Remember every
successfully opened location until the user explicitly forgets it; recency only
orders the list. Forget is available for closed locations and removes the shortcut,
not metadata or project files. An Add workspace action opens the native picker.

Selecting an open workspace changes focus without restarting its sidecar.
Selecting a remembered closed workspace opens it and focuses it after readiness.
Until a new destination is ready, preserve the previous focus and its terminal.
Failure to open the destination leaves the previous focus usable and shows the
destination's error. Coalesce simultaneous opens of the same identity.
Commit the visible focus and durable last-focused location only after destination
readiness and the Taskmaster permission handoff below succeed. A failed or
superseded attempt must not overwrite the previous last-focused location.

Switching detaches the previous terminal view, not its runtime. PTYs keep
draining output, recording bounded history, and feeding Peon in background
workspaces. Each workspace remembers its selected session. Returning restores
that session when still present, including historical replay for an ended
session; otherwise show no selection. Review always follows the selected
session, never an artifact retained from another workspace. No automatic focus
change occurs because a session needs input or a background sidecar recovers.

### Attention

The collapsed switcher shows the total number of live background sessions in
the existing user-attention state. It excludes the focused workspace, idle or
working sessions, and ended sessions. Expanded rows show the count per workspace.
Counts are current state, not accumulating notifications or read/unread state.
Viewing a workspace does not clear a session's underlying attention state.

An unavailable workspace is shown as unavailable, not as zero attention. Mark
its last count stale and exclude it from the confirmed aggregate; accompany
the aggregate with an accessible unavailable indicator. A background failure
does not produce a popup or replace the focused workspace's connection state.
Collect summaries through workspace-bound sidecar requests; do not attach
background terminal views merely to obtain attention information.

## Startup, close, and quit

On app startup, reopen only the last focused workspace. All other locations
remain remembered without sidecar startup. Do not automatically resume coding
sessions. If the last location is missing, show the picker and retain other
remembered locations. Do not silently open a different project.
With no last location, or a missing/inaccessible last location, start no sidecar,
publish no backend port, and acquire no workspace lease until explicit selection.
Remove the current development-repository/home-directory fallback for this case.

Switching away never asks to stop sessions. Explicitly closing a workspace
with live or creating sessions requires a native confirmation naming the
workspace and running-session count: Cancel or Close workspace. Cancel leaves
focus and processes unchanged. With no live or creating sessions, close
directly. Atomically block creation/resume while taking the final runtime-generation
and live/creating-session snapshot, including for the no-dialog fast path. The
sidecar serializes this gate with every start/resume admission, including requests
already in flight; those admitted before the gate appear in the snapshot. After
a dialog, if the generation changed or additional sessions started, refresh
confirmation before including them. Keep the gate during renewed confirmation;
Cancel releases it and leaves existing processes and focus unchanged.
Sessions that finished meanwhile do not require another confirmation.
If a sidecar is unavailable and session liveness cannot be confirmed, show the
last known session count as uncertain and require confirmation before cleanup;
never interpret a failed status request as an empty workspace.

Once confirmed, reject new session creation/resume and new analysis for that
workspace, stop owned sessions and inference, flush terminal metadata/history,
then terminate the sidecar and confirm process exit and lease release. Keep
the workspace visibly closing until cleanup finishes. Bound graceful cleanup;
escalation may target only processes owned by that runtime. Failure must remain
visible and must not report the workspace closed while owned processes survive.
Use a five-second graceful-cleanup deadline followed by a five-second owned-
process termination deadline. Report any survivor after those deadlines rather
than looping forever or broadening termination to unrelated processes.
Ownership comes from retained process handles and the runtime's recorded child
ownership, never PID/name/path matches alone. A failed open against an external
lease owner authorizes cleanup only of our attempted runtime. Do not wait for,
release, or terminate that external owner's lease/processes. A surviving owned
process leaves our runtime unresolved; a surviving foreign owner does not prevent
removal of our failed attempt once our own cleanup is complete.

The current PTY ownership in a sidecar's memory is insufficient after that sidecar
crashes. Before implementing this cleanup promise, prove a native ownership
boundary that outlives each sidecar: Electron must retain a generation-bound OS
containment handle or a surviving supervisor channel before any session or
inference child can execute. Spawn admission fails closed if registration fails.
The owner must enumerate/terminate only its registered descendants and acknowledge
complete exit, including children surviving a sidecar crash; PID records and a
released metadata lease cannot supply that proof. Windows Job containment and
Unix supervision require separate native fixture evidence; the existing provider
Job helper alone does not prove PTY containment. Selecting and recording the
platform mechanism is an implementation-planning prerequisite. Until proved,
unavailable-runtime cleanup remains unresolved and must never report successful
close/quit or launch a replacement over potentially surviving owned sessions.

Closing a background workspace leaves focus unchanged. Closing the focused
workspace selects the most recently focused remaining open workspace, or the
picker when none remain. Persist that focus; with none open, clear last focus.

Explicit app quit uses one confirmation across all live/creating sessions and
unavailable workspaces with unknown liveness, listing known counts and last-known
counts explicitly marked uncertain. Cancel leaves the application running.
Confirm prevents new work and closes all owned runtimes, with the same bounded
cleanup rules. Retain the last focused location for next startup, unlike closing
the final workspace individually. Coalesce repeated quit requests into one
dialog/cleanup operation. Native window close follows the platform's existing
app-lifetime convention; on macOS closing the window does not imply app quit.
Where closing the last window quits the app (Windows/Linux), intercept its
cancelable close event before destroying the window. Use the same coalesced quit
operation; Cancel preserves the existing window, terminal attachment and controls.
Apply the atomic admission gate and final snapshot to every open runtime before
quit cleanup; refresh the aggregate confirmation if any set or generation widened.
Do not start cleanup in one workspace while another still needs confirmation.
Crashes and forced OS termination cannot promise confirmation or graceful flush.

The app-quit confirmation is the working interpretation of the owner's final
go-ahead after that question; it is explicitly included in written-spec review.

## Runtime ownership and request routing

Electron main owns one registry entry per canonical workspace. Each entry owns
its sidecar process lifecycle, restoration readiness, generation, port, private
authorization token, settings-application status, and bounded recovery policy.
One focused-workspace identity is separate from all process generations.
Reuse existing lifecycle/restoration modules behind this registry; do not
duplicate their retry logic or replace the sidecar's single-workspace model.

Every async action captures workspace identity and runtime generation before
awaiting readiness, confirmation, fetch, or model discovery. Never resolve a
mutation against an ambient current-backend URL after the user has switched.
Responses update their owning workspace only. Foreground views additionally
validate the focus epoch before publishing. Late events from an old generation
cannot replace a newer port, token, settings state, or terminal attachment.
This applies to session create/resume/end, active-session persistence, settings,
integration changes, terminal links, plan review, and recommendation handoffs.

The renderer receives workspace identities and validated lifecycle summaries,
never privileged tokens or arbitrary filesystem/network authority. Session
commands retain their narrow contracts and include an owning workspace selector.
IPC types remain separately defined in electron/ and src/; no cross-imports.

## Workspace-specific behavior and shared resources

| Workspace-owned | Application-wide |
| --- | --- |
| Sessions, terminal history, attention, selected session | Remembered locations and focused-workspace identity |
| Workflow observations, repository facts, recommendations and dismissals | Installed coding-tool definitions and executable trust |
| Taskmaster model, context access, exclusions, interval, enabled override | Global defaults and Taskmaster daily budget |
| Taskmaster diagnostics and workspace cache invalidation | Verified reference-knowledge bundle and update checks |
| Active coding tools, local integration configuration | Shared reporter assets |
| Peon selection and retention overrides | Default Peon selection and retention settings |

Use existing global defaults when no workspace override is present. Migrate
existing saved settings without changing effective behavior. A workspace-only
edit cannot replace another workspace's override or invalidate its unrelated
cache. Apply global changes to all affected open sidecars and retain explicit
per-workspace application failures; never report universal success after one
successful push. Save scoped settings under revision checks so a stale draft
cannot replace newer settings for another workspace.

Taskmaster settings reads return an opaque revision of the durable settings
document. Saves carry that expected revision and either a global-default patch
or a patch/reset for one canonical workspace, never a replacement of all workspace
overrides from a UI draft. Under the existing cross-process persistence lock,
reload the document, compare revisions, validate and apply the scoped patch,
and publish atomically. A mismatch returns a conflict with no settings, cache,
diagnostic, or budget mutation; preserve the draft for explicit refresh/retry.
This intentionally allows unrelated concurrent edits to conflict rather than
silently merging stale intent. Successful workspace-only saves invalidate only
that workspace's affected analysis; changing the document revision alone must
not invalidate other workspaces. Global saves invalidate only affected effective
configuration. Electron serialization alone is insufficient for other processes.

Taskmaster follows each workspace's permitted repository instructions and
evidence; shared knowledge does not override local policy. This feature adds
no simple/advanced workflow presets or workflow engine. Existing bounded
excerpts are incomplete evidence and cannot prove a policy is absent. More
complete instruction selection is separate follow-up work, not a claim made
by opening multiple sidecars.

Changing global coding-tool definitions must invalidate/reload every open
sidecar's cached registry before subsequent affected operations. Preserve the
existing cross-process harness document lock and executable identity checks.
Shared reporter publication must tolerate simultaneous installation without
partial files or spurious first-install failures. Reporters continue using each
session's own port and capability, never the focused workspace's port.

Capacity observations remain workspace-local initially. Coding-tool ID alone
does not establish shared credentials or quota; do not propagate a cap into
another workspace as fact without account-scoped evidence. No account discovery
or cross-workspace quota broker is included. Measure aggregate Peon/local-model
load in validation before adding scheduling infrastructure.

## Taskmaster focus permission

Only the focused, ready workspace may start new Taskmaster evaluation, including
deterministic evaluation and model analysis. Background Peon and hook evidence
continues to be recorded; existing recommendations remain stored. On refocus,
reevaluate from current evidence subject to existing debounce, interval, cache,
and budget rules. Do not treat refocus as permission for an extra provider call.

For this proposed mode, the evaluation triggers in taskmaster.md and the
"currently open workspace" wording in taskmaster-knowledge.md are qualified by
focused-workspace permission: opening/restoring a background workspace, accepting
its observations, or reaching a periodic timer does not start evaluation. Update
those accepted-spec passages and ADR 0042's deterministic active-workspace
correlation clause together when accepting this proposal; ADR 0054 remains the
managed-CLI-policy decision. Follow the ADR amendment/supersession sequence;
until then their current single-workspace implementation remains unchanged.

Electron grants/revokes analysis permission through a narrow authenticated
sidecar operation bound to workspace identity, runtime generation and a monotonic
focus-transition epoch. Default new/recovered sidecars to analysis suspended.
Serialize transitions; reject obsolete permission commands. Prepare the destination
to readiness before revoking the old permission, so open failure leaves the old
permission and durable focus unchanged. A revocation acknowledgement means the
old evaluation gate is closed and pending results invalidated atomically with
reservation, spawn and commit checks; it also requests inference cancellation,
never coding-session cancellation. It need not wait for inference exit.

Grant the destination only after confirmed revocation or confirmed exit of the
old sidecar generation. An unavailable status or HTTP timeout is not exit proof.
Bound the revoke/grant exchange to five seconds. On failure, preserve the previous
visible and durable focus, show Taskmaster switching unavailable, and grant no
other workspace. Reconcile any uncertain destination grant by confirmed revoke
or process exit before regranting the previous workspace with a newer epoch.
Late replies cannot complete an abandoned transition. Never terminate coding
sessions merely to make a focus change succeed. After a confirmed old-sidecar
crash, focus may move to a ready workspace, but model analysis remains blocked
until the surviving process owner confirms the old inference exited: the crashed
sidecar's released analysis lease alone is insufficient. Recovery still starts
suspended. These rules trade
a visible failed switch during an unresponsive-sidecar fault for unambiguous
analysis authority, without adding a second cross-process coordination service.

Retain the cross-process analysis lease until actual inference cleanup finishes;
discarding a result alone does not release execution capacity. The destination
may display immediately while waiting for that lease. Preserve durable usage
accounting: a reservation already made is not refunded by switching/cancelling.
Application-wide defaults remain eight evaluations per UTC day and a 60-minute
minimum workspace interval. Model timeout and process-ownership safeguards
remain in force. Native CLI managed-policy effects retain the existing spec's
limits; cancellation cannot undo effects already performed by the CLI.

## Validation evidence and prerequisites

Investigation at revision 7c61883 on Windows, 2026-09-13:

- 38 focused desktop tests passed; a separate two-lifecycle check with mock
  child processes verified independent ports, failure state, and disposal.
- The taskmaster:: Rust filter ran 80 tests: 70 passed, eight failed, two ignored.
- The analysis lease test fails independently: Windows lock contention returns
  error 33 rather than the expected busy result. The lock still excludes the
  second owner; this is error classification, not concurrent execution.
- Seven recommendation-store tests report sharing violations. A representative
  dismiss test fails independently; put keeps its temporary file open while
  calling the Windows replacement helper, which requires it closed first.
- Real two-sidecar active-session behavior and resource usage are unmeasured.

Repair and rerun the Windows lease classification defect
([#543](https://github.com/Rambolarsen/orkworks/issues/543)) and recommendation
replacement defect ([#544](https://github.com/Rambolarsen/orkworks/issues/544)).
Both issues must close with native regression evidence before claiming readiness.
Prove crash-surviving process ownership
([#545](https://github.com/Rambolarsen/orkworks/issues/545)) before implementing the registry's
unavailable-runtime cleanup/recovery path; record the selected platform mechanisms
in ADR 0056 and the implementation plan before implementation proceeds.
Coordinate generation work with [#360](https://github.com/Rambolarsen/orkworks/issues/360)
and [#361](https://github.com/Rambolarsen/orkworks/issues/361), and native validation
with [#525](https://github.com/Rambolarsen/orkworks/issues/525). The validation plan
is a release gate, not evidence that those scenarios have passed.

## Non-goals

No parallel terminal rendering, cross-workspace task planning, automatic session
launch/resume, Git/worktree management, account or credential management,
background coding after app quit, native audio capture/proxying, or settings
that weaken executable trust or the application-wide budget. Multi-window UX,
instruction-collection redesign, and cross-workspace capacity coordination are
separate proposals.
