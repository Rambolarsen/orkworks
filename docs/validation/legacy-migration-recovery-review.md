---
type: Decision Review
title: Partial legacy workspace-copy recovery
description: Source-backed recovery risks and a proposed boundary for interrupted workspace-local metadata copies.
tags: [orkworks, migration, recovery, session-storage]
status: draft
---

# Partial legacy workspace-copy recovery

Review: 2026-11-07. Tracking: [#1489](https://github.com/Rambolarsen/orkworks/issues/1489).
Source baseline: `24fbfc12ffc6571db98aa484b880aac13381ccc3`.
The warnings delivered by [#1487](https://github.com/Rambolarsen/orkworks/pull/1487)
close the diagnostics gap in #670. Recovery behavior is unchanged.

## Recommendation and authority

Keep startup's existing skip policy while recovery is designed. Do not remove
the guard, replay the legacy directory, or automatically copy apparently missing
records. A leftover source is evidence to inspect, not permission to adopt data.

Propose routing a recovery decision through the maintenance/import owner in
[#1244](https://github.com/Rambolarsen/orkworks/issues/1244), consuming the verified
backup capability from [#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
This reuses their accepted quiescence, preservation and activation boundaries;
it does **not** establish that their current design already authorizes importing
additional workspace-local sources. An explicit source-adoption spec amendment
and owner approval are required before implementing that extension.

This document is a review and proposed handoff, not an accepted recovery
protocol, implementation plan, operator repair procedure or approval to alter
live data. No recovery command, UI or second migration engine is proposed for
implementation in this task.

## Current source behavior

Two distinct transitions must stay separate:

| Transition | Current owner | Evidence and boundary |
| --- | --- | --- |
| Workspace-local `.orkworks/` to global format-1 files | [`migration.rs`](../../crates/orkworksd/src/migration.rs) | Best-effort copies on workspace open; no completion manifest or recovery state. |
| Global ordinary-session files to format-2 SQLite | [Accepted storage design](../superpowers/specs/2026-11-01-sqlite-session-storage-design.md#migration-and-single-authority-activation) | Qualified maintenance, backup, staged import and valid activation marker; implementation/rollout remain gated. |

In the pinned source, `migrate_if_needed` checks for the old `workspace.json`,
then skips **all** copying if reading global `sessions/` succeeds and its
iterator contains any entry. The guard does not validate a session, completeness
or copy provenance. Otherwise it copies immediate directory entries from
`sessions`, `events` and `capacity`, then `workspace.json`. A failed mkdir/copy
logs and continues. Sources are retained. Directory enumeration errors retain
the existing handling and are outside #1487's mkdir/copy diagnostic fix.

[`SessionApplication::open_workspace`](../../crates/orkworksd/src/session_application.rs)
acquires/adopts the workspace lease, creates global metadata directories,
constructs the metadata owner and takes its workspace-memory edit guard before
calling the legacy copier. Those fences do not provide an import-completion
record or proof that external legacy writers are quiescent. They must not be
promoted into the stronger maintenance guarantee in the accepted storage design.

[`MetadataStore::delete_session` and `delete_events`](../../crates/orkworksd/src/metadata.rs)
remove files beneath the store's global root; they do not remove the retained
workspace-local migration source. Thus a source-only record can be either a
missed import or history intentionally deleted later. This is a source-derived
ambiguity, not a claim that an affected user's data has been inspected.

## Failure traces and recovery hazards

These are source-derived traces, not executed crash, platform or live-data
qualification. #1487's five temporary-directory tests separately verify copying,
source preservation, the skip, failure logging and continuation. This review
adds no new execution evidence.

| Trace | Result under current code | Why a simple retry is insufficient |
| --- | --- | --- |
| At least one session copies, then an event copy fails | Open continues; later opens find a session entry and skip the entire legacy copy. | Missing payload can remain stranded even after the original error clears. |
| One session copy fails while another succeeds | The successful entry makes later opens skip the missing session. | Nonempty is not complete. |
| No session entry copies, but event/capacity/workspace copying succeeds | A later open can enter the copier again if global sessions remain empty and the old workspace file remains. | Existing destinations can be copied over; retry is not create-only or atomic. |
| A migrated session is deleted globally while another global session remains | Retained local source still contains the deleted session; the guard currently prevents replay. | Copying source-only files could resurrect deliberately deleted history. |
| Every global session is later deleted while the old source still exists | The current guard may permit another legacy copy on a later open. | The existing guard is not a deletion ledger; retaining it does not solve every resurrection case. |
| Global metadata or an event log changed after migration | Old local and global bytes may differ legitimately. | Replacing, concatenating or selecting by mtime cannot establish the intended current record/history. |

These traces distinguish the immediate blocked-retry defect from the broader
absence of completion/deletion provenance. Removing the guard trades one failure
mode for overwrite/resurrection risks rather than supplying recovery authority.

## Options considered

| Option | Benefit | Assessment |
| --- | --- | --- |
| Retry startup copies or remove the nonempty guard | Small runtime edit; reaches some stranded files. | Reject: can overwrite newer files or resurrect deleted data, and supplies no quiescence/conflict/commit evidence. |
| Automatically copy only absent destinations | Avoids replacing a present pathname in the happy path. | Reject as a recovery rule: absent does not mean missing import, check-then-copy races remain, and related metadata/payload sets can disagree. |
| Add a standalone resumable legacy repair engine | Could repair format-1 data before SQLite is ready. | Defer unless an actual incident justifies a separately scoped design; it would need its own ownership, backup, conflict and restart protocol. |
| Extend the existing maintenance/import work after a source-adoption decision | Reuses the accepted preservation and recovery owner. | Recommended design route; does not make currently blocked #1244 implementation ready. |

## Proposed handoff to the storage owner

Before accepting extra local sources into #1244's importer, resolve and record:

1. **Source selection and identity:** exact owner-selected workspace/source roots,
   destination identity and complete inventories; distinguish global current data
   from additional retained evidence. The mere existence of `.orkworks/` cannot
   authorize discovery/import or make it authoritative.
2. **Quiescence and backups:** verified copies of both the retained local evidence
   and current global set before mutation; explicit maintenance, qualified writer
   shutdown and owner resolution after an unclean stop, as already required by
   the accepted contract. A workspace lease alone is insufficient.
3. **Ambiguous records:** report source-only, destination-only, differing,
   unreadable, malformed and identity-conflicting records and related payloads.
   Source-only records require an explicit owner adoption/exclusion decision;
   do not infer missed migration, deletion intent or source priority from absence,
   names, sizes or timestamps. Missing evidence remains unknown.
4. **Publication and restart:** resolve candidates in private staging under the
   existing import owner; retain original/current data and decision manifests.
   An interrupted repair cannot silently fall back to older authority. Reuse
   #1244's qualified activation/recovery boundary instead of adding startup merge.
5. **Qualification:** copied/synthetic cases must include interruption after a
   successful session and before its payload, mixed session-copy results, newer
   destination data, deliberate global deletion, conflicting IDs, concurrent
   source change, unreadable input and repeated recovery. Prove no overwrite,
   unapproved resurrection, source loss or incomplete success. Platform and
   durability claims require execution evidence, not these source traces.

These are requirements for the proposed amendment, not a selected conflict
resolution algorithm or permission for a live-user trial. The accepted storage
contract and producer/backup/native/hierarchy gates remain in force. Urgent
format-1 recovery would need its own explicit scope and design before any
mutating tool or manual copy instructions are delivered.

## Review outcome and next decision

A fresh read-only review found no material source inaccuracies or approval
ambiguities. The handoff is [recorded on #1244](https://github.com/Rambolarsen/orkworks/issues/1244#issuecomment-6704127894).
Docs validation passed: metadata/index checks, 1,202 tests and site/link build.
This completes the review deliverable; runtime migration remains unchanged.
The next product decision is whether to extend #1244's source inventory and
owner-controlled conflict decisions, or commission urgent standalone legacy
repair for a demonstrated incident. No current incident or recovery urgency was
established by this source review.
