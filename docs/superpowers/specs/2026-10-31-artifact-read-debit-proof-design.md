---
type: Design Proposal
title: Artifact read-work debit command and recovery evidence
description: Proposed exact internal debit bytes, retained decision row and format-4 ledger/manifest recovery joins under the accepted debit-only policy.
tags: [orkworks, hierarchy, artifacts, authority, recovery, storage]
status: draft
workflow_status: representation-owner-review-required
---

# Artifact read-work debit command and recovery evidence

Tracking: [#1210](https://github.com/Rambolarsen/orkworks/issues/1210),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176), #744/#746; initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `30e88921ec7a0a4695a31b76f32cccc8b172784a`.
**New representation proposal for scoped owner review.** The
[#1184 debit-only policy is already accepted](../../validation/artifact-read-work-authority.md#owner-acceptance-and-command-proof-handoff--2026-10-31).
This document does not accept a new codec/catalog/retention choice or authorize
conditional transaction fixtures, SQL/database/migration, native/model probes,
production routes, a new worker or runtime implementation.

## Outcome and preserved authority

Bind one permitted internal debit to exact immutable source identities, a store
predecessor and three producing-owner usage values, so complete committed work
can be distinguished from a lost acknowledgement without replay or refund.
The accepted permit-first debit may settle after later capability revocation;
content fetch and bounded response commitment still require current authority.
A debit is no artifact, reporting receipt, source publication or read grant.

Use the accepted [artifact precharge and budgets](2026-10-30-evaluation-artifact-producer-design.md#artifact-read-work-reservation),
[closed owner keys](2026-10-25-hierarchy-candidate-command-manifests-design.md#closed-resource-ledger-subset),
[canonical source identities](2026-10-04-assignment-evaluation-design.md#identities-and-references),
[format-4 selection](2026-10-30-evaluation-artifact-producer-design.md#custody-edges-and-registry-versioning)
and [commit-control equations](2026-10-24-hierarchy-storage-contract-design.md#commit-commitments-without-a-digest-cycle).
#744 ClaimProofV2, its action class, source/request/receipt domains and external
read frames remain unchanged. There is no new bearer or client idempotency field.

## Exact canonical command

`C(x)` is canonical UTF-8 JSON with UTF-8 key ordering, using the existing strict
serializer. Reject duplicate/unknown properties or tags, invalid Unicode, alternate
stored encodings and number/string substitutions. ID and Digest keep #744's exact
bounds. Counter is storage's canonical checked u64 decimal TEXT; positive Counter
excludes zero. Nested source revisions and descriptor.sizeBytes remain lossless
JSON integers under their existing limits, not Counter strings.

Propose a distinct `DebitIdentityV1`, exactly:

```text
{sidecarGeneration:Digest, sequence:positive Counter}
```

This is not ClaimIdentity or a public handle. A separate sidecar-wide checked
volatile debit sequence is consumed once at the successful permit/revoke ordering
point, including attempts that later roll back. It never wraps, resets on switch,
or derives its next value from stored rows. Exhaustion denies a new permit.
A genuinely new sidecar generation establishes a new sequence scope after normal
recovery; it restores no old permit. Namespace rotation and exact resume do not
create more accounting capacity. Sequence gaps are permitted, not missing debits.

`DebitIntentV1` has exactly these fields (none nullable):

```text
{workspaceId:ID, storageEpoch:Digest, admissionEpoch:Digest,
 reviewerAssignment:ChildAssignmentIdentity,
 authorityGeneration:AuthorityGeneration,
 producerSource:ObjectRef, artifact:ObjectRef,
 evaluation:EvaluationReference, descriptor:ArtifactDescriptor,
 expectedCommit:CommitRef,
 usedBefore:{assignment:Counter,run:Counter,workspace:Counter},
 replyBound:positive Counter}
```

ArtifactDescriptor is exactly the original wire descriptor
`{artifactId,version,sizeBytes,contentDigest}`. ObjectRef and CommitRef retain their
existing closed shapes. Proposed `ArtifactReadDebitV1` is exactly
`{schemaVersion:1,permit:DebitIdentityV1,intent:DebitIntentV1}`. These are both the
complete internal command bytes and the persisted nonsecret decision body; no
second generic intent log or raw external request is added. Its digest is SHA-256
of `orkworks.evaluation-artifact-read-debit.v1`, one LF, then `C(body)`.
No token bytes, timestamps, model labels, SQL, path or caller affected-list appears.

### Required semantic joins

- The worker constructs every identity from authenticated
  AssignmentEvaluationCapability and retained metadata after existing framing,
  authentication, rate and slot admission. The external artifact request remains
  exactly schemaVersion/action/evaluationRevision/evaluationDigest. Reads have no
  mutation idempotency key and cannot supply a debit identity or select a bucket.
- reviewerAssignment is the exact output-bearing child v3 reviewer, with its
  original configuration/declaration/target. authorityGeneration is child-launch,
  equals the current authenticated generation, and its sidecarGeneration equals
  permit.sidecarGeneration. All subject/workspace/store/namespace joins are exact.
  A resumed reviewer can read its own retained original stream under a fresh grant;
  it cannot reuse the original publisher's revoked credential.
- producerSource.key is exactly child-assignment-source with identity equal to
  reviewerAssignment. Its digest is the source-owned
  [ChildAssignmentSourceV1](2026-10-31-child-review-source-preparation-design.md#reservation-allocation-attachment-and-assignment-source)
  digest; that proposed #1183 codec remains independently gated. A root bootstrap,
  reserved child ID or synthetic configuration is no substitute.
- artifact.key is assignment-evaluation-artifact with its exact existing identity
  `{producerAssignment,declarationId,artifactId,version}`. producerAssignment equals
  reviewerAssignment; declarationId resolves the one frozen evaluation output.
  Its immutable metadata, stream mapping and descriptor agree byte-for-byte.
- evaluation is exactly the artifact metadata's full EvaluationReference. Its
  reviewerIdentity is that child; its target is the original accepted child result
  in the same workspace/registered repository and satisfies existing independence.
  Version equals evaluationRevision (1..12 in this slice). Evaluation domain digest
  is distinct from descriptor's raw content SHA-256. New target heads do not retarget
  the read; original retained historical bytes remain subject to the same grant.
- expectedCommit is the complete pre-transaction header.latest in this exact
  workspace/storage epoch; it must resolve its existing format-4 proof. All three
  usedBefore values equal the actual prepared owner ledgers at that predecessor,
  reserved is zero, and checked addition of replyBound fits every cap.
- B = descriptor.sizeBytes + len(C({schemaVersion:1,artifact:descriptor,
  evaluation:null})) - 4. replyBound equals that positive B exactly. Enforce every
  existing narrower frame cap, including the 528 KiB own-artifact reporting frame
  and the artifact profile's 2 MiB ceiling; no frame limit is raised here.

The sole worker stages/validates bounded immutable command references outside the
short authority latch, then assigns the checked sequence and installs only fixed
identity/reference state at the permit point. Serialize/hash outside the latch.
The latch does no parsing, copying, SQL/fsync, network, notification or waits.
Actual participating authority transitions/native enforcement remain unqualified;
a field claiming a generation is no proof that the ordering happened.

### Source read set and content boundary

The command has no caller-supplied read-set array or claimed completeness digest.
expectedCommit pins the whole durable prestate. At that header the source owner
must resolve producerSource through the exact plan/configuration/input/approval/
reservation/allocation/attachment chain, plus original target/evaluation/artifact/
stream/receipt/proof and reciprocal custody/dependency/consumer membership. All
applicable #1183 source and consumer codecs must be closed; an absent known consumer
or unenumerated owner is unavailable, never assumed empty. Volatile health/selection/
lease fences still participate independently of the durable header CAS.

Precharge validation may inspect bounded retained artifact and evaluation metadata;
it must not materialize/hash/copy the canonical evaluation content first. The
EvaluationReference resolves the existing evaluation ObjectRef by its owning
registry; no new evaluation-body copy or digest domain is introduced. If the adapter
cannot separate metadata resolution from content access, this read profile remains
no-go. The debit does not certify those bytes' integrity: only after proven charge
and a fresh authority check may content be fetched and its canonical/source/raw
size/digests verified. Corruption then keeps the charge and denies delivery.

After permit acquisition, revalidate expectedCommit, all source/health/dependency/
lease conditions and the exact ledgers in the one transaction. Changed durable
prestate denies this attempt; do not patch expectedCommit/usedBefore/B under the
same permit or automatically retry. The permit freezes only its exact persistence
permission, not source eligibility or actual lease ownership.

## One retained row and exact six-row effect

Propose one required table in the single closed format-4 catalog:

| Table / primary key | Exact value |
| --- | --- |
| artifact_read_debits / C(DebitIdentityV1) canonical BLOB | `{body:ArtifactReadDebitV1,digest:Digest,lastCommit:CommitRef}` |

The immutable body reconstructs the exact full key. Lookup uses the store's exact
workspace/storage-epoch scope plus those complete canonical key bytes, not a hash
index or artifact-ID-only key. Compare full canonical body as well as digest.
Unknown fields, duplicate keys, mismatched permit/body, changed same-key bytes or
broken source/manifest links make the result unavailable; never overwrite or chain
collision entries. Proposed complete key+value ceiling is 16 KiB, within the
existing 2 MiB row ceiling. The entire encoded wrapper/link counts toward it.

Derive exactly three quota_usage keys with resource
evaluation-artifact-read-bytes and owner:

```text
{kind:assignment,assignmentIdentity:A}
{kind:run,workspaceId:A.workspaceId,runId:A.runId}
{kind:workspace,workspaceId:A.workspaceId}
```

A is reviewerAssignment, the artifact producer, even for a cross-run target.
No owner field is accepted from the external request. Existing value shape is
`{used:Counter,reserved:Counter,lastCommit:CommitRef}`. If U is usedBefore, set
U′ = U + (B,B,B), checked, atomically. Caps remain 16/64/128 MiB inclusive; reserved
remains zero. Missing ledgers require unavailable, not lazy zero initialization.

The command changes exactly six rows: insert the one debit row; replace those
three ledgers; replace store_header; insert the new commit_proof. No source,
artifact/mapping/content, head, receipt/slot/lookup, dependency, subject/summary/
lineage/snapshot or prior proof row is changed or deleted. If reconciliation or
another effect is needed, deny and retain its existing fence instead of folding
an unlisted mutation into this command. Proven rollback preserves complete old state.

ChangeManifestV4 retains exactly its existing field shape, with commandKind
evaluation-artifact-read-work and claim:null. Add no debit field or ClaimProofV2
reference. Its sorted changes array contains exactly four entries: the debit
insert (null beforeDigest), and the three quota replacements. Each existing body
commitment must actually change. Header/new-proof remain the two separately
validated control effects: predecessor equals expectedCommit, checked sequence is
predecessor+1, and all four changed rows' lastCommit equals the new CommitRef.

For the new row, hash `orkworks.hierarchy-storage-row.v4`, one LF, then canonical
`{table:artifact_read_debits,key:DebitIdentityV1,body:{body:ArtifactReadDebitV1,digest}}`.
Exclude only the outer lastCommit. Keep intent.expectedCommit and every identity,
precondition/counter/source field in the hash. Existing quota projections and
format-4 manifest domain stay unchanged. Compute command digest, row digests,
manifest digest, then attach excluded new CommitRefs; validate all links separately.
This avoids cycles and binds the exact debit without a generic claim annotation.

The proof of expectedCommit and the new commit proof are both directly retained
for this debit. This does not require recursively retaining every historical
manifest predecessor or replaying an unbounded chain. Required direct links must
resolve; never elect a header from maximum sequence or hashes of surviving rows.

## Recovery, accounting completeness and retention

A live worker with an uncertain disposition retains its one position/permit,
old store/lease and resource charges. Do not release it, fetch content or report
success while disposition is unknown. Same-worker recovery performs a read-only
lookup of its exact frozen key/body after native recovery establishes a coherent
store. A complete row and matching six-row manifest/ledger effects establish the
original committed debit; no new command, permit, proof, increment or refund occurs.
A full recovered old state with header equal to expectedCommit and no debit/effects
establishes rollback. Missing one required row/link or an otherwise unexplained
header is unavailable, not evidence of rollback. Timeout/interrupt alone proves
neither result. Later commands cannot overtake unresolved disposition.

Historical validation checks the stored generation/namespace/source joins at their
original admission; it must not require an old debit generation to equal the
current process or recreate a current bearer from that historical match.

On process restart volatile permits/waiters die. Recovery validates retained debit
rows, original commits and complete ledger totals, but does not replay any intent,
recreate a grant or continue content delivery. Without a captured original intent,
a missing lookup key is not a discovered rolled-back request. A future authorized
content reread always undergoes fresh read admission and receives a new permit/B
charge, even when the external read request bytes are identical. Metadata-only
original mutation receipt lookup still incurs no B charge. No new public debit
status/receipt read operation is introduced.

For each prepared owner, read-work used equals the sum of B from every retained
committed debit assigned to that owner, across retained runs/admission epochs.
Starting zero must have source-owned fresh preparation proof. In validated commit
order, each body's usedBefore equals the prior sums for its three owners, and its
manifest before/after quota commitments must reconstruct from U/U′ and reserved=0.
The current ledger and lastCommit agree with the latest actual contributor (or
proven initial preparation when no debit exists); this checks accounting, not a
new header/head election. Duplicate, missing, forged-zero or dropped historical
charges fence availability. A valid checksum is integrity evidence, not authority
or protection against same-user whole-store rollback.

Retain all debit rows and their source/mapping/custody/ledger and direct proof
validation joins in this first no-purge profile. Register artifact_read_debits as
a mandatory historical accounting consumer of its producerSource, artifact and
resolved evaluation; this is a direct closed-table retention invariant, not a new
ObjectRef dependency purpose, evaluation finding or current-summary input.
Complete bounded incoming membership must include these references before any
future source deletion. No debit pruning, accumulator checkpoint, per-epoch reset,
startup repair or eviction is proposed. A later compaction/deletion scheme needs
separate reviewed proof that lifetime work charges and recovery joins survive.
#1183's complete consumer registry must be reconciled with this proposed consumer
before a source-valid full profile is claimed.

## Bounds and denial oracles

The 16 KiB debit-row limit is a new proposed metadata bound for owner review.
A width-only canonical calculation using maximum allowed ASCII ID/digest widths,
child source/reference wrappers, 20-digit Counter strings, slice revision bounds
and the 512 KiB size integer gives: child identity 1,305 bytes; command body 8,812;
debit key+complete value 9,238. The six new-row key/value widths are respectively
9,238; 1,667; 617; 484; 462 (header); and 3,508 (new proof), total 15,976 bytes.
The manifest is 3,395 bytes; additionally staging a separate command and manifest
would total 28,183 bytes with those new rows. This is a syntactic upper-width
overestimate, not a source-valid state, hash vector, transaction fixture, serializer
qualification or native-memory measurement. Maximal syntactic counters can be
semantically invalid under quotas; actual admission still checks every bound.

Count complete new keys/values/links, manifest and any duplicated/staged bytes
under the existing 16 MiB command ceiling and 4,096 changed-row cap; all physical
indexes, main/journal/scratch and query work remain separately charged/qualified.
No metadata discount is taken from B, retained-artifact or receipt/source pools.
Debit rows and proofs consume existing logical/physical store capacity and may
cause saturation before read-work pools fill. Retention is finite by positive B:
workspace count ≤134,217,728 at B≥1, a deliberately loose arithmetic bound, not a
supported profile. Full recovery/recount/source discovery must fit existing work
limits, including 100,000 visited logical rows per operation; large histories that
cannot do so remain ineligible/unavailable. No automatic chunking, unbounded scan
or new recovery allowance is authorized. A maximum profile remains unqualified.

Preserve one active-or-retiring materializer, zero queued artifact materializers
and the 4 MiB aggregate instance memory budget for selected/retiring buffers.
The separate 16 MiB/4,096-row, worker-eight/32-MiB and native physical defaults keep
their existing proposed/qualification status. This schema accepts none of them
as a measured or universally feasible profile.

| Conditional case | Required oracle after applicable owner/source decisions |
| --- | --- |
| Same external read twice | Distinct permit sequences, two B charges; no read idempotency key or free content replay |
| Same internal key/body recovered after lost ack | Read-only original disposition; unchanged all rows/links/counters, no new permit or content right |
| Same internal key with changed intent, forced digest equality or wrong store scope | Full-key/body comparison denies/unavailable; no overwrite, duplicate charge or cross-owner disclosure |
| Noncanonical/unknown/mixed codec, root reviewer, wrong class/generation/source/descriptor | Reject before debit; pre-existing rate charges remain, no content fetch |
| Stale expectedCommit/U or source/custody/health change | Complete rollback, no patch/retry under same permit; keep relevant health fence |
| Revoke-first / permit-first then revoke | No debit / exact debit may commit under accepted #1184 policy; content and response rights remain independently checked |
| Independent assignment/run/workspace cap reached exactly or exceeded by one | Equality may commit only if all other bounds fit; any overflow preserves complete old state and fetches nothing |
| Row wrapper exceeds 16 KiB, global command/store/work/physical bound fails | Bounded denial/unavailable per existing category; no truncated proof, partial ledger, quota increase or borrowed receipt credit |
| Commit succeeds, fetch corruption/disconnect or response revoke follows | Preserve all three increments and original proof; no refund or inferred successful delivery |
| Missing debit/proof/source or disagreeing complete totals on reload | Unavailable, not guessed zero, repaired lookup, latest-row election or reconstructed authority |
| Crash before commit / after commit before ack | Native recovery establishes full old / full original six-row outcome; no automatic replay; hashes alone do not establish atomicity |
| Namespace/workspace replacement, lease loss or stuck retiring worker | Preserve accepted #1184 closure/retained-worker costs and all historical charges; no new store allowance, replacement worker or actual-lease-loss exception |

## Owner choices and qualification handoff

The new representation decisions for one scoped review are: (1) independent
volatile DebitIdentityV1 and complete ArtifactReadDebitV1 command/digest;
(2) one retained artifact_read_debits row with full-key recovery, 16 KiB wrapper
bound, and mandatory accounting-consumer/proof retention; (3) exactly four
manifest changes plus two controls, claim:null, with no manifest field change;
(4) inclusion in the one closed fresh format-4 catalog and complete accounting.
These choices are proposed, not acceptance inferred from merge or #1184 policy.

A format-3 reader rejects format 4 before table interpretation. A format-4 reader
must support this entire reviewed catalog, including empty required tables, all
#1183 source/consumer additions and original zero-ledger preparation. Missing table
is not an older base profile, missing row is not zero, and old v4 subsets are not
adopted. No old-store import, migration, runtime DDL, mixed digest interpretation
or alternate catalog selected by row presence is authorized. If future deployment
needs another representation, require explicit version/compatibility review.

Use the [qualification handoff](../plans/2026-10-31-artifact-read-debit-qualification.md).
First obtain representation acceptance and close source/consumer preparation;
then independently generate complete conditional old/new states/manifests and
adversarial packing/recovery cases. Only after relevant contracts close may a
separately approved bounded native plan name actual authority transitions, SQL/
commit/recovery/lease instrumentation and OS evidence. Component widths or hashes
qualify none of those mechanisms. #740, source/custody/native, maximum-profile and
runtime/executable-plan gates remain open. Keep #1210 and component issues open
through applicable review/qualification; no runtime work is delivered here.
