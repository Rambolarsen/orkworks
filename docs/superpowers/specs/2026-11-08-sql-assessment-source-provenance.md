---
type: Design
title: SQL assessment source provenance
description: Proposed bounded durable source-creation representation and refusal policy for retained SQL assessment results.
tags: [orkworks, sqlite, assessments, provenance, recovery]
status: draft
workflow_status: owner-decision-pending
---

# SQL assessment source provenance

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Inspection base: `1ab168d35a2cfd8689d82f984b52a0fad278f765` (#1543).
Owner decision pending. This selects a private data/validation prerequisite;
it does not enable SQL assessment capture, publication, recovery or production.

## Outcome and verified gap

A retained assessment must identify the original SQL source creations that
influenced its immutable input. SQL recovery must refuse a tombstoned, missing,
reused or foreign source without assigning the saved result to current rows.
Keep existing repository, knowledge, access, settings and readiness fences.

Source inspection found:

- [Assessment input](../../../crates/orkworksd/src/taskmaster/assessment.rs)
  fingerprints complete observation/recommendation snapshots. The prompt uses
  bounded subsets, but the current identity includes the complete snapshots.
- The same module's `build_assessment_recommendation` emits empty direct session
  arrays and observation evidence. That does not establish source-free input.
- [Workflow observations](../../../crates/orkworksd/src/workflow_observations.rs)
  and [recommendation evidence](../../../crates/orkworksd/src/taskmaster/mod.rs)
  retain session IDs without originating creation identities. Captured ending
  writers know an original creation in memory, but do not persist it in the
  observation record. An observation timestamp is not a creation proof.
- [Assessment records](../../../crates/orkworksd/src/taskmaster/assessment_report.rs)
  support formats1–3. Pending publication stores a complete recommendation;
  recovery upserts it if absent, then saves Succeeded. Its fingerprint and
  process-local workspace instance contain no SQL epoch/creation proof.
- [Assessment storage](../../../crates/orkworksd/src/taskmaster/assessment_store.rs)
  caps the complete JSON record at64KiB. The SQL `exists` query returns current
  IDs only and cannot establish original creation provenance.

The [accepted producer contract](2026-11-08-sql-recommendation-producer-revocation.md#later-sql-aware-publication-qualification)
explicitly reserves this representation decision. The
[reopen barrier](2026-11-08-sql-recommendation-reopen.md) keeps legacy operations
unavailable; this proposal gives no bypass around it.

## Original input closure

An input-origin claim must accompany the immutable input when a qualified owner
first captures it, before inference or pending publication. It supplies original
`sessionId`/`createdAt` pairs and the original database epoch/workspace hash,
not IDs to resolve against the current database. Every input claim must share
that original SQL scope; even equal IDs/creations from another epoch cannot be
adopted. A newly captured source-free input also retains its selected SQL scope.
Current ID-only observations, recommendations and prior assessment records have
Unknown provenance. Do not backfill them from SQL, timestamps, runtime discovery,
report fingerprints or an empty output session list. Do not silently filter them
out of the assessment or call the reduced input equivalent.

The dependency set covers every session-origin input included in the fingerprint,
including input recommendations whose visible summaries carry no IDs. Each
qualified input supplies its full dependency closure, including inherited SQL
assessment provenance. Union those original pairs; reject different creations
for one ID. A qualified recommendation input accounts for its source/evidence,
affected/dismissal, target and completion-packet session references, plus inherited
assessment dependencies. These are input dependencies, not a change to existing
legacy deletion semantics. Readiness-focus context is included: an unqualified
focus cannot be treated as source-free because its JSON omits visible IDs.

Pure repository facts and signed knowledge may supply an explicit source-free
claim through a qualified application owner. Only a complete qualified input
with no session-origin dependencies yields an empty set. A model response,
untrusted file or deserialized provenance cannot assert that condition.

No source-origin producer upgrade is included in this first slice. There is no
production constructor that promotes legacy objects into origin claims. A
private fixture constructor may create trusted test claims only under cfg(test).
Real workflow-observation/recommendation/readiness origin capture needs a later
concrete qualification of its original owner and durable representation. Until
then, session-bearing SQL assessment admission remains unavailable.

## Durable representation and bounds

Choose a strict `SqlAssessmentSources` object with these camelCase fields:

| Field | Meaning |
| --- | --- |
| `schemaVersion` | Exactly1 |
| `databaseEpoch` | Canonical lowercase hyphenated UUID from the checked SQL header |
| `workspaceSha256` | Lowercase64hex SHA256 of exact stored workspace UTF8 bytes, as in SQL deletion identity |
| `assessmentId` | Exact original assessment/report ID |
| `inputDependenciesSha256` | Lowercase64hex digest binding the immutable origin-qualified input snapshot |
| `sources` | Complete sorted unique `{sessionId, createdAt}` pairs |

IDs and creations use the existing SQL deletion identity constraints: nonempty,
at most256 UTF8 bytes; session IDs contain only ASCII alphanumerics, `-` or `_`.
Compare creation bytes exactly, with no timestamp normalization. Sort sources by
session ID byte order; reject duplicate/noncanonical order on decode rather than
silently normalize saved records. Limit128 distinct pairs and16KiB for the
compact UTF8 encoding of one complete provenance object, plus the existing64KiB
limit for the whole assessment record. Each limit applies independently; a
maximal field combination need not fit the byte cap. Overflow is a typed visible
failure before save/publication, never truncation, omission or a larger store cap.

Compute `inputDependenciesSha256` with SHA256 over the fixed domain
`orkworks/sql-assessment-inputs/v1` followed by a zero byte and the canonical
`serde_json::to_vec` encoding of the immutable input-origin envelope. That envelope
contains ordered observation/recommendation/readiness input values and their
origin claims; object keys are recursively sorted, array order is preserved,
and invalid/nonfinite JSON is rejected. Exact source pairs participate. The envelope shape is exactly:

```json
{"schemaVersion":1,"databaseEpoch":"<original epoch>","workspaceSha256":"<original workspace hash>","observations":[],"recommendations":[],"readinessFocus":null}
```

The angle-bracket values above illustrate the fields, not a valid golden vector.
Scope is retained from the qualified origin claims, never filled from current SQL
on recovery. Each observation/recommendation array entry is exactly
`{"value": <existing typed JSON>, "sources": <sorted original pairs>}`.
Preserve the existing snapshot array order. A present readinessFocus has the
same entry shape; its value is the existing immutable ReadinessReference used
by the fingerprint, not transient display context. Its origin claim must still
cover dependencies of that context, which retains independent readiness checks.
The envelope excludes this digest and the newly produced SQL provenance object,
avoiding a hash cycle; input recommendations may retain their prior provenance.
Implementation golden vectors must pin exact bytes. This is integrity binding,
not authentication or proof that a legacy producer knew a creation identity.

SQL-aware input fingerprints use a new internal fingerprint domain/version and
include the complete provenance object along with existing inputs. Repository
discovery may refine its manifest/fingerprint as today, but must retain the
original dependency digest/pairs; it cannot refresh original sources. New source
inputs require a new capture and assessment, not mutation of a pending result.

## Record compatibility and durable ownership

Add AssessmentRecord format4 with required `sqlSources`. It supports standalone
or existing readiness references, retaining their independent validation. Keep
formats1–3 and their ordinary JSON behavior unchanged; they cannot carry SQL
provenance fields, even null, or become format4 through a default. Check raw field
presence at both `sqlSources` and
`pendingRecommendation.assessment.sqlSources` before optional-field decoding
collapses null into None, including format1's otherwise permissive storage check.
Only these typed protocol locations are reserved; prose containing the field
name is not SQL provenance.

For format4, retain `sqlSources` in every state, including no-proposal and
interrupted/failed states. Validate identity/bounds before save and after load.
Report ID, top-level ID and provenance assessment ID must match. A report's
fingerprint and pending recommendation assessment fingerprint must match as
required today. Extend AssessmentProvenance with an optional `sqlSources` that
new SQL recommendations must carry; when pending, it must equal the record's
object exactly. Legacy serialization omits the field.

The published recommendation keeps its full source object after pending clears
or the latest assessment report is replaced. The report alone cannot own
provenance needed by a surviving recommendation. Do not overwrite an existing
recommendation with different bytes or regress accepted/executing lifecycle
state during recovery. Parsing any source object yields data only: it grants no
store permit, SQL operation admission, current runtime ownership or publication.

A future SQL reader/recovery owner must refuse formats1–3 and missing/malformed
provenance before invoking legacy assessment recovery or recommendation I/O.
Preserve the original bytes and pending state with an explicit unqualified/
unavailable result. Do not silently clear a pending result, synthesize sources,
mark Succeeded, rerun inference or rewrite legacy history to manufacture a proof.
Ordinary JSON recovery retains its existing interruption/currentness behavior.

## Typed SQL qualification primitive

Add private bounded worker commands for capture and validation. Capture consumes
an original-input claim plus its expected source pairs; no IDs-only overload.
Within one read transaction, compare the current store header/workspace/epoch
to the original claim scope and validate each exact source row. A source must exist with identical creation and
`deleted=0`; a deletion job with a live row is corrupt, and tombstoned/missing
rows deny qualification. Active/closed/archived rows retain existing evidence
eligibility; this does not add an alive-only policy or freeze metadata revisions.

Capture produces the immutable source data above. Validation consumes saved
source data and freshly checks the same epoch/workspace/exact rows without
changing any field. Unavailable/busy/corrupt SQL is not an empty source list.
Even an empty set validates its header and input binding. Reserve command/result
bytes using existing queue budgets and deadlines; reject the full set before
SQL if either cardinality or encoding bounds fail. Reuse the existing header
and read-budget seams; add no SQL schema/table, generic raw query or long-lived
transaction. No shared application guard may be held across the worker await.

A returned validated value is deliberately not publication authority. A fresh
check alone cannot serialize a later file effect with deletion. The first slice
must not expose a recommendation read/write bypass or call legacy publication.

## Publication and deletion gates retained

A future publisher must retain original application/root/lease/evaluation and
source-owner authority; acquire the same recommendation operation for fresh
source validation and graph effects; and retain its grant through report staging,
recommendation publication and definitive settlement. It needs an opaque
application admission, with no caller-controlled success callback.

Before that path is enabled, SQL cleanup must recognize provenance-only source
dependencies even when the derived recommendation's direct arrays are empty.
Qualify both orderings: cleanup wins and later publication is refused; publication
wins and cleanup's fresh graph pass removes the derived dependency. SQL tombstone
admission can race while a publisher holds the recommendation permit, so a
point-in-time SQL check is not a universal no-later-tombstone guarantee. The
publication/deletion ordering and any action/delivery source checks require their
own reviewed integration before enabling SQL producers. No pending record may
remint provenance after cleanup removes the derived recommendation.

Existing repository/access/readiness/evaluation currentness, graph hashes,
redaction, payload bounds, safe startup, finalization, backup and activation
remain gates. Source provenance grants no PTY delivery, model/native execution
budget, maintenance permission, live session restart or production cutover.

## Owner decision and first deliverable

Accept format4, the bounded SQL source object, explicit Unknown/refusal policy
and the private codec/typed-worker qualification plan. Its first runtime slice
may implement strict data shapes, safe SQL-use rejection and copied-fixture
capture/validation/reopen evidence only. Origin-producing inputs and actual SQL
assessment publication/recovery stay disabled pending their separate qualification.
This costs availability for unproven old inputs and never discards them. No
runtime, race, durability or performance result is claimed by these written docs.
