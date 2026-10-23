---
type: Design Proposal
title: Hierarchy coherent reads, summary recovery and detail codec
description: Proposed publication boundary, durable summary lineage and exact-byte evaluation inspection for hierarchy consumers.
tags: [orkworks, hierarchy, recovery, transport, review]
status: draft
---

# Hierarchy coherent reads, summary recovery and detail codec

Tracking: [#746](https://github.com/Rambolarsen/orkworks/issues/746), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Status: the four recovery-contract choices have owner acceptance recorded below;
producer integration and implementation qualification remain pending. This refines the
[transport](2026-10-17-hierarchy-projection-transport-design.md) and
[payload](2026-10-17-hierarchy-projection-payload-design.md) proposals; it accepts
neither those drafts nor runtime implementation. #740 evidence remains deferred
until November and all role profiles remain unverified/no-go.

## Owner acceptance — 2026-10-23

The owner [accepted the four reviewed recovery-contract choices](https://github.com/Rambolarsen/orkworks/issues/746#issuecomment-6367630698)
after reviewing this proposal at main
`e40889d7552d90c4c8d7bd3aad677f26fd3298c9` against #744's accepted evaluation
behavior and the report, telemetry and projection contracts. The focused review
found no corrections needed to those choices. The owner explicitly replied
“yes, go ahead” to:

- Consistent reads: source changes and dependent summaries publish together;
  incomplete recovery shows unavailable.
- Durable summary history: committed material changes advance the revision,
  including pass → unassessed → pass. Optional-only details and time/cost changes
  do not advance it.
- Bounded storage: 2 MiB per summary material/row, 4,096 summary rows and 256 MiB
  aggregate serialized summary rows per workspace. Exhaustion preserves history
  and makes affected summaries unavailable.
- Narrow inspection: exact result/evaluation records with current/historical
  labels and lossless integer handling. Inspection grants no action authority.

This accepts those written choices, not the full upstream interaction/projection
contracts or a storage implementation. Concrete writer/producer mappings,
transaction integration, storage/durability qualification and crash fixtures,
shared-learning custody and separately approved runtime plans remain gates.
#740 evidence stays deferred until November; no role eligibility, runtime code
or launch is authorized. Keep #746 open.

### Owner confirmation of capacity and recovery tradeoffs

In a subsequent review at main `ff345d5c75b0c259f7c6352ec254e1d57f9959c5`,
the owner [explicitly confirmed the capacity and recovery tradeoffs](https://github.com/Rambolarsen/orkworks/issues/746#issuecomment-6368609380):

- A subject admitted without capacity for its initial summary remains summary-
  unavailable; a later read cannot invent its missing revision history.
- Under summary-capacity exhaustion, source corrections may continue only with
  durable protection that marks affected summaries unavailable. If that protection
  cannot commit, reject the transaction.
  Exhaustion cannot evict history or leave an old pass current.
- Crash recovery requires proven ordered history. Pass → Unassessed → Pass counts
  twice; optional-only changes do not advance the material revision.
- Detail inspection verifies the exact selected version, identity and digest,
  preserving lossless integers. Stale selections require a fresh overview and
  explicit reselection, without implicit retargeting.

This confirms the written behavior, including the capacity tradeoff. The producer,
transaction-integration, storage/crash, shared-custody and runtime-plan gates above
remain open; this confirmation supplies no implementation or launch authority.

## Decisions and boundaries

Propose one workspace publication boundary for source revision capture, dependent
freshness, summary lineage and projection invalidation. Keep filesystem copying
and validation outside its short critical section. Use recoverable source-owner
transactions with durable summary transitions; notifications never commit state.
Use exact canonical bytes for evaluation-detail inspection, with lossless parsing
and an explicit read-time freshness annotation. Neither a detail read nor a
summary-viewed token authorizes a mutation.

Alternatives considered: independent event-driven caches cannot prove that a
correction invalidated every dependent pass; recomputing summary tokens from
current content loses correction round trips; parsing canonical records directly
with JavaScript Number loses u64 values. An engine-wide database migration is
not selected here. The runtime plan must identify an existing or new storage
implementation that satisfies this logical contract and qualify its durability.

This is workspace-local. Cross-workspace custody, shared learning admission,
workspace run discovery, arbitrary evidence-file retrieval, configuration/permission
inspection, raw skill content, and decision command schemas remain separately
gated. Initial detail support below is limited to immutable #744 result and
evaluation records. Unsupported detail kinds have no generic object/path fallback.

## Coherent publication and reads

The workspace boundary orders publication, not all ordinary session I/O. Every
writer of a dependency used by the overview must participate: admitted run/plan
membership and versions, allocation/recovery/capacity, canonical session lifecycle
and attention, runtime bindings, #742 report heads and dependency fences, #743
receipts/usage/currentness, and #744 result/evaluation/disposition/summary state.
Catalog display metadata copied into the overview participates too. A runtime
plan must inventory concrete write paths; an unintegrated writer blocks complete
projection delivery, rather than relying on eventual event arrival.

1. Prepare bounded immutable source objects outside the publication boundary.
   Validate them under their owning contract; do not expose tentative heads.
2. Under the boundary, recheck exact predecessors, dependency membership,
   authority and workspace deletion state. Durable transactions atomically commit
   source-head changes, incoming dependency changes, affected freshness fences,
   summary transitions or summary-unavailable fences, and retry receipts.
   #742's retry-before-head-CAS semantics remain unchanged. Publication becomes
   visible only after the recoverable commit is durable.
3. Before releasing the boundary, invalidate every affected tracked run's prior
   projection and advance its snapshot counter once for that committed visible
   transition. Ordinary in-memory session transitions use the same publication
   boundary without acquiring new persistence obligations. At counter exhaustion,
   retain the transport's terminal version-exhausted fence; source corrections
   remain permitted. Notification is best-effort after publication.
4. For a read, capture immutable source references plus versions and complete
   collection membership under the boundary. Capture source health and dependency
   fence states, not just referenced object hashes. Copy canonical session display
   values here; never retain a mutable session object for later serialization.
5. Assemble and validate outside the boundary within the existing two-attempt,
   five-second and 8 MiB overview limits. Reenter the boundary to verify that all
   captured membership, source-health and revision tokens still match, no recovery
   fence appeared, and the workspace/lifetime/lease remains current. Associate the
   complete bytes with that tracked snapshot version at this publication point.
   Changed inputs retry once within the original deadline or return reconciling.

The validation point is the read's linearization point. A later source mutation
may make its response stale while bytes are in flight; transport version/request
fences and command-time validation still apply. Returning a complete snapshot
never guarantees that it remains current until a user acts. Equal versions must
serialize identically, including all display ordering and source timestamps;
wall-clock request time cannot enter projection content.

A transaction touching many dependents must establish the full affected set
through the authoritative dependency index. It may durably mark them unavailable
and finish bounded recomputation later. It cannot acknowledge the correction
while leaving an affected old summary current. Never silently skip dependents
when a work/space budget is exhausted. Conservative run-wide reconciling is
allowed until exact affected membership is proven. This does not add a new
source correction or user approval operation.

## Producer reconciliation — 2026-10-23

This proposed reconciliation maps the producer contracts at
`b9bbce69a43a858ea9ba7cf5d89e965007dbee6e` to the accepted recovery choices.
It adds no canonical source fields, reporting operation, storage engine or
runtime authority. The existing [exact identity joins](2026-10-15-hierarchy-projection-mapping.md#source-ownership-and-exact-joins)
and [overview payload](2026-10-17-hierarchy-projection-payload-design.md)
remain the consumer shapes. Written producer agreement for the full
material/classification, budget, publication/recovery and detail scope remains
pending; this table is a reconciliation artifact, not that acceptance.

### Source ownership and publication participation

| Producer | Authoritative inputs | Publication responsibility |
| --- | --- | --- |
| #741 immutable configuration | Exact approved child configuration or root bootstrap, criteria/rubric snapshots and canonical serializer | Validate the existing assignment/configuration joins; never replace immutable bytes from a later plan or mutable catalog. Catalog labels copied into the overview participate separately in snapshot invalidation |
| #742 coordination and reports | Admitted run/plan membership, allocation/task/runtime bindings, capacity, blockers, clarification and exact report stream/head | Publish visible coordination changes under the common boundary. Report correction commits its descriptor/head, request receipt, dependent freshness fences and affected preparation changes together; dependent #744 material is either reconciled or fenced before the new head is exposed |
| #743 skill evidence | Exact child binding, evidence-store generation, event/replay receipts, aggregates, coverage and source health | Publish evidence and its deduplication/high-water/aggregate effects atomically or recover them before serving projection. Corrections, clear, retention and detected degradation invalidate affected visible skill data under the same boundary; no telemetry event directly changes #744 quality/completeness |
| #744 results and evaluation | Accepted result head, candidates/finalization dispositions, evaluation stream heads, user invalidations, source-currentness and dependency index | Publish authoritative aggregate state, retained references, summary material/lineage or unavailable fences with source/dependency changes. Include user-stream replacement, purge and detected source-health changes; events cannot substitute for committed currentness |
| Ordinary sessions and projection owner | Canonical session lifecycle/attention and copied display values; tracked run slots and snapshot versions | Capture values under the boundary and invalidate every affected tracked run before another complete read. Durable summary ownership remains #744; ordinary session transitions acquire no new persistence obligation |

These are logical participants, not new source record names or finalized Rust
interfaces. The runtime plan must identify concrete writers, locks, journals,
source-health transitions and restart reconciliation for every row. Retention or
clear is a writer even when it runs outside a reporting request. Current source
health, full membership and dependency fences must be captured/revalidated;
matching immutable object bytes alone do not prove a coherent read.

An exact report retry follows #742's receipt-before-head-CAS order; it cannot
advance either summary or snapshot again. #743 keeps its own replay, alias,
coverage and generation rules: a replay that changes no visible data does not
advance a snapshot, while a real visible coverage change does. Neither owner
may borrow the other's retry or authority rules. Parent/child reporting bearers
never authorize the Electron-only overview/detail read.

### Exact material and detail mappings

| Recovery value | Producer mapping and preservation rule |
| --- | --- |
| `assignmentIdentity` and `accepted` | Copy the complete #744 root/child identity; assemble `ResultRef` from that identity and the accepted `AssignmentResult.resultRevision` plus stored `resultDigest`. No accepted head maps to `kind: none`, never a candidate or latest timestamp |
| Snapshot digests | #744 derives criteria/rubric digests from the exact #741 approved assignment criteria and rubric, including before an accepted result exists. Require evaluation snapshot digests to match; missing source snapshots make material unavailable |
| `requiredOutcomes` | Select required criteria, every rubric dimension and rating from #744's authoritative aggregation of eligible current evaluations. Apply only the display classifications below; retain #744's unassessed/conflict and independent-failure precedence |
| `requiredFindings` | Select current `requiresRework` findings for the exact accepted result. Copy reviewer identity, finding ID/text/location/severity/targets and derive current/disputed state under #744; exclude stale/invalidated findings. Preserve exact revision/digest provenance in detail references, outside material |
| `summaryRevision` | #744-owned durable row for that complete assignment. Accepted result identity and material changes advance it; source-generation or optional-only changes alone do not. Missing lineage cannot be initialized by a read |
| Detail selector/bytes | Resolve only an exact overview `ResultRef` or #744 `EvaluationReference`. Return the original canonical `AssignmentResult` or `AssignmentEvaluation` bytes and verify its own domain digest, never a converted display DTO |
| Detail currentness | #744 validates result/evaluation heads, dispositions, eligibility and every cited source. Resolve report evidence through the exact #742 stream/head and original provenance, never a newer report substituted for the reference. Whole-record historical detail does not recompute the aggregate |

#743 telemetry is not an `EvidenceSource` variant in #744. Its delivery/usage
state cannot be used as a scoring shortcut, change a summary merely because a
badge changes, or populate root telemetry. An optional user assessment of a
reviewer's performance affects its own result only; it never changes the
original worker review's eligibility. These existing boundaries remain intact.

The 49-outcome bound derives from 32 criteria + 16 dimensions + one rating;
only required criteria are selected. The 512-finding ceiling derives from at
most 16 evaluation revisions per result and 32 findings per evaluation; current
eligible heads and narrower limits still govern inclusion. The 2 MiB row/material,
4,096-row and 256 MiB workspace limits are additional projection bounds, not
increases to #741/#742/#743/#744 source admission or byte limits. A source quota
failure still rejects that source transaction. Summary-capacity exhaustion alone
uses the summary-unavailable rules below; it grants no source-quota bypass.

### Cross-contract verification cases

These are synthetic review obligations, not executed runtime evidence:

| Source transition | Snapshot/detail result | Durable summary result |
| --- | --- | --- |
| #742 corrects a report cited only by an optional outcome | Snapshot reflects stale cited evidence; retained evaluation reference is unchanged and whole-record detail may be historical | Unchanged material keeps its revision; no required-result downgrade |
| #742 corrects required evidence while another uncontested failure remains | Fence/recompute affected dependencies; show authoritative Needs rework | Advance if required outcome/finding material changes even when overall stays Needs rework |
| Exact report retry after a newer correction | Return original receipt; current head and projection stay at newer state | No replayed transition or extra revision |
| #743 source-event alias replay with unchanged visible coverage | Preserve original event/count and current snapshot content | No material change |
| #743 coverage correction, trimming, evidence clear or degraded recovery | Coherently update/fence the affected skill display with existing provenance/unknown rules | Telemetry alone cannot change result material or score |
| #744 optional-only evaluation correction changes its digest | Overview/detail references change; old bytes remain historical | Unchanged material keeps its revision, including stable required-finding keys |
| #744 user invalidation/replacement changes a required finding or target outcome | Publish exact disposition/head and dependencies together | Count each committed material transition; no old passing summary during recovery |
| A source quota fails versus summary storage alone being full | Source rejection preserves its prior head; summary-only pressure cannot bypass source rules | Rejection creates no material transition; otherwise authorized source change may fence summary as specified below |
| Child resume changes runtime with accepted material unchanged | Invalidate runtime/action/skill joins; retained evaluation requires fresh authoritative validation | Preserve the revalidated durable revision; restore no reporting/approval grant |

Producer agreement must cover the entire shared boundary and all four accepted
choices, including these mappings, bounds and detail semantics. Concrete storage
qualification, complete writer inventory and crash/codec fixtures remain later
implementation-plan work; #740 eligibility and separate runtime approvals remain
closed gates. No shared-learning admission or custody protocol is supplied here.

## Durable summary material

[#744 owns the summary revision](2026-10-04-assignment-evaluation-design.md#hierarchy-summary-revision-ownership).
The following exact logical material is proposed for that owner; it is internal
projection metadata, outside every canonical source digest. All records reject
unknown/duplicate properties and use #741's canonical serializer and source types.

`SummaryMaterialV1` has exactly:

| Property | Shape and meaning |
| --- | --- |
| `schemaVersion` | JSON integer 1 |
| `assignmentIdentity` | Complete tagged #744 AssignmentIdentity |
| `accepted` | `{kind: "none"}` or `{kind: "result", reference: <payload ResultRef>}` for this exact assignment |
| `criteriaSnapshotDigest`, `rubricSnapshotDigest` | Exact approved #744 snapshot digests |
| `overall` | `meets-requirements`, `needs-rework`, or `unassessed`, derived by #744 |
| `requiredOutcomes` | Exactly every required criterion, every rubric dimension, and rating, sorted by canonical target bytes |
| `requiredFindings` | Every current required-rework finding, including disputed ones, sorted by canonical `(reviewerIdentity, findingId)` bytes |

An outcome has exactly `target` (#744 RequiredImpactTarget) and `disposition`
(`meets`, `needs-rework`, `unassessed`, or `conflicted`). These are display
classifications of #744's aggregate outcome, not another evaluation algorithm:
criterion satisfied / dimension meets / rating 3 maps to meets; established
unsatisfied / below-standard / rating 0..2 maps to needs-rework; unresolved
threshold/outcome conflict maps to conflicted; otherwise unassessed. The owning
aggregate's conflict and independent-failure precedence remains unchanged.
Missing approved criteria/rubric bytes make material unavailable, not an empty
array. No accepted result means required outcomes are unassessed, unless #744
independently establishes a current required failure; it cannot imply a pass.

A required finding has exactly `reviewerIdentity`, `findingId`, `description`,
`location`, `severity`, `requiredImpactTargets`, and `disposition` (`current` or
`disputed`). Text/targets copy the exact finding's bounded source values. Include
only findings #744 determines current for the accepted result; stale findings
remain detail history. A disputed finding cannot establish failure by itself.
The logical reviewer/finding key remains stable across an optional-only evaluation
correction; do not include that evaluation's revision/digest in material merely
because its unrelated optional content changed. Exact evaluation provenance is
still retained and retrievable through the source references in details.

Do not include optional outcomes/findings, rationales, evidence key lists,
observation times, cost/time, runtime generation, source revision tokens, or UI
view state in material. Their changes still invalidate the volatile snapshot and
detail freshness as appropriate. Required finding text/targets/disposition and
required outcome classification changes are material. This makes the proposed
boundary explicit for owner review; it does not silently amend #744's aggregate.

A durable `SummaryRowV1` has exactly `schemaVersion: 1`, `assignmentIdentity`,
`summaryRevision` (the existing canonical decimal string), and `state`, exactly
`{kind: "current", material: SummaryMaterialV1}` or
`{kind: "unavailable", reason: "reconciling" | "exhausted"}`. Nested assignment
identities must match. Source dependency/transaction receipts remain storage
metadata, outside material and source digest inputs. Compare canonical material
bytes, not a hash alone. Once initialized, a change A→B→A commits revisions
1→2→3. Same material with changed dependency tokens keeps the revision. Entering
a temporary reconciling fence does not itself consume a revision: recovery must
retain the preceding committed material and every pending transition in the
transaction evidence before replacing the row. Exhausted is terminal for this
assignment's summary lineage; never serve its former material as current.

### Bounds and admission

Propose at most 49 outcome entries, 512 required finding entries (16 evaluations
×32 findings; narrower source limits win), 2 MiB per material/row, 4,096 summary
rows and 256 MiB aggregate serialized summary rows per workspace. The row byte
cap includes duplicated assignment identity. Source quotas remain independent.
Reserve byte/count capacity before initializing a summary row atomically with a
new subject's first material. If capacity is unavailable, source admission may
continue with summary unavailable; a later read cannot initialize that already
existing subject. Capacity exhaustion must not erase or recreate a prior row.
An existing source subject without proven intact summary lineage remains
unavailable, even when its current outcome looks valid.

Before a material change exceeding its reserved storage, durably fence the
summary and publish the authorized source change under the common boundary.
Retain the old row and recoverable transitions; never keep its former pass current.
Reconciliation may republish a current row only from intact ordered transitions,
with their material changes counted exactly once. No automatic history eviction
creates room. Source mutation logs/checkpoints must have finite byte/work bounds
in the runtime plan; if durable fencing cannot be committed, the source transaction
must fail rather than publish unfenced new state. Source summary-counter exhaustion
alone does not reject an otherwise authorized correction.

## Crash recovery, purge and restart

| Interruption | Required recovery |
| --- | --- |
| Before durable source transaction commits | Expose complete old source/summary or reconciling; discard tentative publication |
| After durable commit, before row/checkpoint installation | Recover committed source, dependency fences, ordered summary transitions and receipt together; affected reads stay unavailable until replay completes |
| After installation, before acknowledgement/event | Retry resolves original receipt; replay cannot increment summary or snapshot a second time in the same lifetime |
| Two material corrections commit while summary is fenced | Retain both transitions; replay A→B→A as two increments, never compare only final A with old A |
| Summary row missing/corrupt, journal/checkpoint proves complete lineage | Recover exact preceding row and all committed transitions before current publication |
| Neither row nor complete lineage proof survives | Summary unavailable; no revision-1 reconstruction from latest result, timestamps or digest |
| Sidecar replacement | New volatile snapshot scope; recover unchanged durable summary revision before complete read; restore no launch/report authority |
| Summary revision maximum, then material correction | Commit source with terminal exhausted summary; no old current disposition, wrap, or restart reset |

The runtime plan must specify checksummed transaction identities, replay
idempotence, durable flush/rename requirements, bounded checkpoint/log rotation,
and crash-injection evidence for each claimed filesystem. A storage adapter that
cannot prove recoverable publication is unsupported for this slice. This proposal
does not claim protection against a same-user rollback of the entire registry.

Subject purge uses #744's dependency boundary and exact subjectRevision CAS.
Fence publication, prove no incoming retained dependency, and durably remove
source subject and summary row together before acknowledging purge. Retained
navigation cannot reinitialize the purged subject or reuse a viewed token.
Workspace deletion retains the existing #745 phases and fences through source
custody reconciliation; this workspace-local proposal supplies no cross-workspace
transfer protocol. Incomplete purge/deletion recovery remains unavailable, never
an empty successful run. View state is local presentation and never recovery proof.

## Exact result and evaluation detail reads

Logical `ReadHierarchyRecord` uses the same Electron-only authentication,
sender validation, selected workspace/lease/lifetime derivation, rate accounting,
and publication fence as the overview. Parent/child bearers do not authorize it.
Concrete route/preload names remain runtime-plan choices. Requests are strict:

- Renderer request: exactly `schemaVersion: 1`, `runId: ID`, `expectedSnapshotVersion: Counter`,
  and `record` (one of the tags below), at most 4 KiB UTF-8.
- Main adds exactly `workspaceId` and `sidecarGeneration` from its owned context;
  it never accepts a renderer-provided workspace, token, or path.
- `record` is `{kind: "result", reference: ResultRef}` or
  `{kind: "evaluation", reference: <#744 EvaluationReference>}`. The assignment
  must belong to the requested run and the reference must appear in the exact
  current validated overview. Otherwise fail stale/invalid without retargeting.

The 4 KiB bound accommodates a complete child evaluation and independent reviewer
identity; the overview's existing 1 KiB request cap is unchanged. Detail and
overview reads share 60 attempts/minute and two in-flight reads per workspace;
no second pool or unlimited recursive evidence fetch. Each detail read has the
same two-attempt/five-second assembly bound. Cancellation releases its slot.

Success is exactly `{schemaVersion: 1, workspaceId, runId, sidecarGeneration,
snapshotVersion, record, canonicalUtf8, freshness}`. `record` echoes the exact
request selector. `canonicalUtf8` is a JSON string containing the exact canonical
UTF-8 source object (without its domain prefix). `freshness` is exactly
`{kind: "current"}` or `{kind: "historical"}`. It describes the record as a
whole under #744's source-currentness rules; current requires every evidence
reference in the record, including outcome, rating, finding and finding-dispute
references, to resolve current sources, with no invalidation/supersession. Historical does not mean
the entire aggregate failed; optional-only stale evidence still follows #744.
Missing freshness proof is a request failure, never current. Display the immutable
record's claims with that annotation; do not derive current aggregate pass or
per-outcome freshness from it. The overview remains the authoritative aggregate.

At publication the snapshot must still equal expectedSnapshotVersion; otherwise
return a bounded request failure `stale-reference` and require a fresh overview
and explicit selection. No alternative result/reviewer/revision is substituted.
Other bounded failure codes are `unavailable`, `invalid-request`, `limit-exceeded`
and `denied`; denied reads reveal no record existence. A valid immutable object
with broken run/assignment ownership or unavailable dependency proof is not success.
Known unavailable summary lineage alone need not block an intact exact record read.
Late responses obey current workspace/run/lifetime/request/snapshot fences and
cannot repaint a new selection, clear workflow failure fences, or mark a summary
viewed. Record selection does not prove that a material summary was displayed.

The source record limit remains 512 KiB unescaped UTF-8. Bound the entire encoded
response to 4 MiB UTF-8, depth 32 for both outer and embedded record, checking
incrementally during decoding/serialization. JSON string escaping can expand
control bytes sixfold; reject over-budget responses without truncation. No
canonical object is parsed through JavaScript JSON.parse before lossless validation.
The outer envelope contains a string, so source integer tokens survive exactly;
the embedded parser must reject duplicate/unknown properties, invalid UTF-8,
noncanonical integer tokens and values beyond the source schema before producing
any display values. Never eval or render this string as markup.

For a result, verify SHA-256 of `orkworks.assignment-result.v1` plus LF plus
canonicalUtf8 bytes against resultDigest. For an evaluation, use
`orkworks.assignment-evaluation.v1` plus LF and evaluationDigest. Validate the
full strict #744 object, assignment/result/reviewer/revision identities and its
canonical serializer equality; a matching hash alone is insufficient. Convert
only explicitly mapped u64 display fields to decimal strings, preserving exact
values. Keep original bytes separately; never hash a converted display object.
Both Electron and renderer validate their independently defined boundary types.
This operation returns no raw output artifact, evidence body, instructions,
credential, filesystem capability, write token or current approval grant.

## Required fixtures and acceptance

These are synthetic acceptance cases for the later runtime plan, not claims of
executed crash tests or current support:

| Case | Expected result |
| --- | --- |
| Session attention changes between capture and final validation | Retry/reconciling; no mixed current No actions snapshot |
| Plan membership/dependency edge changes without changing object bytes | Membership token fails; no partial collection claim |
| Report correction commits but summary recomputation has not finished | Durable affected fence; no new head with current old pass |
| Crash at each durable transaction/row/checkpoint/receipt boundary | Complete old/new state or fenced reads; exact retry and replay apply once |
| Pass→Unassessed→pass while reader disconnected or summary fenced | Two summary increments; old viewed token cannot collapse restored pass |
| Optional-only evaluation correction changes its source digest | Snapshot/detail freshness advance as needed, summary revision unchanged |
| Required finding text/dispute or required aggregate classification changes | New material revision, exact source evaluation still inspectable |
| Row/workspace quota or maximum summary revision reached | Authorized source correction fences summary; no eviction, current old pass or reset |
| Purge races new incoming dependency or inspection | Same boundary orders protection/deletion; no dangling current summary |
| Record contains sizeBytes 9007199254740993 or maximum u64 | Original canonical token/digest preserved; display gets exact decimal string |
| Record bytes have duplicate keys, noncanonical escaping, wrong domain or foreign reviewer | Reject before display, even when JSON is parseable |
| Only finding/dispute evidence becomes stale, while outcome evidence stays current | Detail is historical; stale disputes are ineffective and stale findings cannot establish failure under #744 |
| Snapshot changes during detail read or old request arrives after selection change | Stale failure/drop; no implicit retarget, viewed token or authority |
| 512 KiB record dominated by escaped control text | Respect encoded 4 MiB cap and source text limits; no truncation or unbounded allocation |

Owner acceptance of the four choices is recorded above. Producer agreement
remains pending for the full set: exact material fields/classifications, finite
summary budgets, joint publication/recovery semantics and narrow detail scope,
including writer mappings and transaction integration. Reconcile #742/#743/#744
contracts and prepare separate runtime
plans identifying every writer, storage adapter and shared codec/crash fixture
when their upstream gates permit. Required native capability evidence,
architecture/ADR review and scoped implementation approval remain open. Keep
#746 open; recording accepted written choices does not establish runtime availability.
