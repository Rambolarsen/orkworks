---
type: Design Proposal
title: Research recovery that clears the final plan-completion condition
description: Proposed disposition-only recovery of an already reported research task, preserving original result and native lineage while coalescing recovery and automatic plan completion.
tags: [orkworks, research, recovery, completion, source-contract]
status: draft
workflow_status: proposed-for-scoped-recovery-completion-review
---

# Research recovery that clears the final plan-completion condition

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `dd8c18a3ef5ed22818cf61ae1ebaa81477e96e49`.

**Written proposal for three scoped owner choices.** The owner authorized closing
the positive recovery-to-completion source contract after #1768. The new ordering,
eligibility and compound-effect choices below await acceptance before dependent
fixtures. Earlier [completion acceptance](2026-11-15-research-completion-design.md#scoped-owner-acceptance-—-2026-11-15)
remains valid. This document adds no runtime writer or physical profile, and does
not assert that a real installation has produced the proposed prestate. Gate 2
remains **Investigate**.

## Problem and selected outcome

The [offline completion packet](../../validation/research-completion-components.md#completion-synthesis-and-invalidation-alternatives)
correctly denies completion for its retained observe-terminal recovery: the
original task still has needs_parent_result. Terminal observation establishes no
parent outcome. Preserve that example and every prior artifact byte.

A positive case needs an independently justified reported_complete task **before**
recovery, while an unsettled original disposition still prevents plan completion.
The selected proposed case is a late disposition interruption after turn readiness,
followed by an explicit parent outcome while the run remains blocked. The exact
original successful spawn is already proven and retained; the uncertainty concerns
later disposition/reconciliation. A subsequent observe-terminal decision can
settle that uncertainty without changing the original outcome or launching work.

The interruption is a disposition gate, not a failed-task result. It preserves
the existing turn readiness; subsequent observations preserve any accepted
coordination result. The relevant live
control change still advances taskVersion. This preservation/ordering rule is a
new positive source contract proposed here, not inferred from an enum or accepted
merely because a snapshot can be encoded.

Use the one-task retained-file research profile and existing four recovery kinds.
Only observe-terminal is proposed as the positive final-condition case here.
No extra task, retry attempt, allocation, task-success inference, new recovery
kind or model-authored settlement is introduced. Other kind-specific source
contracts remain intact; this bounded selection is not a proof that no broader
recovery case could ever complete a plan.

## Source constraints and decision basis

| Source | Constraint retained |
| --- | --- |
| [Baseline turn, coordination and completion](2026-09-26-taskmaster-orchestrated-child-sessions-design.md#approved-plan) | A real parent result follows the exact turn gate; every task reported_complete, reservations attached/settled and no interrupted allocation/pending revision are required; resolving the last condition reruns completion atomically |
| [Common task/launch/session controls](2026-11-02-child-launch-control-design.md#task-launch-and-ordinary-session-controls) | Interrupted retains every proven allocation/attachment/runtime; ordinary resume never rewrites original spawn or task/result history; capacity derives from actual owners |
| [Common recovery semantics](2026-11-02-child-launch-control-design.md#original-coordination-result-and-recovery-decisions) | observe-terminal requires exact current ordinary producer revision and terminal state; no-child stays launch_interrupted; allocation/attachment confirmation cannot bless an incomplete transaction |
| [Research source versions](2026-11-12-research-input-control-prefix-design.md#proposed-current-control-records-and-sole-version-ownership) | research_run_controls is sole current run owner; each relevant launch/session change advances live taskVersion once, independently of frozen assignment P |
| [Accepted research recovery representation](2026-11-15-research-continuation-design.md#typed-original-attempt-research-recovery-proposed) | Capture all task-related blockers and original typed snapshots/H; retain recovery-approval-required; immutable original request/receipt and no new attempt |
| [Accepted completion trigger/read/receipt](2026-11-15-research-completion-design.md#original-completion-and-ui-request-codecs-proposed) | Recovery trigger pins exact original decision digest; both decisions capture pre-trigger state and share one coalesced outcome |
| [Pause and resume precedence](2026-10-04-orchestrator-preparation-design.md#state-and-transition-table) | Stage gates, blockers and clarification survive; completing a research plan does not revive a parent or bypass planning gates |

These are source-contract constraints, not evidence of actual writer participation.
The older finite packets are documentation inputs, not a current store from which
to elect a prestate. Required source/native/ordinary ownership and direct proofs
must be independently available; unknown or conflicting ownership denies.

## Proposed positive prestate and ordering

All conditions below are required together for the selected positive branch:

1. One exact original research plan is current, with status active or paused and its original
   definition, approval and sole task/reservation/assignment retained. The run is
   nonterminal; no pending scope revision/proposal remains. The lifecycle selector
   exists from original admission. No completion record for this exact PlanRef
   already exists. Existing completion records use their original outcome path.
2. Original allocation, complete preparation/attachment and successful first spawn
   were proven under the original launch admission. Keep original AllocationRef,
   attachmentId, frozen P, ApprovalRef, admission parent RuntimeRef and nonnull
   spawnRuntime. Their direct source/proof joins exist independently of a current
   ordinary runtime. No missing ledger, unknown original preparation commit or
   unproven first spawn can be repaired by this case.
3. The actual child acquired its original turn readiness through an eligible Stop,
   exact UI action or coherent terminal-without-receipt owner event. Retain that
   full original turn body/digest/proof. Its acceptedTaskVersion can be older than
   today's live taskVersion; readiness is historical source, not a live-version
   equality shortcut.
4. Before the parent outcome, a source-owned late disposition interruption fenced
   launch eligibility and recorded launch.phase interrupted while retaining every
   established allocation/attachment/spawn field. The selected concrete shape uses
   interruptedAt spawn and reasonCode disposition-unknown, with nonnull original
   spawnRuntime. It describes uncertainty about later disposition, not an unknown
   first-spawn success or permission to create another process. Source proof must
   establish that this event actually fits that meaning; strings do not prove it.
5. That disposition-only event preserved needs_parent_result and its original turn
   gate, advancing the live taskVersion once for the relevant launch change. A
   failed/blocked parent result, missing turn or original pre-launch interruption
   cannot be relabelled into this case. The event records its exact task-related
   launch-interrupted blocker and all other mandatory fencing/source effects.
6. A current eligible parent then accepted an original completed coordination
   request against the new live taskVersion, after reviewing the exact report and
   every required gate. Result collection for an existing assignment remains
   allowed while blocked. The task became reported_complete with its immutable
   coordinationResult requestDigest/receipt. Completion was checked in that same
   outcome and correctly remained unavailable because disposition was unresolved;
   this is not a chosen delay of an otherwise eligible completion.
7. Before the recovery capture, the ordinary owner published/recovered a coherent
   current terminal bridge with its exact producerRevision and current RuntimeRef.
   This observation alone did not settle unknown native ownership/disposition.
   Keep the launch interrupted until its source-owned settlement is proven. A
   real bridge change advances its producerRevision and relevant live taskVersion
   once; a repeated observation does not invent another bridge event.
8. At the reviewed H, Main and the source owner capture/revalidate the exact
   terminal bridge, native-object ownership, quiescence of the original operation,
   complete retained source/consumer membership and current read set. All required
   streams/result joins remain valid; no unsupported review/clarification/execution
   variant is hidden. Every original native operation has a proven disposition;
   an unknown operation, lease, commit or source generation prevents success.

This closes the proposed *eligibility contract*. A later offline packet can
supply explicitly hypothetical independent source snapshots satisfying these
obligations; it cannot qualify their real occurrence, freshness or physical
atomicity. Actual event participation and a legal history still need qualification.

### Illustrative live-version accounting

The following labels describe required relations, not an executed history or
admission constants. Frozen P stays at taskVersion 2 throughout.

| Required source outcome | Live task version | Run version | Preserved / changed fact |
| --- | ---: | ---: | --- |
| Original turn readiness | 7→8 | 10→11 | running→needs_parent_result, original turn receipt |
| Late disposition interruption | 8→9 | 11→12 | Keep readiness; launch becomes interrupted and launches fence |
| Explicit parent completed outcome | 9→10 | 12→13 | reported_complete; coordination receipt acceptedTaskVersion 10; plan stays nonterminal because launch is interrupted |
| Actual current terminal bridge event | 10→11 | 13→14 | Preserve outcome; producerRevision advances once; unresolved native settlement remains interrupted |
| Proven observe-terminal settlement plus completion | 11→12 | 14→15 | Launch interrupted→ended; preserve outcome; planVersion 2→3/complete; one run advance |

The sample's final task advance is required because recovery changes launch
semantics, even though task.status and coordinationResult stay unchanged. The
coordination receipt remains at acceptedTaskVersion 10, the turn receipt at 8,
and P at 2. None is retokened to 12. This differs from the old recovery sample's
already-ended/no-task-change observation, whose receipt correctly retains 8.
If other real events occur, derive their actual versions; never force these
sample values, omit a resume mutation or add increments to fill a receipt.

## Exact existing representations and compound outcome

Keep ResearchRecoveryDecisionV1, its four kinds, ResearchRecoveryReadSetV1 and
ResearchRecoveryReceiptV1 unchanged. Use the existing recoveryId/TaskKey and
orkworks.research-source-recovery-decision.v1 + LF digest. Its selected kind is
observe-terminal; expectedTaskVersion equals the live pre-recovery task version,
not the frozen P or older coordination receipt. Allocation/attachment/session
are nonnull and retain the exact original identity and current terminal bridge.
blockers is the complete original sorted task-related blocker set, not a caller's
chosen removable subset.

Keep ResearchLifecycleDecisionV1/RequestV1/ReadSetV1/ReceiptV1 unchanged. Automatic
completion key remains `{kind:"plan-completion",plan:PlanRef}` and its trigger is
exactly `{kind:"recovery",task:TaskKey,recoveryId,decisionDigest}`. That digest
pins the full original recovery body. No trigger CommitRef/new C is added.
The lifecycle expectedPlanVersion is the current nonterminal plan version.

Both readSet.expectedCommit values equal the same original H. Their run reads
are byte-identical original table/key/value snapshots, including lastCommit and
expectedRunVersion/currentInput. Recovery task/launch/session and lifecycle
selector/current-plan/pending-plan reads all describe that same pre-trigger state;
no lifecycle read is reconstructed from the candidate repaired poststate. The
full actual read set also includes all immutable turn/result/approval/allocation/
attachment/runtime/input/report/native/consumer/resource/index/lease/health
owners and direct source/proof correspondence, not just these named snapshots.

Compute the recovery candidate effects first, then evaluate the accepted complete
plan predicate over that candidate. For a successful new selected publication:

- Insert the original recovery record/receipt and repair this launch to ended,
  retaining original admission/allocation/attachment/spawn. Clear only the launch's
  interruptedAt/reasonCode; no history is deleted. The terminal bridge/runtime/
  producerRevision stay unchanged because this recovery observes an already
  coherently published ordinary terminal event.
- Remove only captured launch-interrupted blockers for this exact original attempt
  whose complete source-owned predicates are now proven resolved. Kind, task ID,
  free-text detail or path equality alone cannot prove that correspondence. The
  actual owner must resolve every such source relation; an unsupported/ambiguous
  relation makes publication unavailable. Preserve every independent blocker,
  including recovery-approval-required, native-unavailable or source-changed,
  unless its separate closed owning transition resolves it. Never clear all task
  blockers merely because they were captured for Main review.
- Advance the live taskVersion once for the launch repair, preserving exact
  reported_complete/coordinationResult and every original request/receipt/turn.
  Do not submit another result, retry the task or introduce a task outcome kind.
- With all declared tasks complete, reservations attached/settled, no interrupted
  allocation and no pending scope revision, insert the one lifecycle completion
  record, mark the exact plan complete/planVersion+1, revoke its exact grant,
  clear currentPlan and select completedResearchPlan. Clear an obsolete current
  summary under the accepted owner rules; preserve summary high-water/history.
- Advance sole research_run_controls.runVersion once for the union. Recovery
  receipt records that runVersion and the actual new taskVersion; lifecycle
  receipt records the same runVersion and actual new planVersion. Every changed
  local source/control/selector row and both receipts bind the same actual C.
  No second journal operation, manual finish or parent completion request follows.

A retained unrelated blocker need not erase a true completed plan: it keeps the
run blocked. Retained runtime pause/clarification/stage precedence remains intact;
only an eligible live parent with no planning gates enters synthesizing. An
actually interrupted parent remains paused with its original pause reason and
synthesizing resumeTarget after plan completion; higher-priority stage/blocker/
clarification gates still win on resume. No bearer
is revived or grant recreated. This case grants no launch and does not select
execution. No pending plan, source change or required review is silently discarded.
The research collection/expectation remains active after plan completion, allowing
eligible child reporting under the existing separate authority rules.

Capacity follows actual original native/ordinary ownership. This terminal child
can count zero only when complete membership/terminal/disposition facts prove it;
the result status or a decision label releases nothing. Retain the reservation,
ordinary session and worktree ownership records. Other nonterminal children still
count across all retained run plans. Ordinary resume remains independently gated
and may consume capacity again without reopening this completed plan/task or
restoring an orchestration grant.

## Explicit offline blocker correspondence

A future component packet must supply a separate **fixture-only stipulated input**
LateDispositionBindingsV1, exactly:

```text
{schemaVersion:1,expectedCommit:CommitRef,bindings:[
 {blocker:ControlBlockerV1,reservation:ReservationRef,
  interruptionRun:ResearchRunControlRead,
  interruptionTask:ResearchTaskControlRead,
  interruptionLaunch:ResearchLaunchControlRead,
  disposition:"supplied-resolved"|"supplied-unresolved"|"supplied-unknown"}
]}
```

This is neither a persisted record nor a new trusted evidence/source kind. Do not
embed it in either accepted decision body, digest, receipt or catalog. It makes
the hypothetical source-owner correspondence explicit rather than teaching a
checker to infer it from kind/task/detail. Actual complete native/source discovery
and direct proof remain externally supplied and unqualified.

expectedCommit equals the shared recovery/lifecycle H. The positive branch has 1..16 binding
entries, unique and UTF-8-sorted by blocker.id. It covers **exactly every captured
launch-interrupted blocker for this task** in the recovery's complete task-related
blocker set. Each full blocker body matches that captured body byte-for-byte;
missing, extra, duplicate or altered bindings deny. Other kinds are not removable
through this relation and remain retained, including recovery-approval-required.

Each entry's reservation equals the decision's exact original reservation. Its
three typed snapshots use the unchanged research run/task/launch families and
keys. All have one original interruption CommitRef I in value.lastCommit, in the
same epoch as H and at/before H, and require their original direct source/proofs.
The interruption run body includes that exact blocker/current plan; the task is
needs_parent_result with its original reservation and null coordinationResult;
the launch is the same original attempt, interrupted at spawn/disposition-unknown
while retaining proven allocation/attachment/non-null original spawnRuntime.
Retain the original turn proof and later coordination source separately: the
source-owner ordering is turn strictly before I, then I strictly before the accepted
coordination outcome, which is at/before H. Those original values cannot be
rewritten to the later reported_complete task or captured current run.

A positive selected publication requires all covered entries supplied-resolved,
with each resolution asserting the exact current terminal/native-quiescence and
original-ownership predicates for **that** retained attempt. The string is a
stipulated test input, never proof or self-authorizing production data. Any
supplied-unresolved/unknown entry, conflicting origin/reservation, incomplete
relation or unsupported source predicate makes this positive publication
unavailable; no convenient subset is cleared. Multiple blockers for the same task
need separate exact entries and cannot be collapsed by task/kind equality.
Other independent blockers stay unchanged and can keep the run blocked after
true plan completion. The existing snapshot/discovery/command/job limits apply
to all copies; none of these optional-size examples proves full packing fits.

## Admission, exact retry, races and unknown disposition

Current Main authority, exact workspace/epoch/owner/lease/health and shown original
recovery context/generation precede retained recovery lookup. An identical
accepted recovery body returns its original receipt before live CAS/count, with
no new completion record, version, proof, event or charge. Changed bytes under
the same recovery key conflict. It must resolve the original paired completion
record/outcome when that recovery actually completed the plan; it cannot invent
one on retry. A missing/conflicting original companion or proof is unavailable,
not repaired by another recoveryId or a live recheck against new state.

New admission compares both captured decisions and their complete original state
against the current owner. A source/bridge/native/runtime/input change after Main
review denies the stale decision. Re-capture/review needs a new decision identity;
it cannot mutate an occupied ID. If a concurrent recovery/coordination already
completed the exact plan, resolve its owning original record instead of appending
another completion under a changed trigger or plan version.

An actual terminal observation is not a current parent credential. Recovery
settlement and grant fencing serialize with all launch/report publication at the
qualified boundary. If cancellation wins, no new positive completion may reopen
its cancelled plan/run. If this complete outcome wins, later cancellation must
revalidate/include it under its own exact UI contract. Exact original authorized
retry can return historical receipts after terminal state; a new terminal recovery
writer is not qualified here. Terminal ordinary lifecycle/audit effects retain
separate ownership and cannot restore automation.

Unknown original transaction/native disposition retains the sole original owner,
attempt, protected worktree and capacity. Ordinary stop/end remains independent;
no hierarchy SQL/fsync is held in its input-critical path. An unproven commit/abort
is unavailable/reconciling, never a successful terminal receipt, timeout rollback,
replacement worker or late spawn. The complete actual compound publication,
nonblocking boundary and before-image/direct-proof qualification remain open.

## Catalog, membership, projections and resource obligations

This proposal adds **zero persisted** families, source kinds, field variants, resource names
or domains. Reuse candidate 57 and the accepted original-admission selector; root
counts 28/25 remain conditional. No new initialized ledger, alternate run_states
counter, format registry entry, row hash, manifest or storage migration follows.

Recovery and completion records each charge their full exact key plus committed
projection to the existing shared control pool: 8 MiB/run, 64 MiB/workspace and
32 MiB run aggregate. Copied run snapshots/H/origins occur twice and are charged
twice. Selector/launch/task/plan/run/other owner deltas coalesce original-to-final
once per ledger. No discount, source transfer, missing-ledger backfill or refund
is authorized. Two separately small records do not prove one full command fits.
Keep 2 MiB decision wrappers, 4 KiB receipts, 1 MiB run / 16 KiB task/session/
plan/selector / 32 KiB launch snapshots, the smaller 1 MiB paired job/ordinary
ceiling, 4,096 changed rows/16 MiB command, bounded discovery and physical/scratch
limits. Any smaller bound/exhausted counter denies the entire mutation.

Only each outer lastCommit and its own receipt.commit are excluded from that
record's projection. Original snapshots/H and the recovery digest inside the
lifecycle body remain committed. Build the recovery body digest, then the lifecycle
body digest, then both receipts' committed fields and row projections. Only a
qualified physical publisher subsequently binds C and its excluded outer links;
no physical commitment recipe is supplied here. There is no digest fixed point
or receipt reconstruction from today's controls. Complete actual proof membership
retains original direct origins at/before H in one epoch, full before-images,
original source correspondence and new outcome proofs; it does not replay or elect
maximum predecessors to recover missing history.

Both directions of source/current/historical consumer membership must retain the
original run/bootstrap/plan/approval/reservation/P/allocation/attachment/spawn/
current ordinary runtime, turn/coordination/report/input/assessment, recovery/
lifecycle/selector/summaries and all copied snapshot/proof origins. Include zero-
reference owners and retired/tombstoned lineage. The complete union includes #744,
#745 and #746 consumers, active preparations, indexes, resource ledgers and native
owners. Unknown/unsupported owners deny; a convenient list of matching edges does
not prove completeness. The 4,096 observations/16 MiB and 100,000-visit discovery
limits remain shared with final validation; do not truncate or split the outcome.

## Three owner choices and bounded next packet

Accept or revise these new choices for **offline components only**:

1. Permit the explicitly source-owned late disposition interruption after a proven
   original spawn/turn, preserving readiness and subsequent immutable outcomes while
   fencing launches and advancing relevant live versions. Require proof that the
   interruption preceded the blocking parent outcome; no failed task or old
   incomplete recovery is relabelled as success.
2. Admit the selected observe-terminal settlement only with complete original
   preparation/spawn/current terminal/native lineage. Repair the launch and its
   proven disposition blockers, advance live taskVersion once and preserve every
   immutable outcome and independent recovery-approval/blocker. Supply the exact
   fixture-only blocker correspondence above; native/source facts remain
   externally supplied and unqualified in offline examples.
3. Publish recovery and automatic completion as one original H/C outcome using
   unchanged typed read sets/triggers/receipts, shared run version and complete
   existing family/accounting/membership rules. Preserve original paired retry,
   capacity, stage/runtime/terminal precedence and all qualification gates.

After scoped acceptance, a bounded independent Python/JSON/data-only JavaScript
packet may construct new original source-derived snapshots and literal version/
result/trigger/receipt checks, including the required negative paths and all prior
byte preservation. It may not edit the old negative recovery fixture, assert a
legal history or generate a full physical catalog/header/manifest/row/change hash.
No native/model/runtime call, product module, adapter, migration, executable
product plan or activation is authorized. Actual writer participation, native/
authority/source proofs and complete physical catalog/history remain open.
#1689/#1089 remain open; #740 remains closed research. Gate 2 remains Investigate.

## Bounded review handoff

One writer owns this proposal. One bounded round permits two fresh read-only Luna
reviewers: source/lifecycle correctness and a builder check for important missing
choices. Cap two reviewers; no edits or native/runtime/model/database probes.
Resolve material findings before PR handoff. Verify docs tests/build/anchors,
append-only source routing and all prior non-Markdown validation artifact bytes.
Publication or merge alone does not accept the three choices.


### Bounded review disposition — 2026-11-15

Two fresh read-only Luna reviews completed in one round. Correctness review found
no material issue. Builder review identified the missing exact correspondence
between a removable blocker and its original late interruption. This revision
specifies the fixture-only LateDispositionBindingsV1 input, original typed
run/task/launch snapshots, complete exact blocker coverage, before-coordination
origin order and resolved/unresolved/unknown denial rules. It does not add a
persisted field/family or qualify an actual source/native proof. The three owner
choices remain pending acceptance; no dependent fixtures were changed.

## Scoped owner acceptance and offline handoff — 2026-11-15

The owner answered “keep going” to the three-choice handoff following #1769.
All three choices above are accepted for independent offline components only.
The earlier proposed/pending notes record the proposal's original review state;
this scoped acceptance supersedes them without changing its technical body.
The [positive recovery/completion packet](../../validation/research-recovery-completion-components.md)
supplies new hypothetical source views, original paired records, exact retry and
denial/occurrence checks. It preserves the old negative example and prior bytes.
No actual authority/native/source/ordinary proof, legal physical history,
publisher, format or runtime eligibility follows. Gate 2 remains Investigate.
