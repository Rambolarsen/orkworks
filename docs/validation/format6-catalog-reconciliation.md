---
type: Qualification Audit
title: Format6 catalog and original preparation reconciliation
description: Source-backed inventory of 35 accepted logical families, membership extensions and original paired preparation obligations, with physical qualification unresolved.
tags: [orkworks, hierarchy, preparation, catalog, qualification]
status: draft
workflow_status: logical-inventory-reconciled-physical-closure-required
---

# Format6 catalog and original preparation reconciliation

Tracking: [#1424](https://github.com/Rambolarsen/orkworks/issues/1424),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `2ce689f21210279d11725bc7a536fc3021e7b431`.

## Scope and result

The [authorization representation acceptance](../superpowers/specs/2026-11-05-child-preparation-authorization-design.md#scoped-representation-acceptance--2026-11-05)
accepts the exact format6 representation for further qualification. This audit
reconciles its logical inventory and original preparation obligations; it selects
no replacement codec, physical index, SQL plan or authority mechanism.

The 35 logical families reconcile as 13 subject/storage families, five root
families, four child-source families, nine child-control families, two artifact
families and the two accepted joint-preparation additions. Logical family names
are distinct from object kinds, dependency purposes and derived physical indexes.
The ordinary lineage family is a projection of ordinary storage, not permission
to create a second independent hierarchy copy.

**Gate 2 remains Investigate.** The inventory is complete for the named accepted
slice; full physical membership, original producer ownership, resource/native
qualification and the paired old/new effect witness are not supplied. Unknown
consumers remain unavailable. No database, schema/migration, dependent fixture,
model/native probe or production publisher was run or enabled.

## Exact logical inventory

Keys below name the owning contract's full canonical types. Tuple notation is
that contract's existing exact logical key, not permission to invent a delimiter
encoding. Values, variants and source codecs are inherited through these links:

- **S**: [storage rows and object/head registry](../superpowers/specs/2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation).
- **C**: [compact claim/slot/retry bindings](../superpowers/specs/2026-10-25-hierarchy-compact-bindings-design.md).
- **R**: [root source rows](../superpowers/specs/2026-10-25-root-preparation-producer-design.md#storage-membership-and-atomic-effects) and [root preparation effects](../superpowers/specs/2026-10-28-root-preparation-effects-design.md#closed-initial-registry-additions).
- **K**: [child source rows and transitions](../superpowers/specs/2026-10-31-child-review-source-preparation-design.md#consumer-membership-and-retention).
- **L**: [control row registry and transition ownership](../superpowers/specs/2026-11-02-child-launch-control-design.md#transition-and-recovery-rules).
- **A**: [artifact streams](../superpowers/specs/2026-10-30-evaluation-artifact-producer-design.md) and [historical debit](../superpowers/specs/2026-10-31-artifact-read-debit-proof-design.md#one-retained-row-and-exact-six-row-effect).
- **J**: [ordinary lineage projection](../superpowers/specs/2026-11-05-joint-child-preparation-design.md#original-ordinary-lineage-and-versioned-commitments) and [authorization row](../superpowers/specs/2026-11-05-child-preparation-authorization-design.md#one-immutable-recovery-row).

Producer labels identify required responsibilities, not implemented routes. Every
retained row also retains its owning direct proof under its exact link exclusions.
[The membership registry](../superpowers/specs/2026-11-03-child-consumer-membership-design.md#direct-metadata-and-source-group-registry)
supplies the complete reference recipes for inherited retained consumers; the
short descriptions below route to those recipes rather than replacing them.

| # | Family | Full logical key / codec | Required producer and retained membership |
| --- | --- | --- | --- |
| 1 | objects | ObjectKey / S, K, A | Typed immutable source/result/evaluation publisher; exact ObjectRefs, source groups and registered dependencies |
| 2 | dependencies | (consumer ObjectRef, target ObjectRef, purpose) / S, K, A | Same owning source/custody publication; both full endpoints and purpose, incoming and outgoing |
| 3 | payloads | (AssignmentIdentity, resultRevision, declarationId) / S | Result publisher; exact retained result, approved declaration and content |
| 4 | heads | HeadKey / S | Exact report/result/evaluation stream publisher; original target and stream identities |
| 5 | summary_state | AssignmentIdentity / S | Source/summary owner; SubjectSource, material and checkpoint/applied proof when present |
| 6 | pending_material | (AssignmentIdentity, assignedRevision) / S | Summary owner; both before/after materials and ordered checkpoint lineage |
| 7 | receipt_slots | (AssignmentIdentity, slotId) / S, C | Claimed mutation or original finalizer; exact reserved/consumed purpose, subject/result/stream and proof |
| 8 | candidate_work | candidate ObjectRef / S | Candidate admission/finalization owner; original candidate and nullable outcome slot |
| 9 | quota_usage | (owner, resource) / S, R, K, L, A | Owning source/command; full assignment/run/workspace mapping and retained inventory |
| 10 | claim_proofs | ClaimIdentity / C | Accepted claimed mutation; expanded local assignments/authority/precondition/record and receipt |
| 11 | mutation_lookup | RetryKeyDigestV1 / C | Same claimed mutation; reconstruct and compare complete retry key and original response |
| 12 | commit_proof | positive commitSequence / S, C | Complete owning transaction; direct row/receipt/checkpoint/snapshot obligations, no automatic predecessor replay |
| 13 | store_header | singleton 1 / S | Complete transaction owner; exact latest direct proof in this workspace/epoch |
| 14 | root_drafts | workspaceId / R | Standalone root draft producer/cancel/admission owner; explicitly outside child-consumer locators |
| 15 | root_preparations | {workspaceId, runId} / R | Original root admission; complete approved bootstrap/run/receipt, artifact occurrences and original source group |
| 16 | run_states | {workspaceId, runId} / R | Original root preparation; immutable initial source, not current run-control replacement |
| 17 | parent_attachments | parentSessionId / R, L | Root/parent owner; original root identity and applicable immutable runtime history |
| 18 | preparation_artifacts | {workspaceId, runId, referenceId} / R | Root draft/preparation owner; each exact cited occurrence, raw bytes and digest |
| 19 | child_plan_approvals | {plan:PlanRef, approvalId} / K | Exact UI/plan approval owner; original request/source/receipt and plan |
| 20 | child_reservations | TaskKey / K | Original reservation owner; plan/configuration/approval, unique child session and frozen task version |
| 21 | child_allocations | ReservationRef / K | Original native allocation owner; reservation/plan/group and proven original disposition |
| 22 | child_attachments | childSessionId / K | Original joint preparation owner; original reservation/allocation/P and unique attachment/source |
| 23 | child_control_inputs | PreparedInputRef without inputDigest / L | Source-owned input producer; all typed InputRecords, original producer runtime and root/run |
| 24 | child_runtime_attachments | RuntimeKey / L | Actual parent/child runtime owner; original root or child attachment/source, never proposed spawn identity |
| 25 | child_run_controls | {workspaceId, runId} / L | Run owner; current input, all retained plan references, parent runtime and blockers |
| 26 | child_plan_controls | PlanRef / L | Plan owner; immutable source/task/prepared input, original latestApproval/replaces references |
| 27 | child_task_controls | TaskKey / L | Task owner; original plan/configuration/reservation and nullable coordination outcome |
| 28 | child_launch_controls | TaskKey / L | Launch owner; original reservation/admission/allocation/attachment and established spawn history |
| 29 | child_session_states | TaskKey / L | Joint preparation then actual lifecycle bridge owner; original assignment/attachment and nullable current runtime |
| 30 | child_coordination_receipts | TaskKey / L | Parent coordination owner; original request/outcome/reservation and direct response proof |
| 31 | child_recovery_decisions | {task:TaskKey, recoveryId} / L | Supported original recovery owner; complete frozen snapshots, expected/origin/receipt proofs |
| 32 | evaluation_artifact_streams | {producerAssignment:P, declarationId} / A | First artifact/evaluation publisher; immutable mapping, exact target result and all retained versions |
| 33 | artifact_read_debits | DebitIdentityV1 / A | Qualified precharge owner; producer/artifact/evaluation/mapping, original ledger joins and direct proofs |
| 34 | ordinary_child_lineage | full ChildAssignmentIdentity P / J | Joint ordinary producer projection; exact creation identity/member, source and original C |
| 35 | child_preparation_authorizations | full ChildAssignmentIdentity P / J | Exact permit/joint producer; original intent, four control snapshots, source groups, H/origin/C and ordinary lineage |

The table numbering is explanatory; it is not a wire tag or manifest ordering.
Manifest ordering stays canonical table name then full canonical key bytes.
root_drafts is deliberately excluded from retained child locators by the
[existing explicit rule](../superpowers/specs/2026-11-03-child-consumer-membership-design.md#existing-subject-proof-and-control-families):
admission consumes the original draft and retains the complete approved source.
A child does not pin a later workspace draft. Root draft accounting, artifacts
and cancel/admission atomicity remain owned by R; exclusion is not free storage.

## Membership and physical indexes still requiring closure

The accepted additions require integration into one closed lookup grammar; this
is not authorization to accept arbitrary table/key or field-path queries.

| Obligation | Established contract | Remaining evidence / reconciliation |
| --- | --- | --- |
| Object/dependency membership | Exact ObjectRefs and both dependency endpoints/purpose | Complete bounded forward/reverse membership under one H, with original owning writes and recovery |
| Inherited metadata consumers | Enumerated MetadataTable variants and full direct/source-group recipes | Physical completeness and coherent bounded queries; generic key lookup does not establish absence |
| Ordinary lineage consumer | Fixed P-keyed metadata projection, reciprocal/direct membership and tombstone retention | Incorporate the accepted family into the closed location/target dispatch and ordinary index owner; no duplicated authority |
| Authorization consumer | Fixed immutable P-keyed row, complete typed source/control/proof recipe | Incorporate the accepted family into the same closed dispatch and maintain all forward/reverse/direct occurrences at C |
| Plan-control historical origin | Authorization freezes RunControlRead, PlanControlRead, TaskControlRead and LaunchControlRead | Older ControlSnapshotTable lists run/task/launch/session only and describes RecoveryDecisionV1 snapshots. Reconcile plan origins and authorization-contained snapshots explicitly, preserving recovery's existing variants |
| Exact permit uniqueness | Full PreparationPermitIdentityV1 unique across P | Derived full-key index, duplicate/body/key refusal, insertion/rollback/recovery and retained-copy costs; no new logical family |
| Session/attachment uniqueness | Full retained source history plus ordinary lineage/tombstones | Exact lookup in both directions and cross-run/store-owner evidence; one local primary key is insufficient |
| Direct origin membership | Exact snapshot origin manifest afterDigest; expected H and new C proofs | Preserve historic snapshots when current controls change; no link-only rewrite or recursive predecessor retention |

The plan-origin row records an explicit composition gap between accepted contracts,
not a newly chosen union. A follow-up must make the closed integration unambiguous
before implementation. If it changes an accepted interface or meaning, obtain the
scoped decision for that change; unchanged representation choices stay accepted.

At the inspection base, [schema.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/schema.rs)
contains exactly 17 fixture families, eight explicitly out of producer scope.
The 18 logical families absent from that fixture are claim_proofs, mutation_lookup,
all five root families, all nine child-control families and the two joint additions.
This set difference is a source inventory, not a request to add 18 empty tables.
The fixture's verify_catalog rejects non-table catalog entries; it cannot simply
accept new derived indexes as an already qualified layout.
[find_row](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/connection.rs)
queries one canonical key and proves no complete incoming membership.
[The module boundary](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mod.rs)
keeps mechanics under cfg(test), and [production Connection::open](../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs)
refuses the unqualified profile before path I/O. The partial fixture's older
33-family comment is historical context, not format6 admission.

## Original paired preparation effect reconciliation

This is a producer obligation map, not a complete manifest or independently
generated old/new state packet. Every changed logical key must be coalesced once;
read-only peers retain their old links. H and ordinary epoch/G are captured and
revalidated together with the complete actual read set and actual ownership.

| Owner / family | Required original child-source-attach outcome |
| --- | --- |
| Ordinary store | One complete normalized creating record, initial history/search projections, immutable lineage member and full-key lookup; history generation advances once. No SessionHandle or startup future |
| ordinary_child_lineage | One P-keyed creation projection; preparationCommit receives original C after hashing and independently validates against the complete source/attachment proof |
| child_attachments and objects | Original attachment plus exact derived child-assignment-source; original immutable plan/configuration/input/approval/reservation/allocation joins agree |
| dependencies and direct consumers | All source-provenance and actual typed source/control/lineage/authorization occurrences; physical index maintenance belongs to the same owning transaction |
| child_session_states | Initial prepared bridge at producerRevision "1", runtime null |
| child_launch_controls and owning controls | Prepared attachment outcome and every actually required run/plan/task/control update, with semantic version increments coalesced according to L; no extra increment solely for proof insertion |
| summary_state and actual summary effects | Exact initialized Unassessed/checkpoint branch or accepted never-initialized marker; subjectRevision "1" for the new subject. No invented empty passing criteria |
| quota_usage: five assignment keys | Initialize record, payload, reporting-receipt, artifact-retained and artifact-read-work resources at used="0" / reserved="0" only for the provably new assignment |
| quota_usage: existing scopes | Verify the ten corresponding run/workspace peers already exist; apply only actual aggregate deltas without stamping unchanged peers. Charge all actual source/control/lineage/authorization and other owning pools; do not fund metadata from the five zero rows |
| child_preparation_authorizations | Exactly one original P/permit/intent row; exact permit index and all copied snapshots/direct proof retention charged within existing lower limits |
| store_header and commit_proof | Complete sorted format6 manifest, commandKind child-source-attach, claim null; every actual logical change plus separate header/new-proof equations, not their circular inclusion in changes |

No new result/evaluation/artifact/debit, reporting receipt slot, claim proof,
mutation retry lookup, native runtime attachment or grant is created by this
preparation. Prior approval/reservation/allocation and captured control origins
remain retained; source retry resolves original P/C and never overwrites later
ordinary display metadata. Any actual source/control/summary/proof effect beyond
the rows named here must still be accounted for by its owner; this map does not
assert that its minimum recipe is a sufficient transaction.

Hash unchanged source bodies and snapshot origins, candidate authorization,
complete charges/row commitments, then manifest C; attach only the already named
excluded links afterward. Use format6 row/change domains throughout this fresh
profile, preserving inherited exact exclusions and nested source/proof domains.
No old format5 import, mixed-domain CommitRef or backfill is selected.

## Remaining gates and finite handoff

1. Close the logical membership dispatch for the two accepted additions and
   plan-control origins, then specify and review the complete physical lookup/
   uniqueness representation with every actual key, query bound and owning write.
2. Trace original ordinary metadata normalization, schema2/index/tombstone writers
   and paired connection lifecycle through [producer qualification #1242](https://github.com/Rambolarsen/orkworks/issues/1242).
   Current [session_store.rs](../../crates/orkworksd/src/session_store.rs) defines
   ordinary session/history/search/deletion indexes; those do not implement the
   accepted hierarchy lineage or a joint source publisher.
3. Resolve actual source/parent/revocation and candidate compare-and-consume
   ownership; RegistryRoot remains independently proposed. Retained evidence
   does not prove a live permit, lease or current native runtime.
4. Prove physical membership, lease/health, actual queue/job/SQL/native bounds,
   shared journal/super-journal accounting and retained unknown disposition under
   a separately approved plan. The ordinary 1 MiB joint-job and other lower
   limits can deny otherwise source-valid records; no capacity increase follows.

Only concrete evidence closes each item. Complete independently generated
old/new/manifest/packing fixtures and executable/native work retain their separate
gates. #1424 completes this bounded inventory audit, not full profile qualification;
#1176 stays open and #740 stays closed research.
