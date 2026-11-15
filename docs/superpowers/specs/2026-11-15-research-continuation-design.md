---
type: Design Proposal
title: Research turn evidence, coordination and original-attempt recovery
description: Proposed positive research turn gate, coordination outcome and typed original-attempt recovery codecs with catalog, retention and accounting obligations before complete continuation qualification.
tags: [orkworks, research, coordination, recovery, catalog, qualification]
status: draft
workflow_status: proposed-for-scoped-continuation-review
---

# Research turn evidence, coordination and original-attempt recovery

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `9b4fb72454edf8413caa1bd535f765bccc7f570e`.

**New representation proposal for scoped owner review.** The owner authorized
progress on the remaining research continuation/catalog contracts. This is
permission to prepare that decision, not acceptance of the new codecs below.
Earlier [research authorization acceptance](2026-11-13-research-preparation-authorization-design.md#scoped-owner-acceptance--2026-11-15)
remains valid for its offline packet and does not accept these continuation
families. Merge publishes a proposal. Gate 2 remains Investigate.

## Outcome and selected boundary

Make positive turn readiness, original parent coordination and recovery of the
one original research attempt representable without pretending a report receipt
is task success or casting direct-execution history into research. Keep the
accepted one-task retained-file research source profile, original frozen P,
report/request/head/receipt bytes, and sole research_run_controls runVersion
owner. No generic event object, restored grant or model completeness flag is used.

The [accepted logical lifecycle](2026-10-04-orchestrator-preparation-design.md#logical-lifecycle-acceptance--2026-11-08)
requires automatic same-parent collection, synthesis and preparation after
successful research. This proposal closes only the turn/result/original-attempt
recovery representation seam needed on that path. It does not remove or fulfill
remaining finish/cancel, synthesis/execution handoff, material clarification,
replacement/recovery-plan source, physical catalog/index/proof or adapter work.
Unsupported positive variants stay unavailable; no field is flattened to null.

No dependent fixture, source hash vector, storage header, manifest, database,
Rust/Electron code, adapter, native/model call, migration or executable product
plan accompanies this proposal. A later component packet needs scoped acceptance.

## Source trace and ownership

| Source | Constraint established by bounded inspection |
| --- | --- |
| [Launch baseline turn boundary](2026-09-26-taskmaster-orchestrated-child-sessions-design.md#approved-plan) | Codex Stop, explicit trusted UI action, or actual terminal-without-receipt establishes needs_parent_result; exit alone never establishes success |
| [Research collection contract](2026-10-04-orchestrator-preparation-design.md#research-approval-and-result-collection) | Report receipt and task-turn receipt are separate; an explicit authenticated parent outcome follows the turn gate |
| [Common coordination/recovery codec](2026-11-02-child-launch-control-design.md#original-coordination-result-and-recovery-decisions) | Closed direct-execution request/receipt and full historical recovery snapshots; research tables/domains are not included |
| [Research current-control contract](2026-11-12-research-input-control-prefix-design.md#proposed-current-control-records-and-sole-version-ownership) | Research task/launch/session field sets reuse common semantics but positive coordination/recovery codecs remain unclosed |
| [Research membership recipes](2026-11-12-research-membership-publication-design.md#research-metadata-reference-recipes) | Nonnull unsupported coordination results deny the prefix; both membership directions and every retained origin must be owned |
| [Catalog boundary](2026-11-13-research-initialization-catalog-design.md#concrete-blockers-to-final-catalogversion-selection) | Coordination/recovery/finish/cancel must close before a full fixed catalog/version can be selected |
| [Dormant coordinator](../../../crates/orkworksd/src/taskmaster/coordinator.rs) | Data-only foundation explicitly lacks runtime authority; it is not a producer for these proposed records |
| [Hierarchy connection](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs) | Production open refuses unqualified profile before touching its path; no research publisher is inferred |

A bounded exact-name search for the proposed research families, common selected
coordination/recovery type names and needs_parent_result/completion_evidence in
taskmaster, HTTP and Electron main found no matching implementation at this base.
This is not an exhaustive inventory of differently named or external writers.
No runtime/database/native process was executed for this source inspection.

Use the already proposed research source/control/authority/publication and
SessionApplication bridge owners in the [prefix ownership table](2026-11-12-research-input-control-prefix-design.md#inspected-owners-and-proposed-responsibilities).
The actual report/ordinary lifecycle/UI owners supply trusted event facts;
research control owns current versions and semantic changes; publication owns the
complete atomic effect/proof/resource union. These are documentary boundaries,
not new modules or factories. Electron main owns exact shown UI contexts; neither
the renderer nor a model can supply trusted turn or native recovery observations.

## Shared types and explicit author choices

Use the existing exhaustive TaskKey, PlanRef, ReservationRef, AllocationRef,
RuntimeRef, CommitRef, full frozen ChildAssignmentIdentity P, canonical UTF-8 JSON,
unknown/duplicate-field rejection and ID/Digest types. run/plan/task versions are
JSON integers 1..2^31-1, never Counter TEXT; original frozen taskVersion remains
separate. Storage counters and bridge producerRevision retain their accepted
positive u64 TEXT encodings. Every listed nullable field is required.

The new choices are separate research source domains and three typed logical
families. Alternatives rejected here are report-as-turn/result; storing only
needs_parent_result with no retained turn provenance; reusing child domains or
recovery snapshots; and deriving source owner from whichever table exists.
No source digest is a bearer or evidence that a source owner actually ran.

## Retained original turn-completion evidence proposed

ResearchTurnCompletionV1 is exactly:

```text
{schemaVersion:1,task:TaskKey,reservation:ReservationRef,childSessionId,
 expectedTaskVersion,acceptedTaskVersion,evidence:ResearchTurnEvidenceV1}
```

acceptedTaskVersion equals expectedTaskVersion+1 without overflow. The original
source/control owner derives every identity and version. The retained evidence
is exactly one of these closed shapes:

```text
{kind:"codex_stop_receipt",runtime:RuntimeRef}
{kind:"ui_action",runtime:RuntimeRef,reviewContextId:Digest,
 sidecarGeneration:Digest}
{kind:"child_terminal_without_receipt",session:ResearchSessionStateRead}
```

ResearchSessionStateRead is the complete decoded research_session_states snapshot
specified under recovery below. For the terminal variant it is terminal and
retains its actual last runtime/producerRevision and original assignment/attachment;
it is not a model status or a receipt fabricated from process exit. For a running
child, the actual established runtime must already exist. A failed spawn cannot
become this terminal running-task case.

For Stop/UI variants, runtime is the exact current successful child runtime,
kind child with the original child session/workspace/run, resolving research
runtime history and the coherent nonterminal bridge. UI sidecarGeneration equals
that current runtime's sidecar generation, and reviewContextId binds the exact
shown task/reservation/version/runtime. A current original child report token and
verified Codex Stop integration authorize the Stop source; a ResearchReportCapability
or report descriptor cannot substitute. Native harness capability evidence remains
required; these fields do not qualify another harness's stop signal.

The body digest is SHA-256 of `orkworks.research-source-turn-completion.v1`, one
LF, and the complete canonical body. There is one immutable turn row per TaskKey,
with no new public operation, transport shape, token or model-selected evidence.
The accepted baseline turn endpoint/UI action remains its separate authority seam.

For the Stop variant, this turn row **is the original durable sidecar one-shot
receipt**, not a reference to a second unspecified upstream receipt. Its identity
is the exact TaskKey plus retained original reservation/child/runtime and accepted
version pair. Retain its original C and the complete original source-owner commit
proof for this exact key/value and the paired readiness transition. The source
owner must be the verified authenticated Stop handler, never a generic writer
that accepts an evidence enum. Late Stop delivery after UI readiness resolves
that existing TaskKey gate and preserves the UI row; it creates no second receipt
or unowned native-event record. A digest/manifest alone cannot prove that the
handler received a native event. Offline components supply that original owner/
authentication/proof fact as unqualified input; they cannot distinguish a real
Stop event from a forged owner assertion. Actual handler/integration participation
and provenance qualification remain a prerequisite to product publication. No
raw token, native transcript, additional upstream receipt codec or event log is
invented to claim otherwise.

For a new gate, require exact running task with null coordinationResult, original
reservation/unique attachment, current bridge/event authority and no existing
turn gate. In one complete outcome insert the gate, change the task to
needs_parent_result, advance live taskVersion and runVersion once, and apply all
actual source/index/resource/visible-summary effects. Retain frozen P and launch
admission unchanged. No result, report, head, grant or capacity release follows.

A late eligible Stop after UI readiness, or repeated delivery of an already
recorded readiness event, returns the existing turn disposition without replacing
its provenance or advancing versions. Current applicable source authentication
precedes that lookup; it is not permission to replay revoked child authority.
Different observations for another reservation/runtime do not repair a mismatch.
Actual terminal reconciliation remains an owner event under its own authority,
not a child request that needs a live child credential. A later ordinary resume
cannot erase this gate or produce a second original turn/result for the task.

## Parent coordination request and original receipt proposed

ResearchCoordinationRequestV1 has exactly the common fields:

```text
{schemaVersion:1,task:TaskKey,reservation:ReservationRef,childSessionId,
 expectedTaskVersion,outcome,summary}
```

outcome is completed, blocked or failed. summary is nonempty well-formed UTF-8
through 2 KiB. Use `orkworks.research-source-coordination-request.v1` + one LF +
complete canonical request, distinct from the existing child domain.
ResearchCoordinationReceiptV1 is exactly:

```text
{schemaVersion:1,task:TaskKey,reservation:ReservationRef,childSessionId,
 acceptedTaskVersion,commit:CommitRef}
```

Keep the common 4 KiB receipt ceiling. Authenticate the current exact parent run
bearer/scope and apply the existing parent mutation/request-rate discipline before
retained retry lookup. An identical accepted request returns the original receipt
before live-version CAS with no mutation/charge/event. Changed bytes for that
TaskKey conflict. Retained request/receipt values never restore authority.

A new outcome requires needs_parent_result, its exact original turn row/proof,
exact current taskVersion, original launched reservation/child/source and current
run eligibility. It may collect an existing result while stageContinueRequired
is set under the accepted read/wait/result-only parent authority. It cannot
approve a recovery revision or clear that gate. A terminal run or revoked parent
cannot submit or replay through this parent operation.

Commit the complete original request/digest/receipt, task status
reported_complete/reported_blocked/reported_failed and coordinationResult
`{requestDigest}` together. Increment taskVersion and sole runVersion once;
acceptedTaskVersion is that resulting version. A blocked/failed result also
publishes the exact existing blocker/pause/fencing effects: no subsequent launch
is admitted under an unfenced old grant. Completion unlocks only declared
dependencies after their other conditions; it does not establish result quality,
user acceptance, ordinary terminal state, capacity release or complete the plan.

The parent must check the accepted research report contract and all declared
review gates before reporting completed. A report object, empty report list,
process exit or parent summary cannot substitute for those checks. These checks
are actual admission obligations, not invented completeness booleans in the
request. The one-task source slice does not acquire reviewer/result/evaluation
codecs by accepting this request shape. Unsupported required review evidence
makes completed unavailable. Broader automatic plan completion still requires
its separate complete writer/effect contract; this proposal grants no new delay
or manual approval requirement in the already accepted logical lifecycle.

## Typed original-attempt research recovery proposed

ResearchRecoveryDecisionV1 has exactly the common recovery fields:

```text
{schemaVersion:1,recoveryId,task:TaskKey,reservation:ReservationRef,
 expectedRunVersion,expectedTaskVersion,reviewContextId,sidecarGeneration,
 kind,allocation:AllocationRef|null,attachmentId:ID|null,
 readSet:ResearchRecoveryReadSetV1,observedDirectories:ObservedDirectory[],
 blockers:ControlBlockerV1[],reason}
```

ResearchRecoveryReadSetV1 is exactly:

```text
{expectedCommit:CommitRef,run:ResearchRunControlRead,
 task:ResearchTaskControlRead,launch:ResearchLaunchControlRead,
 session:ResearchSessionStateRead|null}
```

The four tagged reads are closed snapshot types, not arbitrary row bags:

| Read | Fixed family / key | Full value / complete wrapper ceiling |
| --- | --- | --- |
| run | research_run_controls / RunKey | `{schemaVersion:1,body:ResearchRunControlV1,lastCommit}` / 1 MiB |
| task | research_task_controls / TaskKey | `{schemaVersion:1,body:ResearchTaskControlV1,lastCommit}` / 16 KiB |
| launch | research_launch_controls / TaskKey | `{schemaVersion:1,body:ResearchLaunchControlV1,lastCommit}` / 32 KiB |
| session | research_session_states / TaskKey | `{schemaVersion:1,body:ResearchSessionStateV1,lastCommit}` / 16 KiB |

Each read has exactly `{table,key,value}`. PlanControlRead is not added to this
readSet; the actual complete transaction still reads and fences its original plan,
sources, authority, resources, membership/indexes and ordinary/native peers.
Authorization-intent snapshots remain their separate accepted type; this is a
new explicitly research-scoped recovery snapshot union, not a child-union cast.

Every snapshot key reconstructs the decision's exact workspace/run/task and
original reservation. Captured run.body.runVersion equals expectedRunVersion;
captured task.body.taskVersion equals expectedTaskVersion. The task/launch/session
reservation refs equal decision.reservation and resolve its frozen P independently
of those live versions. Captured launch retains the original admission approval/
parent and expected/frozen version pair. A nonnull session binds the same original
attachment, child session and assignment identity, retaining its actual observed
producerRevision/runtime. decision.blockers equals exactly the selected original
task-related blocker bodies in captured run.body.blockers, in their required
unique sorted order, rather than a subset chosen to clear a convenient blocker.
Missing, extra or changed blockers deny. All source and original-parent/workspace
joins remain required even when the session snapshot is null.

Retain the common four kinds and all their original attempt semantics:
settle-no-child, confirm-original-allocation, confirm-original-attachment,
observe-terminal. Keep reason through 2 KiB, at most two sorted unique root/
private-git observations, at most 16 exact sorted original task blockers, and all
native descriptor/ownership rules. No-child requires attachmentId/session null
and complete absence/native-quiescence proof; allocation confirmation requires
allocation nonnull and no attachment/session; attachment confirmation requires
allocation/attachment/session nonnull and an already proven complete original
preparation; terminal observation requires original allocation/attachment and
coherent terminal bridge. Null is never an absence proof. No recovery kind
adopts a directory, fills zero ledgers, restores a grant, spawns, supplies a second
attempt, clears stage continuation or turns timeout into proven rollback.

Use `orkworks.research-source-recovery-decision.v1` + one LF + complete canonical
body. ResearchRecoveryReceiptV1 is exactly the common
`{schemaVersion:1,recoveryId,decisionDigest,runVersion,taskVersion,commit}` through
4 KiB. Current Main-authorized exact shown UI context and generation precede
exact retained retry lookup. A changed body for the same recovery identity
conflicts; an authorized identical retry returns the original receipt without
another version, proof, event or charge. Source/native facts are privately
captured and revalidated; renderer/model fields do not authorize recovery.

For a new decision, expected versions and exact captured run/task/launch/session
rows must still match the coherent admitted current state. Commit the complete
kind-specific original disposition, receipt, blocker/capacity/source/index/resource
and visible-summary effects together. Advance the sole runVersion once for a
new decision, coalescing every relevant live task change once.
The receipt records actual poststate versions; an unchanged task's version is
not advanced merely to fill the receipt. Unknown disposition remains held under
the original ownership/leases until proven, never normalized into a success enum.
Retain recovery-approval-required for failed/interrupted work; retry work needs a
new declared task in an independently approved revision, not a reused reservation.

Historical readSet keys/bodies/lastCommit and expectedCommit retain direct original
proof links under the eventual recognized full research profile. Original epochs
agree and snapshot origins are at/before captured H; the new manifest predecessor
is exactly H and each replaced before-image matches its snapshot. Do not reuse
format-4/6 row domains from the child recovery example. Frozen old versions,
runtime generations and producer revisions are validated historically, never
recaptured from today's controls. Direct proofs do not prove original freshness
or actual native authority; those remain source-owned transactional obligations.

## Logical families, projection and catalog consequences

Propose these three additional mandatory logical families in the eventual single
fresh research catalog; no physical table/layout or format is selected:

| Family / exact key | Complete value | Wrapper ceiling |
| --- | --- | ---: |
| research_turn_completions / TaskKey | `{schemaVersion:1,body:ResearchTurnCompletionV1,digest,lastCommit}` | 64 KiB |
| research_coordination_receipts / TaskKey | `{schemaVersion:1,request:ResearchCoordinationRequestV1,requestDigest,receipt:ResearchCoordinationReceiptV1,lastCommit}` | 16 KiB |
| research_recovery_decisions / `{task:TaskKey,recoveryId}` | `{schemaVersion:1,body:ResearchRecoveryDecisionV1,digest,receipt:ResearchRecoveryReceiptV1,lastCommit}` | 2 MiB |

All are immutable original records. One turn and one coordination outcome per
TaskKey; at most 32 retained recovery decisions across the entire run, using the
common bound rather than 32 per family/task. IDs are never recycled. Smaller
source, command/worker, 2 MiB row, paired job, discovery and physical/store limits
remain independently binding. A maximal legal snapshot may not fit its containing
row/command; deny rather than truncate or claim simultaneous maxima fit.

Committed projections exclude only outer lastCommit and, on the two
receipt-bearing families, receipt.commit. Every original expectedCommit and
embedded snapshot lastCommit stays committed. A terminal turn-evidence snapshot's
old origin is committed too. The new C is not inside any hashed request/body;
no fixed point or extra exclusion is introduced. Validate every excluded link
against its own complete original proof. Row/change domains wait for the final
format; source/request domains above select no storage version.

Charge exact keys plus committed projections, once per retained occurrence, to
the existing shared child-control-retained-bytes run/workspace pools (8 MiB/
64 MiB), reserved zero, and the 32 MiB run aggregate. Copied snapshot/receipt/
source references remain separately charged; no transfer from research source or
report pools, deduplication, new allowance or lifetime read-work refund. Current
control and summary changes contribute their actual original-to-final deltas,
coalesced once per ledger. Existing zero-ledger initialization boundaries remain;
these commands cannot initialize missing resources or erase original history.

The previous candidate has 51 logical names after accepted research authorization.
These three choices would make the additive candidate **54**, conditionally.
It remains neither a final catalog nor a maximum: finish/cancel, synthesis,
execution/recovery-plan source and broader consumers may need further closed
variants/families. Derived physical lookup structures are not extra logical rows
or implicit proof manifest entries. Existing child/ordinary readers remain scoped;
no source/table presence selects a profile or repairs an unsupported record.

## Complete reference and admission obligations

| New owner | Required source-derived retained relations in both directions |
| --- | --- |
| Turn completion | Original task/plan/configuration, reservation/child, unique attachment/frozen P, current successful runtime source for Stop/UI or exact terminal bridge snapshot/origin, and original/new direct proofs; UI context is historical data, not authority |
| Coordination receipt | Original task/reservation/child/source group, unique original turn gate, exact request/result digest and original receipt/new C; retain turn evidence even after ordinary resume/end or later plan completion |
| Recovery decision | Original task/reservation/allocation/attachment/source/runtime group where present; all four typed snapshots, captured H, exact native observations, ordinary lineage P/C when attached, original source/proof/receipt/new C and exact blocker owners |

Extend the accepted logical metadata-origin target recipe to these research
snapshots only where they occur in the new turn/recovery record types. Add no
arbitrary field path, user graph, ObjectRef kind, dependency purpose or research
source cast. The task's coordinationResult requestDigest must resolve exactly one
matching research_coordination_receipts row and its original turn/source/proof
group; a matching digest under child_coordination_receipts cannot repair it.

Before any publication/current read/purge, complete source-owner inventories and
exact forward/reverse correspondence must include these immutable consumers,
including zero-reference owners and terminal/tombstoned ordinary history. The
absence of a head, current turn row or coordination result is not proof all
historical consumers/attempts are absent. Both exact-key and family-membership
reads need stable source/index generations through final validation. No caller
coverage flag or count/hash-only join qualifies them.

The actual read union includes current root/run/bootstrap, original plan/input/
configuration/scope/approval/reservation/allocation/attachment/runtime, required
ordinary ownership/bridge, complete affected collection/source/custody/summary/
tracked-run consumers, ledgers, lease/health and proofs. New writes include the
one owning evidence/receipt/decision plus every actual current control, blocker,
capacity, summary/consumer/index/resource/header/new-proof effect. These lists
assign responsibility, not a complete executable command or qualified row total.
Physical owners must close exact locators and update/reconciliation sets before
full-state/history fixtures. Discovery/final validation share the accepted 4,096
observations/16 MiB cumulative budget and 100,000-visit ceiling; overflow denies.

All transitions preserve ordinary input/stop independence: no input-critical
lock is held through hierarchy SQL/fsync. Unavailable bridge or source outcome
fences hierarchy automation/capacity conservatively. Neither a source digest nor
a task status can qualify the required nonblocking cutoff or paired atomicity.
Recovery returns complete old/new outcome or unavailable reconciliation; no
partial replay, maxima election, new timestamp, restored permit, source-family
fallback or reconstruction of missing proofs/receipts is introduced.

## Acceptance choices and bounded next packet

Three new choices require scoped owner acceptance before dependent components:

1. Retain the closed original research turn gate with its three source-owned
   evidence variants in research_turn_completions; a report cannot supply it.
2. Select the research coordination request/domain/receipt family and exact
   original turn/result linkage under unchanged parent authority/retry/version
   rules; task completion releases no live-child capacity.
3. Select the research recovery body/domain and research-only typed snapshot
   union, its family/projections and shared control accounting/retention/index
   obligations, leaving the full catalog/version and actual native recovery open.

After acceptance, the next bounded packet may contain independent Python/JSON
turn/result/recovery source bytes, original/frozen/live snapshot joins, source
body digests and data-only JavaScript denials. Include Stop/UI/terminal evidence,
wrong research/child domains/families, missing gate, exact retry versus conflict,
original historical versus current values, duplicate identities, wrong recovery
kind/nullability, copied origins, count exhaustion and independent wrapper/job
limits. Label auth/native/coverage/currentness/C/command/proof/outcome inputs as
supplied unqualified facts. Preserve every earlier artifact byte; build separate
prestates rather than patch running/report views into another lifecycle stage.

No storage header, manifest, row/change hash, recognized storage profile, legal
history, actual permit/recovery/turn event, runtime API/route/factory, Rust/Electron
module or executable product plan may be supplied by that component handoff.
An independent written finish/stage-cancel/run-cancel/synthesis/continuation
source contract and complete physical catalog/index/proof/initializer closure
remain necessary before complete research-state/history fixtures. Accepted
same-parent behavior and separate exact execution approval remain unchanged.

## Review and verification handoff

The owning session writes this one proposal and routing only. One bounded review
round uses two fresh read-only Luna contexts: contract correctness and a builder
check for material unanswered offline-component questions. Agent cap two; no
reviewer edits or executes runtime/native/model probes. The owner applies any
required corrections and verifies metadata/index, docs build/render/links,
prior-byte preservation and diff/doc/worktree close-out before PR publication.
Publication/merge of this proposal does not accept the three choices above.

## Written review and local verification disposition

One bounded round used two fresh read-only Luna reviewers. The builder found no
material unanswered offline-component question. Correctness review requested an
explicit retained Stop-receipt identity/proof boundary and the common recovery
cross-field equality rules. The proposal now states that the original turn row
is the durable sidecar one-shot receipt, preserves its exact TaskKey/original
runtime/version/proof joins, and cannot independently prove actual native event
authenticity. It also states exact captured expected-version, reservation/session
and complete selected-blocker equality. Actual source ownership remains unqualified.
No reviewer ran a fixture, database, native process or model probe.

Pinned Node 22 docs validation passes all 1,704 existing tests, the VitePress
build, OKF metadata/index and rendered links. All 20 OKF regressions pass. All
557 earlier non-Markdown validation artifacts remain byte-identical; both prior
technical specifications are append-only. No new fixture/code is included.
Existing bundler circular/large-chunk notices concern unchanged configuration.
Current-head hosted review and CI are recorded separately in the PR. These
checks accept none of the new choices or open runtime/profile/proof gates.

## Scoped owner acceptance — 2026-11-15

The owner answered “Go ahead” to the explicit three-choice acceptance question
for **offline fixtures only**. This accepts the retained research turn evidence,
separate research coordination request/receipt family and typed original-attempt
research recovery representation/accounting obligations at that scope. Earlier
pending wording records the historical proposal state. The accepted logical
same-parent lifecycle and every product capability/activation gate stay unchanged.

The [bounded component plan](../plans/2026-11-15-research-continuation-components.md)
keeps source/auth/native/coverage/currentness/C/proof/outcome facts unqualified.
It authorizes no actual turn/recovery event, physical catalog/format/header/
manifest/row hash, runtime adapter or executable product plan. Gate 2 remains
Investigate; #1689/#1089 remain open.

## Research completion and UI closure proposal — 2026-11-15

The [completion/synthesis/closure proposal](2026-11-15-research-completion-design.md)
makes the next positive research lifecycle and source/effect choices concrete:
automatic plan completion, immutable bounded summaries/current selector and
version-bound Main finish/stage-cancel/continue/run-cancel decisions. Three new
families would make the candidate 57 and original-admission selector adds one
logical root row conditionally. These choices await scoped owner acceptance;
prior acceptance does not extend to them. No terminal-profile cast, storage
format, complete publication/history, dependent fixture or runtime work follows.


## Positive final-condition recovery proposal — 2026-11-15

The [new source contract](2026-11-15-research-recovery-completion-design.md)
proposes a late disposition interruption preserving original readiness/result,
then proven observe-terminal settlement and automatic plan completion in one
outcome. Its three new choices await scoped owner acceptance before dependent
fixtures. It preserves the old incomplete-task negative example, existing codecs,
candidate 57 and runtime/physical qualification gates; no new family or actual
source proof is supplied. Gate 2 remains Investigate.
