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
expected revision inside its transaction. True no-ops preserve bytes, revisions
and scope clocks, including at counter exhaustion. A same-identity harness
last-seen update deliberately advances the revision, matching the JSON contract.
Historical internal writes invalidate history in the same transaction.

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
  outcomes are compared across all four attention policies; JSON event ownership
  remains observable in the original wrapper.
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
| `metadata/session_transactions.rs` | Ordinary internal metadata cores now have typed decisions/no-ops; remaining lifecycle/conditional-native and append-before-replace partial effects need their domain intents. Never offer arbitrary SQL or execute caller closures on the worker |
| `metadata.rs` Peon/harness/provider merges and session titles | Shared pure reducers cover canonical metadata and manual titles. Preserve runtime/automatic-title epochs, Peon events/checkpoints and follow-up native identity capture when routing complete producers to SQL |
| `runtime/terminal_runtime.rs` input-derived attention | Currently reads workspace/session guards through a conditional metadata mutation; capture intent and release input-critical guards before any future database await, then revalidate runtime/input authority before publication |
| `runtime/codex_approval_application.rs` ShowWait/ClearWait | Preserve latest-record guards, live/durable ownership pair, freshness, coordinated-writer gate and the exact side-effect/publication ordering; SQL success alone cannot authorize clearing |
| `runtime/codex_completion_application.rs` | Preserve invocation/order correlation and conditional latest-record edits that retain unrelated work; no production stream or eligibility is enabled here |
| `session_application.rs`, lifecycle, archive/retention owners | Retain per-session reservations, workspace/runtime generations, commit-before-spawn, close-first and external evidence owners while changing admission/await boundaries |

[#1244](https://github.com/Rambolarsen/orkworks/issues/1244) must not treat this
foundation as completion of #1243's writer/authority coverage. Backup, qualified
producers, quiescent migration and controlled rollout retain their separate gates.
