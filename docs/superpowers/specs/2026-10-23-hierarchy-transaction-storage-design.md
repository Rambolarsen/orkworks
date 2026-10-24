---
type: Design Proposal
title: Workspace hierarchy transactions and bounded SQLite storage
description: Proposed durable transaction owner, writer integration, summary checkpoints and bounded storage recovery for hierarchy projections.
tags: [orkworks, hierarchy, storage, recovery, sqlite]
status: draft
---

# Workspace hierarchy transactions and bounded SQLite storage

Tracking: [#746](https://github.com/Rambolarsen/orkworks/issues/746),
[#744](https://github.com/Rambolarsen/orkworks/issues/744), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Source-inspection base: `fe50f57122e55d172736bcccad16819f2ba0ee1a`.
Status: **architecture direction accepted for further design/qualification;
detailed design and numeric limits remain proposed**. See the
[scoped acceptance](#scoped-architecture-direction-acceptance--2026-10-24).
[ADR 0090](../../adr/0090-workspace-hierarchy-sqlite-transactions.md) records the
same scope. Acceptance supplies no implementation or probe authorization.

The [accepted recovery behavior](2026-10-20-hierarchy-read-recovery-design.md)
and [qualification register](../../validation/hierarchy-writer-storage-qualification.md)
remain the requirements. This proposes their storage mechanism; it does not
accept the wider producer/UI contracts, qualify any OS/filesystem, or open
#740's deferred capability gate. Shared-learning custody and runtime plans stay
separately gated. No code, database or live probe is delivered here.
The [scoped #741/#742/#744 technical acceptance](2026-10-04-assignment-evaluation-design.md#technical-contract-acceptance--2026-10-24)
is recorded separately from the later architecture-direction acceptance. Neither
accepts the storage defaults, broader #746 producer/UI mappings or a native
storage profile.

## Proposed decision and alternatives

Use one workspace-local SQLite database for the new hierarchy source records,
heads, source-owned retry/replay receipts, dependency index, summary rows/fences
and ordered pending summary transitions. Commit each logical producer change
and all its dependent protection in one database transaction. Preserve each
producer's canonical record bytes, authorization, quotas and replay semantics.
This adds no cross-workspace transaction and migrates no existing store.

The sidecar already depends on bundled `rusqlite` in
[`Cargo.toml`](../../../crates/orkworksd/Cargo.toml); its existing
[`codex_session_store`](../../../crates/orkworksd/src/codex_session_store.rs)
reads another application's database and is not this module. Reuse the dependency,
not that database or its permissive reading policy.

| Alternative | Disposition |
| --- | --- |
| Extend recommendation manifests across hierarchy stores | Reject for this slice: new cross-owner redo/rollback, lineage and rotation code would sit beside SQLite, which is already available |
| Independent JSON files with post-write invalidation | Reject: cannot atomically bind a source correction, retry receipt, dependency protection and summary transition |
| One SQLite database, rollback journal | Direction accepted for design/qualification: one durable unit without an application redo log or a multi-database commit; detailed contract remains gated |
| SQLite WAL | Defer: concurrent long-lived readers and WAL checkpoint scheduling are unnecessary for bounded captured reads; avoid a second log-growth policy |

SQLite documents rollback-journal atomicity and hot-journal recovery in its
[atomic commit description](https://www.sqlite.org/atomiccommit.html). Its
[PRAGMA documentation](https://www.sqlite.org/pragma.html#pragma_synchronous)
distinguishes EXTRA synchronization from FULL in DELETE mode. These describe
engine behavior, not evidence that OrkWorks' build, VFS, filesystem and device
satisfy the contract. Native qualification remains required.

## Durable unit and storage ownership

Propose `<workspace-metadata>/hierarchy/v1.sqlite`, owned by the selected sidecar
under its existing workspace lease. A single private storage worker owns the
read/write connection. No renderer, reporter, parent/child agent, second sidecar
or external file writer receives a database handle. No `ATTACH`, shared cache,
SQL extension loading, caller SQL, or direct JSON mirror is supported.

The module exposes typed producer commands, coherent capture and exact record
inspection. Callers supply validated source intents and expected identities;
only the module discovers affected dependencies, reserves capacity, writes
receipts/lineage and commits. Callers cannot submit an arbitrary SQL transaction
or assert their own affected set. Authentication and source-owner validation
remain mandatory before admission. For the accepted listed #744 publication
class, current authority is rechecked at its exact claim under the short
revocation ordering before SQL; commit validates the claim binding and all
source/predecessor/dependency/quota conditions. Excluded actions keep their own
current-authority cutoff and qualification requirements.

Proposed internal responsibility split (module names are new, not existing code):

| Owner | Responsibility |
| --- | --- |
| `hierarchy_store` | Connection/configuration, storage epoch/schema, source objects/heads/receipts, dependency membership, capacity accounting, transaction identity, summary checkpoints and atomic purge |
| `hierarchy_publication` | Ordered command admission, pending/health fences, immutable copies of ordinary display values, tracked-run snapshot invalidation and capture/final validation |
| #741/#742/#743/#744 domain modules | Strict canonical validation and domain decisions; submit typed intents, never bypass `hierarchy_store` |

Canonical source objects remain BLOBs with their original domain digest. Storage
metadata never enters a source digest. Strictly validate canonical bytes, full
identity and digest on admission/reload. SQL keys use complete tagged identities,
not session IDs alone. Store u64 source/summary/transaction counters as checked
canonical decimal text; SQLite signed INTEGER cannot represent maximum u64.
Compare/increment them in checked Rust code, not SQL lexical ordering or floating
point. Reaching the internal commit-counter maximum fences new durable mutations;
summary-counter exhaustion retains its distinct accepted behavior.

One transaction includes every changed source object/head, its source receipt,
incoming/outgoing dependency changes, exact #744 material changes or durable
unavailable fences/pending transitions, and their quota counters. The commit
identity is `(storageEpoch, commitSequence, changeDigest)`. `changeDigest` is
SHA-256 over `orkworks.hierarchy-storage-change.v1` + LF + canonical metadata
containing the schema, workspace/epoch, sequence, predecessor commit identity,
command kind and sorted exact before/after row identities and content digests.
It excludes credentials and its own digest. Store this identity in the source
receipt where one exists, and the latest commit identity in the store header.
It is integrity metadata, not a new unbounded audit log or a replacement for
#742/#743 receipts. Hashes do not establish authenticity or protect against a
same-user rollback of the entire store.

## Publication and ordinary-session integration

The single worker executes durable commands FIFO, one at a time through commit
and reference publication. A command reserves a volatile publication position
and marks affected tracked runs reconciling before database work starts. If the complete affected set is
unknown, fence all tracked runs in this workspace. Complete hierarchy reads are
blocked by a pending position; ordinary session/terminal operation continues.
The private worker performs bounded validation/SQL work and commits without
holding workspace, session, metadata-lane, catalog or existing projection locks.
After durable commit, publish the immutable source references and invalidate
snapshots before clearing that pending fence. Notify only afterwards.

Ordinary writers announce a pending source change before changing any copied
hierarchy-visible value, then publish its immutable value and source generation.
Their short critical section uses existing `projection_lock` first, then the
new publication-state latch, then the existing workspace/session/catalog locks
needed by that path. Never acquire `projection_lock` while holding the new latch;
the storage worker acquires neither during SQL or fsync. Metadata-lane writes
announce before acquiring the lane and publish after releasing it. A caller
holding a core lock cannot synchronously wait for the storage worker.

The database commit is the durable command's linearization point. Its source
readers remain fenced until references and snapshot invalidation publish under
the latch; no later durable command overtakes that publication. A failed command
releases its position only after rollback/recovery proves its disposition.
Positions are not source receipts or durable commit counters. Exact retries
retain the original commit identity and do not allocate another transition.

Ordinary copied display values have a separate checked, volatile generation;
they do not acquire a durable command position. Attention, lifecycle, label,
model/time display and runtime identity are not #744 result evidence. Changes
to those copies alone never change summary material or append lineage. Exit,
attention and resume cannot establish quality, failure or acceptance. Runtime
authority/currentness still must be validated for the corresponding operation;
unverifiable runtime joins fence availability, not invent a material outcome.

If an ordinary source changes during a durable command, its generation advances
independently while hierarchy readers remain fenced. After commit the worker
publishes the new database references alongside the latest complete ordinary
copies. A read must validate both tokens; there is no claim that the ordinary
change preceded or followed the durable change in one persisted history. This
is sufficient because ordinary copies cannot change that transaction's material.
A pending ordinary write or unknown generation keeps the read reconciling.
If the checked ordinary-generation increment exhausts its range, mark that
publication lifetime terminal/unavailable for complete hierarchy reads; never
wrap, reuse a token or reset the counter in place. Ordinary controls continue.
Only an explicitly established fresh publication lifetime, with all source
copies/joins recaptured and old requests fenced, may use a new generation scope.
This is not a durable summary reset, sidecar auto-restart or restored authority.

An actual #744 evidence-currentness change (for example detected loss of a
retained immutable source) is different: announce a health fence immediately,
then submit a typed source-owner command. It commits the authoritative health,
affected dependency/material transitions and receipts together. Multiple such
committed changes count in FIFO order, including A→B→A; they cannot be replaced
by one latest-generation display update. If its protection cannot commit, keep
the health fence and deny current reads/actions. No ordinary display callback
may silently perform that source-owner mutation.

A stalled worker cannot stall PTY draining or force ordinary changes to acquire
new durability obligations. Startup loses volatile copies and revalidates all
ordinary/runtime sources before complete hierarchy reads; saved copies cannot
restore runtime or reporting authority.

Read capture obtains immutable database references and copied ordinary values
with their membership, health, pending-position and generation tokens. Assemble
outside the latch; revalidate every token at publication as the recovery contract
requires. Existing two-attempt/five-second/response-size limits are unchanged.
Equal snapshot versions still serialize identically. No SQLite read transaction
stays open while a renderer inspects details or while files are copied/hashed.

### Caller cancellation and authority changes

Queue admission is not source acceptance or a durable receipt. After a bounded
command has entered the worker queue, client cancellation, disconnect or timeout
detaches its waiter; it does not silently remove the command, refund source-rate
charges, imply rollback or release its publication position. The worker still
validates the applicable authority/claim and predecessors and either commits
under the source contract or proves rejection/rollback. Only a listed #744
operation that acquired its exact accepted claim before revoke may finish after
revocation; an unclaimed queued operation has no such exemption. A later exact retry
resolves the committed receipt before mutable-head CAS; an uncommitted attempt
has no fabricated success receipt. Internal coalescing/reordering/dropping of
queued source mutations is unsupported by this proposal.

Once SQL is active, a caller deadline or interrupt request cannot prove its
transaction disposition. Keep the worker charged and affected hierarchy fenced
until rollback, commit or same-worker recovery is established. Committed objects,
receipts and lineage survive a lost response. Never spawn a replacement worker,
rerun a command or delete a journal merely because its waiter disappeared.

Caller cancellation is distinct from actual run/runtime revocation, lease loss
or namespace change. The owner accepted the
[scoped #744 claim/lease/retirement amendment](2026-10-24-hierarchy-authority-ordering-design.md#owner-acceptance-and-handoff--2026-10-24):
revoke-before-claim denies a new operation; claim-before-revoke may complete only
that exact allowed attempted transaction. Bearers close immediately for queued/
later work and reads; durable acceptance still occurs at SQL commit. This is an
explicit exception to the former revocation-before-physical-commit policy, not
a claim that a volatile precheck/interrupt enforces that former guarantee.

The accepted policy also requires bounded conflict for unsettled finish/namespace
rotation, one active-or-retiring worker/claim across switches, and retaining its
old store lease until disposition/close. A stuck worker may hold that old workspace
indefinitely; new hierarchy work cannot bypass it, while ordinary stop/PTY stays
independent of SQL. Actual lease loss gains no exception. Research reports,
launch/control/cleanup/evidence/shared-learning actions and sidecar finalization
retain their distinct contracts and unresolved gates. Before an executable
handoff, qualify the exact claim binding, authority owner, lease lifetime and
publication ordering without ordinary core locks across SQL/fsync. Policy
acceptance does not supply that enforcement proof or approve runtime code.

### Writer routing

Every row below must be routed before complete hierarchy delivery. New owners
are proposals. Existing entry points are inspection anchors, not proof that
all their callers already participate.

| Mutation family | Proposed route and existing integration anchors |
| --- | --- |
| #741 immutable configuration and catalogs | New configuration admission through `hierarchy_store`; copied catalog labels invalidate through [`HarnessStore::mutate_at`](../../../crates/orkworksd/src/harness/store.rs). Immutable approved bytes never follow mutable labels |
| #742 coordination | New run/plan/grant/clarification/allocation commands share the store transaction. Existing [`CoordinatorStore`](../../../crates/orkworksd/src/taskmaster/coordinator_store.rs) remains data-only; do not dual-write or import its approvals as live grants |
| #742 reports | Initial/corrected descriptor, exact head, receipt and preparation/dependency changes commit together. Rate accounting precedes receipt lookup; exact retry precedes head CAS. Retry returns its original descriptor without another transition |
| #743 skill evidence | New event/alias/replay/high-water/aggregate/coverage commands use the same store; clear, trim, generation change and detected degradation are writers too. Preserve #743's distinct replay/retention rules; no scoring from telemetry |
| #744 results/evaluations | Candidate/finalization, result head, reviewer/user stream replacement, disposition/invalidation, eligibility, incoming dependencies and subjectRevision CAS share the transaction |
| #744 summaries | Store-owned initialization, material comparison, pending-transition/checkpoint update, unavailable/exhausted fences, byte/count reservations and purge; no read-time initialization |
| Ordinary lifecycle/runtime identity | [`SessionApplication`](../../../crates/orkworksd/src/session_application.rs): `create_session`, `resume_session`, `transition_session_status`, `complete_session_ending`, `open_workspace_with_identity`; [`terminal_runtime`](../../../crates/orkworksd/src/runtime/terminal_runtime.rs): `set_session_status_for_generation`, `finalize_session_ending`. Fence workspace/lease/runtime replacement before new copies |
| Ordinary attention/work/labels/model/time | `SessionApplication`: `persist_peon_observation`, `apply_attention_signal`, `report_prompt_attention`, `persist_process_transition`, `persist_input_label`, `persist_codex_label_for_runtime`, `persist_output_recency`; [`observed_status`](../../../crates/orkworksd/src/runtime/observed_status.rs) live-field helpers and [`metadata/session_transactions`](../../../crates/orkworksd/src/metadata/session_transactions.rs) commit lane. Include native clears, idle timeouts, report/hook/API paths and direct runtime-handle writes |
| Retention, removal and health | `SessionApplication::forget_session`, [`retention::delete_session_evidence` and `retention_cleanup_once`](../../../crates/orkworksd/src/runtime/retention.rs), metadata quarantine/read errors, startup recovery, [`workspace_gc`](../../../crates/orkworksd/src/runtime/workspace_gc.rs), and proposed #744 purge/#743 clear. Cross-workspace custody remains closed; do not make protected source deletion appear safe through a workspace-only transaction |
| Projection/read | [`SessionProjection::snapshot/list`](../../../crates/orkworksd/src/session_projection.rs) is an ordinary-list precedent only. New hierarchy overview/detail commands use publication capture/final validation, exact membership and source-health tokens |

Before code, produce a caller/direct-write inventory against the implementation
base and map each mutation to a row plus its generation/fence test. Include
background and read-triggered mutation, global catalog reload and non-sidecar
metadata producers. An unsupported external writer blocks complete projection;
file watching after a write is not serialization. #789's API-only producer
preparation does not establish that all producers are mediated.

### Ordering examples

These are proposed contract fixtures, not executed tests:

| Interleaving | Required result |
| --- | --- |
| Ordinary attention changes while a report correction commits | No hierarchy read during pending work; next read pairs the committed report/fences with current attention copies. Attention adds no summary revision |
| Ordinary generation changes after capture | Final validation retries/fails; it cannot repaint the captured values under the new token |
| Two durable required-evidence corrections queue A→B→A | FIFO commit/publication preserves two material increments, regardless of ordinary status changes between them |
| Corrupt evidence is detected while a source command runs | Immediate health fence blocks current reads/actions; source-owner reconciliation commits in order before that fence clears. A display-generation update cannot clear it |

## Summary lineage and application checkpoints

A current summary row is its own durable checkpoint: complete identity, exact
material bytes, summaryRevision and last applied storage commit identity. Keep
this checkpoint as storage metadata outside strict `SummaryRowV1`. A successful
source transaction updates it atomically with the source. No application redo
log is needed for an ordinary fully installed transaction.

When summary capacity prevents installing the new material, atomically retain
the last proven checkpoint, mark the summary unavailable and append ordered
pending material transitions. Each transition retains exact before/after
material bytes, assigned summary revision and originating commit identity;
canonical byte equality, not hash equality alone, decides material change.
Optional-only changes append no material transition. A→B→A retains two increments.
Checkpoint/pending bytes consume the additional bounded lineage budget below;
retained summary rows still consume the accepted row budget.

Reconciliation may fold an ordered prefix into a new complete unavailable
checkpoint and remove that prefix in the same database transaction. This is
proof-preserving compaction, not history eviction to admit a correction. Keep
both the last applied material/counter and all unapplied transitions. Publish a
current row only when the entire sequence has been applied and row capacity is
reserved. Never discard a transition before its checkpoint commits. If neither
checkpoint nor intact pending proof survives, stay unavailable; latest sources
cannot reconstruct revision 1. Initial admission without row capacity records
`never-initialized` storage metadata and never later initializes from a read.

An exhausted summary records a terminal fence; authorized source corrections
need not retain impossible further material increments. Purge is the sole
subject removal path: exact subjectRevision CAS and no incoming retained
dependency, with sources, summary, pending proof and quota changes committed
together. Ordinary evidence deletion cannot bypass it. No automatic history
eviction, destructive recovery, source-quota increase or shared custody is added.

## Proposed finite limits

These are new proposed implementation defaults for owner review. Narrower
#741/#742/#743/#744 source limits always win. Capacity rejection preserves old
state; it never truncates an accepted command or weakens replay/retention.

| Resource | Proposed hard ceiling and policy |
| --- | --- |
| Database page size / main file | 4,096 bytes / 262,144 pages = 1 GiB; verify `page_size` and `max_page_count` on every connection; a larger existing file is unavailable |
| Summary rows/material | Existing 2 MiB per row/material, 4,096 rows, 256 MiB aggregate serialized rows; unchanged |
| Pending lineage/checkpoints | 64 MiB total canonical payload bytes and 4,096 pending transitions per workspace; preserve at least the preceding material for every unavailable initialized subject. New rows/changes must reserve proof capacity before source publication |
| One durable command | 16 MiB total changed payload and 4,096 changed logical rows, including newly written immutable output bytes, canonical metadata, receipts, edges, fences and lineage; reject before commit if all required effects cannot fit |
| Worker admission | One active command, at most eight waiting commands and 32 MiB total queued payload; apply source rates first. Queue pressure is a bounded failure, not a new accepted event |
| SQL work | One transaction per command, no nested savepoints, triggers, SQL-supplied functions or automatic VACUUM; at most 100,000 visited logical rows per command, using qualified query/application accounting rather than treating SQLite VM instructions as rows |
| SQL cancellation deadline | 2 seconds of monotonic interruptible SQL work per command is the proposed cooperative cancellation threshold, shared across its statements rather than reset per statement. Qualify progress-check interval, maximum observed overshoot and abort/recovery handling before claiming support; it is not a two-second return/rollback guarantee. Blocked OS I/O remains separately unqualified |
| Busy/recovery | 100 ms busy timeout; one retained worker with no replacement threads on timeout. A client deadline cannot prove a transaction aborted; retain the pending fence until outcome is reconciled |
| SQLite rollback journal | 2 GiB hard file ceiling, enforced by the qualified storage/VFS adapter on writes, including recovery; `journal_size_limit=0` is cleanup policy only, not an active-log limit |
| SQLite scratch/staging | No application staging files or disk-backed SQL temporary store; `temp_store=MEMORY`, at most 8 MiB statement scratch under the qualified adapter. Reject work that cannot stay within that budget |

The command cap includes source payload bytes when newly written, not just the
canonical manifest that references them. For example, two new 8 MiB path
snapshots already consume 16 MiB before nonempty result/receipt/dependency/summary
metadata; that finalization must fail command admission under these defaults,
even though it fits #744's source-only cap. Existing immutable artifact references
still count their changed metadata and all source occurrence quotas; a physical
reuse is not a source-capacity discount. No unreviewed external payload store,
post-commit write or split command may bypass atomic publication or byte checks.
Payload placement/native publication and a complete worst-case profile budget
need explicit producer/storage qualification; a lower approved output contract
or separately reviewed storage-budget change may be needed. These ceilings do
not promise simultaneous use of every source maximum. See the
[joint #744 review](2026-10-04-assignment-evaluation-design.md#joint-technical-review-handoff--2026-10-24).

Total storage consists of the 1 GiB main file, at most 2 GiB hot journal and
bounded directory/SQLite metadata. A qualified adapter must enforce the journal
and scratch ceilings, not infer them from input size or free-space sampling.
If the existing SQLite binding/VFS cannot do so safely, this adapter is no-go;
do not build a custom VFS implicitly or relax the bounds. Scope and review any
needed adapter work in its own approved plan. No platform is claimed eligible.
The 64 MiB lineage budget includes predecessor/checkpoint material and pending
before/after bytes; it is not 64 MiB per subject or an increase to source quotas.
If durable unavailable protection/proof cannot fit, reject that source change.

Use DELETE journal mode, `synchronous=EXTRA`, foreign keys on, and verified
application/schema identity. On macOS request/verify `fullfsync=ON` and qualify
the actual VFS flush behavior. Read back effective settings; unsupported values
fail closed. SQLite's [journal-size pragma](https://www.sqlite.org/pragma.html#pragma_journal_size_limit)
limits leftover journals, not their active peak. Its
[page-count limit](https://www.sqlite.org/pragma.html#pragma_max_page_count)
bounds the main file; neither setting proves a total storage or memory bound.
There is no WAL checkpoint schedule or application segment rotation. DELETE
journal recovery belongs to SQLite; never unlink a hot journal or reset an epoch
to make startup succeed. Application checkpoint compaction follows the atomic
lineage rule above. Reject unreviewed schema upgrades instead of migrating data.

### Limit feasibility review — 2026-10-24

Review base: `ff4c9f0338eaf1224cfe86027e466174730785f8`. Static manifest/lock
inspection finds `rusqlite 0.32.1` and `libsqlite3-sys 0.30.1`; the sidecar
manifest explicitly selects `bundled`. Those identities are not a native SQLite
build/feature/OS/VFS qualification packet. No dependency feature was changed,
database opened, native fault fixture run or platform qualified for this review.

| Proposed bound | Available mechanism / remaining proof |
| --- | --- |
| 1 GiB main file | Stock `max_page_count` with verified 4,096-byte page size, current page count and effective readback; an oversized existing file cannot be made eligible by requesting a lower maximum |
| 2 GiB active rollback journal | No `journal_size_limit` peak cap; requires qualified write/recovery enforcement. Without that evidence the adapter stays no-go |
| 8 MiB statement scratch | `temp_store=MEMORY` routes eligible temporary work but does not enforce this aggregate memory limit. A qualified scoped mechanism is still required |
| 100,000 visited logical rows | No generic SQLite runtime row-visit limit. Prove query/application-specific counting for every admitted statement and recovery path; returned rows or VM instruction callbacks are not a substitute |
| Two-second SQL cancellation threshold | Progress/interrupt controls provide cooperative cancellation opportunities, not proof of prompt return or transaction outcome. Effective API/features, interval/overshoot, preparation/step/commit coverage and abort reconciliation need qualification |

These distinctions follow SQLite's [page/journal/temp PRAGMAs](https://www.sqlite.org/pragma.html),
[runtime-limit categories](https://www.sqlite.org/c3ref/c_limit_attached.html),
[progress callback](https://www.sqlite.org/c3ref/progress_handler.html) and
[interrupt](https://www.sqlite.org/c3ref/interrupt.html) contracts. In particular,
progress callbacks use approximate VM instruction intervals. The wrapper's
[version-pinned feature list](https://docs.rs/crate/rusqlite/0.32.1/features)
separates optional hooks/limits from the declared bundled profile; selecting an
API/feature/FFI route belongs to a separately reviewed implementation choice,
not evidence supplied by the current manifest.

SQLite's [temporary-file inventory](https://www.sqlite.org/tempfiles.html)
includes statement journals and other auxiliary file classes. Qualify every
file class and its write/recovery path; do not infer absence of disk spill from
`temp_store` alone. Unknown/unbounded auxiliary files keep the adapter no-go,
so the main-plus-journal total above is a required proved bound, not an observed
fact. No new external payload store or implicit custom VFS is authorized.

The [heap-limit API](https://www.sqlite.org/c3ref/hard_heap_limit64.html) scopes
limits across connections in the process. Do not silently use process-global
allocator/heap/configuration changes as a per-hierarchy scratch limiter: ordinary
SQLite users such as the Codex reader must retain their behavior. Any needed
scoped adapter or broader process-level design requires separate review/approval.

## Recovery and qualification

Open under the workspace lease; verify path ownership, schema/epoch/settings,
then let SQLite recover its hot journal before exposing records. Validate store
integrity, canonical object digests, exact head/receipt/dependency joins, quota
counters and checkpoint/pending continuity. Startup validation is bounded by
the main-file ceiling and runs in the single worker; reads stay unavailable
until it finishes. Ordinary session controls remain usable. A corrupt or missing
previously initialized store is unavailable, not an empty successful run.

New store creation is explicit UI preparation only, with a new random storage
epoch and no import/rebinding of old tracked subjects or grants. Missing-store
reads never create a database. A new epoch invalidates old storage references;
it does not approve a run or adopt history. Initialization must durably publish
the database and its directory entry before admitting its first subject.
Restoring/replacing an initialized store requires a separately reviewed recovery
procedure; direct byte-for-byte rollback remains a same-user limitation.

If commit returns an ambiguous I/O error, fence affected hierarchy reads and
mutations, reopen/recover on the same worker, and resolve the source receipt and
commit identity before retry. Never rerun a command merely because the client
saw an error. A committed change whose response/event was lost returns its
original receipt. A replacement sidecar has a fresh volatile snapshot scope;
reconcile durable rows and external sources, restoring no bearer or launch grant.

The [qualification register's crash oracles](../../validation/hierarchy-writer-storage-qualification.md#storage-and-crash-evidence-packet)
apply, including each SQLite journal write/sync/delete, DB commit, publication
fence clearing, lineage compaction, purge and restart boundary. Add full-file,
logical-quota, VFS-write-limit, SQL interrupt and stuck-I/O fixtures. Kill-only
results do not qualify power-loss durability. Record exact bundled SQLite/VFS,
OS/filesystem/mount settings and retained disk evidence. Broken joins or incomplete
lineage must yield unavailable even when SQLite's structural integrity check passes.

## Acceptance and handoff

The owner accepted the SQLite/private-worker DELETE/EXTRA direction for further
design and qualification only. Detailed module responsibilities, ordinary-writer
integration and finite limits, including capacity rejection and adapter no-go
conditions, retain their review/acceptance gates. Merging this document or ADR
does not expand the recorded acceptance. Reconcile producer contracts before a
scoped implementation and qualification plan; a complete caller inventory, concrete strict storage
schema/digest fixtures and adapter feasibility evidence are prerequisites for
that plan. #740 eligibility, #741/#742 review, #744/#746 projection agreement,
shared custody and runtime-plan approvals remain independent gates. Keep the
issues open; no runtime availability follows from this proposal.

## Architecture owner-review handoff — 2026-10-24

**Disposition: SQLite/private-worker DELETE/EXTRA architecture direction accepted
for further design/qualification only; detailed choices/defaults and no-go gates
remain open. This is not an executable implementation handoff.** The design
uses one selected-workspace SQLite durable unit/private worker, typed
producer commands, DELETE/EXTRA journaling, FIFO durable publication, independent
ordinary display generations and proof-preserving checkpoints. JSON mirrors,
a general file-transaction broker, WAL scheduling and automatic destructive
recovery remain excluded from this proposed slice.

Two bounded read-only reviews checked atomicity/recovery and limit feasibility;
the owning session checked the accepted producer contract and primary SQLite
API documentation. This review added explicit caller-cancellation/ordinary-
generation-exhaustion outcomes and the feasibility matrix above. It also names
the still-unqualified authority/commit ordering and auxiliary-file coverage;
no profile is claimed merely because SQLite provides transactions or PRAGMAs.

The owner decision must distinguish mechanism/limits from evidence: the 1 GiB
main / 2 GiB journal / 8 MiB scratch and 16 MiB complete-command limits are still
proposed, with the two-second value clarified as a cooperative cancellation
threshold. A maximum #744 payload must fit with all its metadata/lineage under
the complete command cap. The accepted #744 request lanes/receipt credits remain
independent and are not reset or guaranteed physically writable by this store.

The scoped direction acceptance permits only further design/qualification
preparation. Before runtime code, complete caller/direct-write inventory, strict
storage schemas and canonical fixtures, full source/authority/retention mapping,
payload placement/whole-command budgeting, native clock/SQL/allocator/VFS/file-
class evidence and a separately approved implementation/qualification plan.
#740 remains no-go/deferred until November; broader #741/#742 decisions,
#744/#746 producer/projection agreement and #745 shared custody retain their
gates. The review itself delivered no model/native probe, automatic adapter work
or runtime code. Keep #744/#746 open.

## Scoped architecture direction acceptance — 2026-10-24

The owner answered “go ahead” to the recommendation to accept the one-workspace
SQLite/private-worker direction with DELETE/EXTRA journaling **for further design
and qualification only**. This accepts the direction reviewed in PR #1028 at
`9aa0a417c19cdf449925565dfcec803537c46d37`, not every detailed contract choice
or numerical policy in this document. The acceptance record changes no schema,
canonical domain, authority rule, limit or qualification oracle.

The 1 GiB main / 2 GiB journal / 8 MiB scratch / 16 MiB complete-command defaults
and all other proposed numerical limits remain proposed pending enforcement proof
and separate acceptance. A complete command must account for payloads plus all
metadata; source maxima do not guarantee an eligible publication. Retain the
existing no-go conditions rather than infer a working enforcement mechanism.

Next preparation is the bounded caller/direct-write inventory and strict
schema/publication-budget specification needed to propose a qualification plan.
This acceptance executes no probe, qualifies no adapter, approves no executable
plan, and authorizes no runtime implementation or migration. Broader component
decisions, #745/#746 custody/producer agreement and #740's November capability
gate remain separate. Keep the component issues open.


## Detailed storage preparation — 2026-10-24

The [caller/schema/publication-budget pass](2026-10-24-hierarchy-storage-contract-design.md)
proposes closed storage metadata/commit commitments, inline payload placement
and whole-command accounting against a source-traced baseline. It identifies
missing domain registry entries, worst-case profile proof and a concrete
revocation-between-precheck-and-SQL-commit race. These remain no-go conditions;
the pass is neither an accepted detailed contract nor an executable handoff.
The scoped direction acceptance and all numerical/default/native/runtime gates
above remain unchanged.
