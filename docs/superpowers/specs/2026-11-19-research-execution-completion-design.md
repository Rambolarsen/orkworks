---
type: Design Proposal
title: Execution final completion and terminal continuation
description: Proposed typed execution-plan completion trigger, absorbing run terminal transition, authority revocation and collection/wait disposition under the sole research run owner.
tags: [orkworks, research, execution, completion, terminal, qualification]
status: draft
workflow_status: proposed-for-scoped-terminal-completion-review
---

# Execution final completion and terminal continuation

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738); consumers
[#610](https://github.com/Rambolarsen/orkworks/issues/610),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `e18b85454cbe` (origin/main, including the
research-to-execution proposal merged in
[#1914](https://github.com/Rambolarsen/orkworks/pull/1914)).

**Proposed for scoped owner review.** The
[source-coverage audit](../../validation/research-source-coverage-audit.md)
names execution final completion and terminal continuation as the last
unclosed variant row before catalog/initializer closure. This proposal closes
that written boundary only: the typed execution-plan completion trigger, the
absorbing run-terminal transition with complete authority revocation,
collection/expectation retirement, wait disposition and honest retained
capacity, and the terminal source/proof/consumer mapping. It makes new
placement/effect choices explicit; they are not acceptance inferred from a
merge. Execution-child launch/reservation/allocation/attachment under
execution joins, multi-task execution plans, failed/blocked execution
recovery revisions, the direct-task skip path, terminal UI correction
profiles and final #744 evaluation of execution results remain separate
#1689/#610 work. No dependent fixture, Rust/Electron module, storage format,
migration, probe or runtime execution is supplied. Gate 2 remains
**Investigate**.

## Outcome and preserved decisions

Make the accepted final transition representable: when the execution plan's
declared task reaches its exact accepted coordination outcome under the
unchanged turn gate, the plan completes automatically through the already
accepted typed trigger, and that same compound outcome carries the run to its
absorbing `complete` state — revoking the run bearer, every grant and parent
planning authority, waking outstanding waits with `run_closed`, retiring the
proper active collection memberships, and retaining summaries, children,
artifacts and all historical proof — with no child termination and no
capacity release for still-live children.

Preserve unchanged: the accepted
[source grammar](2026-11-12-research-source-codecs-design.md) and per-task
chain records; the [input/control prefix](2026-11-12-research-input-control-prefix-design.md)
including the sole run owner; the
[turn/coordination/recovery codecs](2026-11-15-research-continuation-design.md),
whose shapes are TaskKey-generic (their execution-task reuse is an explicit
owner choice below, not inherited admission behavior);
the [completion/lifecycle records](2026-11-15-research-completion-design.md)
including the typed trigger union, lifecycle decision/readSet/receipt
machinery, selector and summary invalidation; the accepted
[scope-rejection](2026-11-16-research-scope-rejection-design.md),
[prospective-membership](2026-11-16-research-prospective-membership-design.md),
[post-rejection](2026-11-16-research-post-rejection-report-design.md),
[stage-report](2026-11-16-research-stage-report-design.md),
[review-task](2026-11-18-research-review-tasks-design.md),
[clarification](2026-11-18-research-clarification-design.md) — merged
proposal contracts whose choices await scoped acceptance — and
[execution-proposal](2026-11-19-research-execution-proposal-design.md)
contracts; #742 report/request/head/receipt rules; #744 evaluation/artifact/
debit domains; and the transport/grant rules (final completion ends the run
and revokes the run bearer and all launch grants; already-launched children
retain ordinary session lifecycle and manual-integration records).

Rejected alternatives: a TerminalReportRunV1 producer or conversion; a second
runVersion owner or a new terminal counter; killing, adopting or transferring
children at completion; releasing capacity for nonterminal children; a
manual completion approval step in the already-accepted automatic lifecycle;
and inferring success from a report, exit or elapsed time.

## Source trace and obligations

| Source | Established constraint |
| --- | --- |
| [Transport and grants](2026-10-04-orchestrator-preparation-design.md#transport-and-grants) | Final execution completion ends the run and revokes the run bearer and all launch grants; requests racing completion cannot reserve a child after the fencing transition; already-launched children retain ordinary lifecycle |
| [State table](2026-10-04-orchestrator-preparation-design.md#state-and-transition-table) | Final execution plan complete → `complete`: revoke run bearer/grants, retain visible results; manual integration remains separate; `complete`/`cancelled` are absorbing — parent exit, restart or UI resume never returns a terminal run to `paused` or restores authority |
| [Baseline completion predicate](2026-09-26-taskmaster-orchestrated-child-sessions-design.md#approved-plan) | Every declared task reported_complete, all reservations attached and settled, no interrupted allocation or pending proposed revision; the check reruns atomically when any transition resolves the last blocker |
| [Typed triggers](2026-11-15-research-completion-design.md#original-completion-and-ui-request-codecs-proposed) | The closed coordination/recovery trigger union pins the exact accepted outcome; commit trigger, completion and coalesced effects in the same C; trigger retry resolves the original source outcome |
| [Turn/coordination codecs](2026-11-15-research-continuation-design.md) | TaskKey-generic families already cover any declared task's turn gate and coordination receipt; the parent checks the accepted report/result contract before reporting completed; completion unlocks declared dependencies only, never capacity |
| [Terminal compatibility](2026-11-15-research-completion-design.md#fencing-rollback-and-terminal-compatibility) | No TerminalReportRunV1 producer or conversion; keep research state complete/cancelled in `research_run_controls`, preserving raw gates/blockers/clarification values; broader terminal UI corrections stay unavailable until their complete mapping is qualified |
| [Finish retirement precedent](2026-11-15-research-completion-design.md#source-owned-transition-and-effect-union) | Finish/run cancellation explicitly retire every proper active collection and collecting expectation, preserving selected reports/versions/history; no lifetime debit rewrite or refund |
| [Execution proposal boundary](2026-11-19-research-execution-proposal-design.md) | One declared execution task; the grant exists unexercised until the separately gated launch chain; `awaiting_execution_approval` retains `resumeTarget=awaiting_execution_approval` |
| [#610 scope](https://github.com/Rambolarsen/orkworks/issues/610) | Authenticated turn readiness or explicit UI action gates exact version-bound parent outcomes; reports/exit alone never prove success, quality or user acceptance; no orchestration-owned transfer, commit, merge or automated cleanup |

These are bounded inspection facts at the inspection base. Use the proposed
research source/control/authority/publication, SessionApplication bridge and
Main approval seams from the
[prefix owner table](2026-11-12-research-input-control-prefix-design.md#inspected-owners-and-proposed-responsibilities).
No new module is created; type names are not factories or an existing runtime.

## Selected terminal slice

The run whose declared execution plan completes through its accepted typed
trigger reaches the absorbing terminal state in the trigger's compound
outcome. Failed/blocked execution outcomes do not complete the run: they
publish the accepted blocker/pause/fencing effects and pause launches until a
recovery revision — whose execution source stays unclosed here — is approved.
Run cancellation remains the accepted `cancel-run` decision. The direct-task
skip path and multi-task execution plans remain unavailable.

**Execution-task codec reuse is an explicit owner choice, not inherited
behavior.** The accepted turn/coordination/recovery codec *shapes* are
TaskKey-generic, but the accepted coordination *admission obligation* is
research-bound: it requires the parent to check "the accepted research report
contract and all declared review gates" before reporting completed — a check
whose accepted form does not exist for an execution task, and which must not
be silently generalized. This proposal therefore selects, as a new owner
choice: the turn gate, coordination request/receipt and recovery decision
**shapes** are reused byte-unchanged under the execution task's key, while
the coordination admission obligation for an execution task is the task's
accepted **ordinary-child result/turn boundary** — the baseline result
contract (`orkworks.result.submit.v1` reporting and the accepted turn
gate/authenticated turn-readiness or explicit UI action) — with #744
evaluation of that result and the execution role/profile qualification
remaining separately gated. No research report contract or review gate is
checked for an execution task; no research report/review codec is imported
into execution. The actual launch chain that produced that child remains
separately gated and its admission evidence stays unqualified. Nothing in
this proposal launches a child or qualifies an adapter. For the packet's
turn-gate evidence: reuse
the [continuation packet's](../../validation/research-continuation-components.md)
three-variant labeling unchanged — the execution child's runtime refs are
synthetic supplied data under the same explicit supplied-handler/fixture-tool
assumption that packet records, and the unqualified launch chain means no
real runtime origin is claimed for any variant.

**Packet-prestate authority boundary.** No prior offline packet constructs
the execution-proposal prestate (plan source with `researchBinding`, plan/
task controls, approval/grant context) — the
[execution-proposal contract](2026-11-19-research-execution-proposal-design.md)
is merged but its five choices are still pending scoped acceptance and its
component packet does not exist. The packet authorized by this proposal may
therefore construct that prestate only as **synthetic supplied context**,
labeled against the execution-proposal contract's pending choices, and
validates only this contract's choices (trigger reuse and the terminal
transition). Constructing those bytes creates no acceptance of the
execution-proposal choices and no legal history. The prestate's run-state
value is the retained handoff value `awaiting_execution_approval` with an
explicit supplied label — the launch chain's own positive post-launch state
writer is separately gated, so no other value is authorized, and the label
records that the prestate's post-launch state is supplied, not qualified.

## Typed execution-plan completion trigger

`ResearchCompletionTriggerV1` is reused byte-unchanged for the execution
plan: the coordination form pins the execution task's exact accepted
`ResearchCoordinationRequestV1` and its receipt's resulting
`acceptedTaskVersion`; the recovery form pins the exact accepted
`ResearchRecoveryDecisionV1` body/digest under its
`{task,recoveryId}` key. No new trigger variant is added; a transition that
would complete the plan through an unsupported trigger denies as a whole,
exactly as the accepted rule requires. The lifecycle record family is reused
with the `plan-completion` key kind under the execution `PlanRef`: one
original completion record per exact execution plan, built first (no new C
inside any hashed body), committed with its trigger and the complete
coalesced effects in the same C.

The completion predicate over the trigger's candidate poststate is exactly
the accepted baseline, with **no added conditions**: the declared execution
task `reported_complete` under its exact original turn/result/source group,
its reservation attached and settled, no interrupted allocation, and no
pending proposed scope revision. A retained unanswered clarification request
does **not** block completion — the accepted lifecycle completes plans while
a clarification gate persists and keeps it in the poststate as a preserved
raw clarification value; a material unanswered question blocks dependent
proposal publication/approval, never plan completion. The completing
prestate additionally requires `stageContinueRequired` false (a
stage-cancelled plan is cancelled, never completed; a paused parent has no
planning authority to report the outcome) — stated as this proposal's own
precondition, not as accepted baseline content. The grant plays no
precondition role at all: an active grant for this plan is revoked by this
completion (effect, not condition), and the accepted one-grant-at-a-time
rule already forecloses any other active grant. The completion's admission
is
part of its authenticated source trigger (not a new parent request/rate
allowance); an exact original coordination/recovery retry returns its
original outcome and never appends a second completion; rechecking an
already completed execution plan verifies its original record and is
read-only.

The accepted trigger union now has three variants — coordination, recovery
and the accepted scope-rejection form. For the execution plan in this slice,
the coordination and recovery forms apply; the scope-rejection form resolves
a pending proposed scope revision of a proposed/current plan, and this slice
closes no scope-revision variant for execution plans, so it cannot arise
here — the predicate's "no pending proposed scope revision" term covers the
general case, and a scope-rejection-driven completion of an execution plan
is unavailable until that variant is separately closed.

## Absorbing run-terminal transition

Plan completion and run terminal transition share the trigger's one compound
outcome. Beyond the accepted plan-completion effects (insert one completion
record, mark that plan complete/version+1, revoke its grant, select
`completedResearchPlan`, advance `runVersion` once with the trigger), the
same outcome carries:

| Owner | Required change / preserved facts |
| --- | --- |
| Run state | Set `complete` — the absorbing terminal value — in the same coalesced advance; never `paused` or `synthesizing`, and never later changed by a child ending/resuming, a report arrival or a resumed parent. On the terminal poststate the pause-only fields `pauseReason`/`resumeTarget` are null (nothing is paused), while every raw blocker, gate and clarification value — including the run control's raw `stageContinueRequired` and any historical selector `pendingStageContinue` pointer — is preserved exactly; terminal state makes a retained gate inoperative, never permission to continue |
| Authority | Revoke the run bearer and every launch grant (research grants are already revoked by their own completions; the execution plan's grant — active or unexercised — is revoked here), revoke parent planning authority and every child-report capability whose owner rules bind them to run liveness, under the existing nonblocking close-only rules. Grants are never serialized: the revocation is represented offline as **label assertions only** per the execution-proposal contract's rule (proposed label keys `grantExisted:"supplied"`, `grantRevokedAtCompletion:"supplied"`); the completion record's "revoked grant's historical binding facts" are exactly the already retained immutable approval facts — approval ID, approval digest, plan digest and the receipt's commit ref from the execution-proposal approval source/receipt — as historical data, never live authority |
| Waits | Wake outstanding parent event waits with `run_closed`; the ended parent needs no further authorized read; no reconnect is automatic. `run_closed` is an effect label under the accepted wait rules, not a record the packet stores |
| Collections | Retire every proper active collection and collecting expectation in the owned complete set (accepted finish pattern): `active`/`collecting` false with version advances coalesced once, preserving selected reports, versions and all historical relations; no ledger fabrication or counter reset. In this slice the owned complete set is exactly the research task's collection and expectation rows — the execution proposal created none, and the execution task's own expectation belongs to the separately gated launch chain, so its absence in the prestate is asserted explicitly, never inferred from a missing row |
| Pointers | Clear `currentPlan`/`pendingPlan` and the parent runtime pointer; clear any obsolete `currentSummary` under its existing owner when the completion's selection supersedes it, retaining summary/version/history; retain `planReferences`, `completedResearchPlan`, every remaining summary/history and every raw blocker/gate/clarification value |
| Children | Terminate nothing: live children keep consuming run capacity until their ordinary lifecycle actually ends; each subsequent ordinary end/resume remains independently observable and updates derived capacity without reopening the terminal run. Capacity is derived from the ordinary session/launch controls, not a retained resource row: the packet represents the live-child case as the execution task's session/launch control rows (supplied synthetic data, terminal=false with their actual producerRevision) plus the accepted derived-capacity policy labels; the "capacity release for nonterminal children" denial asserts the policy label against those rows |
| Consumers | Invalidate every recognized pending proposal (none may survive terminal state) and apply the accepted source-owned summary invalidation for any binding change; retain immutable history |
| Proofs | Commit all logical changes, required direct proof links, derived lookup maintenance, one header advance and one new complete proof outcome together; no post-commit registration or partial success |

Recovery/rollback follow the accepted fencing rules: proven rollback keeps
the complete old state, proven commit keeps the complete new state, and
unknown disposition holds original ownership conservatively with hierarchy
capacity fenced — exposed as unavailable/reconciling rather than a successful
terminal receipt. Run cancellation must not wait for native disposition or
PTYs; a retained unknown attempt keeps its recorded capacity under the
original ownership. Ordinary input/stop/end never waits on hierarchy SQL/fsync
or holds an input-critical lock through it.

## Terminal source, projection and consumer mapping

Terminal state lives only in `research_run_controls` (state `complete`) with
`run_states` retained as the historical seed: **no TerminalReportRunV1
producer, conversion, projection cast or second counter is introduced**, and
the older completed/cancelled `run_states` variant is not rewritten, emptied
or satisfied by erasing fields. A read-only display label is not persisted
source/profile qualification. Raw stage gates, blockers and clarification
values are preserved exactly as the accepted terminal-compatibility rule
requires; the broader terminal UI correction profile mapping (whose separate
Main correction exception grants no initial child write or finish/cancel
authority) remains unavailable for this path until its complete
source/consumer/profile mapping is separately qualified.

Required membership in both directions, following the accepted recipes with
no new dependency purpose:

| Owner / key | Complete required typed references |
| --- | --- |
| Execution plan-completion record / `{plan:PlanRef}` | The exact typed trigger and its accepted coordination/recovery source group, the executed task's turn/result/reservation/attachment chain and frozen identity, the execution plan source with its `researchBinding`, the plan/task controls with their final versions, the revoked grant's historical binding facts (never live authority), original readSet/H, original/new direct proofs and the receipt/new C |
| Terminal run control / RunKey | The original run/bootstrap, every ordered plan reference with its final status/version, the absorbing state's owning completion record, the selector's final `completedResearchPlan`/`currentSummary`, retired collection/expectation rows with their selected reports, every preserved blocker/gate/clarification value, and original/new direct proofs |

Zero-reference owners, historical runs, retired expectations and tombstoned
ordinary linkage stay inside every complete inventory; absence of a head,
pointer or receipt never proves absence of consumers. The accepted discovery
bounds (4,096 observations / 16 MiB cumulative, 100,000-visit ceiling and
smaller worker limits) apply unchanged. No new logical family or object kind
is proposed: this transition is expressed entirely through the already named
families (lifecycle records, run/plan/task controls, collection/expectation,
selector), so the conditional candidate catalog stays at **63 logical
names**; this remains neither a final catalog nor a maximum.

## Accounting and compatibility

Every changed control/collection/expectation/selector original-to-final delta
coalesces once per ledger under its existing pool, with the exact pool named
per occurrence: the execution plan-completion lifecycle record, the terminal
run-control delta and the retired collection/expectation deltas charge to
the existing shared **child-control**-retained-bytes run/workspace pools
(8 MiB / 64 MiB) — the families those rows belong to; the execution plan
source and approval object were charged at their creation to the shared
**child-source**-retained-bytes pools under the execution-proposal contract
and are unchanged here. The execution task's chain occurrences were charged
when created, and completion charges only actual
changed-row deltas. No lifetime debit rewrite, capacity refund,
deduplication or new allowance follows. The accepted 32 MiB run aggregate,
8 MiB/64 MiB shared pools, command, physical, discovery and smaller limits
win independently; a maximal combination that exceeds a containing budget
denies.

No existing source digest, wire field, report identity, dependency purpose,
version default, cap, receipt version or volatile credential serialization
changes. Old stores are never imported, extended on first read or retokened;
the proposal can be reverted without store effects.

## Owner choices and bounded next packet

Accept or revise these proposed choices for **offline components only**:

1. Reusing the accepted typed trigger union and lifecycle record machinery
   for the execution plan's completion — one original record per exact
   execution `PlanRef`, committed with its trigger and coalesced effects —
   together with the **execution-task codec reuse choice**: the TaskKey-
   generic turn/coordination/recovery shapes reused byte-unchanged under the
   execution task's key, with the coordination admission obligation for an
   execution task redefined as the task's accepted ordinary-child result/turn
   boundary (no research report/review check, no research codec import), and
   #744 evaluation/profile qualification separately gated.
2. The absorbing run-terminal transition in the same compound outcome:
   `complete` in the sole run owner, complete authority revocation
   (bearer/grants/planning/child-report capabilities), `run_closed` waits,
   collection/expectation retirement per the accepted finish pattern,
   pointer clears with retained history, and no child termination or
   capacity release for live children.
3. The terminal source/projection mapping: terminal state only in
   `research_run_controls` (no TerminalReportRunV1 cast), preserved raw
   gates/blockers/clarification values, and the broader terminal UI
   correction profile mapping staying unavailable until separately
   qualified.
4. The two-row membership recipes above, expressed entirely through already
   named families (candidate 63 unchanged), with zero-reference owners and
   retired memberships inside every complete inventory.
5. Failed/blocked execution outcomes never complete the run: they publish
   the accepted blocker/pause/fencing effects, and execution recovery
   revisions stay separately unclosed.

After scoped acceptance, the bounded packet covers `docs/validation/` only:
an independent Python generator, retained JSON, data-only JavaScript
checker/tests and evidence prose, plus documentation routing and LF
attributes. Required examples: the execution task's turn gate and accepted
coordination outcome driving plan completion and the absorbing run-terminal
transition in one compound outcome; exact trigger retry returning the
original outcome; active-grant revocation at completion (label assertions
with the retained approval-fact bindings); retired
collection/expectation membership with retained selected reports; a live
child still consuming capacity after terminal state; `run_closed` wait
disposition; preserved raw gates/blockers/clarification values with no
TerminalReportRunV1 cast; the synthetic execution-proposal prestate labeled
against that contract's pending choices (including its supplied
`awaiting_execution_approval` post-launch state label); failed/blocked
execution retaining the run
nonterminal; and denial cases (unsupported trigger, second completion,
capacity release for nonterminal children, terminal state reversal, missing
consumer, stage-cancelled plan completion attempt, wrapper overflow). All auth, transport, coverage, currentness,
C/command/proof, grant, launch-chain, capability/profile-qualification,
supplied-handler/fixture-tool turn-evidence and outcome facts remain
explicitly supplied and unqualified. No legal event history, complete
transaction, storage format, runtime/native/model call, database, migration,
product module or executable product plan is authorized.

## Review and verification handoff

One writer: the owning session. One bounded round with two fresh read-only
reviewers: accepted-contract correctness (joins, trigger reuse, terminal
absorption and the #610/#742/#744 boundary) and builder completeness
(material unanswered offline-component questions). Reviewer cap two; no
reviewer edits, fixtures, runtime, native or model probes. Correct material
findings before handoff; verify docs metadata/index, build, rendered links,
prior-artifact byte preservation, append-only prior bodies and diff/doc/
worktree close-out. Publication/merge of this proposal does not accept the
five choices above.

## Local review disposition

One bounded round used two fresh read-only reviewers. The builder review
identified seven material gaps, all about offline-packet constructibility:
no precedent packet exists for the execution-proposal prestate (and
constructing it needs an authority statement); the prestate run-state value
for a live execution child has no authorized writer; grant revocation and
"historical binding facts" had no representation; the live-child capacity
example named no representable row/value set; the turn-gate variant and its
runtime-ref provenance were unpinned; the collection/expectation prestate
inventory was undefined; and the accounting pool naming was ambiguous
between the child-source and child-control pools. The correctness review
confirmed the trigger/record reuse scoping, the absorbing effect union, the
no-TerminalReportRunV1 mapping, retirement, capacity, fencing and the
candidate-63 arithmetic, and found two material issues — the completion
predicate silently adding a clarification condition (the accepted lifecycle
completes plans while a clarification gate persists) and the execution-task
codec reuse presented as inherited rather than an explicit choice whose
research-bound admission obligation is unsatisfiable for an execution task —
plus four ambiguities (the omitted third trigger variant, a self-cancelling
grant predicate term, the dropped obsolete-summary clear, and inaccurate
"accepted" phrasing for the merged proposal contracts).

This revision resolves all of them: the packet's execution-proposal prestate
is explicitly synthetic supplied context labeled against that contract's
pending choices, with the prestate state value `awaiting_execution_approval`
under an explicit supplied label; grant revocation is label-assertion-only
with the retained approval facts named as the historical binding facts;
capacity is derived from supplied session/launch control rows plus policy
labels; the turn-gate evidence reuses the continuation packet's three-variant
supplied-handler labeling; the prestate collection inventory is exactly the
research task's rows with the execution-task absence asserted; the exact
child-control vs child-source pool is named per occurrence; the predicate is
restated as exactly the accepted baseline (no clarification condition, no
grant condition) with the stage gate as this proposal's own precondition and
the three-variant trigger union's applicability stated; the execution-task
codec reuse is an explicit owner choice with the admission obligation
redefined as the ordinary-child result/turn boundary; the obsolete-summary
clear is restored; and the merged-proposal contracts are described as
pending scoped acceptance. No accepted technical definition changed and no
runtime qualification follows. Neither reviewer executed code, a fixture,
database or native/runtime process.

## Remaining gates

Execution-child launch/reservation/allocation/attachment under execution
joins, execution recovery revisions for failed/blocked outcomes, multi-task
execution plans, the direct-task skip path, execution role/profile
qualification under #741/#740's closed-research standard, the broader
terminal UI correction profile mapping, final #744 evaluation of execution
results, actual native/runtime participation, complete physical
catalog/version/initializer/proof closure and separately approved executable
plans remain open. #1689/#1089/#610 stay open; #740 stays closed research.
Gate 2 remains **Investigate**.
