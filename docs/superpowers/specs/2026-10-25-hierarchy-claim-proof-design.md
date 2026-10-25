---
type: Design Proposal
title: Hierarchy claim-proof metadata and storage commitment version 2
description: Proposed strict nonsecret claim records, action bindings, receipt and commit joins, versioned hashes and recovery fixtures for the accepted scoped publication policy.
tags: [orkworks, hierarchy, authority, storage, recovery]
status: draft
---

# Hierarchy claim-proof metadata and storage commitment version 2

Tracking: [#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `d48142f596a343d0bd6813acef119a911a5424de` (PR #1033).
**Proposed storage schema/encoding for review; not accepted or executable.**
The owner accepted the [scoped #744 claim policy](2026-10-24-hierarchy-authority-ordering-design.md#owner-acceptance-and-handoff--2026-10-24)
for further design/qualification. This supplies its next schema preparation,
not permission to create a database, migrate a store, run a native/model probe
or implement runtime behavior. Numerical storage defaults remain proposed;
#740 stays deferred/no-go until November. Source-domain JSON, wire receipts,
action IDs, quotas, claim/revoke policy and finalization authority are unchanged.

Read with the [storage row/commit proposal](2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation)
and [#744 canonical records](2026-10-04-assignment-evaluation-design.md#canonical-records).
This proposal defines claim-specific rows and v2 commitments; it does not close
all #741/#742/#743 domain schemas, writer coverage, native resource/lease proof
or the separate finalization-versus-writable-ownership ordering.

## Chosen representation and limits of the evidence

Use one immutable claim-proof row for each newly accepted mutation in the
already accepted #744 action class, stored atomically with its original receipt
and source/dependency/summary effects. The receipt and commit manifest name the
same proof. Keep authorization-decision metadata out of source records/digests
and the existing ReportingMutationReceipt wire object. Do not retain bearer
bytes, raw requests, token hashes, machine/process/account identity, wall-clock
claims or a second event/audit log.

The proof records what the trusted sidecar decided, not a portable credential
or independent proof that the latch ran before revocation. Canonical hashes
check retained bytes/joins; they cannot authenticate history against same-user
store replacement or reconstruct a lost decision. Native instrumentation/races
must still qualify the accepted ordering. A proof is never an input to claim,
launch, retry, finalization or renewed authority.

Alternatives: embedding metadata in canonical candidates/evaluations would
change source digest domains; putting a new field in wire receipts would change
accepted reporting; retaining a raw request repeats bounded source bodies and
violates the no-second-request-log contract. A separate proof row with bounded
references preserves these contracts and makes its byte/retention costs explicit.

## Common encodings and claim identity

Use #744's strict canonical serializer and existing ID/Digest/AssignmentIdentity,
AuthorityGeneration, ResultRef, EvaluationReference, ResultPredecessor,
EvaluationPredecessor, ReviewerIdentity and expectedUserSlot types. JSON parsers
reject unknown/duplicate keys/tags, wrong nullability and noncanonical integers.
Stored canonical bytes must match reserialization exactly; never normalize a
corrupt stored proof into eligibility. BLOB keys compare exact canonical bytes.

`Counter` is the storage proposal's canonical decimal TEXT u64, 0 through
18446744073709551615, with no sign/leading zero/fraction/exponent/whitespace.
Positive Counter excludes zero. Source JSON numeric revisions retain their
existing lossless number encoding, not Counter strings. Digest means exactly
64 lowercase hexadecimal characters. admissionEpoch uses #742's existing
64-character lowercase hexadecimal admission-namespace encoding.

`ClaimIdentity` has exactly `sidecarGeneration: Digest` and
`sequence: positive Counter`. A sidecar-wide checked sequence is consumed once
at each successful claim transition, including a claim whose attempted SQL later
rolls back. It is not assigned on queue admission, replay or response delivery.
It never resets/reuses within that sidecar generation, including across workspace
switches. Exhaustion closes new claims while ordinary controls continue; no
automatic restart or restored grant. A genuinely new sidecar generation has a
new sequence scope. Recovery never resumes an old generation/counter or derives
a new active counter from the maximum retained proof. Holes are valid after
rolled-back claims; proof sequence is not durable commit or request order.

The implementation must use the existing qualified fresh sidecar-generation
source; unknown/reused generation is no-go, not an inferred identity. Claim
identity is nonsecret metadata and cannot authenticate a request. There is still
one active-or-retiring worker/claim in the sidecar, not one per action or epoch.

## Strict decision and committed proof shapes

`ClaimDecisionV1` has exactly the following fields; all are present:

| Field | Exact shape / invariant |
| --- | --- |
| claimIdentity | ClaimIdentity issued at the claim/revoke latch transition |
| workspaceId | Existing ID, equal to the selected owned store and all assignment bindings |
| storageEpoch | Digest, exact initialized store epoch; never read from caller input |
| admissionEpoch | Existing admitted namespace, sidecar-derived; old-epoch mutation replay cannot claim |
| action | One of the five full #744 mutation action IDs in the table below |
| requestDigest | Existing SHA-256 `orkworks.assignment-report-request.v1` + LF + the complete strictly validated canonical mutation request, including idempotencyKey |
| authority | Exactly one of the two variants below, copied from authenticated current authority at claim |
| subject | Complete AssignmentIdentity owning the mutation receipt/capacity; evaluated target for evaluation/disposition, producing assignment for candidate |
| precondition | Exactly the action-specific tagged variant below |

`authority` is exactly:

- Producer: `{kind: "producer", assignmentIdentity: AssignmentIdentity,
  authorityGeneration: AuthorityGeneration}`. Candidate authority assignment
  equals subject; child evaluation uses the reviewer's exact child assignment,
  which can differ from the evaluated target. The generation tag matches that
  assignment and its sidecarGeneration equals claimIdentity.sidecarGeneration.
- UI: `{kind: "workspace-ui"}`. Current UI authentication still happens at claim.
  The workspace/admission namespace above is its accepted durable retry scope,
  not an account or token principal. A target's historical runtime generation
  is provenance, never current UI authority. No caller-selected actor is admitted.

| Exact action | Authority / exact precondition properties |
| --- | --- |
| orkworks.result.submit.v1 | Producer for that child/root subject. `{kind: "result-predecessor", predecessor: ResultPredecessor}` |
| orkworks.evaluation.submit.v1 | Producer for the exact child reviewer. `{kind: "evaluation-predecessor", result: ResultRef, reviewerIdentity: <child ReviewerIdentity>, predecessor: EvaluationPredecessor}` |
| orkworks.user-evaluation.create.v1 | UI. `{kind: "user-slot", result: ResultRef, expectedUserSlot: <existing exact expectedUserSlot>}`; no issued userReviewId exists in the decision yet |
| orkworks.user-evaluation.correct.v1 | UI. `{kind: "evaluation-predecessor", result: ResultRef, reviewerIdentity: <user ReviewerIdentity>, predecessor: EvaluationPredecessor}` |
| orkworks.user-evaluation.invalidate.v1 | UI. `{kind: "evaluation-head", evaluation: EvaluationReference}`; exact evaluated result digest comes from the request and joined source |

Result/evaluation references must belong to subject. For child evaluation,
precondition.reviewerIdentity.assignmentIdentity equals authority.assignmentIdentity.
Every nested workspace agrees; a different reviewer run does not change receipt
ownership to that run. No caller may substitute a child authority for UI or vice
versa. The stored precondition is the request's exact expected predecessor/slot,
not a later head. An already-claimed intent cannot be widened after revocation.

`ClaimProofV1` has exactly `schemaVersion: 1`, `decision: ClaimDecisionV1` and
`record: AcceptedRecordRef`, where AcceptedRecordRef is exactly one of:

- `{kind: "candidate", assignmentIdentity: AssignmentIdentity,
  candidateDigest: Digest}` for result.submit.
- `{kind: "evaluation", reference: EvaluationReference}` for the three
  evaluation submit/create/correct actions.
- `{kind: "disposition", evaluation: EvaluationReference,
  dispositionRevision: 1, dispositionDigest: Digest}` for invalidate.

All digests/identities must resolve the exact unchanged canonical source body.
The proof's decision is frozen at claim; record is filled only after the source
contract derives the committed object, notably the server-issued userReviewId
for user create. That late derived reference cannot change decision/action/target/
request bytes or authorize a different stream. A rejected/rolled-back mutation
has no committed ClaimProofV1 or accepted mutation receipt. Candidate finalization
outcomes use their existing internal proof/receipt path, not this producer claim.

No SQL, hashing or serialization runs inside the short claim latch. Prepare
strict bounded request data outside it, assign the sequence and capture immutable
binding references at claim, then hash/serialize the immutable decision outside
the latch. At commit compare the exact frozen decision, source and precondition
binding under the qualified worker ordering; a digest match alone cannot replace
byte/identity validation. This schema supplies no implementation of that ordering.

### Reconstructing the request binding without a raw request log

Use the original wire receipt's idempotencyKey/action/requestDigest, exact
canonical source object and the following historical binding, not mutable heads:

| Action | Reconstructed request / validation |
| --- | --- |
| Candidate | Complete stored AssignmentResultCandidate plus schemaVersion/action/idempotencyKey; its generation/assignment/predecessor matches the decision |
| Child evaluation / user correction | Complete stored AssignmentEvaluation plus schemaVersion/action/idempotencyKey; reviewer, result and predecessor equal the decision |
| User create | Remove only reviewerIdentity and reporterSource from the accepted evaluation to recover evaluationDraft; add the decision's exact expectedUserSlot plus schemaVersion/action/idempotencyKey. Verify the removed values were sidecar-issued user provenance and the exact new stream |
| User invalidation | Complete stored disposition plus the decision evaluation's resultDigest and schemaVersion/action/idempotencyKey; resolve its exact evaluation/result/reviewer target |

Hash those reconstructed canonical request bytes under the unchanged request
domain and compare with both proof and receipt requestDigest. Verify all source,
action, principal/namespace/key and predecessor relationships even if hashes
match. Retain immutable predecessor/slot records required for that validation;
current head advancement cannot retarget the binding. This adds only the small
missing slot/authority/precondition metadata, not another 512 KiB source/request
copy. Missing required source/receipt/binding proof fences mutation/receipt
service; never fabricate it from a surviving artifact or digest.

## Proposed format 2 placement and commitment graph

This defines a new candidate on-disk format identity, not a migration. The
proposed store_header has schemaVersion 2; its latest and every retained CommitRef
resolve only ChangeManifestV2 and v2 row commitments. CommitRef's property shape
is unchanged; its digest domain is determined by the strictly validated format.
Do not mix v1/v2 manifests or infer version from a digest. The prior filename
candidate does not select a parser; unknown/mismatched initialized format is
unavailable, never a missing/empty store. Creation still needs explicit UI
preparation and full lost-store detection; this pass creates no file or epoch.
Existing v1 bytes/fixtures remain unchanged and unsupported for this format;
no automatic upgrade, rebind, dual write, copy or history reset is authorized.

Start from #1030's logical tables with these exact additions/replacements:

| Table / key | Exact format-2 change |
| --- | --- |
| claim_proofs / canonical ClaimIdentity | `body: ClaimProofV1` canonical BLOB, `proofDigest: Digest`, `commit: CommitRef`. The row is immutable; one per newly accepted claimed mutation |
| receipt_slots / existing assignment/slot key | The mutation ReceiptBinding gains exactly `claim: ClaimProofRef`. All other mutation binding properties remain. Candidate-finalization binding has no claim property and cannot masquerade as a mutation |
| commit_proof / existing positive commitSequence | body is ChangeManifestV2, hashed in the v2 domain; the new claim row and receipt slot are part of its changes |
| store_header / singleton 1 | schemaVersion 2 with unchanged workspaceId/storageEpoch/latest properties and exact header/proof transition invariants |

`ClaimProofRef` is exactly `{claimIdentity: ClaimIdentity, proofDigest: Digest}`.
The receipt slot's claim ref, manifest claim ref and proof row key/digest must
match exactly. The proof record resolves the slot's existing record/response;
all commit links equal that same committing manifest. One proof cannot be reused
by a second receipt, action, subject, epoch or SQL commit. No new lookup aliases
are created on retry, user token renewal or sidecar restart.

`ChangeManifestV2` has exactly schemaVersion 2, workspaceId, storageEpoch,
commitSequence, predecessor (CommitRef or null), commandKind, changes (the existing
sorted exact row changes), and `claim: ClaimProofRef or null`. Existing command-kind
encodings remain; no new arbitrary command tag is introduced. candidate-admission,
evaluation-write and evaluation-invalidation require a claim ref with a matching
action/record/subject. initialize-store requires null; all other existing internal
or excluded command kinds also require null and retain their separate unresolved
authority/qualification gates. Null is never a fallback for a listed mutation.

Hash canonical ClaimProofV1 with `orkworks.hierarchy-claim-proof.v1` + LF.
Hash ChangeManifestV2 with `orkworks.hierarchy-storage-change.v2` + LF.
Hash row envelopes `{table, key, body}` with `orkworks.hierarchy-storage-row.v2`
+ LF, preserving #1030's exact key/body and payload commitment rules. Exclude
only the previously enumerated storage CommitRef link paths plus
`claim_proofs.commit`. The receipt's new claim ref stays IN its commitment; the
claim row's exact proof body and proofDigest stay IN its commitment. No source
property, ClaimIdentity, authority, predecessor, target or requestDigest is
removed. Header/latest and the new manifest's own commit_proof row remain
control effects checked by their explicit header/proof equations, not self-hashed
changes. Deletion of an older eligible proof is still a listed row change.

Compute in this order: source/request digests → complete claim proof and its
proofDigest → claim ref and receipt/claim row commitments → manifest digest →
outer CommitRef links. ClaimProofV1 contains no CommitRef; its accepted source
reference uses only source-domain digests. Attaching the outer commit cannot
change proofDigest/row/manifest hashes. Changing authority, request binding,
source reference or claim ref does. Validate excluded commit links separately;
exclusion is not permission for link-only rewrites or cross-commit substitution.
For claim_proofs specifically, the commitment envelope is exactly
`{table: "claim_proofs", key: <ClaimIdentity>, body: {body: <ClaimProofV1>,
proofDigest: <Digest>}}`: the inner body is the validated logical proof object,
not a base64 string or JSON-quoted BLOB. Its physical BLOB bytes must equal that
object's canonical serialization. This fixes one representation for the new row;
it does not fill unclosed encodings in other domain/lookup rows.
This breaks the hash cycle without suppressing the authorization metadata.

## Admission, quotas, retention and recovery

The existing request/rate/authentication/exact-retry/CAS order is unchanged.
An authorized exact retry resolves the original receipt and its proof before
mutable-head checks, with no new claim, sequence, proof row or transition.
A revoked producer cannot inspect that receipt just because its proof survives.
Source/key conflicts and validation/capacity denials create no proof/receipt.
Once a claim exists, rollback may consume its volatile sequence but cannot
publish acceptance. An ambiguous outcome keeps the sole worker/claim/fence
charged until complete old/new source/receipt/proof/lineage recovery.

Charge the entire claim row, its canonical key, digest, outer commit link and
required claim lookup/index bytes to its owning mutation receipt's existing
16 KiB slot budget, together with all pre-existing slot/key/response metadata.
The target assignment/run/workspace owns evaluation proof costs, even for a
reviewer from another run. One physical proof is charged once; references in
receipt/manifest/other rows count their own bytes. Complete-command/main-file/
changed-row budgets independently include every new row/reference/control
byte. Every new claimed mutation adds its proof row to changed-row counting.
Do not raise 16 KiB/1,088-slot/run/workspace limits or borrow future user credits.

A source-valid maximum record may therefore fail storage admission when its
full proof/receipt/metadata cannot fit. Check exact size before claim/SQL and
again for server-derived user-create metadata before commit; failure rolls back
without an accepted mutation. Protected slot reservations are logical capacity,
not physical writability. This pass establishes no eligible maximum profile
and supplies no VFS/allocator/scratch or storage-headroom guarantee.

Proof count is at most the number of retained consumed mutation slots, never
the number of requests/reads/retries: it cannot exceed the accepted 1,088 slots
per assignment or the workspace's slot-byte capacity. Reserved unused slots
and internal finalization receipts create no producer proof. Keep a proof and
its referenced commit/source/receipt validation material while any retained
owner needs them. Explicit eligible subject purge removes solely owned proof,
receipt, source and dependency effects atomically; no deletion on completion,
namespace rotation, timeout, token refresh or rate-window reset. No proof/receipt
compaction may break another retained consumer's validation.

On reload, verify format/header/latest, exact epoch/workspace, manifest hashes,
complete changed rows and row commitments, claim row hash/key, receipt/manifest
claim equality, source/request reconstruction, action/authority/subject/precondition
joins, exact commit links and ledger/proof ownership. Missing or multiple claims,
wrong generation tag, orphan proof/receipt, unsupported version/domain, future
proof link or unprovable source lineage keeps affected service unavailable.
Never infer a claim from commit time, run status, maximum sequence or receipt
alone. No proof re-creates a live authority record or permits SQL re-execution.
Proof metadata does not prove historical latch timing on its own; native ordering
qualification remains necessary even if all these joins validate.

## Canonical byte fixtures

These are deterministic ASCII encoding/hash vectors, not granted authority or
executed SQL/recovery tests. The synthetic source below has no outputs; a real
admission still needs a valid approved empty-output contract and all source
checks. Repeated hexadecimal values are placeholders for encoding only, never
actual generations/epochs or capability evidence.

Source candidate canonical bytes (no trailing newline):

```json
{"assignmentIdentity":{"bootstrapConfigurationDigest":"0000000000000000000000000000000000000000000000000000000000000000","bootstrapId":"b","kind":"root","parentSessionId":"p","rootAssignmentId":"a","runId":"r","workspaceId":"w"},"authorityGeneration":{"generation":"2222222222222222222222222222222222222222222222222222222222222222","kind":"parent-runtime","sidecarGeneration":"1111111111111111111111111111111111111111111111111111111111111111"},"outputs":[],"predecessor":{"kind":"no-head"},"schemaVersion":1}
```

Candidate digest is `547170cf0dd1f2cafa8f1583fe6b4134b119dd9ca19d56aa1b8b3446d9a756c1`.
Wrap that candidate in exactly `{schemaVersion:1,action:"orkworks.result.submit.v1",
idempotencyKey:"k",candidate:<above>}`, using canonical JSON property ordering;
its unchanged request-domain digest is
`7a209f1d25cbf8bfe0c209950d4094f03fc8a2871da9f6ea61d7c417eb73b1fe`.

The following canonical ClaimProofV1 is exactly 1588 UTF-8 bytes, without a
trailing newline. Its proof-domain digest is
`e5b108ae5a4e471892d17cd301b523b527771f954e7a853813a382f0dfe4ef60`:

```json
{"decision":{"action":"orkworks.result.submit.v1","admissionEpoch":"4444444444444444444444444444444444444444444444444444444444444444","authority":{"assignmentIdentity":{"bootstrapConfigurationDigest":"0000000000000000000000000000000000000000000000000000000000000000","bootstrapId":"b","kind":"root","parentSessionId":"p","rootAssignmentId":"a","runId":"r","workspaceId":"w"},"authorityGeneration":{"generation":"2222222222222222222222222222222222222222222222222222222222222222","kind":"parent-runtime","sidecarGeneration":"1111111111111111111111111111111111111111111111111111111111111111"},"kind":"producer"},"claimIdentity":{"sequence":"18446744073709551615","sidecarGeneration":"1111111111111111111111111111111111111111111111111111111111111111"},"precondition":{"kind":"result-predecessor","predecessor":{"kind":"no-head"}},"requestDigest":"7a209f1d25cbf8bfe0c209950d4094f03fc8a2871da9f6ea61d7c417eb73b1fe","storageEpoch":"3333333333333333333333333333333333333333333333333333333333333333","subject":{"bootstrapConfigurationDigest":"0000000000000000000000000000000000000000000000000000000000000000","bootstrapId":"b","kind":"root","parentSessionId":"p","rootAssignmentId":"a","runId":"r","workspaceId":"w"},"workspaceId":"w"},"record":{"assignmentIdentity":{"bootstrapConfigurationDigest":"0000000000000000000000000000000000000000000000000000000000000000","bootstrapId":"b","kind":"root","parentSessionId":"p","rootAssignmentId":"a","runId":"r","workspaceId":"w"},"candidateDigest":"547170cf0dd1f2cafa8f1583fe6b4134b119dd9ca19d56aa1b8b3446d9a756c1","kind":"candidate"},"schemaVersion":1}
```

The claim-row commitment above is exactly 1839 canonical UTF-8 bytes
under the row.v2 domain, digest
`fa27e3252a6904acddbbae255d7302199caf0bfd2764f202ca82b6ce374e474a`. Reconstruct it from the full proof above and its proofDigest, using
the exact envelope just defined. Adding either a valid outer CommitRef or a
different outer CommitRef leaves that commitment unchanged; the separate
commit-link validator must reject the different one. Changing proof bytes/key/
proofDigest changes the commitment. Excluded links are never ignored for joins.

Format-2 initialization manifest canonical bytes are exactly 218, without a
trailing newline:

```json
{"changes":[],"claim":null,"commandKind":"initialize-store","commitSequence":"1","predecessor":null,"schemaVersion":2,"storageEpoch":"0000000000000000000000000000000000000000000000000000000000000000","workspaceId":"w"}
```

Its v2 manifest-domain digest is
`344ce7538ddfb4cc0d6af47b3d240c6476ade7570682172203fd7b94ed2ff60c`.
The separate #1030 v1 initialization vector remains 205 bytes and its old
domain/digest; swapping only the parser/domain is not a supported upgrade.

| Mutation/race fixture | Required oracle |
| --- | --- |
| Maximum claim sequence followed by another claim | First uses its unique identity; next fails closed without wrap/reset or new grant |
| Claim sequence gap after rollback | Valid historical gap; no phantom proof or missing-commit election |
| Reordered valid input keys / duplicate keys / rounded source u64 | Same canonical bytes after valid parsing / reject duplicates / reject lossy source representation |
| User-create proof names the newly issued stream as its expected predecessor | Reject: expectedUserSlot is the request's prior slot, not the generated reviewer identity |
| Child evaluation claims its evaluated target as producer | Reject unless that is the actual exact independent reviewer binding; worker self-review rules still deny self-evaluation |
| Same idempotency key with changed expected slot/request bytes | Conflict; never a receipt alias or a second proof |
| Attach correct commit link after hashes / replace claim authority | Hashes unchanged by excluded outer link / proof and row/manifest commitments change |
| Receipt and manifest point at different claim proofs | Unavailable, even if both proof hashes and source digests validate separately |
| Format-2 candidate/evaluation mutation has null claim | Reject; no v1/internal fallback or generic trusted-worker bypass |
| Crash before/after SQL commit | Complete old state or complete source/receipt/proof/lineage transaction; no claim restoration or blind replay |
| Proof bytes fit 16 KiB alone but combined slot metadata does not | Reject/rollback without acceptance; no quota increase, truncation or uncharged side table |
| Purge/dependency race or proof required by historical receipt | Exact owning dependency boundary protects validation material; no dangling accepted receipt |
| Research report, launch, internal finalization or purge uses producer claim | Reject action mismatch; keep its distinct authority/qualification contract |

Before any executable handoff, extend shared vectors to child reviewer/user-create/
correction/invalidation, all predecessor variants, Unicode escaping and maximum
source identities, missing/corrupt/duplicated links and exact worst-case slot/
command/row budgets. These ASCII vectors alone do not qualify serializers,
source schemas, native storage or the accepted authority ordering.

## Review and next gate

Proposed choices: checked generation-scoped ClaimIdentity, closed decision/proof
union, one receipt-owned proof row, format/header/manifest/row v2 domains, strict
source/request/receipt/commit joins and the explicit no-migration rule. These
need written technical acceptance; the already accepted authority tradeoff is
unchanged. No other domain registry or action class is made executable.

Next bounded design work is completing the child/UI and maximum-identity
cross-language vectors plus exact receipt/proof/command budget accounting,
including the existing unclosed lookup/domain metadata. Writer/producer/lease/
health/finalization ordering, native resource/durability evidence, #740 eligibility,
shared custody and separately approved executable plans remain gates. Keep
#744/#746 open; neither merging this proposal nor a valid proof hash authorizes
runtime code or a probe.


## Action/width fixture and budget pass — 2026-10-25

The [fixture catalog and measurements](../../validation/hierarchy-claim-proof-fixtures.md)
cover the five actions, child/UI predecessors and Unicode source reconstruction,
plus maximum local identity/counter/text widths. Combined logical value accounting
finds at least 19,153 bytes for the max child-evaluation slot and 17,439 for child
stream invalidation, before unclosed overhead, exceeding the existing 16,384-byte
cap. Those cases cannot be admitted under this representation; a proof fitting
alone is insufficient. No cap/profile/schema is changed here. Review exact
compact bindings or explicit profile limits and close key/index/command budgets
before claiming eligibility or accepting an executable handoff.

## Compact-binding follow-up — 2026-10-25

The [compact exact-binding proposal](2026-10-25-hierarchy-compact-bindings-design.md) responds to #1044's two 16 KiB receipt-slot overflows. Its proof-v2/storage-v3 encoding preserves
all 16 original full bindings by exact expansion and measures at most 9,503
declared logical metadata bytes across three rows, including a lookup row and
Base64 wire response. Original proof-v1/format-2 vectors remain unchanged.
This is a new schema proposal requiring review, not acceptance, a migration,
physical storage qualification, native authority evidence or runtime approval.
Whole-command, reserved/finalization and native platform gates remain open.

## Compact codec acceptance — 2026-10-25

The owner [accepted #1049's exact compact codecs](2026-10-25-hierarchy-compact-bindings-design.md#owner-acceptance-and-handoff--2026-10-25)
for further design/qualification: proof-v2 exact local references, the three
claimed-mutation metadata rows and full-key retry lookup, canonical original
receipt bytes, scoped format-3 commitments and logical-byte accounting. This
supersedes acceptance-pending wording only for those named choices. Earlier
proof-v1/format-2 encodings and fixture bytes remain historical and unchanged;
no mixed parsing or implicit upgrade is accepted.

Source/request/wire fields, quotas and accepted claim/lease policy are unchanged.
No physical/whole-command qualification, remaining domain/reserved/finalization
schema/default acceptance, migration, probe, executable plan or runtime approval
follows. The linked record lists the remaining gates and next bounded metadata/
budget design work. #740 stays deferred/no-go until November; keep #744/#746 open.
