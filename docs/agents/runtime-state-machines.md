---
type: Implementation Reference
title: Runtime state machines
description: State transitions and invariants for OrkWorks desktop and sidecar workflows, with reproducible audit findings.
tags: [orkworks, runtime, state-machines, verification]
---

# Runtime state machines

This inventory traces the principal production workflows from the Electron
workspace owner through the renderer and sidecar. The audit baseline is
`7471775a`. The desktop resume table includes the corrected behavior and the
findings below record the baseline behavior. See [domain entities](domain-entities.md)
for vocabulary and [architecture](architecture.md) for component boundaries.

State dimensions are independent unless a transition explicitly couples them.
In particular, process lifecycle, history organization, observed attention, and
renderer selection have different owners.

## Workspace ownership and sidecar readiness

Sources `apps/desktop/electron/{workspaceSwitchCoordinator,sidecarLifecycle}.ts`.
See [sidecar recovery](architecture.md#sidecar-lifecycle-and-runtime-recovery).

| Machine and current state | Event and guard | Next state |
| --- | --- | --- |
| Folder chooser idle | Open other folder while workspace controls are available | One queued or visible chooser |
| Folder chooser queued, visible, or opening its selection | Another chooser request arrives | Share the pending request and its outcome |
| Folder chooser pending | Cancelled, failed, or workspace opening settles | Release the chooser so a later action can open it |
| Workspace `picker` or `ready` | Validated request for another workspace, serialized with close and quit | `closing` when a runtime is owned, otherwise `opening` |
| Workspace `closing` | Owned cleanup completes | `picker`, then `opening` if switching |
| Workspace `opening` | Destination starts, matches the requested workspace, and restores | `ready` at the new generation |
| Workspace `opening` | Startup or restore fails and its attempted runtime is cleaned up | `picker` with diagnostics |
| Workspace `closing` or `opening` | Cleanup remains unresolved | `unresolved`, with replacement blocked |
| Sidecar `starting` | Current child announces a valid port | `ready` |
| Sidecar `starting` or `ready` | Current child fails, times out before readiness, or exits unexpectedly | `failed`, with cleanup tracked independently |
| Sidecar stopping | Explicit shutdown settles | Cleanup completes or preserves the unresolved failure |

**Invariants.** The coordinator serializes workspace operations. A stale child
cannot publish readiness for a new generation. Retry must respect cleanup
ownership, and an unexpected process exit alone cannot prove cleanup.

## Session lifecycle, organization, and history

Sources `crates/orkworksd/src/{session_application,session_actions,session_view}.rs`,
`runtime/terminal_runtime.rs`, and `apps/desktop/src/sessionRuntimeState.ts`.
See [transition ownership](domain-entities.md#transition-ownership-and-observation-windows)
and [session organization](runtime-contracts.md#session-organization).

| Machine and current state | Event and guard | Next state |
| --- | --- | --- |
| Runtime absent or eligible ended | Create or explicit validated resume reserves the runtime | `creating` |
| Runtime `creating` | Owned child starts | `alive` |
| Runtime `creating` or `alive` | Startup failure, PTY wait error, native app-server failure, or initial prompt delivery failure | `stopping` / phase `ending`, with outcome `error` |
| Runtime `creating` or `alive` | The PTY driver observes child exit and drains output | `stopping` / phase `ending`, with `ended` for successful exit, `error` for unsuccessful exit, or `killed` for an observed explicit stop |
| Runtime `ending` | Owned finalizer persists terminal metadata | `dead` / phase `ended` |
| Runtime `ending` | Terminal persistence fails | Retains `ending`; the same generation may retry finalization |
| Organization `active` | Archive or confirmed Delete reserves the target and awaits durable ending | `archived` or removed |
| Organization `closed` | Archive, eligible resume, or confirmed Delete | `archived`, `active`, or removed |
| Organization `archived` | Restore or confirmed Delete against finalized state | `closed` or removed |

**Invariants.** The PTY driver carries the child exit status through output drain.
An explicit stop takes precedence over an unsuccessful child exit, while an
independent runtime failure still follows its existing error path.
Durable terminal metadata takes precedence over an ending handle in live
projection. Resume must reject an ending handle or archived history.
Archive and Delete wait for the captured runtime, and archived history is
protected from automatic age/count cleanup. A terminal connection does not own
the PTY. The SQLite session store and its recovery stages have separate
activation gates in the [session storage spec](../../specs/session-storage.md).

## Attention, prompt authority, and capacity

Sources `crates/orkworksd/src/runtime/{observed_status,prompt_authority,terminal_status}.rs`,
`session_application/hook_observation.rs`, and `capacity_state.rs`. See
[attention authority](harness-integration-contracts.md#prompt-attention-authority).

| Machine and current state | Event and guard | Next state |
| --- | --- | --- |
| Live attention working or idle | Qualified observed prompt, or supported current-generation harness report | Waiting / Needs You |
| Prompt pending | Additional ambiguous or mixed elicitation | Pending with ambiguous authority |
| Prompt pending | Committed input, mapped resolution for a matching sole prompt, or lifecycle clear | Prompt cleared |
| Live attention working | Idle detector passes current-generation, freshness, and authority checks | Idle |
| Capacity clear | Current output analysis finds a supported limit signal | Capped latch |
| Capacity capped | Recheck is armed and fresh output supports clearing the latch | Clear |
| Resume capacity check pending | Fresh output is observed and the pending indicator advances | Check no longer displayed |

**Invariants.** The application checks lifecycle, runtime generation, and source
authority when it applies observations. Raw hints cannot clear a stronger pending
prompt. An old capacity analysis cannot overwrite a newer control revision.

## Renderer synchronization, resume, and navigation

Sources `apps/desktop/src/{workspaceUpdates,workspaceSessionController,sessionPolling,recommendationSource,shellNavigation}.ts`
`apps/desktop/src/components/RecommendationsPanel.tsx`, and
`crates/orkworksd/src/workspace_updates/split.rs`.
See [workspace update transport](../adr/0092-workspace-update-stream.md) and
[fixed desktop shell](architecture.md#fixed-desktop-shell).

| Machine and current state | Event and guard | Next state |
| --- | --- | --- |
| Session source disabled | Workspace ready and admission open | REST fallback and stream connection pending |
| Stream awaiting baseline | First valid snapshot for the expected workspace | Stream healthy; current REST fallback retires |
| Stream healthy | Contiguous delta for the current instance and generation | Baseline and sequence advance together |
| Status-first stream subscribed | Watcher reports a recommendation change or the 30-second reconciliation deadline arrives | Publish recommendation invalidation to the current stream |
| Stream connecting or healthy | Transport failure, heartbeat timeout, invalid message, or explicit resync | Baseline discarded; REST fallback and reconnect |
| Recommendation refresh idle | First subscriber, invalidation, or fallback timer fires | One request in flight |
| Recommendation refresh in flight | Invalidation arrives | Dirty until settle, then one follow-up refresh |
| Recommendation refresh subscribed | Last subscriber leaves | Inactive; older result cannot publish |
| Recommendation action pending | Same-workspace list refresh returns a pre-action snapshot | Action stays pending and retains ownership of its settlement |
| Recommendation action pending | Dismiss or Run cleanup succeeds in the current workspace | Clear the acted-on audit notice and reconcile through a read started after the write |
| Recommendation action pending | Workspace changes or admission closes | Clear local progress and ignore late completion |
| Resume idle | Resume admitted for the row | Pending, with one operation per row |
| Create pending | Receipt arrives before the list has observed the new row | Publish the pre-spawn row and track its startup outcome |
| Create pending | A session list has already observed the row when the receipt arrives | Keep the observed row and reconcile its startup outcome |
| Created row tracked | A structured startup failure or error row is observed | Report the failure once and retire its tracking |
| Resume pending | Backend URL resolves while admission and the row reservation are current | Send the resume to the sidecar |
| Resume pending | Backend URL resolves after admission or the reservation is invalidated | Skip the stale dispatch |
| Resume pending | Duplicate request, or old operation finishes after replacement | Current pending operation is unchanged |
| Resume pending | Success still owns its row and workspace admission | Update that row and fence older list reads |
| Resume pending | Successful resume also owns the foreground and no intervening selection occurred | Focus the resumed row |
| Resume pending | Archive, delete, or another organization action supersedes the same row | The older resume cannot publish or focus the row |
| Resume pending | Current failure or admission closes | Clear progress for the owned operation; recover current data on failure |
| Delete pending | Current workspace receives successful removal | Dispose its terminal, remove the row, and reconcile the remaining list |
| Delete pending | Delete fails, including after partial cleanup | Keep the error and reconcile removal only if a current list omits the row |
| Active session write idle | Renderer submits the active row under current admission | Queue and send one write |
| Active session write in flight | Another selection is submitted in the same workspace | Queue it behind the current write |
| Active session write in flight | Current write succeeds or fails | Settle its caller and release the next queued write |
| Active session write queued | Workspace or admission changes | Skip the stale write. Replacement workspace submissions use a fresh queue |
| Shell Terminal or Review | Explicit session selection | Terminal for the selected session |
| Shell Terminal | Current selected session opens an available review artifact | Review with a return target |
| Shell Review or inspector | Close or target disappears | Restore a valid surface and focus target, or apply a visible fallback |
| Shell on older workspace | Workspace generation advances | Clear session, review, and inspector state tied to the former workspace |

**Invariants.** A stream delta needs a matching baseline and cursor. Reconnect
starts with a new authoritative snapshot. Periodic invalidation lets the
desktop retry the recommendation query on a healthy stream. Workspace admission fences old
operations, selection order governs focus, and source order governs publication
of session data. Selection and row freshness must both survive asynchronous
resume. Inspecting a different subject does not itself select a terminal.

## Terminal attachments and replay

Sources `apps/desktop/src/{terminalAttach,terminalStore}.ts`,
`components/{CenterPanel,HistoricalTerminal}.tsx`, and `crates/orkworksd/src/runtime/terminal_http.rs`.

| Machine and current state | Event and guard | Next state |
| --- | --- | --- |
| Live terminal absent | Still-current renderer awaits backend and attaches | Socket connecting |
| Socket connecting | Connection opens and the handle is current | Interactive, with bounded pending input |
| Socket attached | Socket closes or the component is disposed | Input disabled or attachment removed; PTY lifecycle proceeds independently |
| Closing or closed socket without a terminal end event | Renderer returns to the still-live session | Replace the unavailable transport and attach with bounded replay |
| Healthy socket | Renderer returns to the session | Reuse the existing handle |
| Socket closed without live output | Historical fetch succeeds for a still-owned handle | Bounded replay, at the recorded grid when available |
| Historical terminal loading | Replay query settles for the current session | Loaded, empty, or error |

**Invariants.** History is read-only. An attachment loss does not stop the
sidecar-owned process. Archived rows are excluded from live-terminal pruning's
keep set even when their projected lifecycle has regressed.

## Peon and Taskmaster

Sources `crates/orkworksd/src/runtime/peon_runtime.rs`,
`crates/orkworksd/src/taskmaster/{mod,runtime,store}.rs`, and
`crates/orkworksd/src/session_application/recommendation_delivery.rs`. See
[recommendation lifecycle](../../specs/taskmaster.md#recommendation-lifecycle).

| Machine and current state | Event and guard | Next state |
| --- | --- | --- |
| Peon idle or prior terminal attempt | Current session becomes eligible | Candidate |
| Peon candidate | Runtime identity acquires an available diagnostic lease | In flight |
| Peon in flight | Matching attempt publishes a result, fails, or times out | Completed or Failed |
| Taskmaster evaluation idle | A trigger reserves an attempt | Queued, then Running when claimed |
| Taskmaster evaluation running | Current attempt settles, or owned recovery interrupts it | Succeeded, Failed, or Interrupted |
| Recommendation proposed | User accepts or dismisses, evaluator supersedes or expires, or rollup attaches an eligible member | Accepted, Dismissed, Superseded, Expired, or RolledUp |
| Workflow recommendation proposed | Explicit approval reserves delivery before terminal input | Executing |
| Workflow recommendation executing | Delivery succeeds and the reservation settles | Accepted |
| Workflow recommendation executing | Delivery fails and cancellation settles | Proposed for retry |
| Workflow recommendation executing | Explicit dismissal recovers a stuck reservation | Dismissed |
| Workflow recommendation accepted | Verified completion is reported for the approved action | Completed |

**Invariants.** Peon results and lease cleanup are scoped to the attempt and
runtime that own them. Taskmaster attempt state is distinct from the last
outcome. Delivery contention retries settlement under the original ownership
without repeating terminal input. Unchanged dismissed evidence does not
resurface immediately. Rollup members retain exact family lineage. Proposed ordinary-child orchestration is tracked separately
under [the coordinator design gate](../../specs/taskmaster.md#coordinator-design-gate).

## Peon provider verification in Settings

Source `apps/desktop/src/components/SettingsModal.tsx`.

| Current state | Event and guard | Next state |
| --- | --- | --- |
| Any open verification state | User edits the Ollama URL | Schedule verification for the new draft and clear the previous result |
| Any open verification state | Settings opens, provider changes, or Refresh models is requested | Verify the current selection immediately |
| Verification scheduled | Debounce expires for the current draft | Verification in flight |
| Verification in flight | Current request settles | Publish models and the verified URL, or show a retryable error |
| Verification scheduled or in flight | Modal closes or unmounts | Cancel the timer and invalidate any in-flight reply |
| Verification scheduled or in flight | An earlier request settles | Keep the current draft and verification state |

**Invariants.** The new request owns verification from the time it is
scheduled. Only its reply can publish models, normalize the URL, show an error,
or clear the busy indicator. Apply requires verification of the current draft.

## Captured SQL observation persistence

Sources `crates/orkworksd/src/workflow_observations/sql_origin/{persistence,reader}.rs`.
See [captured workflow evidence files](architecture.md#captured-workflow-evidence-files).

| Current state | Event and guard | Next state |
| --- | --- | --- |
| Batch captured | Original deadline permits bounded intent, inventory, and first-write preflight | Prepared for first effect |
| Batch before first effect | The deadline expires, including after the inventory scan | Expired without publishing a sequence or segment |
| Batch prepared | A duplicate receipt is verified against the retained segment and counter | Receipt returned without a write |
| Batch prepared | Pins and budgets are valid, and the first sequence advance starts | Original batch owns its file effects through settlement |
| Batch writing | An append or rewrite finishes successfully | Receipt returned and pins, offsets, and shared budgets advance |
| Batch writing | I/O fails with uncertain effects | Prior receipts survive and later candidates are refused |
| Batch writing | Original deadline passes after the first effect | Remaining candidates still settle under the original reservation and budgets |

**Invariants.** The batch scans once and charges every reread to its shared
budget. Before the first effect, expiry leaves the observation directory
untouched. After the first effect, deadline expiry cannot abandon the original
write or grant its ownership to a retry. The original sequence advance can
survive an uncertain write, so sequence values need not be contiguous.

## Desktop updates

Source `apps/desktop/electron/updateService.ts`. See
[packaging and release](architecture.md#packaging-and-release).

| Current state | Event and guard | Next state |
| --- | --- | --- |
| Unavailable | Development build, unsupported platform or version | Unavailable |
| Never checked, Up to date, Available, or eligible Error | Start an update check | Checking |
| Checking | Current operation returns channel-matching release information | Available or Up to date |
| Available or eligible download Error | User starts download for the current candidate | Downloading |
| Downloading | Current engine operation returns the matching artifact | Downloaded |
| Downloaded | Verification and confirmation succeed | Installing |
| Downloaded | Confirmation declined | Downloaded |
| Checking, Downloading, or install attempt | An owned operation fails | Retryable Error |

**Invariants.** Engine events are scoped by operation identity. Install requires
the verified candidate, explicit confirmation, and successful workspace
shutdown before invoking the installer.

## Workspace hook warnings

Source `apps/desktop/src/components/WorkspaceHookNotice.tsx`. See
[coding-tool hooks](../user/coding-tools.md).

| Current state | Event and guard | Next state |
| --- | --- | --- |
| Workspace and coding-tool selection active | Mount, focus, Settings closes, or retry timer fires | Check integration status with one current request |
| Status check in flight | Successful reply with actionable warnings | Show the confirmed warnings and schedule another check |
| Status check in flight | Successful reply with no actionable warnings | Clear the warnings and stop the timer |
| Status check in flight | Current read fails | Preserve its scope's last successful warning state and schedule a retry |
| Checking or waiting | Settings opens, workspace changes, or the coding-tool selection changes | Cancel the timer and request. A new workspace or tool selection starts with no warnings |

**Invariants.** Only a successful response can change the warnings within
its current scope. A superseded request cannot publish or schedule a retry.

## Confirmed findings and regression evidence

The resume findings are grouped in [#2101](https://github.com/Rambolarsen/orkworks/issues/2101).
The tests in `apps/desktop/tests/workspaceSessionController.test.ts` drive the
production controller with deferred responses, without timing-dependent sleeps.

| Impact order | Reachable invalid transition on the baseline | Evidence and correction |
| --- | --- | --- |
| 1. Incorrect session lifecycle display and terminal attachment cleanup | Start resume, start a list read, deliver the successful resume, then deliver the list containing the prior ended row. The local row goes `dead → alive → dead` without a new process termination. | The baseline test observed `dead` where it expected `alive`. Successful resume now advances the source generation before updating the row. |
| 2. Unexpected terminal selection | Start resume for A, select B, and then deliver the successful resume for A. The late callback selects A after the deliberate selection of B. | The baseline test observed selection callbacks for B and then A. Successful resume now checks the selection generation independently of the row update. |

Both resume regressions failed on the baseline and passed with the new guards.

### Missing recommendation refresh on a healthy stream

[#2129](https://github.com/Rambolarsen/orkworks/issues/2129) tracks a stale or
unavailable Actions list on the status-first stream. After a missed watcher
event or a failed list request, the connected desktop waits for invalidation
because fallback polling only runs while the stream is unhealthy. The
publisher's 30-second reconciliation previously refreshed sessions and rearmed
watches without invalidating recommendations. The regression test
`periodic_reconciliation_invalidates_recommendations_without_a_watcher_event`
in `crates/orkworksd/src/workspace_updates/split.rs` advances that deadline
without changing the producer revision, then checks that the publisher
invalidates recommendations. The same test checks that a subsequent producer
event still invalidates promptly.

### Resume reached the backend after admission changed

[#2152](https://github.com/Rambolarsen/orkworks/issues/2152) tracks the
dispatch race. The controller awaited its backend URL, then sent the resume
before checking whether the workspace still owned the operation. Tests delayed
the URL result until admission closed and reopened, a replacement workspace was
adopted, or the controller was disposed. Each ordering sent a request on the
baseline. The current controller checks admission and the exact pending-resume
reservation before dispatch. A separate test confirms that two valid resumes
for different sessions can still dispatch in the same workspace.

### Nonzero PTY child exit was recorded as a successful end

[#2108](https://github.com/Rambolarsen/orkworks/issues/2108) tracks the loss
of the child exit status. The wait task discarded the status. The audit
reproduced exit code 23 emitting an `ended` event. The merged fix carries
the exit status through output drain. Its regression test
`runtime_exit_reports_nonzero_child_status` runs a real PTY child that exits
with code 7 and checks both the error event and the finalized outcome.
Successful exit and an explicit stop still record `ended` and `killed`.
[Implementation and regression test](https://github.com/Rambolarsen/orkworks/pull/2145)

### Closed terminal socket was reused on reselection

[#2163](https://github.com/Rambolarsen/orkworks/issues/2163) tracks a live
terminal that stayed read-only after its socket closed. In the Electron shell
fixture, returning to the running session reused the same closed handle.
The renderer now replaces a closed handle without a terminal end event when
reattaching to a live session. The regression verifies that input reaches the
replacement socket and that returning to a healthy session reuses its handle.

### Recommendation actions left a stale proposed card after refreshing

[#2182](https://github.com/Rambolarsen/orkworks/issues/2182) tracks a stale
actionable card after a successful Dismiss or Run cleanup. The Electron shell
scenarios defer the action reply and publish a same-workspace invalidation with
the still-proposed row. Both actions formerly skipped their follow-up refresh
because the read advanced the same generation. A third scenario delays the
pre-action list reply until after cleanup succeeds. Action settlement now uses
the workspace generation. It clears the notice for the affected card and waits
for a fresh list when a pre-action read is still pending.

### Observation rewrite fixtures could expire before testing retention

The successive-rewrite fixture returned `Storage(Expired)` in isolation. The
byte-limit rewrite fixture also expired in the full suite and passed on rerun.
Both exercised the production preflight deadline while testing retention and
accounting. They now use the existing test-only numeric deadline. A dedicated
batch test expires after inventory and checks that no sequence or segment
appears. The existing after-counter test confirms that an admitted batch still
settles past the original deadline.
[Issue #2194](https://github.com/Rambolarsen/orkworks/issues/2194)

### A stale Ollama verification replaced an edited URL

The Settings component regression starts verification, edits the URL, and
then resolves the old reply before the replacement's debounce expires.
Previously the old canonical URL replaced the draft and the old model catalog
became selectable. The test now verifies that the draft survives, the old
catalog stays hidden, and the new reply supplies the verified URL and models.
[Issue #2212](https://github.com/Rambolarsen/orkworks/issues/2212)

### Repeated folder requests opened a second chooser

Two Open other folder requests queued before the first finished previously
invoked the chooser twice, including after cancellation. A request made during
restoration could also queue another chooser to appear after the workspace was
ready. The coordinator tests defer selection and restoration to reproduce both
orderings. Requests now share the first chooser through settlement, and the
workspace history controls disable Open other folder while switching.
[Issue #2233](https://github.com/Rambolarsen/orkworks/issues/2233)

### Concurrent resumes dropped a successful row update

Two Resume requests could dispatch for different rows, but the second request
changed the foreground generation and discarded the first successful reply.
The controller tests defer both replies, settle them in either order, and
check each row before a new list arrives. The result now updates the row
while it owns the reservation. Foreground and selection checks still govern
focus, and a newer organization action invalidates an older resume on that row.
[Issue #2244](https://github.com/Rambolarsen/orkworks/issues/2244)

### A failed hook status refresh hid an active warning

The Electron fixture shows the Codex trust warning and runs its scheduled check
with a `503` response. The original implementation cleared the warning and
left no timer. The warning now remains visible until the retry confirms the
hook is active. The same fixture checks that a late response from a previous
workspace cannot reintroduce a warning.
[Issue #2255](https://github.com/Rambolarsen/orkworks/issues/2255)

### A slow selection write could restore the wrong session on restart

The sidecar remembers the last active-session request it handles. The
controller regression delays the first write until after the second selection
is made. Previously the older write could finish last and leave the older ID
in workspace memory. Writes now settle in submission order within each
workspace and admission lifetime. Tests verify that rejection releases the
next write and workspace replacement retires queued writes.
[Issue #2262](https://github.com/Rambolarsen/orkworks/issues/2262)

### A late create receipt rewound a newer session row to Creating

The create endpoint returns the pre-spawn row. The controller tests delay that
reply until the session list has published either Running or Failed. The
original controller replaced both rows with Creating and deferred the failure
notification until another list. It now retains the observed row and reports
an already-observed startup failure once the receipt confirms ownership.
[Issue #2273](https://github.com/Rambolarsen/orkworks/issues/2273)

### A successful Delete left its row visible until refresh

The controller test delays both a pre-receipt list and the post-delete read.
The sidecar acknowledges deletion, then the current read fails. Previously the
desktop disposed the terminal but still showed the deleted row. It now removes
the acknowledged row immediately, keeps a newer selection, and rejects the
older list. Failed deletes still reconcile against an authoritative list.
[Issue #2281](https://github.com/Rambolarsen/orkworks/issues/2281)

### Reselection reused a terminal socket during its close handshake

The Electron fixture marks an inactive terminal socket Closing before returning
to its still-live session. The old cache rule only replaced Closed sockets and
reused a transport already shutting down. Reselection now replaces the handle
in either state. The fixture delivers the old socket's close event afterward
and verifies that the fresh handle still accepts input.
[Issue #2289](https://github.com/Rambolarsen/orkworks/issues/2289)

The inventory covers the workflows above at their ownership boundaries. It does
not exhaustively enumerate every harness-specific adapter, release job, or
inactive storage path. Keep the transition row and its regression evidence in
sync when a later change touches one of these guards.
