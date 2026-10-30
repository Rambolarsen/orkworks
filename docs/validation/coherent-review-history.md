---
type: Validation Evidence
title: Coherent conditional child review history
description: Independent candidate/result, frozen review-input, evaluation correction, user invalidation, artifact receipt and retained-charge joins under accepted source schemas, without publication or custody qualification.
tags: [orkworks, hierarchy, evaluation, validation, artifacts, accounting]
status: draft
---

# Coherent conditional child review history

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
This closes the specific placeholder-predecessor gap identified in the
[source-accounting packet](assignment-accounting-witnesses.md) by adding a new
internally coherent **conditional source** history. Earlier catalogs, checkers,
source bytes, limits and authority policy are unchanged. No storage publication,
producer admission or runtime eligibility follows.

## Reproduce

```bash
python3 docs/validation/generate-coherent-review-history.py > /tmp/coherent-review-history.json
cmp /tmp/coherent-review-history.json docs/validation/coherent-review-history.json
node --test docs/validation/coherent-review-history.test.mjs
pnpm --dir docs docs:build
```

Use repository Node 22. Python emits independent canonical UTF-8/domain hashes,
raw artifact hashes, requests, receipts and byte accounting. The fixed Node
consumer validates those joins and invokes the unchanged R2 checker for its
captured report/evidence bytes. Both inherited input catalogs are fingerprinted;
LF attributes preserve the new catalog on Windows.

## Exact source history

The fixture declares a child worker identity and a distinct child reviewer in
another run of the same workspace. It uses the existing child identity and
candidate shapes, with explicit synthetic workspace/run binding. The candidate
has no output declarations and its result preserves its complete source with
revision 1, the candidate-domain digest and empty time/cost evidence. This is
allowed for the synthetic no-materialized-deliverable context, not evidence that
an actual assignment/configuration was approved or admitted.

The frozen criteria/rubric and three original captured R2 report evidence sources
are retained unchanged. The exact `ReviewTargetInput` selects the accepted child
result, no output bytes and those three sources, ordered by source-key digest.
Its 4,357-byte input bound includes the complete result, criteria/rubric and every
selected report's byte length. A required `assignment_evaluation` server-artifact
declaration supplies the existing 512 KiB per-version bound and scoped output
contract; these nested component objects are not a full approved v3 configuration.

Root targets are rejected because root results remain user-review only.
Necessary task/reservation/session/configuration differences are checked, but
actual role/profile eligibility, repository registration, allocation/effective
write-scope contributor exclusion, source authorization/freshness and read-scope
admission remain unqualified. There is no `eligible=true` assertion.

| Event | Exact source connection | Target retained record bytes | Additional reviewer artifact raw bytes |
| --- | --- | ---: | ---: |
| Candidate plus accepted child result | Complete source-domain result/candidate binding | 1,580 | 0 |
| Initial child evaluation | Revision 1, no-head, exact result and criteria/rubric digests | 4,536 | 2,956 |
| Same-reviewer correction | Revision 2 names actual revision-1 digest, with nonempty rationale correction | 7,726 | 6,146 |
| Explicit user invalidation | Exact current revision-2 digest; actor user; freezes the stream | 8,881 | 6,146 |

All earlier source bodies remain counted. Target record charges belong to the
worker assignment/run/workspace, including evaluations and disposition. Reviewer
artifact content remains separately attributable to the reviewer assignment/run
and workspace. The values are source-content occurrence totals, not ledger rows,
physical sizes, declared production artifact quotas or proof of atomic charging.
No reviewer result was published: target/result-payload byte contribution stays
zero and cannot be used to hide artifact-production bytes.

## Requests, receipts and artifact bytes

Each evaluation uses the existing `orkworks.evaluation.submit.v1` request and
`ReportingMutationReceipt`. Invalidation uses the Electron-only
`orkworks.user-evaluation.invalidate.v1`, including the exact joined result digest;
the disposition source does not acquire a new resultDigest field. Child sources
retain the exact reviewer and child-launch provenance. User invalidation does not
impersonate a child correction or reopen its run.

Artifact version equals evaluation revision. Each child receipt and artifact-read
reply names the same immutable artifact ID/version, exact canonical evaluation
length and raw-content SHA-256. That hash is distinct from the source's domain-
prefixed evaluation digest. Invalidation has no artifact. These are accepted
existing request/receipt/read-reply schemas, not an invented storage artifact row,
index or publication mechanism.

The harness rejects a repaired placeholder predecessor, stale invalidation,
wrong result/snapshot/reviewer/generation, user impersonation, root/self/cross-
workspace review, insufficient input budget, changed evidence/declaration, wrong
artifact hash/version/read content, changed request and a correction after
invalidation. Rehashed sources still fail their semantic joins. No authentication,
retry namespace, rate clock, live grant, revocation race or SQL rollback is tested.

## Accepted receipt contribution, not persistent rows

Under the explicit conditional assumption that this result has no previous child
review revision, its first two child-revision ordinals use slots 320/321 and
322/323. The existing accepted mapping reserves a write/invalidation pair per
revision. The initial write consumes 320; correction consumes 322; invalidation
consumes only 321, the stream's first paired credit. Slot 323 stays protected and
reserved. The stream's used/reserved byte contributions are respectively
16,384/16,384, 32,768/32,768 and 49,152/16,384 at each target-owner receipt scope.
These figures exclude pre-existing candidate/result/user credits and other
streams, and are not complete quota totals or serialized receipt-slot rows.

## Complete publication effect audit

| Source-owned effect | Defined connection used here | Missing complete transaction evidence |
| --- | --- | --- |
| Target immutable evaluation/disposition and stream head | Exact result, reviewer, revisions, predecessor and source-domain digests | Full retained target state, bounded ordinal discovery, head/CAS and source-owner command manifests |
| Reviewer artifact creation | Exact declaration, raw bytes, ID/version/size/hash in receipt and read reply | Closed immutable artifact/content/custody/index codecs, producer aggregate/staging limits and complete accounting effects |
| Receipt, claim and replay lookup | Existing request/receipt bytes and conditional paired-credit semantics | Exact full-key current-authority claim, slot/proof/lookup effects and complete 16 KiB packing for this history |
| Target record and reviewer production charges | Retained per-occurrence source sums, separate target/reviewer ownership | Reconciled owner ledgers and all command/whole-store/physical limits |
| Read/evidence/dependency protection | Frozen typed input and captured immutable report descriptors | Full configuration/plan consumer registry, incoming/outgoing membership, custody/retention and source-health/currentness publication |
| Summary and subject currentness | No aggregate outcome claim is made here | Full source-owned summary/checkpoint/lineage/CAS effects and atomic snapshot invalidation |

The storage registry defines evaluation/disposition objects but still gates the
reviewer artifact producer and broader consumer custody codecs. This packet does
not close that gap with generic rows or a caller-supplied affected list. There
is no complete SQL manifest or native transaction claim.

**Next design prerequisite ([#1176](https://github.com/Rambolarsen/orkworks/issues/1176)):** specify the minimal immutable evaluation-artifact
producer custody/content/index and finite aggregate-budget contract, with exact
ownership, retention/deletion joins and publication effects. Review any new codec,
quota or authority choice with the owner before dependent transaction fixtures.
Full target/reviewer configurations and source producer/admission, complete
manifests/denial/retry/race/native/physical qualification, #740 evidence and
separately approved executable plans remain open. Keep component issues open.
