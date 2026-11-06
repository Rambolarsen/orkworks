---
type: Design Proposal
title: Pair-local SQLite namespace and pre-operation reservations
description: Proposed isolated SQLite instance, public VFS admission and fixed SQL-memory arena for the original Windows linked pair; no implementation or native execution is authorized.
tags: [orkworks, hierarchy, sqlite, windows, resources, proposal]
status: draft
workflow_status: owner-decision-required
---

# Pair-local SQLite namespace and pre-operation reservations

Tracks [#1468](https://github.com/Rambolarsen/orkworks/issues/1468), a specification
prerequisite under [#1176](https://github.com/Rambolarsen/orkworks/issues/1176) and
#738. Inspection base: `19b6a2eb`. This is a proposed replacement candidate for
further qualification, not an accepted backend, execution plan or implementation.

## Problem and unchanged authority

The [namespace/resource audit](../../validation/windows-pair-namespace-resource-audit.md)
identifies operations that stock win32 performs before any pair-budget check:
auxiliary creation/growth/deletion, statement-journal spill and recovery allocation.
Main-handle equality and after-operation samples cannot provide that admission.

Reuse the accepted [original PairSlot/PairCore owner](2026-11-06-original-pair-owner-design.md#exact-lifetime-root-and-completenew-handoff)
and [joint resource contract](2026-11-05-joint-child-preparation-design.md#worker-storage-recovery-and-bounds).
The same ordinary worker receives the original core before native creation and
keeps the same connection through CompleteNew and Serving. There is no new
process, replacement worker, movable connection or caller-provided SQL/path/VFS.
The original source/newness/lease/claim/plan policies and all lower admissions
remain binding. In particular:

- Hierarchy: 4,096-byte pages and 1 GiB main-file ceiling. No new ordinary
  physical-file quota is introduced.
- Auxiliary files: one shared **2 GiB** total across both participants, journals,
  super-journal, statement/temp/spill representations and retained recovery work.
- SQL scratch: a **separate 8 MiB** ceiling. Neither ceiling is multiplied by file,
  attachment, attempt, workspace switch or recovery phase.
- Ordinary/hierarchy intent, queue, source/discovery, retained copy, artifact and
  completion limits remain independent; satisfying native admission cannot
  enlarge them or establish maximum-profile fit.
- StoppedOccupied and Unknown retain their original owner and position. Process
  loss does not authorize initialization, adoption, cleanup or a fresh epoch.

## Alternatives and selected proposal

| Candidate | Disposition |
| --- | --- |
| Unchanged stock backend plus a complete closed-program worst-case bound | Remains a valid alternative, but the audit supplies no complete bound for the coupled ordinary workload, spill, generated-name deletion and recovered inputs. This proposal does not assert impossibility |
| Public VFS plus thread-local allocator attribution in the application's existing SQLite instance | Not selected: allocator configuration is global to that instance; attribution would additionally need to close shared/global allocations, startup ordering and cross-thread frees without changing unrelated connections |
| Dedicated child SQL process | Not selected: introduces a new IPC/process-loss and owner-transfer protocol, rather than preserving the accepted same-worker original core |
| **Private SQLite instance, public VFS admission and a fixed allocator arena** | Proposed: isolate the engine's mutable allocator/VFS state from unrelated ordinary stores while retaining the accepted worker/core lifetime |

The proposed instance uses the **same pinned SQLite 3.46.0 amalgamation** as the
audit, compiled into a private symbol namespace. It is not a new database format
or an alternative storage engine. It is a different build/backend candidate and
must not inherit the passing stock-win32 component's qualification.

This explicitly proposes an exception to the accepted **no custom VFS/shims**
selection, limited to this private Windows linked-pair profile. The default
rusqlite/libsqlite3-sys instance and unlinked ordinary stores keep their current
allocator, VFS and policy. No global allocator hook or 8 MiB process-wide heap
limit is installed into that existing instance. No private SQLite layout cast,
WIN32_SET_HANDLE substitution or borrowed-handle close is permitted.

Costs are material: a second pinned SQLite build, a small private public-C-API
adapter instead of passing its objects to rusqlite, a VFS wrapper and an arena
allocator require maintenance and independent native qualification. Accepting
this proposal selects that cost for investigation; it does not prove feasibility.

## Private module and original lifetime

One private `PairEngine` module is retained inside the original PairCore. Its
interface accepts only owner-created admitted commands and original/recovery
associations from the accepted source owner. It exposes disposition/status and
bounded results, never a raw connection, VFS selector, native handle, SQL string,
path authority or resource-release token. SQL/text/blob values and allocated
objects may not cross between the two SQLite instances.

Before the first private-engine initialization or native open, the same worker
reserves the complete engine/control/guard inventory under the existing owner
budgets. It installs that instance's public allocator and VFS configuration while
no private-instance API is concurrently in use. The existing global single
active-or-retiring hierarchy position prevents a second PairEngine while the
first is retained. Initialization failure retains any admitted/native side effect
under the original state machine.

All private SQLite initialization, connection calls, statement finalization,
shutdown and wrapper callbacks belong to this owner. No helper SQL threads,
shared cache, extension loading, external page-cache buffer, mmap, WAL, VACUUM,
additional attachment or alternate VFS path is allowed. Their absence must be
proved in the pinned build and closed command program, not inferred from a flag.
Global symbols, allocator state and SQLite pointers must be mechanically shown
not to alias the application's default SQLite instance.

## Pre-operation resource state

PairCore retains a bounded native inventory. Each auxiliary entry binds the exact
owner/command, class, generated canonical name, creation generation, native
identity when obtained, all owning/SQLite handles, state and charged upper extent.
No suffix or successful existence check is custody proof. A failed open without
a returned handle still retains its namespace claim, reserved extent and outcome.

The following are proposed **additional restrictive limits**, not new allowances
or replacements for an existing lower bound:

| Admission | Proposed limit / charge |
| --- | --- |
| Simultaneously retained auxiliary entries, including failed/closing/delete-pending entries | 16 per PairCore; both main journals, super-journal and every temporary/spill class share this count |
| Retained naming-directory components | 64 across the union of the two role paths and private temporary namespace; shared components count once |
| Canonical native path, including terminator | 4,096 UTF-16 code units; deny a longer linked profile rather than truncate or change ordinary path behavior |
| VFS callbacks with native I/O/namespace effects or reads per original admitted command/recovery operation | 65,536, counted before delegation; re-entry and retries keep the same counter |
| Sum of requested VFS read/write bytes per that operation | 2 GiB, counting repeats and both main/auxiliary files; this is a work cap separate from the lifetime auxiliary-space ceiling |

Each entry, filename, callback buffer and allocator bookkeeping byte is reserved
before allocation from the 8 MiB SQL arena below; the frozen source plan and
existing queue/result copies still obey their own lower budgets. Native guard
counts are bounded by the component/auxiliary limits plus the fixed two main
roles and qualified fixed engine handles. Every underlying handle must be in
that inventory; internal duplicate/native handles need a measured conservative
charge before the associated open. An unclosed hidden-handle path denies the
candidate; the count is not a promise that every build fits.

Every native-effect/read callback belongs to the original admitted operation
frame, including initialization, error rollback, close and recovery. A phase
change or repeated callback cannot mint a fresh work counter. Pure freeing of a
valid owned block remains possible and never restores spent I/O-work credits.
The underlying VFS object's public `szOsFile` storage is owner-resident in the
arena, independent of SQLite's outer wrapper allocation. A failed/ambiguous close
cannot free that retained native state merely because SQLite releases its outer
object; no private structure interpretation or copied native ownership is used.

Admission uses checked arithmetic. For each auxiliary write/growing truncate,
compute a conservative new upper extent from the maximum old retained extent,
checked end offset and **qualified native/preallocation rounding**, then reserve
the increase against the single 2 GiB total before delegation. A build/volume
whose rounding cannot be bounded is unsupported. Charge all simultaneous
representations during spill/copy and any old extent retained by an unsuccessful
shrink. Main-file operations also consume work/scratch admission; hierarchy keeps
its separate 1 GiB ceiling, and ordinary keeps its existing physical policy.

A failed or ambiguous native operation retains the full admitted upper extent.
Only a completed, identity-bound size/absence observation under the original
owner can reduce it, after all relevant operations/handles have a known outcome.
Returning an error, dropping a callback or reaching a deadline does not release
charges or determine the SQL old/new disposition. Saturation denies before the
next effect; no eviction, fallback, counter reset or automatic retry is supplied.

## Public VFS enforcement matrix

The [public VFS interface](https://www.sqlite.org/c3ref/vfs.html) supplies open,
access and delete callbacks and object-class flags. The
[public file methods](https://www.sqlite.org/c3ref/io_methods.html) supply read,
write, truncate, close, sync and file-control operations. Those interfaces locate
the proposed checks; their existence is not proof of Windows ownership/durability.

| Class / operation | Proposed pre-effect rule and retained outcome |
| --- | --- |
| Original ordinary/hierarchy mains | Preserve original CREATE_NEW native ownership followed by existing-file-only SQLite opens and public WIN32_GET_HANDLE / FILE_ID_INFO comparison for both roles. Never infer main-file newness from an auxiliary admission |
| Main rollback journals | Claim the exact role-derived name and reserve control/extent before native creation. Create a new file atomically through the owner; retain its creation handle/identity; ask the underlying stock win32 VFS to open that existing file without CREATE. Existing journals require this same core's exact retained custody; no OPEN_ALWAYS adoption |
| Super-journal and collision paths | Admit every generated candidate inside the guarded pair namespace. Unknown/colliding files are never adopted or deleted. A SQLite collision-deletion request without this core's exact custody fails; it cannot be justified by a suffix or repeated collisions |
| NULL-name temporary / statement-journal spill | The wrapper selects an owner-created private pair name and reserves a slot, bytes and memory before creating it. No system temporary directory or implicit extra allowance. In-memory and spilled copies remain charged together until exact release |
| Reads / recovery | Reserve the destination and requested range/work before reading. Exact existing auxiliary identities, sizes, class and authority must already be admitted; file contents/names cannot grant recovery authority. Whole-super-journal allocation must fit the same 8 MiB arena or fail before allocation |
| Writes / truncate | Reserve the conservative upper extent and work first. Forward only a qualified no-hidden-growth operation. Chunk/size hints and other indirect growth paths cannot bypass these callbacks |
| Delete / delete-on-close | Require exact owned auxiliary identity and a qualified handle-based deletion path. Strip delegated DELETEONCLOSE and own that lifecycle explicitly. Close the SQLite-owned handle through its public xClose first when required; retain the original auxiliary guard/charge until actual disposition is known. Never close a borrowed SQLite handle directly |
| File controls / alternate paths | Allow only a closed audited set, including the required public handle query. Reject/return the documented unsupported result for unrecognized controls; never forward a hint that can mutate extents, replace a handle or change VFS/profile. No alternate VFS, read-only fallback, extension or extra ATTACH |
| WAL/shared memory/mmap | Reject the unsupported class/path before native allocation or mapping; the wrapper supplies no new WAL/mmap capability |

The wrapper delegates qualified normal locking/read/write/sync behavior to the
pinned win32 implementation and implements only ownership-specific native
creation/deletion through documented Windows APIs. It may use public structure
fields and methods, never cast a `sqlite3_file` to a private `winFile`. A failed
xOpen follows SQLite's documented pMethods/xClose lifecycle while retaining any
owner-side creation independently of a returned SQLite object.

Directory guards must cover actual naming components, not only canonical string
comparison. Newly created auxiliary guards must reconcile legal SQLite deletion
with sharing modes; reusing main-file no-delete guards would be incorrect.
Exact component walking, reparse refusal, same-volume checks, deletion identity,
stock-VFS hidden operations and native rounding remain qualification targets.
This preserves the accepted legitimate non-adoption/replacement goal; it does
not add a privileged or arbitrary same-user hostile-process sandbox guarantee.

## Fixed SQL-memory admission

The public [sqlite3_mem_methods](https://www.sqlite.org/c3ref/mem_methods.html)
interface configures dynamic SQL memory for a whole SQLite library copy, not
one connection. Its [configuration](https://www.sqlite.org/c3ref/config.html) must
occur before that library copy initializes. That is why this proposal isolates
the private library copy,
rather than attributing allocations by the current request's thread tag.

Reserve one fixed backing arena whose **total is at most 8 MiB**, including
allocator headers, alignment/rounding, internal free lists, private-engine dynamic
heap, page/statement/sub-journal memory and all new VFS control/path/work buffers.
The arena's management storage is inside that same bound; it cannot fall back to
the process allocator, an external page cache, TLS attribution or another arena.
A native build audit must identify any allocation bypass, static work buffer or
extra wrapper scratch and charge it under the same ceiling or deny qualification.
Executable code and OS bookkeeping are not presented as SQL scratch or as free
resources: their fixed/native ownership costs still need the existing worker/
handle admission proof. No total-process-RSS ceiling is claimed.

xRoundup/xSize must report checked admitted capacities. A moving realloc reserves
old and new storage simultaneously before copying; failure leaves the old block
owned. Free returns capacity only after the allocation's actual last use.
Fragmentation can cause earlier NOMEM; no compaction or over-allocation is hidden.
The stricter whole-instance heap cap can reject work that would fit an abstract
scratch-only estimate. This is an explicit proposed restriction, not evidence of
maximum-profile fit or permission to enlarge the arena.

The same core retains the arena and callback state until all private SQL objects,
allocations, VFS/native operations and callbacks are known finished. Shutdown or
zero live allocation count alone does not authorize PairSlot release: the
accepted Serving retirement, StoppedOccupied and Unknown rules still apply.

## Recovery and compatibility

In-process rollback/recovery uses the same core, connection, admitted inventory,
arena and monotone operation counters. A recovery read does not reset an exhausted
command counter. Failed original creation remains StoppedOccupied after proven
logical old state, with original resources retained until process shutdown.
Unknown retains them until an accepted complete disposition; no new worker/owner
or cleanup route is introduced.

After process death this design provides **no recovery-to-live constructor**.
Volatile native identities/inventory are not a durable pair locator. A separate
qualified data-only recovery owner would need exact accepted source/pair/auxiliary
custody inputs and the same pre-operation limits before opening anything. Missing
inputs mean unavailable; it may not discover/adopt files by suffix or replay a
new initializer. This proposal selects no durable auxiliary manifest, new source
family, epoch reset, newness marker or maintenance authority. Existing restart/
durability/source-cutoff gates are still open, rather than implicitly satisfied
by a VFS wrapper.

The default engine and published installers are unchanged by this document.
A private instance must not open old/unqualified pairs, perform migration, or be
mixed with default-engine objects. Enabling its constructor requires a separately
accepted build/profile identity, complete source/catalog/initializer and ordinary
activation contract, actual native evidence and an approved executable plan.
The no-VFS candidate remains the accepted selection until the owner approves a
specific amendment; the earlier stock component result remains scoped to it.

## Owner decision and later finite handoff

The proposed decision is one scoped package **for further qualification only**:

1. Permit a private, symbol-isolated build of the pinned SQLite instance in the
   same original worker/core; use no process-wide changes to the default engine.
2. Replace the no-custom-VFS selection for this candidate with the described
   public-interface wrapper and identity-qualified auxiliary creation/deletion.
3. Enforce the existing separate 8 MiB SQL ceiling with the fixed whole-instance
   arena and no fallback; retain the shared 2 GiB auxiliary total unchanged.
4. Adopt the additional restrictive 16-entry / 64-component / 4,096-code-unit /
   65,536-callback / 2 GiB per-operation work caps described above.

These choices are proposals, including the numerical defaults. Drafting,
reviewing or merging this document does not accept them. No dependent VFS,
allocator/build, fixture database, profile flag, constructor or native/model
probe is authorized. All previous dispatch/native/model budgets remain exhausted.

After acceptance, a concise implementation/qualification plan must identify exact
public-API owners and bounded tests before code or execution. Its finite oracles
must cover every matrix row: cross-instance pointer/config isolation; allocator
failure and realloc peaks; denied open without hidden effects and failed-open
retention; exact main/aux identity, foreign collision non-deletion and temporary
namespace; allowed delete-on-close; rounded growth/shared-cap saturation and
monotone retry/work counters; super-journal and spill/recovery paths; cancellation
and owner retention; shutdown/callback lifetime; complete old/new paired recovery
and source cutoff. The new wrapper requires its own versioned native evidence;
none is inherited from the prior one-process stock-handle component attempt.

## Bounded review and disposition

One bounded read-only lower-cost review round checked correctness against the
accepted owner/resource contracts and separately asked a builder what decisions
were still missing. Neither found a blocking architectural contradiction or an
uncovered audit class. The allocator wording was tightened to identify a private
**library copy**, not a per-connection allocator. Original operation-frame and
owner-resident delegate-storage lifetimes were also made explicit; these retain
the same PairCore and budgets rather than introduce a second owner.

The builder identified owner acceptance of the exact four-part package above as
the only remaining planning decision. Review does not supply that acceptance.
Build/symbol isolation, allocation bypasses, Windows handle/deletion mechanics,
namespace guards/rounding, full source admission, native crash/restart and
durability remain explicitly unverified. No reviewer or author ran SQL, opened a
database, dispatched a workflow, or performed a native/model probe for this design.
