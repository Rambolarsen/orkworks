---
type: Design Proposal
title: Closed research catalog enumeration and owner selection
description: Proposed closed logical catalog for the retained-file research profile — every object kind, metadata family, resource variant and mandatory empty family under one recognized profile, with original source ownership selected at admission.
tags: [orkworks, research, catalog, ownership, closure, qualification]
status: draft
workflow_status: proposed-for-scoped-catalog-closure-review
---

# Closed research catalog enumeration and owner selection

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738); consumer
[#610](https://github.com/Rambolarsen/orkworks/issues/610).
Inspection base: `39a5fcfc41de` (origin/main, including the terminal-completion
packet merged in [#1978](https://github.com/Rambolarsen/orkworks/pull/1978)).

**Proposed for scoped owner review.** The
[source-coverage audit](../../validation/research-source-coverage-audit.md)
names closed catalog and owner selection as catalog-closure item 1. This
proposal closes that written boundary for the selected retained-file research
profile only: the enumerated logical catalog under one recognized profile
(every object kind, metadata family, resource variant and mandatory empty
family), and the original source-ownership selection per RunKey and
AssignmentIdentity at admission. It makes the enumeration and selection rules
explicit; they are not acceptance inferred from a merge. No storage format,
table/column/layout choice, physical row or manifest scheme, migration,
runtime/native/model execution, adapter or executable plan is supplied — the
physical catalog/version selection remains the separately tracked closure
item 5. Gate 2 remains **Investigate**.

## Outcome and preserved decisions

Make the research profile's logical catalog closed and enumerable: a fresh
workspace admission under this profile recognizes exactly the object kinds,
metadata families and resources enumerated below — including the mandatory
empty families — and each RunKey and ChildAssignmentIdentity binds one
original source owner at admission, selected by profile rules rather than by
row presence, version maxima or copied child/terminal sources.

Preserve unchanged: every accepted family's exact key, value, digest domain,
bounds and accounting from the merged contracts and their packets
([source codecs](2026-11-12-research-source-codecs-design.md),
[input/control prefix](2026-11-12-research-input-control-prefix-design.md),
[membership/publication](2026-11-12-research-membership-publication-design.md),
[initialization/catalog](2026-11-13-research-initialization-catalog-design.md),
[authorization](2026-11-13-research-preparation-authorization-design.md),
[continuation](2026-11-15-research-continuation-design.md),
[completion](2026-11-15-research-completion-design.md),
[recovery completion](2026-11-15-research-recovery-completion-design.md),
[scope rejection](2026-11-16-research-scope-rejection-design.md),
[prospective membership](2026-11-16-research-prospective-membership-design.md),
[post-rejection](2026-11-16-research-post-rejection-report-design.md),
[stage report](2026-11-16-research-stage-report-design.md),
[review tasks](2026-11-18-research-review-tasks-design.md),
[clarification](2026-11-18-research-clarification-design.md),
[execution proposal](2026-11-19-research-execution-proposal-design.md),
[terminal completion](2026-11-19-research-execution-completion-design.md));
the child-source families under their own accepted contract
([child source](2026-10-31-child-review-source-preparation-design.md)); the
report registry
([report publication profile](2026-10-29-report-publication-profile-design.md));
#744/#745/#746 ordinary evaluation, learning and projection families (outside
this catalog's research profile, unchanged); and the ordinary session storage
contract (activation gates unchanged).

Rejected alternatives: electing a source owner by table/row presence, by
largest stored version, or by a copied child/terminal source; a partial
catalog where readers support only the nonempty families; per-family catalogs
without one recognized profile; and inventing new ownership identity fields.

## Source trace and obligations

| Source | Established constraint |
| --- | --- |
| [Audit catalog items](../../validation/research-source-coverage-audit.md#catalog-indexes-initialization-and-proofs-still-required) | Closed catalog and owner selection precede discovery/invalidation, the actual initializer, command/proof closure and physical qualification; mandatory empty families are part of the enumeration |
| [Conditional candidate 63](2026-11-19-research-execution-proposal-design.md#families-accounting-and-membership) | 61→63 accepted additions; each packet's arithmetic is reconciled below |
| [Source registry](2026-11-12-research-source-codecs-design.md#proposed-source-registry-accounting-and-ownership) | Six research source object kinds and four source metadata families with their exact keys |
| [Control families](2026-11-12-research-input-control-prefix-design.md#proposed-logical-families-accounting-and-recovery-boundary) | Nine control families including the sole run owner and positive collection ownership |
| [Continuation families](2026-11-15-research-continuation-design.md#logical-families-projection-and-catalog-consequences) | Three immutable continuation families |
| [Completion families](2026-11-15-research-completion-design.md#logical-catalog-initialization-projections-and-resources) | Lifecycle selector, summaries and lifecycle records; original root-admission selector creation |
| [Review additions](2026-11-18-research-review-tasks-design.md#review-request-admission-and-family) | One review-input object kind and the research_reviews family |
| [Clarification additions](2026-11-18-research-clarification-design.md#clarification-decision-and-request-records) | Two clarification families with the closed pending binding |
| [Execution additions](2026-11-19-research-execution-proposal-design.md#families-accounting-and-membership) | One execution-plan object kind and the execution approval family |
| [Preparation authorization](2026-11-13-research-preparation-authorization-design.md#one-retained-research-authorization-and-original-command) | research_preparation_authorizations keyed by the full canonical frozen identity P |
| [Child source catalog](2026-10-31-child-review-source-preparation-design.md#closed-format-4-source-registry-and-commitment) | Child-side families live under their own accepted catalog; whole-catalog reader obligation stated there is preserved, not duplicated here |
| [Report registry](2026-10-29-report-publication-profile-design.md#report-head-and-receipt-registry) | Compact report_heads/report_receipts keys are part of the recognized profile |
| [Owner selection](../../validation/research-source-coverage-audit.md#catalog-indexes-initialization-and-proofs-still-required) | Row presence, largest versions and copied child/terminal sources cannot elect an owner |

## Selected profile and preserved boundaries

One recognized profile: `retained-file-research-v1`. A fresh workspace
admission either initializes this profile's complete catalog or reports
unavailable; partial initialization is unavailable, never repaired at first
read. Ordinary/direct-execution and child-source profiles keep their own
accepted catalogs; a workspace may hold several profiles only through their
own admitted catalogs, never by mixing row families.

This document enumerates and selects owners; it selects no storage format,
physical name, column, schema version or manifest encoding. The eventual
physical catalog/version decision (audit item 5) consumes this enumeration
but remains separately gated. Nothing here authorizes runtime execution,
native/model probes, adapters, migrations or activation.

## Enumerated logical catalog

The research profile's logical catalog enumerates **every name the research
contracts added** — 8 object kinds and 24 metadata families below — plus the
shared report registry. The accepted candidate arithmetic is preserved as the
per-packet reconciliation (50 → authorization +1 = 51 → continuation +3 = 54
→ completion +3 = 57 → review +2 = 59 → clarification +2 = 61 → execution
+2 = 63; terminal completion adds rows, not names), and this enumeration is
its categorical inventory. Every name below is mandatory: it exists in the
initialized catalog even when empty.

### Object kinds (8)

| # | Kind / identity | Owning contract | Notes |
| --- | --- | --- | --- |
| 1 | research-source-capture / ResearchCaptureRef | source codecs | Retained file capture |
| 2 | research-source-scope / ResearchScopeRef | source codecs | Immutable task scope |
| 3 | research-source-configuration / ConfigurationRef | source codecs | Complete v3 configuration |
| 4 | research-source-plan / PlanRef | source codecs | Research source plan |
| 5 | research-source-evidence / {task,referenceId,contentDigest} | source codecs | Capability evidence bytes |
| 6 | research-assignment-source / ChildAssignmentIdentity | source codecs | Frozen assignment source |
| 7 | research-review-input / {task,inputDigest} | review tasks | Reviewer snapshot |
| 8 | research-execution-plan / PlanRef | execution proposal | Execution source plan |
| 9 | objects envelope | membership contract | Shared value envelope (not a name of its own; the eight kinds above use it) |

The eight research object kinds are enumerated; the shared objects envelope is
infrastructure, not a ninth catalog name.

### Metadata families (24)

| # | Family / exact key | Owning contract | Mutable? |
| --- | --- | --- | --- |
| 1 | research_plan_approvals / {plan,approvalId} | source codecs | Immutable |
| 2 | research_reservations / TaskKey | source codecs | Immutable |
| 3 | research_allocations / ReservationRef | source codecs | Immutable |
| 4 | research_attachments / childSessionId | source codecs | Immutable |
| 5 | research_control_inputs / PreparedInputRef **without inputDigest** | input/control prefix | Immutable |
| 6 | research_runtime_attachments / RuntimeKey | input/control prefix | Immutable |
| 7 | research_run_controls / RunKey | input/control prefix | Mutable (sole runVersion owner) |
| 8 | research_plan_controls / PlanRef | input/control prefix | Mutable |
| 9 | research_task_controls / TaskKey | input/control prefix | Mutable |
| 10 | research_launch_controls / TaskKey | input/control prefix | Mutable |
| 11 | research_session_states / TaskKey | input/control prefix | Mutable |
| 12 | research_collections / RunKey | input/control prefix | Mutable |
| 13 | research_expectations / TaskKey | input/control prefix | Mutable |
| 14 | research_turn_completions / TaskKey | continuation | Immutable |
| 15 | research_coordination_receipts / TaskKey | continuation | Immutable |
| 16 | research_recovery_decisions / {task,recoveryId} | continuation | Immutable |
| 17 | research_lifecycle_state / RunKey | completion | Mutable (selector) |
| 18 | research_summaries / ResearchSummaryKey | completion | Immutable |
| 19 | research_lifecycle_records / ResearchLifecycleKey | completion | Immutable |
| 20 | research_reviews / reviewer TaskKey | review tasks | Immutable |
| 21 | research_clarification_requests / {run,questionId,questionVersion} | clarification | Immutable |
| 22 | research_clarification_answers / {run,questionId,questionVersion,answerId} | clarification | Immutable |
| 23 | research_execution_plan_approvals / {plan,approvalId} | execution proposal | Immutable |
| 24 | research_preparation_authorizations / full canonical P | preparation authorization | Immutable |

### Shared report registry (2)

| # | Family / exact key | Owning contract |
| --- | --- | --- |
| 1 | report_heads / exact Stream | report publication profile |
| 2 | report_receipts / {stream,reportVersion} | report publication profile |

### Resources (existing names unchanged)

The fourteen workspace, nine run and five assignment resource names keep
their established meanings and creation rules from the accepted
[resource recipe](2026-11-13-research-initialization-catalog-design.md#exact-fresh-owner-resource-recipe-proposed)
and the [child source pools](2026-10-31-child-review-source-preparation-design.md#atomic-source-preparation-and-finite-accounting):
`child-source-retained-bytes` and `child-control-retained-bytes` (run +
workspace pools, reserved zero), the run/workspace preparation and report
resources, and the five per-assignment pools initialized at attachment. No
new resource name is introduced; the enumeration confirms their placement
under this profile.

### Mandatory empty families

Every family above is created at profile initialization, including those
legally empty on common paths: `research_clarification_requests`/
`research_clarification_answers` (no clarification asked),
`research_reviews` and `research-review-input` (no reviewer declared),
`research_recovery_decisions` (no failed attempt), `research_turn_completions`
/`research_coordination_receipts` (no child launched yet),
`research_summaries` (no synthesis yet), `research_lifecycle_records` (no
completion/finish decision yet), and the execution families (no execution
proposal). A reader that cannot support a whole family reports unavailable;
absence of rows is never absence of the family.

## Original source ownership selection

### Per RunKey

At fresh root admission under this profile, the admission outcome selects one
original source owner for the RunKey: the recognized profile's
`research_run_controls` row is created alongside the initial root input and
the lifecycle selector (per the accepted
[selector admission](2026-11-15-research-completion-design.md#logical-catalog-initialization-projections-and-resources)),
and `run_states` remains the historical seed only. The owner is selected by
the admission rule (profile + fresh admission), never by row presence, largest
versions or copied sources. A RunKey cannot be re-admitted under a second
profile: re-admission under changed bootstrap bytes is a new run identity
under the accepted bootstrap-drift rules.

### Per ChildAssignmentIdentity

At the unique attachment outcome, the ordinary-session lineage producer
publishes the frozen ChildAssignmentIdentity with exactly one original
assignment source under `research-assignment-source` (research tasks) — or,
for the child-source profile, its accepted child-assignment-source analog —
selected by the attachment rule, never by matching IDs, current role or
whichever family has a row. Conflicting ownership (two assignment sources for
one identity) denies. The frozen identity derives from the original
reservation snapshot, complete configuration and the unique attachment, per
the accepted [admission-version rules](2026-10-04-orchestrator-preparation-design.md#proposed-evaluation-admission-version-snapshot).

### Shared registry ownership

`report_heads`/`report_receipts` are owned by the report publication profile
per exact Stream, resolved through the original reservation/configuration —
never through a generic head or maximum version.

### Discovery obligations preserved

This enumeration is the first input to audit item 2 (complete discovery and
invalidation): every family above participates in complete owner-family
membership in both lookup directions, including zero-reference owners,
retired/tombstoned rows and mandatory empty families. The accepted recipes
and bounds are unchanged.

## Reconciliation of the candidate arithmetic

The accepted packet chain derives the conditional candidate as follows:
research authorization 51 → continuation +3 = 54 → completion +3 = 57 →
scope-rejection +3 = 57 (reconciled) → review tasks +1 kind +1 family = 59 →
clarification +2 = 61 → execution proposal +1 kind +1 family = 63 → terminal
completion +0 = 63. The enumeration above counts the same names by category
(8 research object kinds + 23 research metadata families + 2 report registry
+ the child-side and ordinary names carried under their own accepted
catalogs), and the accepted 63 remains the profile's conditional candidate
total. This enumeration is neither a final physical catalog nor a maximum;
audit item 5 still selects the physical format.

## Accounting and compatibility

No source digest, wire field, dependency purpose, version default, cap,
receipt version or credential serialization changes. All accepted pool
placements are confirmed unchanged: `child-source-retained-bytes` for the
source objects, source plans, review inputs, execution plans and their
approvals (the execution contract charges both execution families to the
source pools; only run-control binding changes use the control pools);
`child-control-retained-bytes` for control/lifecycle/continuation/review/
clarification families; report pools for the registry. The 32 MiB run
aggregate and every smaller limit bind independently. Old stores are never imported or extended on first
read; the document can be reverted without store effects.

## Owner choices and bounded next steps

Accept or revise these proposed choices for **written catalog closure only**:

1. The enumerated 31 research-side logical names (8 object kinds, 23 metadata
   families) plus the shared report registry under the one recognized
   `retained-file-research-v1` profile, reconciling the accepted candidate 63.
2. Mandatory empty families and the whole-catalog reader obligation.
3. Original source ownership per RunKey selected at fresh root admission by
   the profile rule, and per ChildAssignmentIdentity at the unique attachment
   outcome — never by row presence, version maxima or copied sources;
   conflicting ownership denies.
4. Shared registry ownership per exact Stream under the report publication
   profile.
5. No new resource names, no format/table selection, and audit items 2–5
   (discovery/invalidation, actual initializer, command/proof closure,
   physical qualification) remain separately gated.

After scoped acceptance, the next bounded written tasks are audit item 2
(complete discovery and invalidation ownership) and item 3 (the actual
initializer's complete creation manifest) — each its own contract with its
own review; no offline component packet is authorized by this document
because the enumeration itself is the deliverable.

## Review and verification handoff

One writer: the owning session. One bounded round with two fresh read-only
reviewers: enumeration completeness/correctness against the merged contracts
and owner-selection correctness. Reviewer cap two; no reviewer edits,
fixtures, runtime, native or model probes. Verify docs metadata/index,
build, rendered links, prior-artifact byte preservation and diff/doc/worktree
close-out. Publication/merge does not accept the five choices above.

## Remaining gates

Physical catalog/version selection (audit item 5), complete
discovery/invalidation (item 2), the actual initializer (item 3),
command/proof closure (item 4), the execution launch chain, execution
recovery revisions, profile qualification under #741/#740's closed-research
standard and separately approved executable plans remain open.
#1689/#1089/#610 stay open; #740 stays closed research. Gate 2 remains
**Investigate**.
