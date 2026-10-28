---
type: Validation Evidence
title: Summary-detail source joins and compact packing audit
description: Narrow logical source-to-display join evidence, complete wire measurements and a conditional compact response upper bound.
tags: [orkworks, hierarchy, projection, evaluation, validation, budgets]
status: draft
---

# Summary-detail source joins and compact packing audit

Tracking: [#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Baseline: `8c64f284`; source contract is the
[accepted summary-detail read](../superpowers/specs/2026-10-26-hierarchy-summary-detail-design.md#scoped-owner-acceptance-and-review-handoff--2026-10-28).

**Disposition:** compact byte arithmetic and one narrow logical source join are
qualified as documentation evidence. Complete producer/source currentness and
runtime qualification remain open. No limit, codec or source digest changes.

## Reproduction and inputs

[summary-source-packing.json](summary-source-packing.json) retains input-file
fingerprints, fifteen complete display measurements, maximum-width root/child
requests, one logical source bridge and the compact-response calculation.
The independent Python generator uses only standard-library JSON/SHA-256:

```bash
python3 docs/validation/generate-summary-source-packing.py --output /tmp/summary-source-packing.json
cmp docs/validation/summary-source-packing.json /tmp/summary-source-packing.json
node --test docs/validation/summary-source-packing.test.mjs
node docs/validation/check-hierarchy-empty-output-profile.mjs
pnpm --dir docs docs:check
```

The source bridge uses the existing
[empty-output logical-store packet](hierarchy-empty-output-profile.json), never
an invented replacement for its result bytes. The fifteen display measurements
use the unchanged [component catalog](hierarchy-summary-vectors.json). Neither
input is promoted to native or complete source-valid producer evidence.

## Source audit and exact logical bridge

| Read value | Existing owner/source | Audit result |
| --- | --- | --- |
| Assignment and accepted reference | #744 complete AssignmentIdentity, canonical AssignmentResult and logical accepted-head row | Bridged for the existing root empty-output result only; digest, revision and repeated identities match |
| Candidate/result bytes | Existing #744 domain hashes and retained logical objects | Canonical candidate/result bytes, candidateDigest, retained result object and accepted head agree; no rewritten source object |
| Summary/material token | Existing logical summary row/checkpoint and #744 durable lineage contract | Copy exact material and revision 2; checkpoint, accepted head and summary commit links agree in this retained logical state |
| Criteria/rubric | Exact owning profile snapshots and existing snapshot domains | Reused unchanged and checked against material; the profile is a fixture witness, not proof of approved bootstrap admission |
| Evaluations, annotations and findings | #744 AssignmentEvaluation, evidence descriptors, stream heads/dispositions and outcome currentness | Empty in this bridge; the earlier display catalog has synthetic references without retained source objects, so its assessed/stale/disputed projections remain unqualified |
| Snapshot/lifetime and viewed state | Projection/publication owner, captured source-health tokens and real local presentation context | Synthetic context only; copying a source generation does not prove a current read lifetime, coherent publication or display |

The bridge joins a 744-byte canonical result to a **3,060-byte** Unassessed summary
response and a **980-byte** renderer request. No evaluation exists, every required
outcome remains Unassessed and no quality/pass is manufactured. The root accepted
result digest is the existing `32c35e61…3966c74c`, not a newly assigned display
identifier. Tests reject source-byte, retained-object, head, summary token/material,
missing/duplicate-row, checkpoint-link and display-digest drift. Paired
snapshot-substitution tests reject criteria/rubric replacements even after the
display digest and wire fingerprints are recomputed. A no-validation stub failed these
join tests before implementation.

[summary-source-packing.mjs](summary-source-packing.mjs) is a narrow documentation
assay of this logical packet. It does not validate arbitrary #744 source records,
authority/approval, every commit proof or a database. The existing logical-store
checker independently verifies its larger state/transition packet. Neither
checker establishes native storage, actual admission or source-health authority.

## Complete request packing

| Maximum-width accepted-result selector | Renderer bytes | Main-forwarded bytes | Limit per hop |
| --- | ---: | ---: | ---: |
| Root | 2,111 | 2,343 | 4,096 |
| Child | 3,039 | 3,271 | 4,096 |

Every ID uses the existing 128-byte ASCII maximum, digests/generations use 64
ASCII bytes, bounded revisions/counters use their widest decimal spelling, and
the accepted result repeats the full assignment identity. The result branch is
larger than `none`; child identity dominates root identity. The main addition is
its complete workspace/generation scope. Allowed ID characters need no escaping.
Thus these are compact structural width bounds for both accepted selector variants,
not examples with short IDs. Tests pass both hops through the existing component
checker. They establish no real admission/membership behind maximum-width IDs.

The original fifteen display responses range from **2,167 to 7,348 bytes**. Their
small size alone is not a general response-packing proof.

## Conditional compact response bound

This calculation assumes compact deterministic JSON with the existing field
sets, source-owned validity and the accepted serialized **2,097,152-byte material
ceiling**. It includes the complete response envelope and every detail member:

| Component | Conservative serialized bytes |
| --- | ---: |
| Material | 2,097,152 |
| Criteria: 32 full descriptions/IDs | 398,977 |
| Rubric: 16 full descriptions/IDs and widest fields | 199,294 |
| Evaluation references: 16 complete child subject/reviewer bindings | 46,465 |
| Outcomes: 49 targets × 16 maximum-width annotations | 288,807 |
| Finding sources: 512 full child-reviewer bindings, empty dispute lists | 834,561 |
| Completeness | 128 |
| Quality | 37 |
| Remaining envelope, selection, property names and punctuation | 3,380 |
| Effective dispute digest references, including conservative commas | 34,304 |
| **Total upper bound** | **3,903,105** |
| **Headroom below 4,194,304 bytes** | **291,199** |

Descriptions use six encoded bytes per raw UTF-8 byte, the worst JSON escape
expansion. Longest tags, false booleans, all eight fixed reasons, maximum counts
and widest decimal spellings dominate their shorter alternatives. All 49 shadow
targets use the longest target/domain forms. Child subject/reviewer bindings
dominate user/root forms. Repeated IDs and incompatible states in the calculation
are deliberate **length-only shadows**, not valid response fixtures. They may
overestimate combinations that owning source limits cannot realize.

The dispute term needs the **source-wide** constraint: every effective dispute
names exactly one finding in an exact other evaluation revision. At most 16
retained evaluation revisions each contain 32 disputes, so all finding-source
lists together contain at most **512** effective dispute references. Each digest
costs 66 encoded bytes plus a conservatively charged comma: 512 × 67 = 34,304.
Applying the per-finding list ceiling 16 independently to every finding would
ignore the owning source quota. The source contract and exact provenance must be
validated before relying on this tighter bound; the display checker alone does
not prove source validity or this source-wide dispute constraint.

Under these assumptions, even the conservative sum fits the existing response
ceiling. This qualifies compact encoding arithmetic, not a production-supported
slice, memory/work bound or guarantee that every permissible transport spelling
fits. Additional whitespace/escaping policy, framing or new fields must still
be counted and rejected at the unchanged ceiling. Source records, material/row
limits, depth checks, incremental allocation, cancellation and shared read budgets
remain independently binding. This result does not remove bounded limit failures
or amend the accepted contract's conservative unavailable behavior.

## Remaining producer qualification and next task

The R2 required-C1 versus optional-O pair still needs **one retained canonical
evaluation** bound to an accepted result, exact approved snapshots and independently
validated EvidenceSource descriptors, followed by two maintained source-health/
correction states. The same immutable source bytes must yield the distinct current
required outcome classifications while C2 preserves Needs rework. Full finding/
dispute provenance and invalidation/replacement streams need equivalent source
closure. Existing digest-shaped display references cannot fill these obligations.

The owning [source/publication mapping](../superpowers/specs/2026-10-20-hierarchy-read-recovery-design.md#source-ownership-and-publication-participation)
identifies logical participants, not a qualified implementation. A later plan must
name actual retained source records, source/dependency health proofs, coherent
capture/revalidation and every detail-only invalidation writer. Evidence custody,
current report heads, unavailable/recovery fencing and real boundary/presentation
races remain unverified. The current join deliberately uses no evidence path or
report, so it cannot qualify stale/disputed or assessed cases by extension.

Next bounded task: build the exact R2 source-record/correction-state packet and
check its hash/identity/evidence joins before claiming producer qualification;
resolve any missing source/custody schema in its owning contract first. Keep
#1089/#744/#746 open. #740, native/storage and separately approved executable
runtime-plan gates remain independent. No live route, model/native probe,
database/migration, new authority or launch eligibility is delivered here.

## Bounded review evidence

One read-only arithmetic review found no actionable omission/underestimate in
the conditional compact bound. A separate read-only source/evidence review found
no blocking join issue and requested stronger snapshot-substitution coverage;
those tests are included. An author checkpoint-link regression failed before
its narrow finalization-state check was added. Neither reviewer ran the suite or
qualified runtime/currentness enforcement, and no post-addition independent
rereview is claimed.
