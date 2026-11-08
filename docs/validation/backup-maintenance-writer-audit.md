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


## Owner-interface investigation — 2026-11-08

Tracking: [#1544](https://github.com/Rambolarsen/orkworks/issues/1544).
This supplementary investigation pins source at
`1ab168d35a2cfd8689d82f984b52a0fad278f765`; the original audit and its line
references above remain pinned to `4e5215db`. It executes step 1's **written
investigation** in the [qualification plan](../superpowers/plans/2026-11-08-backup-maintenance-qualification.md),
under the now [accepted maintenance design](../superpowers/specs/2026-11-08-backup-maintenance-admission-design.md).
The following are actual local interfaces, not proposed maintenance APIs.
Names describe their source capability; none is a complete Modules/Q producer.
Source files linked below use this supplementary pin for interpretation.

### Existing handles and unavailable owner operations

| Writer family / source | Actual admission, retained owner and completion | Readiness and unavailable maintenance operation |
| --- | --- | --- |
| Workspace open and lifecycle — [application](../../crates/orkworksd/src/session_application.rs), [lease](../../crates/orkworksd/src/workspace_runtime.rs), [organization](../../crates/orkworksd/src/session_archive.rs) | Application open resolves/revalidates root identity and retains a `WorkspaceLease`; create/resume use current application/runtime checks. `OrganizationStore::reserve(self: &Arc<Self>, id: &str) -> Option<Reservation>` serializes the particular ID; dropping the reservation releases it. | Ordinary open may create origin/directories or migrate legacy files. No inert all-owner readiness result or workspace-wide lifecycle admission closure exists here. Lease validation covers cooperating owners, not external process termination. |
| Private SQL worker — [worker](../../crates/orkworksd/src/session_store/worker.rs) | `prepare_internal_owned` returns `PreparedInternalMutation`; `enqueue(self, RuntimeWriteResources)` returns an `InternalMutationTicket` and execution-request receiver. `ExecutionRequest::grant` yields `ExecutionGrant`; `execute` releases the worker to run. `InternalMutationTicket::finish` returns `InternalCompletion` retaining permit/resources. `prepare_organization` has analogous ticket/completion ownership. | `close(&self)` awaits worker completion, not outer application/file jobs. `open`/`initialize` are real constructors, not maintenance integrity profiles. No all-module registration, inert closed-image seal or complete reopen/readiness operation follows from them. |
| SQL callbacks/publication — [sql_writes](../../crates/orkworksd/src/session_application/sql_writes.rs), [lifecycle](../../crates/orkworksd/src/session_application/sql_writes/lifecycle.rs) | `SessionWrites::bind` captures root/workspace/store/lease identity; `peon` and `lifecycle` retain work across caller loss. `RuntimeWriteResources` owns the per-ID lane, optional lease and organization reservation. `Outcome` distinguishes no-op, committed and committed-detached publication. Blocking publication retains the completion resources. | No maintenance owner can currently enumerate/await all these outer jobs or reserve their finite continuation slots. A committed-detached result is durable truth, not rollback or complete live-module readiness. |
| SQL external deletion — [deletion](../../crates/orkworksd/src/session_application/sql_writes/deletion.rs), [capacity](../../crates/orkworksd/src/session_application/sql_writes/file_budget.rs) | `capture_deletion` returns `CapturedDeletion` with original ID/creation/deletion identity, lane context, lease-bound cleanup owners and file reservation. Consuming stage methods `remove_replay`, `remove_workflow_evidence`, `remove_images`, `clear_settings`, `remove_recommendations` return `DeletionCleanupResult { files, progress, detached }`. | Result fields deliberately separate physical removal and optional SQL progress. No shared physical-settlement registry or maintenance continuation authority exists. File accounting retains original switched-away jobs but supplies no backup allowance. |
| Replay/settings/image cleanup — [replay](../../crates/orkworksd/src/metadata/replay_cleanup.rs), [settings](../../crates/orkworksd/src/metadata/workspace_memory/sql_cleanup.rs), [images](../../crates/orkworksd/src/image_context/storage/sql_cleanup.rs) | `ReplayCleanup::bind` yields `AnchoredReplayCleanup`; its `remove(&self)` acts on captured anchored paths. `SettingsCleanup::bind` yields `BoundSettingsCleanup`; `clear(&self, id)` returns `(io::Result<()>, Option<Edit>)`. `bind_sql_cleanup` yields `SqlImageCleanup`; `remove` returns file result plus optional `ImageCleanupGuard`. Settings/image guards remain available through progress recording. | These are exact cleanup owners, not authorization for another namespace. Platform restrictions remain real (settings/replay Unix; image implementation macOS/Linux); absent support cannot produce an empty successful profile. None supplies a complete maintenance readiness result. |
| Legacy metadata and mutating reads — [transactions](../../crates/orkworksd/src/metadata/session_transactions.rs), [reader](../../crates/orkworksd/src/metadata/session_reader.rs), [metadata](../../crates/orkworksd/src/metadata.rs) | Per-session transaction mutex/revision protects mutations; `SessionReader::read` can quarantine corrupt metadata under that ownership. Event append is separate I/O. Completion of an individual synchronous write/read is local to that call. | No all-session stop-admission/drain or inert-read interface is supplied. Quarantine and event writes must be admitted as effects; an empty session listing cannot stand in for namespace registration. |
| Terminal replay/size/finalization — [runtime](../../crates/orkworksd/src/runtime/session_runtime.rs), [application](../../crates/orkworksd/src/session_application.rs) | Persistence receiver awaits blocking application append; driver ending drops the final sender, awaits `persist_writer`, then awaits blocking trim. Application append/trim use the workspace mutex; terminal transitions own size changes. | The driver-local join is not an exported maintenance acknowledgement over every producer. No new stop/resume action is authorized. Ending/UI detach does not close other admitted effects or prove readiness. |
| Peon/provider callbacks — [Peon](../../crates/orkworksd/src/runtime/peon_runtime.rs) | Attempt generation/currentness plus cooperative cancellation restrict callbacks. Shutdown joins a `JoinSet`, then can abort async inference after its timeout; blocking post-processing has its own retained work/currentness checks. Production `peon_loop` supplies a pending shutdown future. | The test shutdown receiver is unavailable as a production maintenance handle. Async abort is not blocking-work settlement. No production pause-scheduling plus all-callback acknowledgement/readiness interface is established. |
| Taskmaster scheduling/delivery — [scheduler](../../crates/orkworksd/src/main.rs), [evaluator](../../crates/orkworksd/src/taskmaster/evaluator.rs), [application](../../crates/orkworksd/src/session_application.rs) | Periodic evaluation, analysis lease/in-flight state and retained blocking `Flight` constrain individual analysis work; delivery captures workspace/lease and tracks in-flight deliveries. | No shared closure over periodic/manual scheduling, analysis and every delivery exists. Dropping `Flight` clears its local flag; it does not certify all writers drained. Global scope must be classified separately. |
| Recommendation graph/journal — [operation](../../crates/orkworksd/src/taskmaster/store/operation.rs), [journal](../../crates/orkworksd/src/taskmaster/store/cleanup/removal/journal.rs) | `Owner::claim` / `claim_sql` yield an `Operation`; Drop clears BUSY. `BoundCleanup::claim_sql_cleanup` yields `SqlCleanupClaim`; `admit` consumes exact cleanup admission and `SqlCleanup::remove` yields `JournalReceipt` retaining operation/binding. `JournalReceipt::result` exposes the physical result. | `wait_until_available` observes availability without preventing the next claim. SQL mode is monotonic while the process-local weakly registered owner lives; it is not persisted restart authority. No maintenance drain or inert journal recovery capability is implied. |
| SQL recommendation reopen — [opening](../../crates/orkworksd/src/session_application/sql_writes/opening.rs), [worker](../../crates/orkworksd/src/session_store/worker.rs), [journal](../../crates/orkworksd/src/taskmaster/store/cleanup/removal/journal.rs) | `SqlRecommendationWorkspace::open(state, root, workspace, lease)` owns an opening task and returns a candidate retaining `Arc<SessionStore>`, `RecommendationStore`, lease and identity. `SessionStore::open_leased` keeps the startup lease in the worker; `opening_identity` checks worker identity. `claim_sql_open` validates the binding; private `RecommendationOpenAdmission` gates `admit_open`. | This new path preserves recommendation ownership across reopen but explicitly does not adopt the candidate into `AppState`. It is neither production format routing nor all-module maintenance release/readiness. See the concrete recipe below. |
| Workflow observations — [captured](../../crates/orkworksd/src/workflow_observations/captured.rs) | `capture_writer(id, created)` returns `CapturedWriter` with revocable creation validity. `record_batch(&self, &FileClaim, generation, candidates)` checks validity and admits the claim under the store lock. `FileClaim::cancel`/`expire` compete only while pending; recorded outcomes are returned per item. | A per-creation claim is not a closed workspace writer set. The local deadline/batch cap is not a backup resource profile. No root-wide admission closure, all-rewrite acknowledgement or maintenance readiness operation exists. |
| Observation trim/retention — [retention](../../crates/orkworksd/src/runtime/retention.rs) | Segment trim plans revalidate identity/protection before each effect. Retention uses current application/session/organization guards; the periodic loop separately invokes observation spam cleanup. | No common scheduling pause covers every trim/cleanup job. Each admitted rewrite/deletion and later callback needs its own retained physical completion, including switched-away roots. |
| Startup GC — [GC](../../crates/orkworksd/src/runtime/workspace_gc.rs) | A candidate lease is held through synchronous `remove_dir_all`; unavailable/protected roots are kept. | Cooperating lease exclusion is local evidence, not arbitrary-writer exclusion. No maintenance preservation-set registration or global producer acknowledgement is supplied. |
| Workspace settings/images — [memory](../../crates/orkworksd/src/metadata/workspace_memory.rs), [storage](../../crates/orkworksd/src/image_context/storage.rs) | Workspace-memory `Edit` owns atomic edit admission, with read/write methods and release on Drop. Image bundle publication/cleanup/recovery uses the private bundle lock; publication ownership persists until accept/drop semantics settle. | These owners serialize their own effects, not future uploads/settings changes across the root. Recovery/open is not inert. App-global stores need explicit exclusion/classification, even where their consumers can trigger workspace effects. |
| Branch association/capacity — [branches](../../crates/orkworksd/src/branch_documents_store.rs), [projection](../../crates/orkworksd/src/session_projection.rs) | `report(&self, &Identity, Document)` and `reconcile(&self, &Scan)` use per-identity mutex/atomic writes. Capacity projection changes inspected runtime/provider state under existing guards. | Selected-detail/read-derived routes can write associations. No root-wide admission/drain is supplied. The inspected in-memory capacity path does not establish a separate durable capacity writer; old files still require actual classification. |
| Authenticated ingress, hooks and descendants — [HTTP](../../crates/orkworksd/src/http/session_metadata_handlers.rs), [helper](https://github.com/Rambolarsen/orkworks/blob/1ab168d35a2cfd8689d82f984b52a0fad278f765/crates/orkworksd/scripts/session-metadata.py), [spool](../../crates/orkworksd/scripts/report-harness-event.sh) | HTTP admission checks live identity/token/workspace. Current helper negotiates HTTP and fails closed. Hook spool separately publishes transient files. Individual HTTP success or spool publication only acknowledges that operation. | No shared maintenance rejection spans every relay/consumer or installed external writer. Source cannot establish installed versions, external quiescence or OS descendant termination; excluded transient bytes still have producers. Required native/external owner proof remains unavailable. |

### Concrete original-job recipes

**SQL mutation and publication.** Preparation reserves command capacity and
captures cancellation/deadline; the owning async task retains the original lane
and resources across waiter loss. Enqueue yields one execution request and one
completion ticket. Grant admission occurs under short application currentness
guards; `ExecutionGrant::execute` runs after those guards are released. The
worker delivers actual completion; blocking publication retains
`InternalCompletion` until publication has actually returned, including detached
publication. `MutationCancellation::cancel() -> bool` can win before admission;
false does not mean rollback. Neither dropping the waiter nor finishing the SQL
queue acknowledges the outer publication. Required unavailable operation:
application-owned outer-job registration/settlement, with its finite execution
and publication continuations attached to the original root.

**File removal followed by SQL progress.** `CapturedDeletion` keeps the original
session creation, SQL deletion identity and captured cleanup owners. A consuming
stage call starts owned blocking work, retains file capacity, acquires the
original runtime lane and rereads the exact deletion record. Current closed
context, organization reservation, lease and recommendation-delivery exclusion
are checked before stage admission. File effects run outside application guards;
only successful removal attempts the corresponding SQL progress update. Settings,
image and recommendation ownership is retained through that progress attempt.
The returned file result, optional progress result and detached flag must all
survive caller loss. SQL close before this continuation can strand progress after
real file effects; it cannot convert the job into successful settlement. The
stage methods are real consuming local operations, but they do **not** establish
a maintenance registry's finite, at-most-once child slots or cross-job accounting.
Required unavailable operation: original-job settlement ownership that reserves
and permits only its already-admitted file/progress continuations while Closing,
records partial/unknown outcomes and keeps recovery/protection obligations.

**Recommendation reopen.** The owned opening task captures current application
reader/recommendation/workspace identities, resolves and revalidates the original
root/lease and binds cleanup recovery. `open_leased` opens the worker under that
lease; opening identity is checked against the expected identity. The owner
revalidates root/binding, then under short application guards checks unchanged
context, live waiter and no in-flight recommendation delivery. `admit_open`
consumes private opening admission and revokes legacy recommendation authority
without performing journal cleanup. Errors after worker open await `close`;
failed delivery of the successful candidate also awaits `close`. Worker teardown
drops its database and retained startup lease before publishing finished. This
is concrete owned local reopen evidence, including waiter-loss cleanup. It does
not perform production workspace adoption, format selection, restored-root fence
discovery or all-module readiness. Those release-owner operations remain
unavailable to maintenance; no proposed signature is a substitute.

### State and coverage disposition

| Accepted maintenance phase | Local evidence and exact missing integration |
| --- | --- |
| Open to Closing | Existing claims check their own currentness/cancellation. No atomic application maintenance transition rejects all ordinary jobs/schedulers and registers every captured root/module, including empty namespaces and global/external producers. |
| Closing to Held | Local tickets, joins and cleanup results have real but different completion boundaries. No application owner retains and acknowledges the complete outer-job set and finite original continuations before SQL close. All waits must stay outside projection/workspace/session/module/registry guards. |
| Held / live Q | Worker close can close the connection; it cannot issue complete Modules/revisions, native/root-access proof, integrity/auxiliary inventory or Q. No complete physical/resource profile exists here. |
| First effect / Q retirement | No current cross-owner atomic operation checks capture settlement/backups, retires Q and advances the maintenance serial before effect admission. Existing local claims cannot replace this transition or its cancellation race ordering. |
| Releasing to Open | Owned SQL recommendation reopen is available locally. Production adoption, original-store/module readiness, restore-fence discovery and explicit coordinated release are not supplied by that candidate. |
| Blocked / startup unverified | No maintained lost-owner resolution operation reconstructs complete physical settlement/readiness after volatile state loss. Lease availability, path/digest equality and stale metadata cannot resolve it. New/restarted roots must remain unverified under the accepted design until their real owners prove readiness. |

**Investigation result:** the local interfaces and missing owner operations are
identified; actual producer qualification remains **Investigate**. There is no
closed registration mechanism over every writer/scope, no complete finite-job
settlement integration and no all-owner release/readiness result. Empty namespaces,
mutating reads, callbacks, switched-away roots and unsupported direct writers
remain required coverage, not successful empty modules. This source-only result
supplies no runtime receipt, numerical allowance or permission to execute SQL,
native/model probes, stop sessions, capture/restore or implement a controller.

Fresh source-fidelity and independent coverage/authority reviews found no material
findings in this supplementary investigation. They checked the actual pinned
interfaces and the limited written-inventory completion; neither review executed
runtime operations or qualified the remaining owner/profile inputs.


## Constructor, ingress and completion coverage — 2026-11-08

Tracking: [#1556](https://github.com/Rambolarsen/orkworks/issues/1556).
Source pin: `f5013bb189bfc6bb3155ed7b2cacc447e3069813`.
This follows the now [accepted registration/settlement design](../superpowers/specs/2026-11-08-maintenance-registration-settlement-design.md)
and records concrete integration inputs without changing that design. Earlier
sections keep their own historical pins. Paths and line references in this
section refer to this new pin. Source was read only; no product test, runtime,
SQL/native/model probe, installed-state inspection or live recovery was executed.

**Finding:** registration cannot be added only at `WorkspaceState` installation,
HTTP dispatch or `SessionApplication::new`. Setup already mutates before the
workspace exchange, metadata HTTP has a direct store path, and background/global
producers are started independently. Original-root admission must precede each
physical dispatch/effect, retain the exact domain context, and cover the local
completion owner until physical return. The following mapping identifies
necessary routes; it is not an executable or qualified closed catalog.

### Constructor and root binding

| Exact constructor/source | Observed ordering and required integration input |
| --- | --- |
| [main.rs](../../crates/orkworksd/src/main.rs), 273–327 | Production `AppState` starts with `workspace: None`, then spawns Peon, periodic Taskmaster evaluation, retention and one-shot GC independently. A registration owner must exist before producers can dispatch root-affecting jobs; an active-workspace constructor alone cannot account for startup GC/global work. Current startup supplies no such owner. |
| [session_handlers.rs](../../crates/orkworksd/src/http/session_handlers.rs), 236–295 | `/workspace` resolves `WorkspaceIdentity`, compares the requested identity and calls `open_workspace_with_identity`. After successful open it can await integration reconciliation. HTTP completion therefore spans more than installing the workspace. HTTP dispatch itself supplies no retained root/job claim. |
| [session_application.rs](../../crates/orkworksd/src/session_application.rs), 3514–3602 | `open_workspace_inner` holds projection, resolves the selected repository path and global metadata directory, acquires/reuses the global `WorkspaceLease`, creates metadata directories/origin, constructs or reopens metadata ownership, acquires memory edit and runs migration. Registration at the later `workspace.replace` misses these effects. The repository identity and metadata-root lease are distinct captured inputs; neither proves ownership of a retained local adoption source or restore destination. |
| [metadata.rs](../../crates/orkworksd/src/metadata.rs), 1327–1350 | `MetadataStore::new` constructs workspace-memory, shared session write locks/reader, branch documents, hook observations, organization and checkpoint owners. `reopen_with_shared_workspace_memory` rebuilds the other owners but shares the memory owner. Registration cannot assume all owner identities survive a same-path reopen. Required owner-incarnation/lease checks remain separate from path equality. |
| [session_application.rs](../../crates/orkworksd/src/session_application.rs), 3653–3758 | Preflight opens observations/recommendations, reads sessions and scrubs recommendation orphans before the exchange; evaluation revocation is closed, context rechecked, observations reopened and `workspace.replace` installs the owners. Orphan metadata and recommendation execution reconciliation follow. These reads/open/recovery operations can write; the exchange is not the first effect or a complete readiness acknowledgement. Retired resources are retained by the enclosing open path. |
| [opening.rs](../../crates/orkworksd/src/session_application/sql_writes/opening.rs), `SqlRecommendationWorkspace::open`; [scoped guide](https://github.com/Rambolarsen/orkworks/blob/f5013bb189bfc6bb3155ed7b2cacc447e3069813/crates/orkworksd/AGENTS.md) | The SQL candidate is privately owned and deliberately not adopted into `AppState`. Production open above constructs legacy metadata/observation/recommendation owners, not this candidate. SQL tests/local handles cannot stand in for production format selection, complete startup registration or original-store readiness. |

The accepted startup rule still requires blocked/unverified registration before
ordinary writers become available. Current open is a mutating legacy workflow,
not that qualified startup operation. A future constructor change must admit its
exact startup/recovery effects separately, preserve existing domain checks and
retained owners, and leave ordinary scheduling closed until actual readiness.
This investigation does not authorize that change or invent a provisional native
identity for a nonexistent output directory.

### Concrete ingress routes and completion owners

| Entry / exact source | Current downstream ownership and coverage obligation |
| --- | --- |
| Session create/resume/delete/forget, active session/harnesses — [session_handlers.rs](../../crates/orkworksd/src/http/session_handlers.rs), 326–442, 551–581; [application](../../crates/orkworksd/src/session_application.rs), `create_session`, `resume_session`, `delete_session`, `forget_session` | Application workflows own launch, organization and durable cleanup. Existing request return or session-state publication cannot replace original lifecycle/finalization job tracking. Scope must include setup/image publication and any delayed launch/failure callback before runtime adoption. |
| Agent metadata read/patch — [session_metadata_handlers.rs](../../crates/orkworksd/src/http/session_metadata_handlers.rs), 57–138, 210–245 | HTTP spawns blocking `perform` directly; it locks workspace then sessions, checks live token/record identity and calls `validate_agent_patch` or `apply_agent_patch`. It does not pass through `SessionApplication`. The blocking closure captures `AppState`, not an original root claim. Future capture/register/revalidate must preserve the exact selected context; gating only the HTTP method or application facade misses this route. Read/quarantine classification still belongs to the actual reader. |
| Harness identity/attention and local relay — [session_handlers.rs](../../crates/orkworksd/src/http/session_handlers.rs), `report_harness_session_with_headers`, `report_harness_session_from_local_relay`, `report_attention_route`; [identity completion](../../crates/orkworksd/src/session_application/identity_completion.rs) | Application identity merge has a separate completion/publication owner and may schedule label refresh. Relay authentication/currentness is local domain admission, not maintenance closure. Original identity publication and later refresh need finite declared completion ownership; excluded hook-spool bytes do not exclude downstream session effects. |
| Agent observation — [workflow_observation_handlers.rs](../../crates/orkworksd/src/http/workflow_observation_handlers.rs), `report_workflow_observation` | Authentication/rate/idempotency validation delegates persistence to `record_agent_workflow_observation`, then can schedule evaluation. Observation publication and new evaluation are distinct admissions; an old observation claim cannot authorize a fresh analysis after Closing. |
| Session title — [session_title_handlers.rs](../../crates/orkworksd/src/http/session_title_handlers.rs), 21–43; [titles.rs](../../crates/orkworksd/src/session_application/titles.rs), `session_title` | HTTP dispatches blocking application title mutation. The blocking owner must retain admission/completion even if the handler waiter is dropped; its current identity checks are retained rather than replaced. |
| Document content/latest/review and session listing — [branch_documents_handlers.rs](../../crates/orkworksd/src/http/branch_documents_handlers.rs), `content`, `use_latest`, `review`; [branch application](../../crates/orkworksd/src/branch_documents_application.rs), `read_document_content`, `reset_document_pin`, `request_document_review`; [projection](../../crates/orkworksd/src/session_projection.rs), 640–671 | Content/latest use blocking application work; listing prepares scans and invokes branch projection under current-owner checks. Branch association/reconciliation and metadata/read-derived mutations require coverage even on GET/list paths. HTTP method names cannot establish inertness. |
| Terminal WebSocket/output/size/ending — [router](../../crates/orkworksd/src/main.rs), `/sessions/:id/terminal`; [runtime](../../crates/orkworksd/src/runtime/session_runtime.rs), `persist_writer`; [application](../../crates/orkworksd/src/session_application.rs), `append_terminal_output_batch`, `trim_terminal_output`, `persist_terminal_size_with_snapshot`; [ending_completion.rs](../../crates/orkworksd/src/session_application/ending_completion.rs), 106–130 | Persistence drain/trim and ending's owned blocking completion are later boundaries than a socket response or live status. Ending returns separate replay/metadata/event/publication outcomes; a compatibility publication boolean is no durability promise. Maintenance still rejects active/pending runtimes without stopping them. |
| Peon/provider callbacks — [main startup](../../crates/orkworksd/src/main.rs), 301–307; [Peon runtime](../../crates/orkworksd/src/runtime/peon_runtime.rs); [application](../../crates/orkworksd/src/session_application.rs), `persist_peon_observation`, `persist_final_peon_scan`, `record_peon_workflow_observations` | Background attempts and blocking post-processing have their own currentness checks. Register actual work before physical dispatch and track completion after callback return; production scheduling has no qualified maintenance pause/drain operation. Test shutdown is not that operation. |
| Taskmaster manual/periodic/delivery — [taskmaster_handlers.rs](../../crates/orkworksd/src/http/taskmaster_handlers.rs), `analyze_taskmaster`, `assess_workflow`, recommendation action handlers; [main.rs](../../crates/orkworksd/src/main.rs), 310–314; [delivery](../../crates/orkworksd/src/session_application/recommendation_delivery.rs), `settle_recommendation_delivery` | Application graph updates, model scheduling and delivery/event settlement are separate physical work. Delivery retains original target/guard while repeatedly waiting for BUSY ownership to clear; that loop has no finite maintenance wait bound. It cannot be labelled a qualified finite continuation without closing its actual bound/unknown disposition. |
| Retention, observation spam cleanup and GC — [retention.rs](../../crates/orkworksd/src/runtime/retention.rs), `retention_cleanup_task`, `delete_session_evidence`; [GC](../../crates/orkworksd/src/runtime/workspace_gc.rs), `gc_stale_workspaces_task` | Retention separately invokes spam cleanup. Legacy `delete_session_evidence` removes observation/events/settings/images/metadata/organization and retries recommendation cleanup once; it is not the SQL deletion job protocol. Startup GC dispatches blocking root removal outside current workspace selection. All need actual original scope/completion coverage. |
| Integration mutations and inspection-derived revocation — [integration application](../../crates/orkworksd/src/harness_integration_application.rs), `mutate`, `list_workspace`, `reconcile_unreferenced`; [workspace response](../../crates/orkworksd/src/http/session_handlers.rs), 283–295 | Install/repair/uninstall and reconciliation are separate owners; even inspection/list can revoke stale prompt authority. They capture integration metadata as well as external configuration. Classify actual workspace metadata effects versus excluded repository/native/global bytes; exclusion cannot silently bypass root-affecting consequences. No repository/native backup scope is added. |
| Workspace settings/images and global producer changes — [router](../../crates/orkworksd/src/main.rs), 368–550; [application](../../crates/orkworksd/src/session_application.rs), `set_active_session`, `set_active_harnesses_at`; [image owner](../../crates/orkworksd/src/image_context/storage.rs) | Existing workspace-memory and private image locks own their files. Global provider/harness/settings routes are not automatically part of the backup, but any follow-on workspace writes/launch/publication still require actual admission. A global-settings-only exclusion is not proof of no captured-root effect. |

This is concrete routing evidence, not a claim that a router listing enumerates
all native or installed producers. Every affected constructor, direct store caller,
background callback and owner-specific child remains a required integration check.
Empty namespaces still need their actual owner. Unsupported external direct
writers, root alias/overlap, installed helper state and descendant quiescence
remain unavailable physical/profile inputs.

### Exact retained SQL completion transfers

These private source paths are integration inputs; none is production routing:

- In [sql_writes.rs](../../crates/orkworksd/src/session_application/sql_writes.rs),
  173–197 and 240–343, the owned async task takes the original lane/lease into
  `RuntimeWriteResources`, enqueues and concurrently waits for execution/currentness
  and `InternalMutationTicket::finish`. Grant execution runs after shared guards
  release. `InternalCompletion` moves into `current_async`'s blocking closure,
  whose `_completion` outlives the guarded publication. The outer task sends the
  final `Outcome` through a oneshot; lost reply delivery does not create a retained
  maintenance result record. The required integration retains the obligation and
  actual outcome across that entire chain, including detached publication and
  no-document/failed-projection paths, rather than equating ticket completion with
  outer completion.
- In [deletion.rs](../../crates/orkworksd/src/session_application/sql_writes/deletion.rs),
  196–258, `capture_deletion` captures original context, reserves file capacity and
  spawns blocking lease-bound cleanup binding. A two-second timeout stops awaiting
  the binding handle; it does not cancel its blocking work. Its binding resources
  remain with that closure until it returns. This pre-stage work also needs an
  independently retained completion obligation; tracking only `remove_*` misses it.
- In the same file, 296–423, a stage's blocking closure owns capacity, original
  lane, reservation/lease and cleanup guards through actual file removal and SQL
  progress, then computes detachment and sends the result. The result channel is
  a waiter-delivery mechanism, not an application-owned settlement ledger. Missing
  progress after real removal remains a separate recovery/protection obligation.
  Each stage awaits its own progress attempt before returning; this does not
  enforce ordering against every SQL close path. The accepted design requires
  the future outer-job coordinator to settle those continuations before close.
- [opening.rs](../../crates/orkworksd/src/session_application/sql_writes/opening.rs)
  retains an owned opening task/candidate and closes on failed delivery/error after
  worker open. Complete release still needs production adoption and all-owner
  readiness. The accepted partial-readiness rule retains already prepared/reopened
  handles while Blocked until actual safe teardown or qualified resolution.

**Disposition:** exact source entry and retained-owner inputs are documented.
Actual complete constructor/ingress registration, finite producer recipes and
application-owned outcome retention remain unavailable. The binding timeout and
BUSY delivery loop are explicit inputs to resource/deadline qualification, not
new allowances. The accepted qualification plan's remaining producer/physical/
readiness boxes stay open. Next work must close actual enforcement and finite
resource ownership before a bounded implementation handoff; no model-only
catalog, sampled zero count or mock drained response satisfies that gate.

Fresh independent coverage/authority review found no material gap in this bounded
mapping. Source-fidelity review identified wording that could imply global SQL
close ordering already exists; it now distinguishes the per-stage progress wait
from the required, unavailable outer-job/close integration. Neither review supplies
physical/native evidence or changes accepted authority.
