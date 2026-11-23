---
type: Design Proposal
title: Research command effects and original proof closure
description: Proposed logical command-wide capture, effect, version, accounting and original-proof obligations for the retained-file research profile, preserving operation-specific authority and retries.
tags: [orkworks, research, commands, proofs, qualification]
status: draft
workflow_status: proposed-for-scoped-command-proof-review
---

# Research command effects and original proof closure

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738); consumer
[#610](https://github.com/Rambolarsen/orkworks/issues/610).
Inspection base: `3c3ccd91fef4` (origin/main), including initializer acceptance
[#2020](https://github.com/Rambolarsen/orkworks/pull/2020).

**Proposed for scoped owner review, for written command/proof closure only.**
This is [source-coverage audit item 4](../../validation/research-source-coverage-audit.md#catalog-indexes-initialization-and-proofs-still-required).
It joins the accepted [catalog enumeration](2026-11-23-research-catalog-closure-design.md#enumerated-logical-catalog),
[discovery/invalidation ownership](2026-11-23-research-discovery-invalidation-design.md#invalidation-ownership)
and [three initializer outcomes](2026-11-23-research-initializer-manifest-design.md#scoped-owner-acceptance--2026-11-23)
with each command's existing source-owned effects. It supplies a logical effect
and proof contract, not an executable command codec, physical manifest, working
transaction or production publisher. Gate 2 remains **Investigate**.

## Outcome and preserved boundaries

A successful new command must have one complete original-to-final effect union,
not merely a valid report, receipt, source digest or local control update. The
owning publication outcome includes its source and ordinary lineage, complete
consumers, resources, lookup maintenance and header/new-proof controls. Each
required original direct proof remains resolvable after publication.

Use the existing documentary source/control/authority/publication/ordinary-bridge
owners; this document creates no second writer, run counter or authority service.
Every accepted key, body, reference, digest domain, exclusion, receipt, resource,
version ceiling and permission rule remains under its owning contract. Research
records are never routed through child-source tags, a generic row-edit operation,
source-health or a #744 claim/debit merely to obtain a writable command.

The tables below name **logical operations**, not new serialized `commandKind`
strings, request tags or arbitrary table/key/value APIs. The research preparation
contract's existing `research-source-attach` choice remains its own named internal
kind. Other missing executable mappings remain unavailable until their exact
producer and representation are closed; this effect map cannot fill those gaps.
The later physical profile must recognize every admitted operation and its full
proof/index representation. No format/table/layout/header/manifest/version or
new commitment domain is selected here (audit item 5 remains separately gated).

Rejected alternatives: committing local effects then registering consumers;
caller-chosen affected lists; per-row version increments; link-only restamping;
receipt-based authority; rebuilding original outcomes from today's rows; and
splitting an oversized outcome into separately accepted pieces.

## Sources and investigated uncertainties

| Source | Constraint carried into this proposal |
| --- | --- |
| [Child command precedent](2026-10-31-child-review-source-preparation-design.md#common-read-commit-and-retry-contract) | Capture H and full source-owned reads; complete effect union; no generic writable bag, claim substitution or cross-owner transaction inferred from ordinary creation |
| [Research prefix](2026-11-12-research-input-control-prefix-design.md#proposed-normal-version-trace-and-local-effect-contract) | Original frozen reservation identity differs from live taskVersion; source/native phases are distinct; report arrival changes no task outcome or capacity |
| [Report publication](2026-11-12-research-membership-publication-design.md#first-report-read-set-and-original-to-final-effects) | Authority before receipt lookup; immutable input/selection and all consumer/resource/index/proof effects; captured drift denies |
| [Research authorization](2026-11-13-research-preparation-authorization-design.md#one-retained-research-authorization-and-original-command) | Original authorization, assignment, attachment and ordinary prepared bridge share their original preparation outcome; no missing-peer creation |
| [Completion](2026-11-15-research-completion-design.md#source-owned-transition-and-effect-union) | Final-condition triggers and completion share one C and sole version advances; plan completion preserves eligible collection and actual child capacity |
| [Recovery completion](2026-11-15-research-recovery-completion-design.md#exact-existing-representations-and-compound-outcome) | Both decisions capture the same original H/run reads, not intermediate repaired snapshots |
| [Commit/proof precedent](2026-10-24-hierarchy-storage-contract-design.md#commit-commitments-without-a-digest-cycle) | Exact predecessor, changed-body commitments, named exclusions, protected direct links and complete old/new recovery; historical format bytes are not a research format choice |

The main uncertainty is how an eventual physical profile proves the complete
cross-owner outcome and native cutoff. Investigation of the child command
precedent and research authorization/completion contracts shows that this is
already an explicit unqualified boundary. An ordinary create followed by separate
hierarchy writes, a valid hash or an accepted logical recipe supplies no such
mechanism. This proposal preserves that blocker instead of inventing a paired
transaction or treating header agreement as native ownership proof.

A second risk is giving all retry callers the same terminal-state exception.
The report, coordination, review and Main decision contracts instead distinguish
current child/parent authority from retained Main-authorized historical retries.
The operation-specific rules below preserve that difference.

## Capture, derive and revalidate one complete outcome

H denotes the owning hierarchy store's captured `header.latest` in its exact
workspace/storage epoch; C denotes the new outcome's CommitRef. These are existing
notations, not newly serialized fields. Under an eventual qualified publisher:

1. Resolve the exact typed operation, original source owner and authority under
   its owning admission policy. Normalize/validate the request where that policy
   prescribes it; enforce its current authority and rate checks before original
   retry lookup. An exact accepted retry follows the separate read-only path below.
2. For a genuinely new outcome, capture H, full original read values/commitments,
   live control versions, immutable source and original proofs, complete lookup
   ranges and source/index correspondence. Include required absent keys and empty
   families through complete discovery, never through a caller's absence flag.
3. Derive the candidate **internally** from those captured originals. Resolve the
   complete affected consumers and invalidations under the accepted owner table.
   Evaluate any prescribed completion predicate over that candidate, retaining
   original read snapshots for every compound decision.
4. Coalesce the full original-to-final effect union and exact resource projections
   below. Check every independent limit, including discovery and final validation.
   Unsupported owners, missing producers/representations or unknown effects make
   the whole operation unavailable; they are not empty effects.
5. At the separately qualified commit/cutoff boundary, revalidate H and the entire
   captured set: read-only peers, absence/ranges, source/consumer membership,
   authority, lease/health and actual ordinary/native facts as applicable. Drift
   denies this captured attempt; never patch snapshots, retarget the normalized
   request or silently retry under changed facts.
6. Commit the complete owning outcome with predecessor H, checked successor
   sequence, one new C and its header/new-proof controls. Only explicit fresh-store
   initialization uses its accepted null predecessor/initial sequence; additive
   profile admission to an existing store, where its eventual physical profile
   permits coexistence, must preserve the actual H and cannot reset the epoch or
   create another null-predecessor outcome. Preserve each owning operation's
   prescribed bounded cursor/outcome event effects, including clarification
   assessment/answer events; deliver notifications only after durable acceptance.
   This selects no new event row or transport mechanism. A callback, event or suppressed response proves
   neither commit nor rollback and cannot replace a current read.

Ordinary/native producers retain their own exact identities, revisions, proofs
and recovery ownership. H does not turn an independent ordinary store into the
hierarchy store or authorize cross-store writes. An operation needing their
coherent joint disposition remains unavailable until its actual bridge and
atomic/recovery boundary are qualified. Ordinary input/stop/end must not wait on
hierarchy SQL/fsync, discovery or recovery, or hold an input-critical lock through it.

### Mandatory effect union

| Owning part | Required coverage, including facts that may remain unchanged |
| --- | --- |
| Source/control | Root/run/bootstrap and original prepared input; exact configuration/scope/captures, plans/approvals, reservation/allocation/attachment/assignment, runtime history and live controls; immutable originals remain retained |
| Deliverable/lifecycle | Exact report/head/history/request/receipt, review/input, turn/coordination/recovery, clarification, summary/selector, proposal and lifecycle records actually consumed or produced by the operation |
| Ordinary/native bridge | Original unique ordinary lineage/preparation, current runtime/producerRevision and capacity/disposition sources; no copied status, directory, model result or timeout settles an unknown native outcome |
| Consumers/custody | Complete forward/reverse source and object membership, active preparations and historical consumers; every actual #744 custody/evaluation/artifact/summary, #745 exclusion/shared custody and #746 tracking/detail owner, under its unchanged recipe |
| Resources | Exact assignment/run/workspace ledger origins, source/control/preparation/report pools and applicable aggregates; original proofs and retained occurrence charges; read-only peers are fenced and not restamped |
| Derived lookup maintenance | Complete final source-to-index correspondence for all changed membership and unchanged required ranges, including zero-reference owners, retired expectations, tombstoned lineage and registered empty families |
| Commit/proof controls | All changed logical rows, original before/final after commitments, original excluded links and direct proofs, plus separate header and new complete proof controls; physical index/proof bytes and mutations are not omitted from complete-command/store accounting |

A row list in a lifecycle/recovery readSet is a closed snapshot **projection**,
not permission to omit the rest of the actual union. Consumer invalidation changes
source-owned currentness/pointers and fences dependent work; it never rewrites
immutable report, review, input, summary or original decision history.

## Per-operation effect recipes

Every row inherits the complete contract above and its linked admission,
state, resource, currentness and retry rules. Read-only fields are not promoted to
writes to stamp C. Different rows or phases are not automatically one transaction.

| Logical operation / authoritative recipe | Required local outcome and boundary |
| --- | --- |
| Workspace initialize-store, root admission, attachment preparation / [accepted initializer](2026-11-23-research-initializer-manifest-design.md#the-complete-creation-manifest) | **Three distinct original outcomes**, each with its own proof/header advance; never one transaction spanning their lifetimes. Register complete families at profile admission; create each required resource/owner/selector/root or child subject row only at its accepted boundary. Include root draft deletion/source insertions and charged ledger replacements; attachment includes its original authorization/assignment/ordinary prepared bridge. Preserve peers and additive profile coexistence; no opportunistic repair |
| Draft prepare/cancel / [initialization recipe](2026-11-13-research-initialization-catalog-design.md#logical-initializer-and-admission-effects) | Original draft/artifacts and preparation-only ledger effects, burned namespace and exact authorized retry; no admitted run/source/subject/resource fabrication. Cancellation deletes only the proven whole unadmitted draft group |
| Parent attach/exact resume / [prefix control](2026-11-12-research-input-control-prefix-design.md#proposed-normal-version-trace-and-local-effect-contract) | Ordinary parent attachment replacement, immutable research runtime history, current parent pointer and the actual owning version effects coherently. Resume preserves gates and immutable bootstrap; retained receipts never revive credentials or an expired grant. No new parent/resume producer is qualified here |
| Research assessment/proposal and scope replacement / [prefix](2026-11-12-research-input-control-prefix-design.md#proposed-normal-version-trace-and-local-effect-contract), [prospective membership](2026-11-16-research-prospective-membership-design.md#current-reports-and-immutable-input-invalidation) | Derive the fresh input before the immutable plan digest, retain exact decision/producer and source objects, insert plan/task controls and declared collection/expectation membership, and update pending/current input pointers as prescribed. Preserve prior sources and original collecting ownership; replacement membership is not a second RunKey collection. No grant, reservation or child follows from publication |
| Research exact-plan approval / [prefix](2026-11-12-research-input-control-prefix-design.md#proposed-normal-version-trace-and-local-effect-contract) | Retain exact shown-byte approval/receipt and changed plan/run controls. Any fresh volatile exact-plan grant requires the existing eligibility rules and is never restored from the archive; approval itself neither reserves a slot nor creates a child |
| Research reservation / [original source chain](2026-11-12-research-source-codecs-design.md#original-reservation-allocation-attachment-and-assignment-sources) | Original reservation, frozen expected/assignment version pair, task/launch/expectation and capacity effects atomically **before** native allocation. Capacity counts the unsettled reservation or its unique child once, against both immutable run and plan limits |
| Allocation and spawn intents/outcomes / [prefix](2026-11-12-research-input-control-prefix-design.md#proposed-normal-version-trace-and-local-effect-contract) | Each durable intent precedes its native action; the later outcome is a distinct publication with newly captured original facts. Publish successful allocation/runtime sources only on actual proof; retain original attempt, observations and interrupted/unknown disposition otherwise. No transaction is held across native allocation/spawn, and intent is not success |
| Ordinary end/resume / [prefix](2026-11-12-research-input-control-prefix-design.md#proposed-normal-version-trace-and-local-effect-contract) | Coherent actual ordinary runtime/producerRevision and session/launch/task/run bridge effects; derived capacity changes only from those sources. No second orchestrated attempt, reopened terminal run/plan, synthesized coordination result or restored grant |
| First/corrected research report / [publication](2026-11-12-research-membership-publication-design.md#first-report-read-set-and-original-to-final-effects), [post-rejection](2026-11-16-research-post-rejection-report-design.md#captured-input-report-transition), [stage report](2026-11-16-research-stage-report-design.md#report-producer-and-immutable-input-effects) | Complete report/head/original receipt plus input derived from the captured latest manifest, exact selected stream and expectation/collection versions, current pointer/run advance and all stale-summary/proposal/tracking effects. Copy the old assessment unchanged and retain its own origin. Preserve rejected prospective plans and cancelled-stage gates. No task outcome/version, ordinary bridge revision, grant, capacity release or automatic plan completion follows from report arrival |
| Original turn evidence and parent coordination / [continuation](2026-11-15-research-continuation-design.md#retained-original-turn-completion-evidence-proposed) | Retain exact original turn/runtime/source evidence and then the distinct authenticated parent request/result/receipt with actual task/run advances. A deliverable is not a turn result. Researcher completion is report-only; the review task enforces its own current-review gate. A final eligible coordination result includes automatic completion in the **same** outcome |
| Original-attempt recovery / [recovery](2026-11-15-research-continuation-design.md#typed-original-attempt-research-recovery-proposed), [positive completion](2026-11-15-research-recovery-completion-design.md#exact-existing-representations-and-compound-outcome) | Preserve the four exact kinds and original identities; clear only proven resolved owner-bound blockers, never all task blockers. If recovery resolves the final condition, both original decision snapshots bind the same H and both receipts the same C; run/task/plan advances are coalesced, not a later completion journal call |
| Scope rejection / [rejection union](2026-11-16-research-scope-rejection-design.md#typed-completion-trigger-and-compound-effects) | Retain original Main rejection; cancel only the rejected proposal, retire only its exact prospective membership, and rerun completion over the candidate. Any eligible current-plan completion, grant revocation, selector/summary effects and original receipts share the rejection's C and original read snapshots. Preserve approved child/report ownership and actual capacity |
| Review snapshot/outcome and reviewer replacement / [review contract](2026-11-18-research-review-tasks-design.md#review-request-admission-and-family), [currency](2026-11-18-research-review-tasks-design.md#currency-conflicts-and-invalidation) | Retain the original snapshot at its reservation/preparation boundary and immutable outcome with exact current selected-report binding. Outcome arrival changes no task coordination version/result. Corrected reports stale the old review without rewriting it; a replacement needs its own exact approved task/source chain, not a second outcome under the old TaskKey |
| Clarification assessment / [asking-side effects](2026-11-18-research-clarification-design.md#asking-side-assessment-effects) | New fresh-decision input and byte-equal immutable question request, exact pending binding, sole run advance and all stale consumers. Keep stage/blocker/runtime precedence; no child/report/grant/capacity effect |
| Main clarification answer and later fresh assessment / [answer effects](2026-11-18-research-clarification-design.md#atomic-answer-effects) | Answer/request/receipt and user-origin next input copy the old decision unchanged, clear only that pending binding and invalidate stale consumers. A fresh parent decision is a **separate later outcome** requiring its own eligible runtime; retain the answer records it uses. No answer inside a resume, overwritten resolved request or implicit approval |
| Research completion and synthesis / [completion effects](2026-11-15-research-completion-design.md#source-owned-transition-and-effect-union) | Completion joins its actual typed final-condition trigger, plan/version/grant and selector/current-plan effects in one C. Synthesis is separately authorized, inserts its immutable summary and selects the next summary version without changing input or inventing a fresh assessment. Plan completion preserves eligible collecting rights and nonterminal child capacity |
| Main finish/cancel-stage/continue-stage/cancel-run / [lifecycle effects](2026-11-15-research-completion-design.md#source-owned-transition-and-effect-union), [stage continuation](2026-11-16-research-stage-report-design.md#explicit-continuation-consumes-current-evidence) | Exact action-specific decisions/readSets/receipts; whole owned invalidation/retirement/revocation union. Stage cancellation retains collecting rights and its independent continue gate; continuation consumes current input plus the exact original cancellation and clears only that gate. Finish/run cancellation retire proper collections and revoke applicable authority without ending children, refunding capacity or erasing raw gates/history |
| Execution proposal/replacement and exact approval/rejection / [execution contract](2026-11-19-research-execution-proposal-design.md#execution-proposal-plan-source), [decision effects](2026-11-19-research-execution-proposal-design.md#exact-approval-rejection-and-invalidation) | Proposal binds current summary/completed research plan/input and publishes **no input revision**; invalidate every earlier recognized pending proposal explicitly. Approval retains original shown-byte source/receipt and eligible volatile grant; rejection retains its lifecycle decision and only that proposal's cancellation/pointer effects. Neither supplies the separately gated execution launch chain |
| Final execution completion / [terminal union](2026-11-19-research-execution-completion-design.md#absorbing-run-terminal-transition) | Typed coordination/recovery trigger, completion and absorbing terminal transition share C. Revoke all applicable run/planning/grant/child-report authority, retire proper collections, close waits and invalidate pending proposals. Preserve the research selector invariant, immutable history, raw gates and actual live-child capacity; no TerminalReportRunV1 conversion or second run counter |

An exact request/source that does not fit its selected owning branch is unavailable;
this table never widens the examples into a general publisher. Direct-task skip,
broader execution launch/recovery, unsupported terminal correction, and any
unclosed source/consumer/ordinary/native variant remain gated. A named semantic
event without its exact admitted command representation is not an executable
operation. No caller may choose a nearby table row or tag to bypass that boundary.

## Coalesced versions and exact accounting

Use the accepted sole `research_run_controls.runVersion` owner. Advance it once
for one admitted semantic run transition, even when that outcome contains several
records, decisions, invalidations or completion effects. A changed plan control
advances its planVersion once; the task/launch/session owner advances live
taskVersion only for the event its contract defines; an actual ordinary event
advances its own producerRevision. Frozen assignmentTaskVersion and original
reservation/source pairs never change. Collection/expectation and input/summary
versions follow their own exact rules, never a generic increment-per-row policy.
A summary pointer clear alone does not allocate a new summary version. Unchanged
rows keep their original links. Every overflow denies the whole new mutation.

Coalesce each exact ledger key from original directly to final state using its
owner's charge projection. Equal bytes retained as separate occurrences remain
separately charged; no intermediate-version charging, deduplication discount,
new allowance, missing-peer creation or lastCommit-only rewrite. Report pools do
not borrow #744 receipt credits or artifact read-work debits. Original input,
report/review/summary and snapshot history remains charged under its own pools.

Keep complete-command **4,096 changed rows / 16 MiB** distinct from cumulative
discovery/final-validation **4,096 observations / 16 MiB**, the 100,000-visited-row
ceiling, cooperative SQL deadline and every smaller row/job/worker/resource/store/
index/physical limit. Header/new-proof controls and required physical lookup work
count in their applicable complete-command/store bounds even when excluded from
the logical change list. Deletion is not negative command-size credit. A locally
legal set of rows can exceed a containing bound and must deny, without truncation
or splitting. No simultaneous-maxima, complete byte total or physical peak is
qualified by this document.

## Original proof closure, retries and uncertain disposition

For every changed logical key, retain its exact original before and final after
commitment once, with the insertion/deletion null counterpart under its owning
codec. No missing, extra or duplicate logical changes; no link-only churn.
Header and the new proof are separate controls, not recursive entries in their
own logical manifest. A changed row's owning outer lastCommit and a newly issued
receipt's commit name C. Copied snapshot/receipt/checkpoint origin links retain
their own original proofs under their exact codecs; they are not all replaced
with C. Unchanged rows retain their actual original direct links. Internal source identities remain
committed; exclude only paths explicitly named by the owning codec. This document
adds no research exclusion or replacement hash algorithm.

Resolve each required retained row, immutable snapshot, receipt, checkpoint and
other excluded commit link to its **own original complete proof**, in the same
owning workspace/epoch. Where the contract captures a snapshot at H, its direct
origin is at or before H and its full original before-image/source correspondence
is proven. Compound recovery/rejection/completion decisions capture the same
original H and original reads, never repaired candidate snapshots. Original
receipts and new outcome links agree on C without inserting new C into a body
whose digest is needed to compute C. Preserve each existing codec's construction
order and named exclusions; a new fixed point or missing physical binding makes
the attempted profile unavailable.

Protect every proof still needed by retained owners/history, even after current
pointers clear or runs close. Do not require recursive replay of all unrelated
predecessors, elect a header/owner from maxima, synthesize a missing receipt,
reconstruct an old result from current rows, rebuild indexes on read, or adopt
format-3/4/6 commitments. A source digest or valid receipt alone is not proof of
the complete original outcome or actual cutoff.

### Operation-specific exact retry boundary

| Caller/operation | Checks and read-only retry behavior |
| --- | --- |
| Child research report/correction or review outcome | Current exact child capability/generation/runtime/run/epoch/owner/lease/health and the owning rate checks precede original lookup. Final revocation denies before lookup. Identical accepted bytes return the original receipt before live CAS, with no new write/version/event/charge; no Main terminal exception or #744 claim-first cutoff is borrowed |
| Parent planning/coordination | Current exact parent bearer/scope/generation and owning mutation-rate policy precede lookup. A terminal/revoked parent's archive cannot restore that authority. The existing result-only stage rights do not authorize new planning/proposals; exact accepted retries precede live CAS under the owning operation's policy |
| Main UI approval/answer/lifecycle/recovery | Current Main workspace/epoch/owner/lease/health and exact shown-context authorization under the specific contract precede original lookup. Where that contract permits a previously accepted identical retry after run termination, it returns original historical bytes before live CAS; a new action's eligible-parent/nonterminal/version preconditions must not be incorrectly imposed on that read-only retry. Historical success restores no pointer, summary selection, grant or runtime |
| Original preparation or native/ordinary outcome producer | Its own exact permit/source identity, current cutoff/lease/health and original-outcome policy govern. The preparation authorization is not a #744 claim and cannot be changed into child-report authority. No generic retry API, successful native outcome or repair permission is created by this table |

Conflicting request bytes/identity fail under the owning conflict rule. Missing
or ambiguous original disposition remains unavailable. Exact accepted retry is
not a fresh mutation, initialization, event, charge or proof/header advance.
Currentness-sensitive reads still resolve current sources; a historical receipt
does not reselect an old summary or certify current report/review bindings.

Proven rollback keeps the complete old state; proven commit keeps the complete
new state; unknown I/O/lease/commit/native disposition retains the original owner,
attempt and conservative capacity, exposes unavailable/reconciling, and fences
dependent work. Apply the existing independent close-only revocation rules;
no successful durable terminal receipt is inferred from a timeout or transport
abort. Never replace the worker, replay into a new epoch, restore volatile
credentials, purge protected proofs or launch a second attempt to resolve doubt.

## Owner choices and next boundary

Accept or revise these three choices for **written command/proof work only**:

1. The complete capture/derive/revalidate/effect-union contract, including the
   per-operation recipes and their unsupported-variant boundaries; typed source
   owners derive all affected effects and preserve operation-specific authority.
2. One H-to-C original-to-final outcome per actual durable phase, coalesced sole
   versions and ledger deltas, complete original direct-proof retention and
   operation-specific read-only retries. Compound triggers share one outcome;
   initializer/native phases and fresh reassessment stay distinct as prescribed.
3. Unchanged independent bounds and fail-closed recovery, with no executable
   tag/format/physical manifest selection, dependent fixture or runtime authority.
   Audit item 5 remains the next separately reviewed physical-profile boundary.

Acceptance would close only this **written logical** boundary. It would not
qualify a complete command, real producer/cutoff, actual ordinary/native bridge,
physical format/index/proof profile, legal full-state history, adapter or
activation. No offline component packet follows from this proposal. Exact
command representations and physical obligations must be explicitly closed
where still missing before dependent fixtures or an implementation plan.
#1689/#1089/#610 stay open; #740 stays closed research.

## Review and verification handoff

One writer: the owning session. One bounded round with two fresh read-only
reviewers: contract correctness (authority/retries, full effect and original-proof
joins), and builder completeness (missing command/phase/version/consumer cases
and physical-boundary honesty). Reviewer cap two; no reviewer edits, fixtures,
product/native/model/database execution or new harness. Verify documentation
metadata/index, docs tests/build/dead links, rendered sections, preservation of
prior validation artifacts, append-only audit changes and diff/doc/worktree
close-out. Publication/merge does not accept the three choices above.


## Bounded review disposition

One round used two fresh read-only `gpt-6-luna` reviewers for contract correctness
and builder completeness. Neither found a material logical-contract gap within
the stated written scope. The builder noted that the clarification rows could
name their required cursor events more explicitly; the shared outcome rule now
preserves those existing bounded events, matching the clarification contract's
asking-side and answer effect tables. This is a wording clarification, not a new
event codec, persistence rule or transport choice.

Review covered operation-specific child/parent/Main retries, same-H/same-C
recovery and rejection completion, coalesced versions, original proof origins,
consumer/resource coverage and honest physical/executable deferrals. Reviewers
ran no fixtures or product/native/model/database probes. Static agreement and
documentation checks do not qualify a physical manifest, original authority,
actual cross-owner outcome, full-state history or production writer.
