---
type: Validation Evidence
title: Offline research review-task components
description: Independent finite review-task source examples under the accepted #1908 contract, with supplied reviewer/authority facts and explicit unqualified boundaries.
tags: [orkworks, research, review, sources, qualification]
status: draft
workflow_status: review-components-only
---

# Offline research review-task components

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: the merged review-task contract plus the acceptance record
(`b9f2524d8acf`).

The owner [accepted the five review-task choices](../superpowers/specs/2026-11-18-research-review-tasks-design.md#scoped-owner-acceptance-—-2026-11-19)
for offline components only on 2026-11-19. These are separate decoded examples
and supplied-fact oracles, not an event sequence, complete store, publisher or
authority implementation. Gate 2 remains **Investigate**.

## Artifacts and source boundary

- [Independent Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-review.py)
  reads the unchanged prefix and startup packets; it never calls JavaScript.
- [JSON oracle](research-review-components.json) retains the two-task plan with
  the optional `reviewTask` member, the ordering denial, the adapted reviewer
  configuration with recomputed digests, one-task and two-task collection
  shapes, the review-input snapshot with cited excerpts, the consistent review
  outcome with its request/receipt family row, the stale-review and
  reviewer-only replacement views, the typed summary citation and effect
  labels.
- [JavaScript checker](research-review-components.mjs) independently rebuilds
  finite source bodies from the original packets, validates exact
  keys/values/domains/joins/digest exclusions and wrapper widths, and supplies
  the admission/citation helper oracles.
- [Tests](research-review-components.test.mjs) regenerate exact UTF-8/LF
  bytes, exercise the semantic mutations and denial cases, and pin literal
  domains, bounds and labels.

## What the packet establishes

- The declared reviewer task consumes the researcher task's exact approved
  scope reference (`ResearchScopeRef` embeds the researcher task key), with
  the digest-free `research-review-input` delivery entry and the ordering
  denial for a configuration that embeds a snapshot digest.
- The review-input snapshot binds the exact selected report evidence source,
  delivers the complete approved question set, and carries only the cited
  excerpts; a report with no file-kind references yields an empty
  `citedExcerpts` array.
- The immutable outcome carries one status per approved research question;
  the family row is keyed by the reviewer TaskKey with request and receipt
  retained and no duplicated body; exact retry returns the original receipt
  before live CAS.
- A corrected report stales the review while the reviewer expectation row is
  unchanged; replacement runs through the reviewer-only replacement plan
  variant (`researchDependency` carrying the original researcher TaskKey,
  scope and configuration; `batches=[[replacementTaskId]]`), including the
  same-report verdict-replacement case, never an automatic re-review.
- The summary cites the typed `ResearchReviewEvidenceSource` within the
  unchanged 128 combined ceiling; an empty `reviewReferences` still requires
  proven no-required-review membership; a conflicting review is never cited
  as satisfying the required-review gate.

## Supplied and unqualified facts

Reviewer child authority, generation, epoch, owner, lease, health and request
rate; current Main shown context; actual capture origin, UI approval,
reservation/allocation/attachment/runtime authority; complete
source/consumer/index membership; original H/C/proofs; resource ledgers and
actual effects; the launch chain that produced the reviewer child; #741
capability/profile qualification. Matching digests and repeated source bytes
establish none of those facts. Source fingerprint hashes cover documentation
bytes; CommitRef changeDigest hashes explicitly unqualified labels. No wire
decoder, header, manifest, row/change hash, runtime reducer, native/model
probe, database, migration or executable product plan is supplied. The
clarification, execution-proposal and terminal-completion packets are later
separate bounded tasks under their own acceptances.

## Verification and measurements

Node 22 `node --test` runs all prior documentation packets unchanged (2,147
tests before this packet; the packet's own 14 regressions pass, including
independent regeneration equality, semantic-mutation denials, supplied-fact
helpers and literal domain/bound pins). The generator-derived measurements
record the exact key plus committed occurrence projection for the retained
rows; they are not storage charges or a legal transaction witness.
