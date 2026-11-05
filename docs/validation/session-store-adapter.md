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

Organization changes and deletion-tombstone admission independently validate the
latest canonical finalization inside their SQL transaction: terminal status,
`dead` lifecycle and `ended` phase, with matching stored identity/revision. An
Archived index choice alone is insufficient because it intentionally survives
producer lifecycle changes. Unfinished, unreadable or inconsistent canonical
metadata blocks the action before organization/history/search/tombstone effects.
Real-store regressions cover creating/active/ending metadata beneath Archived,
reopen, unchanged rejection and successful retry after finalization, plus invalid
canonical shape/identity/creation/revision. This is a metadata prerequisite; the
application must still prove physical runtime closure and retain its exact action
reservation. It does not qualify SQL Archive/Delete runtime routing or external
cleanup/recovery.

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

## Native approval hook metadata

Activation, Stop, UserPromptSubmit and conservative permission fallback now have
typed private SQL intents. The JSON owner reuses their pure metadata rules and
the shared hook validation/state stages. SQL checks the latest exact native root,
creation/revision, lifecycle and user-source protection; accepted writes preserve
unrelated work/extensions and revoke prior attention proof even for equal values.
Fallback retains existing activity time and cannot overwrite a concrete prompt.

The source-only `SessionWrites::report_native_hook` captures its original runtime,
control, input, attention, tracker and prompt authority before metadata I/O.
Shared validation retains token/fingerprint/root/scalar checks, source timestamps,
trusted boundary watermarks, turn/correlation rules and the original monotonic
first receipt. Temporary transition contention returns Retry rather than claiming
that a valid report was invalid. This remains retryable after an earlier stage
commits; an obsolete runtime or workspace is distinguished from a busy guard.
A missing live native root retains a scalar-validated approval for later identity
binding; a known mismatching root rejects it. Missing rows and storage errors
remain typed.

A process-local hook application identity binds the staged continuation and all
issued native effects. Equal report values at exhausted hook counters cannot
adopt an older pending application. Metadata stages use existing bounded worker
reservations and per-session lanes, validating execution and publication under
short guards and retaining the workspace lease through the definitive result.
Caller cancellation synchronously retires an unclaimed operation; once claimed,
its owner finishes the remaining stages internally even if the caller disappears.
The same cutoff covers a hook's native effect. Hook-generated effects also retain
the original reporting capability and revalidate it at admission, execution claim
and publication. Observer effects remain independent of hook tokens.
Every SQL wait releases the
projection, workspace, session and prompt guards. No-op/error paths do not publish
an attention write or adopt a persisted proof. A committed write that loses live
ownership returns committed-detached explicitly. Temporary guard contention
retains the immutable packet for fresh application without adopting a stored proof.
If an earlier native wait is already durable and the fresh native attempt is a
no-op, the validated hook recovers through a new conservative scoped write and
publication before accepting the retry, retaining user/concrete-prompt protection.
While execution validation is blocked, expired worker completions are drained
independently so their lane, lease and reservation can retire without waiting for
the application mutex. A rejection racing the guard callback still retains its
Busy versus Obsolete classification; execution after a successful claim remains
definitive.

A report may contain several scoped writes, matching the existing activation
then fallback order. Failure after an earlier committed stage does not roll it
back. A retry revalidates the immutable report and rereads current SQL; it never
reuses a prepared command or renews the first-receipt grace period.

`CodexHookReportRelay::consume_ready_sql` exercises this path against actual
private approval mailbox files. It preserves bounded pending/retry quotas and
immutable cached receipts before awaiting SQL application, including when a drain
is cancelled, exposes the first typed storage failure separately,
and consumes detached commits without acknowledging live application. Private
identity envelopes now use the authenticated SQL path described below, with no
JSON session-metadata fallback. The production drain still selects its JSON path.

Regressions cover exact SQL tuples/proofs and protected no-ops, actual mailbox
approval and identity, close/reopen storage retry past the grace period,
transition contention before and after commit, token replacement at native claim
and publication, blocked-claim expiry/resource retirement, restoration of live
attention after a skipped publication, cancellation without receipt renewal,
unknown-root retention,
an eleven-case stale-read matrix, caller cancellation on
both sides of the execution cutoff, detached publication, equal-report overlap
at exhausted counters and earlier native emissions fenced by a pending boundary.
A controlled command consumer delivers actual PTY input while hook SQL is stalled.
This is source integration evidence, not a qualified installed SQL producer or a
larger-history performance result.

Ordinary JSON identity validation/merging, lifecycle/input bookkeeping,
external events, producer coordination and version-specific eligibility remain
prerequisites for SQL selection. Production native startup/clear gates stay closed; this optional
backend does not select SQL or migrate a workspace.

## Authenticated native identity reports

The optional SQL mailbox drain now sends private Codex identity packets to
`SessionWrites::report_native_identity`. This source-only entry requires the
existing live native runtime, exact run generation and reporting capability.
The JSON owner and SQL entry share scalar/lifecycle-pair validation; the SQL
worker reuses the typed HarnessReport reducer for confidence, sticky Codex
identity, unchanged-report freshness and complete extension preservation.
Creation identity and exact metadata revision are checked transactionally.

A replacement requires an authenticated root SessionStart with source clear and
an existing recorded reset for the exact previous durable native ID. The shared
execution owner checks that reset authority again immediately before granting
SQL execution. Live publication accepts only the originally captured runtime,
control channel, creation, native binding, tracker instance and identity epoch,
with current capability and cancellation state. A native identity change or reset
replaces the process-local identity epoch and invalidates old hook correlation,
including at checked-counter exhaustion. No persisted field reconstructs authority.

Identity-only writes do not borrow an attention effect's input/attention fences:
ordinary typing and explicit user attention can continue during the SQL read.
Publication changes live resume metadata only, preserving the newer attention
and input state. It never acknowledges native attention proof. Lower-confidence
or unauthorized identity changes remain ignored according to the shared reducer.
Hook-trust telemetry retains its external-file owner, and the optional SQL path
now uses the guarded observation follow-up below without invoking its session-JSON
callback. The native-label follow-up also uses the source-only SQL path below.
Ordinary HTTP routing and producer qualification are unchanged.

Identity and approval metadata share bounded reservations, a per-session lane,
lease retention, claim/cancellation cutoff, guard-contention classification and
expiry drainage. Guards never span SQL I/O. Cancellation before claim prevents
the write; after claim an owned task retains the definitive result and publishes
only into the original current owner. Busy publication retains the immutable
packet for a fresh revision-checked application. A durable commit that loses
ownership is explicitly detached, and storage errors remain typed.

The actual mailbox tests cover an approval arriving before identity, identity
binding, and a later retry of the original approval. Identity packets cache
original bytes before the SQL await, stay bounded by the same receipt quotas,
and cannot change their root unnoticed after drain cancellation. Storage failure
retains the packet; reopening retries against current SQL without JSON fallback.
Sixteen real application tests also cover confidence/no-op scope, invalid scalar
and lifecycle reports before storage, a ten-case stale-owner read matrix,
intervening input/user attention, revoked reset authority, concurrent revision
conflict, cancellation before/after claim, detached commits, retry after busy
publication, and bounded expiry while projection validation is blocked. A real
PTY receives input while an identity commit is stalled and the current identity
can still publish afterward. This is availability evidence, not latency or
installed-producer qualification.


## Native label refresh persistence

A current private SQL identity publication may schedule automatic-title enrichment.
Identity publication first synchronizes the tracker’s accepted durable binding, so
a normal observation of that same root cannot invalidate dependent lookup work.
Before lookup it captures the exact runtime/control/creation/tracker/native identity
epoch, capability, cancellation and refresh epoch. The existing five-second
per-native-ID cooldown and three bounded lookup attempts are preserved. Read-only
native lookup remains exact-ID, prefers saved name over generated title, and
continues to ignore unavailable/missing native rows without clearing a title.
Lookup work and SQL waits run outside terminal-critical shared guards.

The SQL owner tries the legacy refresh gate rather than waiting for it under
application guards. An opaque epoch rotates with existing generation reservation,
invalidation and clear/recreation, so exhaustion cannot resurrect a stale lookup.
The typed CodexLabel reducer is shared with JSON persistence: only a live active
Codex record with matching native ID and automatic label source can change title
and Codex provenance. Manual/unknown sources remain protected. Identical native
title/source is a true no-op; attention, summary, activity, scoped freshness and
unknown extensions remain independent.

Before execution claim and publication, the original context, refresh epoch,
reset block and reporting capability must still match. Writes share the bounded
reservation/lane/lease and cancellation cutoff with identity/approval metadata.
A caller cancelled before claim cannot write; after claim an owned task drains
the definitive result and either publishes to the original owner or reports a
committed-detached outcome. Busy publication can retry through a fresh checked
read/write, including a no-op durable title. Metadata storage errors stay typed;
a background optional enrichment logs failure and cannot undo an already accepted
identity. No metadata JSON fallback is invoked.

This source-only adapter does not select SQL in production, qualify ordinary HTTP
producers, enable native startup/clear, migrate data or establish measured typing
latency. Ordinary JSON identity validation/merging, lifecycle/input and other
external writer orchestration, producer qualification and migration/recovery remain prerequisites for selection.


## Hook execution observation follow-up

The optional SQL identity path forwards a valid hook fingerprint to the existing
workspace-owned `codex-hook-observation.json` owner. This file remains separate
from ordinary SQL metadata. Accepted, unchanged, ignored-lower-confidence and
ignored-identity-change reports retain the JSON owner's observation semantics.
Missing/invalid fingerprints never write an observation. No session JSON read,
attention mutation, native proof adoption or hook eligibility change is involved.

Before SQL waits, the operation binds runtime/control/creation/native identity,
capability/cancellation and the selected file owner's opaque clear epoch. Bounded
mailbox admission retains that epoch before awaiting the mailbox read, even while
the payload is unknown. The first validated bytes then become immutable before
SQL awaits. Cancellation during either read cannot renew an admitted receipt's
observation authority after a clear. A stamp from a different workspace file owner
is rejected. This is a
process-local execution fence, not proof of operating-system process identity or
new native hook trust.

If the 64 receipt slots are exhausted, one per-relay observation epoch floor also
covers untracked overflow packets. It never advances. Once that epoch is cleared,
the relay suppresses subsequent observations for its remaining lifetime, while
identity packets remain retryable and deliverable. A fresh relay can observe a
later execution normally. This conservative limit avoids both an unbounded
filename/epoch map and renewed authority after overflow; it does not change
identity persistence or the existing separate approval overflow floor.

After the durable identity decision and any current live publication, short
application validation admits at most one file claim for the workspace. Contention
returns Retry without queuing behind filesystem work. The owned claim retains the
workspace lease and writes the atomic observation file in a blocking task after
all application guards are released. Caller cancellation before SQL claim cannot
write; after claim the owned workflow retains definitive completion. A lost
runtime/capability before file claim or an invalidated clear epoch suppresses the
observation. An already admitted file write remains definitive, and a subsequent
clear serializes after it and removes its result. Legacy observation writes and
clears share this owner; reads keep their existing atomic-file behavior.

A file write error is typed and keeps the immutable mailbox packet retryable. The
SQL identity may already be committed and published: this is an external effect
failure, never a rollback acknowledgement. A retry uses the existing identity
rules and cannot renew the original clear epoch. Failed replacement leaves the
previous observation intact and releases exclusion for a later retry.

Coverage uses real SQL/application/mailbox and external files, deterministic
stalls at file execution, and a controlled consumer delivering real PTY input
while the file writer is stalled. It includes ignored reports, clear during SQL
read, clear after original receipt, cancelled-drain retry, contention, typed file
failure/recovery, stale runtime/token/workspace, foreign file owner, detached
identity, pre-claim cancellation and owned completion after caller cancellation.
Codex Settings install/repair/uninstall and unreferenced cleanup now use the owned
action described below. The legacy JSON observation follow-up is now integrated
as described below; ordinary identity validation/merging and
queued-before-first-receipt trust timing qualification remain separate gates. Production SQL selection, installed producers, native
startup/clear, migration/recovery and larger-history performance are not qualified
by these isolated tests.

## Codex Settings external-file scheduling

Legacy harness and grouped-key Codex mutations, plus unreferenced cleanup, retain
the existing probe-outside-lock validation flow. Admission rechecks the original
workspace path, metadata file owner and lease; replacing metadata or its lease at
the same path rejects the action before configuration work. Grouped actions still
check harness document and active-selection revisions. A narrow owned integration
handle shares the observation owner without exposing ordinary session metadata.

At that cutoff the application admits at most one Codex action per external file
owner. Overlap returns the existing revision-conflict error rather than adding
waiters to a stalled filesystem. The owned action retains its workspace lease,
resolved definition, enabled state, tool result and reporter assets, and runs
configuration/reconciliation, observation clearing and resulting status in one
blocking task after releasing projection/workspace guards. Cancellation during
probing admits no action. After admission the action completes definitively even
if its HTTP waiter disappears or the selected workspace is replaced; it edits
only the original workspace and never publishes into its replacement.

Clear still invalidates captured SQL receipts before waiting behind a previously
claimed observation write, then removes that write's result. Real clear I/O
failures now reach the integration application; committed hook configuration is
not rolled back or falsely acknowledged as a successful clear. Grouped routes
retain their existing diagnostic-status mapping. Admission is released after
success or error. Repeating an unchanged install retains its existing observed
trust rather than clearing it. Other harnesses keep their synchronous prompt
revocation ordering; generic inspection remains outside this Settings slice.
The separate legacy identity observation qualification is described below.

Regression coverage uses actual ignored hook configuration and reporter assets,
real workspace leases and file-owner exclusion. It covers all five mutation/
cleanup routes, same-path owner/lease replacement, cancellation on both sides of
admission, definitive old-workspace completion, bounded overlap/retry, clear
failure after configuration commit, idempotent observed trust, and actual PTY
input delivery while a clear waits behind an observation claim. These are source
checks, not an installed release or large-history latency qualification.

## Legacy JSON identity observation follow-up

The existing HTTP identity path preserves its merge dispositions and compatibility:
valid Codex hook fingerprints still record execution for accepted, unchanged,
lower-confidence and ignored-identity-change results. Missing/invalid fingerprints
never write. Bearerless legacy reports keep their existing best-effort telemetry
eligibility; this refactor introduces no new native/process authority or credential
requirement. A report that was authenticated binds that original bearer and must
not downgrade after revocation.

The route captures the selected file owner's clear epoch before its original
metadata lookup, along with workspace ownership, creation identity and the optional
runtime/controller. Its synchronous pre-execution cutoff revalidates those facts
and the original valid bearer, then tries a single external-file claim. It never
waits behind file exclusion under projection/workspace guards. An obsolete epoch
skips telemetry. A busy file reports an optional diagnostic without changing the
identity HTTP result; a later fresh report may recover. Ordinary JSON record reads
and identity merging still retain their existing synchronous owner and are not
qualified as a complete nonblocking storage producer here.

Once claimed, blocking file work owns the original workspace lease until its
definitive result, even after HTTP cancellation or selected-workspace replacement.
A subsequent clear follows that claimed write and removes its result. File errors
are typed internally and logged by the owned writer even if its waiter disappears;
the identity response remains the existing best-effort identity acknowledgement,
not a durable observation acknowledgement or rollback claim. Failed staging keeps
a previous observation intact and releases exclusion. The independent optional
native-label continuation binds/schedules before the new file await rather than
capturing a replacement runtime afterward.

Legacy mailbox identities share the SQL receipt book: at most 64 retained packets,
immutable first-validated bytes, original pre-read clear epochs and one constant-size
overflow floor. After that floor is cleared, a relay conservatively suppresses its
remaining observations while still delivering identities; a new relay can observe
fresh execution. A packet's receipt stays retained until a blocking owner completes
unlink or confirms absence. Cancellation or unlink failure cannot renew the packet's
authority. Changed/corrupted bytes revoke correlation and are never delivered as the
original packet. This does not qualify execution timing before the first receipt.

Regressions use real HTTP/file/mailbox paths, workspace leases, failed staging and
unlink, actual PTY input during a stalled write, stale workspace/creation/runtime/
controller/lease/token checks, cancellation during read/write/unlink, immutable-byte
replacement, overflow and fresh-relay recovery. Native startup/clear, ordinary
producer qualification, remaining lifecycle/input/external writers, migration and
large-history performance gates remain open; production metadata remains JSON.


## Owned JSON identity target lookup

The first HTTP harness-session target lookup now captures a typed, immutable
session-file reader plus original workspace/lease and optional runtime identity,
control channel and creation before awaiting file I/O. The same legacy decoder,
normalization, provenance hydration and corrupt-file quarantine are used by owned
and synchronous reads; no new metadata writer or independent transaction lock is
created. Corrupt-file quarantine shares the exact existing per-session write-lock
registry and rechecks the parsed bytes under that lane before rename, so a
repair committed through that lane cannot be quarantined by an obsolete read.
Arbitrary direct file writes bypass this arbitration; their producer qualification
remains open. Ordinary reads do not acquire that write lane. Production remains JSON-backed.

Eight read slots per metadata owner bound admitted blocking tasks, with no waiting
queue. Overload returns HTTP 503 before identity merging or observation writing;
worker/task failure returns 500 rather than presenting an empty successful result.
Legacy missing/unreadable/corrupt record handling is otherwise unchanged. A cancelled
waiter cannot release its slot or original lease while the actual file operation is
still running. Workspace/lease/reader and runtime/control/creation currentness are
checked after the read and before request continuation; stale results return 409.
Existing bearerless legacy telemetry compatibility and original authenticated
observation checks remain in place. Prompt-hook registration still uses its existing
synchronous transition owner after the initial read.

The legacy relay treats target-read overload as retryable and retains its existing
bounded receipt or conservative overflow floor. Retrying cannot acquire a new clear
epoch; it does not treat a 503 as a successful identity delivery or discard the
packet. This does not qualify timing before the first mailbox receipt.

Real HTTP regressions pause the initial file lookup to verify application guards,
a single Tokio worker and actual PTY input remain available. They also cover stale
workspace/reader/lease/runtime/control/creation, runtime adoption/removal, cancelled
read ownership, bounded overload, fresh-report recovery and relay retry after clear.
Corrupt-file tests preserve a repaired valid record and prove quarantine and
replacement serialize in the same existing transaction lane.
The transactional JSON identity merge, reset/observation validation reads and other
metadata writers are still synchronous and unqualified. Resume publication and label
reservation now consume the [transaction identity completion](#transaction-identity-completion)
without rereading JSON. This is source scheduling evidence, not complete legacy
producer qualification, SQL activation, migration, installed-harness/native proof
or measured history latency.

## Transaction identity completion

The legacy metadata identity transaction returns its disposition plus the exact
committed identity fields (ID, creation, harness, resume memory and label source).
Accepted and unchanged reports carry that private result; unchanged reports retain
the existing last-seen update and do not append an extra capture event. Ignored,
missing, invalid and failed writes grant no publication result. Enum-returning
callers and HTTP status mapping stay compatible; no result is serialized or stored
as a new capability.

`SessionApplication` captures original application/workspace/reader/lease and optional
runtime identity, control channel, creation and harness before the existing
synchronous transaction. Resume publication checks that context and the committed
creation against the current handle. A durable result may remain accepted while
publication is detached; it neither adopts a replacement runtime nor implies
rollback. Publication uses the transaction's resume value rather than a second
JSON read.

The optional Codex label reservation also consumes this private result without
reading JSON under its refresh/workspace guards. It checks original context,
current live native ID, authenticated report class and the original bearer inside
the reservation gate, retaining the existing cooldown. Automatic-title eligibility
uses the committed label source. If a user claims the title before reservation, a
refresh can still be scheduled; final persistence rejects that overwrite. The
regression test explicitly reserves that stale snapshot and verifies rejection.
Only an authenticated completion retains that original bearer in private memory;
a later caller cannot substitute a replacement token. It is never logged,
persisted or serialized.
If a user title arrives after the captured transaction, the later label persistence
owner still checks the latest record and rejects that overwrite; a cached snapshot
cannot grant title ownership. Native label enrichment remains display-only.

Regression coverage exercises exact accepted/unchanged/ignored records and events,
failed staging and recovery, replaced runtime/control/creation/harness, adoption
or removal, stale label workspace/reader/lease/native context, wrong/revoked tokens,
revocation during a reservation wait, and a later manual title. Independent read
pauses prove resume publication and label reservation do not reread JSON.

This is publication/interface preparation for the accepted storage plan, not an
async JSON mutation worker. The transaction, reset-authorization read and final
observation validation still have their existing synchronous owners. Conflicting
lifecycle and metadata writers need shared application reservations before moving
those writes outside guards; merely spawning this transaction would not qualify
that scheduling. SQL selection, producer qualification, migration/recovery, native
eligibility and measured performance gates remain open.


## Dormant lifecycle application lane

The source-only `SessionWrites::lifecycle` boundary admits typed transition and
finalization metadata using the same worker/session lane as Peon and native
writes. It captures the original live runtime/control, creation, harness,
resume context, phase/status, attention token and metadata source/provenance.
Workspace lease and exact
metadata-reader identity must remain current. Capture, execution claim and
committed publication are memory-only guarded operations; admission, lane waits
and SQL run outside those guards.

The existing mutation byte/slot budget and original two-second deadline cover
lane and worker waits. Before execution, cancellation prevents a write. After
claim, a detached definitive owner retains lane, lease and result through
publication even if its HTTP/task waiter disappears. Stale publication returns
committed-detached rather than claiming rollback. Repeated ending transitions
remain true no-ops; finalization preserves the committed pending terminal status.

Close-intent organization reservations remain separate: a Close/Archive/Delete
request waits for the physical runtime finalizer while holding its own action
reservation. The finalizer must not acquire that action reservation. The short
metadata lane is shared by conflicting writes, not held around a wait for
process exit or another operation that needs the lane.

Real-worker regressions cover transition/finalization fields, unchanged legacy
JSON, stale owners before execution, runtime replacement during execution,
queued cancellation, definitive completion after caller cancellation, lane
expiry, independent session lanes and rejected/unavailable writes. Guards and a
single Tokio worker remain available during a paused SQL execution.

This is a dormant metadata boundary, not production lifecycle qualification.
JSON transition/finalization methods retain live-first process observation and
failure cleanup. Replay-size/event ordering, resume/spawn admission,
close-first archive/deletion and all other conflicting writers still need their
existing owners joined before production routing. No method here requests exit,
spawns/resumes a process, appends external events or makes replay size durable.
SQL selection, migration, native eligibility and measured performance remain gated.


## Legacy ending history ordering

The existing JSON finalizer now confirms the selected runtime's final replay
grid before committing an existing ending record. It uses the existing terminal
size serialization gate and metadata transaction/event-cache owners. After the
record commits, its status event is appended before the live ended mirror.
Original workspace/lease/reader, runtime/control/creation and harness identities
are captured before blocking-job dispatch and replay-gate admission, then
rechecked at the effect
cutoff and live publication. Record creation/harness are checked before the grid
write and again inside the metadata transaction.

A private completion reports each stage as not attempted, absent, written or
failed, separately from live publication. Replay failure for an existing record
and metadata failure preserve ending state and leave no terminal status event;
repair followed by a freshly validated attempt can complete them. Event append
failure cannot undo committed metadata: the record/live completion remains known
and the failed event stage is reported explicitly. An uncertain append is not
retried blindly. Absent metadata or workspace retains the existing known physical
runtime fallback without claiming durable metadata/events; a missing record can
therefore complete its runtime even when replay is unavailable.

The terminal resize decoder ignores zero and out-of-range dimensions; runtime
admission independently rejects zero before enqueueing a native resize. This
prevents startup capture from replacing a valid grid with an unrenderable one.
Valid positive `u16` grids and omitted-field defaults remain compatible.

The runtime invokes this owner as a retained blocking job. Its completion and
failure logging survive cancellation of the async waiter; a JoinError reports
unknown completion, never inferred rollback. Real-file tests pause before
history effects, between grid/record/event stages and before live publication.
They exercise failed grid/metadata writes and repair, committed event failure,
corrupt/missing records, replacement workspaces/readers/leases and runtime/
control/creation/harness owners. A real runtime finalizer test keeps a single
Tokio worker progressing while persistence is paused; a cancelled waiter still
leaves the definitive job to finish.

This qualifies ordering and honest partial results on the legacy owner. JSON
I/O still occurs under its existing shared guards; it does not qualify terminal
input latency during those stalls. The dormant SQL lifecycle boundary has not
acquired replay/event responsibilities, and complete conflicting-writer,
resume/spawn, close-first, producer and migration gates remain open. No new
format, cross-file atomicity, retention policy or installed-harness claim is made.

## Dormant SQL finalization history

The explicit `SessionWrites::finalize_with_history` operation composes final-grid
confirmation, lifecycle SQL commit, external status-event append and original live
publication. `SessionWrites::lifecycle` remains a metadata-only primitive. The SQL
worker never writes replay/events, and this source path is not production-selected.

The operation captures the original workspace/reader/lease, runtime/control,
creation, phase and source context before dispatch. Its prepared mutation reserves
the existing bounded queue budget before waiting for the per-session lane. An exact
SQL record read validates creation/harness and ending state before external effects;
missing/corrupt SQL cannot fall back to valid session JSON. After the existing grid
serialization gate is acquired, bounded memory checks revalidate the original owner.
A retained blocking job owns file work and worker waits outside application guards
and Tokio workers. The existing grid gate spans grid, SQL and event stages.

Failed grid confirmation prevents SQL/event effects. A SQL error reports the
confirmed grid and storage error without claiming rollback; an unavailable
definitive result can leave commit status unknown. Failed event append preserves committed metadata and
allows original live publication; it never implies rollback or an automatic retry.
The event uses the exact committed status, timestamp and final observation. The
completion retains the worker lane, lease and budget through event and publication;
replacements cannot receive that publication. Caller cancellation before SQL claim
can leave an already admitted grid-only effect; after claim, the original owner
consumes the definitive result and logs partial failures even without a waiter.

Real SQL/filesystem tests in `sql_finalization_history_tests.rs` cover failure and
repair, ordered stage visibility, exact event content, missing/inconsistent records,
replaced memory owners and cancellation on either side of SQL execution. The PTY
case stalls external history with an independent OS watchdog on a single Tokio
worker, checks SQL/guard availability and delivers actual input to another runtime.
A one-blocking-thread case also proves the retained owner performs its short memory
checks directly instead of recursively waiting on the same blocking pool.
This evidence does not qualify all conflicting producers, process exit/cleanup,
resume/spawn, close-first archive/deletion, installed helpers, migration or numeric
large-history responsiveness budgets. No cross-file atomicity is promised.

## Runtime history completion boundary

`runtime/terminal_runtime/ending_persistence.rs` connects the runtime's history
completion stage to typed JSON and explicit captured SQL owners. The default
runtime selects JSON and retains its prior final-scan behavior and unknown
JoinError diagnostic. This is not a selector for the entire SQL runtime pipeline:
`persist_final_peon_scan` still owns synchronous JSON provider context and
workflow-observation file effects. The captured SQL provider command is qualified below; the current scan still
needs its provider branch routed through that owner and its workflow file effects
qualified before a full SQL scan/finalizer can be selected. No production SQL
activation occurs.

The SQL constructor captures the original runtime generation outside the Tokio
worker, then obtains an immutable application-bound `CapturedEnding` from
`SessionWrites`. It retains exact workspace/reader/lease, runtime/control,
creation, phase/resume and attention/source/provenance through a scan wait. The
consumable handle has private fields and cannot renew its ownership at completion.
The existing direct SQL history operation captures and immediately consumes the
same handle. No mutation lane or SQL queue slot spans provider inference; the
existing command budget/deadline starts with metadata preparation. This boundary
neither schedules provider work nor changes its existing limits.

Runtime completion preserves the complete backend result: JSON stage results or
unknown JoinError, and SQL typed StoreError/per-stage results. A grid failure,
committed metadata plus failed event, detached commit and unavailable completion
cannot collapse into one boolean success. Existing lane/cancellation/lease and
publication fences continue to own definitive admitted work. Dropping an unused
capture has no file/SQL effects and releases its original lease.

Tests in `sql_runtime_ending_tests.rs` exercise early capture across control,
workspace/reader/lease, runtime/creation, phase and attention/source replacement;
wrong application/generation/phase; explicit SQL completion without reading or
repairing invalid legacy JSON; unchanged JSON routing; typed unavailable and
partial file failures; cancelled waiters; lease release and a single-Tokio-worker
capture stall with an independent OS watchdog. The existing close-first
`runtime_closed` predicate still requires PID/control-channel cleanup after
metadata completion; tests do not qualify SQL Archive/Delete or process adoption.
The original JSON runtime/final-scan and SQL history suites remain applicable.

## Captured ending provider context

`CapturedEnding::persist_provider_context` borrows the original SQL ending owner
so provider metadata can precede history completion without renewing authority.
`EndingProviderContext` reuses the existing provider reducer and NONE write scope,
but checks the latest durable lifecycle phase inside the SQL transaction. A record
that is already ended yields an unchanged receipt even when its live ending
publication was detached. The generic provider command remains unchanged.

The method prepares a bounded command before waiting for the existing session
lane, carries the original deadline and cancellation claim, and checks the same
application/workspace/reader/lease, runtime/control/creation and source snapshot at
execution and receipt. An owned task retains definitive lane/lease/budget resources
through receipt after caller cancellation. Replacement after commit is committed-
detached, not rollback. Missing/queue/size/deadline/unavailable errors remain typed;
unavailable completion cannot prove rollback and never authorizes blind retry or
JSON fallback. Receipt currentness does not project live provider fields or prove
process state. Provider-only changes preserve attention/activity/provenance/title
clocks; each accepted write still advances metadata revision once, including repeat
values. No events, workflow observations or provider processes are started here.

Real SQL tests in `sql_ending_provider_tests.rs` cover the stored/live phase gap,
exact provider fields and repeated revisions, preserved clocks, history completion
with the same capture, owner replacement, creation conflict, missing data, queue and
payload rejection, deadline/unavailable errors, cancellation before/after claim,
and retained original resources. A single-Tokio-worker test stalls SQL behind an
independent OS watchdog and delivers approved input to a separate runtime control
receiver. That test acknowledges the command at the receiver; it is not a new PTY
or numeric latency qualification. The existing real PTY/history tests still apply.

Default production routing remains JSON. The legacy final scan still performs its
provider write through its current JSON owner. Evidence recording retains a
separate file store with deletion, sequence and idempotency state; its qualification
is a prerequisite before the full captured SQL final-scan pipeline can be selected.
No workflow-store redesign, migration, activation or measured speedup is claimed.

## Captured workflow evidence

`CapturedEnding::with_workflow_evidence` composes the original history capture
with the existing workflow file owner before inference. History-only capture
remains memory-only and now retains a cheap clone of its original workflow
store, so later evidence composition cannot adopt a replaced store. Evidence registration shares store instance, directory,
cache, sequence, evaluation generation and mutex; it never reopens a path. A
weak permit registry binds session/creation and is pruned on capture/deletion.
SQL identity/ending phase and original memory/store fences are checked after
registration and again before evidence execution; valid JSON cannot substitute
for missing/unavailable/replaced SQL. Registration can expire after two seconds;
its memory-only blocking job retains capacity/lease until it drains.

The captured batch limits five candidates and a complete encoded intent to one
MiB, counting without allocating an oversized serialization. One process-wide
budget (shared across application/workspace owners) permits 32 jobs and 32 MiB,
separate from SQL admission. The existing session mutation lane serializes the
file job with lifecycle writes. All lane, SQL and filesystem waits occur outside
application guards and the Tokio executor. No lane spans provider inference.

Grounding retains original indexes and final-scan keys. Shared validation keeps
legacy confidence/content/fingerprint rules. Under the workflow mutex, permit
validation and an atomic pre-admission cancellation/expiry claim precede sequence
or file effects. An admitted batch finishes before deletion removes its segment
and forgets duplicate keys; delete-first rejects the old permit, including after
a failed removal. The counter and ordinary recording/deletion behavior otherwise
stay unchanged. No application callback runs under the evidence mutex.

Ordered candidate receipts distinguish ungrounded, rejected, accepted, duplicate
and typed record errors. File persistence errors can have partial/unknown durable
effects and are never automatically retried by this adapter. An admitted job
retains lane/lease/capacity through its definitive result, even without a caller.
Replacement produces a detached receipt and cannot request evaluation. Current
newly accepted observations request deterministic evaluation only; delayed refresh
checks the original store/lease/generation while holding its workspace guard.
This does not start model analysis or qualify existing recommendation I/O.

Tests in `workflow_observations/captured.rs` and
`runtime/peon_runtime/sql_ending_evidence_tests.rs` cover shared sequence/reopen/
duplicate state, registry cleanup, deletion and failed deletion, cancellation,
expiry, original-owner replacement, missing/ended/reused SQL, original-index
receipts, partial persistence errors and same-capture history completion. Budget
tests pin exact job/encoded-byte boundaries. The input fixture stalls an admitted
file batch with an independent OS watchdog on one Tokio worker, checks guards
and SQL availability, and acknowledges approved input at a separate runtime
receiver. It is a controlled admission stall, not a physical fsync-stall or new
PTY/numeric-latency qualification. A one-blocking-thread case checks nested-wait
independence. Runtime final-scan routing, close-first SQL Archive/Delete, migration,
activation and larger-history measurements remain unqualified; production stays JSON.

## SQL final-scan runtime composition

`terminal_runtime/final_scan.rs` composes the qualified provider, workflow evidence
and history owners. The explicit SQL constructor captures the original runtime
through `EndingPersistence::sql`, then registers its evidence permit before any
inference wait. Output capture checks that immutable owner and exact store;
reassembly runs outside application guards. The normal runtime entry explicitly
selects JSON, while a separate source-only SQL entry uses the same runner. No
workspace format selector, migration or activation is introduced.

The runner keeps the existing empty-output rule and provider final-scan timeout.
After inference, SQL fallback comes from an authoritative identity/ending-phase
read and original memory/store check. Missing, ended, corrupt or unavailable SQL
rejects persistence without reading valid legacy JSON. It does not reacquire a
replacement owner after the wait. Provider metadata precedes workflow evidence,
which precedes the original history completion. Their existing lanes, budgets,
expiry/cancellation claims, file serialization and definitive receipts still own
each stage; no lane or queue reservation spans inference.

The result is a stage report, not a boolean durable-success acknowledgment. An
outer SQL error rejects the original owner/preflight; an outer successful report
retains optional provider/evidence results plus the complete history result.
Evidence receipts retain per-candidate failures and history retains partial
grid/metadata/event results. An independent stage failure may coexist with later
history completion, matching legacy best-effort progression, without claiming
rollback or automatically retrying a partial/unknown write. Cancelled inference
releases the unused capture. Cancellation after stage admission lets that owner
consume its definitive result, but does not automatically admit later stages.

Inference snapshot precedence, frozen ending fallback, existing final fallback
and null recovery use the existing helpers. JSON keeps its generation/phase
checks and synchronous scan decision/file effects together on one blocking job,
followed by its qualified history owner. This moves its waits off the Tokio
worker without qualifying the legacy workspace guard held during JSON file I/O.
JSON keeps existing evaluator scheduling; SQL relies on its evidence owner's
scoped deterministic scheduling only, so duplicates do not schedule a second
evaluation and model analysis stays with its existing scheduler.

`sql_final_scan_tests.rs` covers full provider/evidence/history ordering, exact
fields and snapshots, duplicate identities, original owners changed across a
controlled inference wait, pre-dispatch stale capture, SQL loss/ending/unavailable
state with valid JSON, provider/evidence/grid/event partial failures, empty and
missing-inference fallback, cancellation before/after stage admission and both
explicit runtime entries. An independent OS watchdog stalls the real SQL provider
stage on one Tokio worker while approved input is acknowledged by another runtime
receiver and application guards stay available. This is receiver/stage evidence,
not a new physical PTY/fsync or numeric-latency qualification. A one-blocking-worker
case checks that composition does not introduce nested pool waits.

This qualifies the source-only final-scan composition. Physical exit cleanup,
close-first SQL Archive/Delete, installation/producer selection, migration, live
activation and 10k/100k performance evidence remain separate gates.


## Captured SQL close-first organization

`SessionWrites::capture_organization` supplies a source-only typed action view.
Its coherent exact-row worker read contains validated canonical metadata, indexed
organization state and organization revision in one transaction, bounded by the
existing read/queue budgets. The capture retains the original workspace lease,
reader, organization store, creation identity and optional runtime/control owner;
memory checks bracket the SQL read. It holds no action reservation or session lane
during display. This is an internal stale-action precondition, not an HTTP token
or a production projection registry.

Consuming the view permits Archive from Active/Closed and Restore from Archived.
A fresh SQL preflight and original-memory check precede the action reservation
and any stop signal. Active Archive captures the existing `CapturedRuntimeStop`,
closes its exact optional evaluation carrier outside all application guards, then
rechecks original workspace/runtime ownership before signaling its captured sender.
The blocking close job retains the reservation and lease even if the request is
cancelled; such cancellation never proceeds to a SQL organization write. A blocked
cutoff cannot signal a replacement. No carrier factory/attachment/activation is
introduced; production carriers remain None. Active Archive then waits up to
the existing 15-second close budget at 25-ms intervals, and leaves the finalizer's
SQL lane free. Persisted ending alone is insufficient: the original runtime must
also have no PID, no receiver, no resume and no terminal attachment, with dead/ended
live state. Remembered finalized rows require no runtime. Unsupported actions,
unknown/corrupt/missing/unavailable SQL and stale owners cannot fall back to JSON.

A typed prepared organization command shares the existing per-session worker lane,
32-write/two-second admission budget and atomic execution grant. Original authority,
physical closure and any close deadline are rechecked under short memory guards;
SQL starts only after those guards are released. The transaction independently
checks canonical finalization, creation and organization revision. It changes
organization/history generation while preserving canonical metadata, native
identity and activity. No external file or JSON organization write is added.

Lane, lease, action reservation, queue budget and definitive receipt remain owned
through SQL completion and a blocking publication check. Cancellation before the
grant prevents the organization write and releases pending resources, even behind
a stalled worker; an already sent stop signal is not rolled back. After the grant,
caller cancellation retains completion, including a successful grant still in
transit from its blocking validator. Cancellation/expiry competes on that exact
grant CAS; losing cannot discard its result. Organization validation does not
depend on the unrelated global prompt-authority transition lock. The receipt includes the new revision and
committed/committed-detached status; replacement authority receives no projection
publication. A current original owner only emits its session-view invalidation.
SQL projection/HTTP token selection remains a subsequent integration gate.

Real-store/runtime tests cover stale capture and preflight views, control/runtime/
creation/lease/reader/organization replacement, every physical-close predicate,
finalizer-lane independence, close timeout, cancellation on both sides of admission,
lane and actual worker-queue expiry, retained resources, detached completion,
remembered rows and unchanged legacy files. A single-Tokio-worker test stalls the
real SQL organization stage behind an independent OS watchdog while application
guards and approved input acknowledged at another runtime receiver remain
available. A one-blocking-worker fixture checks composition without nested waits.
The fixture models the driver's finalization using the real SQL lifecycle reducers
and same worker lane, then controls its live cleanup flags; the separately qualified
lifecycle application/publication owner is not requalified here. This is
stage/receiver evidence, not physical PTY/fsync or numeric latency proof.

The HTTP action route and production projection remain JSON. Complete SQL Delete
and recoverable external-file cleanup, producer/installation selection, migration,
activation and measured 10k/100k release budgets remain separate gates.

## SQL deletion recovery discovery

The private worker exposes lightweight pending-deletion pages (1–100 jobs,
256 KiB maximum response) and a selected tombstone read by exact session ID and
creation identity (canonical record plus at most 4 KiB envelope). Both consume
existing read admission and result-byte reservations; SQLite work remains on the
worker connection, under its read deadline. The page seeks the deletion-job ID
index and loads no canonical session payloads. Selected reads validate canonical
identity/revision/finalization and return progress with metadata from one transaction.

Continuation is bound to the database epoch and workspace. It is a changing
keyset traversal, independent of history generation: completing earlier jobs
must not invalidate later pages. Start a new pass from the beginning to discover
jobs inserted behind the cursor. Pages and selected records are discovery only;
they carry no lease, runtime authority, cleanup reservation or permission to
remove external content. A stale creation identity cannot read a replacement.
Unhidden or orphaned jobs and inconsistent canonical metadata fail explicitly;
ordinary detail/history/existence/retention continue to exclude tombstones.

Real SQLite regressions cover progress across reopen, completion during paging,
insertion behind a cursor with a new discovery pass, foreign-store cursors,
invalid limits, reused identity, corruption and exclusive
lock failure without an empty-success fallback. Maximum escaped identities and a
near-limit selected payload fit the declared response reservations. Query-plan
checks cover indexed seeks, an SQLite authorizer rejects canonical payload reads
during page discovery, and worker tests cover the shared eight-read admission
limit, byte accounting and release. These are correctness/boundary checks, not
measured latency or physical crash qualification. External cleanup stage owners,
complete SQL Delete integration/recovery scheduling and qualified activation
remain follow-ups. Production continues using JSON.

## Captured SQL Delete admission

The source-only captured organization owner has an explicit `begin_delete`
operation returning `DeletionAdmission`: exact session/creation and committed or
committed-detached status. It commits only a non-displayable tombstone and its
progress-zero recovery job. The existing `apply` API still permits Archive and
Restore only; admission cannot be mistaken for complete external Delete success.
The worker's prepared organization-action result distinguishes a revision change
from deletion admission without inventing a revision for the latter.

Delete shares the existing reservation, captured-stop revocation, physical-close
wait, original-owner validation, bounded SQL queue/grant and retained completion
path. It leaves the SQL lane free for the runtime finalizer. After closure it
rereads canonical SQL metadata outside guards, preserving original creation and
organization revision while taking the finalizer's latest metadata revision.
The original transaction rechecks both revisions and canonical finalization.
Later changes conflict without a job; a valid JSON copy never recovers failed
SQL. Pre-grant cancellation/expiry prevents admission; successful atomic grants
retain completion through late delivery, caller cancellation or deadline expiry.

Real runtime/store tests cover Active, Closed, Archived and history-only rows,
physical-close/finalizer independence, progress and exact identity across reopen,
retained replay/JSON files, stale memory and durable views, metadata/organization/
creation races, canonical lifecycle regressions beneath closed index state,
missing/unreadable/unavailable SQL, queue-full/busy errors and queued expiry.
Admitted tests hold the original lease/lane/action reservation until definitive
completion and distinguish replacement-runtime detachment. Successful-grant
cancellation/deadline tests cover the grant-delivery gap. Delete and Archive
share a controlled single-Tokio-worker stall fixture with an independent OS
watchdog and another runtime's acknowledged approved-input receiver.

The finalizer fixture uses real SQL lifecycle reducers and the same lane while
controlling live cleanup flags; this does not requalify the separate lifecycle
application publisher. Receiver/stage evidence is not physical PTY latency,
fsync/crash qualification or measured 10k/100k performance. Admission itself removes no external payload,
workflow evidence, recommendation, legacy metadata or organization file. After admission resources release normally; the durable job remains
pending. Recovery discovery/receipts grant no destructive authority. Captured
cleanup owners, idempotent stage definitions/scheduling, production routing,
migration and activation remain prerequisites. Production remains JSON.

## Captured SQL replay cleanup

`capture_deletion(id, created)` binds an exact finalized tombstone to original
workspace, lease, metadata reader, organization owner and optional closed runtime/
control identity. It rechecks memory after SQL capture. `remove_replay` re-reads
the exact job under the shared session lane, reserves the organization action,
and atomically admits a blocking file job under original physical-close fences.
The two-second pre-admission cancellation/expiry cannot cancel an admitted job;
its original lease/lane/reservation and shared budget persist through definitive
completion, including caller loss, late completion and switched-away owners.
Validation/publication uses memory guards only; file/SQL waits hold none.

The metadata facade captures the original checkpoint-cache owner without I/O.
On Unix it opens root/events directory handles without following their final
symlink, unlinks only `<id>.ndjson`, `<id>.terminal` and `<id>.terminal-size`, then
syncs events and workspace directories. A missing events directory still syncs
the workspace. Repeated calls sync again even after successful unlink followed
by sync failure. Cache invalidation follows successful sync; unrelated payloads
and symlink targets survive. Non-Unix cleanup returns Unsupported before effects
until separately qualified; it cannot acknowledge false durable progress.

After file success the worker records `progress | 0x01` with `complete=false`,
preserving other stage bits. No row/job completion occurs. Must-use receipts
separate file results, optional progress results and detached ownership. A file
error can represent partial removal and never attempts progress. A SQL error
after durable files remains explicit and retryable; neither case claims external
rollback. Conflicting application writers must use the shared lane/reservation
before production selection. Raw private store primitives are not an alternate
application cleanup path.

Real-store and filesystem tests cover reopen/retry, ordinary/wrong/reused/missing
identities, canonical corruption, stale memory/lease/reader/control/organization,
physical reopening, reservation conflicts, cancellation/expiry before admission,
admitted cancellation/deadline/detachment with retained resources, partial unlink
errors and SQL Busy after synced removal. Both directory sync failures are
injected and retries repeat sync when files are absent. Symlink directory/target
and invalid-ID boundaries are covered. Valid IDs up to 256 bytes remain supported:
the live directory must report a positive component bound before unlinks.
Longer exact basenames are absent and are never resolved as truncated aliases;
unknown limits fail before effects. Directory sync still precedes progress. A single-Tokio-worker OS-watchdog test
checks free application guards and another runtime's acknowledged approved input;
a one-blocking-worker test checks absence of nested blocking waits. Workflow
file jobs and cleanup use the existing shared 32-job/32-MiB intent budget and its
unchanged boundary tests.

This is source ordering and controlled receiver/failure evidence, not physical
power-loss, filesystem/profile qualification or numeric latency proof. Legacy
JSON deletion remains unchanged. Workflow evidence, images, last-active settings,
recommendations, final row/job removal, scheduling, producer/selector integration,
migration and activation remain follow-ups. Production remains JSON.

Replay directory handles are opened at execution time. Ordinary root/events
replacement between capture and removal remains an unqualified ownership boundary
tracked in [#1442](https://github.com/Rambolarsen/orkworks/issues/1442); the source
fixture results above do not establish activation readiness for that case.

## Captured SQL workflow evidence cleanup

`workflow_observations/deletion.rs` adds a durable variant under the original
shared mutex. Controlled directory-sync failures exercise unlink-success/sync-
failure and already-absent retry: caches remain until synchronization succeeds,
while old captured writer permits are revoked on every valid removal attempt.
Invalid IDs are rejected before effects, exact overlong basenames are not
resolved, root/workflow handles stay bound across directory replacement, and
segment symlinks are unlinked
without touching their targets. Other observations and the sequence survive;
reopen sees only the intended deletion. Initial symlinked directories cannot
obtain cleanup anchors; legacy opening still works but durable deletion fails.
Non-Unix has a fail-closed fixture.

`sql_writes/delete_evidence_tests.rs` exercises the real SQL tombstone/job and
shared store: bit `0x02` survives reopen without final completion; replay/legacy
metadata and unrelated evidence remain. Replaced store instances fail before
revocation. Pre-admission cancellation/expiry preserve the original writer;
admitted caller loss and owner replacement finish on the retained original
store with a detached receipt. File errors do not attempt progress, SQL busy
after successful removal reports the partial outcome, and retry preserves other
bits. The existing replay suite exercises the unchanged shared runner's stale
workspace/runtime/reader/lease checks, job/identity revalidation, lane/reservation
and capacity boundaries, and completion with one blocking worker.

A single-Tokio-worker fixture holds the actual evidence mutex on an OS thread
while admitted deletion stalls. Projection/workspace/session/PID guards remain
available and another runtime acknowledges input before the bounded watchdog
releases the mutex. This is controlled concurrency evidence, not measured live
latency or hardware power-loss qualification. Production remains JSON; no
migration, automatic cleanup scheduling, retention change or platform activation
is claimed. The [implementation plan](../superpowers/plans/2026-11-05-sql-evidence-cleanup.md)
records scope and remaining gates.
