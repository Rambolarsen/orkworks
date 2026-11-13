---
type: Design Proposal
title: Fresh research initialization and catalog boundary
description: Proposed complete fresh-owner resource recipe and initial research run-control placement, with a source-traced candidate catalog and explicit research authorization and format blockers.
tags: [orkworks, research, initialization, catalog, accounting, qualification]
status: draft
workflow_status: proposed-for-scoped-initialization-review
---

# Fresh research initialization and catalog boundary

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `2c06a108916e555457a9ebc5e608fbfbc482fbb7`.

**Written proposal; new initialization choices await scoped owner review.**
The research source, input/control and membership component acceptances do not
accept a fresh-store initializer. This document makes the required resource
creation and initial current-owner placement concrete. It also records why the
candidate family inventory is not yet a complete storage format. Gate 2 remains
Investigate. No dependent fixture, runtime/native execution, model probe,
database, migration, adapter or executable product plan accompanies it.

## Result and bounded next step

Propose one fresh-owner recipe: fourteen workspace resource rows at new-store
creation; nine resource names at run scope after root admission; five existing
assignment resource rows for a freshly prepared subject. Create the sole
research_run_controls row in the original root admission, not on a later first
report or first control access. All named resources already exist in accepted
contracts. The new choice is their complete creation boundary/union for research,
not a new pool, source digest, report allowance or authority capability.

Do **not** select a research format number/hash domain from a union of table
names. The additive candidate below has fifty names, but research original
preparation authority/ordinary lineage and several positive transition codecs
remain unclosed. Those owners must resolve the complete catalog first. Reserving
a format now would make either an incomplete format or another later change
look unavoidable. Neither an old format nor a guessed next version is eligible.

The bounded sequence is: review these two initializer choices; if accepted,
permit only offline key/value/charge components without storage commitments;
then close research original-preparation/paired evidence and the complete
catalog/version decision before full-state/history/proof fixtures. No runtime
implementation plan is part of that sequence.

## Source-backed reconciliation

| Source | Established obligation / scope | Research consequence |
| --- | --- | --- |
| [Initial root effects](2026-10-28-root-preparation-effects-design.md#resource-keys-and-zero-initialization) | Nine workspace zero rows; three assignment and three run reporting/record/payload rows at root admission; fresh draft preparation-byte ledger | Initial-only witnesses omit artifact, source, control and report resources required by a research-capable run |
| [Fresh report-ledger extension](2026-11-11-report-retention-initialization-design.md#exact-resource-and-provenance-mapping) | Adds workspace report zero at initialize-store and run report zero at original root admission; no draft-lifetime report ledger | Preserve those boundaries and original-proof requirements; its format-3 ten-ledger witness is not a research initializer |
| [Fresh child source preparation](2026-10-31-child-review-source-preparation-design.md#exact-resource-keys-and-owner-mapping) | Five assignment resource names; their ten run/workspace peers already exist. Source pool exists before proposal | Root preparation for the new profile must create artifact peers explicitly; child preparation cannot repair them |
| [Selected controls](2026-11-02-child-launch-control-design.md) | Control resource exists in both scopes before the first control row; exact original-to-final charges | Initial research current-control insertion requires its run/workspace control charges atomically |
| [Research source registry](2026-11-12-research-source-codecs-design.md#proposed-source-registry-accounting-and-ownership) and [control registry](2026-11-12-research-input-control-prefix-design.md#proposed-logical-families-accounting-and-recovery-boundary) | Research uses the existing shared source/control pools; one research current-run owner, original run_states historical | No extra research allowance or second runVersion, and no current-owner initialization by a report writer |
| [Thirty-five-family reconciliation](../../validation/child-preparation-catalog-reconciliation.md#full-logical-inventory-and-original-attach-applicability) | Named selected child/ordinary families; physical/producer qualification remains open | A name count is not an admitted research catalog |
| [Original preparation authorization](2026-11-05-child-preparation-authorization-design.md#outcome-and-scope) | Original direct-execution-only attach; child source object and child control snapshots | Cannot cast the research source/runtime/control examples into an authorized child-source-attach |

The [production Connection::open](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs)
still refuses an unqualified profile before using its path. The
[module fence](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/mod.rs)
keeps generic mechanics cfg(test). These are bounded source observations, not an
exhaustive writer inventory or installation/native result. No database was opened.

## Candidate logical inventory: fifty names, not a closed format

An additive union preserves all thirty-five inherited family names and adds the
four research source metadata, nine research input/control metadata and two
compact report families. The six research object kinds remain variants of
objects, not six additional tables. Existing child and research namespaces are
distinct. These counts describe the candidate to reconcile, not a decision that
this exact union is sufficient or that every inherited variant is research-valid.

| Group | Names | Count / exact key owners |
| --- | --- | --- |
| Existing core/root | store_header, objects, payloads, heads, dependencies, summary_state, pending_material, receipt_slots, candidate_work, quota_usage, commit_proof, claim_proofs, mutation_lookup, root_drafts, root_preparations, run_states, parent_attachments, preparation_artifacts | 18; [inherited full inventory](../../validation/child-preparation-catalog-reconciliation.md#full-logical-inventory-and-original-attach-applicability) and its owning closed codec links |
| Existing child source metadata | child_plan_approvals, child_reservations, child_allocations, child_attachments | 4; original PlanRef/approvalId, TaskKey, ReservationRef and childSessionId |
| Existing child controls | child_control_inputs, child_runtime_attachments, child_run_controls, child_plan_controls, child_task_controls, child_launch_controls, child_session_states, child_coordination_receipts, child_recovery_decisions | 9; [accepted exact keys/wrappers](2026-11-02-child-launch-control-design.md) |
| Existing artifact ownership | evaluation_artifact_streams, artifact_read_debits | 2; original producer/declaration stream and full DebitIdentity |
| Existing paired preparation evidence | ordinary_child_lineage, child_preparation_authorizations | 2; full ChildAssignmentIdentity; original lineage/authorization codecs stay scoped to their accepted path |
| Research source metadata | research_plan_approvals, research_reservations, research_allocations, research_attachments | 4; [accepted research source keys](2026-11-12-research-source-codecs-design.md#proposed-source-registry-accounting-and-ownership) |
| Research controls/collection | research_control_inputs, research_runtime_attachments, research_run_controls, research_plan_controls, research_task_controls, research_launch_controls, research_session_states, research_collections, research_expectations | 9; [accepted research control keys](2026-11-12-research-input-control-prefix-design.md#proposed-logical-families-accounting-and-recovery-boundary) |
| Compact report registry | report_heads, report_receipts | 2; exact Stream and stream/reportVersion under the [unchanged compact codecs](2026-10-29-report-publication-profile-design.md#report-head-and-receipt-registry) |

Derived membership/index structures are not extra logical family or manifest
changes. The [accepted paired index layout](2026-11-05-paired-membership-index-design.md#scoped-physical-design-acceptance--2026-11-05)
and later explicit derived-index exception retain their scope; research lookup
recipes do not automatically extend its format6-only locator/projection unions.
No RegistryRoot acceptance follows from this inventory.
Generic heads and compact report_heads cannot both own the same research stream;
the final catalog must preserve the accepted compact placement, including its
logical HeadKey mapping, rather than treating the union as permission for two heads.

### Concrete blockers to final catalog/version selection

The existing OriginalPreparationIntent binds a child-assignment-source ObjectRef,
four child control snapshots, an executing direct-execution run and an original
parent-source permit. Research uses different object kinds, control families and
a researching run. The existing ControlSnapshotTable/PlanControlRead union does
not name research rows. Similar field names or full AssignmentIdentity equality
cannot broaden that closed representation or its late-saving/no-spawn policy.

Before selecting a final research storage version, the owning design must close:

- Original research preparation admission/authorization and its own exact retained
  decision evidence, source/control snapshot types, ordinary member/lineage joins,
  uniqueness/proof/membership/accounting placement and retry/rollback/disposition.
  Reusing a child permit or introducing a new variant/family requires a concrete
  scoped decision; this document chooses neither.
- Whether the ordinary schema2 lineage member and its source digest resolution
  can remain byte-identical under a reviewed research source-owner mapping, or
  need a separately versioned representation. Neither casts nor a schema bump
  are inferred. [Ordinary storage activation](../../../specs/session-storage.md)
  and paired broker/native evidence remain gated independently.
- The exact fixed catalog, supported closed row/object/reference/resource unions,
  original source-owner selection per RunKey/AssignmentIdentity, physical research
  indexes and recovery rules. No current row's presence chooses a profile/owner.
  All required empty families/indexes must be recognized, not treated optional.
- Research-specific coordination/recovery/finish/cancel continuation and any
  terminal handoff used by that catalog. Existing logical behavior is preserved;
  missing positive codecs cannot be normalized into null/empty variants.

The final catalog may therefore have more than fifty names or a different
explicitly reviewed composition. Do not call fifty a maximum, register a partial
format7, or reuse format-3/4/6 commitments for research metadata. No new format
number, header/manifest schemaVersion, row/change domain or source-proof envelope
is selected here. The existing nested source/request/response/receipt domains
remain unchanged. Complete proof-bearing fixtures must wait for that decision.

## Exact fresh-owner resource recipe proposed

Reuse existing quota_usage key `{owner,resource}` and value
`{used:Counter,reserved:Counter,lastCommit:CommitRef}`. Counters are canonical u64
decimal TEXT, distinct from bounded source JSON versions and signed runSequence.
W is `{kind:"workspace",workspaceId}`; R is
`{kind:"run",workspaceId,runId}`; A is
`{kind:"assignment",assignmentIdentity}`. All identities derive from original
retained admission/source ownership, never a selected UI run or current epoch.

| Existing resource | W at initialize-store | R creation boundary | A at fresh subject preparation |
| --- | --- | --- | --- |
| reporting-receipt-bytes | zero | Root admission, zero | zero |
| assignment-record-bytes | zero | Root admission, zero | zero |
| result-payload-bytes | zero | Root admission, zero | zero |
| evaluation-artifact-bytes | zero | Root admission, zero | zero |
| evaluation-artifact-read-bytes | zero | Root admission, zero | zero |
| summary-row-bytes | zero | No run row | No assignment quota row; actual summary branch has its existing owner |
| summary-row-count | zero | No run row | Same |
| summary-lineage-bytes | zero | No run row | Same |
| summary-pending-count | zero | No run row | Same |
| preparation-retained-bytes | zero | Draft prepare with exact charge; original draft-to-source replacement at admission | No assignment row |
| preparation-run-count | zero | Workspace only; draft adds one retained run, admission does not add another | No assignment row |
| child-source-retained-bytes | zero | Root admission, zero before any research proposal | No assignment row |
| child-control-retained-bytes | zero | Root admission with exact initial research_run_controls charge | No assignment row |
| report-retained-bytes | zero | Root admission, zero | No assignment row |

Thus there are fourteen workspace names, nine run names after root admission
and five assignment names per freshly prepared subject. The root is an assignment
owner too. Root admission creates both artifact assignment rows and both artifact
run rows explicitly; it does not advertise the older three-row root subset as all
five. A later child in the same run inserts only its five new assignment rows;
the five run/workspace peer names already exist. Every zero is a real retained
row and proof-member obligation, not an inferred missing value.

Freshness must be proven from the complete owning inventory at the captured
boundary. Empty input, no head or a newly spelled identity is insufficient.
Initialize-store requires complete selected-profile absence; fresh R/A zero
creation requires absence of their owned report/history, slots/records/payloads,
artifacts/streams/debits and conflicting preparation/source/control ownership.
Retained/orphan/ambiguous ownership denies instead of being repaired by a zero.
Reconcile pre-existing workspace peers from every retained run/epoch without
resetting them. These are logical proof obligations, not a claim that a complete
physical lookup or fresh-store detector is already available.

These are existing resource meanings/caps, not fourteen independent allowances
or new active capabilities. In particular, the presence of zero artifact rows
does not admit the root/researcher as a new artifact producer. Exact role/source/
output/capability rules still decide that. Shared source/control/report balances
include every retained run/epoch; starting another run cannot reset them.

Draft prepare/cancel gains no report, source, control or five-resource subject
row. The only fresh run ledger before admission is its preparation-retained-bytes
row. Explicit cancellation deletes it only under the existing whole-unadmitted-
draft ownership proof; a namespace reservation remains burned. Existing workspace
resources and all unrelated runs/subjects retain their values and direct links.

## Initial current research owner in the original admission

Propose that the new profile's original root subject-preparation also inserts
research_run_controls[RunKey] with schemaVersion 1, the accepted research profile,
runVersion 1, assessing state, exact immutable run/bootstrap/repository/source
bindings, and currentInput pointing to original root input-1. All plan references
are empty; currentPlan/pendingPlan/parentRuntime, clarification/pause/resume fields
are null, blockers empty and stageContinueRequired false, exactly as the accepted
[initial component view](../../validation/research-prefix-components.md#finite-current-views-and-prospective-consumer-ownership).

Keep root run_states as the complete immutable initial source/seed. Its version-1
value is historical, not a second current counter. Do not create child_run_controls
or a terminal mutable run_states variant for this RunKey. The final catalog must
bind that ownership at original admission; a largest-counter or present-table
test is never its discriminator. No new profile field is added to a source/receipt
by this proposal.

Input-1 stays the root's original shape with null decision/empty records and no
invented research producer or inputDigest field. Its current ref derives the
unchanged initial-body input domain. No research_control_inputs, runtime history,
collection/expectation, plan, ordinary child, permit or grant is created here.
First real parent attachment remains a later separately qualified event. It
advances the current owner once; it must not create/reinitialize that owner.

The initial run-control occurrence is charged to R/W child-control-retained-bytes
and the existing run aggregate. The fresh run control ledger is nonzero at C;
the workspace control ledger receives the exact same new occurrence delta once.
The new source run row remains zero; merely validating the workspace source or
artifact/report peer rows does not rewrite their lastCommit. Root source/bootstrap/
initial state/artifact occurrences keep their original preparation charges.

## Logical initializer and admission effects

C below is only the eventual creating command's CommitRef, **not a chosen storage
hash or a successful command**. K is the existing separate header replacement/new
proof pair; both count toward complete command/store size. All changed keys have
one original-before/final-after value; unchanged rows keep their original links.
Counts below reconcile the closed logical recipe under the inherited initial-only
root branch. They do not include invented physical index rows in a logical
manifest or qualify actual bytes, source authority or a whole-store outcome.

| Logical operation in proposed fresh profile | Exact changed logical group | Rows including K |
| --- | --- | ---: |
| initialize-store | Fourteen zero workspace quota rows, no sources/drafts/runs, null predecessor, K | 16 |
| root-draft-prepare | Existing draft, n artifact inserts, run preparation-byte row, two workspace preparation replacements, K; all other new resources remain untouched | n + 6 |
| Exact root-draft-cancel | Existing exact draft/artifact/run-preparation-ledger deletions and two workspace preparation replacements, K; no admitted-source deletion | n + 6 |
| Root admission, initialized summary | Six core changes; thirteen fresh run/assignment resource rows; two preparation-byte replacements; one workspace control replacement; three existing summary-resource replacements; K | 27 |
| Root admission, never-initialized summary | Same except omit the three summary-resource replacements; preserve accepted unavailable marker/null row/checkpoint | 24 |
| Exact authorized root-admission retry | Original immutable receipt/source tuple after current authorization and complete original history checks; no fresh-owner absence test or mutation | 0 |

n is the existing selected capability-artifact reference count, at most sixteen;
each retained occurrence stays separately owned/charged, including equal bytes.

The six core admission changes are exact draft deletion, root_preparations insert,
run_states insert, reserved parent_attachments insert, summary_state insert and
initial research_run_controls insert. Artifact rows remain retained/read-only.
The thirteen new resource rows are 5 A + 5 R + 3 R source/control/report. The draft's
R preparation ledger is a separate existing replacement, not one of these rows.
The original initial-only admission had six fresh subject rows; this proposal
adds seven fresh rows (two A artifacts, two R artifacts and three R source/control/
report), one initial run-control row and one workspace control replacement.
Therefore initialized admission is **27** = original 18 + 7 + 1 + 1;
never-initialized admission is **24** = original 15 + 7 + 1 + 1.
Equivalently: 6 core + 13 fresh resources + 2 preparation replacements + 1
workspace control replacement + 3 summary replacements + K(2) = 27. These
equations, not a historical eighteen-row manifest, govern the proposed new union.

The initialized branch changes summary-row-bytes, summary-row-count and summary-
lineage-bytes only; pending count stays unchanged. The never-initialized marker
charges no nonexistent SummaryRow/material, but all actual marker/wrapper/proof/
control bytes still count. A later read cannot lazily initialize the subject.
No root/control/report counter reset, slot reservation, result, evaluation,
artifact stream, debit, material transition, start receipt or native action follows.

## Accounting and direct creation provenance

Use each existing resource's exact projection and bounds. Research source/control
rows share their existing 8 MiB run / 64 MiB workspace pools; run preparation/report
aggregate stays 32 MiB and workspace report usage stays 256 MiB. Each smaller
source/row/pool/global limit wins. Repeated stored source/snapshot occurrences
stay charged. Control/source bytes do not consume any of the new zero assignment
record/artifact/read-work rows. Creation does not grant read-work credit.

Report ledgers count report object/head/receipt occurrences, not their own quota
rows. Quota rows, all excluded outer links, header/proof, exact physical index
storage and ordinary data remain in their own complete-command/store/physical
budgets. Preserve the accepted complete-command 4,096-row/16 MiB and separate
discovery 4,096-observation/16 MiB limits. The proposed logical counts are no proof
that coupled maximum source/snapshot/index bytes fit those limits.
Existing retained-proof bytes/count and whole logical-store bytes/count remain
derived admission/recovery invariants, not extra quota_usage rows among the
fourteen. No cached total, self-charging proof ledger or new allowance is created.

For every owner/resource key, sum actual inserts/deletes/replacements directly
from original state to final state, then write it once. No delta means no stamp
rewrite. Source/control workspace peers may already contain unrelated runs'
nonzero retained usage. Fresh R/A zeros do not reset those peers. Missing or
ambiguous existing peers deny; publication, child attach and read paths cannot
create a missing historical ledger.

The eventual null-predecessor initialize-store proof must contain all fourteen
workspace rows. Original root receipt/source must identify the admission C whose
complete logical union includes all thirteen new resource rows and initial
research current control. Subsequent mutable rows point to their own updating
proof; their current values need not retain C in lastCommit. Original creation
proofs and required direct read/origin links remain protected independently.

No proof is manufactured by these equations. Storage commitment domains,
exclusions for any new paired/authorization representation and complete original
manifests wait for catalog closure. Historical initial-only, report-ledger,
source/control and membership artifacts retain exact original bytes/status. A
matching zero ledger, source digest, proof-shaped hash or visible ordinary child
cannot stand in for the original admitted recipe.

## Compatibility, recovery and bounded owner decision

This is new-store/new-run-only design. It creates no activation/migration route
for format-3/4/6 stores, partial mechanics, old ordinary schema pairs, lost/missing
peers or legacy source histories. No startup backfill, header repair, role-based
source cast or alternate catalog selected from present tables is allowed. A
reader for the eventual profile must recognize its complete reviewed catalog
and proof history or refuse it; initialization is not an exception to that rule.

Recovery must expose complete old/new outcome or unavailable reconciliation.
Preserve the pending draft on denied admission, its burned namespace identity,
all unrelated balances and original source/control/history/proof ownership. No
zero reconstruction, counter election, index rebuild, automatic purge/refund or
volatile capability restoration follows from this proposal. Ordinary stop/input/
end stays independent of hierarchy SQL/fsync and recovery scans. Unknown native/
paired disposition is held by its actual owner, not resolved by a ledger value.

The author proposes two new choices for scoped acceptance:

1. Fourteen fresh workspace resources, full nine-name run and five-name subject
   resource coverage, with the exact creation boundaries above. Draft lifetime
   remains preparation-only; no resource/cap/meaning or old profile is changed.
2. Initial research_run_controls insertion and its exact R/W control charges in
   the original root admission, keeping run_states historical and all runtime/
   collection/plan/child creation absent. No delayed current-owner initializer.

Acceptance would permit only offline documentation components: an independent
Python key/value/charge oracle, JSON, data-only JavaScript comparisons/tests and
evidence/routing. They may compare fresh-zero keys, actual initial-control charge
projections, fixed logical row-set cardinalities, original-proof requirements as
explicit assumptions, initialized/never-initialized branches and missing/extra/
reset/draft/retry denials. They must use labelled unqualified C links and **no
storage header/change/row hash, eligible store, complete history or publisher**.
Keep all old artifacts unchanged. This is not authorization for a full-state
fixture, physical schema/initializer, source factory, adapter, migration, native/
model probe, Rust/Electron module, dormant runtime reducer or executable plan.

The candidate fifty-family reconciliation is evidence, not a third approval of a
complete catalog. The next missing written authority decision is research original
preparation/ordinary lineage and retained authorization, followed by the full
catalog/version/proof/index closure. Existing child policy and physical layout
acceptance are not reopened. Actual source/currentness/authority, cutoff, ordinary
bridge, native/resource feasibility and broader history/terminal/consumer gates
remain blocking; #1689/#1089 stay open and #740 stays closed research.

## Written review and verification disposition

One bounded round used two fresh read-only Luna reviewers for correctness and
builder/evidence completeness. Neither found a required correction. They checked
resource creation and charging, the 16/27/24 logical counts, historical seed versus
current owner, and the explicit candidate-catalog/authorization limitations. The
author subsequently made inherited complete absence/reconciliation obligations
explicit above; no new freshness mechanism, policy or profile recognition was
selected. Reviewers ran no fixture, database or runtime/native process.

Pinned Node 22 docs verification passed all 1,607 existing tests, OKF metadata/
index and rendered-page/dead-link checks. All 20 OKF regressions, independent
resource/count arithmetic and source-derived 35+15 candidate-name reconciliation
passed. All 549 earlier non-Markdown validation artifacts remain byte-identical;
the prior membership and first-report technical bodies remain append-only.
Existing VitePress circular/large-chunk warnings concern unchanged bundler
configuration. Passing these checks or static reviews does not qualify a fresh
store, original producer, physical catalog/index/proof, authority, ordinary/native
behavior or the complete creation/publication history.

## Scoped owner acceptance and offline initializer components

The owner answered “keep going” to the explicit request to accept both choices
for **offline initializer components only**. This accepts the fresh resource
creation recipe and original-admission research current-control placement at
that scope. It does not close the candidate catalog or research authorization,
select a format/domain, or authorize a physical initializer or runtime work.

The bounded component packet uses unchanged root source/projection examples to
derive ledger keys/values, initial research control charges, initialized/never-
initialized branches and logical row-location counts. All origins are labelled
unqualified. Header/proof controls are labels only: there is no header value,
manifest, row/change hash, complete store, publisher or legal history. Existing
artifact bytes remain unchanged and all broader qualification gates above remain.

The earlier pending wording is historical for these two accepted component
choices only. The [offline initializer packet](../../validation/research-initialization-components.md)
records finite values/charges and their explicit limits; catalog/authority/format
and full-history qualification remain blocked.
