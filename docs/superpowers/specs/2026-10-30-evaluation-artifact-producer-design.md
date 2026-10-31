---
type: Design Proposal
title: Immutable evaluation-artifact producer custody and publication
description: Proposed closed artifact metadata and stream mapping, shared canonical content custody, finite producer/read budgets and atomic evaluation publication effects.
tags: [orkworks, hierarchy, evaluation, artifacts, custody, publication]
status: draft
---

# Immutable evaluation-artifact producer custody and publication

Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176),
[#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746); initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `5caab58d` / PR #1177.
**Proposal for scoped owner review. Drafting or merging does not accept the new
codec, custody, numerical/read-work or capacity choices and authorizes no
dependent transaction fixture, adapter, migration, probe or runtime code.**

## Outcome, evidence and boundary

Close the artifact producer boundary exposed by the
[coherent conditional review history](../../validation/coherent-review-history.md).
That packet joins 2,956/3,190-byte review revisions and their receipts but has no
producer custody/index or complete publication transaction. Specify those
logical records, occurrence accounting and effect recipes, obtain one scoped
owner decision, then qualify independent complete-state/packing witnesses under
the separate source, authority and implementation gates.

Reuse the [evaluation source and reporting contract](2026-10-04-assignment-evaluation-design.md#declared-outputs-and-authorized-sources),
[typed review input and structured outputs](2026-10-04-agent-role-configuration-design.md#proposed-typed-reviewer-input--2026-10-24),
[accepted compact format 3](2026-10-25-hierarchy-compact-bindings-design.md#proposed-storage-format-3-and-compatibility),
[slot mapping](2026-10-24-hierarchy-storage-contract-design.md#source-registry-and-remaining-schema-work),
[aggregate source budgets](2026-10-29-assignment-aggregate-budgets-design.md#owner-acceptance-and-component-handoff--2026-10-30)
and [complete-command accounting](2026-10-25-hierarchy-candidate-command-manifests-design.md#complete-command-effect-recipes).
No source evaluation/request/receipt/read-reply field or domain changes.

The least-certain boundary was whether an artifact needed a new external blob
store. The existing storage contract permits canonical BLOB object bodies and
payload-style digest commitments. Propose instead to materialize artifact content
from the exact retained canonical evaluation body, with explicit reciprocal
custody. This avoids another content copy and its staging protocol. It is a new
custody choice, not an existing verified optimization or a storage discount.

The important consequence is retention: target evaluation history and reviewer
artifact history now protect each other. Do not disguise that cycle as safe
single-subject purge. This first profile deliberately provides no purge of those
connected subjects; saturation can require waiting for a separately reviewed
joint-custody deletion policy. That usability cost is a named owner decision.

## Supported producer and source bindings

Only a #741 output-bearing child v3 `review` or rubric-eligible `verification`
assignment with one required `assignment_evaluation` server-artifact declaration
and a nonnull exact `ReviewTargetInput` can use this profile. The target is an
accepted child result in the same workspace/registered repository; cross-run
review retains its exact target owner. Root and reviewer-performance child
targets, ordinary/legacy configurations, arbitrary uploads and other artifact
kinds are excluded. Existing #744 independence/contributor rules remain binding.

Before publication, resolve full retained approved plan/configuration bytes,
frozen reservation/attachment/task-version identity, tool/profile evidence,
requested/effective reporting actions, declaration, review target, criteria/rubric,
read scope, evidence sources and active reviewer authority generation. Check
bytes and source-owned digests, not a UI label or supplied boolean. Missing
source/admission/configuration/consumer codec is unavailable/no-go. This proposal
supplies none of those upstream records or #740 eligibility.

One producer AssignmentIdentity and declaration bind one target result and one
reviewer stream for their lifetime. Resume changes the active capability binding,
not stored assignment, target, declaration, stream, artifact ID or old versions.
A changed target/output contract/configuration needs a new reviewed assignment;
never retarget an existing mapping. No new bearer class, transport route, general
artifact listing or filesystem permission is introduced.

## Proposed closed records and content custody

Use existing ID/Digest/Counter encodings and #741 canonical UTF-8 serialization.
Unknown/duplicate properties, invalid Unicode/numbers, noncanonical stored bytes,
unsupported versions, ambiguous joins and unmatched old history are unavailable.
Content hashes are integrity evidence, not authentication or whole-store rollback
protection. All definitions below are new proposals requiring acceptance.

### Immutable artifact object

Add `assignment-evaluation-artifact` to the reviewed source ObjectKey registry.
Its identity is exactly `{producerAssignment, declarationId, artifactId, version}`:
producerAssignment is the complete child AssignmentIdentity; IDs use existing
bounds; version is integer 1..12 and equals the bound evaluation revision.

Its `EvaluationArtifactV1` body is exactly:

```text
{schemaVersion:1, producerAssignment, declarationId, artifactId, version,
 evaluation:EvaluationReference, sizeBytes:lossless JSON u64, contentDigest:Digest}
```

Hash the full canonical body with `orkworks.assignment-evaluation-artifact.v1`
plus one LF. Store in the existing objects envelope `{body,digest,lastCommit}`.
The body reconstructs every ObjectKey field; its evaluation reviewer is exactly
`{kind:"child",assignmentIdentity:producerAssignment}`. Its declaration and target
resolve the frozen producer configuration. Propose a 16 KiB maximum canonical
artifact metadata body; retain the existing separate 512 KiB evaluation-content
bound and all lower command/store/adapter limits.

`sizeBytes` and `contentDigest` describe the exact canonical UTF-8 bytes of the
referenced retained AssignmentEvaluation body, with raw SHA-256, not its
source-domain digest. Resolve the exact evaluation ObjectRef from the complete
EvaluationReference, verify its canonical bytes/domain digest and raw size/hash,
and return those same bytes in the existing artifact-read reply. No mutable file,
path, highest revision or receipt alone supplies content. This profile stores no
second content BLOB, Base64 copy, external file or generic upload row.

The source evaluation body is physically stored once, while its reviewer artifact
occurrence is charged fully below. If later accepted into the reviewer's own
result, finalization still creates the existing result/declaration-bound payload
occurrence and charges it separately; this proposal does not qualify that writer.

### Immutable producer stream mapping

Add one closed table, `evaluation_artifact_streams`. Its key is exactly
`{producerAssignment,declarationId}`. Its value is exactly
`{schemaVersion:1,artifactId,target:ResultRef,lastCommit}`. Propose an 8 KiB maximum
complete canonical key/value pair. Resolve full producer/declaration/target bytes;
the target's assignment is child, with the exact accepted revision/digest.

Create the mapping atomically with the stream's first evaluation/artifact. The
sidecar assigns artifactId; it is stable across revisions and resumed generations.
Its meaning is scoped to producer/declaration: an identical ID elsewhere cannot
alias a read because every operation already binds that producer/target/declaration.
No global ID-only lookup or reverse-index authority is introduced. A correction
verifies the same mapping and inserts only its new artifact version; it does not
rewrite an unchanged mapping or create a new ID.

Recover versions through the twelve possible version keys for this exact mapping,
not a maximum-ID election. Retained versions form the exact contiguous evaluation
chain; each joins its evaluation and original receipt/claim/proof. A mapping
without version 1, an artifact without a mapping, a hole, orphan body or missing
proof/owner ledger is unavailable. Never rebuild an ID/mapping from a surviving
receipt or adopt an unrelated artifact. An absent mapping is fresh only when
source-owned preparation and complete bounded history prove no prior publication.

### Custody edges and registry versioning

Add exactly two dependency purposes, with full ObjectRefs:

- `evaluation-artifact-content`: consumer is this artifact object; target is its
  exact canonical evaluation object. It protects the bytes used to materialize
  artifact content, including historical versions.
- `evaluation-artifact-origin`: consumer is that evaluation object; target is
  this artifact object. It protects the original artifact metadata/mapping joins
  required by the evaluation's reporting receipt and its immutable provenance.

Both edges are unique and inserted with each revision. Their endpoints,
reviewer/declaration/target/size/digests and original receipt must join exactly;
these purposes cannot connect arbitrary objects. No source field is added to an
evaluation. Mapping ownership follows the producer subject and its retained
artifact objects; it cannot be removed while any version or reference survives.

Propose a distinct **storage format 4**, reserved for this artifact-profile
contract, so profile selection is persisted rather than inferred from whether
new rows happen to exist. Store_header and every ChangeManifest have
schemaVersion 4, with otherwise unchanged closed field shapes. Use
`orkworks.hierarchy-storage-change.v4` and `orkworks.hierarchy-storage-row.v4`,
each followed by one LF. All CommitRefs resolve only through that format's
manifest/domain. Register the new object kind, mapping table, dependency purposes,
quota resources and internal read-work command in its closed catalog. Existing
source/request/receipt domains and ClaimProofV2 field/proof domain stay unchanged;
old format-3 headers, manifests, row digests and goldens are never rewritten.

The format-4 row projection uses the accepted format-3 field/link exclusions for
its unchanged rows. The new mapping row excludes only lastCommit; artifact objects
use the objects projection. Every other key/field remains committed. Every new
row and edge is in the complete manifest with normal header/proof equations.
The new header/new-proof control effects retain their separate validation, not
self-referential changes.

A reader supporting only format 3 rejects schemaVersion 4 before interpreting
rows, including an otherwise empty freshly initialized store. A format-4 reader
requires its entire reviewed table/purpose/object/resource catalog and the exact
source-owned fresh preparation state; it must not infer a base profile when an
extension table, required zero ledger or row is missing. Unknown tables including
empty tables, unsupported variants or mixed v3/v4 commitments are unavailable.
There are no alternate format-4 profile catalogs chosen from row presence.
Upstream missing source/consumer codecs must be closed and reconciled into this
same versioned catalog before any executable profile is eligible; adding a new
required kind later needs explicit version/compatibility review.

This is new-store/profile preparation only: no deployed store, automatic table
addition, old-artifact adoption, on-disk upgrade/downgrade, digest repair or
migration is authorized. Even logically identical old source bodies cannot be
silently imported as prior acceptance. Future compatibility/DDL, exact catalog
fixtures and fresh-store selection need an explicit reviewed plan.

## Proposed finite production, read and materialization budgets

These are author-proposed logical defaults, not measured requirements or native
capacity evidence. Ceilings are inclusive; MiB is 1,048,576 bytes. Each resource
has independent assignment, owner-run and workspace quotas; workspace totals
include retained old runs/epochs. Root/read-only user identities cannot borrow a
producer pool. Existing target record/payload/receipt and stricter global limits
still apply; no quota or store ceiling is raised.

| New resource | Producing assignment | Producing owner run | Workspace |
| --- | ---: | ---: | ---: |
| evaluation-artifact-bytes | 8 MiB | 32 MiB | 64 MiB |
| evaluation-artifact-read-bytes | 16 MiB | 64 MiB | 128 MiB |

Use the existing quota_usage key `{owner,resource}` and exact
`{used:Counter,reserved:Counter,lastCommit}` value, checked canonical u64 TEXT.
Both proposed resources have reserved=0. Required zero ledgers belong to explicit
fresh source-owned subject/run/workspace preparation, before reporting authority;
this proposal adds no reporting permission to initialize them lazily. Missing
historical rows or contradictory sums fence the profile, never infer zero.
Upstream child preparation remains a separate source/authority gate.

### Retained artifact charge and saturation

For each retained artifact, charge its full raw sizeBytes plus canonical bytes of
its complete ObjectKey and objects envelope with lastCommit omitted. Charge each
stream mapping's full key and value without lastCommit once. Sum exact logical
occurrences independently at producer assignment, producer run and workspace.
The raw-content contribution is charged even though its retained evaluation body
is shared physically; no digest sharing, head replacement or cross-run target
mapping earns a discount. Local links, custody edges, proofs and physical indexes
remain additionally charged to complete command/store/native budgets.

Target assignment-record-bytes still charges the full evaluation/disposition body
to the worker target and its owner run/workspace. The artifact metadata is not one
of those four evaluation-source record kinds and does not borrow that pool.
Reviewer artifact production is not result-payload-bytes. A later result payload
copy counts again under its own source contract.

Initial/correction publication must fit every target record, producer artifact,
receipt, summary/lineage, command/row/visited-row/whole-store and physical limit
atomically. There is no reservation of future artifact/version capacity. Propose
bounded 409 reason `evaluation-artifact-capacity-exhausted`; where independent
resources fail together, retain target record reason first, then this artifact
reason, then result-payload reason. This adds no wire field or peer usage detail.
Saturation preserves old records, mapping/versions, candidate/work state and
protected credits, with no new receipt, partial artifact, quality failure,
truncation, eviction or automatic retry. A full but valid retained pool does not
block an otherwise authorized exact receipt retry; mutable capacity is checked
only for genuinely new writes. Corrupt/over-limit reload remains unavailable.

The 8 MiB per-producer pool is above twelve 512 KiB raw versions (6 MiB), but
metadata, command/store and native ceilings still constrain fit; no maximum
production profile is qualified. Shared run/workspace pools can fill first.
An evaluation invalidation adds no artifact bytes and deletes no old version;
it still needs target-record, receipt, dependency/summary and complete capacity.

### Artifact read-work reservation

The second resource is lifetime cumulative artifact reply materialization work,
not retained-content storage, a source read grant, or a sliding rate bucket.
Before fetching content, derive a conservative exact-size bound from the retained
size and existing reply schema: sizeBytes plus canonical bytes of
`{schemaVersion:1,artifact:<original descriptor>,evaluation:null}` minus the four
bytes of null. It bounds the same canonical evaluation inserted as an object,
not as an escaped JSON string. Reject a reply over the existing 2 MiB frame.

Propose internal commandKind `evaluation-artifact-read-work`: after existing
current evaluation-capability authentication, scope and read-rate admission,
resolve exact mapping/artifact/retained evaluation metadata and atomically add
that positive reply bound to all three producing-owner read-work used counters,
plus normal header/new proof effects. Revalidate the source/read set and expected
header. No record, artifact, receipt, subject material or read authority is created;
unchanged subject/summary rows must not be rewritten. Zero reserved; no refund
after disconnect, corruption, denied response or an unsuccessful content read.
A content reread charges again; metadata-only original receipt lookup does not.
All existing read/request rates, worker and complete command limits still apply.

This new internal work command has **no #744 claim-first exception**. Current read
authority and lease/source/dependency fences must remain valid at its durable
reservation cutoff; do not reuse an evaluation-write ClaimProofV2 or delay stop.
A native fence for that excluded action is unqualified. If it cannot be proved,
artifact reads remain no-go, not an inferred permission from a durable charge.
After reservation, check current authority before content fetch and response
commitment; stored work never restores a revoked grant. No earlier source/request
or claim policy is broadened here.

Read-work exhaustion returns proposed bounded 409 reason
`evaluation-artifact-read-capacity-exhausted`, without fetching content or issuing
a replacement token. It exposes no peer usage and keeps the existing error
envelope / 2 KiB safe message bound. Temporary materializer/memory admission
pressure uses existing bounded 429 concurrency/capacity denial; unverified
source/store/authority disposition remains 503/unavailable, not quota exhaustion. Completion/resume, new versions, namespace
rotation and ordinary retention do not reset cumulative usage. This no-purge
initial profile retains its owner ledgers; a future deletion policy must preserve
historical run/workspace work charges rather than recreate free capacity.

### Bounded materialization and staging

Propose one active-or-retiring artifact builder/read materializer per sidecar
instance, zero queued artifact materializers, and a 4 MiB per-sidecar-instance
aggregate live artifact frame/canonicalization/reply-buffer budget, including
owned request/response copies from both selected and retiring workspaces. An
old materializer retains this one slot and its memory charge until disposition
and buffer release; switching workspaces cannot create another allowance or a
replacement worker. Independent sidecar instances each have their own allowance;
this is not a host-wide shared allocator or machine memory ceiling.
This is a stricter slice admission limit, not a change to the accepted reporting
rates. Keep existing framing→authentication→rate→lookup ordering and bounded busy
errors; memory pressure or an exact retry cannot create uncounted waiting buffers.
This artifact-specific lease/pool does not admit or consume the existing reserved
UI evaluation/invalidation request lanes, and ordinary stop/cancel remains
independent. Those operations keep their own source/transport capacity rules.
The existing global single storage worker, eight waiting commands / 32 MiB queued
payload, complete-command and SQL/native scratch limits continue independently.

No application staging files, external blob store, background copy worker,
replacement thread or disk-backed temporary artifact is permitted. Materialize
from verified canonical bytes only within the bounded worker/snapshot; stop at
size+1 and fence a mismatch, rather than trusting a descriptor to allow an
unbounded allocation. Qualify allocator/copy/serialization overhead and native
query/cancellation behavior before declaring a supported profile. The 4 MiB
budget is proposed; it does not prove that a worst-case full effect set fits.
Broader retained input/evidence, ordinary reports and non-artifact scratch remain
under their own owner contracts, not exempt through this local budget.

## Atomic source-owned effect recipes

Resolve fresh authority and exact target/reviewer input scope before retry lookup.
For a saved exact authenticated request, return its original bytes without new
artifact, proof, revision, capacity charge or snapshot change. Current authority
and rate/memory admission remain required. A changed request conflicts; a stale
head or frozen stream cannot be bypassed by calling it a new artifact version.

Use the existing complete union Q,S,D,P,K from the candidate-manifest contract:
coalesce changed owner/resource rows; expand every affected subject/summary and
incoming/outgoing dependency under its owning contract; protect required proofs;
include header/new proof and every logical/physical-index cost. Discover the
complete bounded set from retained owners/indexes, never a request-provided list.
Unknown consumer kind, unfinished upstream codec or visited-row/effect overflow
is unavailable/no-go. A named S/D does not certify that those producers are closed.

| Action | Exact new producer core before full Q,S,D,P,K expansion |
| --- | --- |
| Initial child evaluation | Insert evaluation object and its stream head; insert artifact object and immutable stream mapping; insert consumed child-write slot and reserved paired invalidation slot at next exact target admission ordinal; one accepted evaluation-write claim/proof and full-key lookup; insert the two exact custody edges and evaluation-evidence edges |
| Same-stream correction | Insert new immutable evaluation and artifact objects; replace exact evaluation stream head after predecessor CAS; verify unchanged mapping/ID/target; insert new child-write/reserved invalidation pair plus claim/proof/lookup and new custody/evidence edges. Keep every prior object/edge/receipt/version |
| User invalidation | Insert exact disposition for the current evaluation head, consume only the stream's first paired invalidation slot, insert its accepted user-invalidation claim/proof/lookup and source-owned freeze/summary effects. No artifact, mapping or old edge deletion; correction-pair reserved credits remain protected |
| Own artifact content read | Current scoped read plus the separate read-work reservation above; no source/receipt/summary mutation and no write claim |

For writes, the accepted #744 claim/lease tradeoff applies only to its existing
named evaluation/invalidation publications; adding artifact effects does not grant
a larger request/source scope. Revalidate complete typed intent, exact header,
predecessor/result/source/evidence/ledger/CAS and all capacities at commit. The
source-owned target and producer subjectRevision/dependency transitions must both
be represented when their ownership/membership changes; coalesce each subject's
accepted event transition with the owning #744/#746 recipe. Do not publish one
side before its authoritative source/evidence/summary effects.

At the source-owned commit/publication boundary, invalidate every affected
#746/#1089 current detail/overview snapshot before any new current projection can
be read or emitted. This includes corrections whose rationale/record identity
changes while material stays byte-identical: do not increment material revision
merely to force invalidation, or let an unchanged summary conceal stale detail.
Keep affected reads fenced while commit/publication disposition is unresolved.
Exact retries and read-work-only metadata changes create no new source/snapshot
transition. The complete producer coverage and native publication fence still
need separate qualification; this paragraph supplies no new snapshot field.

The hierarchy store alone derives row intents and changed set. No source exposes
SQL, arbitrary affected rows or an upload endpoint. Emit the original canonical
ReportingMutationReceipt only after complete durable outcome and authorized
response commitment; its artifact descriptor matches the new source object and
immutable mapping. A lost response preserves the one exact saved receipt.

## Retention, deletion, corruption and recovery

Correction advances selection but retains every artifact/evaluation/edge and full
charge. Invalidation freezes writes, not read/custody history. Current own-stream
artifact reads remain governed by the existing scoped read capability; a receipt
or ended reviewer does not revive authority. User UI history/read support is not
introduced by this profile.

Reciprocal custody intentionally creates protected cross-subject references.
Single-subject purge of either connected target or reviewer is denied while the
other's retained objects/receipts depend on it, even when both runs are terminal.
There is no automatic edge unlink, body-copy conversion, TTL, head-only purge,
quota release, joint-purge command or permanent saturation recovery path here.
Unrelated eligible subjects retain existing purge rules. Workspace deletion/shared
learning custody retains #745's separate fence/reconciliation gates; this profile
cannot bypass them. This restriction is a proposed initial-slice cost for review.

Recovery must establish the complete old or complete new record/artifact/mapping/
receipt/lookup/edge/ledger/summary/proof state. Verify raw evaluation bytes, both
source digests, exact ownership, contiguous stream versions and original receipts.
No orphan artifact or maximum version proves acceptance. Missing/corrupt state,
unknown required consumers or unresolved commit disposition keeps hierarchy reads
unavailable; ordinary PTY draining and stop stay independent. Do not reconstruct
missing metadata from receipts, reset quotas, delete protected history, or spawn
replacement workers. Read-work reservation may commit without a delivered reply;
its charge remains, with no fabricated artifact or successful read receipt.

## Owner choices and qualification handoff

Scoped review choices are: (1) metadata artifact object + immutable stream mapping
using shared exact evaluation bytes; (2) reciprocal custody with no connected-
subject purge/recovery from saturation in this slice; (3) distinct format-4
header/manifest/row domains and closed-profile selection, with new-store-only
compatibility; (4) 8/32/64 MiB retained artifact and
16/64/128 MiB cumulative read-work pools with no future reservation/refund/reset;
(5) new excluded read-work metadata command, per-sidecar 4 MiB /
one-active-or-retiring materialization policy and bounded capacity reasons.
Existing source/request/receipt domains,
permissions, target record/payload/receipt pools and claim/lease policy are unchanged.

After acceptance, follow the [gated qualification plan](../plans/2026-10-30-evaluation-artifact-qualification.md).
First close source-owned preparation/configuration/consumer joins and the excluded
read-work authority cutoff; then independently pin exact metadata/commitment and
complete initial/correction/invalidation/read-work old/new states, denials, replay,
retention, packing and fault oracles. Physical/native/allocator/read/worker/maximum-
profile and #740 qualification need separate approved executable plans. No
production schema, adapter, runtime or source-valid transaction is delivered by
this proposal. Keep #1176/#741/#744/#746 open for their applicable remaining gates.


## Owner acceptance and Gate 2 handoff — 2026-10-30

After the concrete proposal/limits and its no-purge/no-reset costs were presented,
the owner said “go ahead”. This records scoped acceptance of the reviewed choices
in merged PR #1181 (`11c99357809f24f3282846d3a60e7fa82f12175f`), exact reviewed
head `4db4f144b45c60e750b1249c37c25256f0746918`, for further source/authority
closure and qualification:

- The closed metadata artifact object/domain and immutable producer/declaration
  stream mapping, with shared exact evaluation content and reciprocal custody.
  Connected target/reviewer history cannot be purged individually in this slice;
  no permanent saturation recovery or automatic deletion is supplied.
- Distinct new-store-only format 4, explicit header/manifest/row version/domains,
  closed-profile selection, unchanged existing source/request/receipt/ClaimProofV2
  domains and no old-store migration/adoption.
- Separate producing-assignment/run/workspace retained artifact limits of
  8/32/64 MiB and cumulative artifact read-work limits of 16/64/128 MiB, inclusive,
  with zero future reservation, retained occurrence accounting and no read-work
  refund/reset. The reviewed metadata/stream bounds and bounded capacity/error
  policy are included; no prior record/payload/receipt/store limit is raised.
- The exact internal read-work precharge boundary, explicitly without extending
  the write claim-first exception or using a charge as a read grant. Its native
  current-authority/physical-commit ordering remains unqualified.
- One active-or-retiring artifact materializer and a 4 MiB aggregate budget per
  sidecar instance, including retiring-workspace buffers, no queued materializers,
  replacement worker or staging files; existing reserved UI lanes remain separate.

This supersedes earlier proposed/owner-review-required status for these named
choices only. It changes no reviewed definition, value, byte codec or authority
rule. The [source/authority closure audit](../../validation/evaluation-artifact-source-closure.md)
starts the next gate and records concrete prerequisites
[#1183](https://github.com/Rambolarsen/orkworks/issues/1183) and
[#1184](https://github.com/Rambolarsen/orkworks/issues/1184).
Gate 1 in the [qualification handoff](../plans/2026-10-30-evaluation-artifact-qualification.md)
is complete. Gate 2 is **Investigate**, not passed: retained child preparation/
configuration/consumer codecs and read-work reservation authority still need
closure before dependent complete-state/manifest/packing fixtures.

No broader #741/#742/#744/#746 lifecycle/component acceptance, complete format-4
catalog/source profile, native mechanism, eligible adapter, database/migration,
dependent format-4 fixture/checker creation or execution, live model/native probe
or executable/runtime plan is
approved here. New source-codec or authority choices still need scoped owner
review. Keep #1176 and owning component issues open for their remaining gates.


## Proposed retained child source follow-up — 2026-10-31

The [child source/preparation proposal](2026-10-31-child-review-source-preparation-design.md)
and [gated handoff](../plans/2026-10-31-child-review-source-qualification.md) address
#1183 with a bounded single-task retained-data slice, exact source/configuration/
input/approval/reservation/allocation/attachment joins, fresh preparation and source
consumer retention. Its new child/plan/leaf/source codecs, resource and profile
choices need scoped owner review before dependent fixtures. Existing technical
contracts, artifact acceptance, identity/wire domains and all catalogs/checkers are
unchanged. This does not pass artifact Gate 2, qualify underlying control/native
sources or widen claim-first; #1184/#740 and executable plans remain separate.
