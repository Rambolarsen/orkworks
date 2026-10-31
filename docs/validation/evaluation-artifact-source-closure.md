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
