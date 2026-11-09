---
type: Design Contract
title: Checked SQL observation input owner
description: Proposed private application owner for bounded observation-only input qualification using original persisted origins and exact SQL source validation.
tags: [orkworks, sqlite, observations, assessments, provenance]
status: draft
workflow_status: proposed
---

# Checked SQL observation input owner

Parent: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243) (remains open).
Source pin: `f9608963` (includes classifier #1648).
This proposal requires owner acceptance before dependent implementation.
Reuse the accepted [input classification](2026-11-09-sql-assessment-input-classification.md),
[source provenance](2026-11-08-sql-assessment-source-provenance.md), and
[observation origin](2026-11-08-sql-observation-origin-capture.md) contracts.
The [session storage specification](../../../specs/session-storage.md) remains
authoritative; production selection remains JSON.

## Outcome and boundary

Add one private preparation operation to the existing bound `SessionWrites`
application owner. It can return a sealed, owned observation component whose
complete selected membership has original SQL origin and checked source rows.
It cannot construct production `CapturedInputSet`, `SelectedScope`, or
`OriginalInputs`. Recommendations, readiness, model execution, scheduling,
publication, routing, migration and activation remain outside this delivery.
An empty observation component proves only that this scan selected no
observations; it never proves a source-free complete assessment.

The existing classifier remains the policy owner. Its private module receives a
purpose-specific consuming factory for the new sealed application result. No
public generic qualified-input constructor, raw-pair grant, deserialization grant,
boolean authority callback or caller-selected current identity is introduced.
Names below describe contracts; private Rust layout is a builder choice.

## Source evidence and decision

| Source at the pin | Existing behavior | Required addition |
| --- | --- | --- |
| `session_application/sql_writes.rs`, `SessionWrites::bind` | Retains SQL worker, original workspace identity and metadata reader; requires a selected leased workspace | Capture the original workflow instance, strong lease and root identity for this operation and compare them at final admission |
| `sql_writes/lifecycle/history/sql_origin.rs`, `open_observations` | Validates one original producer source and reads a workspace snapshot; drops candidate/lane before returning | A separate complete-selected-source preparation operation; the old read receipt stays data-only |
| `sql_writes/observation_opening.rs`, `open` | Checks original SQL opening and retains snapshot/root/lease/worker; permits copied offline opening without current application workspace | Do not upgrade this receipt into assessment authority |
| `workflow_observations/operation.rs`, `SqlCandidate` | Canonical memory admission serializes file operations; dropping candidate releases busy state | Retain that admission through scan and source validation, then settle without holding it during future model work |
| `workflow_observations/sql_origin/reader.rs`, `Snapshot` | Strict bounded inventory retains selected values, order, optional origins and capacity | Consume these actual records without reconstructing claims from detached DTOs |
| `session_store/assessment_sources.rs`, `check_assessment_source_rows` | One bounded transaction checks scope and exact live creations, including deletion-job inconsistency | Reuse row-check semantics in a purpose-specific observation batch read without fabricating `OriginalInputs` |
| `taskmaster/assessment_sources/classification.rs` | Qualified claims and selected scope currently have test-only construction | Add only observation-component consumption of the sealed application result |

Choose a new operation on the original bound application owner. Retrofitting the
read receipts would grant authority to callers that never captured current
application ownership. Looking up current IDs would replace original creation
identity. Repeated scalar validation would lose the single-transaction batch
contract. A general provenance framework adds no needed capability here.

## Preparation and sealing contract

1. Before asynchronous waiting, capture the original metadata-reader Arc,
   workflow-store instance, workspace path, strong lease Arc, root identity and
   bound SQL worker. Require the currently selected workspace to match the bound
   owner. Capture the worker's cached original opening identity without I/O;
   validate it against SQL after reserving capacity and starting the deadline.
   Never adopt a later header returned by `opening_identity()`.
2. Reserve the existing full 1 MiB observation-read slot and create one original
   two-second deadline before admission or file work. All queues, blocking work,
   SQL validation, final sealing and delivery share that deadline. No retry
   refreshes it. Keep captured resources alive until the actual job settles,
   including caller cancellation; do not release capacity while work continues.
3. Acquire the canonical SQL observation candidate using the existing checked
   root/lease and short memory-only application admission. Keep its operation
   admission across the strict file scan and subsequent SQL await. This is a
   memory exclusivity flag, not a held mutex guard. Existing `FileClaim::admit`
   performs its sole Pending-to-Admitted transition here, before file work.
   Hold no application mutex,
   filesystem mutex guard or SQL transaction across unrelated awaits.
4. Scan the complete selected observation inventory with the accepted strict
   reader and original scope. Preserve actual value bytes/semantics, selection
   and order. Validate segment/value/origin binding using the existing codec.
   Legacy or missing origin makes the entire observation component Unqualified;
   preserve every selected value and do not filter, backfill or rewrite origins.
   Malformed or interrupted evidence remains explicit failure, never empty input.
5. For fully tagged input, derive the exact source-pair union from those original
   tags. Refuse conflicting creations, more than 128 unique sources, or a source
   check DTO exceeding 16 KiB before worker submission. Do not substitute current
   creation times. Empty selected input still requires checked original scope.
6. Submit one private typed observation-source batch read to the original worker.
   In one bounded read transaction, compare current database header with the
   worker's original opening, expected epoch and exact workspace hash, then check
   every supplied original pair. Exact creation, `deleted = 0` and no deletion
   job are required. A live row with a deletion job is Corrupt; missing, deleted,
   reused or foreign sources are Conflict. Nondeleted active, closed and archived
   sessions remain eligible. The result is data, not an admission token.
7. After the transaction, recheck original root/lease/worker liveness and deadline.
   Under short nonblocking memory guards, compare the original workspace,
   metadata reader and workflow instance. This is a separate final owner check,
   not a second FileClaim transition or a change to its state machine. The
   callback does no I/O. Only this checked application
   owner may seal `ObservationInputAdmission`, owning the actual snapshot,
   original scope and retained context. A stale owner, timeout or lost caller
   receives no usable admission. Check caller liveness and deadline again at
   delivery; failed delivery drops the result. Busy retries remain within the
   same deadline. FileClaim cancellation only wins before initial admission;
   after admission, loss of the receiver suppresses delivery while the actual
   job settles and releases its retained resources.
8. A consuming factory within the private classification owner accepts only that
   opaque admission, with no additional caller values or pairs. It creates an
   opaque `ObservationMembers` containing the actual qualified members in their
   original order and retaining their original context. The factory cannot mint
   global selected scope or complete assessment authority. Perform conversion and
   bounded encoding within the original job/deadline before delivering the
   component; the admission cannot escape as a later reusable minting capability.
   There is no Clone,
   Deserialize, detachable claim-list getter or unguarded constructor.

Unqualified output retains the complete original snapshot and capacity as data,
without a qualified admission. Other failures return explicit existing error
categories (Conflict, Corrupt, Busy/Expired, TooLarge or Unavailable as applicable);
no fallback read, fresh scope, implicit retry or partial qualified output occurs.
The operation is private and unused by production selectors in this delivery.

## Retention, bounds and deletion

The job and successful component retain the original worker, root identity,
strong lease, workflow owner and read-capacity reservation. These survive
movement into a future owner; exposing values for inspection does not transfer
authority. Release canonical file-operation exclusivity after preparation and
actual result delivery/settlement. Do not retain a busy file candidate across
future assessment/model work. Caller loss drops any sealed result and settles
actual work before releasing its resources.

Keep existing scan limits: 4,097 directory entries, 4,096 segments, 64 MiB read,
65,536 lines, 10,000 selected records, 768 KiB selected bytes, 64 KiB record/line,
and 2 KiB origin. Keep existing worker queue/read accounting and bounded reply;
the new command contains at most 128 pairs/16 KiB and returns only a small
validation result. A single original two-second deadline covers all stages.

Add an owner-local consuming snapshot adapter rather than cloning the complete
snapshot into a second input list. Transfer records in order, at most one bounded
record conversion at a time, retaining `ScanRequest::hold_capacity()` after the
snapshot payload is consumed. Share immutable scope data within the component
rather than copying epoch/workspace strings into every retained member. Count
accepted compact envelope bytes with a bounded writer before additional encoding;
refuse component encoding above 1 MiB. Keep selected payload (768 KiB), one-record
conversion scratch (64 KiB), the source DTO (16 KiB), and existing scan bookkeeping
within the existing encoded reservation. This is encoded accounting, not a claim
that Rust allocations occupy exactly 1 MiB. The full classifier must still enforce
its accepted 1 MiB complete-assessment cap when a later owner combines components.
No public drain API or configurable budget is required.

SQL validation is point-in-time. A tombstone visible in that transaction refuses
the component. A tombstone committed afterwards may make it stale; it does not
change what the original input was. Later typed assessment capture/publication
must revalidate exact source rows under its own accepted fences. This proposal
provides neither a long-lived SQL transaction nor a guarantee against later
deletion. Canonical file admission serializes managed observation file mutations,
not every independent SQL metadata mutation.

The existing stored origin digest binds identity and value; it is not a signature
against a local actor able to forge canonical files. Parsing that tag alone
continues to grant nothing. The new checked owner is the explicit trust boundary
for accepted persisted origins, with original opening/root/lease and exact-row
checks. No new origin is invented or persisted on reopen.

## First delivery and acceptance evidence

After owner acceptance, use a concise implementation plan for this bounded slice:

1. Add the private bounded batch read and shared internal exact-row checks; retain
   existing assessment and scalar observation validation behavior and error mapping.
2. Add the checked bound-owner preparation job, consuming snapshot adapter and
   opaque admission/component boundary. Keep existing read owners data-only.
3. Exercise real producer records through this owner into the classifier in tests.
   Only a test assembler may supply the remaining trusted complete context;
   production complete-input constructors and call sites remain absent.

Required evidence includes valid multi-source and empty components; preserved
order/value/original pair binding; legacy/mixed inventory refusing the entire
component even if the same ID exists in SQL; and malformed evidence refusing.
Test replacement worker/header/root/lease/workspace/reader/workflow instance,
missing/deleted/reused rows and inconsistent deletion jobs. Gate the race before
the SQL read, after it, and before final sealing; demonstrate the stated deletion
semantics, original deadline, capacity retention through cancellation and actual
settlement, and release of file exclusivity after preparation. Test source-count,
DTO, component and complete-envelope bounds at their edges. Use actual sealed
owner output in positive classification tests, not manually rebuilt known claims.

Before PR handoff, run focused owner/store/classifier and existing observation
regressions, repository verification and required runtime code review. Keep
migration/activation and whole-assessment authority explicitly gated. Acceptance
of this proposal authorizes only this private observation-component slice.
