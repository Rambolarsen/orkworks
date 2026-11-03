---
type: Design Proposal
title: Retained child source and control consumer membership
description: Accepted scoped reference-discovery registry, historical retention and bounded coherent membership reads for the accepted child source/control slice; no storage implementation or complete-state qualification.
tags: [orkworks, hierarchy, child-sources, consumers, custody, storage]
status: draft
workflow_status: owner-accepted-component-qualification
---

# Retained child source and control consumer membership

Tracking: [#1183](https://github.com/Rambolarsen/orkworks/issues/1183),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspected base: `bce053da297cabc738ffc4a99b1b702bcd450665`.
Contracts: [accepted child sources](2026-10-31-child-review-source-preparation-design.md#consumer-membership-and-retention),
[accepted selected controls](2026-11-02-child-launch-control-design.md#scoped-owner-acceptance-and-component-handoff--2026-11-02),
[artifact custody](2026-10-30-evaluation-artifact-producer-design.md#custody-edges-and-registry-versioning),
[historical debit retention](2026-10-31-artifact-read-debit-proof-design.md#recovery-accounting-completeness-and-retention).
**Scoped choices accepted for bounded component qualification on 2026-11-03.**
No new table, database, DDL/migration, producer implementation, native/model probe
or current authority is supplied by this document. The original #1341 merge
recorded a proposal; the separate owner acceptance is recorded below.

## Decision and boundary

Close the selected logical reference registry and discovery protocol. Keep the
existing objects/dependencies representation, direct typed metadata references
and historical debit/proof membership distinct. Do not add a generic persisted
consumer graph, ObjectRef kind/purpose, source digest, reference-count quota,
consumer registration command or caller-supplied affected list.

Membership is derived from complete retained records under the storage owner.
The proposed lookup projections below are logical owner requirements, not an
extra authoritative table. Both lookup directions must be transactional and
complete; their physical representation and native query/recovery behavior need
separate adapter review and qualification. An incomplete index cannot be replaced
by a source checksum, a current-head scan or a claim that no consumer exists.

This does not select or approve the separate storage-kernel execution plan in
[#1337](https://github.com/Rambolarsen/orkworks/pull/1337). It adds no production
caller to that plan. No old store/catalog subset is upgraded. Only the one closed
fresh format-4 catalog can eventually admit this slice, with all required owners
present even when a particular table is empty. Unknown #743/proposal/learning
consumers remain unavailable; the registry is not a global absence proof.

Rejected alternatives: pinning only current plans; replacing metadata references
with invented ObjectRefs; counting identical source bytes once; persisting an
independent graph that can drift from its producer; repairing missing historical
edges during reload; and treating a retention edge as freshness or read authority.

## Exact logical lookup types

Reuse accepted canonical UTF-8 JSON, full ID/Digest/Counter/source-key shapes and
byte-equality keys. Every field below is exhaustive. These are internal logical
query/result types, not public messages, durable rows or new commitment domains.

ConsumerLocationV1 is exactly one of:

```text
{kind:"object",reference:ObjectRef}
{kind:"metadata",table:MetadataTable,key:ExactTableKey}
```

MetadataTable is closed to the families enumerated in the registry below.
ExactTableKey is that table's existing complete canonical logical key, never a
hash, selected subset, JSON string or delimiter-joined surrogate. An object
location includes its exact body-domain digest; metadata identity is its existing
full table/key and is resolved at the captured header, not by latest-row guessing.
Objects and dependencies are not admitted as arbitrary metadata locators.

ReferenceTargetV1 is exactly one of:

```text
{kind:"object",reference:ObjectRef}
{kind:"metadata",table:MetadataTable,key:ExactTableKey}
{kind:"control-origin",table:ControlSnapshotTable,key:ExactTableKey,
 rowDigest:Digest,origin:CommitRef}
{kind:"commit-proof",commit:CommitRef}
```

ControlSnapshotTable is only child_run_controls, child_task_controls,
child_launch_controls or child_session_states. control-origin resolves a complete
frozen snapshot in a retained RecoveryDecisionV1 plus its direct origin proof;
it does not require the current mutable row to retain the old body. Use the
accepted format-4 row projection to reconstruct rowDigest, excluding only the
snapshot's outer lastCommit. CommitRef and every nested historical ref otherwise
remain ordinary committed data. Exact original after commitment and workspace/
storage-epoch joins are required, not an unbounded predecessor replay.

A logical membership occurrence is exactly
`{consumer:ConsumerLocationV1,target:ReferenceTargetV1}`. Occurrences are derived
from the registry, never accepted from a caller. Full canonical tuples identify
lookup membership; duplicate tuples from different fields of one consumer may
collapse for graph reachability only. Preserve and charge every original field,
copy and physical-index occurrence. Existing dependencies still retain their
exact purpose in their own key; this view neither merges nor rewrites those rows.

Both forward lookup by complete consumer and reverse lookup by complete target
return the same derived membership at the same coherent header. For a current
metadata consumer, the read set also captures its complete value/row commitment
and original lastCommit. No consumer version is an active credential. During
historical validation use the frozen reference and original admission, not today's
runtime generation, head or live task version.

## Closed object dependency registry

Keep full existing ObjectRefs and original source domains. The following recipes
are exhaustive for the selected source/control profile. Reconstruct references
through their complete original task/run bindings before comparing endpoints.
A null optional reference creates no edge; an unknown variant is unavailable.

| Consumer | Required dependency occurrences and joins |
| --- | --- |
| child-source-plan | Existing child-source-provenance edges to its exact child-source-configuration and nonnull child-review-input. Reconstruct the configuration's full ConfigurationRef from the owning TaskKey plus the plan's configurationId/configurationDigest; preserve the original plan binding |
| child-source-configuration | Existing child-source-provenance edges to every exactly cited child-source-evidence object and nonnull child-review-input. Evidence IDs/digests/task equal the frozen capability references; no extra retained evidence is admitted |
| child-assignment-source | Existing child-source-provenance edges to its exact plan/configuration and nonnull input; all repeated reservation/frozen assignment/configuration joins remain required |
| child-review-input | Existing review-input edges to its exact original worker assignment-result and every selected assignment-evidence-source. The worker source/configuration and criteria/rubric are additionally protected by the typed source-group membership below |
| assignment-evidence-source | Its original closed report/source custody mapping protects the exact research-report version. Never follow citation text or URLs as new retained-object edges or fetch grants |
| assignment-evaluation | Existing evaluation-evidence edges to exactly cited evidence sources and the accepted evaluation-artifact-origin edge to its exact producer artifact. Preserve exact result/reviewer/evaluation identity and every historical revision |
| assignment-evaluation-artifact | Existing evaluation-artifact-content edge to the exact canonical evaluation supplying its bytes; immutable stream mapping/producer source are direct metadata/source-group joins below |
| assignment-result-candidate, assignment-result, assignment-disposition | Existing #744 subject-owned result/output/evaluation/predecessor/receipt/claim/proof joins remain one subject group. Do not fabricate cross-subject dependency purposes to represent those joins |
| child-source-evidence, research-report | No additional outgoing dependency is inferred from opaque fixture bytes, inline text, cited URLs or nested report prose. Existing typed source mappings remain required; an unsupported recognized consumer cannot be discarded as prose |

The two artifact custody edges deliberately form a cycle. Traverse with exact
visited keys and the bounded budget below; retain the whole protected component.
A cycle is neither corruption nor permission to omit an edge. It grants no
transitive content read. Source-provenance edges protect immutable history; they
do not automatically stale an evaluation or increment subjectRevision.

The existing proposal-input purpose is not permission to store a generic #743 or
learning proposal. No such object codec is selected here. A retained consumer
outside this closed registry blocks the affected operation until its owner closes
the exact codec, quotas and freshness/health effects. Do not assert that absent
implementation or an empty current selection proves absent retained consumers.

## Direct metadata and source-group registry

The storage owner resolves these joins without inventing ObjectRefs for metadata.
Object consumers also have direct source-group membership: each child-source-plan
resolves its original root preparation and exact prepared/initial input binding;
each child-source-configuration resolves its original plan/run; each
child-review-input resolves the worker's original child-assignment-source and
configuration through its exact ResultRef/AssignmentIdentity; and each
child-assignment-source resolves all its original approval/reservation/allocation/
attachment metadata. The exact assignment-evaluation-artifact/evaluation consumers
resolve their original producer/target assignment source and immutable stream
mapping. A dangling source-group join is unavailable even if all object edges fit.
Each listed family uses its already accepted exact key/value codec; no free-form
table/key pairing or field-path query is supported. Embedded full copies retain
all their typed references under the containing consumer as well.

| MetadataTable or source group | Complete required references |
| --- | --- |
| root_preparations | Its original bootstrap/run/approval/receipt and direct owning CommitRefs; the matching initial run_states, original reserved/attached parent identity and every exactly cited preparation_artifacts occurrence |
| run_states | Its immutable initial input/state/bootstrap/run identity resolves the same original root preparation. Later current state belongs only to child_run_controls; this seed cannot repair it |
| parent_attachments | Its exact original root/run/bootstrap identity resolves root_preparations. An attached current generation resolves child_runtime_attachments; reserved has no runtime ref. Replacing current generation preserves every referenced original runtime history |
| preparation_artifacts | The containing original root preparation and its exactly cited reference ID/raw content digest; identical bytes under distinct IDs remain separate occurrences |
| child_plan_approvals | Exact plan, original approval request/source digest and receipt's direct owning proof. Keep every historical approval; an old retained approval is no grant |
| child_reservations | Its exact TaskKey/plan/configuration/approval and frozen resulting assignmentTaskVersion; original reserved childSessionId remains unique. Resolve the original approval row, not the plan control's latestApproval |
| child_allocations | Its original reservation and plan/group; allocated descriptors never replace the original source worktree. Current phase cannot erase the immutable allocation |
| child_attachments | Original reservation/allocation, complete frozen AssignmentIdentity and unique childSessionId/attachmentId; protect its child-assignment-source and prepared subject group |
| child_control_inputs | Every InputRecordV1: configuration→child-source-configuration; review-input→child-review-input; result→assignment-result; research-report→the exact assignment-evidence-source/report mapping; prior-plan→child-source-plan. Its producer RuntimeRef resolves the original runtime history, and goal/bootstrap/run joins resolve root_preparations |
| child_runtime_attachments | Parent variant resolves its original root/run/bootstrap and exact parent identity; child variant resolves the original attachment/reservation/assignment-source. Reserved/proposed generations never create this row |
| child_run_controls | Original root/run/bootstrap/repository/source-worktree bindings; exact currentInput; all ordered planReferences including current/pending; parentRuntime; all nonnull blocker TaskKeys. Every referenced task/control belongs to this run |
| child_plan_controls | Exact immutable plan, task and nullable latestApproval/replaces. replaces must be the original prior-plan entry in the plan's exact prepared input. Retain the original prepared input, not today's currentInput |
| child_task_controls | Exact immutable TaskKey/plan/configuration, nullable original reservation and coordinationResult's exact original coordination receipt/requestDigest. Live taskVersion never rewrites reservation identity |
| child_launch_controls | Exact original reservation; admission approval and parentRuntime; nullable original allocation/attachment; every established original spawn runtime. A proposed spawn-pending generation has no runtime-history target until successful original spawn is proven |
| child_session_states | Exact original reservation/attachment/assignment-source/session identity and nullable current RuntimeRef. Ordinary resume may change that current ref without deleting or retargeting original spawn history or coordination results |
| child_coordination_receipts | Exact original task/reservation/session, canonical request/outcome and original response/commit proof. Source-group retention does not convert parent coordination into quality or release child capacity |
| child_recovery_decisions | Exact original task/reservation/allocation/attachment plus every typed reference in the frozen run/task/launch/session snapshots. Preserve each control-origin, readSet.expectedCommit, each direct original row proof and the new recovery receipt proof. A read-only captured row needs no link-only rewrite |
| evaluation_artifact_streams | Exact producer assignment/source/configuration/declaration and target ResultRef; all retained contiguous artifact versions, evaluation versions and their original receipt/claim/proofs. No max-version or artifact-ID-only reconstruction |
| artifact_read_debits | Exact intent.producerSource, artifact, resolved evaluation, immutable stream mapping, original producer/target source groups, three historical owner-ledger validation joins and the direct expectedCommit/new-commit proofs. Preserve its complete full-key row and original admission; never require old generations to equal the current process |

A child PlanRef source group also retains the complete immutable plan/configuration/
input/evidence bytes, every original approval/reservation/allocation/attachment,
original root preparation and all runtime/control/input/history references above.
The group is derived from exact PlanRefs and original identity joins, not a new
stored group row or a transitive read grant. One task per selected plan does not
allow the run owner to forget old plans, old ordinary resumes or failed attempts.

Artifact_read_debits remains a direct historical accounting consumer. Its accepted
command still changes exactly one debit row, three ledgers, header and new proof;
no dependency/index logical row, subject revision, source or old proof is mutated.
Any physical lookup maintenance must be part of those same owning writes, not an
extra logical manifest entry or later registration transaction. If the adapter
cannot supply complete metadata-only lookup within that recipe, deny this profile
rather than amend the six-row command here.

### Existing subject, proof and control families

The selected profile also retains the existing exact codecs for heads, payloads,
summary_state, pending_material, receipt_slots, candidate_work, quota_usage,
claim_proofs, mutation_lookup, commit_proof and store_header. These are the
remaining allowed MetadataTable variants. Their key/value variants are exactly
[#744 storage](2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation),
[compact proof/slot/lookup](2026-10-25-hierarchy-compact-bindings-design.md#three-closed-claimed-mutation-metadata-rows),
[slot/finalization](2026-10-25-hierarchy-slot-finalization-design.md) and
[candidate effect](2026-10-25-hierarchy-candidate-command-manifests-design.md#exact-state-lookup-and-index-contract)
contracts. The following reference recipes are exhaustive for those variants;
an unclosed row codec remains unavailable rather than becoming a generic bag.

Define SubjectSource(A) by exact AssignmentIdentity: root resolves its original
root_preparations key/bootstrap/root identity; child resolves its complete
child-assignment-source and original attachment/reservation/configuration group.
Define MaterialRefs(M) only for the exact accepted
[SummaryMaterialV1](2026-10-20-hierarchy-read-recovery-design.md#durable-summary-material):
SubjectSource(M.assignmentIdentity), its nonnull accepted ResultRef when
accepted.kind=result, every child reviewerIdentity in requiredFindings through
SubjectSource, and approved subject criteria/rubric bytes matching its two snapshot
digests. accepted.kind=none contributes no result. Required outcome tags, finding
IDs/text/locations and user review IDs do not invent ObjectRefs. Exact evaluation
provenance remains in the result's retained stream/subject group, outside material.

| Metadata family / existing key | Exhaustive non-link reference recipe |
| --- | --- |
| heads / exact tagged stream key | target ObjectRef plus SubjectSource for every assignment identity in the stream key. The target must reconstruct that exact stream; no head election or pointer retargeting |
| payloads / `(AssignmentIdentity,resultRevision,declarationId)` | SubjectSource, its approved declaration and the unique retained result for that exact assignment/revision whose output entry names this declaration/content digest. Resolve the full ResultRef from that original result; an ambiguous/missing row is unavailable, not a supplied digest |
| summary_state / AssignmentIdentity | SubjectSource; MaterialRefs(row.state.material) only for current nonnull row; MaterialRefs(checkpoint.material) and checkpoint.applied's direct proof when checkpoint is present. Unavailable/never-initialized variants contribute no invented material or missing checkpoint |
| pending_material / `(AssignmentIdentity,assignedRevision)` | SubjectSource; MaterialRefs(before) and MaterialRefs(after); the exact checkpoint/ordered pending lineage for that subject and its direct origin proof. Both material copies are retained and charged |
| receipt_slots / full `(AssignmentIdentity,slotId)` | SubjectSource and unchanged purpose/result/reviewer binding under the accepted reserved/finalization slot variant. For consumed claimed mutations, resolve receipt.claim to claim_proofs, expand its local identities/record/precondition, and decode responseBase64 under the original strict mutation-receipt codec. For candidate-finalization, resolve its exact candidate and nonnull accepted result from the distinct original response. No producer claim is fabricated for that path |
| candidate_work / exact candidate ObjectRef | Candidate object, SubjectSource and nonnull outcomeReceipt's exact subject/slot key. Accepted/rejected/superseded/abandoned states preserve original candidate/ordinal/receipt ownership; they do not name a newer result |
| quota_usage / exact owner/resource | Assignment owner→SubjectSource; run owner→that exact root_preparations; workspace owner→same initialized workspace/epoch. No unrelated subject edge is emitted by a shared ledger. Its current owning proof remains required |
| claim_proofs / ClaimIdentity | Expand the one/two assignments under ClaimProofV2's closed local-reference rules. Retain SubjectSource for each identity, producer authority's exact historical runtime when applicable, record's exact candidate/evaluation/disposition object and its corresponding consumed receipt slot/lookup. Precondition result-predecessor.kind=revision retains the subject's exact preceding result; evaluation-predecessor retains its full result, child reviewer source when present, and preceding evaluation when predecessor.kind=revision; user-slot retains its result; evaluation-head retains its exact evaluation. no-head variants add no predecessor. Decision generations remain original provenance, never current authority |
| mutation_lookup / RetryKeyDigestV1 | Its exact claim proof and full subject/slot key derived from assignments[0]/slotId; original full retry-key/response/source validation joins must agree. No alias or additional scope is inferred |
| commit_proof / positive commitSequence | Nonnull manifest.claim resolves its exact original claim proof. Directly validate each required retained origin row/snapshot commitment against changes, but do not turn every changes key/digest into a surviving-row obligation. A changed/deleted historical row need not still exist merely because its manifest remains. predecessor is checked where the owning command requires the expected/predecessor proof; it is not automatically traversed or retained recursively |
| store_header / singleton 1 | latest resolves its exact direct commit_proof in the same workspace/epoch. It grants no source/body adoption or maximum-sequence reconstruction |

Every table's actual outer lastCommit/commit/origin and the explicit checkpoint/
receipt links retain their direct owning proofs with the accepted exclusions;
no missing link is initialized by this recipe. Historical debit usedBefore values
validate the original before/after quota projections through its direct manifest
joins, not today's mutable quota values or a fabricated historical ledger row.
An index must support the above reference recipes in both directions; calling
them one subject does not permit an orphan claim, lookup or payload.

A dependency row owns the exact consumer/target/purpose tuple; its ObjectRef
endpoints and owning proof remain retained. It is indexed by both full endpoints,
not accepted through arbitrary MetadataTable/dependencies locators. Objects use
object locations. root_drafts is not a retained child source consumer: the root
producer consumes its exact draft at admission and retains the complete original
approved source. A child source never pins or reconstructs a newer workspace draft.
Existing standalone root-draft behavior is unchanged and outside these locators.

Direct proof retention includes each retained row/receipt/checkpoint/transition's
owning proof and any explicitly required predecessor/expected proof. Validate full
CommitRef, manifest row key and before/after commitments. Do not recursively treat
every manifest predecessor as a new retained obligation: that would create the
unbounded chain expressly excluded by accepted debit/recovery contracts. Logical
proof queries never elect a header from maximum sequence or resurrect missing rows.

## Coherent discovery and publication

One storage owner binds discovery to the exact workspace/storage epoch and
store_header.latest. The command is constructed internally from validated source
intent and complete retained records. Models, renderers and requests cannot
supply the consumer set, an absence flag or a purported completeness digest.

1. Decode the supported closed catalog and validate the exact changed producer
   identities and original direct joins. Resolve forward dependency/metadata/
   historical memberships and reverse incoming memberships at the same header.
2. For every returned location, read its full owning record or verified metadata
   projection, validate its exact origin/body/key/ref joins and derive the same
   occurrence. Dangling, extra, missing, duplicated inconsistent or unrecognized
   index entries are unavailable. No partial result or pagination token is a
   complete consumer set; all continuation pages must finish within one admission.
3. Expand retention/proof/source groups separately from freshness/material effects.
   Build the existing complete Q,S,D,P,K affected union, coalescing identical
   changed row keys while charging every retained occurrence and all new copies.
   Preserve all historical consumers, not only heads or selected plans.
4. Stage bounded copying/parsing/hashing outside core authority locks. Under the
   source owner's separately qualified transactional/current-authority ordering,
   revalidate the captured header, complete membership and source/lifecycle/health/
   lease conditions. Recompute the complete old-to-final membership changes from
   actual old/new records; revalidate the whole read set, not only changed rows.
5. Commit the producer, existing required dependency effects, direct physical
   lookup maintenance, quotas, summary/lineage/fences, header and proof together.
   Any changed header/membership denies that attempt; do not patch its captured
   facts, split publication or queue a second registration transaction. Existing
   exact authenticated retries return their original result without new edges,
   revisions, reservations, index rows, charges or grants.

The physical owner must prove both index directions are complete from the closed
source catalog before enabling dependent operations. A matching returned record
proves only that occurrence; it cannot prove no other incoming consumer exists.
Reload verifies complete correspondence with retained owning rows under bounded
private recovery. A missing or corrupt index fences the store; this proposal
permits no durable graph rebuild, row insertion, missing-zero repair or authority
restoration during read/recovery. A future explicit recovery mechanism needs its
own reviewed disposition and resource proof.

Artifact precharge retains its metadata/content boundary. Do not materialize,
copy or hash canonical evaluation content to derive memberships before a proven
B debit. The adapter must expose qualified coherent metadata-only indexes/joins;
this proposal supplies no such physical projection or permission to read content
early. Source validation at other lifecycle phases keeps its own bounded owner
rules and cannot be used as an uncharged read-request cache.

## Freshness, retention and ordering outcomes

| Event | Required membership/result |
| --- | --- |
| Proposal/configuration/input publication | Retain exact immutable objects and all prescribed edges/direct memberships in the original source transaction, before approval/allocation; no fabricated reviewer session is needed |
| Approval, reservation, attachment or ordinary resume | Extend or replace only the actual producer's typed references atomically; preserve original immutable sources/attempt/runtime history. No gap where a protected target is unpinned |
| New result/report/evaluation head | Keep old bytes/edges and use existing exact cited-outcome freshness rules. Only owning #744/#742 material changes advance subjectRevision/summary lineage; detail-only source/snapshot invalidation does not |
| Artifact correction or debit | Preserve every historical artifact/evaluation/mapping/debit consumer. Add only the original command's owning effects; no retargeting, new grant, freshness event or B refund |
| Parent/child end, plan cancellation/supersession or recovery | Revoke applicable volatile authority independently; retain historical source/proof consumers. Ordinary stop cannot wait for discovery, SQL or index recovery |
| Native cleanup, forget, purge or workspace deletion | This selected no-purge profile authorizes no connected source/artifact/control deletion. Check complete incoming custody and existing #745/shared-learning fences before any separately approved future removal; an unknown consumer denies |

A report correction stales only outcomes citing the changed evidence under the
accepted rules; an unrelated criterion failure stays intact. Retention closure
is not a blanket result invalidation or summary event. Snapshot invalidation is
published after the durable outcome under the existing ordered fence contract;
best-effort notification cannot be the only protection against old passing data.
A source transaction unable to publish required durable effects must deny/fence,
not commit a subset and rely on a later volatile notification.

## Proposed finite discovery profile

These are new stricter logical discovery choices for scoped acceptance, not
claims that existing defaults already select or implement them:

- At most **4,096 logical record observations or unsuccessful exact-key probes**
  in one discovery/admission, across all forward/reverse/source/proof expansions.
  One independently returned membership/index occurrence counts once; reading
  its owning row/projection counts once more. An exact-key lookup that returns
  one owning record counts once, not an additional success-probe charge. Each
  separately returned joined record/projection and every failed exact-key seek
  counts; a multi-row join is never one observation. Each pagination/index page
  costs one observation in addition to its entries; a separate end-of-range or
  empty-range probe costs one. Re-reading the same record, validating a second
  returned index occurrence or resuming another page consumes another observation.
  A repeated in-memory computation over an already captured immutable value adds
  no observation; any additional owned canonical copy still consumes bytes.
  Deduplicate the reachability frontier by full canonical identity to terminate cycles; this does not discount
  actual query/validation work. Size+1/exhaustion detection must itself fit the
  budget; reaching the limit without proving completion denies.
- At most **16 MiB cumulative canonical discovery bytes**: complete keys plus
  values/projections actually observed, every repeated copy and each unsuccessful
  probe key. Check before admitting another owned buffer and stop at the lower
  limit. No content body may be called metadata to evade precharge or body caps.
  Physical/allocator/query scratch and all request/staging copies also obey their
  existing smaller pools; this is no additional memory allowance.
- Existing complete-command **4,096 changed rows / 16 MiB**, row/source/subject/
  summary limits, source/control pools, artifact 4 MiB materializer, global worker/
  queue, main-file/journal/page and adapter limits apply independently. Reading
  4,096 rows does not authorize changing 4,096 additional rows or borrowing space.

No persistent consumer resource, new quota row or charge discount is introduced.
Logical lookup projections count their actual physical index/command/store costs.
Existing source/control metadata is charged to its already selected pools; a
reference in a frozen recovery copy is charged again as part of that copy.
Exactly 4,096 observations or 16 MiB may be admissible only when completion is
already proven and every other bound fits. The first additional observation/byte
is denied before publication. A bounded result set alone does not bound SQLite
VM work, pages or native cancellation; supported query plans and actual physical
limits remain separately unqualified. Do not promise all declared maxima fit.

## Owner decisions and bounded handoff

The scoped accepted package is: the exact typed lookup
locations/targets and closed registry; forward/reverse metadata/proof projections
owned atomically by the existing rows without a new logical graph table; coherent
header-bound complete discovery with no read-time repair; and the stricter
4,096-observation / 16 MiB cumulative discovery profile. Existing source/control/
artifact/debit fields, domains, six-row debit effects and no-purge pools do not
need repeated approval. This proposal changes no public transport or authority.

After acceptance, independently derive membership sets from the accepted source/
control packet, pin all selected forward/reverse/proof/debit cases and missing/
extra/historical/overflow denials. Keep those components explicitly synthetic.
Actual complete physical indexes, producer and ordinary lifecycle publication,
source-owned currentness/absence, authentication/revocation/native ordering and
complete consumer/CAS/full-state/packing witnesses remain separate qualification
work. Acceptance permits the bounded component packet below, not complete-state
fixtures or executable implementation.
#1183/#1176 remain open, and #740 remains closed.

Review routing: root owns all edits; one read-only Luna correctness reviewer
checks registry/retention/effect compatibility, and one independent read-only Luna
builder reviewer checks closed types, missing decisions and bounded handoff. One
round, at most three active agents including root; no writers or probes. Risk:
dependencies 4, blast radius 4, state changes 2 (proposed lookup/discovery contract,
no executable schema), reversibility Unknown, uncertainty 4. Sum incomplete;
readiness is written owner review only, not complete storage/source qualification.


## Bounded review disposition

One read-only correctness reviewer found no material conflict with the accepted
source/control/debit rules. The independent builder reviewer identified two
important ambiguities: the inherited #744 metadata-family reference inventory
and the logical observation unit. The author added the exhaustive per-family
recipes above (including compact proof/precondition/receipt expansion, exact
summary-material references and bounded direct proof retention), and defined
separate returned occurrence/owning record/page/miss observations. These are
author-applied clarifications from one round, not an independent second post-edit
review. They were part of the registry/budget package subsequently accepted below; actual
physical index completeness, metadata/content separation and native behavior are
UNVERIFIED. No extra reader schema or persisted graph was introduced.


## Scoped owner acceptance and component handoff — 2026-11-03

After [#1341](https://github.com/Rambolarsen/orkworks/pull/1341), reviewed head
`f00c26a6bc0619129b562ca4ea243712aa33eb2d`, the owner answered “go ahead” to
the exact approval request for this registry, logical lookup ownership and
4,096-observation / 16 MiB cumulative discovery limits before bounded membership
fixtures. This accepts the closed typed locations/targets/reference recipes,
coherent forward/reverse physical lookup ownership without a new logical graph,
header-bound complete discovery with no read-time repair and the explicit budget
units. Unchanged source/control/debit fields, pools and effects need no repeated
approval. The separate #1337 storage-kernel plan is not adopted or approved.

The [membership component packet](../../validation/child-membership-components.md)
pins selected source/control, historical control-origin/proof, reciprocal artifact
and debit retention relations, plus conditional discovery traces. Its alternative
cases are not one coherent store. It supplies no actual physical-index completeness,
metadata-only precharge, proof/native/CAS/producer authority or source-valid full
transaction. Existing subject/receipt/claim/summary storage and complete owner
qualification remain independent gates; #1183/#1176 stay open, #740 stays closed.
