---
type: Design Proposal
title: Research profile initializer creation manifest
description: Proposed complete fresh-profile initializer manifest for the retained-file research profile — every required owner, selector, empty family and resource row created in one original outcome with original proof links, under the accepted catalog enumeration and discovery ownership.
tags: [orkworks, research, initializer, catalog, qualification]
status: draft
workflow_status: proposed-for-scoped-initializer-review
---

# Research profile initializer creation manifest

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738); consumer
[#610](https://github.com/Rambolarsen/orkworks/issues/610).
Inspection base: `aa3f256fa4a5` (origin/main, including the discovery-contract
acceptance merged in [#2018](https://github.com/Rambolarsen/orkworks/pull/2018)).

**Proposed for scoped owner review.** The
[source-coverage audit](../../validation/research-source-coverage-audit.md)
names the actual initializer as catalog-closure item 3. This proposal closes
that written boundary over the accepted
[catalog enumeration](2026-11-23-research-catalog-closure-design.md#enumerated-logical-catalog)
and [discovery ownership](2026-11-23-research-discovery-invalidation-design.md#complete-discovery-inventory):
the complete fresh-profile creation manifest — every required owner row,
selector, mandatory empty family and resource row created in one original
outcome, with original proof links, peers preserved, exact occurrence
accounting, and no missing-ledger repair. No storage format, table/layout,
migration, runtime/native/model execution, adapter or executable plan is
supplied. Gate 2 remains **Investigate**.

## Outcome and preserved decisions

Make the profile initializer's manifest enumerable: a fresh workspace
admission under `retained-file-research-v1` creates, in one proven atomic
outcome, the complete reviewed catalog — every owner, selector, empty family
and resource row the enumeration requires — with a header advance and one new
complete proof, peers preserved, and exact occurrence accounting. The
initializer is the only writer that creates required rows; later commands
never repair, backfill or deduplicate them.

Preserve unchanged: every accepted family's exact key/value/digest domain,
bounds and pool placement from the merged contracts; the accepted fresh
resource recipe and admission counts
([initialization/catalog](2026-11-13-research-initialization-catalog-design.md#exact-fresh-owner-resource-recipe-proposed),
[completion selector admission](2026-11-15-research-completion-design.md#logical-catalog-initialization-projections-and-resources));
the child-source pools' creation rules
([child source](2026-10-31-child-review-source-preparation-design.md#atomic-source-preparation-and-finite-accounting));
and the ordinary preparation families under their own contracts. No physical
format/table/layout/manifest encoding is selected — audit item 5 remains
separately gated.

Rejected alternatives: creating required rows at first read or first use;
fabricating zero ledgers from a source constructor; backfilling missing
ledger peers; a partial initializer whose manifest omits mandatory empty
families; and per-family opportunistic initialization.

## Source trace and obligations

| Source | Established constraint |
| --- | --- |
| [Audit item 3](../../validation/research-source-coverage-audit.md#catalog-indexes-initialization-and-proofs-still-required) | Create every required owner, selector, empty family/index and ledger in its original outcome; preserve peers and exact occurrence accounting; no missing-ledger repair, deduplication discount or counter reset |
| [Accepted enumeration](2026-11-23-research-catalog-closure-design.md#enumerated-logical-catalog) | 8 object kinds, 24 metadata families, report registry; mandatory empty families; whole-catalog reader obligation |
| [Discovery ownership (accepted)](2026-11-23-research-discovery-invalidation-design.md#complete-discovery-inventory) | One discovery owner per family; both directions mandatory; complete inventories include zero-reference owners |
| [Fresh resource recipe](2026-11-13-research-initialization-catalog-design.md#exact-fresh-owner-resource-recipe-proposed) | Shared control/source/report/preparation pools already exist at admission; explicit creation union and initial run owner at root admission |
| [Selector admission](2026-11-15-research-completion-design.md#logical-catalog-initialization-projections-and-resources) | Original root admission creates the lifecycle selector (summaryVersion 0, four null pointers) alongside the initial run owner; initialized 27→28 / never-initialized 24→25 conditional rows |
| [Owner selection (accepted)](2026-11-23-research-catalog-closure-design.md#original-source-ownership-selection) | RunKey owner selected by the admission rule; AssignmentIdentity owner at the unique attachment |
| [Recovery boundary](2026-11-12-research-membership-publication-design.md#accounting-recovery-and-compatibility) | Fresh research creation must initialize its complete reviewed catalog and full resource inventory with original proof links; the zero-report-ledger packet is not that initializer |

## The complete creation manifest

The initializer's manifest splits across the accepted creation boundaries —
**three distinct original outcomes**, each with its own header advance and
new complete proof, exactly as the accepted initialization and child-source
contracts separate them. No outcome defers required rows to later commands;
every created row carries its accepted exact key/value/wrapper shape and its
original `lastCommit` proof bound to its own outcome's header advance.

### Outcome 1 — workspace initialize-store (created once per workspace profile admission)

| Created row | Owner contract | Mandatory empty at creation |
| --- | --- | --- |
| workspace resource ledgers (fourteen names) | initialization/catalog recipe | All at zero, reserved zero, original proof |
| workspace `child-source-retained-bytes` / `child-control-retained-bytes` pools | child source contract | Zero |
| workspace report-retained pools | report publication profile | Zero |
| (all 24 metadata families + report registry rows exist per-run; none is created at workspace scope — the families are run-keyed or task-keyed; the workspace scope carries only resources and the profile registration) | catalog enumeration | — |

### Outcome 2 — root admission (created at fresh root admission, one RunKey)

| Created row | Owner contract | Initial value |
| --- | --- | --- |
| `research_run_controls` / RunKey | input/control prefix | Initial run owner, runVersion 1, `assessing`, currentInput = root input-1 ref, no plans/grants |
| `research_lifecycle_state` / RunKey | completion | Selector: summaryVersion 0, four null pointers, disposition null |
| `research_collections` / RunKey | input/control prefix | Created at proposal time — NOT at admission (empty at admission) |
| `research_clarification_requests` / run-scoped keys | clarification | Empty family (created at profile admission; rows appear on events) |
| `research_clarification_answers` / run-scoped keys | clarification | Empty family |
| `research_turn_completions` / TaskKey keys | continuation | Empty family |
| `research_coordination_receipts` / TaskKey keys | continuation | Empty family |
| `research_recovery_decisions` / {task,recoveryId} keys | continuation | Empty family |
| `research_summaries` / ResearchSummaryKey keys | completion | Empty family |
| `research_lifecycle_records` / lifecycle keys | completion | Empty family |
| `research_reviews` / TaskKey keys | review tasks | Empty family |
| `research_execution_plan_approvals` / {plan,approvalId} keys | execution proposal | Empty family |

| run resource ledgers (nine names) | initialization/catalog recipe | Per the accepted recipe: the preparation ledger is **replaced with its charged value** (the root preparation is not free), the control ledger is **nonzero** covering the initial run owner and lifecycle selector charges, and the remaining ledgers are created at zero, reserved zero, each with original proof |
| run `child-source-retained-bytes` / `child-control-retained-bytes` pools | child source contract | Zero |
| run report-retained pools | report publication profile | Zero |
| `run_states` historical seed | ordinary storage | Accepted historical seed, unchanged |

**Complete family registration.** The manifest registers **every** name in
the accepted enumeration, split by creation timing:

- **Created rows (with values):** `research_run_controls` (run owner),
  `research_lifecycle_state` (selector), the nine run resource ledgers and
  the control/source pools, the workspace resource ledgers and pools,
  `report_heads`/`report_receipts` as registered empty families.
- **Registered empty families (identity-keyed; rows appear only through
  owning commands):** all 24 metadata families not created with values —
  `research_plan_approvals`, `research_reservations`, `research_allocations`,
  `research_attachments`, `research_control_inputs`,
  `research_runtime_attachments`, `research_plan_controls`,
  `research_task_controls`, `research_launch_controls`,
  `research_session_states`, `research_collections`,
  `research_expectations`, `research_turn_completions`,
  `research_coordination_receipts`, `research_recovery_decisions`,
  `research_summaries`, `research_lifecycle_records`, `research_reviews`,
  `research_clarification_requests`, `research_clarification_answers`,
  `research_execution_plan_approvals`,
  `research_preparation_authorizations` — and all 8 object kinds
  (`research-source-capture`, `research-source-scope`,
  `research-source-configuration`, `research-source-plan`,
  `research-source-evidence`, `research-assignment-source`,
  `research-review-input`, `research-execution-plan`), which gain rows only
  through their owning commands. `run_states` is the accepted historical
  seed, not a registered research family.

A reader that cannot address a registered empty family reports unavailable;
absence of rows is never absence of the family. A manifest omitting any
enumerated name — family or object kind — is incomplete and denies.

**Root subject rows at admission.** The accepted fresh-owner resource recipe
makes the root itself an assignment owner: root admission therefore also
creates the root's five assignment resource rows (all zero, reserved zero,
original proof) and the root's initial `summary_state` (subjectRevision 1,
accepted branches). A later child attachment creates **its own** assignment
rows under the child's identity — it can never inherit or reuse the root's.

### Outcome 3 — attachment preparation (created at the unique attachment, per ChildAssignmentIdentity)

| Created row | Owner contract | Initial value |
| --- | --- | --- |
| five fresh assignment resource rows (reporting-receipt-bytes, assignment-record-bytes, result-payload-bytes, evaluation-artifact-bytes, evaluation-artifact-read-bytes) | child source contract | All zero, reserved zero |
| initial `summary_state` (subjectRevision 1, Unassessed row/checkpoint or never-initialized marker) | child source contract | Accepted branches |
| `research-assignment-source` object + provenance membership | research source codecs | Full retained bytes |

### Profile registration and coexistence

The initializer binds the workspace to profile `retained-file-research-v1`
**additively**: a workspace already holding an ordinary/direct-execution or
child-source profile keeps those profiles' admitted catalogs byte-identical
(peers preserved), and the research catalog is admitted alongside them under
the accepted owner-selection rule (per RunKey at admission, per
AssignmentIdentity at attachment). Coexistence is explicitly permitted;
mixing **row families** across profiles is unavailable — no research row is
created inside another profile's family or vice versa. The registration
never rewires, re-initializes or retokenizes another profile's rows.

## Manifest completeness and proof rules

- The complete changed-row set is the manifest: every created row, every
  pool delta (zero→zero created rows are real effects), the profile
  registration, the header advance and one new complete proof outcome —
  committed atomically. No post-commit registration or partial success.
- Original proof links bind each created row to the initializer's own C;
  peers (existing rows from earlier profiles in the same workspace) are
  preserved byte-identical and never re-initialized.
- The manifest is derivable from the accepted enumeration + discovery
  ownership: every family the discovery inventory names appears either as a
  created row (owner/selector/resource rows) or as a registered empty family
  (identity-keyed families). A manifest that omits any enumerated name is
  incomplete and denies.
- No missing-ledger repair, deduplication discount, counter reset, first-read
  backfill, or zero-ledger fabrication by a source constructor. The
  initializer is the only admitted creator of required rows.
- All accepted bounds apply: the shared child pools, the 32 MiB run
  aggregate, and every smaller limit. Creation rows are real effects with
  original-to-final deltas coalesced once per ledger.

## What stays gated

No storage format/table/layout/manifest encoding is selected — audit item 5
consumes this manifest. The command/proof closure (item 4: the full
source/consumer/resource/index/header effect union, captured H, coalesced
versions and original proofs per command) remains separately gated, as do
the execution launch chain, profile qualification and executable plans. No
offline component packet is authorized by this document — the manifest
enumeration is the deliverable, validated through docs checks and hosted
review.

## Owner choices and bounded next steps

Accept or revise these proposed choices for **written initializer-closure
work only**:

1. The complete creation manifest above — workspace resources, run owner +
   selector, registered empty identity-keyed families, assignment-scope
   fresh rows — as one atomic initializer outcome, the sole admitted creator
   of required rows.
2. Empty-family registration (recognized at admission; rows appear only
   through owning commands; readers unable to address an empty family report
   unavailable).
3. Original proof links on every created row bound to the initializer's own
   header advance, peers preserved, no repair/backfill/deduplication.
4. Profile registration binding the workspace to
   `retained-file-research-v1` under the accepted owner-selection rule.
5. Audit item 4 (command/proof closure) and item 5 (physical
   catalog/version selection) remain separately gated; no component packet
   follows from this contract.

## Review and verification handoff

One writer: the owning session. One bounded round with two fresh read-only
reviewers: manifest completeness against the accepted enumeration/discovery
ownership and proof/accounting correctness. Reviewer cap two; no reviewer
edits, fixtures, runtime, native or model probes. Verify docs metadata/index,
build, rendered links, prior-artifact byte preservation and diff/doc/worktree
close-out. Publication/merge does not accept the five choices above.

## Remaining gates

Physical catalog/version selection (audit item 5), command/proof closure
(item 4), the execution launch chain, execution recovery revisions, profile
qualification and separately approved executable plans remain open.
#1689/#1089/#610 stay open; #740 stays closed research. Gate 2 remains
**Investigate**.
