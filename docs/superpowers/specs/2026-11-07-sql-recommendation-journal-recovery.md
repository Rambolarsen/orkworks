---
type: Design
title: SQL-bound recommendation journal recovery
description: Proposed exact-job journal identity, legacy refusal and bounded restart recovery for ordinary-session deletion.
tags: [orkworks, sessions, sqlite, recommendations, recovery]
status: draft
workflow_status: proposed
---

# SQL-bound recommendation journal recovery

Parent: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Inspection base: `418cacba3b7b3f9b9ce90b1ab6d59d11c1fecf42` (#1505).
This is a proposal for owner review, not implementation or activation approval.
The accepted [session storage design](2026-11-01-sqlite-session-storage-design.md#external-payloads-events-backup-and-deletion)
already requires recoverable identity-bound external deletion. This document
closes the recommendation journal representation and recovery-routing choices.

## Outcome and boundary

An interrupted recommendation cleanup can resume only for the same database,
workspace and exact tombstoned session creation. It must never adopt another
job's journal or silently run legacy recovery. SQL progress advances only after
file completion; failure keeps the tombstone/job and original evidence visible.

Keep the private single-worker SQL owner, workspace lease, runtime/session lane,
close-first admission, recommendation operation owner and descriptor-relative
filesystem checks. Retain current graph closure and evidence policy. No new
public API, bearer capability, automatic startup retry, final job completion,
producer activation, migration, retention change or live repair is included.
Production continues to select JSON. Windows remains unsupported for this
anchored filesystem path until separately qualified.

## Evidence and alternatives

- [Captured cleanup](../../../crates/orkworksd/src/session_application/sql_writes/deletion.rs)
  re-reads the exact tombstoned `(id, created_at)` before admitted effects and
  retains the recommendation receipt through SQL progress. It currently refuses
  every nonempty pending journal because v2 contains no job/store identity.
- [Selected recovery read](../../../crates/orkworksd/src/session_store/deletion_recovery.rs)
  already joins the exact tombstone/job and reads canonical metadata in a short
  transaction. `store_header` has the database UUID epoch and canonical workspace;
  returning them with the selected job needs no SQL schema migration.
- [Anchored v2 protocol](../../../crates/orkworksd/src/taskmaster/store/cleanup/removal/journal/protocol.rs)
  checks current root/lease/leaf ownership, bounds inventory/backup bytes, and
  supports incomplete publication and repeated recovery without restoring rows.
- [Legacy recovery](../../../crates/orkworksd/src/taskmaster/store.rs)
  deletes directories without `manifest.json` before parsing any version. Merely
  adding identity fields to a manifest cannot protect initial-publication crashes.
  Serde's existing v2 struct also ignores unknown fields: an optional v2 identity
  field would not establish a distinct, fail-closed protocol.

Choose a new strict v3 deletion manifest and a reserved identity-bound transaction
directory name in the existing `.rollup-transactions` owner. A separate journal
namespace adds a second discovery/routing path without avoiding the need to stop
ordinary writers during pending SQL cleanup. A separate identity sidecar creates
another publication/cleanup boundary. Neither is needed for this scoped protocol.
Generic v2 recovery under a SQL job remains forbidden.

## Exact identity and encoding

Define a private immutable `SqlDeletionIdentity`, minted by the SQL worker only
from a successful selected tombstone/job read in the same transaction as the
header read. Fields are the canonical lowercase UUID database epoch,
`workspaceSha256` (lowercase hex SHA-256 of the header workspace's exact UTF-8
bytes), session ID, and exact `created_at` string. Keep the existing nonempty
256-byte ID/creation bounds and safe ASCII session-ID alphabet. Header UUID and
workspace checks remain mandatory. Admission must reject a stored epoch whose
exact spelling differs from `Uuid::parse_str(epoch)?.to_string()`; never
normalize/rewrite the header or hash an alternate spelling. Current store open
checks parseability only, so this is a new SQL-identity read precondition, not a
claim about existing validation. Test uppercase, compact and braced parseable
UUIDs as rejection vectors. The digest is identification, not authority.

The reserved transaction basename is `sql-delete-v1-` followed by 64 lowercase
hex characters. Its digest input is the ASCII domain
`orkworks/sql-recommendation-delete/v1` followed by a NUL byte, then each of the
four identity fields in the order above, each encoded as a four-byte unsigned
big-endian UTF-8 byte length followed by its exact UTF-8 bytes. Hash this input
with SHA-256. Reject noncanonical names; never infer a full identity from a hash.
Directory-name comparison binds even a manifest-less partial publication to the
currently selected identity. When a complete manifest is present, compare all
four stored fields exactly as well as the expected basename.

The v3 manifest is one strict JSON object with exactly `version` (3),
`committed` (boolean), `sqlDeletion` (the four named identity fields:
`databaseEpoch`, `workspaceSha256`, `sessionId`, `createdAt`) and `entries`.
Entry fields and relative-path/hash encoding are the existing v2 deletion-only
contract; reject unknown fields at every new object level, duplicate keys,
wrong types, replacements, invalid hashes and unsupported versions. Both the
draft and committed manifest carry the identical full identity and entries.
Only `committed` changes. Do not add optional SQL fields to legacy v1/v2 structs.

No independent job nonce or schema bump is selected: the current durable job key
is `(session ID, creation)` within one database epoch/workspace. Its lifetime
ends only after all required cleanup, including journal removal. A future finalizer
must refuse completion while any pending journal exists. Current
`deletion_progress(..., complete=true)` does not enforce that filesystem gate;
its safe application finalizer is a required pre-activation deliverable outside
this recovery task. Bypassing that gate or
reintroducing an old journal after job completion supplies no recovery authority.
If implementation exposes a supported same-key job lifetime that can overlap a
retained journal, stop and revise this identity contract before proceeding.

## Publication and legacy routing

Before any canonical effect, preserve the current exact SQL job read, runtime
closed check, workspace/store identity, original lease, per-session lane and
organization reservation. Minting/copying an identity never grants these rights.
Capture and blocking I/O stay outside application input-critical guards.

Under one retained recommendation operation permit:

1. Preflight the entire bounded journal parent before any recovery or new
   publication. Accept absent/verified-empty legacy scaffolding. A nonempty v1/v2
   journal, another SQL basename, multiple pending transactions, unexpected entry,
   symlink, replacement or unreadable parent refuses this job before effects.
2. With no pending transaction, prepare the existing complete connected removal
   graph from the bounded anchored inventory. Revalidate its original bytes and
   leaf identities. Create the expected reserved basename exclusively and sync
   the parent before writing backups. Never replace or reuse an existing pending
   directory as a fresh transaction.
3. Reuse bounded byte-preserving backups, directory/file syncs and draft-to-commit
   publication. No canonical recommendation unlink precedes a durable committed
   v3 manifest. The new identity fields count toward the unchanged manifest cap.
4. Complete deletion through the same anchored recovery engine under the same
   permit. Retain the receipt, lease and reservation through definitive SQL
   progress persistence and detached-result checks, including caller loss.

Legacy `recover_transactions` must preflight all parent names before its first
mutation and refuse every reserved `sql-delete-v1-` entry, including missing or
partial manifests. Ordinary graph/store operations must encounter that refusal
under their shared operation owner before they can read/publish a graph. This
includes existing instances, not only `RecommendationStore::new`. A mixed
legacy/SQL parent must not recover the legacy entry first. The generic anchored
v2 recovery entry point must also refuse the reserved entry. Empty scaffolding
may retain its present compatibility behavior. This is required compatibility
routing, not a general producer-authority implementation.

Unmodified old binaries are unsupported on a migrated workspace under the
accepted contract; this proposal does not claim they recognize the reserved
prefix. Qualified startup must route an activated SQL workspace through its SQL
owner before any legacy recommendation constructor. That production selection
and recovery scheduler remain separate rollout work.

## Restart and progress

A recovery discovery page is only a list of candidates. For each selected job,
read the exact tombstone plus header identity, acquire the existing original
application/lease/lane/reservation authority, and compare the expected basename
before filesystem effects. Use the recovery-only binding that avoids unbounded
legacy startup recovery. Re-read current job/header identity at admission;
a different epoch/workspace/creation, missing job, changed owner, live runtime
or unavailable SQL store denies the attempt without touching journal/records.

Recover only one expected SQL transaction under the shared nonblocking operation
permit. A foreign pending transaction remains visible for its own job; do not
scan or recover it on behalf of the selected job. All partial states retain the
existing anchored root/ancestor/child/leaf identity and strict graph checks.

| Observed state | Allowed result under exact admitted identity |
| --- | --- |
| No journal / verified empty parent | Inspect the full bounded graph, then run the usual removal pass; absence alone does not establish completed cleanup |
| Reserved directory, no complete manifest | Only validate and discard known bounded uncommitted scaffolding/backups using current originals; never unlink canonical records; ambiguous/foreign bytes remain an error |
| Complete draft, optionally interrupted commit temp | Validate exact identity/entries and the existing draft/temp/backup rules, then discard uncommitted journal material without canonical removal |
| Durable committed manifest | Validate exact identity, hashes and reconstructed original graph before remaining canonical unlinks; never restore deleted recommendations |
| Interrupted backup/manifest/directory cleanup | Reuse the established repeated-recovery validation/order; known missing already-removed leaves are idempotent, replacements are errors |
| Invalid/unsupported/foreign/over-budget state | Preserve available evidence, return a visible failure and leave SQL progress unchanged |

A partial first manifest temp is recoverable only when its exact expected draft
bytes can be reconstructed under the existing complete-backup evidence rule.
Otherwise fail visibly; a matching basename does not excuse malformed contents.
Remove and sync that temp before discarding its backup evidence. Do not widen
v2's existing partial-prefix rules or treat arbitrary files as cleanup debris.

After recovering a committed transaction, perform a fresh bounded session-removal
pass while retaining the same recommendation permit. The prior transaction may
have completed file effects before SQL progress failed; the pass must be an
idempotent no-op or remove newly observed references using the existing graph
policy. A recovery helper must not release/reacquire the permit between recovery,
inspection, publication and the receipt handed to SQL progress.

Persist `progress | 0x10` only after all file work succeeds. SQL Busy/failure
leaves the job pending even if canonical bytes are already gone; retry rechecks
all authority and evidence. Preserve earlier bits. Do not finalize the row/job
in this task. Other stages retain their current contracts.

This does not solve stale producers after permit release. SQL-aware source/
tombstone revalidation or revocation, complete writer qualification and final
job-completion gates remain required before activation. Cross-file effects are
not atomic with SQL, and rollback cannot restore deleted payloads.

## Bounds and qualification

Reuse 1,024 logical records, 1 MiB per record/manifest and 32 MiB per bounded
inventory/backup phase; identity/envelope bytes fit inside those caps, not extra
allowances. Count exact root/transaction scaffolding separately as in the current
anchored protocol. Retain constant open descriptors with one-at-a-time backup
reopens and the existing 64-descriptor child regression. Use macOS/Linux
source-only support; unsupported platforms fail before effects. No new queues,
retry timers, caps, performance budgets or platform guarantees are selected.

Required evidence: cross-job/session/creation/epoch/workspace denial; missing and
corrupt SQL; strict codec/name vectors; no-manifest bootstrap; draft/commit/temp
crashes; interruptions during recovery itself; graph/unrelated-byte preservation;
foreign leaf/root/ancestor replacement; mixed legacy/SQL preflight before effects;
legacy existing-instance refusal; original lease/permit retained through SQL
Busy/caller loss; progress/reopen/no-op; free application guards and acknowledged
PTY input while recovery is stalled. Include capped and low-descriptor fixtures.

The implementation [plan](../plans/2026-11-07-sql-recommendation-journal-recovery.md)
assigns these checks. Full repository verification and current-diff manual/CI
reviews are required before merge. No power-loss or live latency improvement is
claimed by process-interruption tests.

## Owner decision and next gate

Proposed decision: accept the strict v3 identity envelope, reserved digest-bound
basename, legacy preflight refusal and exact-job retained-permit recovery flow
above for source-only implementation under #1243. Existing SQLite authority,
backup, producer, migration and native/hierarchy gates remain unchanged.

Owner approval is pending. Record the decision here and as a dated clarification
of [ADR 0103](../../adr/0103-sqlite-session-storage.md) before dependent protocol
implementation; do not mark this proposal accepted merely because its docs PR
passes CI or merges.
