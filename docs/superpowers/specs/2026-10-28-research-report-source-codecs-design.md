---
type: Design Proposal
title: Research report source codecs for evidence fixtures
description: Proposed closed ResearchReport leaves and exact descriptor joins needed before R2 report-correction source fixtures.
tags: [orkworks, preparation, reports, evaluation, codecs]
status: draft
---

# Research report source codecs for evidence fixtures

Tracking: [#1131](https://github.com/Rambolarsen/orkworks/issues/1131), owning
[#742](https://github.com/Rambolarsen/orkworks/issues/742), consumer
[#744](https://github.com/Rambolarsen/orkworks/issues/744) and
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspected baseline: `00f8c4e9`.

**Status: proposed for scoped owner review.** The owner authorized the R2 source
packet after the [source/packing audit](../../validation/summary-source-packing.md).
Inspection found the report's nested descriptor shapes still described in prose.
This document proposes that prerequisite; it does not treat fixture authorization
as acceptance of new source fields, leaf/default choices or source validity.
No dependent report-vector/checker, runtime or model/native work is delivered.

## Gap and selected boundary

The owning [report contract](2026-10-04-orchestrator-preparation-design.md#research-question-and-task-contracts)
names findings, evidence references and explanatory arrays without their closed
nested field sets. Its [stream/head/correction protocol](2026-10-04-orchestrator-preparation-design.md#current-report-head-and-corrections)
and request fingerprint are already exact. #744's
[report EvidenceSource](2026-10-04-assignment-evaluation-design.md#candidate-result-and-evidence-sources)
requires an exact stored descriptor identity, size and content digest. Neither a
hash of invented nested fields nor digest-shaped placeholders close that join.
No other ResearchReport leaf codec was found in the inspected specs, ADRs or
runtime declarations; this is a source-definition gap, not a runtime experiment.

Close the existing descriptor with one bounded finding record, file/URL evidence
variants and explanatory text arrays. Keep the existing report domain/version,
stream/head objects, request fingerprint, generation/author rules, count/byte
ceilings and correction authority. Referenced excerpts remain separately retained
source material; no inline full artifact, fetch operation or path capability is
introduced. Concrete custody/admission and runtime interfaces remain separate.

An alternative is an opaque report body. That would leave source-field validation,
classification/evidence joins and canonical-byte claims unqualified, so it does
not meet this fixture prerequisite. A file-only initial codec would support the
R2 example but leave the already described URL branch undefined; the proposal
closes both branches, while the first R2 packet can use only file references.

## Closed descriptor and inherited scalar types

Use #741's existing ASCII ID (1–128 bytes), lowercase SHA-256 Digest,
#742's plain bounded JSON integer counters and generation strings. `Text` is
nonempty well-formed UTF-8 through 2 KiB, preserving exact spelling/normalization.
Reuse the [UTC Time representation](2026-10-17-hierarchy-projection-payload-design.md#common-types-and-strict-validation)
for retrieval time: valid UTC RFC3339, `Z` or `+00:00`, optional 1–9 fractional
digits and at most 35 ASCII bytes. Preserve source spelling, never invent now.
All records reject duplicate/unknown properties, invalid tags/encodings and
out-of-bound values before hashing. No coercion or truncation.

`ResearchReport` has exactly these required members:

| Members | Representation / rule |
| --- | --- |
| `schemaVersion` | JSON integer 1, following #742's durable-record encoding |
| `reportId`, `reportVersion` | ID; integer 1–8 |
| `runId`, `planId`, `planRevision`, `taskId` | IDs; plan revision 1–64; exact approved research assignment |
| `reservationId`, `childSessionId`, `configurationDigest` | IDs/Digest resolved from that assignment |
| `sidecarGeneration`, `launchGeneration` | Existing lowercase 64-hex immutable producer/lineage identities |
| `questionIds` | Unique sorted ID set, 1–32; exactly the approved task's question-ID set; immutable across corrections |
| `findings` | Ordered ResearchFinding[], 0–32; preserve submitted answer order |
| `uncertainties`, `blockers` | Ordered Text[]; proposed combined maximum 16 entries |
| `evidenceReferences` | Unique ResearchEvidenceReference[] sorted by `id`, 0–64 |
| `recommendations`, `limitations` | Ordered Text[]; proposed combined maximum 16 entries |
| `author` | Exactly `{kind: "child"}` or `{kind: "user"}`, set by the existing authenticated operation |
| `contentDigest` | Existing report-domain digest defined below |

All arrays are present, including empty arrays; no nullable member is added.
Do not add `workspaceId`, task-version/CAS fields, timestamps, receipts, currentness
or bearer/grant data to this object. Workspace remains the sidecar-owned stream/
storage context consumed by #744 ReportIdentity. Expected task/run/head values
remain in the existing request fingerprint, not the immutable report descriptor.
This proposal interprets the two paired 16-entry limits conservatively as combined
limits; that clarification is a new choice for review, not a claim about prior
wire behavior. An empty finding list may report a blocker/uncertainty; it does not
mean all questions were answered or an assignment passed.

## Finding and evidence leaves

`ResearchFinding` has exactly:

| Member | Representation / rule |
| --- | --- |
| `questionIds` | Unique sorted nonempty ID set, at most 32; subset of report.questionIds |
| `answer` | Text |
| `evidenceReferenceIds` | Unique sorted ID set, 0–64; every ID resolves exactly one report evidence reference |
| `classification` | `established`, `inferred` or `unverified` |

The whole serialized finding remains within the inherited **2 KiB** limit,
including its IDs and punctuation. `established` requires at least one resolved
reference; `inferred` and `unverified` may lack references and retain their label.
A resolved reference or this classification alone does not establish truth,
source custody, independent review or a #744 assessed outcome. Do not reinterpret
research findings as #744 required-rework findings or reuse their identity space.
No new stable finding ID is needed: the report is the immutable evidence object.
Per-question completeness remains a preparation/evaluation obligation, not an
inference from the presence of a finding or an empty blocker array.

`ResearchEvidenceReference` is exactly one of:

| Variant | Exact members |
| --- | --- |
| File | `kind: "file"`, `id: ID`, `path: Text`, `revision: Text`, `location: Text`, `excerptDigest: Digest` |
| URL | `kind: "url"`, `id: ID`, `url: Text`, `retrievedAt: Time`, `excerptDigest: Digest` |

The whole serialized reference remains within the inherited **1 KiB** limit.
That total cap wins over individual text/ID maxima. File `path` uses the existing
root-relative scope-path rules: forward slashes, no absolute/drive path, empty,
`.` or `..` component, backslash or NUL. `revision` identifies the exact immutable
source revision retained by the source owner; `location` is a non-executable
locator within it. The approved research assignment supplies the repository/
worktree/source scope; a report cannot grant access by naming a path/revision.

For a deterministic proposed URL profile, validate the ASCII `URI` grammar in
[RFC 3986, Appendix A](https://www.rfc-editor.org/rfc/rfc3986.html#appendix-A),
with case-insensitive `http`/`https` scheme, explicit authority and nonempty host,
no userinfo, and, when present, a nonempty decimal port in 0–65535. Reject malformed
percent escapes and raw non-ASCII/control/space/backslash input; this codec takes
an already encoded URI, never silently converts an IRI. Preserve exact spelling,
including query/fragment, escape case and explicit port. Do not percent-decode,
normalize, resolve redirects or round-trip through a normalizing URL serializer. The existing approved
external-source scope must separately authorize that exact source. Both variants
are plain display data, never executable markup, shell input or a fetch token.
Existing prohibitions on secrets/full transcripts remain binding.

`excerptDigest` is SHA-256 of the exact nonempty raw excerpt bytes retained
separately under the declared source scope. The proposed component qualification
profile bounds each excerpt at **2 KiB raw bytes**, measured before any base64,
JSON escaping or UTF-8 normalization; no normalization is allowed. The complete
report can therefore refer to at most 128 KiB of distinct excerpt bytes for one
version (64 references × 2 KiB), before custody metadata. This is a new bounded
fixture/source-qualification choice for owner review, not an approved production
retention allowance or exception to #742 run/evidence accounting. Source/custody
owners must separately close aggregate admission, retention, sharing, deletion and
recovery before runtime implementation; no report profile alone authorizes storage.
An absent, empty, over-profile-limit or unverifiable excerpt cannot qualify that
reference as established/current in this component profile. It is not the whole-file digest, report content
digest or #744 source key. A descriptor alone proves none of those bytes exist.
Before source qualification, validate the exact excerpt, revision/retrieval and
scope joins under the owning custody mechanism; missing/unverifiable bytes stay
unavailable. Do not fetch a URL or inspect a live path merely to repair a fixture.

## Serialization, descriptor size and #744 joins

Use #741's canonical serializer: UTF-8 object-key order, validated array order,
compact deterministic JSON, exact integer tokens and no Unicode normalization.
Question/reference-ID sets and the evidence table use the sorted orders above;
answer and explanatory arrays preserve their order. Each sorting rule above is
an explicit proposal for this formerly open leaf contract.

1. Compute `contentDigest` as SHA-256 of `orkworks.research-report.v1` plus one LF
   followed by the complete canonical descriptor **without only contentDigest**.
   Include `schemaVersion`, every nested leaf, generation and sidecar-set author.
2. Store the complete descriptor **with contentDigest** as canonical UTF-8 bytes.
   Both the stored descriptor and its complete submission/correction request
   independently obey the existing **32 KiB** ceilings; a descriptor fitting its
   own cap does not imply the surrounding request fits.
3. #744 report EvidenceSource.sizeBytes is the exact byte length of that stored
   canonical descriptor including contentDigest, not the hash input, wire-string
   escaping, excerpt size, source-key wrapper or domain prefix.
4. Its contentDigest equals the report's contentDigest. Its complete ReportIdentity
   maps every existing lineage member plus sidecar-owned workspaceId. Derive
   sourceKey over the unchanged complete #744 EvidenceSource and existing domain.
   No bare report hash or lookup ID substitutes for that source key.
5. The existing ResearchReportHead resolves this exact descriptor/version/digest
   in the exact stream. Reading a receipt or retained bytes does not establish
   head currentness, custody or current authority.

The existing stream/head/request-fingerprint field sets and domains are unchanged.
Request fingerprints continue to include the accepted descriptor **with** its
contentDigest and the exact expected head/versions. Child/user generation checks,
exact-retry-before-CAS, bounded rate/storage accounting and durable dependent
fencing remain owning #742 obligations. A user correction preserves assignment,
question IDs, scope and predecessor lineage while setting `author.kind: user`;
it cannot become a child-authored report. No report correction rewrites #744's
immutable evaluation or silently rebinds its source keys.

## Scoped fixture handoff after acceptance

No new source goldens/checker precede acceptance of these descriptor/leaf choices.
After acceptance, the bounded R2 packet should include:

- One exact accepted result and approved criteria/rubric context, plus one canonical
  user evaluation, preserving root user-review and child eligibility boundaries.
- Separate report streams for C1, C2 and optional O, so correction of one report
  cannot accidentally stale a different outcome sharing that report. Retain exact
  excerpt/reference bytes and declare which source/custody facts remain synthetic.
- Original/corrected descriptor bytes, heads, expected-head/request fingerprints
  and #744 EvidenceSource/sourceKey joins. Use the same evaluation bytes in both
  histories; do not substitute corrected report versions into its references.
- History A corrects only C1; history B corrects only O. Keep C2 current in both.
  Required findings and outcomes reflect their own exact evidence health: C1's
  stale finding is excluded from current material in A, while C2's failure remains; optional
  O cannot downgrade current C1/C2 in B. Both raw evaluations may be historical.
- Structural/hash/order/size mutations, wrong author/lineage, wrong stream/head,
  changed same-version bytes, stale CAS and exact-retry versus current-head cases.
  Pure logical fixtures cannot establish actual authorization or atomic recovery.

Existing 8-version, 32 MiB run metadata, head/receipt and request-rate ceilings
remain. Complete approved source membership/custody, actual currentness/dependency
writers, coherent publication/native/storage and separately approved executable
runtime plans remain unqualified. #740 stays deferred/no-go until November.

## Owner decisions and review boundary

Review these proposed choices before component fixture work: (1) the closed report,
finding and file/URL leaf shapes, including per-finding question links and combined
paired-list bounds; (2) ordered answer/text arrays versus sorted ID/reference sets,
classification/reference requirements, the precise ASCII HTTP(S) URI profile
and 2 KiB per-excerpt component limit; and (3) complete stored-descriptor size
including contentDigest for the #744 join.

Acceptance would authorize documentation component fixtures and further source
qualification only. It would not accept a native role profile, custody mechanism,
runtime implementation, migration, model/native probe or broader lifecycle design.
Keep #1131/#742/#744/#1089 open until their scoped handoffs are resolved. This
proposal is source-definition work; no R2 source-validity or live report is claimed.

## Bounded written review

A read-only builder check reported “nothing important left” for planning the
documentation fixture task, excluding the explicitly pending owner choices and
runtime/custody gates. A separate correctness review requested an explicit excerpt
limit and suggested precise URL acceptance rules. The author added the proposed
2 KiB component excerpt bound and RFC 3986-based ASCII URI profile above. These
remain proposals; no post-edit independent rereview, component test or source
qualification is claimed. No old technical definition or validation artifact was
changed to establish this codec by implication.

## Scoped owner acceptance — 2026-10-28

The owner answered “yes, go ahead” after the concrete PR #1133 proposal and
these choices were presented: closed report/finding/file/URL records and
ordering/count rules; the 2 KiB raw-excerpt component profile and explicit URI
rules; and the exact stored-size/hash join into #744. These choices are accepted
for documentation component fixtures and further source qualification. This
supersedes pending-owner wording above for those named choices only.

The accepted technical definitions are unchanged by this record. Approval does
not establish captured source truth/custody, actual UI approval or runtime/role
authority, complete admission, report-head publication or native/storage
qualification. #740 and separately approved executable runtime plans remain
gates. Build the bounded canonical R2 evaluation/evidence/correction-state packet;
label synthetic context and unverified obligations explicitly.

## Accepted-codec component packet — 2026-10-28

The [report/R2 component packet](../../validation/r2-report-components.md) pins
canonical report and #744 source/evaluation bytes, normalized correction requests
and two logical head/material histories under synthetic captured context. It
implements the accepted documentation-fixture handoff without qualifying actual
approval/admission, source custody, report publication, #740 or runtime behavior.
Existing technical definitions and prior fixture artifacts are preserved.
