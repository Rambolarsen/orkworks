---
type: Design Proposal
title: Research complete discovery and invalidation ownership
description: Proposed complete owner-family discovery inventory in both lookup directions over the accepted catalog, per-family invalidation ownership and shared finite bounds, under the accepted catalog enumeration.
tags: [orkworks, research, discovery, invalidation, membership, qualification]
status: draft
workflow_status: proposed-for-scoped-discovery-review
---

# Research complete discovery and invalidation ownership

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738); consumer
[#610](https://github.com/Rambolarsen/orkworks/issues/610).
Inspection base: `e7ceaa9b4f97` (origin/main, including the catalog-closure
acceptance merged in
[#2001](https://github.com/Rambolarsen/orkworks/pull/2001)).

**Proposed for scoped owner review.** The
[source-coverage audit](../../validation/research-source-coverage-audit.md)
names complete discovery and invalidation as catalog-closure item 2. This
proposal closes that written boundary over the
[accepted catalog enumeration](2026-11-23-research-catalog-closure-design.md):
the complete owner-family discovery inventory in both lookup directions,
per-family invalidation ownership, zero-reference/retired/empty-family
obligations, and the shared finite bounds. Every family's exact key, value,
digest domain, field set and per-row recipe remain owned by its merged
contract — this document synthesizes and assigns discovery/invalidation
ownership; it does not restate or alter them. No storage format, table/layout,
migration, runtime/native/model execution, adapter or executable plan is
supplied. Gate 2 remains **Investigate**.

## Outcome and preserved decisions

Make discovery and invalidation ownership closed and enumerable: for every
family in the accepted catalog, exactly one owner supplies its complete typed
membership in both directions; every event that can invalidate retained
consumers names its affected owners; and every complete inventory includes
zero-reference owners, mandatory empty families and retired/tombstoned rows.
Absence of a pointer, head, receipt or matching edge proves nothing on its
own — the accepted rule from the
[membership contract](2026-11-12-research-membership-publication-design.md#complete-discovery-and-the-positive-first-report-slice),
now extended to the full catalog.

Preserve unchanged: every accepted recipe and bound from the merged
contracts (listed in the
[catalog enumeration](2026-11-23-research-catalog-closure-design.md#enumerated-logical-catalog));
the accepted membership discipline ([child membership](2026-11-03-child-consumer-membership-design.md):
full typed identities, source-derived forward/reverse lookup, historical
direct proof retention, coherent complete discovery, no read-time repair);
the accepted discovery bounds; #744/#745/#746 ordinary families and the
child-source/report-registry catalogs under their own acceptances.

Rejected alternatives: caller-supplied affected lists; per-event ad-hoc
consumer crawls; invalidation by notification/receipt; repairing missing
membership at read time; and truncating discovery to fit bounds.

## Source trace and obligations

| Source | Established constraint |
| --- | --- |
| [Audit item 2](../../validation/research-source-coverage-audit.md#catalog-indexes-initialization-and-proofs-still-required) | Complete discovery and invalidation derive both lookup directions and historical direct origins for every admitted variant; absence of a head or matching edges does not prove absence of prospective consumers |
| [Catalog enumeration (accepted)](2026-11-23-research-catalog-closure-design.md#enumerated-logical-catalog) | 8 object kinds, 24 metadata families, shared report registry; mandatory empty families; admission-time owner selection |
| [Membership contract](2026-11-12-research-membership-publication-design.md#research-metadata-reference-recipes) | Exact family/key recipes; complete owner-family coverage; no orphan/duplicate/alias |
| [Continuation recipes](2026-11-15-research-continuation-design.md#complete-reference-and-admission-obligations) | Turn/coordination/recovery source-derived relations in both directions |
| [Completion recipes](2026-11-15-research-completion-design.md#source-membership-and-invalidation-obligations) | Summary/lifecycle/selector membership; source-owned current-summary and pending-proposal invalidation |
| [Review recipes](2026-11-18-research-review-tasks-design.md#collection-expectation-and-membership-extension) | Review-input/review-row/reviewer-expectation/summary-citation membership |
| [Clarification recipes](2026-11-18-research-clarification-design.md#precedence-resume-and-retention) | Request/answer/input-record retention in both directions |
| [Execution recipes](2026-11-19-research-execution-proposal-design.md#families-accounting-and-membership) | Execution plan/approval membership |
| [Invalidation precedent](2026-11-15-research-completion-design.md#source-owned-transition-and-effect-union) | Report/input changes invalidate summaries/proposals under source-owned currentness; unknown consumers deny |
| [Bounds precedent](2026-11-12-research-membership-publication-design.md#complete-discovery-and-the-positive-first-report-slice) | 4,096 observations/16 MiB cumulative discovery budget, 100,000-visit ceiling, no truncation or splitting |

## Complete discovery inventory

For every family in the accepted catalog, the table below assigns discovery
ownership. "Forward" means the owner derives its outgoing typed references
from original source bytes; "reverse" means the owner is discoverable as a
consumer/target from each of its exact references. Both directions are
mandatory; a one-directional family is unavailable. Every row inherits its
exact key/value/field rules from the owning contract — this column assigns
ownership, not new recipes.

| Family (exact key) | Discovery owner | Forward / reverse obligations |
| --- | --- | --- |
| research-source-capture / ResearchCaptureRef | research source validation | F: capture → repository/source-worktree bindings, task source group, retained bytes. R: discoverable from its scope's capture list |
| research-source-scope / ResearchScopeRef | research source validation | F: scope → every capture, question/excerpt joins. R: from its plan, configuration, expectations, review inputs |
| research-source-configuration / ConfigurationRef | research source validation | F: config → cited evidence objects, scope used by startup settings, embedded instruction snapshots. R: from plans, input records, expectations |
| research-source-plan / PlanRef | research source validation | F: plan → configuration/scope/group, frozen prepared-input binding. R: from approvals, reservations, plan controls, summaries, execution researchDependency |
| research-source-evidence / {task,referenceId,contentDigest} | research source validation | F: evidence → task/configuration capability reference + raw bytes. R: from its citing configuration |
| research-assignment-source / ChildAssignmentIdentity | research source validation | F: assignment → plan/config/scope/approval/reservation/allocation/attachment. R: from session states, #744 identities, launch/session/turn/coordination rows |
| research-review-input / {task,inputDigest} | review-task source validation | F: snapshot → reviewer plan/config/reservation/attachment, target report identity, scope, cited excerpts. R: from the reviewer task controls and its outcome row |
| research-execution-plan / PlanRef | execution source validation | F: plan → binding members (summary/completedPlan/input), configuration, artifacts. R: from approvals, plan/controls, lifecycle records |
| research_plan_approvals / {plan,approvalId} | research source validation | F: approval → plan, request fingerprint, review context, receipt. R: from its plan, reservations |
| research_reservations / TaskKey | research source validation | F: reservation → task/plan/config/scope/approval, frozen pair. R: from allocations, attachments, expectations, task/launch controls, turn/coordination rows |
| research_allocations / ReservationRef | research source validation | F: allocation → reservation, group, descriptors. R: from attachments |
| research_attachments / childSessionId | ordinary lineage producer | F: attachment → reservation/allocation, frozen identity. R: from session states, runtime attachments, assignment sources |
| research_control_inputs / PreparedInputRef **without inputDigest** | research input/control owner | F: input → root goal/bootstrap, config/scope/capture records, report/answer records, decision provenance. R: from run controls, plans, collections, summaries, clarification records |
| research_runtime_attachments / RuntimeKey | session bridge owner | F: parent/child attachment → original root/run/bootstrap or attachment/reservation chain. R: from run controls, session states, turn evidence |
| research_run_controls / RunKey | research control owner | F: run → currentInput, ordered PlanRefs, blockers, clarification binding. R: from every family keyed by RunKey |
| research_plan_controls / PlanRef | research control owner | F: plan control → task, approvals, replaces. R: from run controls (planReferences/current/pending), lifecycle readSets |
| research_task_controls / TaskKey | research control owner | F: task → plan/configuration, reservation, status. R: from expectations, session/launch controls, turn/coordination/recovery rows |
| research_launch_controls / TaskKey | research control owner | F: launch → reservation, frozen admission approval/parentRuntime, allocation/attachment, spawn history. R: from task controls |
| research_session_states / TaskKey | session bridge owner | F: session → reservation/attachment/assignment source, RuntimeRef. R: from task controls, turn terminal evidence |
| research_collections / RunKey | research control owner | F: collection → declared tasks, reviewTask member. R: from run controls, expectations |
| research_expectations / TaskKey | research control owner | F: expectation → collection, config/scope, reservation, selectedReport. R: from collection, task controls, first-report eligibility |
| research_turn_completions / TaskKey | research control owner | F: turn → task/reservation/child, evidence variant (runtime source or terminal session read). R: from task controls, coordination receipts |
| research_coordination_receipts / TaskKey | research control owner | F: coordination → task/reservation/child, request/result digests, turn gate. R: from task controls, completion triggers |
| research_recovery_decisions / {task,recoveryId} | research control owner | F: recovery → task/reservation/allocation/attachment, typed snapshots, native observations. R: from task controls, completion recovery triggers |
| research_lifecycle_state / RunKey | research control owner | F: selector → completed research plan, current summary, stage-cancel pointer. R: from lifecycle records, run controls |
| research_summaries / ResearchSummaryKey | research control owner | F: summary → plan/input/producer/report+review references. R: from selector, execution researchBinding |
| research_lifecycle_records / ResearchLifecycleKey | research control owner | F: record → request, typed snapshots/H/proofs, receipt/new C. R: from selectors, plans |
| research_reviews / reviewer TaskKey | research review owner | F: review → task/plan/config/reservation, reviewed report binding, cited references, turn/coordination receipts. R: from review inputs, summary citations |
| research_clarification_requests / {run,questionId,questionVersion} | research clarification owner | F: request → run/bootstrap, containing manifest, byte-equal question, producer. R: from answers, pending bindings, input manifests |
| research_clarification_answers / {run,questionId,questionVersion,answerId} | research clarification owner | F: answer → request, shown context, origin binding, new input revision. R: from clarification-answer input records, requests |
| research_execution_plan_approvals / {plan,approvalId} | execution source validation | F: approval → proposal, shown context, grant-binding facts, receipt. R: from approvals family, plan controls |
| research_preparation_authorizations / full canonical P | preparation authorization owner | F: authorization → the full canonical frozen identity P, original preparation provenance and body/proof joins. R: from the identity's ordinary member resolution; unique permit identity across both child and research authorization families |
| report_heads / exact Stream | report publication owner | F: head → report ObjectRef through the stream, history/receipt prefix. R: from expectations, evidence sources |
| report_receipts / {stream,reportVersion} | report publication owner | F: receipt → report, normalized request/closed response binding. R: from heads, history |

### Inventory completeness rules

Every complete inventory must include: zero-reference owners (families with
no rows and rows with no incoming/outgoing references); mandatory empty
families; retired expectations/collections and tombstoned ordinary linkage;
historical inputs and superseded plans; terminal runs and their retained
gates. Current-pointer-only scans are prohibited. A returned list of matches
establishes neither that all owners exist nor currentness — complete
source/index correspondence is required before current reads, writes or
purge. Unknown or unsupported consumers deny; they are never counted as
zero.

## Invalidation ownership

Each invalidating event names its affected owners; the effect union commits
in the owning outcome per the accepted transition contracts. The table
assigns discovery-side ownership of each invalidation edge:

| Invalidating event | Affected consumers (invalidation owner) |
| --- | --- |
| Research report accepted/corrected (new head) | Report publication owner → expectation selection, new prepared-input revision, current-input pointer, stale summaries (source-run detail snapshots), stale pending proposals, stale reviews (currency through the corrected selection) |
| Clarification answer accepted | Clarification owner → new input revision (copied decision), cleared pending binding, recognized pending proposals, current summary |
| Fresh parent assessment published | Input/control owner → stale copied assessments (never promoted), proposals bound to superseded inputs, and the current summary whose exact input binding the new revision supersedes (cleared under the selector's existing owner, immutable bytes retained) |
| Plan completion | Control owner → launch grant revocation, obsolete current summary clear, pending-proposal invalidation |
| Stage cancellation | Control owner → grant revocation, pending-proposal invalidation, retained gates (clarification/proposal mutations fenced) |
| Finish/cancel-run | Control owner → all run/child-report/grant authority, recognized pending proposals, collection/expectation retirement |
| Scope revision rejection | Scope-rejection owner → rejected prospective plan/expectation, completion rerun |
| Summary publication | Control owner → execution researchBinding currentness, tracked-run detail snapshots |
| Review outcome published | Review owner → nothing auto-invalidated (immutable; currency resolved through the reviewed report binding) |
| Terminal completion | Terminal owner → every recognized pending proposal, collections/expectations, run-closed waits |

Invalidation never rewrites immutable history; it advances source-owned
currentness/pointers and fences dependent work. A transition whose affected
consumers cannot all be discovered denies as a whole.

## Shared finite bounds

Discovery and final validation share the accepted cumulative research bound
— **4,096 logical observations/probes and 16 MiB canonical discovery bytes**,
each returned occurrence, separate read, repeated lookup, failed exact-key
seek, page and end-probe counting — plus the existing 100,000-visited-row
ceiling and cooperative SQL deadline. Reaching a bound without completion
denies; overflow never truncates or splits one outcome. All smaller worker,
content, scratch, command, store, index and physical limits apply
independently.

## Owner choices and bounded next steps

Accept or revise these proposed choices for **written discovery-closure
work only**:

1. The complete discovery inventory above — one owner per family, both
   lookup directions mandatory, with the completeness rules (zero-reference
   owners, mandatory empty families, retired/tombstoned rows, historical
   inputs; no current-pointer-only scans; unknown consumers deny).
2. The invalidation-ownership table assigning each invalidating event's
   affected consumers, with immutable history never rewritten and
   whole-transition denial when any consumer is undiscoverable.
3. The shared finite bounds applied unchanged across the full catalog
   (4,096 observations/16 MiB, 100,000-visit ceiling, no truncation).

After scoped acceptance, the next bounded written task is audit item 3 (the
actual initializer's complete creation manifest with original proof links)
under this discovery/invalidation ownership; no offline component packet is
authorized by this document — the ownership assignment is the deliverable,
validated like the prior contracts through docs checks and hosted review.

## Review and verification handoff

One writer: the owning session. One bounded round with two fresh read-only
reviewers: enumeration completeness against the merged contracts and
invalidation-ownership correctness. Reviewer cap two; no reviewer edits,
fixtures, runtime, native or model probes. Verify docs metadata/index,
build, rendered links, prior-artifact byte preservation and diff/doc/worktree
close-out. Publication/merge does not accept the five choices above.

## Remaining gates

Physical catalog/version selection (audit item 5), the actual initializer
(item 3), command/proof closure (item 4), the execution launch chain,
execution recovery revisions, profile qualification and separately approved
executable plans remain open. #1689/#1089/#610 stay open; #740 stays closed
research. Gate 2 remains **Investigate**.
