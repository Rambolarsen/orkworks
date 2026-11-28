---
type: Qualification Audit
title: Initial evaluation publication trace in the fresh pair
description: Source-only trace from original result and reviewer preparation to the ten core evaluation writes and their proof boundary.
tags: [orkworks, hierarchy, evaluation, publication, qualification]
status: draft
---

# Initial evaluation publication trace in the fresh pair

This follows the [candidate inventory](evaluation-publication-profile-inventory.md)
for [#1176](https://github.com/Rambolarsen/orkworks/issues/1176), at
`7e01f0e8e9a126ae3e539c669ddb01c6ac6c6d02`. The candidate is the accepted
fresh hierarchy format 6 and ordinary schema 2 pair. The scope is the first
`evaluation-write` for one child reviewer and one accepted child target.
The [production opener](../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs)
still refuses the profile before inspecting its path.

## Trace the admitted prestate

Let T be the target child assignment, R its exact accepted ResultRef, P the
distinct reviewer child assignment, and d the frozen evaluation declaration.
E and A are the new revision-1 evaluation and version-1 artifact. These names
refer to full canonical identities. The paired view binds both epochs and
headers H/G to the same retained owner and broker connection.

1. The [accepted fresh-pair preparation contract](../superpowers/specs/2026-11-05-fresh-pair-preparation-design.md#scoped-contract-acceptance--2026-11-06)
   supplies the written seed, original root/parent and allocated-child attachment
   path. Its owning proofs establish an original preparation. The evaluated
   target still needs its own accepted-result publication.
2. The [result-finalization recipe](../superpowers/specs/2026-10-25-hierarchy-candidate-command-manifests-design.md#complete-command-effect-recipes)
   supplies the logical route from T's candidate and outcome slot to R, the result
   head and the exact accepted response. The original result object and its owning
   proof must resolve R, with the relevant payload, source and receipt joins.
   The fresh-pair preparation contract supplies no witnessed format-6 outcome
   for that event. The [conditional review history](coherent-review-history.md#exact-source-history)
   contains source bytes for R but supplies no admitted paired commit. **R is the
   first unsupported join after the written preparation boundary.**
3. Given a proven R, P must resolve through its own original plan, approval,
   reservation, allocation, attachment and child-assignment source. Its frozen
   configuration, review-read evidence, independence, scope and declaration d
   must agree with the same R. The actual current capability and authority cutoff
   remain with the [typed source and publication owners](../superpowers/specs/2026-11-05-evaluation-source-admission-design.md#admission-freshness-and-source-writer-participation).
4. Given both joins, the owner would prove exact no-head CAS and absence of the
   P/d mapping and contradictory history, including original receipts and orphan
   rows. Then it would derive the next target-result slot across all reviewer
   streams and verify ledgers, summaries, checkpoints, pending lineage and retained
   consumers at H/G. These checks use [bounded source-owned discovery](../superpowers/specs/2026-11-03-child-consumer-membership-design.md#coherent-discovery-and-publication).

The remaining trace is conditional on that prestate. It classifies the accepted
keys and proof obligations without supplying absent rows or authority.

## Locate the ten core writes

The [producer recipe](../superpowers/specs/2026-10-30-evaluation-artifact-producer-design.md#one-evaluation-write-outcome)
defines the effects. The [paired catalog inventory](child-preparation-catalog-reconciliation.md#full-logical-inventory-and-original-attach-applicability)
names each owning logical family. In the table, an object reference includes its
source digest, and an object key is the existing full typed key.

| Core effect | Candidate family and key | Direct join to verify |
| --- | --- | --- |
| Insert E | `objects`, E's ObjectKey | Canonical evaluation binds T, R, P, revision 1 and its source evidence. |
| Insert A | `objects`, A's ObjectKey for full P/d/artifactId/version 1 | Artifact body resolves E's full EvaluationReference and canonical content size/hash. The raw content hash and both source-domain digests retain their own meanings. |
| Insert stream head | `heads`, evaluation kind with full T, R revision/digest and child reviewer P | Proven no-head precondition and exact E reference. |
| Insert immutable mapping | `evaluation_artifact_streams`, full P and d | Exact R and artifactId agree with A and the original response descriptor. |
| Consume write slot | `receipt_slots`, T and slot 320+2q | Original canonical response and committed mutation/claim agree with E and the request. |
| Reserve first invalidation slot | `receipt_slots`, T and slot 321+2q | Protected reservation binds R and P; it shares the target-result ordinal q of the write. |
| Insert claim proof | `claim_proofs`, full ClaimIdentity | Accepted ClaimProofV2 expands T and P in their original roles and binds the actual generation and no-head decision. |
| Insert retry lookup | `mutation_lookup`, RetryKeyDigestV1 | Exact retry scope joins the claim, consumed slot, request digest and response. |
| Insert content dependency | `dependencies`, A, E and `evaluation-artifact-content` | The exact retained evaluation supplies A's canonical content. |
| Insert origin dependency | `dependencies`, E, A and `evaluation-artifact-origin` | E resolves its exact immutable artifact. |

As specified, q is 12 times (R.resultRevision − 1) plus the next retained child
admission index for R. Distinct streams share that index. The ten rows are the
minimum new-write core. Additional source evidence edges and the actual
coalesced Q,S,D,P,K effects follow the accepted recipe. Replacing the captured header at H and
inserting the new commit proof are separate control effects. Unchanged peers keep their original links.

## Locate the physical proof boundary

The [paired index contract](../superpowers/specs/2026-11-05-paired-membership-index-design.md#proposed-derived-hierarchy-structures)
specifies the derived owner stamps and full-key membership edges, including the
reverse B-tree over the same rows. The publisher would derive their exact deltas
from the owning rows and closed reference recipes. Identity bindings and
contributors change only when their owning sources change. The existing
`artifact_read_metadata` projection is keyed by A's full ObjectRef. It joins
the producer source, declaration, R, E and descriptor to the exact artifact and
evaluation row digests and commits. Derived physical effects belong in the
observed write set and cost account. Header and manifest stamps use the existing
control proof equations.

The producer recipe uses format-4 header, manifest and row commitments.
The [format-6 contract](../superpowers/specs/2026-11-05-child-preparation-authorization-design.md)
defines separate storage commitment domains and retains the nested source,
receipt and claim codecs. The initial evaluation command still needs a
complete format-6 commitment and proof mapping.
The [format-4 test reload registry](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/reload.rs)
lists the evaluation tag, but the registered name supplies no candidate writer
or physical outcome. Each format uses its own storage commitment domains.

The next compatibility decision needs the exact format-6 owning proof path
from accepted result finalization through the first evaluation. It must account
for original source and receipt links, complete net row/index effects, the
header/proof controls and independent before/after evidence. Preserve each
nested codec and its digest. The present trace gives no authorization for a new
codec, fixture, migration or native run. Until the missing proof path and the
actual participating owners are established, the candidate remains unavailable
and Gate 2 remains **Investigate**.
