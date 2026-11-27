---
type: Design
title: Evidence for atomic evaluation publication
description: Proposed evidence for complete physical effects, bounded admission, and recovery of evaluation artifact publication.
tags: [orkworks, hierarchy, storage, evaluation, qualification]
status: draft
---

# Evidence for atomic evaluation publication

[Issue #1176](https://github.com/Rambolarsen/orkworks/issues/1176) tracks qualification of
the [evaluation artifact producer](2026-10-30-evaluation-artifact-producer-design.md).
The initial write, correction, and user invalidation already have logical effect recipes.
This proposal defines the evidence needed to show that a source-owned transaction
applies those recipes in full, fits its bounds, and leaves a recoverable outcome.
Gate 2 remains **Investigate**.

The evidence unit is one named profile, one workspace and storage epoch, and one
exact request. The packet identifies the source/build revisions, the applicable
codec and catalog, the command and scoped lookup key, and the old header H. When the selected contract uses a paired view, also record
the ordinary storage epoch, companion generation G, and the actual paired owner
and connection generation.
A profile must account for its complete physical catalog and the source owners
whose state it uses. The accepted [fresh-pair index design](2026-11-05-paired-membership-index-design.md#scoped-physical-design-acceptance--2026-11-05)
and [research physical profile](2026-11-24-research-physical-profile-design.md#scoped-owner-acceptance--2026-11-24)
provide their own scoped decisions. This proposal selects no new layout or format
and uses the producer's accepted format-4 logical commitments.

## Establish the complete old state

Each command needs a source-owned inventory pinned to H. For every required
lookup, record the full typed key or bounded range, its owning record families
and physical mappings, the controlling source generation, and the exact value,
absence, or range termination. Include retained source, configuration, capability
and current authority identities with the accountable writer and cutoff for each
contributing source. Cover the frozen v3 configuration, criteria, rubric, evidence,
review-read authority, allocation, and joined result and snapshot identities under
the owning rules. Use the [source-admission contract](2026-11-05-evaluation-source-admission-design.md#admission-freshness-and-source-writer-participation)
and the [source and membership audit](../../validation/evaluation-artifact-source-closure.md#logical-lookup-closure-required-by-the-three-recipes)
to enumerate the required groups.

Cover the exact target result, producer and frozen declaration, current head,
immutable mapping, retained evaluation and artifact versions, dispositions,
receipt reservations, directly required proofs, both directions of custody
membership, and direct metadata and source groups. Include the owning ledgers,
summaries, checkpoints and pending lineage. An absent head or mapping requires
the accepted original-history absence checks. For each bounded partition, the
qualification oracle compares index results against authoritative records and
retained origins using a separate enumeration path. It must detect an omitted
authoritative record even when both indexes agree. An empty result, a page without a verified end, or a membership
pair without its owning record leaves the proof incomplete.

Revalidate the captured read set, including absence and range boundaries, under
the source-owned publication fences at the same H. For a paired profile, recheck
H/G and the paired owner/connection before admission, prove the exact old/new
paired views, and require full correspondence at one H/G before a reopened pair
is available. A matching H alone cannot establish freshness of another owner. The profile must explain
how it reaches closure within the accepted bounds. The packet uses the existing
authority rules for artifact materialization and exposes only the evidence
needed to verify commitments.

## Compare intended and observed effects

Derive the expected effects from the verified old state and the accepted producer
recipes. Include the complete Q,S,D,P,K union and coalesce each changed logical
key to its original-before and final-after commitments. Check the header and
new commit proof as separate control effects. For each changed logical key, the
selected profile must enumerate the required physical row and index effects.
Derived index effects enter the physical comparison and cost accounting and their
owning codec governs manifest membership. Retained peers keep their commitments
and links.

| Operation | Distinguishing effects to prove |
| --- | --- |
| Initial write | Prove complete original absence and the exact no-head precondition. Publish evaluation, artifact, head, immutable producer/declaration mapping, custody edges, receipts and proof together. Consume the next shared target-result write ordinal and reserve its paired invalidation credit. |
| Correction | Prove the exact predecessor, next version and immutable mapping. Publish the new version and its reciprocal edges, and retain prior versions and proofs. The next shared ordinal has a new write and reserved invalidation pair. Its paired credit stays unspendable. |
| User invalidation | Prove exact current-head user authority and resolve the first-write pair through the original evaluation, consumed write slot and proof. Insert the immutable disposition and consume only that original paired credit. Retain the head, mapping, versions, custody edges and correction pairs. Apply the owning current-result and learning exclusion. |

Include every actual source-owned dependency, ledger, summary and lineage effect.
Coalesce shared ledger keys by owner and resource while charging target and
producer occurrences under their distinct rules. Account for physical index
maintenance and for the owning snapshot invalidation before newly current
projections become readable. A detail-only correction can invalidate snapshots
while retaining their peers' unchanged material revisions.

Obtain the observed effects independently of the recipe planner. The evidence
must account for all writes in the transaction through the closed physical
catalog, and independently read the old and resulting values at their exact
headers and applicable paired generations. Compare the complete key sets and original/final commitments, including
index entries, controls, and preserved neighbors. A comparison limited to keys
listed by the planner cannot detect an extra write. Internal storage work with
no logical effect still counts toward native physical costs. An additional
logical change, a missing reciprocal update, or an unexplained retained-row
rewrite fails the comparison.

Report three cost views for the same command. Discovery has its separate
4,096-observation and 16 MiB limits, including unsuccessful probes, continuations,
and the required revalidation. The complete command has its 4,096-changed-row
and 16 MiB limits under its accepted packing rules. Physical accounting covers
actual row/index work, controls, packing and native resources under the selected
profile's bounds. The packet must show exact totals at each applicable owner
and bound, including costs excluded from canonical commitments. A missing term
leaves admission unproven. See the [membership bounds](2026-11-03-child-consumer-membership-design.md#proposed-finite-discovery-profile)
and [writer qualification evidence](../../validation/hierarchy-writer-storage-qualification.md#storage-and-crash-evidence-packet).

## Prove refusal, recovery and retry

| Observed outcome | Evidence required |
| --- | --- |
| New request denied or transaction known to have rolled back | Establish the complete old durable publication state and unchanged protected credits, with no partial hierarchy publication. Record the disposition under the existing conflict, unavailable or capacity rules. |
| Claimed request commits | Establish the entire new state, including the exact scoped lookup, canonical receipt and direct proof links. Show the accepted claim/close ordering and native writer lease through the required cutoff, then the ordered projection publication. |
| Claim or durable/response disposition is unresolved | Keep the authoritative unavailable fence until the claim's exact outcome is proved. A rollback requires native disposition, exact scoped lookup absence and the complete protected old state. A commit requires the original receipt, commit and direct row/origin proofs with the complete new state. Use the outcome's exact header and applicable paired generation. |
| Exact authorized retry after response loss | Resolve the original canonical response under current authentication and the existing response-commitment rules before evaluating a new predecessor or charge. Demonstrate the saved result even when current pools are full. Changed request bytes under the same scope follow the conflict rules. |

Cover source changes while the hierarchy header is unchanged, revoke-first denial
and claim-first close with the original claim and required live lease. A native
lease loss follows the existing fence and disposition rules. The
[authority owner](2026-11-04-evaluation-publication-authority-design.md#concrete-closing-and-lock-integration)
and [claim proof](2026-10-25-hierarchy-claim-proof-design.md) contracts govern these
events. Resolve proof links by their exact keys. A durable commit and successful
authorized response are separate observations. Exercise both sides of response
commitment relative to close. An exact response committed before close may be
delivered later. A response commitment attempted after close is denied, including
when the earlier claim is still allowed to settle.

## Review and qualification handoff

An independent reviewer should be able to reject a packet with agreeing indexes
that both omit a retained consumer, a changed source or companion generation at
the same H, an extra key absent from the expected set, an invalidation that spends a
correction's reservation, or a command that fits its changed-row budget but
exhausts discovery. At the existing crash boundaries, the reviewer should be
able to distinguish a proven rollback, a proven commit with lost response and
an unresolved outcome using the same evidence.

The proposed decision is to accept these evidence requirements for written
qualification. The next written checkpoint is an inventory for one candidate
profile mapping each required source, effect and bound to its owner and evidence
path, with unsupported entries recorded as unavailable. A full-state and packing
exercise needs a concrete closed profile and separate authorization for its
execution. This document supplies no database, migration, fixture, runtime route
or native/model probe. The separate [Windows live-reader proposal](https://github.com/Rambolarsen/orkworks/pull/2222)
retains its own authorization boundary. #1176 remains open.
