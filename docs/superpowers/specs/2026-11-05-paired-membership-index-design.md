---
type: Design Proposal
title: Physical paired membership and index contract
description: Accepted physical membership/index direction for further qualification, with actual producers and bounded native recovery still gated.
tags: [orkworks, hierarchy, membership, storage, qualification]
status: draft
workflow_status: physical-representation-accepted-qualification-pending
---

# Physical paired membership and index contract

Tracking: [#1428](https://github.com/Rambolarsen/orkworks/issues/1428),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Inspection base:
`c0c8a974b4d9981ceea053ea5514ebeffbc4b0ac`.

## Decision proposed and scope

Propose one derived physical membership layout within the accepted new-store-only
format6 hierarchy and schema2 ordinary pair. It implements the
[35-family audit](../../validation/child-preparation-catalog-reconciliation.md)
and [logical membership recipes](2026-11-03-child-consumer-membership-design.md).
The source records remain authoritative. The new physical rows/indexes are
transactionally derived projections, not additional logical families, ObjectRefs,
manifest changes, dependency purposes, authority proofs or public query messages.

The author selects one exact forward relation plus a reverse index, fixed identity
bindings and a separate artifact metadata projection. Alternatives rejected:
scanning all source bodies on each admission (unbounded/content-before-debit),
an independently committed graph (source/index gaps), and digest-only endpoint
keys (cannot prove full-key identity or absence). No generic caller graph API.

These new physical encodings and recovery/compatibility choices are proposed,
not accepted by prior policy/representation approvals. No executable SQL/DDL,
database opening, migration, product code, fixture or native/model probe is
included. Actual ordinary/source/revocation owners, RegistryRoot and hard native/
resource qualification remain separate gates. #740 stays closed research.

## One private storage interface

The paired storage owner exposes internally only exact typed source-command
planning and bounded coherent discovery. Callers supply an authenticated source
intent, never table names, SQL, graph edges, an affected set, completeness flag,
index digest or ordinary database handle. Its private implementation owns the
fixed codecs, projections, indexed seeks, budgets and paired transaction.

A captured view binds workspaceId, hierarchy storageEpoch/H, ordinary database
Epoch/G, and actual paired owner/connection generation. Both databases are read
through the one retained broker connection in a coherent read transaction.
Observe both headers and their complete read set in that transaction; a later
write transaction revalidates both plus all source/ownership/health conditions.
No independent connection, cached ordinary view or separately acquired pair of
headers establishes this view. A changed H or G denies the captured command;
no patched view or silent second attempt.

Capturing this view does not hold an authority latch through SQL. Source permits
and actual lease checks retain their separately qualified ordering. No index or
view grants an original source permit, runtime attachment or content-read grant.

## Exact physical key encoding

Every structured key/value below uses existing strict canonical UTF-8 JSON C(x)
in BLOB columns with binary equality/order. No delimiter concatenation, hash-only
identity, locale collation, floating conversion or implicit JSON path comparison.
Digest is the existing exact lowercase digest; CommitRef and source revisions/
Counter encodings remain unchanged. Unsupported/duplicate/unknown fields deny.
Physical names below are fixed proposed catalog names, not caller strings.

Use the accepted ConsumerLocationV1/ReferenceTargetV1 encodings with these
**explicit format6-only physical extensions**, called ConsumerLocationV2 and
ReferenceTargetV2 here:

- MetadataTable adds ordinary_child_lineage and
  child_preparation_authorizations with their accepted complete P key. It keeps
  every other selected metadata family and excludes objects/dependencies as
  arbitrary metadata locations. root_drafts keeps its separate draft behavior.
- control-origin additionally permits child_plan_controls only when derived from
  the authorization's accepted PlanControlRead. The original recovery-decision
  snapshot union is unchanged; a recovery record cannot name this new variant.
- All remaining object/metadata/control-origin/commit-proof fields and full source
  keys remain exact. No generic field path, body fragment or new object purpose.

Object locations include their exact source digest. Current metadata locations
retain stable table/key identity; each captured owning value/row digest and link
are separately validated. Historical control-origin includes frozen rowDigest
and original CommitRef; it never resolves to today's mutable control body.

## Proposed derived hierarchy structures

These structures live in the hierarchy peer and are private physical indexes.
A fresh format6 physical catalog requires all of them even when empty. Their
rows derive only from the closed recipes; no independent insertion/update route.

| Physical structure | Exact key/value and indexed order |
| --- | --- |
| membership_owners | Primary key C(ConsumerLocationV2). Value is the closed owner stamp below. One row for each eligible retained object/metadata consumer, including zero-reference consumers; no root_drafts or dependencies pseudo-consumer |
| membership_edges | Primary key `(C(consumer),C(target))`, each in its own BLOB column. No independent value. Both endpoints use the closed V2 types. A mandatory reverse B-tree orders `(C(target),C(consumer))` over the same rows; it is not a second separately writable edge table |
| identity_bindings | Primary key `(namespace,C(key))`, namespace a fixed tag below. Value exactly C(binding). A separate fixed physical identity_binding_contributors relation keyed `(namespace,C(key),C(consumer))` identifies every retained owning source that establishes that binding; mandatory reverse index by complete consumer |
| artifact_read_metadata | Primary key C(artifact ObjectRef). Value exactly `{producerSource:ObjectRef,reviewerAssignment:ChildAssignmentIdentity,evaluation:EvaluationReference,descriptor:ArtifactDescriptor,target:ResultRef,declarationId:ID,artifactRowDigest:Digest,artifactCommit:CommitRef,evaluationRowDigest:Digest,evaluationCommit:CommitRef}`. Fields derive from the exact original artifact/evaluation/immutable stream mapping and accepted debit joins; no evaluation text/content bytes or supplied credential |

membership_edges stores distinct full endpoint tuples for graph reachability.
Repeated fields may collapse only there; source/copy/physical occurrence charges
remain intact. A zero-reference owner row distinguishes a recognized empty recipe
from an unsupported/missing projection, but its presence alone proves neither
correctness nor completeness. Unknown/missing/extra owners/edges fence the profile.

The owner stamp is exactly `{kind:"row",rowDigest:Digest,owningCommit:CommitRef}`
for normal row consumers, `{kind:"header",latest:CommitRef}` only for store_header,
or `{kind:"manifest",commit:CommitRef}` only for commit_proof. Normal rowDigest
uses that family's accepted format6 projection; owningCommit is its actual
lastCommit/pending origin or exact source owning proof. Header/latest and a new
manifest remain separate controls, never rows fabricated into their own changes.
The header/manifest stamps reuse their exact direct proof equations without
inventing a row commitment or graph commit for those excluded controls. Physical owner rows and edges do not enter logical commitments
or acquire independent source authority. Snapshot origin targets retain their
containing authorization/recovery snapshot and original manifest commitment.

### Fixed full-key identity bindings

| namespace | Exact key | Exact binding and contributors |
| --- | --- | --- |
| session-owner | sessionId:ID | Tagged parent binding `{kind:"parent",workspaceId,runId,parentSessionId}` or child binding `{kind:"child",task:TaskKey}`. Original parent source/runtime and child reservation/attachment/session/lineage contributors must derive the same role and complete owner. One session cannot belong to a second run/task or change role |
| child-attachment | attachmentId:ID | Full original ChildAssignmentIdentity P. Original attachment, source, session bridge and ordinary lineage must agree |
| preparation-permit | PreparationPermitIdentityV1 | Full P. Only the original authorization contributes; one permit cannot map to two assignments |
| ordinary-lineage | full P | `{childSessionId:ID,createdAt:Text,attachmentId:ID,assignmentSourceDigest:Digest,preparationCommit:CommitRef}` from the exact ordinary canonical member. A hierarchy contributor must agree with the ordinary full-key projection |

These are uniqueness witnesses derived from original sources, not new identity
allocators or live ownership. The source's existing keys/uniqueness checks still
apply. All retained contributors are exact consumers under their closed recipes;
a missing expected contributor or an extra source with a conflicting binding is
unavailable. There is no last-writer-wins binding replacement. Preserve bindings
and contributors through cancellation, end, Delete and linked tombstones.

The ordinary peer retains its accepted full-P lineage projection/index derived
from canonical metadata. Its childSessionId/id and id/created_at constraints stay
in the ordinary owner; the paired publisher compares that projection with
ordinary-lineage/session-owner and actual sessions, including deleted rows.
Unlinked ordinary sessions still occupy their IDs. All ordinary creators and
relevant deletion/resume/import paths must participate in these checks before
linked eligibility; the hierarchy binding table alone cannot prevent a foreign
ordinary create. Actual producer participation remains unqualified under #1242.

## Owning writes and complete lookup/absence

For every supported original source/control/publication transition:

1. Validate full old and proposed new source rows and their direct proofs under
   the exact command's existing authority. Derive owner rows, all distinct
   membership tuples, every binding/contributor and artifact projection from the
   closed recipes. Hash/copy outside authority locks and within existing budgets.
2. Revalidate H/G, complete source read set, expected physical projections and
   actual source/lease/health conditions. Compare the complete old derived set
   with stored forward rows, not only proposed new edges. A discrepancy denies;
   it is not repaired by the producer.
3. In the same accepted source transaction, apply logical rows, normal dependency
   effects, ordinary canonical member/projection, every physical derived delta,
   quotas/summary/lineage, header and proof. Coalesce each logical key once.
   Derived indexes add no ChangeManifest entry or extra subject/control revision.
4. Commit the accepted paired outcome and publish ordered immutable references/
   invalidations before releasing its position. No later index registration,
   independent graph flush or source-validity token is returned early.

Forward lookup is an exact owner-key seek and bounded edge prefix range. Reverse
lookup is an exact full-target prefix range over the reverse index. Direct proof
lookup uses full CommitRef plus commit_proof's exact sequence key, comparing epoch
and digest; it never seeks the maximum. Source groups traverse the fixed typed
recipes, with full canonical visited identities to terminate custody cycles.
Binding lookup uses exact namespace/key and verifies complete contributors.
No unindexed JSON scan, arbitrary recursive SQL or eager content loader is exposed.

A successful result proves its range ended in the same captured view, all returned
owning records/projections validate and the stored correspondence was qualified
before admission and preserved by every intervening writer. A missing primary
key or empty reverse range proves absence only under those invariants and full
retained membership, including historical/tombstone sources. A locally matching
edge, count, hash or owner sentinel cannot establish global completeness.

Pagination is private to one admission/read transaction. No caller continuation
or partial result is accepted as complete. Count every page, entry, returned
owner/projection and failed/end-range probe under the accepted 4,096 observations
and 16 MiB cumulative canonical copies; size+1/exhaustion work must also fit.
Stop before the next excess allocation/query; deny if completion cannot be proven.
Indexed seeks do not qualify SQLite VM/allocator/scratch limits automatically.

## Artifact metadata and content separation

The original qualified producer validates canonical artifact/evaluation content,
source/stream/declaration/descriptor joins and row commitments at its own permitted
publication phase, then writes artifact_read_metadata atomically. Its fields are
exact projections of those originals, not user-supplied labels, a new digest domain
or a content cache. Current source eligibility and authenticated reviewer generation
are still checked separately at a read request.

Precharge discovery reads only the dedicated projection, exact source/control/
ledger metadata, memberships and direct proofs needed by the accepted DebitIntent.
It does not SELECT, materialize, copy or hash the canonical evaluation/artifact
body, result payload or evaluation text. No per-request fallback to such bodies
or read-triggered reconstruction/reload is allowed. Bind artifact/evaluation row
commitments and original mapping/descriptor through the already qualified source
projection and owning manifests. The accepted read debit keeps its four logical
changes plus header/new proof; maintaining its derived owner/edge/projection facts
adds no logical index row or extra debit.

Only after the original proven B debit and current read authority may the bounded
materializer fetch/hash/copy content under its existing limit. Missing/corrupt/
ambiguous projection or descriptor/source disagreement denies, without a debit
refund or new content-read permission. Retention membership is never freshness,
reporting capability or transitive read authority.

## Qualification, sparse recovery and corruption

Fresh creation must establish the complete selected logical and physical catalog
through its separately closed initializer. Existing format5/schema1 files, partial
test stores or missing peers are not adopted, backfilled, reset or upgraded.
A future format6 reader recognizes this one proposed fixed physical layout and
exact index/column definitions, not row-presence-selected subsets. An existing
format6 implementation with a different physical layout cannot be silently reused;
there is no deployed eligible format6 store assumed here. A different later layout
needs its own explicit compatibility decision. No new logical domain/version is
selected just for these derived structures.

Before enabling a reopened pair, a qualified private recovery pass must prove
complete source-to-index correspondence in both directions at one H/G, including
all zero-reference owners, binding contributors, ordinary tombstones and artifact
metadata. It validates canonical source codecs and required direct proofs, derives
the expected sets and compares them exactly with stored sets. It neither trusts
counts/hashes as completeness nor elects/replays a maximum sequence. Validate
latest and every required origin/expected/receipt/checkpoint link; do not require
all unrelated predecessor proofs or every row ever mentioned by an old manifest.

This pass is **not a content-read request or an uncharged read-work cache**.
Any content-bearing canonical validation must be an explicitly approved recovery/
maintenance operation under its own existing owner/resource rules before read
admission. This proposal grants no such operation or numeric allowance. If those
owners cannot qualify the scan within existing bounds, reopening remains
unavailable; artifact requests cannot trigger it or bypass B. Pure metadata opens
cannot claim full content/projection correspondence from only matching digests.
No fixture or proposed index is evidence that this recovery budget is achievable.

A mismatch, missing origin/body/index, unknown writer or interrupted/unknown paired
outcome fences dependent operations. Recovery may inspect the proven complete
old/new state under its original sole owner; it may not durably rebuild indexes,
insert absent zero rows, invent permits/receipts or restore capabilities. Repair,
migration or broader recovery would require a separate concrete proposal.
Immediate ordinary stop/input remains independent of scans/SQL and qualification.

## Resources, verification oracles and handoff

Every physical key/value, contributor, index page and write amplification counts
in existing physical store/page/journal/command/queue/native scratch bounds.
Original metadata copies retain their source/control accounting; no new persistent
consumer quota or sharing discount. Logical retained-byte ledgers continue their
accepted projections; physical derived storage is additionally counted as physical
storage, not secretly inserted into a new logical resource. The ordinary 1 MiB
joint job includes all referenced staged copies; lower limits deny legal larger
sources without widening the profile. No maximum supported profile is claimed.

A later approved implementation/qualification plan must cover:

- Every one of the 35 families and closed object/purpose variants, including empty
  recipes, original authorization's plan origin and ordinary lineage/tombstones.
- Deliberately missing/extra/forged/reversed index entries, swapped full keys,
  dangling contributors, conflicting session/attachment/permit/P bindings and
  correct local edges in a globally incomplete set: unavailable, never repaired.
- Both summary branches, complete original-attach union, every physical delta and
  unchanged peer; stale H/G/source/read set denies without patching or second save.
- Metadata-only precharge instrumented to prove no content query/copy/hash before
  B; missing projections deny rather than falling back to an eager objects loader.
- Lost response, sparse unreferenced proof gaps, corrupt required direct origin,
  cancellation-before/after-permit, true ownership loss and unknown disposition.
- Inclusive observation/copy boundaries and first excess, physical write/index/
  journal/scratch bounds, rollback and old/new paired disposition without a later
  registration gap. Logical assertions do not replace native durability evidence.

The scoped decision is this exact derived owner/edge/reverse-index layout, fixed
identity bindings/contributors, artifact metadata projection, explicit V2 physical
locator extensions and fail-closed qualification/compatibility policy. Existing
source codecs, authorization/ordinary representations, logical manifests/domains,
quotas and authority choices stay unchanged. After one review and scoped owner
acceptance, reconcile an executable initializer/producer/recovery plan only when
actual owners and hard-resource/native prerequisites are closed. #1176 remains
open; no activation or repeat approval of unchanged decisions follows here.


## Bounded design review

One bounded pair of read-only Luna reviewers checked the final substantive draft:
correctness against the accepted source/membership/debit/joint contracts, and
builder/completeness against the exact proposed physical representation. Neither
found a required correction or important unanswered design question. During that
same round the author explicitly named identity_binding_contributors and the
row/header/manifest owner-stamp union; both reviewers checked the clarified text.

The review covers typed full keys, reverse/forward correspondence, identity
contributors, artifact metadata/precharge separation, original six logical debit
effects, paired H/G and fail-closed sparse recovery. It establishes no actual
producer participation, feasible recovery/native resource budget, physical
implementation, SQL/DDL, eligible profile or activation. New physical choices
remain owner-pending; no additional review round or native test is claimed.


## Scoped physical-design acceptance — 2026-11-05

After [PR #1430](https://github.com/Rambolarsen/orkworks/pull/1430), reviewed head
`b12837e2114106031a6166b15a0b4aaf48d3b886`, merged
`866ed60d85a90946269cd99b6a7a347374a174ad`, the owner answered “keep going”
to approval of the proposed physical index layout, artifact metadata projection
and recovery rules **for further qualification**. [#1431](https://github.com/Rambolarsen/orkworks/issues/1431)
records this unchanged scoped decision; [ADR 0108](../../adr/0108-paired-membership-indexes.md)
records the persistence direction.

This accepts the exact derived owner stamps/edge relation/reverse index,
identity bindings/contributors, artifact_read_metadata projection, physical V2
locator extensions and H/G correspondence/absence, sparse recovery and fresh-pair
compatibility choices above. Earlier proposed/owner-pending wording is historical
for those exact choices. Existing bounded correctness/builder reviews remain the
technical review evidence; no new review round or qualification result follows.
Do not ask again for this unchanged physical design or earlier accepted decisions.

This is not acceptance of RegistryRoot, actual source/ordinary publishers, feasible
recovery scans or hard resource/native enforcement. It authorizes no code, SQL/
DDL/database/schema/migration, fixture, executable/native plan, model probe, live
registration or activation. The [source-publisher handoff](2026-11-05-source-publisher-execution-design.md#physical-index-acceptance-and-source-owner-handoff--2026-11-05)
now distinguishes that separate registry choice from the accepted storage layout.
#1176 stays open; #740 stays closed research.


## Scoped derived-index exception — 2026-11-05

A subsequent read-only review of #1430 identified that the earlier
[consumer registry](2026-11-03-child-consumer-membership-design.md#decision-and-boundary)
expressly prohibited a generic persisted consumer graph. The owner then answered
“go ahead” to the explicit narrow exception for these fixed derived indexes,
maintained atomically with authoritative records, and to this exact physical
representation for further qualification. This decision is tracked by
[#1433](https://github.com/Rambolarsen/orkworks/issues/1433).

The [dated registry exception](2026-11-03-child-consumer-membership-design.md#scoped-derived-index-exception--2026-11-05)
and [ADR 0108 amendment](../../adr/0108-paired-membership-indexes.md#amendment-explicit-derived-index-exception--2026-11-05)
make that policy relationship explicit. #1432's physical-layout acceptance and
all reviewed technical definitions remain unchanged. This permits only the
closed source-derived physical layout above, never independent graph writes,
caller-supplied edges, new logical commitments, quotas or authority.

This clarification supplies no new technical/native qualification evidence. All
implementation, recovery-scan, executable/native-plan and activation gates remain
in force. RegistryRoot stays independently proposed; #1176 stays open and #740
stays closed research. No repeat approval of these unchanged decisions is needed.
