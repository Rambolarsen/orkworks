---
type: Validation Evidence
title: ResearchReport components and R2 correction histories
description: Independent canonical report/evaluation bytes and logical head-correction assays distinguishing stale required evidence from stale optional evidence.
tags: [orkworks, preparation, reports, evaluation, hierarchy, validation]
status: draft
---

# ResearchReport components and R2 correction histories

Tracking: [#1131](https://github.com/Rambolarsen/orkworks/issues/1131),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089).
Source: [scoped codec acceptance](../superpowers/specs/2026-10-28-research-report-source-codecs-design.md#scoped-owner-acceptance--2026-10-28).
Baseline: `4d62d773`.

## What this packet establishes

[r2-report-vectors.json](r2-report-vectors.json) retains exact canonical component
bytes generated independently by Python standard-library JSON/SHA-256. It includes
five report versions across separate C1/C2/O streams, a separate URL-reference
component, captured raw excerpt bytes, five #744 EvidenceSource records and source
keys, five exact #742 normalized request-fingerprint payloads, one candidate/result,
one user evaluation, captured accepted-result/evaluation references, three logical
SummaryRowV1 stages and two alternative correction histories.

The reference/counter fields are syntactically complete. Their supplied approval,
configuration, membership and generation context is **synthetic and unqualified**:
the packet does not contain the full #741 bootstrap/configuration, #742 admitted
run/plan/authority or UI authorization proof behind those references. In particular,
opaque configuration references are not asserted to hash a retained configuration.
Do not describe this packet as a complete source-valid admitted run or a supported
producer. It qualifies selected canonical bytes and joins under captured inputs.

```bash
python3 docs/validation/generate-r2-report-vectors.py --output /tmp/r2-report-vectors.json
cmp docs/validation/r2-report-vectors.json /tmp/r2-report-vectors.json
node --test docs/validation/r2-report-components.test.mjs
pnpm --dir docs docs:check
```

Every stored component records UTF-8 length and raw SHA-256 fingerprint. Source
objects additionally use their existing domain hashes; raw fixture fingerprints
introduce no new source domain. The report hash excludes only contentDigest; its
stored canonical descriptor includes that property. #744 sizeBytes counts those
complete stored bytes, and sourceKey hashes the complete unchanged EvidenceSource,
not the report hash or excerpt digest. Coherent substitutions are tested directly,
including use of hash-input length instead of stored-descriptor length.

## R2 counterexample with one immutable evaluation

The canonical user evaluation is **2,192 bytes**, SHA-256 domain digest
`d96d06860d0f4c68125a5862a0f91da773e1d5c617b7f4b6e7ffca331c48fdf7`.
It cites original version-1 reports separately for required failures C1/C2 and
optional outcome O; its required-rework findings cite C1/C2's exact source keys.
Quality is explicitly Unassessed, with no invented score/evidence. The accepted
result has the fixture's explicitly empty declared-output profile. No lifecycle,
completion claim or report classification is used as a quality shortcut.

| Logical branch | Head advanced | Current required findings | Completeness | Overall / raw record | Material token | Response bytes |
| --- | --- | --- | --- | --- | --- | ---: |
| A: required C1 corrected | C1 version 1 → 2 | f-c2 only; f-c1's stale evidence excludes it from current material | Unassessed percentage; 1 unsatisfied / 1 unassessed | Needs rework / historical | 3 → 4 | 5,181 |
| B: optional O corrected | O version 1 → 2 | f-c1 and f-c2 remain current | Assessed 0/2 required satisfied | Needs rework / historical | 3 → 3 | 5,657 |

Both responses retain the same complete evaluation reference/digest. The evaluation
bytes still contain all original findings and claims. A correction creates a new
report descriptor/content digest/sourceKey; it does not rewrite the evaluation or
substitute the new report into an old reference. The retained C2 failure keeps
Needs rework in both branches. Optional O makes the raw record historical in B
without downgrading unrelated required outcomes or their material summary.

The 609-byte candidate and 735-byte result are hashed as complete #744 objects.
The five stream report descriptors are 1,054–1,064 bytes; the renderer detail requests are 962
bytes. All these selected components fit their existing ceilings. This does not
replace the [general compact-packing audit](summary-source-packing.md).

The report catalog is the union of bytes needed for two mutually exclusive
histories, not proof all versions were published in one live store. Per-branch
accepted heads are derived by replaying the exact normalized requests. Catalog
presence or a larger version number never elects a head. UI read-lifetime witnesses
are separate synthetic contexts; no live equal-version or publication claim follows.

## Closed report and captured transition checks

[r2-report-components.mjs](r2-report-components.mjs) is **documentation-only**.
It checks the accepted closed descriptor/finding/file+URL records, sorted ID sets,
ordered arrays, reference/question membership, combined paired-list counts and
serialized report/finding/reference/request limits. Captured excerpts are compared
as exact canonical base64/raw bytes, including decomposed Unicode and the inclusive
2 KiB component bound. Naming a file or URL never performs I/O or grants access.

The URI check follows the accepted ASCII RFC 3986 profile, preserving scheme,
escape and explicit port spelling. It performs no network/DNS lookup or normalizing
URL serialization. UTC calendar/precision/offset form is checked. Leap-second dates use the pinned
[IERS table through Bulletin 72](https://hpiers.obspm.fr/iers/bul/bulc/Leap_Second.dat)
(July 2026; published expiry 2027-06-28). Unknown/future `:60` dates are rejected
until an explicit evidence refresh. Tests distinguish known historical dates from
fabricated month-end leap seconds. Actual retrieval time and source custody remain
unqualified; tests perform no clock/source lookup. The URL component is a codec alternative and is never
published beside the conflicting file-based version in either R2 history.

`applyCaptured` is a pure logical transition assay **after** assumed captured
identity, authorization and rate admission. Its supplied lineage binding is not a
live capability lookup. It checks normalized author/stream/generation joins before
saved-request lookup, uses exact immutable expected-predecessor lineage for user
correction, returns exact retries before live task/run/head CAS and advances the
logical run/head once for new publication. Changed same-version bytes, stale CAS,
fixed-report-ID bypass, version gaps and a ninth version are rejected without
mutation. Child/user versions share one eight-version stream budget.

The returned `accepted`/`retry` objects and saved-request lists are test-harness
metadata, **not a new production receipt schema or persistence interface**. The
request-fingerprint payload, stream and ResearchReportHead shapes themselves are
those already defined by #742. Real authority revocation, sender validation,
request-rate accounting, receipts, quota admission and transaction durability are
not simulated or claimed. Mixed-generation resume/namespace/recovery histories
remain outside this selected assay.

The packet's exact SummaryRowV1 stages are prepared/no result revision 1,
accepted-but-unassessed revision 2 and evaluated revision 3. These are logical
material transitions, not database commits. The checker derives required material
from the immutable evaluation and replayed heads, verifies those three stages and
then permits only revision 4 for the material-changing C1 correction or retained
revision 3 for optional-only O. Self-consistent retokening and substituted result/
evaluation head references cannot bypass those joins.

## Evidence limits and producer handoff

The new suite was run against a no-validation stub first; all initial 72 tests
failed before implementation. Additional failing-first regressions exposed missing
material-token and captured-head joins. Direct sourceKey/size tests exclude stale
outer fingerprints as the reason for rejection. Independent regeneration/cmp and
LF-preserving checkout checks cover retained artifact reproducibility.

The captured snapshots, excerpt bytes and normalized operation kinds establish
component input consistency, not that a real user approved a run or a coding tool
enforced a role. ResearchReport classifications do not prove natural-language truth
or promote an evaluation to current. Likewise, the scenario's user-review identity
is a captured input, not evidence that Electron issued it.

The next qualification boundary is **publication effects**: an actual accepted
report correction must atomically bind its descriptor/request/receipt/head, exact
dependency affected set, required material/revision or unavailable fences and
snapshot invalidation before another current detail read. This packet proves no
such storage/worker/clock/authority boundary, crash behavior or dependency-index
completeness. Actual source retention/custody, complete admission, native/platform,
#740 capability and separately approved executable runtime plans remain gates.
No live tool/model/native probe, process, database, migration, HTTP/IPC route or
browser presentation is delivered. Keep broader #742/#744/#1089 work open.

## Bounded review evidence

One read-only source/join review found no blocking issue in the captured R2
records, material lineage or stated evidence limits. A separate codec/protocol
review found that arbitrary month-end leap seconds were accepted. New tests
failed first; the author then pinned known IERS positive leap dates and rejected
unlisted `:60` values. No post-fix independent rereview or runtime/time-source
qualification is claimed. Historical failing-first runs and manual regeneration/
checkout checks are execution evidence reported here, not cryptographic claims
embedded in the source packet.

## Report-publication prerequisite audit — 2026-10-29

The [R2 publication boundary audit](r2-publication-boundary.md) maps the required
and optional correction histories to atomic dependency/material/fence effects
and detail snapshot invalidation, preserving the old evaluation and retained
source edges. It identifies the explicitly missing research-report receipt/ledger
and mutable preparation/plan consumer codecs tracked by
[#1136](https://github.com/Rambolarsen/orkworks/issues/1136). This is a contract
inspection and qualification handoff, not complete changed-row fixtures, executed
race/crash tests or runtime eligibility. Existing component evidence is unchanged.
