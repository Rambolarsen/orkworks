---
type: Qualification Audit
title: Remaining research source and variant coverage
description: Source-backed map of scope-resolution, research-review, clarification and execution gaps before complete research catalog and history qualification.
tags: [orkworks, research, sources, lifecycle, qualification]
status: draft
workflow_status: written-coverage-audit-only
---

# Remaining research source and variant coverage

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `4a7851ef` (origin/main, including the positive recovery packet
merged in [#1772](https://github.com/Rambolarsen/orkworks/pull/1772)).

## Result and scope

The accepted logical lifecycle is broader than the offline research codecs.
The packets establish finite decoded component relationships for a single
retained-file research task, with no material clarification or required review.
They do not establish a complete source-valid state, history or production writer.
The positive recovery packet closes one previously negative completion case;
it does not close pending scope-revision resolution, research review, material
clarification or execution continuation. Gate 2 remains **Investigate**.

This audit derives missing obligations from existing contracts. It selects no
new field, tag, family, domain, format, credential, source owner implementation
or executable plan. Recorded acceptance remains valid at its original scope;
new representations still need scoped review before dependent fixtures. No
runtime, native/model probe, database, migration or product code is supplied.

## Evidence and current production seams

The inspection read the accepted
[ordinary-child completion and revision rules](../superpowers/specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md),
[clarification/research lifecycle](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md),
[research source grammar](../superpowers/specs/2026-11-12-research-source-codecs-design.md),
[input/control prefix](../superpowers/specs/2026-11-12-research-input-control-prefix-design.md),
[membership contract](../superpowers/specs/2026-11-12-research-membership-publication-design.md),
[initializer/catalog boundary](../superpowers/specs/2026-11-13-research-initialization-catalog-design.md),
[continuation](../superpowers/specs/2026-11-15-research-continuation-design.md)
and [completion contract](../superpowers/specs/2026-11-15-research-completion-design.md).
Their later acceptance appendices govern the component scope; historical
proposal introductions do not revoke that acceptance.

A bounded search of `crates/orkworksd/src` and `apps/desktop/electron` for
`ScopeResolution`, `ClarificationRequest`, `ClarificationAnswer`, `ResearchSummary`,
`ResearchSourcePlan`, `research_run_controls`, `research_lifecycle`,
`ResearchReview`, `PreparationDecision` and `researchNeeded` found no matches.
This is reproducible exact-name evidence, not exhaustive absence proof for
external or differently named implementations.

| Inspected source | Current responsibility and boundary |
| --- | --- |
| [coordinator.rs](../../crates/orkworksd/src/taskmaster/coordinator.rs), module header | Dormant data-only records explicitly lack runtime authority; its generic plan is not a research source codec |
| [hierarchy_storage/mod.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mod.rs) and [connection.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs), `Connection::open` | Mechanics remain test-only; production open returns unqualified profile before using the path |
| [Existing research owner map](../superpowers/specs/2026-11-12-research-input-control-prefix-design.md#inspected-owners-and-proposed-responsibilities) | Separates dormant validation, ordinary lifecycle, Main authority and missing research publication; its proposed research modules are documentary boundaries, not existing producers |
| [Selected child source grammar](../superpowers/specs/2026-10-31-child-review-source-preparation-design.md#outcome-and-selected-slice) | Direct worker/reviewer sources do not constitute a research reviewer or execution continuation of a research-owned run |

No installed-app behavior is inferred. Ordinary lifecycle observations,
source hashes, copied snapshots and a supplied CommitRef do not prove an
atomic hierarchy bridge, actual native disposition or original authority.

## Supported component variants and their stopping points

Here “supported” means only the named offline examples and finite checks.
It does not mean general wire decoding, adapter eligibility or runtime support.

| Area | Existing bounded evidence | Limit that must remain explicit |
| --- | --- | --- |
| Source and startup | [Capture/scope](research-capture-scope-components.md), [startup](research-startup-components.md), [original chain](research-chain-components.md) | One task, retained UTF-8 files, empty dependencies, no reviewer, no future report input or parallel plan |
| Input and collection | [Prefix](research-prefix-components.md), [membership](research-membership-components.md) | ResearchDecisionV1 fixes clarificationNeeded=false/researchNeeded=true; input records cover configuration, scope, captures and selected research report only |
| Admission and preparation | [Initializer](research-initialization-components.md), [authorization](research-authorization-components.md) | Conditional resource values and original research preparation joins; no full catalog, source-valid creation or actual paired/native outcome |
| Turn and result | [Continuation](research-continuation-components.md) | Three turn-evidence alternatives, explicit parent outcomes and four original-attempt recovery kinds; hypothetical Stop is not native capability evidence |
| Completion | [Completion](research-completion-components.md), [positive recovery](research-recovery-completion-components.md) | Coordination and exact eligible recovery triggers only; the positive recovery keeps an independent approval blocker and leaves the run blocked |
| Synthesis and UI | [Completion](research-completion-components.md) | Summaries require proven no-required-review membership for empty review refs; six UI alternatives do not qualify arbitrary clarification, pending execution proposals or a complete terminal producer |

The completion contract's lifecycle trigger union has exactly coordination and
recovery forms. “Scope resolution” in the remaining-work handoff refers to
resolution of a pending proposed scope revision, including its rejection. It is
not a new name for ResearchScopeV1 capture validation or original-attempt recovery.
A transition that clears the last condition through an unsupported trigger must
deny as a whole; it cannot commit the resolution alone and defer completion.

## Missing source owners and effect boundaries

The responsibilities below route to the already documented proposed seams:
research source validation, semantic control, authority, publication, Main UI
context and ordinary lifecycle bridge. They do not appoint a new independent
writer or assert that those modules exist. Each future source contract must
close its exact key/body/domain, immutable origin, request/receipt/retry,
current read set, original-to-final writes and complete membership before bytes.

| Remaining variant | Established logical behavior | Source and effect contract still missing |
| --- | --- | --- |
| Pending scope proposal and resolution | A pending scope revision blocks completion. Rejection or any other final-condition resolution reruns completion atomically. Changed scope requires an exact new plan approval; prior live children constrain revision approval | Exact pending proposal identity/source, authorized rejection/resolution and immutable disposition/receipt, freshness and current pointer; typed completion trigger and captured versions; one compound outcome containing resolution, completion/grant revocation, sole run advance, dependent invalidation and charges |
| Research reviewers and report-consuming tasks | Approved research declares researcher/reviewer tasks and batches. Reviewers consume reconciled research; missing research/conflicting required review/material uncertainty blocks ready execution | Research plan/configuration/dependency and retained read-input variants; exact reviewed report versions, reviewer identity/configuration/runtime, review outcome and conflict/currentness selection; summary review-reference codec; complete collection/consumer and invalidation ownership |
| Material clarification and changed answers | Only explicit Main-authorized UI answers resolve requests. Stale answers reject, identical answer IDs retry, conflicts fail; changed answers create a new preparation revision. Unanswered material questions block dependent approval | Closed request/answer/provenance records and bounded suggestion/text fields; input-record/decision and current pending-reference variants; original receipts and user-origin input producer; atomic answer/input/summary/proposal effects, runtime-pause/stage-gate precedence and historical retention |
| Research-to-execution proposal and approval | Same parent/run, new planId, exact current input and completed research; new execution approval, one launch grant, independent live-child capacity and new plan worktree groups | Execution source codec joining exact SummaryRef/completed PlanRef/current input and bootstrap; fresh assessment including skip-research; exact shown-byte approval, rejection/replacement/invalidation and grants; complete execution plan/task/ordinary/source ownership while retaining the sole research run counter |
| Execution final completion and terminal continuation | Final execution completion revokes run authority. Complete/cancelled are absorbing; no automatic child termination or cross-plan transfer | Exact execution final-result/completion trigger, terminal source/projection/consumer and proof mapping; complete authority revocation and wait/collection disposition; honest retained capacity/native outcomes and original retries |

Applied to each row, the existing documentary seams imply this responsibility
map; the exact routes and producer agreements are still missing:

- **Scope resolution:** `research_sources.rs` for pending/disposition joins,
  `research_control.rs` for the completion predicate and coalesced versions,
  `research_authority.rs` with Main `researchApproval.ts` for current authorized
  context, and `research_publication.rs` for the complete compound outcome.
  Ordinary/native owners must supply any settlement facts consumed by the check;
  rejection itself neither ends a child nor invents a new bridge event.
- **Research review:** `research_sources.rs` for declared inputs/reviewer outcomes,
  `research_control.rs` for required-review/conflict/currentness selection,
  `research_authority.rs` for distinct exact scoped read/report rights, and
  `research_publication.rs` for retained consumers and invalidation. Actual
  reviewer runtime origin belongs to `session_application/research_bridge.rs`.
  Review delivery/transport remains unassigned: the proposed
  `http/research_handlers.rs` child-report seam is not a general review API.
- **Clarification:** `research_sources.rs` for immutable request/answer/input joins,
  Main `researchApproval.ts` with `research_authority.rs` for trusted UI answer
  provenance, `research_control.rs` for pending/input/gate transitions and
  `research_publication.rs` for their atomic retention/invalidation. These names
  do not close a clarification UI operation or user-origin producer codec.
- **Execution proposal/approval:** `research_sources.rs` for exact proposal and
  input/summary provenance, Main `researchApproval.ts` with
  `research_authority.rs` for new exact approval and its separate grant,
  `research_control.rs` for the original sole run owner and plan/task/capacity
  checks, and `research_publication.rs` for all retained effects. The ordinary
  bridge owns actual child/runtime/terminal facts; transport and execution-source
  admission agreement remain unclosed.
- **Final execution/terminal:** `research_control.rs` for terminal absorption and
  the actual final-condition outcome, `research_authority.rs` for revocation,
  `research_publication.rs` for complete consumer/proof effects, and the ordinary
  bridge for honest child/native disposition. `research_sources.rs` must resolve
  the typed original outcome. This does not cast a terminal projection or make
  Main the author of a parent coordination result.

The [small direct-task path](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md#example-workflows)
also skips both clarification and research when supplied evidence suffices.
Its skip assessment/input and execution sources remain unclosed in this research
profile. It cannot fabricate a completed research plan or SummaryRef to fit the
research-to-execution branch. The same execution responsibility seams above need
an explicit direct-path agreement. Future-output independent review is only an
intention until the result exists; the accepted #744 flow then needs a separately
user-created review run and exact approval, without transferring the finished
worker run's planning bearer. This is distinct from research-report review.

Research-report review is not automatically #744 assignment evaluation. The
[exact-result timing boundary](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md#proposed-exact-result-reviewer-timing--2026-10-24)
and [selected retained reviewer grammar](../superpowers/specs/2026-10-31-child-review-source-preparation-design.md)
require already accepted target inputs for their own profile. They provide no
deferred future-output selector, conversion of a research report into a #744
result, or permission to fill required reviewReferences with an empty list.

Clarification may refine the authorized goal but cannot broaden immutable
bootstrap permissions. New parent instructions/skills/profile or a new goal
needs its separate UI creation/relaunch path. Parent/runtime resume preserves
pending clarification, blockers and stage-continue precedence; it neither
answers a question nor reapproves a cancelled plan. These are existing logical
constraints, not newly selected clarification codecs.

Execution continuation must retain research_run_controls as the original sole
mutable runVersion owner. Similar child or TerminalReportRunV1 fields cannot
supply a second counter, a delayed initializer or a profile switch. Research
completion/summary never supplies execution approval or frees ordinary capacity.

## Catalog, indexes, initialization and proofs still required

The conditional inventory is **57 logical names**, derived as the earlier
50-name candidate plus research preparation authorization (1), continuation (3)
and lifecycle/summary (3). This is neither a maximum nor a complete executable
catalog. Missing variants above may change its reviewed composition. The
[completion placement](../superpowers/specs/2026-11-15-research-completion-design.md#logical-catalog-initialization-projections-and-resources)
already requires a lifecycle selector at original root admission, adding one
conditional logical row (initialized 27→28; never-initialized 24→25). The earlier
initializer packet does not retroactively contain that selector or prove its
creation. The fourteen workspace, nine run and five assignment resource names
keep their established meanings; there is no new zero-ledger allowance.

Before a complete physical-history fixture, its owning design still needs:

1. **Closed catalog and owner selection.** Enumerate every supported object,
   metadata/key/value/reference, origin, command/read snapshot and resource
   variant under one recognized profile, including mandatory empty families.
   Select original source ownership per RunKey/AssignmentIdentity at admission;
   row presence, largest versions and copied child/terminal sources cannot elect it.
2. **Complete discovery and invalidation.** Derive both lookup directions and
   historical direct origins for plans, inputs, report heads/receipts,
   prospective expectations/collections, runtime/ordinary lineage, turns,
   results, recovery, lifecycle selectors/records and summaries. Include each
   admitted new variant's consumers and zero-reference owners. Absence of a
   head or matching edges does not prove absence of prospective consumers.
3. **Actual initializer and resource inventory.** Create every required owner,
   selector, empty family/index and ledger in its original outcome; preserve
   peers and exact occurrence accounting. Reconcile complete source/control/
   report/preparation and aggregate deltas, copied snapshots and physical
   indexes/proofs. No missing-ledger repair, deduplication discount or counter reset.
4. **Command and original proof closure.** Join the full source/consumer/ordinary/
   resource/index/header effect union, captured H and one new C; coalesce sole
   live versions once. Retain every original receipt, excluded commit link and
   immutable direct proof. Exact retries use current authority before original
   lookup and return historical outcomes without reconstructing today's rows.
   A source hash or matching receipt is not proof of a complete command.
5. **Physical and authority qualification.** Select the full format/commitment/
   proof/index/recovery contract without retokening format-3/4/6 artifacts;
   satisfy every smaller row, snapshot, coupled-job, command, discovery and
   physical limit. Independently qualify current authority/cutoff, custody,
   ordinary storage activation and native bridge participation. Finite supplied
   facts and component byte totals establish none of those boundaries.

## Bounded next task and acceptance boundary

The recommended next written task is the pending-scope-revision rejection and
completion source contract. It closes the smallest remaining completion-trigger
gap before expanding into research review, clarification or execution codecs.
It should name exact pending/source ownership, Main-authorized request and
original receipt, typed trigger, versioned compound effects, complete retained
membership and unchanged quota/compatibility boundaries. It must distinguish
rejection from approved revision replacement and preserve existing live-child
and scope-approval constraints.

This is a recommendation within #1689, not approval of a guessed schema or a
requirement to reapprove existing lifecycle behavior. A concrete new
representation needs one scoped owner decision before dependent offline
fixtures. The other matrix rows remain explicit prerequisites to claiming
broader catalog coverage. Keep #1689/#1089 open and #740 closed research;
this audit cannot establish Gate 2 completion or authorize activation.

## Scope-rejection representation proposal — 2026-11-16

The [scope-rejection proposal](../superpowers/specs/2026-11-16-research-scope-rejection-design.md)
makes the exact prior-plan input, Main request/original retry and typed compound
completion trigger reviewable. It also identifies the singleton collection's
unclosed reserved-replacement source membership: a pending pointer cannot supply
that proof or authorize a second active task. The new choices remain proposed;
no dependent fixture, legal pending history or actual publisher follows.


## Reserved replacement membership proposal — 2026-11-16

The [written prospective-membership proposal](../superpowers/specs/2026-11-16-research-prospective-membership-design.md)
addresses the remaining singleton reserved-current-plan boundary after accepted
scope rejection. It proposes exact non-collecting pending membership and distinct
current-task report/current-input invalidation, with no approval handover.
New choices require scoped acceptance before finite components. It supplies no
legal pending history, complete source/index/physical profile or runtime gate.

## Post-rejection reporting prerequisite — 2026-11-16

The [post-rejection report contract](../superpowers/specs/2026-11-16-research-post-rejection-report-design.md)
proposes source/effect rules after proven B rejection with A either still current
or completed. It preserves rejected B-derived current input and original A
reporting ownership, and names completed-plan summary invalidation and original
outcome/proof obligations. Earlier prospective membership acceptance stands;
the post-resolution writer choices remain proposed and qualify no legal history.

## Stage-cancelled reporting written prerequisite — 2026-11-16

The [stage report/continue proposal](../superpowers/specs/2026-11-16-research-stage-report-design.md)
closes the next written source-role and current-input interaction after #1802.
Its three new choices remain proposed for scoped offline acceptance. Cancelled
approved-plan collecting ownership is distinct from rejected prospective-plan
membership; current report input must not clear the independent continue gate
or rewrite the original cancellation. No positive fixture, source-valid history,
complete catalog/initializer/physical profile or runtime/native activation follows.
Earlier technical audit statements remain historical and unchanged.
