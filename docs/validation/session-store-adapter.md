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
remains JSON-backed. The async application boundary handles Peon metadata and source-only native
approval effects. Input bookkeeping, lifecycle/spawn and external events retain
their current owners and must join this boundary before activation.

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
the transition mutex. Attempt lease and diagnostic read guards are try-acquired
inside that registry callback and retained through capture, execution claim or
publication; contention rejects conservatively. Attempt cleanup cannot retire the
owner between validation and application. The final atomic cancellation check
under those guards is Peon callback admission: callbacks admitted before the
unchanged immediate shutdown flag store may finish; later callbacks, including
post-commit publication, reject. This is separate from caller cancellation/expiry,
which still competes on the exact worker execution CAS. Shutdown retains its
bounded drain and never waits for attempt ownership to signal cancellation.
These callbacks do not re-enter the registry/Peon owners, wait or do I/O.
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
exclusion at execution/publication, attempt retirement/shutdown between validation
and callback, and post-commit publication failure. A controlled runtime-command consumer
writes to an actual PTY while this application SQL boundary is stalled. This
proves guard/input-delivery independence at this boundary, not a qualified
SQL-backed producer profile, complete terminal workload or #1247 latency result.

This boundary is not producer migration. Events/checkpoints and policy selection
remain outside it. All conflicting production writers must participate in the
per-session admission protocol before SQL selection; the unchanged JSON/runtime
paths must not be mixed with the SQL authority.


## Native observer metadata preparation

The source-only native ShowWait/ClearWait metadata predicates are shared pure
reducers, with typed private SQL intents under the accepted #1243 contract. The JSON observer retains its
runtime/effect/source checks, final monotonic freshness callback, live projection
and acknowledgement behavior. SQL uses the latest canonical record and exact
worker/session/creation attention proof for ClearWait; ShowWait publishes a new
proof only after commit. No-op or failed writes preserve the prior proof.

A ClearWait intent carries the observation's process-local monotonic start time,
never persisted or exposed as a durable credential. Both early admission and the
point immediately before SQL commit check the existing native freshness policy
(including future/expired observations and its inclusive boundary). SQL receives
typed data, never an application callback. A private store clock seam allows real
transaction rollback tests when freshness expires during staged SQL work. The
freshness check admits the irreversible commit; filesystem/commit completion is
not hard bounded, matching the JSON check immediately before atomic replacement.

Regressions cover real JSON-observer/SQL metadata parity, protected tuples at
revision exhaustion, cross-creation/worker proof rejection, work-only preservation,
consumed-proof rejection, rollback/freshness expiry before commit, paused-worker
expiry and reopen ownership loss. A real SQLite update hook advances only the test
clock after the update is staged, proving expired work rolls back and retains its
proof; the shared native freshness policy includes its 300 ms boundary and rejects
future observations.

The source-only native application owner now uses the worker-shared per-session
lane and bounded admission/definitive-result protocol. It reads the exact SQL
record without runtime guards, captures an issued reducer effect under short
projection/transition/workspace/session/authority guards, and revalidates that
capture when claiming execution and publishing. SQL observation/hook/tick entry points reduce and mint an opaque effect together
under the issuing tracker/attention guards, before releasing them or reading SQL.
There is no SQL issuance entry point accepting a queued raw effect. The effect retains
its issuing tracker instance, hook revision, live attention token and original
clear observation start; capture cannot adopt a replacement tracker or borrow a
newer observation for an equal candidate/fence value. Capture binds tracker instance,
runtime/control, creation, native root, hook/fence, committed input sequence,
live attention ownership and prompt authority. Cancellation, user/question/options
attention, ending sessions and superseded effects reject conservatively. Clear
also requires the live/SQL ownership pair and the exact fresh observation.

The initial SQL revision is checked transactionally. A concurrent durable update
(including an unrelated work-only update) returns conflict rather than applying
a stale native identity snapshot; no retry is inferred. Pre-capture descriptive
updates remain preserved. This conservative application precondition is separate
from the metadata primitive's ability to merge unrelated work on the latest row.
The owner releases all shared guards before SQL starts, retains its lane/lease
through definitive completion and publication, and acknowledges the reducer only
after commit into the same current context. A changed context after the execution
cutoff yields committed-but-detached, without acknowledgement into that context.
SQL no-op/failure/expiry cannot publish or consume the prior live ownership.

Application regressions cover current ShowWait and test-only coordinated
ClearWait, closed production clearing, queued input/sequence/hook/tracker/root/
attention/question/options/user/lifecycle/workspace/cancellation changes, caller
cancellation before/after execution, durable revision conflict, expired clear and
runtime replacement, tracker replacement before capture with an identical effect,
and old-clear rejection after an equal effect is reissued from a fresh observation.
A controlled runtime-command consumer delivers bytes to an
actual PTY while native SQL is stalled after its execution claim. This proves
input independence at this boundary, not complete terminal-load or #1247 latency
qualification.

The optional source-only SQL observer backend now captures its runtime/tracker,
creation/control, input/sequence, hook/fence, live identity/attention and prompt
authority before a metadata read, releases all shared guards for that read, and
rechecks before constructing the RPC snapshot. The snapshot also binds its exact
application owner. Metadata supplies only the already accepted native root;
it must agree with the live native reference, and loaded threads never discover
or rebind identity. A due tick effect uses the existing reservation boundary.
Successful live publication skips RPC so a later poll captures the new attention owner.
If a tick is a no-op, detaches from live publication, or conflicts, the owner
continues to RPC using that original snapshot. Rejected effects cannot starve
observation. The reducer candidate and prior proof remain available for safe
retry; detached durable state grants no new proof or permission to clear it.
Other storage failures still propagate. Tick and RPC effects carry this original
context through the effect application's own SQL read and recheck it at admission,
so an obsolete effect cannot adopt a replacement controller or prompt authority.

After RPC, a second unlocked SQL read hydrates the latest canonical metadata.
The owner rechecks the original snapshot before feeding the result into the
reducer and mints effects in that same guarded call. Identity changes discovered
by synchronization discard the old result; unrelated descriptive writes remain
preserved. A response requested before its snapshot is rejected. The original
first-request monotonic timestamp stays attached through read, scheduling and
commit freshness checks; metadata latency cannot refresh old evidence.

NativeObserverControl has an optional SQL path using these methods and the same
owned, bounded RPC client and cancellation/shutdown ordering. Its production
constructor still selects JSON. SQL failures retain their typed error in a
separate bounded observer watch, including returned read/write command deadline
failures. Stale-context conflicts and reducer-discarded native evidence remain
ordinary rejections. These failures neither masquerade as empty metadata nor
signal the native failure channel that terminates a PTY. Storage presentation
still belongs to #1246 before production selection. Source regressions exercise
real SQL reads without JSON session files, owned native RPC, stale context and
root changes, preserved newer work, stalled-read guard release, cancellation,
shutdown while SQL is stalled, and storage failure without runtime termination.

Hook relay activation/boundary/fallback writes still use their unchanged JSON
owner and need typed SQL orchestration. That routing, lifecycle/input bookkeeping,
external events, producer coordination and version-specific eligibility remain
prerequisites for SQL selection. The production native startup/clear gates stay
closed; this optional backend does not select SQL or migrate a workspace.
