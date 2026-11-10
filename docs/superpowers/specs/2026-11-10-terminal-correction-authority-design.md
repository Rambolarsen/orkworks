---
type: Design Proposal
title: Terminal research-report correction authority ordering
description: Proposed exact-operation cutoff and source-owner handoff for terminal user corrections before native publication qualification.
tags: [orkworks, hierarchy, reports, authority, qualification]
status: draft
workflow_status: proposed-for-owner-review
---

# Terminal research-report correction authority ordering

Tracking: [#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#742](https://github.com/Rambolarsen/orkworks/issues/742), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `cd6415f0384561d52b952a4c7d20fdd3c48275a7`.
This is Gate 1's written investigation and concrete proposed decision from the
[terminal publication qualification plan](../plans/2026-10-29-terminal-report-publication-qualification.md#gate-1--exact-ui-authority-and-ordering-decision).
**Owner acceptance is pending.** Existing #742 authority rules remain binding.
No new authority, codec, fixture, runtime plan, native probe or implementation is
authorized by this proposal or its merge.

## Outcome and existing evidence

The accepted [current-summary detail contract](2026-10-26-hierarchy-summary-detail-design.md)
and its component/source/packing packets already exist. The
[terminal correction packet](../../validation/report-publication-components.md)
shows conditional complete row effects for the same retained evaluation under
stale required C1 versus stale optional O. It does not establish that a real
producer was entitled to commit those rows. This proposal advances that authority
boundary; it does not repeat or replace the read DTO, aggregation or viewing rules.

The [terminal profile](2026-10-29-report-publication-profile-design.md#authority-snapshots-and-recovery)
currently requires current UI authority through durable disposition, excludes
#744 claim-first, and holds native execution no-go without an enforceable fence.
Completion/cancellation and ordinary child end do not themselves revoke an
otherwise eligible explicit current-epoch terminal user correction. Child report
and launch authority remain closed. A report receipt proves neither terminal
settlement nor live authority.

## Source trace: implemented precedents and missing stages

The following is inspection evidence at the revision above, not executed native
qualification. Function names identify the inspected seams; links resolve their
source files. No terminal report-correction pipeline was found by searching
`correctResearchReport`, `TerminalReportRunV1`, `ReadHierarchySummary`,
`report_head` and `research_report` in Electron, sidecar HTTP and coordinator
modules. That bounded search does not prove absence of every differently named
or external producer.

| Stage / owner | Actual source evidence | Missing proof for terminal correction |
| --- | --- | --- |
| Main-process request scope | [main.ts](../../../apps/desktop/electron/main.ts), `withReadyBackendGeneration` captures readiness/generation, registers an AbortController and checks generation again after the operation | No exact terminal-correction intent object or sidecar commit handshake |
| Fixed transport | [taskmasterSettings.ts](../../../apps/desktop/electron/taskmasterSettings.ts), `taskmasterRequest` chooses fixed resources and calls `assertCurrent` before/after fetch and response reading | Resource union has no report correction; response-time checks cannot undo persistence |
| UI scope closure | [workspaceSwitchCoordinator.ts](../../../apps/desktop/electron/workspaceSwitchCoordinator.ts), `closeCurrent` calls `onCloseAdmission` before awaiting runtime close; [main.ts](../../../apps/desktop/electron/main.ts)'s callback cancels requests and increments `backendGeneration` | No shared ordering between that local callback and a remote hierarchy SQL commit |
| Sidecar identity check | [taskmaster_settings_handlers.rs](../../../crates/orkworksd/src/http/taskmaster_settings_handlers.rs), `authorize_taskmaster_request` compares the configured main-process token with the request header | Authentication alone supplies no exact intent, selected-scope generation or revocation ordering |
| Selected runtime precedent | [readiness_handlers.rs](../../../crates/orkworksd/src/http/readiness_handlers.rs), `assess`/`get_status` authorize and resolve Taskmaster runtime | These are readiness operations, not report admission, bounded publication queue or final validation |
| Lease lifetime | [workspace_runtime.rs](../../../crates/orkworksd/src/workspace_runtime.rs), `WorkspaceLease::acquire` holds an advisory lock on an open file and `Drop` releases it | A future report worker must retain the original owner through disposition; path/PID/file existence is insufficient |
| Terminal creation | [coordinator_store.rs](../../../crates/orkworksd/src/taskmaster/coordinator_store.rs) owns durable plan data; ordinary session ending is separately owned under [session application](../../../crates/orkworksd/src/session_application.rs) | Neither precedent creates the profile's proven terminal run, closed grants, initialized report history or complete retained consumer inventory |
| Commit, reference publication and response | Existing [conditional packet](../../validation/report-publication-components.md) and [profile](2026-10-29-report-publication-profile-design.md) specify effects | No qualified production correction transaction, universal writer invalidation, authority owner or response commitment was demonstrated |

The current main-process `closeCurrentRuntime` callback cancels restoration and
awaits `sidecarLifecycle.stop()` before another workspace opens. It does not
implement an independently draining hierarchy worker across a workspace switch.
Preserving ordinary operation while retaining old hierarchy ownership therefore
needs a separately reviewed lifecycle/ownership integration; the existing callback
is not evidence that the proposed drain behavior is available. The lifetime cap
must cover the retired owner before any replacement hierarchy admission; changing
processes must not bypass it. This packet proposes authority semantics, not a new
sidecar topology or permission to change current workspace cleanup.

## Alternatives and proposed decision

1. Preserve revoke-before-physical-commit denial. This remains a valid contract,
   but no qualifying native mechanism is supplied here. A final precheck, abort,
   interrupt or suppressed response leaves the persistence race unresolved.
2. Serialize UI closure/ordinary stop behind storage. Do not recommend this:
   storage can stall, and the existing responsiveness contract forbids ordinary
   stop waiting for SQL or holding application/publication locks across I/O.
3. **Recommend one separately scoped terminal user-correction admission.**
   Serialize its exact admission against sidecar-observed scope closure using
   memory-only ordering. Admission-first may finish that one attempted transaction;
   closure-first denies it. Durable acceptance remains at SQL commit. This changes
   #742's cutoff and needs explicit owner acceptance; #744 acceptance does not cover it.

This proposed permission applies only to the existing terminal/evaluation-only
profile: explicit Electron-authorized correction of an existing research-report
head in a proven completed/cancelled current-epoch run. It excludes initial or
child report submission, active preparation, proposal/plan/clarification changes,
run creation/finish/cancel, grants, launches, evaluation publication, evidence
mutation, purge, deletion, shared learning and arbitrary SQL. Unsupported retained
consumers still make the entire operation ineligible. No report is coerced into
#744's claim class and no terminal state or bearer is reopened.

### Exact authority and local-versus-sidecar closure

A future private sidecar authority owner must bind admission to the original
Electron-selected workspace/sidecar lifetime and UI scope, exact terminal run and
epoch, complete report stream, predecessor version/digest, canonical request
fingerprint and request key, immutable proposed descriptor bytes, and original
worker/store lease. These are semantic requirements referencing existing source
identities, not a new serialized schema or authority supplied by the renderer.
Missing, ambiguous, closing or exhausted identity denies admission.

The main process must validate sender/frame/origin and current scope before issuing
the request. Main closes local admission immediately when its scope changes and
suppresses stale delivery. **Local closure is not the proposed persistence cutoff.**
A request already sent may reach sidecar admission after the local UI changed but
before the sidecar processes closure. Under this proposal it may still persist.
The owner decision must include that distributed timing cost, not just an
already-running SQL transaction. No shared clock, cancelled fetch or Electron
counter proves sidecar closure.

The sidecar cutoff is the memory-only ordering of its exact admission versus
processing the matching closure. A future main/sidecar protocol must identify the
scope and acknowledge its closure after fencing unadmitted requests; out-of-order
requests for a closed scope remain denied, including after switching away and back.
A successful close acknowledgement proves admission is closed, not that earlier
admitted work rolled back or settled. If closure delivery/acknowledgement is lost,
main reports the old write disposition unknown and cannot promise persistence
denial. Recovery/replacement cannot adopt a pending intent. No automatic resend,
new scope or replacement worker resolves uncertainty. Exact transport, generation
allocation, acknowledgement codec and bounded closure state remain qualification
prerequisites, not invented fields in the existing report request.

### Execution and disposition

Queue entry and rate charging grant no surviving permission. The one worker
strictly prepares and validates the fixed intent outside the authority latch,
then admits it under short memory-only ordering with current scope closure.
No parsing, hashing, I/O, callbacks or waits occur under that latch. The existing
single active-or-retiring hierarchy worker remains shared with other domains;
this proposal adds no parallel worker, queue, budget or receipt pool.

After admission, the original worker may attempt its one transaction after scope
closure. It must validate source heads, complete dependency/index membership,
current epoch, health, quotas and lineage at their owning transaction boundary.
Permission does not freeze source eligibility. Failure rolls back; no automatic
retry or second admission follows. Unknown commit disposition retains the original
slot, store and lease and fences affected reads/actions until proven disposition.
Timeout or response loss proves neither commit nor rollback.

On commit, retain the profile's complete atomic effects and durable acceptance
point, then invalidate every affected run snapshot before releasing the publication
position or serving current reads in that store. Detached old-scope completion
must not publish into the newly selected workspace. It records durable disposition
in the original store for later recovery; no old event may clear a new scope's
failure fence. Response delivery separately requires the original main/frame and
scope still current. A stale caller can receive no success while its write has
committed; the UI must not describe that as a rollback. Later exact retry requires
fresh authorization/rate admission and returns the original receipt under existing
idempotency rules, without repeating effects or invalidations.

## Proposed per-event authority table

| Event | Persisting the exact operation | Delivery / lifecycle |
| --- | --- | --- |
| Run finish/cancel or ordinary parent/child end | Terminal user correction remains eligible only under the existing proven terminal profile; these events create no permission and prove no settlement | Child/grant authority stays closed; ordinary stop never awaits correction SQL |
| Main selection change, detach, window close or request cancellation | Close local admission immediately; sidecar closure-first denies, exact admission-first may finish; the transport interval above is an explicit cost | Recheck original frame/scope before response commitment; no hidden success or viewed-summary mark |
| Sidecar receives closure before a delayed or queued request | Deny admission for that old scope, including after a later return to the same workspace | Do not revive the original intent from a receipt or reused path |
| Namespace rotation | Close old-epoch admission; do not activate a new epoch until the old admitted operation has proven disposition; unresolved rotation returns conflict/unavailable | Old history is read-only after completed rotation; no counters or keys reset |
| Workspace switch / voluntary lease handoff | Retain the original lease until the sole old worker closes; admit no replacement hierarchy worker while it is active/retiring | Ordinary new-workspace operation must not wait on hierarchy SQL; hierarchy availability can remain blocked indefinitely |
| Actual lease loss, uncertain ownership, directory replacement or workspace deletion | No permission to write through lost ownership; fence and reconcile. Deletion/handoff must not discard the store/lease before disposition. No native eligibility without a demonstrated safe owner protocol | No replacement adoption, process-kill promise or journal deletion to manufacture completion |
| Source health failure, dependency insertion/removal or purge | Require complete source-owned serialization/currentness proof; reject or fence if eligibility cannot be preserved | A valid admitted UI intent cannot bypass protection, revive stale C1 or erase C2's known failure |
| Sidecar exit / replacement | Volatile admissions die; recover durable receipts/state only, never re-execute a lost operation | New lifetime and complete recovery precede reads; no resurrected bearer or stale response |

## Compatibility, recovery and acceptance boundary

Until explicit acceptance, this proposal does not amend the accepted terminal
profile or #744's exclusions. Existing immutable result/evaluation/report bytes,
summary material domains, read/viewing DTOs, request/receipt codecs and caps stay
unchanged. A future amendment must reconcile #742, the terminal profile, #744's
exclusion note and #746 read/publication expectations together. Any new authority
decision metadata, closure acknowledgement, schema version, retention bound or ADR
must be concretely reviewed before dependent fixtures/adapters; this document
selects none of their encodings or new numeric limits.

**One owner decision requested:** accept or revise the narrowly scoped admission
cutoff, including the local-to-sidecar closure interval, stale-response separation
and potentially indefinite retained old lease/worker cost, for further written
qualification only. This would not authorize execution, a runtime implementation
plan or Gate 2 completion.

After that decision, close the authority-owner/transport/metadata contracts and
Gate 2's real terminal producer, receipt-ledger initialization and complete consumer
proof. Only accepted closed contracts can feed new exact/adversarial fixtures.
Native qualification must then demonstrate both admission orderings, delayed/lost
closure and old-scope replay, stalled SQL with independent ordinary stop, source
and ownership races, crash/recovery and immutable retry. Retain the existing
C1 3→4 versus O 3→3 material behavior, snapshot invalidation for both and no viewed
mark from stale or hidden presentation. Existing writer/storage/native/#740 and
separately approved executable-plan gates remain open.

This documentation-only change can be reverted without a state migration. Future
native persistence recovery/downgrade is unqualified; a source revert cannot be
claimed to undo committed corrections.

## Review evidence and remaining handoff

Two fresh read-only reviewers checked the draft against the source/accepted
contracts and against Gate 1 completeness respectively. Both reported no blocking
findings. They confirmed the separate pending #742 amendment, distributed closure
cost, and distinction between SQL acceptance and response delivery. This is
authored proposal review, not independent producer agreement or executed native
evidence. A subsequent author clarification records the current stop-before-open
sidecar lifecycle and the still-missing draining-owner integration explicitly.
Gate 1 remains pending owner decision; Gates 2–3 remain unqualified.
