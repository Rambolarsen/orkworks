---
type: Validation Evidence
title: Evaluation-artifact source and authority closure audit
description: Bounded contract and source investigation separating accepted artifact choices from missing retained child producer records and excluded read-work commit ordering.
tags: [orkworks, hierarchy, artifacts, sources, authority, qualification]
status: draft
---

# Evaluation-artifact source and authority closure audit

Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176),
[#1183](https://github.com/Rambolarsen/orkworks/issues/1183),
[#1184](https://github.com/Rambolarsen/orkworks/issues/1184); initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `11c99357809f24f3282846d3a60e7fa82f12175f`.
The [artifact producer choices are owner accepted](../superpowers/specs/2026-10-30-evaluation-artifact-producer-design.md#owner-acceptance-and-gate-2-handoff--2026-10-30)
for further qualification. This audit starts Gate 2; it supplies no source-valid
transaction, new source schema, native ordering mechanism or runtime eligibility.

## What acceptance settles

The immutable artifact metadata/stream/content-custody representation, format-4
selection, no connected-subject purge, producer/read-work limits, no-refund/reset
policy, excluded read-work precharge and per-sidecar materializer/memory policy
are no longer awaiting that scoped design decision. Their actual source owners,
fresh preparation, complete consumer/counter membership and native enforcement
remain independent. Keep those two evidence categories separate.

## Investigated contract and source joins

| Join / responsibility | Established source | Gate 2 consequence |
| --- | --- | --- |
| Logical v3 reviewer configuration, typed review input and structured output | [#741 typed input](../superpowers/specs/2026-10-04-agent-role-configuration-design.md#proposed-typed-reviewer-input--2026-10-24) and [v3 output placement](../superpowers/specs/2026-10-04-agent-role-configuration-design.md#placement-versions-and-canonical-bytes) define exact target/declaration/read scope and content identities | Do not invent another logical identity or widen the read grant; complete retained configuration/allocation/consumer representation still needs its source-owner codec |
| Frozen child task version and unique attachment | [#742 admission snapshot](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md#proposed-evaluation-admission-version-snapshot) fixes assignmentTaskVersion to the resulting successful reservation version and requires exact unique attachment | A supplied child ID or latest mutable task counter cannot close the source join; use the retained committed reservation and original attachment proof |
| Configuration/plan and allocation/grant/recovery storage | [Registry's explicit remaining work](../superpowers/specs/2026-10-24-hierarchy-storage-contract-design.md#source-registry-and-remaining-schema-work) leaves these kinds inadmissible until their owners supply closed fields/keys/domains/quotas | Artifact format 4 cannot adopt illustrative objects or pretend unknown consumers are absent; #1183 closes this specification prerequisite |
| Fresh artifact/read-work zero ledgers | [Accepted producer budget rule](../superpowers/specs/2026-10-30-evaluation-artifact-producer-design.md#proposed-finite-production-read-and-materialization-budgets) requires explicit source-owned fresh preparation before reporting authority | Missing historical rows are unavailable; evaluation submission cannot initialize guessed-zero assignment/run/workspace usage |
| Source/result/snapshot/evidence components | [Coherent review history](coherent-review-history.md) binds declared synthetic identities and exact source bytes while explicitly leaving full configurations, source admission and custody unqualified | Neither that packet nor earlier root preparation supplies a complete child reviewer prestate or format-4 transaction |
| Read delivery authorization | [#742 final response commitment](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md#proposed-retained-review-input-admission) rechecks current capability/generation/scope/custody under revocation serialization before a bounded transport handoff | It orders delivery of bytes, not persistence of the earlier read-work quota precharge |
| Write claim exception | [Exact amendment exclusions](../superpowers/specs/2026-10-24-hierarchy-authority-ordering-design.md#amendment-scope-and-exclusions) name the allowed #744 writes and forbid inheriting that exception for arbitrary SQL | New read-work metadata has no evaluation-write claim permission; #1184 must resolve its own reservation cutoff without an implicit policy amendment |

A root bootstrap's accepted source/preparation contract does not replace a child
reservation or attached reviewer identity. Ordinary session lifecycle metadata
also cannot serve as an artifact grant or source preparation record.

## Bounded production-source inspection

These useful searches ran from the inspected task worktree; no live session,
SQL store, credential, model or native fault probe was created:

```bash
rtk proxy rg -n 'evaluation-artifact-read-work|evaluation_artifact_streams|EvaluationArtifactV1|AssignmentEvaluationCapability|AssignmentReviewReadCapability|ClaimProofV2|assignmentTaskVersion' crates/orkworksd/src apps/desktop/electron apps/desktop/src docs/superpowers/specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md
rtk proxy rg -n 'WorkspaceLease|complete_session_ending' crates/orkworksd/src/workspace_runtime.rs crates/orkworksd/src/session_application.rs
```

The first exact-symbol search returned no matches at this base. That establishes
only absence of these named implementations in those roots; it is not proof
that no differently named/external writer or future native solution exists.
The evidence below checks the actual nearby owners rather than treating that
negative search as an impossibility finding.

- [CoordinatorStore](../../crates/orkworksd/src/taskmaster/coordinator_store.rs)
  begins by stating that it owns durable definitions/approval data and no runtime
  authority (lines 1–4); StoredPlan holds plan/status/approval history (lines
  49–55). File-publication logic there is not a current evaluation-capability or
  read-work physical-commit fence merely because it is durable.
- [WorkspaceLease](../../crates/orkworksd/src/workspace_runtime.rs) is an open
  advisory-lock file guard (lines 161–185). Its retained FD establishes workspace
  ownership; file existence is explicitly not the signal. It does not itself
  serialize token/generation revocation with an artifact-work SQL commit.
- [complete_session_ending](../../crates/orkworksd/src/session_application.rs)
  checks ordinary runtime generation/lifecycle, persists the final replay grid
  and ended metadata/status (lines 3621–3695). That flow cannot be promoted into
  an artifact reservation/attachment source or hierarchy worker disposition proof.

The current schema/worker/claim designs are still gated implementation directions.
No inspected source call supplies the accepted artifact work transaction, its
complete retained child producer prestate or a qualified native reservation fence.
This is a bounded finding, not an exhaustive codebase audit.

## Separate authority events, preserve the accepted policy

| Event / boundary | Required evidence or result |
| --- | --- |
| Current read request accepted and rate admitted | Exact capability/generation/producer/stream/declaration scope; admission alone creates no content or persistence permission |
| Durable work charge queued/built | Revalidate exact source/header/ledgers and all budgets; no source content fetched or grant restored by preparing an intent |
| Revocation versus work-charge physical commit | The read-work command remains excluded from write claim-first. Identify an enforceable reservation cutoff or present a separately reviewed amendment; do not count a precheck/response check as the fence |
| Charge committed, then revoke/disconnect/error | Preserve the no-refund used counters; deny newly uncommitted fetch/response under their current scope rules. A work charge proves no delivered bytes |
| Final bounded response commitment | Apply existing #742 response ordering. Already committed bytes may arrive after revocation; uncommitted buffers are discarded, with no blocking network I/O inside the boundary |
| Namespace/lease/workspace replacement or retiring worker | Preserve exact ownership and disposition, block unknown authority, keep one active-or-retiring materializer and no counter reset; no replacement worker or stop delay |

The existing write-ordering design already explains the physical-commit gap and
rejects core locks/stop delays across SQL/fsync as an assumed solution. This
audit does not invent a new mechanism or conclude one is impossible. The accepted
read-work cutoff is preserved; no new permission or claim class is selected.

## Scoped handoff and disposition

[#1183](https://github.com/Rambolarsen/orkworks/issues/1183) owns the closed retained
child reviewer configuration/reservation/attachment/preparation records and complete
consumer/zero-ledger joins. [#1184](https://github.com/Rambolarsen/orkworks/issues/1184)
owns the exact read-work reservation authority investigation and any concrete
alternative that would need owner review. They are specification/research
prerequisites of #1176, not runtime implementation issues or independent approval
to alter the accepted source/custody/claim policy.

Acceptance record: Gate 1 complete. Investigation result: Gate 2 **Investigate**;
source/consumer and reservation-ordering evidence is not closed. Gates 3–4 remain
gated. No metadata/hash fixture, full state/manifest, database, migration, adapter,
live model/native probe or executable/runtime change was produced by this audit.
Existing logical source/codec decisions stay accepted in their original scope;
new source or authority decisions receive one concrete scoped owner review before
new dependent fixtures. Keep #1176 and the owning component issues open.


## Child source proposal handoff — 2026-10-31

The [selected child source design](../superpowers/specs/2026-10-31-child-review-source-preparation-design.md)
proposes exact retained record interfaces and consumer/preparation joins for #1183.
Its [qualification handoff](../superpowers/plans/2026-10-31-child-review-source-qualification.md)
keeps new codec/profile/budget choices pending owner review and actual underlying
control/source/authority closure distinct from component bytes. This audit and
older fixtures/checkers remain unchanged; Gate 2 and #1184 are not passed by the
new proposal, and no runtime or dependent transaction is authorized.


## Read-work authority investigation — 2026-10-31

The [#1184 investigation](artifact-read-work-authority.md) traces the actual nearby owners, separates
durable debit permission from content access and bounded response commitment, and
records complete charge/denial/recovery oracles. It finds no qualified physical
commit/revocation fence in the inspected integration. Its narrowly scoped internal
debit-permit alternative is **proposed, not accepted**; the existing cutoff and
no-go remain. Owner decision, exact proof/command codecs, #1183 source closure and
a separately approved bounded native plan still precede dependent transactions.
Gate 2 remains Investigate; no #744 claim reuse, fixture, native probe, database or
runtime implementation follows.


## Debit policy accepted; exact representation remains — 2026-10-31

The [#1184 acceptance record](artifact-read-work-authority.md#owner-acceptance-and-command-proof-handoff--2026-10-31)
completes the written investigation/owner-decision/handoff: one exact internal
permit-first debit may reach disposition after later capability revocation,
without content/delivery rights or #744 ClaimProofV2 reuse. This supersedes the
prior owner-pending cutoff wording only for that debit. Source/health/lease/quota
validation and ordinary stop independence remain binding.

[#1210](https://github.com/Rambolarsen/orkworks/issues/1210) tracks exact command/
permit-proof/recovery/format-4 representation; no codec was selected or accepted
by the policy decision. #1183 source/preparation/consumer/custody joins and actual
native authority/lease/retirement remain unclosed. Gate 2 remains Investigate;
dependent complete-state/manifest fixtures, database/native/model probes,
migration and runtime work remain gated.


## Current source-owner and membership audit — 2026-11-04

[#1375](https://github.com/Rambolarsen/orkworks/issues/1375) revisits Gate 2 at
`e89fdc4fedaee691297710d2e52547f5110e831e`, after the initial/correction/invalidation maps and production
admission fence.
This is a bounded source-to-obligation audit of the checked-in sidecar and desktop,
not a native run, complete writer census or eligible publication result. The
historical audit above remains evidence of its original base. Closed
#1183/#1184/#1210/#1356 deliverables are not reopened.
Here T and P denote the complete target and producing child AssignmentIdentity;
result, reviewer stream and declaration bindings remain exact under the accepted
publication maps linked below.

### Actual seams and their evidence limits

| Required owner | Current source evidence | What remains unqualified |
| --- | --- | --- |
| Retained child preparation/configuration/identity | [child_sources.rs](../../crates/orkworksd/src/taskmaster/child_sources.rs), validate_unverified_child_sources_v1, decodes supplied run/input and worker/reviewer bundles and checks snapshot, source, configuration and independence joins. Its classification is explicitly unverified-source-bytes-only; located calls are in child_sources_tests.rs | An actual source producer that commits preparation/approval/reservation/allocation/attachment, proves fresh run/workspace/assignment ledgers and registers a live binding. Supplied valid bytes do not establish those events or their custody |
| Root preparation and source group | [root_bootstrap.rs](../../crates/orkworksd/src/taskmaster/root_bootstrap.rs), validate_unverified_root_bootstrap_v2, validates caller-supplied retained bytes and exposes the same unverified classification; located calls are in root_bootstrap_tests.rs | Original source-owned run/root admission, ordinary-session/control/runtime-history bridge and retained proof participation. A root validator does not admit a child reviewer |
| Plan definitions and approval persistence | [coordinator_store.rs](../../crates/orkworksd/src/taskmaster/coordinator_store.rs), StoredPlan/CoordinatorStore, owns JSON plan/status/approval history and expressly no runtime authority | Exact accepted child-source records and current plan/run/runtime/grant participation in the artifact publication boundary. Durable approval data alone supplies no evaluation capability |
| Current publication claim/revoke/response owner | [#1372's concrete proposal](../superpowers/specs/2026-11-04-evaluation-publication-authority-design.md#scoped-decision-requested) traces ordinary lifecycle/prompt/runtime/lease seams and proposes a separate owner and non-I/O first unit | Its new owner/two-phase close/identity/token choices still await scoped owner decision at this base. Even that first unit would supply synthetic ordering evidence, not source registration truth or physical commit/lease enforcement |
| Source-owned hierarchy publication/store | [hierarchy_storage/mod.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mod.rs) compiles mechanics only under cfg(test); [connection.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs), Connection::open, returns unqualified profile before I/O | No eligible initial/correction/invalidation publisher or admitted format-4 store is exposed. The ten/nine/four-row minimum cores in the accepted maps are obligations, not existing commands |
| Retained record/journal mechanics | Test-only [publication.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/publication.rs) journals one object/metadata row/edge, not an accepted source action; [schema.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/schema.rs) has a partial 17-family catalog; [connection.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/connection.rs), find_row, seeks one canonical key | Complete 33-family support, source-command atomic effects and coherent forward/reverse/direct metadata membership. One key lookup or matching returned object cannot prove complete retained membership or a valid absence |
| Ordinary session persistence/runtime ownership | [session_store.rs](../../crates/orkworksd/src/session_store.rs) owns ordinary sessions, organization/search/deletion and their indexes. [SQL lifecycle](../../crates/orkworksd/src/session_application/sql_writes/lifecycle.rs) and [EndingPersistence](../../crates/orkworksd/src/runtime/terminal_runtime/ending_persistence.rs) capture ordinary runtime/history ownership | These are possible integration precedents only. Their schema, execution tickets, generation checks, replay/events and completion do not supply hierarchy AssignmentIdentity, evaluation claims, artifact custody or incoming-consumer indexes |

The bounded call-site and symbol checks are reproducible from this inspection
base (no database, model, native fixture or foreign worktree was read):

```bash
rtk proxy rg -n 'validate_unverified_child_sources_v1|validate_unverified_root_bootstrap_v2|hierarchy_storage' crates/orkworksd/src apps/desktop/src apps/desktop/electron
rtk proxy rg -n 'AssignmentEvaluationCapability|AssignmentEvaluationDisposition|ClaimProofV2|evaluation-write|evaluation-invalidation|evaluation_artifact_streams|artifact_read_debits' crates/orkworksd/src apps/desktop/src apps/desktop/electron --glob '!**/hierarchy_storage/mechanics/**'
rtk proxy rg -n 'CREATE TABLE|CREATE.*INDEX|ROW_TABLE_DDL|find_row' crates/orkworksd/src/session_store.rs crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/schema.rs crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/connection.rs
```

The first search locates only validator declarations/test calls and the hierarchy
module declaration. The second has no matches outside excluded test mechanics at
this base. The third distinguishes ordinary indexes from the partial fixture's
canonical-key layout. These facts do not prove absence of differently named,
external or future owners. The explicit no-I/O classifications, test-only module
boundary and production admission refusal independently constrain what the
located code can establish; no symbol absence is used as an impossibility proof.

### Logical lookup closure required by the three recipes

The [accepted membership contract](../superpowers/specs/2026-11-03-child-consumer-membership-design.md#coherent-discovery-and-publication)
requires one storage owner to bind every forward/reverse/direct lookup and final
read-set validation to the same workspace/storage epoch and exact header. The
following are qualification obligations, not proposed SQL/index DDL or a new
generic graph schema. Each physical implementation must return the owning typed
record or qualified metadata projection and prove complete correspondence in both
directions, including absent keys, historical consumers and direct proof links.

| Lookup purpose | Required source-owned relation / closure evidence |
| --- | --- |
| Source/preparation and authority binding | Full T/P identities resolve original root/child source groups, plans, approvals, reservation/allocation/attachment, control/runtime/ordinary history and explicit owner preparation. A source digest or current session ID alone proves neither event custody nor fresh zero-ledger state |
| Exact stream, artifact and frozen state | Resolve head by the exact result/reviewer stream, mapping by P/declaration, and every required version's artifact/evaluation/receipt/custody joins. Verify unique disposition/no prior disposition and no-head/mapping absence from complete retained history. No largest-version or surviving-receipt reconstruction |
| Global child slots and original invalidation credit | Enumerate this exact target result's twelve possible child admissions across streams with their receipts/proofs. Distinguish global admission order from per-stream revision; resolve the stream's original first-write pair for invalidation. Current-head reserved credit alone is not spendability proof |
| Exact retry and direct proofs | Retry lookup resolves original full scope, claim, consumed slot and canonical response/source. Header/latest and every retained row/receipt/checkpoint/origin resolve their own required proofs. Sparse retention is allowed; neither maximum-sequence election nor recursive predecessor replay may recover missing acceptance |
| Shared retained accounting | Prove complete target record/receipt and producer artifact/read-work ownership under exact assignment/run/workspace resource keys, including retained old runs/epochs. Validate original preparation and charged occurrences; no missing row becomes zero and physical content sharing earns no discount |
| Incoming consumers, affected material and custody | Expand both full ObjectRef endpoint directions and direct metadata/source/proof groups, including historical versions, receipt/claim/lookup, summaries/checkpoints/pending lineage and historical debit consumers. Preserve no-purge custody and derive actual source-currentness/material effects separately from retention membership |

Initial publication additionally needs complete no-head/no-mapping and fresh
source proofs; correction retains the original mapping, all prior versions and
protected credits; invalidation retains the head/artifact history and uses only
the first-write paired credit. The exact immutable body/source rules remain in
[the three publication maps](../superpowers/specs/2026-10-30-evaluation-artifact-producer-design.md#initial-child-evaluation-publication-derivation--2026-11-04).
Neither a row's own hash nor an empty result page proves that no additional
consumer, prior stream, disposition or charge exists.

All discovery observations, unsuccessful exact probes and returned metadata
copies count against the accepted 4,096-observation / 16 MiB cumulative limits;
complete-command rows/bytes are a separate bound. Finish every continuation
within that admission or deny; do not truncate at a page boundary. Revalidate
membership and the entire source read set before the owning atomic producer,
physical lookup maintenance, Q,S,D,P,K and ordered snapshot publication. No
later registration transaction may fill a custody/index gap.

Artifact read-work retains its metadata-only boundary: a future adapter cannot
copy/hash/materialize evaluation content to discover these relations before the
proven B debit. The ordinary metadata/search index and a decoded test fixture
are not qualified projections for that purpose. Publication-time source validation
retains its separate owner rules and grants no uncharged read-request cache.
Missing/corrupt/extra/ambiguous membership fences the profile; this audit selects
no durable rebuild, missing-row repair, implicit migration or authority restoration.

### Bounded next handoff

| Work boundary | Evidence required before claiming closure |
| --- | --- |
| Proposed non-I/O authority core | Obtain the scoped decision on #1372 before its concise execution plan. Synthetic claim/revoke/response tests would close only their stated software-ordering invariant; no registration, source/index, lifecycle or native adoption follows |
| Actual source-owner participation | Name a supported retained T/P prestate and each real preparation/result/approval/control/runtime publisher plus its ordinary-session bridge. Trace how source truth and original proofs enter and remain current at the same publication boundary; do not relabel a validator result or ordinary completion as that owner |
| Physical membership/profile design | Supply the closed catalog and complete bounded metadata-only forward/reverse/direct lookup representation, its original owning writes and refusal/recovery proof under one header. Review any new physical/compatibility choice before dependent implementation; 33 empty table names or one matching edge are not closure |
| Complete conditional command and native qualification | Proceed only when the applicable source/authority decisions and owners are closed. Independently prove full old/new rows, effects, counters, indexes, direct proofs and packing/denial; then use a separately approved bounded executable/native plan for actual lease/claim/retirement/durability and response disposition |

No eligible production owner or complete physical membership mechanism was
established by this bounded inspection. Gate 1's existing policy decisions stay
accepted; Gate 2 remains Investigate and Gates 3–4 remain gated. Keep #1176 open.
This audit changes no contract, format, codec, quota, authority or approval and
authorizes no dependent fixture, SQL/database, runtime route or native/model probe.
