---
type: Design Proposal
title: Original pair native owner and SQLite handle binding
description: Accepted pair-owner and Windows handle-binding candidate for further qualification, with native execution and actual eligibility still gated.
tags: [orkworks, hierarchy, storage, native, ownership, qualification]
status: draft
workflow_status: owner-protocol-accepted-native-qualification-pending
---

# Original pair native owner and SQLite handle binding

Tracking: [#1450](https://github.com/Rambolarsen/orkworks/issues/1450),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Inspection base:
`00822d53073295c694410e2baf92850a5bc766c7`.

## Proposed decision and scope

The [actual native-owner audit](../../validation/fresh-pair-native-owner-audit.md)
finds no owner joining original file reservation, SQLite's actual files and paired
disposition. Propose that owner inside the already selected
session_store::joint_preparation module, with one transfer to the retained ordinary
worker and the existing single active-or-retiring hierarchy position.

The concrete native binding candidate is **Windows + local NTFS + the pinned
bundled SQLite win32 VFS**. This selects a candidate to qualify, not Windows linked
eligibility or a change to shipped platform support. macOS/Linux remain without a
selected native handle-binding backend here; their directory handles or a VFS
movement flag are not silently substituted. No custom VFS, private SQLite layout
cast, /dev/fd or proc-fd filename, alternate storage engine or imported pair.

Reuse the accepted [fresh initializer/original preparation contract](2026-11-05-fresh-pair-preparation-design.md#scoped-contract-acceptance--2026-11-06),
broker, indexes, RegistryRoot and source policies. These new owner states,
reservation/binding and platform choices remain proposed. No code, database/SQL/
DDL/schema/migration, probe, fixture or executable/native plan is supplied.

## Evidence and limits of the chosen interface

[Cargo.lock](../../../crates/orkworksd/Cargo.lock) pins libsqlite3-sys0.30.1;
[Cargo.toml](../../../crates/orkworksd/Cargo.toml) selects rusqlite0.32 bundled/hooks.
The local bundled amalgamation identifies SQLite3.46.0. Static inspection of that
version's win32 xFileControl returns its underlying handle for
SQLITE_FCNTL_WIN32_GET_HANDLE; its Unix branch implements HAS_MOVED as a movement
query. This is source evidence only, not a native result or a portable API claim.

SQLite documents [per-schema file control](https://www.sqlite.org/c3ref/file_control.html)
and [WIN32_GET_HANDLE](https://www.sqlite.org/c3ref/c_fcntl_begin_atomic_write.html).
Propose querying main and hierarchy separately and comparing their actual handles
with retained creation handles. FILE_POINTER returns a sqlite3_file pointer;
its [public structure](https://www.sqlite.org/c3ref/file.html) exposes methods,
not a portable OS descriptor. HAS_MOVED reports movement, not equality with a
particular retained creation handle. These observations do not prove that no
other Unix mechanism could be designed.

Microsoft's [FILE_ID_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info)
provides volume serial plus128-bit file identity for comparing handles. Use that
full tuple privately, retaining the original handles. Do not truncate it to the
existing source-directory codec or persist it as a new source identity.

[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
documents exclusive new creation and sharing rules. The candidate retains new
file handles allowing read/write sharing but omitting delete sharing, and uses
non-inherited directory handles that also omit delete sharing. Compatibility
with actual SQLite opens, namespace guards, journals and shutdown must be tested;
these docs do not establish durability or OrkWorks qualification.

## One private owner, not a caller proof

Propose a non-Clone OriginalPairOwner, the composite of the lifetime root and
worker core defined below. It is not solely owned by a request future. It retains:

- The exact selected workspace identity/lease and storage-root namespace guards,
  both original newly created main-file handles/native identities and fixed roles.
- The qualified fresh ordinary UUID epoch/hierarchy storageEpoch, actual retained
  connection generation and original complete immutable initialization plan.
- The unique hierarchy position and all admitted queue/plan/copy resources; after
  transfer, the ordinary worker's actual connection and both schema associations.
- Its exact phase/disposition and any retained native side effects. No caller
  supplies handles, SQL, paths, affected lists, identity tuples or an eligible flag.

### Exact lifetime root and CompleteNew handoff

The selected workspace/storage coordinator retains one PairSlot in the global
active-or-retiring position. PairSlot is the lifetime root: it holds the unique
position claim, worker-control/join state and exact association/phase. Requests
borrow that association and never own its release. Workspace switching retains
the old PairSlot until its supported terminal retirement; it cannot overwrite it
or start a replacement hierarchy position.

Before the first native creation, move the one non-Clone PairCore (frozen plan,
workspace lease/identity, all admitted resources) once into the ordinary worker.
That same core creates and retains all directory/file handles and the connection/
schema associations. PairSlot stays in the coordinator; it is not transferred.
The worker retains its PairCore in its run loop through initialization, Serving
or StoppedOccupied. No request future, callback or decoded result owns the core.

CompleteNew is an in-place transition of that core from Initializing to Serving,
not a move to another broker/connection. After full validation and ordered pair
reference publication, the coordinator changes the same PairSlot association to
Serving under its short state synchronization. The core retains both original
creation handles, all namespace guards, lease, connection/schema bindings, frozen
initialization plan and admitted resource charges; the PairSlot retains the one
position. The term linked broker denotes this same worker/core in serving mode.
A returned InitializedPair reference contains only owner-created association/
epoch/result information; it cannot own or release those resources or reopen
admission. A lost reply changes neither core nor slot ownership. Serving shutdown
must use the separately qualified linked-worker drain/retirement contract; this
candidate supplies no destructor shortcut or second owner.

The source/namespace authority supplies original authorization, never a boolean
freshness claim. No new serialized token, bearer, on-disk decision journal, logical
family, epoch reset, public constructor or quota is introduced. A future production
constructor stays unavailable until its actual native/authority/resource owners
are qualified; a decoded test value cannot construct one.

The private interface accepts an original owner-authorized FreshPairPlan, performs
reserve/bind/initialize under that owner, and returns only one of its sealed
complete-old/complete-new/unknown dispositions. Completion is bound to this exact
owner/plan/connection generation, not a callback that asserts success. Consumers
cannot extract a general Connection or independently commit either participant.
The result of initialize is not an ordinary activation marker or live capability.

## Native reservation and binding sequence

This is the proposed original owner protocol, not instructions to execute SQL now.
All native work and allocation occurs outside core authority locks.

1. Acquire the one hierarchy active-or-retiring position before native creation.
   Capture the selected original workspace/namespace/lease and authorized fresh
   epochs; reserve all resource admission before further allocation. Transfer the
   PairCore to the ordinary worker before step2; it performs the native binding
   work thereafter. Revalidate actual ownership at each required transition. No missing-path inference can
   adopt a previously occupied pair or burn a replacement epoch.
2. Retain native naming-directory guards from the selected owned storage root to
   the two fixed leaf locations. The Windows candidate opens/validates the actual
   directory components without accepting reparse redirection; keeps the required
   guards through binding/disposition; and denies unsupported/ambiguous native
   namespace or filesystem identity. No user path/URI/schema/VFS is accepted.
   Native component walking and delete-sharing behavior are qualification targets,
   not a claim that WorkspaceIdentity alone already implements this protection.
3. In that same worker/core, create ordinary and hierarchy files with CREATE_NEW,
   read/write access,
   read/write sharing only, and non-inherited handles. Record each handle's full
   FILE_ID_INFO and validate regular non-reparse local NTFS files on the qualified
   volume. The two identities must differ. Keep both handles open; do not close
   the first if the second creation fails. Partial creation is retained explicitly,
   never reclassified as no native effect or automatically deleted/recreated.
4. In that same PairCore/worker, open ordinary main with existing-file-only access
   and attach exactly the fixed
   existing hierarchy peer using the selected win32 VFS and rollback profile.
   The worker cannot call today's single-file Database::create or independently
   initialize a participant. Any open failure retains the original owner outcome.
5. Under exclusive connection ownership, call sqlite3_file_control for main and
   hierarchy with WIN32_GET_HANDLE; initialize outputs defensively and require
   SQLITE_OK and valid handles. Borrow those handles only while their connection/
   schema remains owned. Query FILE_ID_INFO directly on each and compare with its
   role's retained creation handle. Reject swapped/equal/foreign identities.
   Validate the selected built-in VFS/build and effective connection settings;
   unsupported controls/shims or identity queries deny, not a pathname fallback.
6. Keep the creation/directory guards throughout all work. Repeat actual handle
   equality and namespace/lease checks before the first mutation, before outcome
   classification and before publishing any new pair reference. No DETACH,
   reattach, VACUUM, connection replacement or schema reassociation is permitted
   inside this operation. Any unexpected replacement invalidates the original
   binding and leaves it unavailable pending original-owner disposition.

The small FFI bridge is isolated to the worker implementation and documented
SQLite/Windows interfaces. It never casts sqlite3_file to winFile/unixFile, owns/
closes a SQLite-borrowed handle, substitutes a handle with WIN32_SET_HANDLE, or
makes concurrent SQLite calls. After an ambiguous connection close or missing
schema it cannot reuse the borrowed handle. Native identity equality is data-file
binding, not source authority, integrity, content permission or complete recovery.

A successful handle comparison alone does not qualify the path namespace guards.
Both requirements must be satisfied. The selected protection targets legitimate
namespace replacement/non-adoption; it does not redefine OrkWorks as a sandbox
against a privileged hostile process or waive any accepted source/lease check.

## State, cancellation and exact initialization

| Owner state | Permitted transition and retained evidence |
| --- | --- |
| ReservedPosition | Original authority/resources held, no file created; cancellation may deny and release only after proving no creation began |
| ReservingFiles | Original native creation in progress; retain every observed created handle/identity and plan. Failure/timeout is not rollback or a new attempt |
| Bound | Both actual SQLite schema handles match original native files, namespace/lease/profile checks hold; still no logical initialization or capability |
| Initializing | One original complete initializer transaction; no independent participant commits, no handle/startup/activation publication |
| CompleteOld | Original owner proves no initialization or complete logical rollback and inventories native side effects. If any file was created, transition to StoppedOccupied; logical rollback does not release its slot/resources or permit a new attempt |
| StoppedOccupied | Same PairCore/worker and PairSlot retain original handles/plan/lease/resources/position after proven logical old state with creation side effects. No serving, cleanup, retirement handoff or fresh retry is supplied |
| CompleteNew/Serving | Original owner proves both exact initialized catalogs/epochs, ordinaryG="0", hierarchyC1/thirteen resources, physical correspondence and qualified durability. Publish references and update the same PairSlot/core to Serving; no resource or connection transfer |
| Unknown/Retiring | Retain original connection/worker/namespace/file/lease/plan/resources and position until supported old/new disposition. No new owner, replacement worker or reinterpretation of an Err/drop/timeout |

State names are private ownership states, not a persisted command schema or a
second application-level commit protocol. The containing owner retains the job
when a request disappears; Drop cannot release the unique position while native
work continues. Native operations finish/classify through their original worker,
with retirement/destruction outside core latches. Immediate ordinary stop/input
must not wait for native file binding, SQL or classification.

Initialization writes the already accepted complete schema2/format6 catalogs and
exact thirteen workspace ledgers under one engine transaction. Neither source nor
ordinary activation is exposed. New C links, header/proof controls, physical owner
stamps and all accounting keep their accepted equations; no new journal record
or proof family makes an otherwise unsupported outcome acceptable.

Cancellation/authority before initialization still follows its applicable strict
accepted cutoff. This proposal invents no fresh-initializer claim-first permit.
The separate original-child preparation permit covers only its exact later save.
Actual source/namespace authority at required physical acceptance remains an
independent qualification gate; the native file handle comparison cannot satisfy
it or authorize root/parent/proposal/reservation/allocator/spawn writes.

A successful COMMIT return is classified CompleteNew only with the complete
required paired-state, native binding, resource and durability evidence. An error
is CompleteOld only when the same owner proves that complete logical outcome and
its native side effects. Otherwise it is Unknown, even if both filenames exist
or one header looks initialized. Never split outcomes by ordinary versus hierarchy
or publish a healthy subset. Exact lost-response lookup identifies the original
pair/epochs/plan result under current authority and issues no new creation, C/G,
capability, refund or initialization.

## Retirement and process loss

Only a denied attempt proved to have created no file or other native side effect
may release its PairSlot after the actual worker has exited. If any file was
created, even a proven complete logical rollback enters StoppedOccupied and keeps
the same core, handles, lease, plan, resources and position until process shutdown.
This deliberately blocks new hierarchy work in that process. No post-failure
maintenance handoff, occupied-path recording or automatic retirement is supported
by this candidate. Unknown retains its original core/slot likewise; it cannot be
converted to StoppedOccupied without proving the complete logical old outcome.

Native IDs/handles and the slot are volatile, not a durable pair locator. Process
death releases them but does not authorize a new attempt. This candidate exposes
**no startup-create, restart-to-fresh, old/partial-file adoption or recovery-to-live-
owner route**. Startup remains profile-unavailable. A future actual constructor
must obtain the already-required independently qualified genuine-newness/source
reservation; pathname absence, CREATE_NEW success, empty files or new epochs cannot
mint it. This proposal neither constructs that reservation after restart nor
claims a volatile StoppedOccupied state survives process death.

A separate qualified data-only recovery owner may inspect the original data under
its accepted epoch/catalog/source/proof/membership rules, without restoring this
owner or permission to initialize. Re-enabling initialization or retiring failed
native reservations across process loss requires its own concrete durable newness/
reservation and maintenance/restart design, not an assumed existing marker. This
candidate selects no such journal/marker, reset, deletion or broader cleanup. It
therefore makes no restart eligibility claim and does not pretend to solve that
remaining gate. A proven CompleteNew already served by the retained core follows
its existing supported linked shutdown/recovery contract, not a fresh initializer.

## What this candidate does not qualify

The documented main-file handle query gives a concrete identity-binding candidate.
It does not observe or bound all journals/super-journal/scratch, establish directory
or power-loss durability, prove paired rollback after a crash, or implement source
physical-commit revocation. Do not infer those properties from NTFS, a VFS name,
SQLite settings, a successful query or a future software test.

All existing file/auxiliary/scratch/queue/command/discovery/ordinary intent limits
apply. Charge every native guard/handle, retained root/plan/copy, worker/result and
unknown lifetime to the owning admission; this design creates no additional pool.
A finite native handle/namespace-walk/resource proof and auxiliary ownership/
recovery enforcement must be supplied before qualification; no maximum fit or
fallback allowance is asserted. The Unix backend remains unselected, not emulated
by HAS_MOVED or private struct layout. macOS/Windows/Linux ordinary behavior and
published installers remain unchanged by this written candidate.

## Bounded verification and decision handoff

A later separately approved native/implementation plan must test exact original
handle binding for both roles; unsupported control and invalid handle refusal;
reparse/ancestor/file replacement; swapped/aliased/foreign files; partial creation;
SQLite reopen/attach failure; cancellation/drop at each state; independent input;
complete old/new/unknown and loss of reply; retained retirement ownership; missing
peer/process loss and exact data-only recovery; actual resource/durability bounds.
No synthetic admitted source is an interim test factory. Public API compatibility
must be verified against the bundled build on the target system before any
profile result. No such test or native probe ran here.

The new scoped design choices are the retained owner/state-transfer protocol,
Windows/NTFS public-handle binding candidate, exact PairSlot/PairCore lifetime
and fail-closed StoppedOccupied/no-restart treatment with no persistent journal. After review and owner
acceptance they support further qualification only. Actual native namespace,
restart/disposition, hard resources and strict authority mechanisms must be closed
before a coupled executable initializer/publisher plan. Existing logical, policy,
codec/seed/index/RegistryRoot decisions remain accepted. #1176 stays open;
#740 stays closed research.


## Bounded review disposition

One bounded read-only Luna correctness/builder round examined the substantive
proposal. Correctness found no material contradiction in the documented native
handle candidate, retained unknown ownership, strict source exclusions or explicit
qualification limits. The builder raised two material ownership questions:

- The post-CompleteNew lifetime root/resource transfer was ambiguous. The author
  specified PairSlot as the stable coordinator root and PairCore as the once-moved
  worker-owned state, with CompleteNew an in-place Serving transition retaining
  every native/connection/position resource; requests own none of their release.
- CompleteOld retirement relied on an unspecified occupied-path maintenance
  record. The author removed that assumption: creation side effects now enter
  StoppedOccupied, retaining the same resources/position; this candidate offers
  no failed-reservation handoff or startup/restart-to-fresh/recovery-to-live-owner
  route and selects no durable marker. Such a broader protocol remains gated.

These are explicit author-selected proposed lifetime/availability decisions,
not prior owner approvals. No second review round or post-edit reviewer approval
is claimed. Actual native namespace, resource, strict cutoff, durable newness/
restart and source qualification remains unverified; no test, probe or eligible
profile follows from the review. The existing accepted contracts are unchanged.


## Scoped owner-protocol acceptance — 2026-11-06

After [PR #1451](https://github.com/Rambolarsen/orkworks/pull/1451), head
`fe42ba2337843404c8712fa6854659910d19ce9d`, merged
`12ddf37d243f2e5a690ec8525d286123c074a62d`, the owner answered “keep going”
to approval of this Windows/NTFS candidate and ownership/failure policy for
further qualification. [#1453](https://github.com/Rambolarsen/orkworks/issues/1453)
and [ADR 0111](../../adr/0111-original-pair-native-owner.md) record that decision.

Accept the exact PairSlot/PairCore lifetime, in-place Serving transition,
documented Windows native-handle binding candidate, and StoppedOccupied/no-
restart-to-fresh policy above. This includes the explicit author resolutions
recorded in the bounded review; earlier owner-pending wording is historical for
those exact choices. All prior accepted contracts stand; no repeat design approval
is needed. The existing review record is reused without a new technical round.

This acceptance authorizes further qualification only, not candidate implementation,
native probe/test execution, SQL/database creation, an eligible profile or activation.
Unix binding, actual namespace/auxiliary/restart/durability, strict source cutoffs
and hard resource evidence remain unqualified. The [bounded component probe plan](../plans/2026-11-06-windows-pair-handle-qualification.md)
is a separately gated next step; it constructs no source, initializer or live
PairCore and supplies no full publisher proof. #1176 stays open; #740 closed.

## Scoped native enforcement replacement — 2026-11-06

The owner accepted [#1469's VFS/router amendment](2026-11-06-pair-native-enforcement-design.md#scoped-architecture-acceptance--2026-11-06)
for further qualification, recorded in [ADR 0112](../../adr/0112-pair-native-enforcement.md).
This replaces only the stock-win32/no-custom-VFS candidate selection in the
earlier accepted contract. Same-worker PairCore/PairSlot ownership, exact native
identity, source/newness/physical cutoffs, failed/unknown disposition and budgets
remain unchanged. The separate envelope, implementation, native execution,
restart/durability and activation gates are not satisfied by this acceptance.
