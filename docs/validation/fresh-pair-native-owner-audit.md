---
type: Qualification Audit
title: Actual fresh-pair native ownership and disposition seams
description: Static audit of existing file, lease and worker constructors against the accepted fresh schema2/format6 pair, without treating ordinary or test mechanics as qualified pair ownership.
tags: [orkworks, hierarchy, initialization, native, ownership, qualification]
status: draft
workflow_status: native-owner-protocol-required
---

# Actual fresh-pair native ownership and disposition seams

Tracking: [#1447](https://github.com/Rambolarsen/orkworks/issues/1447),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Inspection base:
`32011f5340328f36fbad7667cc8e06fccde7449e`.

## Outcome

[#1446](https://github.com/Rambolarsen/orkworks/pull/1446) already records acceptance
of the [fresh-pair preparation contract](../superpowers/specs/2026-11-05-fresh-pair-preparation-design.md#scoped-contract-acceptance--2026-11-06).
Its thirteen-resource seed, root/run schedule, first-parent command and supported
original preparation path need no new approval record. This audit follows the
required actual-owner qualification rather than revisiting those decisions.

Located ordinary code provides a native workspace-directory handle, advisory
lease, single retained SQL worker and queued/executing cancellation distinction.
It does **not** provide an accepted fresh-pair constructor or complete paired
native disposition. The next concrete missing deliverable is the owner protocol
joining original file reservation/identities, the broker's SQLite opens and
original pair publication/recovery. There is no qualified initializer to execute
merely by combining the existing initialize methods.

This is a bounded static call-site audit, not an impossibility proof or a bug
claim against the existing ordinary schema1 adapter. No code, SQL/DDL/database,
recovery scan, test fixture, native/model probe or runtime activation ran. No
new codec, limit, policy, native mechanism or executable plan is selected here.

## Implemented constructors versus the accepted pair

| Actual source and operation | What the source supplies | What it does not establish for the accepted pair |
| --- | --- | --- |
| [WorkspaceIdentity::resolve/revalidate](../../crates/orkworksd/src/workspace_runtime.rs) | Canonical requested workspace path, retained directory File and native Unix device/inode or Windows volume/file-index. revalidate compares retained identity and a fresh resolution of the requested path | Identities of both database files or auxiliary files, fresh-file history, pair catalog/epochs, original initialization disposition or source authority. Workspace-directory currentness is not database-file identity |
| [WorkspaceLease::acquire/Drop](../../crates/orkworksd/src/workspace_runtime.rs) | Creates/opens the metadata-directory lock file and retains its exclusive advisory lock until Drop unlocks it. The file's mere existence is explicitly not the ownership signal | A database constructor, native pair reservation, atomic creation/publication, auxiliary-file ceiling or source/revocation-at-commit interlock. A lease cannot turn missing peer/history into freshness |
| [Database::create](../../crates/orkworksd/src/session_store.rs) | Explicit one-path create_new, then SQLITE_OPEN_READ_WRITE by path; configures a single connection and commits ordinary schema1/header/application_id/user_version with a UUID epoch and G=0 | A retained original database File identity across the two opens, both-file reservation, schema2 lineage, hierarchy attachment or paired original outcome. The temporary creation File is not stored in Database; the returned value stores a Connection and attention owners |
| [Database::open](../../crates/orkworksd/src/session_store.rs) | Opens without a create flag; checks ordinary application/user/schema version1, workspace, epoch/generation and quick_check, then configures that ordinary connection | Newness proof or schema2/format6 admission. It does not open/adopt a missing hierarchy peer or validate complete paired membership. Rejection of schema2 is intentional current-source refusal, not an implemented lineage profile |
| [SessionStore::initialize/open/start/launch](../../crates/orkworksd/src/session_store/worker.rs) | One named thread resolves root/session-store/v1.sqlite, creates or opens the one ordinary Database, signals startup readiness, runs its command loop and drops its connection before finished=true | A returned owner retaining both database/native directory identities and the original fresh creation plan. launch receives path/workspace/create, not the accepted pair reservation/epochs or an admitted hierarchy connection |
| [Hierarchy Connection::open](../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs) | Refuses unqualified profile before I/O | No production fresh format6 constructor, typed initializer or paired return value is behind the fence |
| [hierarchy module boundary](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mod.rs) | Keeps incomplete catalog/journal mechanics behind cfg(test) | Test connection/schema routines cannot establish the selected production pair or be promoted into its source owner |

The create_new temporary and later SQLite open are distinct calls at the ordinary
call site. This audit does not claim an observed replacement race or an exploit.
It identifies the absence of a retained native file identity/comparison in that
constructor, which the accepted pair independently requires. No existing-path
adoption, automatic cleanup/recreation or assumption of hostile-path protection
is introduced to bridge that gap.

## Execution, cancellation and retained resources

The [worker](../../crates/orkworksd/src/session_store/worker.rs) has concrete patterns
worth reusing without relabeling their authority:

- Ticket/MutationCancellation compete on the queued-to-executing state. A dropped
  request after execution begins deliberately does not assert rollback.
- ExecutionRequest::grant claims execution under the ordinary application's short
  currentness guards; ExecutionGrant::execute sends Run after those guards are
  released. This ordinary grant is not the hierarchy preparation permit or a
  source physical-commit proof.
- RuntimeWriteResources can retain a per-session lane, optional Arc workspace
  lease and organization reservation through execution/result delivery. They do
  not contain two database file identities or a fresh-pair creation plan.
- SessionStore::close marks the queue closed and waits for the finished watch;
  SessionStore Drop marks it closed without awaiting it. run_worker drains its
  queued jobs before exiting once both queues are empty and closed. Startup
  ownership guards close an unowned startup worker. These are worker-lifetime
  paths, not cancellation of native I/O or proof that a pair rolled back.
- Ordinary startup reports Result readiness for its single Database. Its error
  type and close completion have no typed pair-specific proven-old/proven-new/
  unknown disposition. An Err, timeout or missing receiver cannot be converted
  into either accepted paired outcome without the qualified original owner.

The accepted single hierarchy active-or-retiring position, retained old workspace
resources and independent stop/input rules must be added by their actual owning
integration. A separately retained ordinary lane/lease does not itself prevent
another hierarchy worker or establish a source cutoff.

## Configuration is not native resource qualification

[Database::configure](../../crates/orkworksd/src/session_store.rs) sets and reads
DELETE journal mode, EXTRA synchronous mode and foreign_keys, with a 100 ms busy
timeout. The ordinary worker bounds admitted intent/result bytes and queues and
uses a two-second admission/execution-grant deadline. These are implemented
ordinary settings/control paths, not evidence of a bounded multi-file transaction,
filesystem flush or an upper bound on I/O already in progress.

The separate [test mechanical schema](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/schema.rs)
sets page/max-page and other connection settings for its incomplete single-store
fixture. That module is test-only; its settings do not qualify the selected
ordinary+hierarchy journals, super-journal, native scratch or pair recovery.
The ordinary max_page_count/progress-handler occurrences found by this inspection
are in tests, not a pair-wide enforcement path.

No live SQL or filesystem crash experiment was run. In particular, this audit
claims no measured 1 GiB hierarchy file, shared 2 GiB auxiliary or 8 MiB SQL scratch
guarantee, no maximum-width fit and no cancellation-before-physical-commit result.
All those accepted resource/authority requirements retain their existing status.

## Original-owner protocol still required

These are exact obligations of the already named private
[session_store::joint_preparation broker](../superpowers/specs/2026-11-05-joint-child-preparation-design.md#owners-and-the-permitted-connection),
not newly selected fields/types, a caller factory or an implementation plan.

| Phase | Required evidence before dependent execution |
| --- | --- |
| Reserve | Original authorized fresh-pair operation, selected workspace/lease and both original native file/directory identities with qualified fresh ordinary/hierarchy epochs. Both genuinely new files are required by #1441; absence of one current path/header is insufficient |
| Bind connection | Show how the ordinary worker's actual SQLite opens and fixed hierarchy attachment remain bound to those exact original native identities, including reopen/path replacement/failure. Borrowed typed participants cannot commit independently; caller paths/SQL/ATTACH handles are not accepted |
| Initialize | One immutable complete catalog/seed/physical-correspondence plan and one original paired engine transaction; ordinary schema2 G=0 and hierarchy format6 C1/thirteen workspace keys. No parent/source/session/runtime or activation escapes before the qualified original result |
| Denied/rollback | Distinguish pre-mutation denial, proven complete old logical pair and file-creation side effects. SQL rollback is not asserted to erase newly created filesystem objects. No epoch reuse, silent delete/recreate, substitute owner or implicit retry |
| Commit/unknown | Prove complete original new pair before publication; retain the original worker/lease/native handles and plan on unknown outcome. A lost response yields only exact original lookup under current authority. A new process recovers data, not volatile authority or a new creation attempt |
| Reload/retirement | Verify both epochs/catalogs/projections and required sparse origins under the same retained pair owner, and qualify auxiliary ownership/resources. Immediate stop stays independent; a new workspace cannot replace unresolved hierarchy work with another worker |

The interface and native reservation/open/disposition mechanism must be concrete
before its constructors can be implemented or any executable/native qualification
plan claims to exercise that owner. A current-source Result from the schema1
adapter or pure source validator is not an interim admitted pair witness. Existing
new-store/schema/domain/seed/policy approvals are not reopened by this obligation.

Actual parent/UI/grant/allocator and earlier source-command cutoff qualification
remain separate prerequisites to the later original-child path; native file
ownership alone cannot supply them. This audit narrows the immediate task to the
fresh-pair owner protocol, rather than another full catalog/approval exercise.
#1176 stays open; #740 stays closed research.

## Reproducible inspection and limits

The cited functions and module boundaries were read at the pinned base. Bounded
searches used these source-only paths (no runtime stores or foreign worktrees):

```bash
rtk proxy rg -n 'create_new|open_with_flags|fn create|fn open|fn configure' crates/orkworksd/src/session_store.rs
rtk proxy rg -n 'Database::create|Database::open|fn launch|RuntimeWriteResources|ExecutionRequest|impl Drop|fn close' crates/orkworksd/src/session_store/worker.rs
rtk proxy rg -n 'fn resolve|fn revalidate|fn acquire|native_directory_identity|try_lock_exclusive' crates/orkworksd/src/workspace_runtime.rs
rtk proxy rg -n 'max_page_count|progress_handler|journal_mode|synchronous|busy_timeout' crates/orkworksd/src/session_store.rs crates/orkworksd/src/session_store/worker.rs crates/orkworksd/src/taskmaster/hierarchy_storage
```

Results establish the stated behavior of located functions, not absence of every
possible differently named, external or future mechanism. No test was run to
infer runtime/native guarantees. Documentation checks validate this report's
metadata/links, not the native claims its required owner must eventually prove.


## Concrete protocol proposal — 2026-11-06

[#1450](https://github.com/Rambolarsen/orkworks/issues/1450) provides the
[original pair-owner protocol proposal](../superpowers/specs/2026-11-06-original-pair-owner-design.md).
It chooses a retained ownership/state-transfer design and a documented Windows/
NTFS handle-binding candidate, explicitly leaving Unix binding and actual native/
resource/recovery qualification unavailable. These are new owner-pending choices,
not evidence that this audit's missing constructor is implemented or qualified.
No code, native probe, database creation or runtime activation follows.
