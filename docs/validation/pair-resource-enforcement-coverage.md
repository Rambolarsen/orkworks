---
type: Qualification Audit
title: Pair controlled file-growth and allocation enforcement coverage
description: Pinned-source pre-effect ownership map and bounded offline accounting slice under accepted file-length/fixed-arena assurance; exact native build and enforcement remain unqualified.
tags: [orkworks, hierarchy, sqlite, windows, resources, qualification]
status: draft
workflow_status: source-map-recorded-build-and-mechanism-unqualified
---

# Pair controlled file-growth and allocation enforcement coverage

Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspection base: `72c0fc4d`. This is Gate 1 evidence under the
[resource qualification plan](../superpowers/plans/2026-11-24-pair-resource-assurance-qualification.md),
not backend implementation or a numerical native E/S search.
[ADR 0118](../adr/0118-pair-resource-assurance.md) supplies the accepted hierarchy
1 GiB/shared auxiliary 2 GiB logical file-length high-water and fixed 8 MiB arena
meanings. Native host allocation, helper workspaces and stacks remain outside
those numeric guarantees. All lower/source/lifetime/authority gates remain.

## Result, source identity and evidence limits

**The controlled-route obligations and a private accounting slice are specified;
Gate 1 remains Investigate for the exact build/startup and native adapter closure.
Gate 2 is not unlocked.** No located production pair VFS/router/core implements
the map. [Hierarchy Connection::open](../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs)
still rejects the unqualified profile before I/O. The ordinary
[session-store adapter](../../crates/orkworksd/src/session_store.rs) opens through
rusqlite without the proposed pair owner; its configuration is not pair proof.

The cached libsqlite3-sys 0.30.1 `sqlite3/sqlite3.c` was read directly and its
SHA-256 recomputed as
`c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d`, SQLite 3.46.0.
All C line references below use those exact bytes, obtainable from the
[pinned crate source](https://docs.rs/crate/libsqlite3-sys/0.30.1/source/sqlite3/sqlite3.c).
The [prior callback inventory](pair-native-envelope-input-closure.md#public-callback-inventory)
remains the 16-VFS/18-file/7-allocator routing map; this document supplies concrete
pre-effect/retention behavior instead of treating a repeated count as closure.

[Cargo.toml](../../crates/orkworksd/Cargo.toml) declares rusqlite 0.32 with bundled
and hooks; [Cargo.lock](../../crates/orkworksd/Cargo.lock) pins rusqlite 0.32.1 and
libsqlite3-sys 0.30.1. The inspected lock dependency edges into this SQLite family
are orkworksd to rusqlite and rusqlite to libsqlite3-sys. Cached rusqlite manifest
features map bundled to libsqlite3-sys/bundled plus modern_sqlite, modern_sqlite
to bundled_bindings, and hooks to an empty feature list. This is source/declared
feature reconciliation, not a successful target-resolved feature graph.

One `cargo metadata --locked --offline --filter-platform x86_64-pc-windows-msvc`
read failed before output because windows_x86_64_msvc 0.48.5 was not cached. No
network fetch, build/compiler, SQL/database or target execution followed. Do not
call that failure a SQLite incompatibility, authentication error or completed
Windows build qualification.

The cached build.rs, lines40–84, can select linked rather than bundled SQLite via
LIBSQLITE3_SYS_USE_PKG_CONFIG, loadable_extension, sqlcipher or in_gecko branches.
Its bundled flags at122–140 include API armor, column metadata, DBSTAT, FTS3/5,
JSON1, load-extension support, memory management, RTREE, STAT4, SOUNDEX,
THREADSAFE=1 and URI support. Lines256–291 also accept feature-driven flags,
SQLITE_MAX_VARIABLE_NUMBER/EXPR_DEPTH/COLUMN and LIBSQLITE3_FLAGS. The cc/compiler
inputs can add further flags. None of those environment values or a compiled
artifact was inferred from Cargo.lock or printed from this session.

A qualified build must retain source/feature/compiler/target/flag provenance and
reject drift from its reviewed recipe. In particular, compiled support for
extensions, WAL, mmap, virtual tables, URI options or atomic-write optimizations
is not permission to use them in the pair's closed command set. No private SQLite
copy, global feature change or extra default from #1470 is selected here.

## Ownership before an engine callback

Names here describe accepted future owners, not existing callable objects:
PairSlot retains the unique active-or-retiring position; PairCore retains native
identity/inventory, high-water and pending operation state; private NativePairIo
is its VFS seam inside session_store::joint_preparation. An engine sqlite3_file
contains only a reference to this owner. No caller-supplied identity, SQL, path,
budget or authority flag constructs an admitted owner.

For an auxiliary object with retained charge `h`, reserve checked
`h_new = max(h, requested_end)` and checked `T - h + h_new <= 2,147,483,648`
**before** I/O. Include all simultaneous owned/pending/uncertain auxiliary objects
of both participants once by actual object identity. Apply the independent
1,073,741,824-byte hierarchy EOF check. The ordinary main remains under its
existing owner; it is not another hierarchy or auxiliary contributor.

An accepted growth reservation is retained even if I/O fails, writes partly,
shrinks, is cancelled or loses its reply. Pending native work retains the original
buffer/context/handle until completion is proved. It must not be cancelled by a
request object's destructor. Shrink never refunds. Auxiliary reclamation alone,
proved by the original owner after drained engine/native references and actual
object/name disposition, permits charge and inventory reuse. No error, absent
name, engine close or new request supplies that proof.

Pinned wrappers make this separation necessary: sqlite3OsOpenMalloc,26345–26367,
allocates the file-object block through sqlite3MallocZero and frees it on failed
open. sqlite3OsClose,26119–26124, calls xClose but ignores its result and clears
pMethods; sqlite3OsCloseFree,26370–26374, then frees the wrapper. Failed-open/close
native objects and charges therefore must survive in PairCore, not solely in the
engine-owned allocation or the caller's return code. The public
[file-method contract](https://www.sqlite.org/c3ref/io_methods.html) also allows
xClose after failed xOpen when pMethods was left non-NULL. Error cleanup must be
safe in either case and must not transfer source or reclamation authority.

## File-method pre-effect map

Pinned dispatch: sqlite3OsRead/Write/Truncate/Sync/FileSize/Lock/Unlock and file
controls,26125–26204; shared-memory/mmap dispatch,26207–26246. The API choices below
are the proposed adapter recipe to qualify, not observed Windows behavior.

| Methods | Selected behavior and pre-effect check | Original retained outcome |
| --- | --- | --- |
| xRead | ReadFile on the owned regular handle with explicit checked offset; use the engine's already charged buffer, no growing bounce buffer. Admit recovery inventory/lengths before reads. Zero-fill the unread tail on SQLITE_IOERR_SHORT_READ | No EOF refund or invented smaller size; incomplete/unknown read is not recovery authority |
| xWrite | WriteFile with an explicit non-append offset on the same owned handle; validate signed inputs/checked end and reserve the complete requested EOF increase first, including retries | Full high-water on partial/error/unknown outcome; source buffer/native state stays owned until completion |
| xTruncate | SetFileInformationByHandle(FileEndOfFileInfo), only on the original handle; growth uses the same admission, shrink keeps high-water. No FileAllocationInfo shortcut | No refund from setter status or a later size query |
| xFileSize | GetFileInformationByHandleEx(FileStandardInfo) on the original handle, validate signed EOF and association, return the real value | Unknown/mismatch denies; never clamp the reply to the capacity |
| xSync | FlushFileBuffers on the owned file for applicable sync flags, with no authority latch held | Failure/uncertainty retained; file flush alone is not paired/directory/power-loss durability proof |
| xLock, xUnlock, xCheckReservedLock | LockFileEx/UnlockFileEx and a real non-blocking reserved-lock probe implementing the pinned lock state machine on the same object; no-op success from a lease is forbidden | Core retains exact acquired ranges and partial/failed downgrade state until supported disposition; cancellation grants no unlock proof |
| xClose | Drain the engine reference, then perform only eligible original-owner native close/disposition; CloseHandle errors or uncertain completion do not release the core | Engine wrapper may disappear despite xClose error; original inventory, charges and position survive |
| xSectorSize, xDeviceCharacteristics | Supply only justified selected-device/build values and properties; no unsupported atomic/safe-append/batch-write claim to skip journal work | No EOF allocation or source-capacity inference; device-property/native compatibility still unqualified |
| xShmMap, xShmLock, xShmBarrier, xShmUnmap | WAL is excluded: no mapping or native work. Version/reachability and safe rejection/cleanup must be closed before callbacks are installed | No shared memory capacity escape; a void barrier is not a way to report a supported WAL profile |
| xFetch, xUnfetch | mmap stays zero; the safe optional fetch result is a NULL mapping with SQLITE_OK, paired with valid no-mapping unfetch semantics | No OS mapping or uncharged buffer; do not claim an image/configuration already enforces this |
| xFileControl | Closed dispatch in the next section; never delegate unknown opcodes to stock win32 | A hint's ignored return value never authorizes growth |

[WriteFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-writefile)
documents explicit OVERLAPPED offsets even with a synchronous handle. The recipe
uses synchronous handles and retained operation data; it must reject append-only
access/sentinel offsets and must not silently switch to asynchronous I/O.
[NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile)
documents synchronous options and their required SYNCHRONIZE access. Their actual
interoperability/error/cancellation behavior remains native qualification, not an
inference that all OS calls finish within a deadline. Stop/input stays independent.

[LockFileEx](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex)
permits lock ranges beyond EOF. A lock range is not an extending write and is not
charged as file length solely because its lock offset exceeds the EOF cap. The
actual SQLite lock protocol, partial upgrade/downgrade and native outcome still
need proof. [SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle)
distinguishes EOF, allocation and deletion information classes; only the intended
operation may be dispatched. No native calls ran for this map.

## File controls and callers that ignore them

sqlite3OsFileControlHint,26194–26196, discards its callback result. SIZE_HINT at
59776/61537 and CHUNK_SIZE/SIZE_HINT at104089–104090 cannot act as admission fences.
Stock winFileControl,50183–50204, can turn a hint into winTruncate when chunking
is enabled. The private adapter must not forward that branch.

| Opcode class | Closed adapter behavior |
| --- | --- |
| SIZE_HINT, CHUNK_SIZE, SIZE_LIMIT | Return SQLITE_NOTFOUND without touching native allocation/EOF. SQLite documents SIZE_HINT as an estimate; SIZE_LIMIT is an in-memory-VFS control, not a disk quota. All actual extension still passes xWrite/xTruncate |
| HAS_MOVED | Revalidate original object/root association and write the actual result, or error. databaseIsUnmoved,61223–61242, treats NOTFOUND as historically unmoved, so NOTFOUND is not identity evidence |
| WIN32_GET_HANDLE | Only the accepted private, borrowed actual main-handle query; never create/replace/adopt an object or transfer ownership. The private caller verifies full identity/role before use |
| PRAGMA | Return NOTFOUND so normal engine processing continues, within the separately closed command/authorizer policy. Returning OK would claim the VFS handled the pragma (139174–139211) and can suppress the intended setting |
| SYNC, COMMIT_PHASETWO | Optional VFS notification, NOTFOUND; pinned callers convert it to OK at63483–63485 /59256–59257. Actual xSync and original transaction disposition remain required, not replaced by the notification |
| MMAP_SIZE, PERSIST_WAL, BEGIN/COMMIT/ROLLBACK_ATOMIC_WRITE, OVERWRITE, temp-filename and other unsupported controls | No effect and NOTFOUND; refuse the higher-level operation/profile if it requires an excluded effect. Do not fabricate a property or success that makes those paths reachable |
| Other ordinary hints/diagnostic controls | NOTFOUND unless this exact reviewed implementation explicitly supplies the result and charges any returned allocation. No unbounded formatting or stock-VFS fallback |

The public [file-method API](https://www.sqlite.org/c3ref/io_methods.html) specifies
NOTFOUND for unrecognized controls. sqlite3_file_control,183839–183884, handles
FILE_POINTER, VFS_POINTER, JOURNAL_POINTER, DATA_VERSION, RESERVE_BYTES and
RESET_CACHE above xFileControl. A VFS reject list therefore cannot restrict all
connection controls: the ordinary worker's closed API/command boundary must also
exclude unsupported calls. Raw pointers stay private and confer no authority.

## VFS operation ownership map

Pinned public layout1780 onward; dispatch26252–26342.

| Slots | Selected owner/check before native work |
| --- | --- |
| xOpen | Classify exact fixed main or owned auxiliary name/role plus flags; reserve existing inventory/name/handle admissions, bind original object and forbid unknown collisions. Rooted NtCreateFile uses FILE_CREATE for genuine new auxiliaries, FILE_OPEN only for admitted existing objects; no overwrite/supersede/append, allocation hint or arbitrary URI/path. Existing main engine references use the original object, not a pathname reopen |
| xAccess | Existing-only rooted probe of the exact admitted grammar. Unknown/unowned collisions deny instead of inviting SQLite deletion; absence must be proved. Recovery child names bind the admitted inventory, not just an engine flag |
| xDelete | Only original-owned auxiliary/phase; SetFileInformationByHandle(FileDispositionInfo) with retained DELETE access. No main deletion, pathname fallback or name-only refund; retain uncertain object/name outcomes |
| xFullPathname | Bounded canonical internal name copy into caller output; reject unsupported grammar/length/overflow before native work. Naming is not source permission |
| xDlOpen, xDlError, xDlSym, xDlClose | Extension loading excluded; reject/null result, bounded caller error output and safe no-handle cleanup, no LoadLibrary or allocated error string. Compiled extension support does not change that policy |
| xRandomness | Fill only admitted caller output via the selected bounded entropy helper; do not replace the process default VFS. sqlite3_randomness,33962–34030, can use that default VFS independently; disclose its native costs |
| xSleep | Existing bounded wait/cancellation policy on the worker, no growing buffer/authority latch. Requested sleep is not a proof of native completion time |
| xCurrentTime, xCurrentTimeInt64 | Scalar time helper/output with supported error/fallback behavior;26327–26342 selects the int64 callback by version, else uses xCurrentTime. No heap formatting |
| xGetLastError | Bounded writes into caller buffer including its zero-length/null form (26324–26325); retain the real error, no new allocation or lossy success |
| xSetSystemCall, xGetSystemCall, xNextSystemCall | No syscall-hook mutation/enumeration path selected; safe NOTFOUND/NULL behavior through the public ABI, no stock/native dispatch replacement |

All callback-owned variable SQL buffers use the arena; stable original-owner
inventory/handle/request state also consumes its existing lower budget. No new
F/P/entry/handle default is imported. Every native API structure/stack/helper cost
outside the region is disclosed host cost, not secretly promised inside 8 MiB.
Unsupported ABI versions/configurations remain unavailable rather than omitting
required safe callbacks or advertising unsupported properties.

## Allocation route and lifetime map

| Route / pinned source | Admission/ownership under the accepted contract |
| --- | --- |
| sqlite3Malloc and mallocWithAlarm,30464–30545 | Router xMalloc must admit checked aligned payload/bookkeeping itself, with or without prior xRoundup. Memory-statistics and non-statistics routes differ. Failure retries use the same owner; no original-allocator fallback |
| xMalloc, xRoundup, xSize | One fixed 8,388,608-byte region includes all alignment/bookkeeping/free-list state. Checked rounding fails on overflow; xSize reports actual usable block size without charging or inventing extra capacity |
| xRealloc, xFree | Pointer provenance determines original arena/default ownership, not current TLS. Admit old/new coexistence before copy; failed growth preserves old storage; no arena/default migration. Cross-context free/size retains correct registration; unexpected escape invalidates eligibility and retains original position |
| xInit, xShutdown; sqlite3MallocInit,30374–30389 | Early router installed before any SQLite initialization, save/delegate original lifecycle for unpaired/global work; do not replace another allocator or reset live SQLite. Registry/callback storage stays alive until all allocations/references drain |
| Lookaside, setupLookaside,180565–180665; sqlite3DbMallocRawNN,30880–30926 | Internal lookaside backing uses sqlite3Malloc; supplied pBuf bypasses that allocation and must already belong to the arena or deny. Slot reuse needs no duplicate charge. Existing foreign backing/connection cannot be adopted into pair accounting; its fallback/copy paths still use the original region |
| Page cache, sqlite3PCacheBufferSetup/pcache1InitBulk/pcache1Alloc,55430–55530 | A supplied process pool can bypass xMalloc; bulk/default fallback uses sqlite3Malloc. Actual shared pool/custom PCACHE configuration must be accounted without sharing uncharged storage or deny. No successful GETMALLOC/router call proves cache provenance |
| Memory journals,106162–106271 and106352–106402 | FileChunk allocations use sqlite3_malloc and stay in the arena. Spill uses xOpen then xWrite; rejecting prohibited disk classes before native create keeps original memory on failure. No memory-plus-disk fallback pool. Only permitted rollback/super-journal disk classes reach shared EOF accounting |
| Recovery pager_delsuper,59635–59711 | Two file wrappers and content/path allocation coexist. Internal requests go through the arena; validate full recovery inventory/native lengths before sizing/reads and reject OOM/oversize. Do not assert the maximum legal journal fits memory |
| sqlite3StackAlloc macros,20644–20658 | Default routes to sqlite3DbMallocRaw; SQLITE_USE_ALLOCA changes it to stack allocation. Record the actual build choice; do not infer arena coverage or numeric stack bounds from the macro name |
| Worker/native/app callback state | Pair calls, including open/prepare/finalize/error/destructors, must enter actual original context; fixed native values are outside-region host costs, controlled variable callback storage is inside. Any retained non-SQL owner record also obeys its already applicable lower pool; no unknown growing Rust buffer is exempt |
| Default VFS/global initialization/unpaired calls | Saved original allocator and existing owners remain independent. The default PRNG path is not the private VFS. Any pair arena allocation escaping into global state invalidates qualification/retains its original registration; initializing once is not proof the default helper is never called again |

The [allocator API](https://www.sqlite.org/c3ref/mem_methods.html) describes public
methods; the pinned direct-xMalloc path above controls version-specific coverage.
No global heap-limit, forced memory-statistics switch, alternate SQLite copy,
smaller stack, custom page cache or process isolation is selected as a workaround.
Actual startup/configuration custody and complete pair-call coverage remain
unqualified; native cost disclosure cannot turn controlled allocations into OS
allocations by declaration.

## Bounded private implementation slice and remaining gates

A narrow next code candidate is the **actual retained EOF accounting owner**,
not a second independent budget model or native adapter. Proposed paths below are
under crates/orkworksd/src/, only once Gate 1 is closed and a bounded executable
slice is authorized:

- session_store/joint_preparation/resource_accounting.rs: private per-original-
  core charge state, typed main/auxiliary roles and owned growth/reclamation
  transitions. The same state must be consumed by future NativePairIo; no second
  ledger, source grant, caller-supplied budget or production constructor.
- Its local tests: before/equal/after both capacities; two participant journals
  and super-journal together; checked overflow/negative/unknown input; zero-byte
  create admission; overlapping reservations on the same object without double
  count; partial/error/unknown I/O; no shrink refund; wrong identity/generation;
  retry and no release until complete original reclamation. Assertions inspect
  retained state and a fake adapter's absence of invocation on denial.
- Only private test wiring in session_store.rs and joint_preparation/mod.rs;
  no real file/SQL/VFS/router creation or source-registration seam. The arena
  implementation follows its own reviewed layout/provenance slice; EOF tests
  cannot qualify it or supply its configuration proof.

The slice consumes original owner-established identity and existing finite entry/
handle admissions. Reserving returns a transition bound to that original entry;
only the same retained state resolves its completion/reclamation. A lost request
cannot drop the charge or issue another owner. Concrete Rust interfaces must
retain these invariants; this document does not execute or prewrite them.

Source/ABI mapping is useful, but it does not complete the following evidence:

1. Actual reviewed target/compiler/feature/flag manifest and a successful complete
   target dependency reconciliation; the offline metadata limitation above is
   unresolved. No native build is authorized to hide that limitation.
2. Actual early startup/configuration owner, external cache/lookaside provenance,
   closed pair-call/context registration and exact native/FFI error recipe. The
   identified APIs are candidates, not proof of their compatible implementation.
3. Actual original-source inventory/recovery/newness and all existing lower pool
   admissions; SQLite names/flags/lengths or a test factory do not supply them.
4. Separately authorized finite implementation/native qualification, including
   locks, ABI, partial/error/close/delete outcomes and paired durability. Existing
   native/compile budgets remain consumed; product activation stays gated.

One-time scope/coverage review: **Ready as source evidence and an identified
private slice; Investigate before executing that slice.** The reviewed route map
names pre-effect checks and retention for all existing public slots; unresolved
build/configuration/native evidence above is not hidden as a passed Gate 1.
No code, database/SQL, target build, compiler/native/model probe, allocator/VFS
installation, migration, profile eligibility or budget renewal occurred.
