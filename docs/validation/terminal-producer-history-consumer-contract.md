---
type: Contract Investigation
title: Terminal report producer, history and consumer closure
description: Gate 2 source-owner investigation separating accepted terminal correction contracts from missing producer, ledger and complete consumer provenance.
tags: [orkworks, hierarchy, reports, producers, dependencies, qualification]
status: draft
workflow_status: gate-2-investigate
---

# Terminal report producer, history and consumer closure

Tracking: [#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `92944ab4fd5dba0b7d69f52b5f2a609bcb750ace`.

**Gate 2 remains Investigate.** This supplies the written investigation in the
[producer qualification plan](../superpowers/plans/2026-10-29-terminal-report-publication-qualification.md#gate-2--real-terminal-prestate-and-consumer-proof).
It selects no schema, command name, default, migration or lifecycle topology.
Accepted protocols and retained artifacts stay unchanged. No runtime/native
execution, dependent fixture, model probe or executable plan is supplied.

## Accepted inputs and evidence limits

| Input | Carries forward | Does not establish |
| --- | --- | --- |
| [#742 lifecycle acceptance](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md#logical-lifecycle-acceptance--2026-11-08) | Same-parent planning, exact approval, absorbing final completion/cancellation, retained children and immutable configuration | Closed terminal transition effects or report-ledger initialization |
| [Terminal correction profile](../superpowers/specs/2026-10-29-report-publication-profile-design.md#scoped-owner-acceptance--2026-10-29) | TerminalReportRunV1, compact heads/receipts, evaluation-only edges, source-health CAS, quotas and correction effects | A real terminal prestate producer or complete inventory |
| [Accepted UI protocol](../superpowers/specs/2026-11-10-terminal-ui-authority-protocol-design.md#scoped-protocol-acceptance-and-component-handoff--2026-11-10) | Exact scope/control/admission cutoff and receipt-v2 metadata; SQL commit is durable acceptance | Producer eligibility, native authority or historical command legality |
| [R2 sources](r2-report-components.md) and [conditional publication](report-publication-components.md#evidence-and-boundary) | Immutable bytes and conditional C1/O effects | Admission/termination provenance; the seed's subject-preparation label is not a legal creation command |
| [Authority components](terminal-authority-components.md#isolated-receipt-membership-is-not-a-legal-publication-transaction) | Offline scope and receipt-v2 joins | Receipt-only manifest is not a complete publication; originalProducerQualified is an assumption |

Sidecar admission-first permits one exact terminal/evaluation-only attempt through
sidecar-observed closure; closure-first denies. Stale success delivery is denied
separately. Ownership loss has no write exception. The original worker/store/lease
may remain retained indefinitely through disposition without making ordinary stop
wait for SQL. #744 claim-first remains excluded. These decisions are not reopened.

## Repository trace

Bounded search covered `crates/orkworksd/src`, `apps/desktop/electron` and
`apps/desktop/src` for TerminalReportRunV1, ResearchReportHead,
submitResearchReport, correctResearchReport, report_heads, report_receipts,
report-retained-bytes and terminal-report-corrections: no matches at this base.
Follow-up research_report/researchReport and report_publication/reportPublication
searches in the coordinator and Electron trees identified no producer. This does
not prove absence of differently named or external writers.

| Source / entry point | Observed responsibility | Gate 2 consequence |
| --- | --- | --- |
| [CoordinatorStore](../../crates/orkworksd/src/taskmaster/coordinator_store.rs), module contract and transition | Durable plan definitions/approval state; checks digest, generation and allowed PlanStatus before publishing StoredPlan. Module excludes runtime authority | Terminal plan status is not a terminal run, initialized history or settled child/grant authority |
| [Ending completion](../../crates/orkworksd/src/session_application/ending_completion.rs), complete_session_ending_result / complete_session_ending_captured | Captures original ordinary session context and writes final replay/metadata under identity checks | Ordinary ending is not TerminalReportRunV1. This extracted implementation is behind the older plan's session_application.rs pointer |
| [WorkspaceLease](../../crates/orkworksd/src/workspace_runtime.rs), acquire / validate_directory / Drop | Retains advisory-lock file; Unix directory validation compares its inode | A lease primitive is not hierarchy producer retention/disposition proof |
| [Electron callbacks](../../apps/desktop/electron/main.ts), onCloseAdmission / closeCurrentRuntime, and [switch coordinator](../../apps/desktop/electron/workspaceSwitchCoordinator.ts) | Closes local admission, cancels generation-bound work and awaits sidecar stop before opening another workspace | Independent hierarchy draining remains missing integration, not an existing protocol implementation |

The [storage registry](../superpowers/specs/2026-10-24-hierarchy-storage-contract-design.md#source-registry-and-remaining-schema-work)
leaves allocation/grant/recovery and configuration/plan/proposal schemas as
prerequisites. The [initial preparation profile](../superpowers/specs/2026-10-28-root-preparation-effects-design.md#scope-and-rejected-alternatives)
is assessing/null-runtime and excludes reports/plan consumers. It cannot produce
terminal state or report history. No inspected source closes that command chain.

## Terminal producer contract to close

#742 must name actual commands/direct writers and complete before/after row
inventories with original proof links for each applicable path below. These are
accepted lifecycle obligations, not proposed command tags.

| Path | Required provenance |
| --- | --- |
| Final execution completion | Admitted run, immutable run/bootstrap configuration, approved plan/revision and final coordination transition; revoke run bearer/grants and child report authority. PlanStatus/task success alone cannot select completed |
| Research-only/no-go/declined execution finish | Version-bound UI finish before execution approval, settled reservations and no interrupted allocation; retain summary/children/artifacts/disposition. Research-stage completion continues planning and is not final completion |
| Run cancellation | Authorized/version-checked cancellation, authority closure and pending reporting/allocation disposition; preserve blockers/clarification/history even if unsupported by the narrow variant |
| Interrupted transition recovery | Original command/row/receipt/resource proof yields complete old/new state or unavailable reconciliation; no inference from PTY end, event, timeout or maximum version |

Join terminal `{workspaceId,runId}` to parentSessionId, admissionEpoch,
runSequence, runDefinitionDigest and bootstrapConfigurationDigest. runId equals
epoch plus hyphen plus sequence. Ordered unique plans preserve exact
planId/planRevision/planDefinitionDigest triples and include the report's plan and
revision. Approved task/reservation/configuration and question/scope sources must
justify the stream. Check runVersion and inputRevision/inputDigest against their
predecessor history; there is one mutable runVersion owner, not a terminal cache.

TerminalReportRunV1 permits only completed/cancelled, null currentPlan and
currentParentRuntimeGeneration, empty blockers, false stageContinueRequired and
null pendingClarification. The lifecycle's `complete` and profile's `completed`
labels do not authorize a cast/conversion. Close the mapping and full effects;
never clear blockers, gates, clarification or historical inputs to fit it.
Unsupported terminal variants retain evidence outside this correction profile.

Completion does not prove all PTYs ended or authorize worktree deletion. Prove
pending-report disposition, closed child/grant publication authority and retained
allocation/runtime evidence instead. Parent detachment is not that proof. No new
child cutoff follows from the separately accepted terminal UI exception.

## Report history and accounting contract to close

| Join / owner | Required proof |
| --- | --- |
| Full #742 stream S → assignment | Workspace/run/plan/revision/task/reservation/child/configuration match approved sources; generations remain immutable version provenance |
| S → head → descriptor | One compact report_heads authority, fixed reportId, contiguous accepted prefix 1..head.reportVersion, at most eight; no duplicate generic head or maximum-version election |
| Accepted version → request/receipt → commit | Reconstruct operation, expected task/run version, predecessor and descriptor; compare exact canonical request bytes and digest. Resolve original links and complete legal command effects, not hash membership alone |
| Initial/child history | Version 1 is child submission with null expected head; later child submissions need authentic runtime/assignment and revocation ordering. Terminal correction creates neither initial history nor renewed child authority |
| v1/v2 receipts | Recognized variants retain original provenance; v2 records new terminal UI corrections. Historical retry does not require current lifetime/decision equality. No v1 retagging or recreated authority; unqualified provenance is unavailable |
| Report-retained-bytes, run/workspace | Sum canonical key plus committed-body projection for every owned report object/head/receipt, omitting only named outer links; reserved zero. Charge v2 metadata in every occurrence |
| Preparation/shared capacity | Complete non-report preparation usage plus terminal run-state charge; preparation/report share 32 MiB/run. Workspace report cap stays 256 MiB; no #744 credit, eviction or terminal refund |

The initialization gap is concrete. The
[initial resource contract](../superpowers/specs/2026-10-28-root-preparation-effects-design.md#resource-keys-and-zero-initialization)
initializes nine workspace resources and selected fresh run/assignment resources,
but not report-retained-bytes. The
[terminal profile](../superpowers/specs/2026-10-29-report-publication-profile-design.md#resource-and-command-budgets)
requires report ledgers already present and forbids creating/resetting them.
Neither permits inserting zero rows into an existing/ambiguous store. #742/storage
must close fresh-store/run creation and first-report atomic charges, including
all preparation occurrences. This investigation selects no extension; unknown
non-report preparation usage cannot be supplied as zero.

Preserve the 2 KiB complete receipt key/value cap: retained v2 component 1,244
bytes fits; maximum-width 2,149 rejects. Neither establishes complete legal command
fit. Keep 16 MiB / 4,096 changed rows, shared resource/lineage limits and checked
overflow. Recovery cannot synthesize receipts from descriptors, reset counters
on restart/rotation, elect surviving heads or upgrade rows. Preserve evidence and
keep relevant current reads/writes unavailable until qualified reconciliation;
old epochs remain read-only. Exact retry needs current authorization/rate/source
access before lookup and original response before live CAS, with no new effects.

## Complete consumer and index contract to close

| Owner / consumer | Complete membership obligation |
| --- | --- |
| #744 report EvidenceSource | Exact report identity, sourceKey, size/digest reconstruct its edge; retained uncited sources can still protect reports |
| #744 retained evaluation | Reconstruct unique edges from all outcome/quality/finding/dispute sources; include historical, ineligible and invalidated records, not only current heads |
| #744 projection | Resolve assignment/result, reviewer head/disposition and eligibility/currentness; retention membership differs from selected eligible subjectRevision tuples |
| #742 preparation/summary/proposal/plan/launched binding | Full owner inventories prove unsupported report consumers absent; terminal status does not retire historical references |
| #741 configuration / other review inputs | Complete owners establish absence; unclosed registry entries are unknown, not zero |
| #745 learning/custody | Local profile admits no shared learning; retained/ambiguous protection fences operation/deletion. Workspace inventory cannot establish repository-wide absence |
| #746 owner-run tracking | Derive source run and dependent assignments' tracked owner runs from full ownership; invalidate all visible changes before current reads, including optional-only changes |

Accepted graph: report ← evidence-source ← retained evaluation. Correction does
not delete/retarget old edges to v2. Reconstruct exact source-to-edge equality
from complete known inventories under the same boundary as insertion/removal,
purge and publication. Missing **and extra** edges reject. A consistent edge table
cannot prove all owners were enumerated. Unsupported/unknown consumers make this
profile ineligible; do not ignore/delete them or silently expand its effects.

A real completed run may retain a report-consuming summary/proposal/plan and
therefore fail this profile despite terminal status and ended children. No
inspected contract establishes those references retired. Demonstrate an eligible
history or propose a concrete consumer/retirement extension for separate review.
This neither erases consumers nor proves all real histories ineligible.

Recovery compares reconstructed membership with the physical index before current
reads/writes/purge. Count scans, overflow sentinels, repeated lookups and final
validation against 100,000 visited rows. Physical indexes/query plans/cooperative
deadlines remain adapter evidence. Overflow denies without truncation or splitting
the affected set. Insertion-first protects its target; publication/purge-first
forces the later consumer to revalidate or fail.

## Required observations and next handoff

Keep original R2 source/evaluation bytes. Required C1 correction makes C1 stale,
excludes f-c1 and advances material 3→4; optional O correction keeps required
findings and material 3. C2 stays a current failure and overall Needs rework in
both. Both change selected source-currentness tuples, advance subjectRevision
once and invalidate affected detail snapshots. Retention edges remain. Missing
lineage is unavailable, not reconstructed material or zero metrics. Preserve
explicit full-current-material presentation for viewed marking; receipts/tokens
cannot satisfy it.

Next is a **source-owner written closure**, not another conditional success packet:

1. #742 finish/cancel commands and complete predecessor/terminal effects,
   including frozen scope, allocation and pending-report disposition.
2. #742/storage fresh history/ledger creation and legal child/user provenance,
   compatibility and recovery, reconciling initial/terminal profiles without adoption.
3. #742/#744/#746 complete consumer/owner-run inventories and edge reconstruction,
   unsupported-consumer exclusion and #745 custody fences.

Each closure needs authoritative fields, full before/after effects, bounds,
rejection/recovery and canonical contract reconciliation. New codec/default or
consumer-retirement choices need scoped acceptance before dependent fixtures;
accepted protocols need no repeat general approval. Request the missing source
specifications through #1089 rather than inventing rows or empty-history flags.

Gate 2 exits only when mappings are closed/reviewed in canonical contracts and
real provenance replaces captured inputs. Native authority/lifecycle/storage,
custody, applicable capability evidence and separately approved executable plans
remain independent. #740 is closed research with partial practical evidence, not
a request to reopen universal containment research. Keep #1089/#742/#744/#746 open.

Local review against Gate 2's four requirements finds terminal, history/accounting,
consumer/index and projection handoffs above. Ready as a written investigation;
**not Gate 2 completion or executable handoff**. Reverting this document changes
no store; future schema downgrade/recovery remains undefined until designed.

## Fresh-ledger proposal — 2026-11-11

The [initialization proposal](../superpowers/specs/2026-11-11-report-retention-initialization-design.md)
under [#1679](https://github.com/Rambolarsen/orkworks/issues/1679) makes the ledger
creation gap concrete: workspace zero report usage at fresh store initialization
and run zero usage at first source admission, with original proof recognition and
no backfill of older stores. These are proposed choices, not acceptance or a
producer-qualified history. Gate 2 remains Investigate; finish/cancel, original
report publication and complete consumer/index proof remain open.
