---
type: Design
title: SQL workflow observation origin capture
description: Proposed original-owner capture and durable observation provenance without legacy backfill or assessment activation.
tags: [orkworks, sqlite, observations, provenance, ownership]
status: draft
workflow_status: owner-decision-pending
---

# SQL workflow observation origin capture

Parent: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Inspection base: `d4565340052e21d3de97128091908c513ee0d5b4` (#1551).
This is a proposed prerequisite, not implementation approval. Reuse the accepted
[assessment provenance contract](2026-11-08-sql-assessment-source-provenance.md).
It remains accepted; this proposal selects the first actual origin owner and a
new private observation representation and operation boundary.

## Outcome and scope

A newly recorded final-scan observation retains the original SQL workspace,
database and session creation that supplied its evidence, across reopen and
bounded segment rewriting. SQL assessment input capture can distinguish that
record from an unproven legacy observation without looking up today's creation
for an old session ID. No observation, recommendation or readiness store moves
into SQLite. Production remains JSON.

First implementation is source-only: a private qualified observation owner,
strict data reader and captured final-scan writer, plus the corresponding private
SQL observation-deletion path. No HTTP/production route or assessment scheduler
is enabled. Authenticated agent reports, ordinary live Peon reports,
recommendation origin producers and readiness origin producers remain gated.
Mixed legacy inputs make SQL assessment admission unavailable, even if this
one producer has qualified records. No filtering, migration or backfill follows.

## Verified source owners and narrowing

| Input/owner | What exists at the inspection base | Consequence |
| --- | --- | --- |
| [CapturedEnding](../../../crates/orkworksd/src/session_application/sql_writes/lifecycle/history.rs) | Captures original runtime, creation, root, workflow-store instance and lease before the final-scan wait | Suitable starting owner; add checked SQL scope before inference, never after the result returns |
| [Ending evidence](../../../crates/orkworksd/src/session_application/sql_writes/lifecycle/history/evidence.rs) | Registers a captured writer, validates original creation/ending phase, uses the session lane, grounds at most five candidates and retains file-job capacity/claim | Extend this flow; keep SQL/file waits outside application guards and cancellation settlement intact |
| [CapturedWriter](../../../crates/orkworksd/src/workflow_observations/captured.rs) | Creation keys the revocation permit, but the writer retains only session ID and validity | In-memory revocation is not durable origin data |
| [Observation store](../../../crates/orkworksd/src/workflow_observations.rs) | StoredObservation adds idempotency hashes; legacy open tolerates corruption, truncates tails and rewrites bounded segments | Do not run that opener before a strict SQL origin reader; protect provenance during every rewrite |
| [Recommendations](../../../crates/orkworksd/src/taskmaster/mod.rs) and [completion](../../../crates/orkworksd/src/taskmaster/completion.rs) | Mutable evidence/source/affected/dismissal/target/completion and inherited assessment dependencies | A later owner must propagate the complete closure on every mutation, including rollups; no empty-array shortcut |
| [Readiness](../../../crates/orkworksd/src/taskmaster/readiness.rs) and [focus](../../../crates/orkworksd/src/taskmaster/readiness_focus.rs) | Reads a projected recent observation list and stores its digest, then reconstructs saved focus | Missing session IDs in that projection do not establish source freedom; snapshot creation needs its own original closure |
| [Assessment input](../../../crates/orkworksd/src/taskmaster/assessment.rs) | Fingerprints full retained observation/recommendation snapshots | Do not drop unqualified inputs to make a SQL run eligible |

Rejected alternatives: backfilling old observations violates original identity;
using parsed assessment provenance as an origin grant confuses data with owner
authority; separate per-observation provenance files add file growth and a
second commit boundary. Keep origin and observation in one existing segment
record, while retaining the existing sequence-counter crash semantics.

## Original claim and immutable value

The captured final-scan owner obtains a sealed, non-deserializable observation
origin claim before inference. A bounded typed SQL read checks the captured
session creation and returns its original scope from the same transaction:
canonical database epoch and SHA-256 of the exact stored workspace bytes.
The claim retains the original root/lease, worker and application identities.
It is not constructed from a parsed observation, model output or caller-chosen
current session ID. The first read applies only to a newly captured input;
it is never a recovery/backfill operation on retained observations.

Check application/runtime/store identity before and after that asynchronous
read. Preserve the originally captured creation. Epoch/workspace replacement
before capture completes refuses the claim; bind against the checked opening
identity of the retained worker, not a newly adopted current header. Revalidate
that same original scope and exact live creation under the existing session
lane before file admission. A matching ID/creation in a different epoch fails.
No SQL I/O or file work runs under terminal-critical application guards.
Both original capture and pre-file revalidation use the accepted source-row
conditions in one bounded read transaction: original epoch/workspace, exact
creation and deleted=0. Missing/tombstoned/reused/foreign rows refuse with
Conflict; any deletion job paired with a live row is Corrupt even when its
creation looks equal. Closed and archived nondeleted rows keep existing
eligibility. These checks do not serialize a later file operation; the original
lane, application admission and file grant provide that separate ordering.

The claim accounts for the final scan's original session output only. Existing
grounding and candidate validation remain mandatory. Structured cross-session
inputs cannot use this constructor; they need a later complete-closure owner.
Session references inside untrusted prose are not converted to authority.

After validation assigns the observation's UUID, sequence, timestamp, source,
confidence and fingerprint, bind the resulting complete public
WorkflowObservation value. Preserve that value exactly while retained. Compute
`observationSha256` as SHA-256 of domain `orkworks/sql-observation-value/v1`,
then a zero byte, then compact UTF-8 JSON with recursively sorted object keys,
array order preserved and no nonfinite values. Exclude the origin wrapper and
internal idempotency fields from this digest. Existing payload/idempotency
hashes continue to perform their separate retry function.

## Private durable records and finite bounds

Use the existing per-session NDJSON segment and sequence file. The new strict
tag is `sql_observation_v1`, with exactly these fields:

```json
{"record":"sql_observation_v1","observation":{},"origin":{"schemaVersion":1,"databaseEpoch":"<original canonical UUID>","workspaceSha256":"<lowercase 64hex>","sessionId":"<original ID>","createdAt":"<exact creation>","observationSha256":"<lowercase 64hex>"},"idempotencyKeyHash":"<existing hash>","payloadHash":"<existing hash>"}
```

This shape is illustrative, not a valid test vector; observation is the complete
existing typed value. Reject unknown or duplicate protocol fields before a
Value conversion can erase them. Origin sessionId must match observation
sessionId and the segment name. IDs/creations use the accepted deletion/source
constraints (nonempty, at most 256 UTF-8 bytes; IDs ASCII alphanumeric, `_`, `-`).
Canonical epoch and lowercase 64hex digests follow the accepted assessment codec.
The observation UUID/sequence/source/confidence and existing candidate rules
remain valid; a value digest is integrity binding, not authentication.

New limits: at most 2 KiB compact UTF-8 per complete origin object, and 64 KiB per
complete tagged observation/tombstone line excluding its newline. Existing
1,000 observations/2 MiB per segment, 1,024 retry tombstones, 10,000 workspace
observations and 15-minute retry window remain. Count all origin/tag bytes in
segment size; preserve current bounded eviction policy without increasing caps.
Overflow refuses before counter/file changes, never drops provenance to fit.
Golden vectors pin exact value/digest and both new boundary rules.

Qualified eviction emits `sql_tombstone_v1` carrying the existing tombstone
fields plus an exact copy of origin. The copied digest still names the evicted
value; a tombstone does not reconstruct that value or become assessment input.
Unknown/legacy tombstones retain their existing representation and cannot gain
origin metadata during trim or retry. Rewrite/anti-spam paths preserve origins
of surviving qualified observations and copy them only into their own eviction
tombstones. No public WorkflowObservation or request wire field is added.

Strict SQL inventory has additional independent limits: 4,097 directory entries
(including sequence, temporary, unknown and empty entries), 4,096 segment files,
64 MiB aggregate bytes read and 65,536 nonblank segment lines (observations and
both kinds of tombstone). Charge every inspected/read entry and byte, including
legacy and tombstone-only segments; do not multiply the per-segment limits into
an unbounded workspace allowance. Keep at most the existing 10,000 selected
observations in memory while streaming bounded segments. Only the sequence
file and valid per-session NDJSON names are complete
inventory entries. Count temporary/unknown entries before refusing them;
nonregular files and incomplete inventory are unavailable, not omitted input.
An empty new directory may lack sequence. Otherwise sequence must be readable,
parse as u64 and be at least the highest retained observation/tombstone sequence;
a larger counter preserves existing failed-append gaps without repair.

Strict scan results additionally cap the compact encoded selected observation/
origin snapshot at 768 KiB. An existing top-10,000 selection that exceeds this
limit returns TooLarge; never return fewer selected records to fit. Reserve one
full 1 MiB slot through the existing application file-job budget before scanning;
use at most one 64 KiB line buffer and stream directory/segment enumeration.
The remaining reservation covers encoded intent/index allowance. No increase
to its 32-job/1 MiB-per-job/32 MiB-total limits is selected. Charge provenance in
writer intents too. These are encoded accounting limits; actual heap/native
resource qualification remains separate, and no physical memory proof follows.

Use non-following descriptor-relative regular-file reads and the original
request's 2-second file-job deadline, including queue, permit and scan time;
do not reset it per file. Reserve bounded blocking-job capacity before scanning,
and stream one bounded segment at a time. A synchronous regular-file syscall
may settle after the deadline: retain original resources and deny late delivery,
never spawn a replacement scan. Exceeding any count/byte bound returns TooLarge;
expiry returns Expired; all return no partial/empty assessment snapshot and
perform zero writes. These source-only support limits do not limit history
rows or claim that 100k history/assessment performance has been qualified.

## Strict reads, retries and compatibility

A SQL reader inspects complete bounded segments without invoking legacy open,
recovery, truncation, normalization or trimming. Malformed/unknown/duplicate
SQL protocol fields, interrupted lines, inconsistent filenames/identities,
incomplete reads and out-of-bound segments return visible unavailable/corrupt
results, preserve every byte and never become an empty snapshot. It may parse
ordinary legacy records as unqualified data for inventory; they cannot produce
an origin claim. Returning a qualified assessment input requires every selected
retained input to be qualified and its current SQL source creation to validate.
The eventual assessment union still has 128-source/16 KiB/64 KiB independent limits.

SQL idempotency is scoped to original epoch/workspace/session creation as well
as the existing key and payload. A duplicate qualified record or qualified
retry tombstone must match that original scope and payload; acknowledge its
original ID/sequence without rewriting, minting a new origin, advancing the
counter or recreating an evicted observation. A colliding legacy record,
unknown tombstone or foreign/reused origin returns an explicit unqualified or
conflict result; never upgrade it through a duplicate acknowledgement.
Existing ordinary legacy retry behavior remains unchanged in legacy mode.

An append failure may leave a sequence gap as today; it is not durable success.
A partial line is preserved and blocks strict SQL reads. No new automatic tail
repair is authorized. A complete admitted record may outlive caller loss;
settlement retains original resources until its actual finite completion and
reports committed/detached or failure through the existing owner. A timeout is
not permission to recapture scope or replay a different write.

## Shared owner and deletion ordering

Use an observation-specific canonical-root shared admission owner, following the
existing recommendation owner's lifecycle rather than introducing a generic
storage framework. All legacy observation operations that can read, repair,
trim, sequence, write or delete files must register before effects. Existing
clones and independent opens share the mode while any original owner is alive.
Legacy-to-legacy behavior keeps its existing serialization; SQL admission is
nonblocking Busy while any legacy operation is active. Once checked SQL opening
admits, mode is monotonically SQL and later legacy operations refuse visibly.
A failed pre-admission check makes no transition; a failure after admission keeps
revocation. No reset, fallback, strong global registry or new durable marker.

Legacy constructors reject any non-followed `format.json` or `session-store`
entry before mkdir, tail repair or trim, as the accepted recommendation refusal
floor already does. Inspection errors are not absence. Standalone SQL-tagged
segment evidence also refuses legacy parsing before effects; perform a bounded
non-following preflight rather than letting the tolerant parser truncate an
unknown final tag. This is a refusal floor, not #1244's full startup resolver.

The distinct private SQL opener validates existing worker/header/workspace,
original root and lease, admits the canonical owner, then strictly reads data.
It creates no database, adopts no AppState and grants no generic publication or
cleanup authority. Its opaque operation grant excludes overlapping SQL file
operations. Fresh opens after owner loss require the same checked SQL path;
known artifact evidence continues to deny legacy even if the weak owner expired.

Before final-scan publication, hold the original session lane, validate current
SQL creation and application identity, obtain that owner's operation grant,
and admit the retained FileClaim under short application checks. Move the grant,
lease and capacity into blocking file work. Keep existing writer revocation;
no validation-only read grants external I/O. File operations never reenter SQL
while holding an observation file lock. Registration, file work and definitive
settlement retain original ownership through cancellation.

The existing SQL Workflow deletion stage must use an exact-job SQL observation
grant from this same owner. Retain the complete SqlDeletionIdentity captured
before waits and compare it with the current header/session/job under the
original lane before granting removal. A replaced epoch, creation, job identity
or disappeared job refuses before any segment/counter/retry-state
change. Ordinary progress advancement within that same job is not identity
replacement; retry retains the existing stage-bit behavior. Completed means
job removal or loss of its matching tombstoned-row precondition, not merely
progress bits becoming set; do not capture a fresh job identity at removal. This is a new Workflow
boundary: today's special captured-identity comparison covers Recommendations
only. The new grant must enforce the captured identity for Workflow too; a legacy delete method is not a bypass. Tombstone,
original application reservation/root/lease, deletion progress and per-session
lane checks remain. An admitted observation batch finishes before deletion,
which then removes its segment and revokes the matching writer. Deletion first
prevents a delayed final-scan write from reintroducing it. Non-target segments,
sequence and unrelated retry state are preserved. No recommendation cleanup,
other recovery owner or finalization progress bit changes are selected here.

## Qualification and owner decision

The [plan](../plans/2026-11-08-sql-observation-origin-capture.md) separates the
strict data boundary from private producer/deletion composition. Tests use copied
fixtures, real SQL, reopened owners and bounded stalls. No live user history,
installed harness, power-loss, performance or production behavior is qualified
by this document or its example JSON.

The proposed decision is: accept the new tagged origin/tombstone representation
and bounds, original-before-inference capture, and shared SQL admission/legacy
refusal policy for source-only implementation. Recommendation and readiness
origin contracts remain separate; assessment scheduling/publication, actions,
full startup, backup/migration, native/hierarchy and activation remain gated.
