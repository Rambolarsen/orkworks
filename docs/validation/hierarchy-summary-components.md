---
type: Validation Evidence
title: Hierarchy summary-detail component fixtures
description: Independent synthetic request and response bytes with structural, display-metric and local viewed-identity checks for the accepted summary-detail contract.
tags: [orkworks, hierarchy, projection, evaluation, validation]
status: draft
---

# Hierarchy summary-detail component fixtures

Tracking: [#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Source: [scoped owner acceptance](../superpowers/specs/2026-10-26-hierarchy-summary-detail-design.md#scoped-owner-acceptance-and-review-handoff--2026-10-28).

## Packet and reproduction

[hierarchy-summary-vectors.json](hierarchy-summary-vectors.json) retains fifteen
synthetic display components. Each has exact renderer request, main-forwarded
request and successful response JSON, UTF-8 byte count and raw SHA-256 fingerprint.
Python standard-library JSON/SHA-256 produces the canonical fixture bytes
independently of the JavaScript checker. These raw fingerprints introduce no
source digest domain. Criteria/rubric digests use their existing #744 domains.
Result/evaluation/configuration references are explicitly synthetic digest-shaped
values, not hashes of qualified source records.

```bash
python3 docs/validation/generate-hierarchy-summary-vectors.py --output /tmp/hierarchy-summary-vectors.json
cmp docs/validation/hierarchy-summary-vectors.json /tmp/hierarchy-summary-vectors.json
node --test docs/validation/hierarchy-summary-components.test.mjs
pnpm --dir docs docs:check
```

[hierarchy-summary-components.mjs](hierarchy-summary-components.mjs) is a
**documentation-only component checker**, never a production decoder, producer
or preload API. Tests pass independently retained goldens through both request
shapes and the complete response. Mutation tests exercise rejection rather than
matching source text. The initial no-validation stub failed rejection/viewed
behavior before implementation. Additional failing tests exposed and corrected
Unassessed-versus-conflict material, material/annotation contradiction, declared
rubric ordering and preservation of an adverse claim beneath a dispute. A bounded
correctness review found evidence-reason/full-assessment and Meets-material
consistency gaps; regression tests failed before their fixes. The stale fixture
now takes an explicit original claim separately from source state. A coverage
review suggested relaxing criterion order/descriptions; those suggestions were
rejected against the owning #744 stable-ID snapshot order and #741 required
CriterionSnapshot description fields, now pinned by recomputed-digest tests.
No post-fix independent rereview or complete source qualification is claimed.

The bounded JSON reader rejects duplicate decoded keys (including escaped aliases),
malformed UTF-8/Unicode, unsafe/non-decimal integer tokens, extra data and depth
above 32. Transport whitespace, object order and valid JSON escapes remain legal;
canonical encoding belongs to retained goldens and snapshot digests, not a new
transport requirement. Raw request/response byte caps are checked before decoding.
A documentation check may inspect an already bounded whole buffer; it does not
qualify production incremental allocation/serialization or cancellation.

## Executed component coverage

| Area | Component check and evidence boundary |
| --- | --- |
| Exact selection and scope | Complete root/child assignment, accepted head, known decimal summary revision, run, snapshot and derived main scope; a synthetic fresh-overview witness is required |
| Source snapshot context | Exact criteria/rubric bytes and existing domain hashes; required criterion set/order, optional descriptions and declared rubric dimension order are retained |
| Annotation and provenance joins | Every selected evaluation is accounted for per target; unique response-local digests, full overview references and logical finding origin/dispute joins; no digest-only lookup |
| R2 stale C1 versus stale optional O | Same retained evaluation references and Needs rework, different current required outcomes/completeness; no raw evaluation bytes or source correction lifecycle is established |
| Conservative metrics | Known failure plus Unassessed, legacy zero required, ratings 0/2 and 2/3, missing dimension and rating 3 over a failing dimension; no averaging or denominator shrinkage |
| Findings | Current/disputed material, origin reviewer and exact local dispute references; original adverse claim stays visible beneath effective Unassessed; actual evidence health remains a producer obligation |
| Local viewed identity | Explicit, visible, complete, painted, same-request witness plus current exact response; no result, failed/incomplete/stale response, hidden/partial presentation or changed scope yields no mark |
| Token semantics | Nonmaterial rating 0→2 changes snapshot but retains material/viewed identity; A→B→A with a new summary revision does not reuse the earlier identity |
| Bounds | Inclusive 4 KiB/4 MiB UTF-8 wire and depth limits, collection overflow, 2 KiB description overflow, schema types, unknown/duplicate fields and digest/reference drift |

The metric checker validates display consistency **given** synthetic producer
material, annotations and `fullyAssessed` decisions. It does not implement #744
aggregation from immutable evaluations/evidence. Known currentness, eligibility,
conflicts and source provenance cannot be established from display objects alone.
The synthetic `fresh` and presentation flags are local test witnesses, not new
transport records or proof that a browser displayed text or retained focus.
No view state is persisted and no action/grant is issued.

Criterion ordering/descriptions inherit [#744 snapshot hashing](../superpowers/specs/2026-10-04-assignment-evaluation-design.md#criteria-completeness-and-quality)
and [the source snapshot fields](../superpowers/specs/2026-10-27-root-bootstrap-source-schemas-design.md).
Descriptions for optional criteria are included; the description field itself is
required. Rubric dimensions retain declared order rather than criterion ordering.

## Remaining qualification

This packet does not establish source-valid result/evaluation records, complete
bootstrap/admission inputs, durable summary lineage, currentness/dependency health,
writer participation or detail-only invalidation coverage. In particular, the R2
pair does not prove the shared immutable evaluation's canonical source bytes, and
the dispute vector does not prove finding/dispute evidence or source-head changes.
Those need owning producer fixtures and full source/custody closure.

Authentication/sender/origin validation before lookup, sidecar/UI authority,
shared 60-attempt/two-slot admission, two-attempt/five-second assembly, cancellation,
failure envelopes and denial non-disclosure require real boundary implementations.
The fixed failure codes are reused from the existing detail contract; this task
adds no failure-response schema. Generic reader byte-limit tests do not prove
all source maxima fit complete valid responses; material/row packing, worst-case
work/allocation and production writer/native/storage qualification remain open.
The pure viewed witness does not exercise a browser render commit, scrollable
material surface, accessibility, focus protections or the bounded 64-run cache.

Next is a producer/source and packing qualification slice joining exact current
source records and maintained dependency proofs to these display vectors, with
real writer invalidation and presentation-race coverage in the later approved
runtime plan. Keep #1089/#744/#746 open. #740 remains deferred/no-go until November;
no profile, model/native probe, database/migration, runtime route or launch
eligibility follows from this component evidence.
