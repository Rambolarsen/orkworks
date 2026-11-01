---
type: Design Proposal
title: SQLite session storage and bounded history
description: Proposed ordinary-session database ownership, history queries, producer migration and recoverable cutover.
tags: [orkworks, sessions, sqlite, history, migration]
status: draft
workflow_status: proposed
---

# SQLite session storage and bounded history

Tracking: [#1241](https://github.com/Rambolarsen/orkworks/issues/1241), parent
[#1240](https://github.com/Rambolarsen/orkworks/issues/1240). Inspection base:
`120744c71d28cdc760c4f0c572fea82c45ce8116`.
The owner accepted SQLite as the direction and requested this work. The concrete
contract below, its limits and migration are **proposed for owner review**.
No runtime code, live migration, retention change or native qualification is
authorized by this document. The [proposed product spec](../../../specs/session-storage.md)
defines the outcome; this document defines the proposed implementation contract.

## Outcome and scope

Keeping years of history must not add work to terminal input or routine live
updates. Ordinary session metadata and organization have one authoritative
SQLite store. The Sessions view reads live sessions plus bounded history pages.
Replay and images are opened by exact session identity when requested.

Preserve metadata/provenance, checked revisions, summary snapshots, recorded Git
context, native IDs, plan/worktree references, archive decisions, replay, event
history and current retention settings. Add no full terminal search, automatic
session resume, native archive synchronization, multi-workspace runtime or
automatic replay pruning. [#1249](https://github.com/Rambolarsen/orkworks/issues/1249)
separately designs replay retention; migration does not implement it.

Recommendations, workflow observations, capacity records, workspace settings,
readiness artifacts and the proposed hierarchy store retain existing owners.
[ADR 0090](../../adr/0090-workspace-hierarchy-sqlite-transactions.md) applies to
separately gated hierarchy records; this ordinary-session database neither
implements that proposal nor shares a transaction with it.

## Evidence and current caller inventory

The initial investigation found 724 records, five running, 1.2 MiB of session
JSON and about 1.04 GiB of event/replay files. Two full list reads took 804–887 ms
on the earlier runtime. After the owner replaced that runtime, seven read-only
requests on 2026-11-01 returned 725 records, five alive, and 1,489,172 bytes each
in 181.33–226.24 ms including client decode. Backend PID was 40505. The source
checkout included #1223. These are uncontrolled live-list samples, not a paired
speedup benchmark, isolated storage costs or typing-latency measurements.
Raw timing records stay local at `/tmp/orkworks-sqlite-history-baseline/live-list.json`;
no session contents belong in the design or issue evidence.

| Current owner / source | Operation and migration obligation |
| --- | --- |
| [MetadataStore](../../../crates/orkworksd/src/metadata.rs) | Session replacements, legacy normalization, attention/Peon merge, event/checkpoint and replay I/O; split ordinary metadata authority from external payload I/O |
| [session_transactions](../../../crates/orkworksd/src/metadata/session_transactions.rs) | Latest-record mutation, scope clocks, checked u64 revision and conditional/native ownership; preserve arbitration in typed store commands |
| [SessionApplication](../../../crates/orkworksd/src/session_application.rs), [titles](../../../crates/orkworksd/src/session_application/titles.rs) | Create/resume, hooks, metadata patches, title, plan, input/exit bookkeeping and state actions; await durable decisions outside core locks |
| [session_archive](../../../crates/orkworksd/src/session_archive.rs), [retention](../../../crates/orkworksd/src/runtime/retention.rs) | Identity-bound organization and close-first deletion; move organization into SQL, preserve runtime/action authority and external cleanup ordering |
| [terminal_runtime](../../../crates/orkworksd/src/runtime/terminal_runtime.rs), [session_runtime](../../../crates/orkworksd/src/runtime/session_runtime.rs), [peon_runtime](../../../crates/orkworksd/src/runtime/peon_runtime.rs) | Output/activity and inference writes; retain coalescing and generation fences, never queue database work while retaining an input-critical lock |
| [codex_approval_application](../../../crates/orkworksd/src/runtime/codex_approval_application.rs), [codex_completion_application](../../../crates/orkworksd/src/runtime/codex_completion_application.rs) | Conditional dormant mutations and side effects; preserve exact ownership ordering and closed production gates |
| [session_projection](../../../crates/orkworksd/src/session_projection.rs) | Reads every remembered JSON, native existence/plan/Git enrichment and capacity projection; replace display inventory with live reads and indexed pages |
| [session_handlers](../../../crates/orkworksd/src/http/session_handlers.rs), [workspace_updates](../../../crates/orkworksd/src/workspace_updates.rs) | Complete REST list and changed-row stream; version the new bounded protocol, never interpret an incomplete page as an authoritative removal |
| [workspaceSessionController](../../../apps/desktop/src/workspaceSessionController.ts), [api](../../../apps/desktop/src/api.ts) | Whole-list replacement/merge, terminal pruning, selection restore and polling fallback; consume pages and detail by identity without pruning unloaded sessions |
| Workspace open in SessionApplication | `read_all_sessions` feeds orphan reconciliation and recommendation orphan scrub; replace with indexed lifecycle candidates and exact existence queries, not the first UI page |
| Taskmaster/session conflict consumers | Audit all consumers of complete SessionInfo projections during implementation; use exact membership/aggregates or bounded iteration appropriate to each existing domain contract |
| [session-metadata.py](https://github.com/Rambolarsen/orkworks/blob/120744c71d28cdc760c4f0c572fea82c45ce8116/crates/orkworksd/scripts/session-metadata.py) | Existing authenticated API-only helper; currently requires a gated launch marker, so its presence alone does not qualify general producers |
| [POSIX](../../../crates/orkworksd/scripts/report-harness-event.sh), [PowerShell](https://github.com/Rambolarsen/orkworks/blob/120744c71d28cdc760c4f0c572fea82c45ce8116/crates/orkworksd/scripts/report-harness-event.ps1), [OpenCode](../../../crates/orkworksd/scripts/opencode-session-reporter.js) reporters | HTTP identity/attention reports and private native spools; keep spools separate from the metadata database, qualify installed versions |
| MVP agent instructions, generated APM/config assets and arbitrary external agents | Legacy direct session JSON/event writes are permitted today; update canonical instructions and generated outputs through their owner, establish supported producer matrix before cutover |

This is a source-owner inventory, not a claim that every call site is already
qualified. #1242 must inventory the source-controlled/generated/installed paths
and fixtures before activation. Existing external direct writers cannot be
made transactional merely by adding SQLite.

## Storage ownership and schema

Use `<workspace-metadata>/session-store/v1.sqlite` under the retained workspace
lease, with a small sibling `format.json` activation record. A UUID database
epoch binds cursors and backups; a checked store change counter invalidates
history pages. Neither is an authorization credential.

One private worker owns one connection and all ordinary-session SQL, including
reads. Use rollback `journal_mode=DELETE`, `synchronous=EXTRA`, foreign keys,
and a 100 ms SQLite busy timeout; verify effective settings on open. This keeps
one connection and avoids adding a WAL/checkpoint lifecycle for short captured
reads. Reconsider WAL only with measured read/write contention, as a separate
design revision. SQLite documents [rollback durability settings](https://www.sqlite.org/pragma.html#pragma_synchronous)
and [WAL tradeoffs](https://www.sqlite.org/wal.html); those documents are not
qualification of OrkWorks on a particular filesystem/device.

| Logical SQL records | Contract |
| --- | --- |
| Store header | Schema version, canonical workspace identity, database epoch, history generation and import manifest digest |
| Sessions | Primary session ID; immutable creation identity; normalized complete metadata JSON as one canonical SQL value; separate checked metadata revision |
| History columns | State, harness/model, recorded repository/branch, outcome, parsed recency, title and summary projection; derived from canonical metadata in the same transaction |
| Organization | Choice closed/archived, creation identity, checked organization revision and changed-at; Active remains runtime-derived; identity-constrained relation to session |
| Metadata search index | FTS5 over title, summary, recorded repository/branch and exact native-ID text; maintained in the same transaction; never indexes replay/events |
| Deletion jobs | Exact session/creation identity, operation state and bounded cleanup progress; recover external cleanup without presenting deleted rows or resurrecting them |

Do not add an ORM or a permanent public SQL interface. Reuse bundled `rusqlite`;
verify FTS5 support in every build profile. Canonical metadata can remain JSON
inside SQLite: queried fields have indexed columns, and unqueried fields do not
each need a new SQL column. Import preserves unknown JSON members in an extension
map; later patches preserve them. Unknown schema versions fail visibly.

Store revisions as canonical decimal TEXT because existing u64 counters exceed
SQLite's signed INTEGER range. Rust validates/increments them without wrapping.
Use checked signed integers only for new counters explicitly limited to i64.
Never sort decimal-text revisions lexically or use floats. Import missing legacy
metadata revision as zero, preserving the existing contract; preserve creation
identity and effective provenance, including legacy mtime-derived freshness.

Indexes cover `(state, recency DESC, id DESC)`, harness plus that ordering,
session identity and external/native identity lookup. The SQL schema and query
plans must demonstrate actual index use in #1243/#1245. An arbitrary metadata
search can still be expensive; the query budget below applies to it too.

## Interface, scheduling and publication

The module exposes typed `create`, `patch(expectedRevision, scope, intent)`,
`setOrganization(expectedIdentity, expectedRevision, intent)`, `get(id)`,
`listLive(ids)`, `history(query)`, `exists(ids)`, retention candidates and
deletion-progress commands. Callers never submit SQL or arbitrary filesystem
paths. Domain validators retain authority; store commands check the latest
record and preserve source-priority/age/scoped-write rules transactionally.

Input-critical projection/workspace/sessions locks may capture bounded runtime
identity and immutable intent only. Release them before awaiting worker admission
or SQL. Refactor current synchronous metadata closures at affected call sites;
wrapping them in a database call while retaining existing guards is prohibited.
Per-session application reservations serialize conflicting lifecycle/metadata
commands, including pending finalize/resume/close. Revalidate captured workspace,
runtime generation and control/source ownership at the existing operation's
authority cutoff. A store completion cannot authorize a replacement runtime.
The implementation plan must trace each dormant conditional-native writer;
SQLite alone cannot move or weaken its check/effect ordering.

Proposed scheduler: at most 32 mutation commands and 8 read requests queued;
each carries at most 1 MiB of serialized intent, with a total 32 MiB admission
budget. The normalized canonical metadata record, including preserved extensions,
is limited to 1 MiB on import and after every mutation. A history page returns
at most 256 KiB; an exact complete-detail/metadata response has a separate 2 MiB
limit, including envelope and bounded derived availability fields. Never truncate
canonical metadata to satisfy a response cap. Import streams records outside the
normal runtime queue. Unsupported oversized legacy records stop import with a report;
they are retained unchanged and never silently dropped/truncated. Durable content
limits require explicit owner repair/export, not automatic data loss.

Serve up to eight mutations then one queued read, retaining FIFO ordering among
mutations and among reads. Reject queued work after 2 s if it has not begun.
Reads use cooperative SQL progress cancellation after 100 ms of computation;
external calls/network serialization happen after capturing rows and closing the
read transaction. Filesystem stalls/fsync are not hard bounded by SQLite's
progress hook or busy timeout. No history/DB operation is on PTY input delivery's
critical path, even during those stalls.

Publish durable record/cache changes and invalidation only after commit.
Creation must commit before spawn is scheduled; resume/state actions retain
their existing admission and cleanup gates. Input delivery and raw PTY draining
continue independently; output-derived metadata remains coalesced, and a failed
flush marks storage degraded rather than claiming durable success. Existing
bounded replay persistence/backpressure is not redesigned by this work.

Before execution, cancellation can prove no mutation happened. After execution
begins, cancellation/disconnection does not prove rollback: await a definitive
result internally, expose unresolved status if transport times out, and require
fresh state/revision before another intent. Never blindly retry a stale command.
Corruption/disk-full/queue/busy failures are distinct from missing rows; preserve
last known display with an unavailable/degraded indication, disable unsafe
mutations and keep terminal interaction available. No empty-list fallback,
automatic database recreation or direct-JSON fallback is permitted.

## Paged history and live transport

Negotiate `sessionReadProtocol=2` before consuming the new endpoints/stream.
New clients use these proposed routes; exact TypeScript/Rust DTO names remain
an implementation choice, but the wire behavior is fixed:

| Read | Behavior |
| --- | --- |
| `GET /sessions/live` | Complete lightweight summaries of current managed runtimes and pending finalizers; cost proportional to live count, never historical count; separate live bounds below |
| `GET /sessions/history?state=closed|archived|all&limit=50&cursor=...&q=...` | Default 50, maximum 100 rows and 256 KiB; lightweight summaries, exact ID and action preconditions; no payload/native/Git scans |
| `GET /sessions/:id` | Exact complete metadata plus current organization and separately attributed availability, at most 2 MiB; expensive plan/native checks are bounded detail/action work |
| Existing replay/event routes | Load only for the selected exact identity; retain current bounds and historical content semantics |

Summary rows are lightweight DTOs, not canonical metadata. Each encoded row is
at most 8 KiB, with explicit preview/truncation flags and exact session identity,
organization and action preconditions preserved. Large title/summary/path display
values use UTF-8-safe previews; complete values remain available by identity.
Fill a history page until its requested row limit or byte budget is reached;
advance the cursor from the last included row and report `hasMore`. At least one
maximum-sized row fits, so large valid records cannot prevent pagination progress.

The complete live response is exempt from the history-page cap: its allocation
and wire size are bounded by `(8 KiB * captured live/finalizer count) + 16 KiB`
and the admission budget, not historical count. Exact canonical data still uses
the detail/metadata route. If the live inventory cannot fit the command/result
memory budget, return an explicit too-large/unavailable error without an
incomplete authoritative snapshot; existing terminal sockets continue. No new
active-runtime count limit or silently truncated live list is introduced.

History sorts descending by the newest valid recorded last-output/activity time,
then ID; invalid/missing times use creation time, then a fixed minimum. Encode
parsed UTC microseconds in the indexed sort column. Query filters include state,
harness and recorded repository/branch exact values. Free text is AND of literal
metadata tokens, with prefix matching for the last token, using FTS5 unicode61
tokenization. Limit text to 256 UTF-8 bytes and eight tokens; reject excess/raw
query operators rather than accepting caller FTS syntax. Search is metadata-only,
ordered by the same recency key, not relevance. See [FTS5](https://www.sqlite.org/fts5.html).

Cursor binds database epoch, workspace identity, history generation, filter/query
digest and last `(recency,id)` tuple; it is opaque presentation state, bounded
to 2 KiB. Changed membership/order/search data increments history generation
transactionally. Reject stale pages with `409 history_changed`; refresh loaded
pages rather than mixing generations. Unrelated live output does not change the
history generation. Reopen/replacement invalidates old stream/page instance tokens.

The version-2 update stream carries complete live changed rows/removals and a
coalesced `history_changed` generation. It never streams all historical rows or
recomputes complete history on each PTY event. Gaps/reconnects re-read live state
and invalidate history pages. Closed/resumed transitions invalidate both views.
REST fallback uses the same bounded reads, not the old full-list projection.
Old unversioned list/stream requests on a migrated workspace return an explicit
upgrade-required response, not a silently truncated list. Electron and sidecar
must negotiate compatible versions before workspace activation.

The existing Sessions surface keeps All unarchived, Active, Closed and Archived
filters and date ordering. Default view shows the live portion plus the first
history page. Keep at most 200 history rows in renderer memory, virtualize the
visible rows and evict old pages while preserving cursor anchors. Retain selected
detail separately from loaded pages. An off-page ID is unloaded, not deleted;
filtering/paging never selects, acknowledges, closes or resumes anything. Fetch
selection restore by ID and preserve one terminal attachment per runtime.

Internal consumers do not use display pages as complete inventories. Existence
joins for recommendation cleanup use exact/batched membership; startup queries
only unresolved lifecycle candidates. Taskmaster and conflict analysis preserve
their current evidence scope through typed aggregates or bounded chunk iteration,
with workspace/generation validation and explicit incomplete-read failure.

## Historical facts and native availability

On durable ending, retain last recorded repository/worktree/branch, model,
summary, outcome and timestamps. History does not run fresh repository scans or
rewrite these fields when the checkout changes. Import preserves the facts
actually recorded; it cannot reconstruct earlier branch states. A legitimate
resume retains the existing session ID/creation identity contract and produces
new activity; this work adds no separate attempt-history feature.

Native ID/reference is retained evidence, not proof of resumability. Lists show
resume availability as unverified until detail/action validation checks the
exact supported native identity. Missing worktrees, replay, plan or native
conversation do not erase the session. Detail distinguishes unavailable,
unsupported and not-yet-checked resources. Current validation never implies
process adoption, latest-session fallback or automatic resume.

## Producer transition and native gates

The current Codex marker/capability is tied to the separately gated #789 native
path. #1242 must introduce an independent general storage-protocol negotiation
for supported ordinary launches; it must not spoof that capability or turn on
native startup/attention clearing. Reuse authenticated revisioned metadata
routes, source rules and actual-context GET/validate-only-PATCH qualification.
Keep its support matrix distinct from native eligibility.

Change canonical agent reporting instructions and reconcile installed stable
helpers through their normal owners/confirmation rules. Qualify Codex, Claude,
OpenCode and any configured custom/generic tools according to what actually
produces metadata; unsupported explicit reporting remains visibly unavailable,
never converted into a claim of working reporting. Hookless observation may
continue where already supported. The qualified negotiated protocol is mandatory
for agent metadata writes in migrated workspaces, with no direct-file fallback.

Legacy direct JSON **reads** also cease to be current after activation. Preserve
read-only migration backups and offer an explicit export for inspection; never
advertise them as a live metadata feed. Event NDJSON remains external and retains
its current legacy append contract; backups require those writers quiescent.

## Migration and single-authority activation

Supported cutover is maintenance at workspace open, with no managed runtimes,
pending lifecycle actions, provider results or metadata/event writers. Stop
background mutation scheduling and acquire the existing workspace lease before
capture. A normal running workspace stays on JSON until a controlled maintenance
window; do not restart another session or silently close sessions to migrate.
Lease acquisition or stale metadata is not evidence that external descendants
have stopped: after an unclean stop, require explicit owner quiescence resolution
before migration. No crash-surviving process adoption/cleanup is added.

1. With JSON still authoritative, create and verify a full workspace backup via
   #1250's maintenance-only procedure. Capture source inventory/hash/size and
   organization identities without mutating the source records.
2. Stream validated records into a private staging database, retaining original
   metadata fields/extensions and hydrated legacy provenance. Validate every
   organization record. Identity mismatches, invalid JSON/revisions, unreadable
   records or unsupported oversized input abort with paths and reasons; orphan
   organization records are inventoried but cannot create sessions.
3. Commit import/schema header and derived indexes. Verify record counts,
   identities, metadata round trips, organization protection, integrity and
   external reference inventory. Recheck the source manifest; any change aborts
   activation. Durably close/flush the staging database.
4. Atomically publish `session-store/v1.sqlite`, then atomically publish and
   directory-flush `format.json` binding format 2, database epoch/schema and import
   digest. **The valid format-2 marker is the activation point.** No runtime starts
   before this completes. Platform implementation must qualify these flush/rename
   operations; unsupported durability cannot be advertised as qualified.
5. Normal legacy opening is allowed only when there is no activation marker,
   final database, session-store directory or owned staging/cutover evidence.
   JSON remains authoritative during the importer before activation, but normal
   workspace opening is blocked if any such evidence exists without a valid
   marker. In particular, a final database with a missing marker is **never** a
   reason to fall back to JSON. Maintenance recovery compares the retained import
   manifest and SQL change counter: only a verified import-only database with
   unchanged sources can finish activation or explicitly discard owned staging.
   Post-cutover writes or ambiguous evidence require same-version backup recovery,
   not a switch to stale JSON. With a valid marker, SQL is authoritative;
   missing/corrupt/mismatched database or unknown/unreadable marker stops open.
   Never initialize empty or reactivate JSON automatically. This is not protection
   against deliberate removal/rollback of every copy of storage-format evidence
   by the same OS user; such destructive recovery needs the complete backup.
6. Legacy JSON remains immutable backup evidence and is not watched/imported as
   current data. A qualified writer never targets it. Detect unexpected legacy
   session JSON changes during reconciliation and surface incompatible-writer
   diagnostics; do not merge them into SQL. Retain migration backup until explicit
   owner cleanup, outside session retention.

The marker is not an OS security boundary. Old unmodified binaries cannot be
forced to understand it; concurrent/older writers are unsupported and must not
be run on a migrated workspace. Packaging/startup can reject known incompatible
client/sidecar combinations, not claim to control arbitrary same-user binaries.
Post-cutover downgrade requires maintenance, a fresh complete backup and a
verified export of **current** SQL records/organization to the older format;
only then may authority switch back. Merely restoring pre-cutover JSON loses
later writes and is not a supported lossless downgrade. Initially the supported
recovery is same-version backup restore; older-format export is a later gate
unless #1244 implements and qualifies it before rollout.

## External payloads, events, backup and deletion

Keep `events/<id>.terminal`, terminal-size sidecars, `events/<id>.ndjson` and
`session-images/` where they are. Do not import replay blobs into SQL or bulk
trim dormant files. Summary checkpoints remain the existing NDJSON history;
this migration does not implement #313 or promise atomic SQL+event append.
Preserve append-before-metadata requirements where they exist. Failed compound
operations report their actual partial result; event presence alone is not
authority that the matching SQL mutation committed. #1241 review must reject any
claim of cross-file atomicity without a separate approved protocol.

#1250 initially supports a **quiescent complete workspace backup**, not online
snapshotting across external writers. Pause all module writers, drain admitted
operations and then close connections while retaining maintenance ownership;
use SQLite's supported backup mechanism if a connection must remain open.
Copy database, activation marker and workspace-owned referenced/related stores,
including events/images, recommendation/observation evidence and workspace
settings, into a private staging backup. Include an inventory with schema/epoch,
relative paths, sizes/hashes and known missing optional payloads; exclude locks,
runtime credentials, transient native relay mailboxes and process handles.
Global tool settings/native conversations and repository/worktree content are
outside this backup and must be explicitly listed as excluded.

Validate the complete set before marking backup success. Restore into staging,
validate all required records/references and activate only in a quiescent
destination under its lease. Existing destination data is backed up and retained
until activation is definitive. Restore grants no live process or native resume
authority. SQLite's [backup API](https://www.sqlite.org/backup.html) covers the
database snapshot, not referenced external files.

SQL state changes and revision checks are transactional. External deletion uses
an identity-bound durable job: retain a non-displayable tombstone, reserve the
session operation, close/finalize first using existing gates, then remove
external content and dependent evidence through existing owners. Persist progress
and retry idempotently; a failure remains visible, does not resurrect a row, and
does not run against a reused identity. Completion removes the row/job only
after required cleanup. Do not claim rollback restores deleted payloads.
Recommendation/evidence protection stays authoritative. Archived/unknown state
remains retention-protected; original 0/0 defaults and applied limits survive.

## Proposed measurable budgets and verification

These are proposed release criteria, not measured SQLite results. Use the same
five active sessions, PTY output/input workload and machine/build profile at
724, 10,000 and 100,000 synthetic historical records. Capture source SHA,
dependency versions, OS, storage medium, row/content distributions, warm/cold
conditions and contention. Keep fixture creation outside the measured interval.

| Surface | Proposed criterion |
| --- | --- |
| Live list/update preparation | No history enumeration, payload reads, native/Git history checks or full-history SQL; work proportional to five live sessions |
| History page | Default 50 / maximum 100 rows, serialized response at most 256 KiB; p95 service time at most 100 ms warm, 250 ms cold on the qualified profile; timeout is explicit failure |
| Search | Same payload/page limits; p95 service time at most 250 ms warm on declared selective and broad queries; explicit budget failure instead of an unbounded scan |
| Input responsiveness | PTY key-to-echo p95 at most 50 ms, p99 at most 100 ms; 100k-history delta versus 724-history at most 5 ms p95 and 10 ms p99 under the same five-session workload |
| Renderer | At most 200 loaded history rows plus selected detail/live rows; history-driven retained heap growth from 724 to 100k at most 10 MiB after GC under identical browsing |
| Runtime memory | No all-history in-memory metadata cache; steady-state history-driven sidecar RSS growth at most 32 MiB between 724 and 100k, excluding database file cache reported separately |
| Failure | Queue rejection/busy/query interruption is explicit; deliberately stalled storage cannot prevent independent terminal delivery; no false durable-success response |

Use at least 1,000 input events and 200 page/search reads per measured workload,
report percentiles and failures, and repeat three runs. Qualify each claimed
platform/filesystem profile; unsupported profiles retain explicit rollout gates.
Storage microbenchmarks supplement, never replace, end-to-end measurement.
Full matrix baselines have **not** been captured during this design-only task;
they require an isolated harness and are a readiness gate before accepting
performance claims. #1247 owns integrated evidence; #1241 must approve the fixture
contract/budgets and record the preimplementation baseline handoff.

Fault injection covers every activation boundary, marker deletion after
post-cutover SQL writes, invalid/missing markers and database, preserved unknown/legacy fields, counter exhaustion, organization
identity, stale runtime/workspace, external cleanup interruption, disk/busy/queue
failure and backup/restore after post-cutover writes. Pagination covers ties,
invalid cursors, history churn, off-page selected IDs and no authoritative removal
from partial results. Verify Taskmaster/conflict consumers with relevant records
beyond the first page and history with missing native resources/worktrees.

## Delivery, review and remaining gate

1. #1241: owner review of this contract and product amendment; settle baseline
   harness/budgets, record the accepted ADR and concise implementation plan.
2. #1242 + #1243: qualify ordinary producers and implement private storage, with
   no production cutover. #1248 fixes historical-fact/availability behavior.
3. #1250 + #1244: complete backup/recovery, migration and cleanup integration.
4. #1245 + #1246: bounded transport and desktop paging using the same negotiated
   contract; preserve complete internal consumers and exact selected history.
5. #1247: integrated failure/performance evidence and controlled owner rollout.
   #1249 remains independent and cannot block preservation-first migration.

Independent builder/completeness and source-correctness reviews on 2026-11-01
identified two material contradictions. Both are resolved here: history pages,
complete metadata and live summaries now have distinct size contracts with
pagination progress; missing markers plus any cutover evidence now block normal
opening rather than re-enable stale JSON. The reviews found no additional
material source/native/hierarchy contract blocker in their inspected scope.
The owner must still approve the proposed contract and limits; runtime behavior,
platform durability and large-history performance remain unverified. Owner
approval is required because this changes the existing storage/protocol/migration
contract, as specified by root AGENTS.md. Approval of SQLite direction alone is
not approval of new numeric limits or a live cutover.
