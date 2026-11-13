---
type: Design Proposal
title: Original research preparation authorization and lineage
description: Proposed original research preparation cutoff, exact retained intent and control snapshots, and source-kind resolution through unchanged ordinary lineage with separate format and native gates.
tags: [orkworks, research, preparation, authority, lineage, qualification]
status: draft
workflow_status: proposed-for-scoped-research-authorization-review
---

# Original research preparation authorization and lineage

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `db51b1ff819594664428e41ef8085b75ecfecadc`.

**New policy and representation proposal for scoped owner review.** Earlier
research source/control/membership/initializer acceptance covers offline
components, not this research persistence exception or authorization codec.
Merge records a proposal. Gate 2 remains Investigate. No fixture, database,
runtime/native execution, model probe, migration, adapter or executable product
plan accompanies it.

## Result and unchanged boundaries

Propose a research-specific original-preparation branch of the accepted one-
preparation discipline: revoke-first denies; permit-first may save only the exact
original paired preparation after later volatile revocation, with no late spawn.
Select separate research intent/snapshots, source digest and authorization family.
Keep the ordinary lineage member's bytes/schema unchanged, resolving its original
source kind through the exact owning command/proof rather than a current role,
counter or whichever table happens to contain a matching row.

These are new research choices, not an implicit extension of the
[accepted direct-execution authorization](2026-11-05-child-preparation-authorization-design.md#scoped-representation-acceptance--2026-11-05).
Its body/domain, source kinds, snapshots, late-saving policy and format6 scope
remain unchanged. Preserve research plan/configuration/scope/reservation/
attachment/assignment and all report/request/head/receipt protocols. In particular,
this preparation policy is not the child report commit cutoff, #744 claim-first,
terminal UI correction authority, an artifact debit or a launch grant.

Reuse the accepted [paired broker/lineage direction](2026-11-05-joint-child-preparation-design.md#scoped-architecture-acceptance--2026-11-05)
as a constraint for further qualification. This proposal adds no connection,
worker, queue, bearer, allowance or independent writer. Actual source ownership,
source-admission machinery, physical paired atomicity, ordinary activation and
native/resource qualification remain missing evidence. RegistryRoot is neither
accepted nor required as an inferred implementation choice here.

## Source trace and the missing research seam

| Source / accepted contract | Observed responsibility / limit |
| --- | --- |
| [Original child preparation policy](../../validation/child-preparation-commit-authority.md#scoped-original-preparation-policy-acceptance--2026-11-05) | One eligible original preparation may save after later volatile close; stronger revoke-before-physical-commit denial was explicitly relaxed only in that scope |
| [Child authorization scope](2026-11-05-child-preparation-authorization-design.md#outcome-and-scope) | Direct-execution-only original allocated attempt; its producerSource and four snapshots name child families, not research |
| [Ordinary worker](../../../crates/orkworksd/src/session_store/worker.rs), ExecutionRequest::grant / ExecutionGrant::execute / MutationCancellation::cancel | Competes at the ordinary execution cutoff and releases application guards before Run; cancel false does not assert rollback. It is not research source permission |
| [Ordinary staged mutation](../../../crates/orkworksd/src/session_store/internal_mutations.rs), final ready_at / tx.commit | Checks readiness after staging before commit; this alone is not a research revocation/commit fence |
| [Hierarchy Connection::open](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs) | Refuses the unqualified profile before using the path; no source publisher is behind it |
| [Research initialization/catalog boundary](2026-11-13-research-initialization-catalog-design.md#concrete-blockers-to-final-catalogversion-selection) | Candidate names and fresh resource recipe do not supply research preparation decision evidence or ordinary source-owner selection |

A bounded search for research_preparation_authorizations,
ResearchOriginalPreparation, original-parent-research-source and
research-source-attach in the sidecar source and Electron main tree found no
implementation at this base. This is not proof that all differently named or
external writers were inventoried. The source reads above executed no database,
native process, model or application runtime.

Proposed documentary responsibilities, not files/APIs created by this handoff:
the research source/control owner derives and validates intent; the actual source
authority owner arbitrates permit/revocation; SessionStore's qualified broker owns
ordinary persistence; the hierarchy publication owner owns its complete logical
effects, resources and direct proofs. No owner may infer another's successful
outcome from a callback, matching digest or ordinary session label.

## New scoped research persistence policy

Cover only the first original, successfully allocated research attempt under its
still-current original parent runtime. The run is researching with the exact
active original plan and current fresh assessment/input; task is launching;
launch is allocated with attachmentId/spawnRuntime null and no interruption or
unknown outcome. The original child/session/attachment must be absent under full
retained/tombstone membership, not merely absent from the current session list.
No resumed/replacement-parent, recovery, parallel/reviewer or second-attempt
variant is normalized into this case.

Before mutation, the actual private owner may acquire one opaque, immutable
research original-preparation permit under the same short cutoff as every
applicable source revocation. It freezes permission only for the already prepared
complete ordinary/hierarchy intent. Revoke-first issues nothing; permit-first
allows that exact original attempt to reach proven commit/rollback after later
**volatile** close. Durable acceptance remains the paired physical commit.
This explicitly relaxes revoke-before-commit denial for this research preparation
only; it is a policy proposal, not evidence the stronger cutoff was implemented.

The complete read set still revalidates: hierarchy H, ordinary epoch/G, original
source/input/control/collection membership, freshness/health, leases, quotas and
all expected old/new facts. If a durable cancel/pause/input change advances H/G
or changes the captured facts, deny/roll back. Permit-first gives no priority
over a committed competing transition, no refreshed view or silent retry. Actual
lease loss or an unsupported/corrupt store still denies; a permit never keeps
physical ownership alive after it was actually lost.

Only the captured volatile authority/liveness decision is frozen for persistence.
Do not reapply that now-closed bearer/grant/runtime admission as a second permit
cutoff, or claim a permit-first save is guaranteed: independent source/native
facts, durable currentness, paired ownership and all transactional checks still
apply. Later capability issuance, registration, read/response delivery and spawn
always use their own current authority. Historical evidence reopens none of them.

Prebuild exact source/ordinary data, authorization bytes, charges, candidate
sequence and immutable complete effect plan outside the short latch. At the
cutoff compare-and-consume only that exact candidate under current original
authority; install immutable references/scalars only. No SQL/fsync, allocation,
parsing/copy/hash, callback or ordinary lock acquisition occurs under the latch.
Ordinary input/stop/end proceeds independently. A queued request, archived approval,
reservation, allocation descriptor or recorded RuntimeRef cannot issue a permit.

Share the existing sidecar-wide **preparation-permit** identity/sequence across
supported child and research original preparations. It remains distinct from
evaluation/debit/ordinary ticket identities despite a similar shape. Consume a
sequence once even if that permitted attempt later rolls back. Do not reset on
workspace/source-kind/namespace/resume, infer next from stored maxima, wrap,
restore a permit or create a second worker/active-or-retiring position. Qualified
fresh generation ownership and every exhaustion rule remain mandatory.

## Closed research intent and authorization body proposed

Use existing canonical UTF-8 JSON, unknown/duplicate-field rejection, ID/Digest,
full frozen ChildAssignmentIdentity P, ObjectRef, RuntimeRef, CommitRef and
canonical u64 Counter TEXT. Keep source JSON integer versions separate.
PreparationPermitIdentityV1 remains exactly `{sidecarGeneration,sequence}` with
positive sequence; this proposal changes no existing child identity bytes.

ResearchOriginalPreparationIntentV1 is exactly, with no optional fields:

```text
{workspaceId,storageEpoch,admissionEpoch,expectedCommit:CommitRef,
 ordinary:{databaseEpoch,historyGeneration,createdAt},
 assignmentIdentity:ChildAssignmentIdentity,
 authority:{kind:"original-parent-research-source",parentRuntime:RuntimeRef},
 attachment:{childSessionId,attachmentId,attachmentDigest},
 producerSource:ObjectRef,
 controls:{run:ResearchRunControlRead,plan:ResearchPlanControlRead,
           task:ResearchTaskControlRead,launch:ResearchLaunchControlRead}}
```

ordinary.databaseEpoch is the existing canonical lowercase UUID; historyGeneration
is canonical Counter TEXT 0..9223372036854775807. createdAt is the exact nonempty
ordinary creating string through 2 KiB, never normalized or treated as authorization
time. Preserve the independent 1 MiB ordinary record/joint-job limits.

Each Research*ControlRead is exactly `{table,key,value}` with value
`{schemaVersion:1,body:<accepted corresponding research control>,lastCommit}`:

| Read | Fixed table / exact key | Existing complete wrapper ceiling |
| --- | --- | ---: |
| run | research_run_controls / RunKey | 1 MiB |
| plan | research_plan_controls / PlanRef | 16 KiB |
| task | research_task_controls / TaskKey | 16 KiB |
| launch | research_launch_controls / TaskKey | 32 KiB |

These are complete original prestate snapshots, not poststate rows, quoted JSON,
arbitrary table/key bags or a checksum of the complete transaction read set.
They do not broaden the separate child/research recovery snapshot grammar.

ResearchOriginalPreparationAuthorizationV1 is exactly:

```text
{schemaVersion:1,permit:PreparationPermitIdentityV1,
 intent:ResearchOriginalPreparationIntentV1}
```

Its whole-body digest is SHA-256 over
`orkworks.research-preparation-authorization.v1` + one LF + canonical body.
No self-digest field or new C enters this body. It records nonsecret original
decision/source semantics, not a bearer, portable grant, wall-clock proof, SQL
log or full ordinary display snapshot. The actual private permit retains the
exact complete immutable execution plan; persistence of this body cannot restore it.

### Required original joins

| Binding | Required exact relation |
| --- | --- |
| Workspace/run | P, root/run/bootstrap and admitted namespace agree; expectedCommit.storageEpoch equals storageEpoch and resolves the exact captured H; ordinary epoch/G equal the paired captured ordinary header |
| Parent | parentRuntime is kind parent with sessionId=P.parentSessionId; equals run.parentRuntime and launch.admission.parentRuntime, has permit.sidecarGeneration and resolves original research runtime history. Actual still-current source authority is separately required |
| Source and attempt | Exact original input/assessment, plan/configuration/scope/captures/evidence, approval, reservation and verified allocation agree with P and all four snapshots. Read original reservation approval, not an arbitrary latestApproval or replacement plan |
| Freshness | Original plan remains the active current plan, assessment is fresh for its original input, no blocker/stage/clarification/pause or unsupported source change intervened. A stale copied assessment cannot authorize original preparation |
| Frozen/live versions | P.taskVersion stays the reservation/launch frozen assignmentTaskVersion. Captured live taskVersion remains its actual independently advanced value; no equality/reset to the frozen value is imposed |
| Intended new source | attachment childSessionId=P.childSessionId; ID/digest resolve the complete deterministic new ResearchAttachmentSourceV1. producerSource is exactly research-assignment-source / P and its unchanged whole-body research source digest. Do not use child-assignment-source |
| Ordinary identity | Exact ordinary id=P.childSessionId and original createdAt; complete full-key/session/attachment/permit/source absence/uniqueness includes tombstones across supported source families. A digest-only binding or existing plausible child cannot substitute |
| Original proof basis | Each snapshot source key/value and lastCommit resolves its original manifest after commitment at or before H in the same storage epoch; all source/root/input/runtime/approval/reservation/allocation direct proofs remain retained |

The deterministic attachment/source in intent are **new poststate bodies**, not
claimed existing rows. Validate their exact bytes before permission; after a
proven commit, resolve them from that original outcome. No model-provided source
kind, completeness flag, native identity or supplied timestamp selects authority.

For the accepted illustrative no-intervening-event trace, captured runVersion 7
and live taskVersion 4 precede prepared run 8/task 5; P remains frozen at 2.
These are examples, never admission defaults. The existing running/report prefix
views at run 10/11 and task 7 cannot stand in for allocated prestate. A future
component must build separate labelled preparation snapshots, preserving old bytes.

## One retained research authorization and original command

Propose one additional logical family, not a source ObjectRef kind:

| Family / exact key | Exact value |
| --- | --- |
| research_preparation_authorizations / full canonical P | `{body:ResearchOriginalPreparationAuthorizationV1,digest,lastCommit:CommitRef}` |

Require exact permit identity uniqueness across both child and research
authorization families, and unique original source ownership of P. One permit
cannot name two P values or source classes; one P cannot acquire two original
authorizations. Future derived contributor/identity lookups must compare complete
canonical identities and original bodies/proofs, never only hashes or table presence.
All historical rows, including tombstoned ordinary members, participate.

Use a new internal logical commandKind `research-source-attach`, claim null,
with the existing complete manifest shape. This is not a new public operation,
HTTP route, caller command string, retry token or #744 mutation claim. The
complete future catalog must explicitly recognize this source command and its
closed effects; no current code/tag factory is supplied. Existing child-source-
attach and its authorization remain unchanged.

Original preparation inserts the new authorization with original attachment/
research assignment, ordinary creating metadata/lineage, prepared launch/session
bridge, five fresh assignment zero rows, summary branch and every actual resource/
membership/index/proof effect. Run/workspace resources must already exist and
reconcile under the accepted initializer recipe. Do not initialize missing peers
or the run current owner here. Auth evidence adds no separate control-version
advance, subjectRevision, receipt slot, report, result, artifact or read debit.

## Byte-identical ordinary member and original source-kind resolution

Propose retaining ordinary schema2's existing member and projection exactly:

```text
orkworksHierarchyPreparation =
 {schemaVersion:1,assignmentIdentity:P,attachmentId,
  assignmentSourceDigest,preparationCommit:C}
ordinary_child_lineage[P] = {createdAt,linkage:<that complete member>}
```

For research, assignmentSourceDigest is the original research-assignment-source
digest. It is not the authorization digest, a child-source digest or a current
configuration checksum. Session id, creation identity, P, original attachment,
source digest and C agree with the complete paired original outcome. Labels,
ordinary revisions/status and later runtime generations cannot change them.
No agent, renderer, hook, imported JSON or generic ordinary mutation may set/adopt
this reserved member.

The new source-kind resolution recipe is exact and proof-owned:

1. The authorized original source intent already resolves its source profile from
   original approved research records. No caller picks a tag/table/kind to acquire
   a permit. Historical ordinary lookup starts with its exact retained P/C.
2. At that C, validate the original complete manifest under the **recognized full
   storage profile**, not a guessed version/domain. The original commandKind
   selects a fixed owner recipe: child-source-attach→existing child authorization/
   child-assignment-source; research-source-attach→new research authorization/
   research-assignment-source. An unknown command or missing proof denies linkage.
3. Resolve the exact selected authorization[P] and source[P]. Their intent, digest,
   attachment, lastCommit and original ordinary lineage join that same C. Check
   complete cross-family uniqueness; another family/row cannot repair a failed join.
4. Resolve historical snapshots/sources using their frozen original refs and direct
   proofs, not today's mutable rows, live task counter or parent generation. No
   recursive replay of every unrelated predecessor or largest-sequence election.

This is proposed linked-profile recognition, not an unversioned widening of old
readers. Preserve the member/schema2 bytes without adding a sourceKind field or
selecting ordinary schema3. Existing format6/schema2 linked readers cannot thereby
admit a research profile; they must refuse an unknown linked catalog/source owner.
No deployed eligible research/schema2 reader is assumed. Unrecognized legacy
members remain unverified data under existing policy, never adopted into lineage.
Actual ordinary producer/cutover, backup/recovery and existing-file pair checks
remain independently gated. No ordinary file or schema is changed here.

## Commitment, accounting and retained membership boundary

For the eventual reviewed profile, auth row committed projection is `{body,digest}`
under full P, excluding only outer lastCommit. Keep intent.expectedCommit and every
embedded old snapshot/source origin committed. Ordinary lineage keeps its existing
preparationCommit exclusion; validate its direct original C separately. Compute
source/attachment/auth bodies and charges before the complete new manifest C,
then attach only those named excluded C links. No hash fixed point or extra
exclusion is selected. Row/change/header domains wait for final format selection.

The new logical authorization increases the previous candidate inventory from
50 to **51 names**, conditionally; derived physical lookups are not new logical
families. This is still not the final full catalog or a maximum. No format7/next
number, header schemaVersion, migration, repair, domain conversion or optional
catalog inferred from rows is approved. Old format-3/4/6 sources/proofs cannot be
retokened or imported. Every original effect/proof/index/initializer must close
under the single final recognized profile before complete-state fixtures.

Charge the complete auth key plus committed value, including all copied snapshots
and old links, to the existing shared child-source-retained-bytes run/workspace
pools (8 MiB/64 MiB) and applicable 32 MiB run aggregate. Current controls retain
their own pool charges; no sharing discount or transfer. Coalesce each original-
to-final ledger delta once. The five new assignment rows stay zero, not proof
credits or borrowed space. Ordinary lineage/source copies retain their original
source ownership charges. No resource name or allowance is added.

The auth wrapper retains the inherited 2 MiB row ceiling, but ordinary 1 MiB
joint-job and all smaller source/worker/command/physical limits independently
win. A legal 1 MiB run snapshot plus other copied fields can exceed the joint
limit and must deny. No maximum-width/row/transaction fit is claimed. Preserve
4,096 observations/16 MiB cumulative discovery, complete-command 4,096 rows/16 MiB,
all actual physical/index/proof/store budgets and unchanged direct-proof retention.

Register the auth as a fixed typed historical metadata consumer of its complete
producer/attachment/plan/input/scope/configuration/approval/reservation/allocation/
root/runtime group, original H, four snapshot origins, ordinary lineage P/C and
new C. Extend logical control-origin targets to the four research control tables
**only inside these authorization snapshots**. This selects no research recovery
snapshot variant, arbitrary field path, ObjectRef purpose or independent graph.
Both lookup directions/permit uniqueness and exact source correspondence must
be maintained by the original outcome, including zero-reference owners and all
historical consumers. Physical projection/index changes remain a separate gate.

## Revocation, outcomes and recovery oracles

| Schedule / result | Required outcome |
| --- | --- |
| Source revoke/supersede before permit | No permit/backend/prepared child/authorization/spawn. Original reservation/allocation remain as their actual owner rules require |
| Permit before volatile close; unchanged H/G/read set | Only the exact complete paired original preparation may settle. Source/run/runtime/namespace authority stays closed; no startup future, SessionHandle, spawn, registration, grant or response commitment follows from that permit |
| Durable cancel/input/control/H/G drift or actual ownership loss | Deny/proven rollback; no refreshed snapshot/source/ordinary data or override of lost lease |
| Proven rollback | Complete old paired state, no new C/G/rows/charges/summary. Consumed sequence is not reused; no second SQL attempt or reservation under the permit |
| Commit, response lost or stale | Complete original data/lineage/auth/proof/charges retained. Fresh applicable lookup authority may inspect the original result; no new timestamp, permit, C/G, charge, grant or automatic spawn |
| Timeout/Drop/interrupt/disconnect/unknown commit | Retain the same immutable plan, sole active-or-retiring position, paired stores/leases/resources until proven disposition. No timeout reset, replacement worker or rollback assertion |
| Workspace/namespace switch while permitted | Detach publication immediately; old owner retains its captured pair through disposition, never publishes references into the new workspace. No new epoch/accounting reset around unknown old I/O |
| Missing/corrupt body/proof/index/peer, one-sided restore or duplicate P/permit | Unavailable; only supported original recovery, no row reconstruction, index rebuild, zero backfill or source-family fallback |

The complete prepared child replaces its unsettled reservation in derived
capacity once. Late save may consume retained source/control/summary bytes and
leave an ordinary creating/history record visible under existing rules, while
the child remains unstarted. Ordinary stop stays independent of SQL; original
worker/store/lease retention can be indefinite until disposition. Cancellation,
ordinary Delete, run completion or a lost response cannot prune connected
history/proofs, refund lifetime read work or make an unknown outcome settled.
There is no new standalone purge or orphan-adoption path.

A later legally eligible retry after proven rollback requires fresh current
original source authority and a new permit sequence for the same original
reservation/attempt. It is not an automatic retry, second native attempt or reuse
of the old permit. Unknown disposition cannot be relabelled rollback to enable it.

## Owner choices and bounded handoff

The three new choices are:

1. Extend the one-original-preparation late-saving/no-spawn policy to this exact
   original research scope, with explicit unchanged-read-set/ownership conditions
   and possible indefinite retained disposition. No report or other source action
   receives the exception.
2. The closed research intent/four-snapshot/body/domain and one P-keyed research
   authorization family/internal command, sharing the existing preparation permit
   sequence and source resource pools, with exact cross-family uniqueness and
   direct historical retention.
3. Byte-identical ordinary schema2 member/lineage, with original C/command-owned
   source-kind resolution and explicit linked-profile refusal of unknown research
   stores. No source-kind field, ordinary schema bump or implicit format6 reuse.

After scoped acceptance, only a bounded **offline component packet** may follow:
independent Python/JSON research allocated-prestate snapshots/intent/auth digest,
ordinary member/source-resolution examples, data-only JavaScript checks and
evidence/routing. Denials must cover wrong source family, live/frozen version and
snapshot joins, duplicate P/permit, copied origins, byte drift, unsupported source
kind, changed H/G, late-save versus spawn separation and lower wrapper/job limits.
Source/native/coverage/permit/physical outcome facts remain labelled assumptions;
no ordinary/hierarchy store, real cutoff, issued credential or legal history may
be inferred. Original-C/command-kind lookups are supplied synthetic facts, not
constructed manifests; the packet may hash the proposed authorization source
body but must generate no storage header, row/change hash or selected format.
Keep every earlier artifact unchanged. This is not a publisher,
runtime reducer, factory, route, schema/SQL/adapter, migration or executable plan.

Before full-state/history/proof fixtures or any implementation, close the complete
research catalog/version/initializer and source-to-index correspondence, actual
source/revocation/ordinary writer participation, metadata-only budgets, native
pair/lease/physical behavior and independently approved execution/activation gates.
Align any required ADR scope only after the concrete decision and before dependent
implementation. Existing child/ordinary architecture and policy acceptance are
not reopened. #1689/#1089 and broader component issues stay open; #740 stays closed
research. Gate 2 remains Investigate.

## Written review and verification disposition

One bounded round used two fresh read-only Luna reviewers for correctness and
builder/evidence completeness. Neither found a required correction or material
unanswered offline-packet question. They checked the new research policy scope,
immutable intent and four snapshots, frozen/live joins, shared sequence/P
uniqueness, original-command lineage resolution and rollback/unknown/late-save
consequences. They ran no fixture, database or runtime/native code. This is static
contract review, not proof of an actual source owner, cutoff, reader or paired
physical commit. Owner acceptance of the three new choices is still pending.

Pinned Node 22 docs verification passed all 1,662 existing tests, OKF metadata/
indexes and rendered-page/dead-link checks. All 20 OKF regressions and prior-byte
checks passed. All 553 previous non-Markdown validation artifacts are byte-identical;
the prior initializer/first-report technical bodies remain append-only. Existing
VitePress circular/large-chunk warnings concern unchanged bundler configuration.
Passing documentation checks supplies none of the open source/catalog/index/
authority/ordinary/native/history qualifications above.
