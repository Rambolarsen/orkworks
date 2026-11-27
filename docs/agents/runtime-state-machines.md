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
| Runtime `creating` or `alive` | Startup failure, process exit, or requested kill | `stopping` / phase `ending`, with terminal outcome recorded |
| Runtime `ending` | Owned finalizer persists terminal metadata | `dead` / phase `ended` |
| Runtime `ending` | Terminal persistence fails | Retains `ending`; the same generation may retry finalization |
| Organization `active` | Archive or confirmed Delete reserves the target and awaits durable ending | `archived` or removed |
| Organization `closed` | Archive, eligible resume, or confirmed Delete | `archived`, `active`, or removed |
| Organization `archived` | Restore or confirmed Delete against finalized state | `closed` or removed |

**Invariants.** Durable terminal metadata takes precedence over an ending handle
in live projection. Resume must reject an ending handle or archived history.
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
and `crates/orkworksd/src/workspace_updates/split.rs`.
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
| Resume idle | Resume admitted for the row | Pending, with one operation per row |
| Resume pending | Duplicate request, or old operation finishes after replacement | Current pending operation is unchanged |
| Resume pending | Current success | Update the row, fence older list reads, and select the resumed row if no newer session was selected |
| Resume pending | Current failure or admission closes | Clear progress for the owned operation; recover current data on failure |
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
`components/HistoricalTerminal.tsx`, and `crates/orkworksd/src/runtime/terminal_http.rs`.

| Machine and current state | Event and guard | Next state |
| --- | --- | --- |
| Live terminal absent | Still-current renderer awaits backend and attaches | Socket connecting |
| Socket connecting | Connection opens and the handle is current | Interactive, with bounded pending input |
| Socket attached | Socket closes or the component is disposed | Input disabled or attachment removed; PTY lifecycle proceeds independently |
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

The inventory covers the workflows above at their ownership boundaries. It does
not exhaustively enumerate every harness-specific adapter, release job, or
inactive storage path. Keep the transition row and its regression evidence in
sync when a later change touches one of these guards.
