---
type: Validation Record
title: Private ordinary-session SQLite adapter
description: Source-only storage foundation, regression coverage and remaining integration gates for ordinary session history.
tags: [orkworks, sessions, sqlite, storage, validation]
status: stable
---

# Private ordinary-session SQLite adapter

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243), under the
[accepted storage contract](../superpowers/specs/2026-11-01-sqlite-session-storage-design.md).
This is a source foundation, with no production format selection or live data
migration. Existing `MetadataStore` and organization files remain authoritative.
The issue stays open until the remaining typed internal writers and application
integration are ready; this adapter alone cannot authorize activation.

## Implemented boundary

The private `session_store` module owns canonical JSON and derived SQL columns,
FTS5, organization and identity-bound deletion tombstones. Search maintenance uses
an explicit indexed session key as the FTS rowid, avoiding text-identity scans. Its worker alone owns
one connection. Staging initialization and existing-store open are distinct;
open never creates a missing database. Unknown versions, workspace mismatches,
corruption and persistence failures return errors instead of an empty store.
Age-retention eligibility has its own indexed nullable timestamp derived only
from valid `lastActivity`, preserving the existing policy independently of history
sorting. Invalid activity times remain age-protected. Retention result reservations
cover worst-case JSON escaping of exact identities.
No activation marker, JSON mirror, replay deletion or native capability is added.
The future selector must retain the selected workspace lease around this owner.

Current typed commands cover create, revisioned agent patch, exact detail,
live-ID summaries, bounded history/search, batched existence, age-retention candidates,
organization changes and deletion progress. Agent mutations reuse the current
field/source/plan validators. Canonical records preserve unknown object members (including nested objects and unchanged typed lists),
revision strings and immutable creation identity. Import-specific provenance,
manifest and organization loading remain migration work.

Internal metadata commands also cover Peon scope merging, harness identity
reports, provider context and user-title changes. Current JSON owners and the
isolated SQL worker execute shared pure reducers in
`metadata/internal_mutations.rs`; source/age arbitration is not duplicated in SQL.
The SQL command checks the latest record, mandatory creation identity and optional
expected revision inside its transaction, then applies the same read-time metadata
normalization as the JSON owner before evaluating the intent. True no-ops preserve bytes, revisions
and scope clocks, including at counter exhaustion. A same-identity harness
last-seen update deliberately advances the revision, matching the JSON contract.
Historical internal writes invalidate history in the same transaction.

Lifecycle transition/finalization commands reuse the same metadata reducers as
the JSON application. They retain pending terminal outcomes and final observation
snapshots, evaluate stale/duplicate transition guards against the latest record,
and invalidate history on entry to or exit from history. Creating and ending
records remain active in the index; an explicit Closed choice does not hide a
later active transition, while Archived remains protected.

Conditional Codex hook wait/completion commands require an opaque process-local
attention proof from this exact worker/session/creation identity and the current
matching root/source/status tuple. They edit the latest record, preserving newer
descriptive work. Accepted attention writes revoke old proofs even when values
are identical; work-only writes preserve them. Proof publication follows commit,
and failed writes/no-ops retain the old proof. Ending retires it; reopening a
database never reconstructs process ownership from persisted fields. The existing
attention-owner identity/counter implementation supplies this proof; it is not a
durable credential or evidence of runtime control. The bounded worker accounts
for serialized proof/envelope bytes along with the canonical result.

These commands are metadata primitives. The existing JSON Peon/harness wrappers
still own events, checkpoints and follow-up identity capture. The SQL worker does
not append external files or execute arbitrary callbacks. Production SQL routing
must preserve those outer operations and their actual partial-failure semantics;
this does not establish SQL/event atomicity or complete producer integration.
Title authentication, live label epochs, runtime/source reservations and native
eligibility remain with their current owners. Capturing a reported native ID does
not authorize native startup or clearing.

The worker admits at most 32 queued mutations and eight reads, reserves serialized
intent plus possible result bytes against 32 MiB, and rejects intents over 1 MiB.
Reservations survive execution and unread result delivery. Eight FIFO mutations
are followed by one waiting read. Startup ownership is guarded before the readiness await; dropping a queued
ticket competes atomically with the worker execution claim. Queued expiry uses
a two-second execution cutoff; cancellation after that cutoff never asserts rollback. Close drains admitted
work and closes the connection; Drop requests closure without joining a stalled
worker on the caller's thread. No connection or database mutex is exposed.

Reads capture indexed lightweight summaries, with 8 KiB rows, 256 KiB history
pages and epoch/workspace/generation/query-bound cursors. Complete metadata has
its separate canonical bound. Preview flags preserve the distinction between
shortened display fields and full records. SQL progress hooks limit read
computation; they do not bound filesystem stalls. This is an internal query
contract, not a new HTTP protocol or a measured end-to-end performance result.

Query normalization uses SQLite's own `unicode61` tokenizer through tiny
connection-local temporary FTS/vocabulary tables; see the
[SQLite FTS5 vocabulary documentation](https://www.sqlite.org/fts5.html#the_fts5vocab_virtual_table_module).
Punctuation separators in native IDs and repository/branch names become safely
quoted literal tokens, with the last token a prefix. The actual tokenizer output
obeys the eight-token limit. Temporary query rows never enter the workspace file.
Preview caps account for worst-case JSON escaping as well as exact identity bytes;
truncation flags compare original and preview byte lengths.

## Regression evidence

Real temporary SQLite fixtures exercise:

- Initialization, FTS5 availability, effective durability configuration and strict
  reopen/version/workspace checks without JSON creation or corrupt-file replacement.
- Durable agent patches, stale revisions, source conflict, u64 exhaustion, unknown
  root/nested extension preservation, oversized rejection and complete-record round trips.
- Checked organization identity/revisions, close-first actions, archived retention
  protection, non-displayable tombstones, monotonic progress and ID reuse rejection.
- Transaction rollback across canonical rows, search and history generation;
  actual SQLite busy and disk-full errors without false durable acknowledgement.
- Query ordering/ties, stale cursors, literal bounded search, bounded previews,
  selective existence, protected retention and index-backed closed-history ordering.
  A deterministic VM-step regression compares live search-index maintenance at
  100/1,000 records; this is not the #1247 end-to-end performance qualification.
- Queue count/byte rejection, fairness, before/after-execution cancellation,
  timely queued rejection while storage is stalled and cooperative SQL interruption.
  Deterministic gates cover startup cancellation after readiness and ticket drop
  between the worker's liveness check and its atomic execution claim, plus
  a descheduled deadline boundary with the caller timeout deliberately unpolled.
- Shared internal-write tests exercise independent Peon scopes and protected hook
  policy, arbitration against a newer agent write, exact Codex identity protection,
  last-seen-only revisions, title/provider ownership, FTS/history invalidation,
  no-ops at u64 exhaustion and invalid/oversized rollback. Actual JSON/SQL Peon
  outcomes are compared across all four attention policies and repeated ended-session
  reports (including activity timestamps); JSON event ownership
  remains observable in the original wrapper.
- Lifecycle/conditional-attention worker tests cover pending/final snapshots,
  duplicate finalization, live-transition rejection, active/history membership
  after finalization/resume, invalid lifecycle status, newer-work preservation,
  identical attention revocation, exact-root checks, missing/stale proof,
  rejection of unrelated intents on the conditional-attention API,
  reopen revocation and a real failed SQL transaction preserving retry ownership.
- An independent real Unix PTY exchanges input/output while the worker is paused.
  This validates this isolated worker boundary, not the unchanged application's
  complete runtime integration or the #1247 latency budgets.

Local verification and review results are recorded in the implementation PR.
No supported installed profile, filesystem durability certification, backup/restore,
100k-history memory result or controlled rollout is claimed by these fixtures.

## Remaining writer and authority work

Do not replace existing synchronous closures with worker waits under their
current guards. The concrete traces that still need typed integration are:

| Owner | Preserved cutoff and required next work |
| --- | --- |
| `metadata/session_transactions.rs` | Ordinary internal metadata cores now have typed decisions/no-ops; transition/finalization and conditional hook completion now have typed metadata cores; other lifecycle/native operations and append-before-replace partial effects still need integration. Never offer arbitrary SQL or execute caller closures on the worker |
| `metadata.rs` Peon/harness/provider merges and session titles | Shared pure reducers cover canonical metadata and manual titles. Preserve runtime/automatic-title epochs, Peon events/checkpoints and follow-up native identity capture when routing complete producers to SQL |
| `runtime/terminal_runtime.rs` input-derived attention | Currently reads workspace/session guards through a conditional metadata mutation; capture intent and release input-critical guards before any future database await, then revalidate runtime/input authority before publication |
| `runtime/codex_approval_application.rs` ShowWait/ClearWait | Preserve latest-record guards, live/durable ownership pair, freshness, coordinated-writer gate and the exact side-effect/publication ordering; SQL success alone cannot authorize clearing |
| `runtime/codex_completion_application.rs` | Shared exact-tuple reducer and SQL attention proof cover metadata only. Preserve stream qualification, invocation/order correlation, live proof and runtime/input fences before routing the application; no production stream or eligibility is enabled here |
| `session_application.rs`, lifecycle, archive/retention owners | Shared transition/finalization reducers cover metadata only. Retain per-session reservations, workspace/runtime generations, commit-before-spawn, close-first and external evidence owners while changing admission/await boundaries |

[#1244](https://github.com/Rambolarsen/orkworks/issues/1244) must not treat this
foundation as completion of #1243's writer/authority coverage. Backup, qualified
producers, quiescent migration and controlled rollout retain their separate gates.

## Runtime reservation boundary

Source-only implementation within the approved storage contract; production routing
remains JSON-backed. The first async application boundary handles Peon metadata
only. Native observer clears, input bookkeeping, lifecycle/spawn and external
events retain their current owners and must join this boundary before activation.

Prepared mutations reserve the existing worker's intent/result byte budget and
pending mutation slot before waiting for a per-session application lane. That
wait and later queue time share the original two-second pre-execution deadline.
Cancellation or expiry before execution releases the reservation without writing.
The owner polls definitive completion from enqueue, including while waiting for
worker readiness. Cancelled queued tickets retire their exact queue entry and
release slot/bytes/lane/lease without waiting for a stalled earlier command;
resources are dropped after the queue mutex is released. Dequeued or executing
jobs retain their definitive worker ownership.
Weak lane entries are shared by the exact worker owner and reclaimed; separate
application handles cannot create separate lanes for one store/session. History
size must not grow the lane inventory. Close rejects still-unqueued preparations
and drains queued/executing work; a preparation cannot enqueue a late write after
close has completed.

A prepared application job asks its application owner for an execution grant
when the worker reaches it, before claiming execution or opening a transaction.
The application briefly rechecks its captured workspace/store instance, runtime
identity, input/output boundaries, committed input sequence, attention/prompt
ownership (issued authority instance, native identity and reset epoch) and Peon
attempt under the
projection/transition/workspace/sessions ordering. The transition mutex is
try-locked: contention conservatively rejects the view without waiting under
projection. It remains held through the validation callback, serializing native
identity/reset transitions with the execution claim and live publication. The
registry mutex also remains held from snapshot comparison through the bounded
in-memory callback, covering authority cleanup paths that remove entries without
the transition mutex. These callbacks do not re-enter the registry, wait or do I/O.
It sends the one-use grant while
those guards are held, atomically claiming queued-to-executing and checking the
absolute deadline before granting. This successful claim is the operation's
execution/admission cutoff; cancellation can win only before that same CAS.
The claim returns a one-use execution token without waking SQL. Only after
the guards have been released does the owner release that token to the worker;
dropping it rejects the job. The claim is serialized with current
input/replacement admission; it is not
native qualification or authority to act on a replacement runtime. Domain/source,
creation and revision checks still happen on the latest SQL record.

The async owner retains the session lane and workspace lease through the definitive
store result, including caller cancellation after the grant claims execution. Before that
claim, queued caller cancellation can win the same state transition and guarantee
no write.
Capture, cutoff validation and publication run on the blocking pool so
contended synchronous guards do not block the async executor. Execution validation
and definitive worker completion are polled concurrently; queued expiry releases
the lane/lease/budget even while a blocking validation job still waits for a guard.
That late validation cannot claim the already expired ticket. Publication
rechecks the same context under short guards; its blocking job retains the
lane/lease/budget even if its async waiter is dropped. No-ops retain the Peon
permanent-hold decision. A committed write with
a changed runtime is reported internally as committed-but-detached; it cannot
publish into that replacement or be misreported as rolled back. No global
projection/workspace/session guard spans lane/worker waits or SQL.

Deterministic worker tests cover prepared slot/byte admission and cleanup, the
shared deadline, rejection/cancellation/late grants, claim-before-SQL release,
close-before-enqueue, admission recovery behind a stalled worker and lane/lease
retention even when a result owner drops.
Real application-state tests cover changed input, workspace/attempt/attention
authority, runtime replacement after commit, caller cancellation before/after
execution, permanent user holds, current-only publication, contended-guard
executor responsiveness, expiry before worker readiness and during blocked
execution validation, transition
exclusion at execution/publication and post-commit publication failure. A controlled runtime-command consumer
writes to an actual PTY while this application SQL boundary is stalled. This
proves guard/input-delivery independence at this boundary, not a qualified
SQL-backed producer profile, complete terminal workload or #1247 latency result.

This boundary is not producer migration. Events/checkpoints and policy selection
remain outside it. All conflicting production writers must participate in the
per-session admission protocol before SQL selection; the unchanged JSON/runtime
paths must not be mixed with the SQL authority.
