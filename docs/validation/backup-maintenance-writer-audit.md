---
type: Source Qualification Review
title: Backup maintenance writer admission and drain audit
description: Pinned writer-family ownership and missing all-writer maintenance guarantees for complete backup capture and restore.
tags: [orkworks, sessions, backup, maintenance, qualification]
status: stable
---

# Backup maintenance writer admission and drain audit

Source inspected on 2026-11-08:
`4e5215db38a3bb7085dc4fa3ce4ca2f1ed4604fe`.
Tracking: [#1535](https://github.com/Rambolarsen/orkworks/issues/1535),
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
Authority: accepted [backup contract](../superpowers/specs/2026-11-07-session-backup-restore-contract.md),
[bundle/evidence v1](../superpowers/specs/2026-11-08-backup-bundle-evidence-v1.md)
and [storage requirements](../../specs/session-storage.md).
Reuse their approvals; this audit proposes no new storage policy or codec.

**Finding:** useful local ownership, cancellation and SQL-close primitives exist,
but the inspected production owners do not compose a qualified all-writer
capture/restore maintenance operation. A held workspace lease, empty session
list, acquired projection mutex, completed SQL close or stopped async task is
insufficient by itself to produce bundle v1's live Q/Modules evidence.

This is source inspection, not runtime/platform evidence. No running-session
inspection, stop, live metadata read, backup/restore, SQL/native/model probe or
product test was executed. The table covers the production writer families
below and their inspected seams; it is **not** a registered closed Modules list
or proof that arbitrary external writers have stopped. Source paths are canonical;
line references refer to the pinned revision, not necessarily later main.

## Current ownership and missing maintenance guarantees

| Writer family and pinned source | Actual source control | What remains unavailable for complete maintenance |
| --- | --- | --- |
| Workspace open/reopen, origin and legacy copying — [session_application.rs](../../crates/orkworksd/src/session_application.rs), 3514–3601; [workspace_runtime.rs](../../crates/orkworksd/src/workspace_runtime.rs), 163–218 | Open holds projection, resolves/revalidates identity and acquires/reuses the lease. It creates directories/origin, obtains workspace-memory edit ownership, then calls legacy migration. Lease uses an advisory exclusive lock; Unix validation checks the captured lock inode against the no-follow directory entry. | Open is itself a mutating operation, not inert backup validation. These guards do not pause/drain every module or prove external descendants stopped. Both roots of adoption and the separate restore destination need actual ownership, not a borrowed global-root lock. |
| Create/resume and lifecycle — [session_application.rs](../../crates/orkworksd/src/session_application.rs), 3833–3846 and 6333–6404; [session_archive.rs](../../crates/orkworksd/src/session_archive.rs), 121–134 and 239–250 | Create/resume delegate through application workflows. Resume rechecks generation, metadata ending, tracked PID, current reader and handle before installing the in-progress claim. Organization has per-session reservations and exact creation/revision checks. | Per-session admission and ending are not a workspace-wide block on new start/resume/organization work. Captured maintenance must reject new admissions before observing its writer set, and account for already admitted lifecycle/finalization effects. No automatic shutdown/resume is permitted by the audit. |
| Private SQL worker — [worker.rs](../../crates/orkworksd/src/session_store/worker.rs), 646–700, 1057–1085 and 1252–1277 | close() marks the shared queue closed, wakes the worker and waits for its finished watch. The run loop exits after both queues empty; the connection is dropped before finished=true. Drop requests closure but does not await the watch. Queue admission also checks closed. | This is a genuine connection/queue closure primitive, not a seal/integrity/auxiliary inventory or all-module drain. It does not await detached application publication/file jobs. A prepared/queued request can still settle or fail; capture must preserve the actual outcomes. SQL format selection remains gated. |
| SQL metadata/provider callbacks — [sql_writes.rs](../../crates/orkworksd/src/session_application/sql_writes.rs), 1–4, 118–170, 185–200 and 235–335 | Source-only SessionWrites binds captured workspace/lease/store identity; short current() checks use projection/workspace/session/attempt identity. Admitted work retains lane/lease/results; blocking publication retains completion resources even after waiter loss. CommittedDetached reports stored truth when live publication cannot be applied. | Neither SQL close nor dropping the caller proves every grant/publication callback has physically settled. Maintenance needs application-level admission rejection and completion ownership extending past SQL execution. Mutexes must not be held across the drain. |
| SQL external deletion jobs — [deletion.rs](../../crates/orkworksd/src/session_application/sql_writes/deletion.rs), 290–410; [file_budget.rs](../../crates/orkworksd/src/session_application/sql_writes/file_budget.rs), 1–46 | Blocking cleanup obtains the original per-session lane, exact deletion identity, current closed context, organization reservation and lease. It retains file capacity through removal and SQL progress. Recommendation admission checks in-flight delivery and transfers exact cleanup ownership. File-job accounting is application-wide, including switched-away workspaces. | A closed session or released application guard is not completed cleanup. SQL close can prevent a later progress write after file effects; that is an unresolved job, not backup success. The existing 1 MiB intent/32-job limits are deletion accounting, not backup bounds or a maintenance drain registry. |
| Legacy metadata transactions and reads — [metadata.rs](../../crates/orkworksd/src/metadata.rs), 1491–1516 and 1791–1827; [session_transactions.rs](../../crates/orkworksd/src/metadata/session_transactions.rs), 1–3; [session_reader.rs](../../crates/orkworksd/src/metadata/session_reader.rs), 16–50 | Metadata uses session-scoped mutation ownership/revisions; legacy snapshots remain unconditional but advance revision. Event append is separate external I/O. SessionReader bounds concurrent work and, on corrupt data, can recheck and rename it into quarantine under the mutation lane. | Revision conflicts and session locks do not stop all future writers. A normal read may mutate quarantine state, so it cannot be used as an inert capture validator. Event append plus metadata is not a cross-file transaction. Required stable capture must cover both. |
| Terminal replay, size and ending — [session_runtime.rs](../../crates/orkworksd/src/runtime/session_runtime.rs), 2670–2690 and 3295–3319; [session_application.rs](../../crates/orkworksd/src/session_application.rs), 2412–2438 | A persistence task consumes batches and awaits blocking append work through the application owner. Driver ending launches final-batch delivery, drops the sender, awaits persist_writer, then performs blocking trim. Application append/trim hold the workspace mutex. Terminal-size persistence remains part of terminal transitions under existing owners. | A UI detach, emitted Ended event or absence of a managed child does not establish completed replay persistence/trim. Maintenance needs the actual completion of terminal/finalization and size/event producers without starting or adopting another runtime. |
| Peon/provider scheduling and post-processing — [peon_runtime.rs](../../crates/orkworksd/src/runtime/peon_runtime.rs), 246–260, 665–690, 987–1006 and 1095–1109 | Attempts check generation/runtime identity and cooperative cancellation before callbacks. Shutdown support cancels, joins with a finite timeout and aborts remaining async inference tasks. The production peon_loop passes a pending shutdown future; test entry has an explicit shutdown receiver. Blocking post-processing may outlive JoinSet abort and rechecks current context. | A general all-writer maintenance controller cannot infer a production stop/drain handle from the test receiver. Cancellation excludes later callbacks but already admitted effects may finish. Aborting a Tokio task is not proof its blocking work is complete. |
| Taskmaster scheduling/delivery — [main.rs](../../crates/orkworksd/src/main.rs), 303–320; [evaluator.rs](../../crates/orkworksd/src/taskmaster/evaluator.rs), 70–78 and 342–406; [session_application.rs](../../crates/orkworksd/src/session_application.rs), 118–145 and 166–214 | A periodic loop schedules evaluation. The model scheduler uses an in-flight flag, analysis lease and a retained blocking task; its Flight drop clears the flag. Delivery has captured workspace/lease identity and process-local in-flight tracking. | Analysis serialization and per-delivery tracking are not a stop-scheduling-plus-drain operation over all workspace mutations. Delayed/manual evaluation and delivery must be blocked or settled, not merely absent at one sample. Global analysis/settings scope must remain explicitly classified, not silently added to workspace backup. |
| Recommendation graph/journal — [operation.rs](../../crates/orkworksd/src/taskmaster/store/operation.rs), 1–97; [journal.rs](../../crates/orkworksd/src/taskmaster/store/cleanup/removal/journal.rs), 30–48 and 88–102 | Shared process-local per-root operation owner serializes legacy/SQL work. Admitted SQL cleanup monotonically revokes later legacy claims; Drop releases BUSY, retaining SQL mode while that owner lives. The source registry keeps weak owner references. | wait_until_available observes a current non-busy state, not a pause on subsequent claims. SQL revocation is cleanup-specific; it is not all-writer maintenance, persisted proof across restart, or permission to reopen/repair journals during validation. |
| Workflow observations and trimming — [captured.rs](../../crates/orkworksd/src/workflow_observations/captured.rs), 70–130; [retention.rs](../../crates/orkworksd/src/runtime/retention.rs), 38–134 and 231–236 | Captured writers bind session creation validity and claim admission under the store mutex. Observation trim captures plans, then rechecks workspace identity/protection per segment. The periodic retention task also invokes spam cleanup. | Per-session revocation and segment locking do not stop new reports/evaluation/trim work for the entire captured root. Every admitted rewrite, sequence update and retained callback needs settlement before Q. |
| Retention and startup workspace GC — [retention.rs](../../crates/orkworksd/src/runtime/retention.rs), 269–337; [workspace_gc.rs](../../crates/orkworksd/src/runtime/workspace_gc.rs), 65–122 | Retention uses projection/workspace/session guards, protected organization state and exact deletion paths. GC acquires a candidate workspace lease and retains it through remove_dir_all; unavailable/retention-protected roots are kept. | Pausing retention alone does not pause observation spam cleanup or all other writers. A retained global lease protects against cooperating GC acquisition, but not arbitrary external mutation. Backup roots/preservation sets must stay outside ordinary cleanup; no GC execution was tested here. |
| Workspace settings and images — [workspace_memory.rs](../../crates/orkworksd/src/metadata/workspace_memory.rs), 17–65; [session_application.rs](../../crates/orkworksd/src/session_application.rs), 4992–5008 and 5035–5074; [image storage](../../crates/orkworksd/src/image_context/storage.rs), 169–176 and 263–289 | workspace.json edits use an atomic edit owner, with expected harness revision at the application entry. Image publication, cleanup and recovery share their private bundle lock. SQL settings cleanup retains its existing edit/lease owner. | One-file edit and image lock ownership do not pause the entire root, uploads or future settings writes. Cleanup/recover operations cannot be treated as inert validators. App-global settings/native stores remain excluded even when their synchronization can initiate workspace writes. |
| Branch-document association and capacity projection — [branch_documents_store.rs](../../crates/orkworksd/src/branch_documents_store.rs), 103–163; [session_application.rs](../../crates/orkworksd/src/session_application.rs), 4930–4988; [session_projection.rs](../../crates/orkworksd/src/session_projection.rs), 940–1038 | Branch records use per-identity mutexes and atomic writes, reached from selected-plan publication/event work. Capacity projection updates runtime/session/provider state under existing guards and input identity checks. | Metadata-derived/selected-detail routes cannot all be assumed read-only; association/event writes require coverage. The inspected capacity write-back is in-memory, not proof of a separate `capacity/<id>.json` writer or durable snapshot. Existing old capacity files still need actual scope classification. |
| Authenticated agents/hooks and external descendants — [metadata handler](../../crates/orkworksd/src/http/session_metadata_handlers.rs), 61–113; [helper](https://github.com/Rambolarsen/orkworks/blob/4e5215db38a3bb7085dc4fa3ce4ca2f1ed4604fe/crates/orkworksd/scripts/session-metadata.py), 40–130; [hook spool](../../crates/orkworksd/scripts/report-harness-event.sh), 355–375 | Metadata HTTP checks current live session, token, workspace and durable identity. Current helper is protocol-negotiated HTTP and fails closed; it has no direct-file fallback. A qualified Codex hook path can separately spool transient reports through temp/write/fsync/rename. | Current source does not establish installed helper versions or external producer quiescence. Authentication/current-session checks are not a global maintenance rejection state. Excluding transient mailbox bytes from a backup does not stop their producers or downstream consumers. Arbitrary/older same-user file writers remain unsupported; no inherited credential was inspected. |

## What SQL close does and does not settle

The actual worker close primitive is stronger than simply dropping a handle:
closed admission, queue exhaustion and connection destruction are ordered before
its finished signal. Preserve that useful guarantee. It does not establish the
outcomes of all application work which can surround SQL commands. In particular,
external cleanup removes files before persisting the next SQL progress bit, and
publication can retain completion resources in a detached blocking callback.
A maintenance owner must account for those jobs rather than calling close first
and declaring the entire workspace frozen.

The SQL close path and test bodies were **read, not executed** here. Existing
source tests such as close_rejects_unqueued_preparations_without_late_writes
(worker.rs 1715–1736) provide names for later scoped verification; this audit does
not turn them into a new pass, physical seal or qualified auxiliary-state result.
No queue empty/closed bit can synthesize the backup owner's integrity result.

## Mapping to the accepted maintenance contract

| Required before capture/restore | Pinned audit result |
| --- | --- |
| Reject new starts/mutations before enumerating writers | Application entry guards exist; the inspected AppState/WorkspaceState and producer paths have no shared capture/restore maintenance admission state. A complete controller is still required. |
| Pause scheduling and drain every admitted effect | SQL queue close, runtime persistence joins, provider cancellation and per-owner reservations are distinct. Their combined physical settlement is unqualified. |
| Preserve ownership while waiting | SQL/file callbacks deliberately retain lane/lease/budget outside application guards; the future controller must preserve them and wait outside projection/workspace/session mutexes. |
| Complete Modules/revisions and current Q | No closed, actual all-writer Modules registry or owner-issued backup maintenance receipt is established by these inspected seams. Empty namespaces still require their real owner under v1. |
| Explicit unclean-stop quiescence resolution | Advisory lease release or stale lifecycle cannot establish descendant termination. Existing accepted owner-resolution/native gates remain; no process adoption/cleanup is added. |
| Inert validation and durable restore fence | Normal open/read/recovery paths can write. Actual inert validators and opener-discoverable restore routing remain separate unqualified dependencies. |
| Full physical resource reservation | Existing worker/file-job limits serve their original owners; no complete capture/preservation/staging/auxiliary/scratch reservation is supplied by them. |

## Next bounded qualification seam

The next useful design is the existing application's **maintenance admission and
physical completion interface**, before any capture module can issue a live Q.
Use the table to define which owners must reject new work, expose an admitted-job
settlement acknowledgement and retain the lease/resources, including work that
has left application guards or whose async waiter has gone away. It must also
handle late HTTP/hook/provider callbacks and workspace replacement without
converting stored receipts into authority.

This is a recommendation for the next task, not an approved new controller or
implementation plan. Its concrete proposal must close lock ordering, no-wait-under-
application-guards, cancellation/owner loss, pending cleanup progress, complete
writer/namespace discovery and actual producer revisions against the accepted
backup/v1 contracts. Reuse the accepted policies and representation unchanged.
No blanket runtime stop, test-only shutdown hook, borrowed budget or mock drained
module can satisfy the missing contract.

## Evidence limits

The audit intentionally records both existing controls and their narrow scope.
It does not prove full OS writer enumeration or establish production readiness
from source comments, test names or absence of a grep match. Installed assets,
native handle/durability/access profiles, supported platform behavior, all-writer
settlement, restore-fence crash routing and complete resource fit remain Unknown.
#1242/#1243/#1248/#1250/#1244 and native/hierarchy/rollout gates remain binding.
The earlier [evidence-owner audit](legacy-adoption-evidence-owners.md) remains
pinned historical evidence; this current audit does not rewrite its source base.

## Review disposition

Fresh source-fidelity review found no material errors in the SQL close versus
detached-work distinction, production/test Peon shutdown distinction or current
HTTP-only helper claims. Fresh coverage/authority review found no material gap
in the bounded writer-family mapping or its explicit lack of closed-registry,
quiescence, native or physical qualification. Neither review executed runtime
operations or supplies a maintenance receipt.
