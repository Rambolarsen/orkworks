---
type: Qualification Audit
title: Pair controlled file-growth and allocation enforcement coverage
description: Pinned-source pre-effect ownership map and bounded offline accounting slice under accepted file-length/fixed-arena assurance; exact native build and enforcement remain unqualified.
tags: [orkworks, hierarchy, sqlite, windows, resources, qualification]
status: draft
workflow_status: metadata-observed-build-and-mechanism-unqualified
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

Current status: the [live-progress batch terminal result](live-reader-progress.md#sole-candidate-attempt-terminal-unavailable-2026-11-28)
records another source-verification timeout after twelve PE pins, one named SDK
pass and one locked fetch. The retained prefix reached archives-end (257 reported
packages), then root-begin/msvc; no complete input manifest, build, oracle or
target execution was admitted. The new allowance and every earlier allowance
are spent. Actual effective flags, linked artifact, startup and native mechanisms
remain unqualified; Gate1 is still Investigate and no executable slice is unlocked.
The [older replacement result](#replacement-build-terminal-result-2026-11-27)
and [source-only diagnosis](pair-reader-timeout-diagnosis.md) retain their
historical scope. The new prefix narrows the latest attempt's last observed phase;
it does not identify an individual runtime hot path or authorize another attempt.

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

## Shared page-cache startup finding — 2026-11-24

Further source inspection follows the unresolved cache/startup edge rather than
repeating this callback map. The bundled memory-management define selects a
unified PGroup; pcache1FetchStage2 can recycle an equal-sized page between caches
without a page allocation callback. That conditional path can supply unpaired-
origin storage to a pair even with an early router; it is not an observed overrun.
Production Codex and OpenCode read-only SQLite consumers make unpaired startup/
coexistence concrete, not merely hypothetical future ordinary storage.

The [build/startup proposal](../superpowers/specs/2026-11-24-pair-build-startup-design.md)
compares an explicit shared-build feature undefinition with leaving the candidate
unavailable or adding a new PCACHE2 integration. The recommended global flag is
owner-pending, affects unpaired memory reclamation, and is not accepted by this
source evidence or merge. Existing resource/worker/VFS choices remain accepted;
Gate 1 stays Investigate and no implementation or execution budget is renewed.

## Build/startup recipe accepted — 2026-11-24

The owner [accepted the reviewed #2043 flag/startup package](../superpowers/specs/2026-11-24-pair-build-startup-design.md#scoped-build-and-startup-acceptance--2026-11-24)
for further qualification. [ADR 0119](../adr/0119-pair-build-startup-custody.md)
records the shared candidate macro undefinition, ordinary reclamation cost and
early bootstrap/getter-default-fill/retained-failure recipe. Earlier owner-pending
wording is historical only for those choices. No actual flag/configuration/code
changed and no target/native mechanism was qualified. Existing source facts,
metadata failure, unknowns and execution budgets stay intact. Gate 1 remains
Investigate for actual artifact/startup/adapter closure; Gate 2 is not unlocked.

## Configuration-writer source audit — 2026-11-24

Inspection base: `bfacb5fa551776b12d53df575e351a25d5583c71`, following the
[bounded bootstrap-state plan](../superpowers/plans/2026-11-24-pair-bootstrap-state-tests.md).
This closes a narrower source question: which inspected layers can initialize
or change the relevant configuration, and what the absence of application calls
can actually establish. It does **not** close Gate 1 or execute that plan.

### Repository and dependency boundary

A source-only scan of all `.rs`, `.c` and `.h` files under
`crates/orkworksd/src/` found no direct sqlite3_config, sqlite3_initialize,
sqlite3_shutdown, SQLITE_CONFIG_*, config_log, auto_extension, set_db_config,
SQLITE_DBCONFIG_LOOKASIDE or enable_shared_cache occurrence. The package build.rs
only compiles the existing listener-identity C helper on macOS/aarch64; it adds
no inspected SQLite configuration writer. This is a scoped textual result, not
a whole-program or alias/indirect-call proof. Existing rusqlite opens still
initialize the library through dependency code.

The lock's named dependency edges into this family remain orkworksd → rusqlite
→ libsqlite3-sys. That can exclude another named lock consumer from this small
inventory, but cannot establish the Windows resolved features, native link map,
forced compiler includes, pre-main code or an actual artifact's writer history.
No Cargo command was rerun; the earlier uncached Windows metadata failure remains.

| Inspected source boundary | Source result | Custody consequence |
| --- | --- | --- |
| rusqlite lib.rs,57/96–97/136–138 | Reexports raw ffi; auto_extension module is available unless loadable_extension; trace module is feature-gated | Bundled/hooks does not seal the C API. An exposed registration API is not evidence that it ran |
| rusqlite trace.rs,27–52 | config_log calls global SQLITE_CONFIG_LOG; its wrapper exists only when the trace module is enabled | No inspected repo call or direct trace feature request; actual feature unification still needs reconciliation. Do not call every dependency config writer absent |
| rusqlite inner_connection.rs,420–449 | Opening checks sqlite3_threadsafe, then allocates/frees a recursive mutex to detect single-thread mode | That check can initialize SQLite before sqlite3_open_v2. It is unsuitable as a pre-install custody probe |
| Pinned sqlite3.c,22887–22945 | Static defaults include core mutex enabled, empty allocator/cache methods, NULL pPage/pHeap and compiled lookaside defaults | These are conditional initial source values, not a live public configuration read or proof no earlier writer changed them |
| sqlite3MemSetDefault,26915–26928 | The selected system allocator default installs all methods via SQLITE_CONFIG_MALLOC | Expected internal writer under the reviewed default build; selecting another allocator macro changes this path. Do not mistake it for an application foreign allocator |
| sqlite3PCacheSetDefault,56349–56366 | Installs the default method table via SQLITE_CONFIG_PCACHE2 | Internal method selection does not prove external PAGECACHE backing-pool provenance |
| sqlite3_initialize,179984–180152 | Initializes mutex/allocator/cache/OS state; may run SQLLOG and EXTRA_INIT hooks when compiled | Configuration history includes initialization and build-selected hooks, not just explicit repo sqlite3_config calls |

The same cached SQLite and build.rs SHA-256 values above were rechecked. Additional
read source identities, relative to the pinned rusqlite 0.32.1 crate:

| File | SHA-256 |
| --- | --- |
| src/lib.rs | `6649c0c96b04ded5a192de3de5adc40d3103fec186f9f056dddfae7d34a36108` |
| src/inner_connection.rs | `3c49805fe5a239c89ff512f603c6f3f514061d653e3fcb5e7b46af1887a98382` |
| src/trace.rs | `0eabb4d60c8f1a32c44fb462cca2a64c10db771a62fd33a6f803c2f5df5ad707` |

### Getter, initialization and build observations

Pinned sqlite3_config,180287–180365, separates MALLOC table replacement from
PAGECACHE pPage/szPage/nPage replacement and PCACHE2 method replacement. GETMALLOC
can install defaults; GETPCACHE2 can do likewise but cannot return the pool.
Retain the accepted omission of GETPCACHE2. NULL pPage in static initialization
cannot be promoted to live pool custody without the complete build/writer path.
The compiled lookaside slot defaults are source observations, not newly accepted
pair counts or permission to disable lookaside.

The initializer sets isInit before the optional EXTRA_INIT call at180141–180145.
An error returned by a build-supplied extra hook can therefore coexist with
initialized state. That hook is excluded by the accepted reviewed-default recipe;
its reachable source path illustrates why a non-OK initialization result must not
be interpreted as pristine state or permission to reinstall/reset. The accepted
sticky retained-failure rule already covers this; no new retry policy is added.

The inspected tree has no `.cargo/config` or `.cargo/config.toml` at repository,
`crates/` or `crates/orkworksd/` level. This does not cover ancestor/CARGO_HOME or
command-line configuration. `rust-toolchain.toml` selects floating stable;
desktop build scripts use ordinary cargo build, and the release workflow uses
windows-latest/stable. None supplies the accepted candidate undefinition or a
fixed actual compiler/linker/SDK/artifact manifest. Historical compile-evidence
scripts still describe their original memory-management define; do not edit or
reuse those artifacts as evidence of the new candidate.

### Remaining closure and next finite action

**Gate 1 remains Investigate; the offline implementation plan remains gated.**
Static inspection now distinguishes application, wrapper, engine-default and
build-hook writers. It cannot manufacture the missing target feature graph,
effective flags, linked artifact or native adapter. No reasonable additional
no-match search can establish those actual-build facts.

Next prepare a separately reviewable finite Windows dependency/build-evidence
proposal: identify the required dependency acquisition and metadata operation,
fixed toolchain/SDK/compiler inputs, exact candidate build, artifact/flag custody,
invocation/time limits and owned cleanup. State explicitly whether it is metadata
only or compiler/native execution, and obtain its applicable execution allowance
before running it. Do not reuse the spent compile/native batch or silently treat
this source audit as that allowance. The broader adapter/source/context and
ordinary-compatibility gates still require their own evidence afterward.

No dependency fetch, metadata retry, compilation, SQL/database access, SQLite API,
bootstrap code, flag change, native/model probe or budget renewal occurred here.
#1176 stays open; production hierarchy remains unavailable before I/O.

## One Windows metadata attempt completed — 2026-11-26

The owner accepted the exact [metadata-only allowance](../superpowers/plans/2026-11-25-pair-windows-metadata-evidence.md#scoped-allowance-accepted--2026-11-26).
Tooling [#2061](https://github.com/Rambolarsen/orkworks/pull/2061), reviewed head
`4b1984ddc8f1f8d6d598799a730f228133abef1a`, merged as
`d5dfcc676ee0a605da1fb073f2135095026babf2`. Hosted review identified Windows CRLF
conversion of frozen inputs; explicit LF attributes and real Git-filter
red/green tests fixed it before the sole dispatch. Current-head Codex then found
no issues; all applicable normal CI checks passed. Copilot review was unavailable
because of a monthly service quota; no extra request was made after diagnosis.

[Run 45763908423](https://github.com/Rambolarsen/orkworks/actions/runs/45763908423)
completed successfully on the frozen merged head. Its one metadata job ran from
05:47:50 to 05:48:16 UTC, within the 15-minute bound. The retained result reports
one locked target fetch and one locked/offline metadata operation, both exit0,
zero compilations and zero target executions. Read-only workflow history observed
one dispatch; original-owned cache/target scratch cleanup completed. No rerun,
alternate target/toolchain, source override, SQLite API or production route ran.
Normal repository CI builds are separate from this metadata experiment.

### Observed identities and graph

| Observation | Retained value |
| --- | --- |
| Runner | windows-2022; ImageOS win22; ImageVersion 20261116.370.1 |
| Installed Cargo | 1.100.0, commit 3acc79c0c841844675bc53a605922dbfc441e750, x86_64-pc-windows-msvc |
| Cargo binary SHA-256 | `f381bae27c59588eacf69208e115b2c001a2998afed83a2e9084ea93620737b1` |
| Installed rustc | 1.100.0, commit 0ef1bc386059596dada8fb65b50425aab2ea637a, LLVM 23.1.5, x86_64-pc-windows-msvc |
| rustc binary SHA-256 | `d4f8a0283de2dba0dc458cd13c333337d2b76c3649eb39e91c4acce618e661a1` |
| Request | Existing default features; x86_64-pc-windows-msvc resolve filter; frozen manifest/lock/toolchain/build.rs hashes in the accepted plan |
| Graph | Format1, 258 packages and 258 resolve nodes; root orkworksd 0.3.0 |
| rusqlite 0.32.1 features | bundled, hooks, modern_sqlite |
| libsqlite3-sys 0.30.1 features | bundled, bundled_bindings, cc, default, min_sqlite_version_3_14_0, pkg-config, vcpkg |
| SQLite source SHA-256 | `c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d` — expected and acquired value match |
| libsqlite3-sys build.rs SHA-256 | `11bebc3787657749279375ab2e875e52259f995dce8916b4907103975e2a76ee` — expected and acquired value match |

Independent readback of the downloaded JSON confirmed the metadata hash/byte
count, root and family incoming edges: orkworksd → rusqlite → libsqlite3-sys.
The latter's cc 1.2.64, pkg-config 0.3.33 and vcpkg 0.2.15 edges have build kind.
These resolved helpers/features do not prove a linked-library selection or actual
compiler arguments. The packages array and resolve graph are not a release
link/unit graph or evidence that any dependency's build script executed.

The actual literal fetch/metadata commands and all four frozen input hashes are
retained in result.json. No LIBSQLITE3_FLAGS change was applied. This observation
closes the earlier missing metadata result for this exact request; it does not
prove the accepted MEMORY_MANAGEMENT undefinition, THREADSAFE/core-mutex state,
external cache/lookaside provenance or a built sidecar's identity.

### Retained evidence and terminal counters

[Artifact 15090528686](https://github.com/Rambolarsen/orkworks/actions/runs/45763908423/artifacts/15090528686)
is the sole allowlisted Actions artifact (114,386 compressed bytes). It expires
2026-12-10 at05:48:12 UTC under the approved 14-day retention. The uncompressed
three-file total is 1,314,992 bytes, below 32 MiB. This canonical record preserves
observations and identities; the complete raw graph/diagnostics remain in that
approved expiring artifact, without extending its retention or persisting caches.

| File | Bytes | SHA-256 |
| --- | --- | --- |
| metadata.json | 1,301,173 | `e20b228d67729cc6ee9443857b330cd0f6d72c6c5274fbef3d749df0f03990c4` |
| result.json | 5,701 | `948bda8ed70b29c983b2611055521889e35e2dd592493c9e1afe6b10c9cdf671` |
| diagnostics.txt | 8,118 | `52edc18426d7944bdbb3cbeb713b622756f6832a9486900542ef9ac1c6b04b01` |

**Terminal: dispatch1/1, fetch1/1, metadata1/1 consumed.** Successful acquisition
supplies no retry credit or compiler/native allowance. Old batches remain spent.
Installed Rust identities are observations, not a selected fixed production
build. MSVC/linker/SDK identities, effective flags, linked artifacts and actual
startup/router/VFS/context behavior still need separately reviewed evidence.

Next finite task: propose an exact compiler-build batch using this resolved graph
and explicit compiler/linker/SDK identities and invocation limits. Obtain its
separate allowance before execution. Do not infer Gate 1 closure, bootstrap code
authority, ordinary compatibility, source permission, recovery/durability or
production activation from this successful metadata result. #1176 stays open.

## Fixed-build input audit — 2026-11-26

The owner accepted the [new fixed-build allowance](../superpowers/plans/2026-11-26-pair-fixed-windows-build.md#scoped-allowance-accepted--2026-11-26).
At this audit, dispatch stood0/1 while its separate tooling was reviewed. The earlier metadata,
compiler and native batches remain spent; no new metadata operation is selected.

The tooling policy at scripts/qualification/pair-fixed-windows-build.inputs.json
pins the four source inputs, installed Cargo/rustc/cl bytes and 37 cached registry
custom-build entrypoints from the retained graph. Acquired crate files must match
Cargo's lock/archive and per-file checksums; any new build script is rejected.
Source, Rust/toolset and selected SDK include/library/tool trees are sealed before
code generation, with acquired registry sources added after the sole fetch, and
compared again before publication. This provides input identity and drift checks,
not independently reproduced outputs or a sandbox against arbitrary native code.

The selected Windows native paths were inspected in cached locked sources:

| Helper | Selected path and exclusion |
| --- | --- |
| libgit2-sys0.18.8+1.9.7 | Existing default features; LIBGIT2_NO_PKG_CONFIG=1 makes pkg-config0.3.33 return before spawning a probe. Require packaged libgit2/src; no vendored-git feature change or submodule download. |
| libssh2-sys0.3.1 | VCPKGRS_DISABLE=1 rejects vcpkg discovery; omit LIBSSH2_SYS_USE_PKG_CONFIG. Existing Windows CNG path, packaged libssh2/src. A missing package cannot be repaired with Git. |
| libz-sys1.1.29 | Existing MSVC path skips pkg-config, vcpkg disabled, packaged source build; no zlib-ng feature selection. |
| ring0.17.14 | Require packaged pregenerated directory and no .git; no assembly regeneration with Perl/NASM. |
| cc1.2.64/find-msvc-tools0.1.9 | Explicit cl/lib/link paths, fixed VC/SDK environment and restricted PATH; no latest toolset or ambient wrappers/flags. |
| Remaining pinned build scripts | Rust cfg/version probes use the selected Rust toolchain. Non-Windows native discovery branches are outside this target. All package files, including script modules and proc-macro source, remain checksum verified. |

The minimal child environment omits inherited credentials, Cargo configuration,
CL/_CL_, CFLAGS/CXXFLAGS, wrappers and package-manager roots. Its PATH includes
only the fixed Rust/MSVC/SDK directories and Windows System32; Git, pkg-config,
vcpkg, CMake, Perl and NASM are not selected helpers. Cargo --offline alone is
not the source/helper closure argument. Missing selected inputs or unknown actual
SQLite compiler options deny the candidate without a rescue build or oracle retry.

The evidence-only macro translation unit checks the undefinition and THREADSAFE1
before and after including the same pinned amalgamation. Its argv is derived from
the actual logged cc command, rejecting response files, forced includes and unknown
options. Object/archive hashes, archive members, final map/import reports and
unsigned sidecar identity must be inspected separately; test fixtures are not
evidence that those artifacts exist. No sidecar or SQLite runtime will execute.

This audit does not close Gate1, qualify startup/allocator/VFS/context behavior or
permit bootstrap implementation. #1176 remains open. The one manual45-minute job
and its terminal evidence record were pending at that point; no dispatch or build had run.

The compile-time inventory additionally pins all14 procedural-macro entrypoints
from the retained graph. Their non-dev dependency closure comprises those14 plus
proc-macro2, quote, syn, synstructure and unicode-ident. Inspection of the cached
source modules found token parsing/generation paths rather than network/helper
launches; windows-implement's rustfmt launch belongs to its cfg(test) module,
which this build does not run. proc-macro2/quote build probes use the explicit
RUSTC. Unknown procedural-macro packages or entrypoints deny the build. This is a
source-and-configuration closure for these trusted pinned packages, not an OS
egress firewall or a claim that Cargo offline sandboxes arbitrary build code.

Link custody requires the unique Cargo compiler-artifact for libsqlite3_sys,
byte equality of its extracted SQLite member with the captured native object and
the native sqlite3.lib member, and final map rows naming that selected Rust
archive/member for SQLite symbols. The fixed lib.exe informational
[/EXTRACT operation](https://learn.microsoft.com/en-us/cpp/build/reference/extracting-a-library-member?view=msvc-170)
copies an existing member without recompilation or execution. Both SQLite
archives and the custody receipt join the same bounded artifact; temporary
extracted copies are removed. Unknown output formats leave custody unavailable.

## Fixed Windows build batch closed unavailable (2026-11-26)

The owner-accepted [fixed-build batch](../superpowers/plans/2026-11-26-pair-fixed-windows-build.md#scoped-allowance-accepted--2026-11-26)
reached its terminal outcome. Tooling [#2069](https://github.com/Rambolarsen/orkworks/pull/2069),
reviewed head `3cf93c33de083b4f86b95c3c6f953bc9ce8e171d`, merged as
`b54c55ecf0b38681eabf5f16ecadf3d366ea796c`. All applicable normal CI passed;
hosted Codex reviewed that head with no findings. Local policy/site tests72 and
docs tests2583 passed. The single read-only review round's source-binding and
archive-custody findings were addressed before merge; a final receipt regression
ensures mismatched tool bytes remain visible. Copilot was quota-unavailable,
without a blind retry. These tooling checks are separate from qualification.

The [pre-dispatch freeze](https://github.com/Rambolarsen/orkworks/issues/1176#issuecomment-7200560910)
recorded the same source/tooling SHA and all four unchanged source hashes. The
fixed input policy SHA-256 is
`6870a4482dbec425369aceadeb79837496fe181924accc70362589b15b6d4dd0`.
No production release workflow, dependency feature or global flag was changed.

[Run45876634487](https://github.com/Rambolarsen/orkworks/actions/runs/45876634487)
was created18:31:38 UTC and its one candidate job ran18:31:42–18:32:02 UTC,
within the45-minute bound. It stopped in toolchain preflight with
`tool-byte-mismatch`. The four source expected/observed hashes matched; installed
Cargo and rustc bytes matched their approved pins. The selected cl.exe file
existed, but its observed SHA-256 differed.

| Selected tool | Expected SHA-256 | Observed SHA-256 |
| --- | --- | --- |
| Cargo1.100.0 pin | `f381bae27c59588eacf69208e115b2c001a2998afed83a2e9084ea93620737b1` | `f381bae27c59588eacf69208e115b2c001a2998afed83a2e9084ea93620737b1` |
| rustc1.100.0 pin | `d4f8a0283de2dba0dc458cd13c333337d2b76c3649eb39e91c4acce618e661a1` | `d4f8a0283de2dba0dc458cd13c333337d2b76c3649eb39e91c4acce618e661a1` |
| cl.exe in VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64 | `fe251ef50a1545b1b0835ee17b1e785459712b38d79e45b5c1d3d28970a36619` | `e87e78b3ef6580b75aebda843f8a22e874612d9f766344046551399f898eb226` |

The runner reported ImageOS win22 and ImageVersion20261116.370.1. The compiler
version query, SDK servicing inventory and complete toolset/SDK/source manifest
were not reached. The directory name and runner image therefore do not establish
the compiler's actual file version, why its bytes changed, or combined component
availability. No Cargo/rustc version child command ran either; their byte matches
refer to the accepted previously observed pins. No compiler substitution or
updated hash was accepted automatically.

### Terminal evidence and counters

[Artifact15144269964](https://github.com/Rambolarsen/orkworks/actions/runs/45876634487/artifacts/15144269964)
is the sole allowlisted artifact:1,216 compressed bytes and2,186 uncompressed
bytes, below256MiB. It expires2026-12-10 at18:31:58 UTC under14-day retention.
The two-file readback independently verified the frozen head, all four source
pins, both matching Rust tool hashes, the cl mismatch and terminal counters.
The raw receipt remains in the expiring artifact; this record preserves its
observations and hashes without extending raw evidence retention.

| File | Bytes | SHA-256 |
| --- | --- | --- |
| result.json | 2,186 | `741d44725037f435b26717892fe66495a63ffba537c8f113575d9bff76059ec6` |
| diagnostics.txt | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

**Terminal: dispatch1/1 consumed; fetch0, Cargo build0, additional preprocessing
oracle0 and resulting-program executions0. All child-operation slots close with
that dispatch; they are not remaining execution credit.** Workflow history shows
exactly one dispatch, run_attempt1 and the frozen head. The receipt records no
selected-tool child commands and reports owned Cargo/target scratch removed. Its empty
diagnostics file is expected because byte verification stopped before launching
the selected Cargo/rustc/compiler tools. There is no candidate executable, SQLite object/archive,
macro observation or link map from this batch. No artifact was executed.

The fixed candidate is unavailable under the approved byte identities. This
closes the finite acquisition task, not Gate1 or #1176. Earlier metadata/compiler/
native batches remain spent; bootstrap implementation, allocator/VFS/context
custody, ordinary compatibility, source/cutoff/newness/catalog/initializer/
recovery/durability and production activation remain gated.

A next task may review the retained compiler-byte mismatch and its provenance,
then propose a named component-availability request or replacement candidate with
a new finite allowance if further runner evidence is needed. That is a proposal,
not permission to query a new runner, amend the fixed pin, retry this workflow or
start the build. No new compiler/metadata/native allowance is created here.

## Compiler provenance review (2026-11-26)

The owner requested retained-evidence review after #2070. No new runner query,
compiler/tool invocation, acquisition or qualification dispatch was performed.
The review found different selection procedures and a historical custody limit:

| Evidence | What it establishes | Remaining gap |
| --- | --- | --- |
| Run42352195449 and its reviewed driver | The earlier driver used vswhere -latest, Launch-VsDevShell and Get-Command cl.exe, then recorded file version19.44.35229.0 and hash fe251…36619 on image20261101.353.1. | The report schema did not retain the full selected toolset path or compiler bytes. The current artifact endpoint lists zero artifacts; the approved one-day retention has elapsed. |
| Run45876634487 | Fixed path VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64/cl.exe had hash e87e…eb226 on image20261116.370.1; all four source pins and Cargo/rustc hashes matched. | File-version/SDK inventory was never reached. The observed hash does not identify why the bytes differ. |

The [older official image manifest](https://raw.githubusercontent.com/actions/runner-images/win22/20261101.353/images/windows/Windows2022-Readme.md)
lists Visual Studio17.14.37827.9 and SDK servicing package10.1.26100.7705. The
[newer manifest](https://raw.githubusercontent.com/actions/runner-images/win22/20261116.370/images/windows/Windows2022-Readme.md)
lists Visual Studio17.14.37906.3 and SDK servicing package10.1.26100.9457. Both
list SDK directory10.0.26100.0. These are published image inventories, not hashes
of the old and new compiler files or proof of the old driver's exact selection.

Image servicing and selection differences are plausible explanations, not a
verified historical root cause. There is no retained compiler binary pair for a
byte comparison, and equal directory/version labels would not prove equal bytes.
No evidence supports a claim of corruption, equivalence or an automatically
acceptable replacement compiler. Earlier component observations remain tied to
their original input identity and do not transfer to the new file.

The [proposed read-only component inventory](../superpowers/plans/2026-11-26-pair-toolchain-inventory.md)
asks for the missing current metadata and co-availability evidence before any
replacement build proposal. It does not reopen the spent batch, change its pin
or grant a new allowance. Gate1 remains Investigate; #1176 stays open.

## Read-only inventory tooling boundary (2026-11-26)

The owner accepted the [exact inventory proposal](../superpowers/plans/2026-11-26-pair-toolchain-inventory.md#scoped-allowance-accepted-2026-11-26)
after #2076. At the tooling review stage, dispatch was0/1. Earlier
batches stay spent; there is no replacement build-pin or compiler/Cargo execution
permission in this inventory.

The new controller selects exactly13 fixed PE paths plus the bounded default
toolset text and SDK registry/directory metadata. File data is streamed under
per-file and aggregate hash limits, with reparse paths denied and identities
checked again after collection. Both source-hash passes and the inventory hash
are charged; the collector reserves evidence/controller headroom inside1GiB.
Missing/drifting observations are retained before outcome classification.

Version numbers come directly from the selected file's embedded
VS_FIXEDFILEINFO data. The reader bounds section counts, three resource-directory
levels, entry counts, resource offsets and fixed-info sizes. It does not consult
localized MUI files, native version APIs, signatures or inspected DLL code.
PE machine describes host-file metadata only. This avoids extending the target
list through a convenience version reader; the [reference .NET implementation](https://raw.githubusercontent.com/dotnet/runtime/main/src/libraries/System.Diagnostics.FileVersionInfo/src/System/Diagnostics/FileVersionInfo.Windows.cs)
uses localized/neutral version API flags. This inventory does not use that API.

The SDK collector reads only the named local machine registry roots and fixed
directory attributes. It checks the entry budget before enumeration and exports
only bounded matching public SDK name/version fields. There is no Win32_Product,
package-manager, default-toolset selection, recursive SDK walk or installer repair.

The Node guard verifies manual attempt1, exact reviewed main checkout and sole
first-run history before launching its one fixed PowerShell controller. A minimal
child environment omits history credentials and inherited configuration. The
controller has a six-minute internal deadline, bounded output and drained process
references before owned working-scratch cleanup. The ten-minute workflow uploads
only result.json, inventory.json and diagnostics.txt, with4MiB/14-day limits.

Offline policy tests cover wrong-head/rerun denial, unknown paths, missing/drifting
inputs, reported byte/entry/text/evidence limits, terminal failures and real
controller timeout/output closure. The registry pre-enumeration guard is source-
reviewed; its actual registry I/O path is not exercised by the synthetic tests.
The embedded parser fixtures cover valid fixed-info data and an invalid RVA;
other section/directory/length bound branches are source-reviewed, not exhaustively
fixture-tested. Windows reader tests use hand-built data fixtures only;
no installed compiler or SDK binary is inspected by those tests. Their observed
missing-reader and missing-version-parser red runs precede implementation. Normal
repository CI remains separate from the one authorized inventory experiment.

Complete metadata would establish this limited observation/co-availability
snapshot, not authenticity, full used-helper closure, historical root cause,
macro/link custody or a replacement build selection. #1176 remains open and
Gate1 stays Investigate; all runtime/bootstrap/source/compatibility gates persist.

The independent review also identified a receipt boundary: dispatch/source/
workspace admission and initial scratch creation precede controller collection.
Denials there can have an Actions terminal result without an artifact receipt.
No file receipt can be promised when checkout/runner/storage itself is unavailable.
Record that provider-observed outcome as unavailable; do not infer completed
inspection or fabricate operation counts from an absent receipt. Once the
controller has established owned writable evidence scratch, ordinary collection
failures retain the bounded JSON result and available observations. The first-run
guard still closes the sole dispatch regardless of which evidence is available.

## Read-only inventory terminal result (2026-11-26)

**Terminal unavailable: SDK registry budget.** The sole accepted read-only
inventory retained the current compiler identity and all13 file observations,
but did not establish the required SDK servicing co-availability. No inspected
tool was executed. This closes the finite inventory task, not Gate1 or #1176.

Tooling [#2083](https://github.com/Rambolarsen/orkworks/pull/2083) merged at
2d9344ef09a17d6dba0ffdcf83fa5ef77c088e22. The
[pre-dispatch issue freeze](https://github.com/Rambolarsen/orkworks/issues/1176#issuecomment-7202395536)
recorded that exact main head and dispatch0/1. The sole manual run
[45889209265](https://github.com/Rambolarsen/orkworks/actions/runs/45889209265),
job140147422534, attempt1, used that head and ended in failure. Job start/end:
2026-11-26 at20:11:37–20:12:00 UTC; collection receipt:
20:11:47.905–20:11:56.522 UTC. Public workflow history independently showed exactly
one dispatch. The ten-minute job bound was not exhausted.

The runner reported Windows/X64, ImageOS win22 and ImageVersion20261123.376.1.
That image differs from the earlier fixed-build image20261116.370.1; it is context,
not proof of historical change or permission to select another image. One own
PowerShell controller ran at C:/Program Files/PowerShell/7/pwsh.exe and returned0.
The Node policy then denied success with sdk-unavailable and sdk-service.

### Current file observations

Every selected file reported status ok, machine8664 and equal initial/final
SHA-256 values. Paths are exactly the fixed selectors in the
[accepted allowance](../superpowers/plans/2026-11-26-pair-toolchain-inventory.md#exact-metadata-surface),
with Rust rooted at C:/Users/runneradmin. Hashes below identify this observation;
they do not accept a replacement build pin or attest authenticity. Numeric
versions come from embedded fixed-info resources, with language1033 when present.
Cargo has no retained version resource; its existing release label remains tied
to the previously observed byte pin.

| File | Bytes | File version | Product version | SHA-256 |
| --- | --- | --- | --- | --- |
| cargo.exe | 27,367,424 | unavailable | unavailable | `f381bae27c59588eacf69208e115b2c001a2998afed83a2e9084ea93620737b1` |
| rustc.exe | 110,592 | 1.100.0.0 | 1.100.0.0 | `d4f8a0283de2dba0dc458cd13c333337d2b76c3649eb39e91c4acce618e661a1` |
| cl.exe | 677,736 | 19.44.35230.0 | 14.44.35230.0 | `e87e78b3ef6580b75aebda843f8a22e874612d9f766344046551399f898eb226` |
| link.exe | 3,252,616 | 14.44.35230.0 | 14.44.35230.0 | `49a5df25c3182a22684375b1d62a025016deffccea247894fe5ca2c85610c6a6` |
| lib.exe | 23,424 | 14.44.35230.0 | 14.44.35230.0 | `69317eda3d44fd723b959c87d5a3b0617e3b7be70b02ff3f51ec45288ec559d2` |
| dumpbin.exe | 23,376 | 14.44.35230.0 | 14.44.35230.0 | `e2c0ec987bb904acfe88c0685cbc836e695cf22007cb17f3ce9f39e785dec980` |
| c1.dll | 3,429,728 | 19.44.35230.0 | 14.44.35230.0 | `3b82d58d21354fdb611efdd5ab82ec0484acfa5f3de37ca6bea42c630ff2c24a` |
| c1xx.dll | 12,330,336 | 19.44.35230.0 | 14.44.35230.0 | `a9348b973d399e7e042a79be7e22dc054e008381a2ef3d9f394300e41aa47470` |
| c2.dll | 10,456,376 | 19.44.35230.0 | 14.44.35230.0 | `e2fdfd6505cd6e9b670922733a914fbf22cae5d226943290548f24eaf699267a` |
| mspdbcore.dll | 1,321,824 | 14.44.35230.0 | 14.44.35230.0 | `a9fa44569d0555a92f53b56cb2b90cdca99bbb030fad2064f247cd03b2b406b1` |
| rc.exe | 55,736 | 10.0.26100.9457 | 10.0.26100.9457 | `9e2ee68da28b3318f50a9f1008d64567a9b402bda384a007a9b71a3f16bd011b` |
| mt.exe | 1,976,744 | 10.0.26100.9457 | 10.0.26100.9457 | `fa74dd31612c78de40169599640fb955ef9ccb9f2dfabf80f100022ff1b8bc22` |
| devenv.exe | 1,065,824 | 17.14.37906.3 | 17.14.37906.3 | `a222735c7b9f4bb454ad5772b2f31a5029ae91691c76c6f2452891f040045fd4` |

Cargo/rustc match their prior accepted byte identities. cl.exe matches the
inspection candidate e87e…eb226 and now has observed file version19.44.35230.0
and product version14.44.35230.0. The older observation was19.44.35229.0 with
fe251…36619; the historical cause of the byte change remains unverified. This
inventory does not transfer earlier macro/link/runtime results to this compiler.

### SDK and bounded text observations

KitsRoot10 was C:/Program Files (x86)/Windows Kits/10/. The fixed10.0.26100.0
Include, Lib and bin directory checks all returned true. The two allowed uninstall
registry roots reported688 subkeys in aggregate. The pre-enumeration256-entry
guard returned registry-budget before collecting entry names/product fields:
scanned_entries0 and products empty. This is unavailable SDK servicing evidence,
not evidence that the required package is absent. The rc.exe/mt.exe resources
report10.0.26100.9457, but those file versions do not replace the required matching
SDK product record or establish complete SDK/helper closure.

The fixed default-toolset text was13 bytes, value14.44.35207, initial/final hash
030613df718548a88970e4c1844a8605d17aba9aa36bfea0b4359d8f28fd62bf.
It remained context only; no default, alternate path or version was selected.

### Receipt identity, limits and closure

Independent literal readback checked the exact file/path allowlist, all13
initial/final hash pairs, Rust and candidate compiler pins, embedded versions,
SDK denial, counters, source identities and cleanup. It did not execute an
inspected binary or merely rely on the production validator's outcome. The
controller hashes match CRLF checkout forms of the committed source blobs at
the frozen head; they are not the LF blob hashes.

| Controller | Physical bytes | SHA-256 |
| --- | --- | --- |
| pair-windows-toolchain-inventory.mjs | 14,782 | `7d87e22d69ec189474cf4f6f4b6938fdb1501d61c0d71aa0ad0a5cdbeff6aa43` |
| pair-windows-toolchain-inventory.ps1 | 14,615 | `636c8bc28effe537a473845425b2b41ebe94037a883a0d9bae7d01b973354885` |

Selected PE files total62,091,736 bytes; the largest is27,367,424 bytes, below
256MiB. The collector charged28 hash passes (13 PE files and the bounded text,
each twice),124,183,498 bytes. Adding both controller-source passes and the
6,733-byte inventory hash gives124,249,025 charged bytes, below1GiB. Available
observations were retained before classification; all selected identities were
stable across the final checks. Owned working scratch was removed after the
controller closed. No inspected PE bytes were uploaded.

[Artifact15150876725](https://github.com/Rambolarsen/orkworks/actions/runs/45889209265/artifacts/15150876725)
contains only the three allowlisted files:2,540 compressed and8,169 uncompressed
bytes, below4MiB. Its API archive digest is
721859d335f7dd34c5a669de107b79e93bf50a9d10792bb73f478abbecf6ee52;
individual file bytes were independently hashed below. It expires2026-12-10
at20:11:56 UTC under14-day retention. This summary preserves observations without
extending raw receipt retention.

| File | Bytes | SHA-256 |
| --- | --- | --- |
| diagnostics.txt | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| inventory.json | 6,733 | `e8095de901eb6383874686538e2fcb7aa0e1e68834c7f72cdee329f0fee728da` |
| result.json | 1,436 | `777acb77c4617647a4925a39cce9ed47872d1f8c4b952faedaeb014d531e7462` |

**Dispatch1/1 consumed; inspected-tool executions0, fetches0, builds0 and target
executions0. All unused operation slots are closed.** The receipt records
runtime_qualified false and replacement_pin_accepted false. No compiler/SDK
substitution, registry-limit increase, retry, fallback query, Cargo metadata or
new build is authorized. Earlier batches remain spent.

A possible separate follow-up is to review a smaller SDK servicing-evidence
method against the retained688-entry budget failure and propose an exact new
allowance if live evidence is still necessary. This is a proposal only. The
current evidence does not support a replacement build yet. #1176 stays open and
Gate1 stays Investigate; runtime/bootstrap, allocator/VFS/context, compatibility,
source/cutoff/newness/catalog/initializer/recovery/durability and production
activation remain gated.


## SDK servicing follow-up proposal (2026-11-26)

After #2088, the owner requested the suggested smaller evidence proposal. The
[registry-only request](../superpowers/plans/2026-11-26-pair-sdk-servicing-inventory.md)
removes installed PE, directory and default-text reads and proposes a new
1,024-entry-per-pass bound for only the missing SDK product metadata. It uses
fixed-buffer reads and retains a separate run/image identity; it cannot join a
later SDK observation into the earlier compiler snapshot as co-availability proof.

This is a proposal, with no tooling or dispatch authority until scoped acceptance.
The old256-entry batch and all prior allowances stay spent. No replacement pin,
build or runtime qualification follows. #1176 remains open and Gate1 remains
Investigate.


## Accepted registry-only tooling boundary (2026-11-27)

The owner accepted the [exact #2092 allowance](../superpowers/plans/2026-11-26-pair-sdk-servicing-inventory.md#scoped-allowance-accepted-2026-11-27)
for separate tooling, one three-minute job and its terminal evidence record.
At the tooling preparation stage, dispatch was0/1. Every earlier batch stays
spent; no installed compiler/SDK PE, SDK directory or default-toolset read is
part of this request.

The new Node controller reuses existing first-run/process helpers read-only.
The fixed isolated Python3.12.10 collector uses fixed-buffer registry reads with
precharged API/buffer budgets and reserved handle-closure calls. It exports only
matching SDK records, bounded statuses/counts and canonical observation digests;
unrelated product strings stay out of receipts and diagnostics. Two planned
observations compare root counts/write markers and sorted membership/value
hashes; write markers cross JSON as decimal strings, not rounded numbers. This
is limited observation stability, not an atomic snapshot or authenticity proof.

Normal offline CI uses fakes and an isolated disposable HKCU fixture for the
Win32 boundary; it never queries installed SDK/uninstall data. Full local
verify-repo was invoked under pinned Node22 and stopped at missing desktop
dependencies; this does not claim a passed local full suite. Repository CI,
fixed interpreter selection and final code review remain pre-dispatch gates.
Any later SDK result is tied to its own run/image and cannot retroactively seal
the earlier compiler snapshot. Gate1 remains Investigate; #1176 stays open.


The [tooling review disposition](../superpowers/plans/2026-11-26-pair-sdk-servicing-inventory.md#tooling-review-disposition-2026-11-27)
records the one bounded independent round and its regression fixes. Both own
source passes now use fixed-size descriptor reads and reject size/type/name
changes before hashing; actual sealed bytes are charged. Failed observations
retain only explicitly allowed metadata fields. Enumerated child values stay
bound to the held parent handle, and failed closes remain outstanding rather
than reporting false closure. Returned metadata length/type fields are included
in fixed-buffer read charges. Stream overflow retains only a failure receipt.

The fixed isolated Python selector, native API fixture and complete failed
orchestration/cleanup fixture passed normal CI on windows-2022
(run45954970061/job140359313136, headdacfb9d9), as well as Windows-latest.
The native fixture touches only an owned disposable HKCU test key; the
orchestration fixture launches only a constant-output Python stub. These checks
supply no installed SDK product observation and do not spend the sole allowance.
At that reviewed tooling stage, dispatch was0/1 pending the merged source freeze.


## Registry-only SDK terminal result (2026-11-27)

**Terminal unavailable: value-format validation.** The sole accepted registry-only
job stopped during its first observation and retained no SDK product record.
This is incomplete servicing evidence, not evidence that the required SDK is
absent. No second observation, retry, fallback query or build ran.

Tooling [#2100](https://github.com/Rambolarsen/orkworks/pull/2100) merged as
9ee4da352d48a326c6bc87d7f11a23dd7ca3ab46. Its four controller/helper files,
workflow and byte-preservation rules matched reviewed final head
3db5d8ee8f7856cbda95e9f40d628a2994648221. All applicable checks passed, including
the single diagnosed normal-CI rerun of an unchanged process-ownership fixture;
hosted Codex review found no issues on that exact head. The
[pre-dispatch freeze](https://github.com/Rambolarsen/orkworks/issues/1176#issuecomment-7212333086)
recorded this merged source and the unused0/1 counter before dispatch.

The sole manual run
[45957491349](https://github.com/Rambolarsen/orkworks/actions/runs/45957491349),
job140367246111, attempt1, used that frozen head and ended in failure. Job
start/end:2026-11-27 at07:17:58–07:18:14 UTC. Receipt start/end:
07:18:07.856–07:18:10.037 UTC. Public workflow history independently showed exactly
one dispatch. Neither the three-minute job bound nor60-second collector bound
was exhausted.

### Available observation and denial meaning

The runner reported Windows/X64, ImageOS win22 and ImageVersion20261123.376.1.
One own isolated CPython3.12.10 X64 collector ran at
C:/hostedtoolcache/windows/Python/3.12.10/x64/python.exe and returned0 while
reporting success false, reason value-format. The Node result denied success
with sdk-unavailable. An exit code0 from this controller is not a successful SDK
inventory; the structured status governs that distinction.

| Registry observation | Retained value |
| --- | --- |
| KitsRoot10 | C:/Program Files (x86)/Windows Kits/10/ after approved normalization |
| HKLM64 SOFTWARE/Microsoft/Windows/CurrentVersion/Uninstall | 257 immediate children; before-write marker134399551354776438 |
| HKLM64 SOFTWARE/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall | 431 immediate children; before-write marker134399565468708431 |
| First pass | collecting at denial; no completed digest or after-count/write markers |
| Matching SDK product records | none retained before denial |
| Second pass | not performed |

The aggregate688-entry preflight fit this new1,024-entry cap. The collector then
rejected a bounded value under its accepted type/string-format rules. The source
label value-format covers an unsupported registry type, invalid length/UTF-16,
missing/embedded termination, empty text or non-printable/non-ASCII text. The
receipt does not export the rejected key/value, type, length or exact subtype;
no particular cause or defective SDK installation is established. Unrelated
product names and the rejected raw payload are not retained.

The Node validator's additional incomplete-pass/product/observation and
hashed_bytes-limit labels reflect missing complete observations and zero
completed observation hashing. They are not evidence of registry drift or hash
budget exhaustion. No runtime/ABI or image-version cause is inferred from them.
The old thirteen-file observations and published image inventories do not fill
this missing SDK product record or supply same-run compiler co-availability.

### Controller identities and work counters

Independent literal readback checked the exact run/head, sole history,
allowlisted files, receipt bytes/hashes, available registry metadata and counters.
All four physical source hashes match CRLF checkout forms of the committed blobs
at the frozen head. These are distinct from the committed source references in
the freeze comment; no installed executable was hashed or uploaded.

| Controller/helper | Physical bytes | SHA-256 |
| --- | --- | --- |
| pair-windows-sdk-inventory.mjs | 13,642 | `3a5637501a70a6b5f618b6ee1a8e76a1cea1569a69bae23d7950f7b1d6a71920` |
| pair-windows-sdk-inventory.py | 11,042 | `e4e7da919448634e06ab29c533ee28caadcede183fb4890aebc0a9f6372e278e` |
| pair-windows-metadata-evidence.mjs | 13,145 | `19aaf7a47a117951820325b33fbeb4206682e00464883990c9f81872d0e20de2` |
| pair-windows-toolchain-inventory.mjs | 14,782 | `7d87e22d69ec189474cf4f6f4b6938fdb1501d61c0d71aa0ad0a5cdbeff6aa43` |

The collector reported109 registry API calls (limit10,000),26,496 precharged
metadata bytes (limit4MiB), zero open handles and zero completed observation-hash
bytes. Fixed-buffer charges are conservative upper bounds, not measured native
heap costs or a claim about actual returned string lengths. Partial entry/value
work is not reconstructed as a complete scan from the root counts.

Controller/helper sources total52,611 bytes, below1MiB. Their two stable hash
passes charge105,222 bytes; adding the598-byte retained inventory hash yields
105,820 total hashed bytes, below8MiB. Owned working scratch was removed after
the collector process and references closed. No complete SDK observation digest,
full registry tree, inspected PE bytes, compiler output or executable candidate
exists from this batch.

### Terminal receipt and authority closure

[Artifact15179437123](https://github.com/Rambolarsen/orkworks/actions/runs/45957491349/artifacts/15179437123)
contains only result.json, sdk-inventory.json and diagnostics.txt:1,658 compressed
and2,558 uncompressed bytes, below256KiB. Its API archive digest is
6bcbd201a8c40158e0dcefbd8bad6e107d759153b66da9090106dfe1cd2af3b3.
It expires2026-12-11 at07:18:10 UTC under14-day retention. Individual file hashes
were independently recomputed:

| File | Bytes | SHA-256 |
| --- | --- | --- |
| diagnostics.txt | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| result.json | 1,960 | `d88b16a042fd809250a0b28526164856a9636078beadee7486c366e6ff258ea2` |
| sdk-inventory.json | 598 | `b842e70ebffac345b2999061a399ae8e146f5dde1b1ddea0f594b737ca1981f2` |

The raw receipt stays in the expiring provider artifact; this summary preserves
its available observations without extending raw retention.

**Dispatch1/1 consumed; inspected-tool executions0, fetches0, builds0 and target
executions0. Every unused slot is closed.** The receipt records runtime_qualified
false and replacement_pin_accepted false. Earlier metadata/compiler/native
batches stay spent. No retry, alternate selector, repair, quota increase, package
probe, Cargo metadata or replacement build is authorized.

A useful separate prerequisite is a source-only review of this strict validation
boundary and what the retained receipt can distinguish. Any changed evidence
method or live runner request needs its own exact scoped decision; this failure
supplies no renewed credit. #1176 remains open; Gate1 stays Investigate and all
runtime/bootstrap, allocator/VFS/context, source/cutoff/newness/catalog/
initializer/recovery/durability, compatibility and production gates remain.


## SDK value-format source review (2026-11-27)

The owner requested source-only review after #2118. This review uses the
[retained terminal record](#registry-only-sdk-terminal-result-2026-11-27) and
[sealed collector source at9ee4da35](https://github.com/Rambolarsen/orkworks/blob/9ee4da352d48a326c6bc87d7f11a23dd7ca3ab46/scripts/qualification/pair-windows-sdk-inventory.py).
No raw receipt was reacquired and no new qualification dispatch or installed
registry query was performed. The derivation uses source and pure integer
arithmetic; standard PR verification remains separate. Consumed controllers/
workflows and all numerical limits are unchanged.

### The label identifies a local admission rejection

In the [sealed native value wrapper](https://github.com/Rambolarsen/orkworks/blob/9ee4da352d48a326c6bc87d7f11a23dd7ca3ab46/scripts/qualification/pair-windows-sdk-inventory.py#L127-L137),
a missing optional DisplayName returns absent; other Win32 errors or a returned
size larger than the fixed buffer produce registry-value. Only after those
checks pass does the [decoder](https://github.com/Rambolarsen/orkworks/blob/9ee4da352d48a326c6bc87d7f11a23dd7ca3ab46/scripts/qualification/pair-windows-sdk-inventory.py#L53-L65)
produce value-format for the accepted type/string predicates. Thus the label
identifies a local decoder rejection after the wrapper's native status/size
checks, rather than a reported Win32 query error. It does not distinguish which
decoder predicate failed.

Printable ASCII and non-empty text are collector admission choices. Windows
[registry value types](https://learn.microsoft.com/en-us/windows/win32/sysinfo/registry-value-types)
include Unicode string representations; printable ASCII is not a universal
registry validity rule. Microsoft also warns that
[RegQueryValueExW](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regqueryvalueexw)
can return string data without proper termination. These are possible classes
the guard distinguishes from its accepted inputs, not diagnoses of this run's
unretained payload. No evidence selects Unicode, empty text, an unsupported type,
termination or another specific formatting defect as the actual cause.

### Counters narrow the failed field without recovering its value

The retained payload has an accepted KitsRoot10, one collecting pass, no after
observations, 109 API calls and 26,496 precharged metadata bytes. In the sealed
[collection schedule](https://github.com/Rambolarsen/orkworks/blob/9ee4da352d48a326c6bc87d7f11a23dd7ca3ab46/scripts/qualification/pair-windows-sdk-inventory.py#L147-L183),
KitsRoot10 is retained only after its successful read/normalization. Failure
therefore occurs in an uninstall child's DisplayName or DisplayVersion read,
before post-enumeration root queries.

Let N be child-name/DisplayName visits and V be DisplayVersion queries attempted
before the denial. Both include the failing attempt when applicable. Every
visited child is closed during exception unwinding, and the two held uninstall
roots are closed too. The sealed counters give:

| Charged work | Source-derived expression |
| --- | --- |
| API calls | 9 + 4N + V: three fixed opens/closes, two root-info calls, one KitsRoot10 query, plus enum/open/DisplayName/close per child and each version query |
| Metadata bytes | 546 + 1,038N + 266V: two 12-byte root-info buffers, one 522-byte KitsRoot10 charge,516-byte name and 522-byte DisplayName charges per child, and 266 bytes per version query |

Solving 9 + 4N + V = 109 and 546 + 1,038N + 266V = 26,496 yields
**N = 25 and V = 0**. Pure integer arithmetic enumeration over the retained 688
root entries and at most 32 version-query attempts independently gave this unique
bounded solution. The 25th child remains within the first root's 257 entries:
HKLM64 SOFTWARE/Microsoft/Windows/CurrentVersion/Uninstall. The trace reconciles
as 28 opens + 28 closes + 2 info calls + 25 enumerations + 26 value queries = 109;
2×12 +25×516 +26×522 =26,496 charged bytes.

**This is a control-flow inference conditional on the sealed implementation and
retained counters, not an exported per-operation trace.** It identifies the
DisplayName stage and execution ordinal, not the key, name text, registry type,
returned length or exact decoder defect. Earlier optional names may have been
absent; no earlier entry is promoted to a complete SDK observation. There were
no DisplayVersion queries in this inferred prefix. The charged bytes are fixed
buffer reservations and do not reveal the rejected value's actual length.

The ordinal cannot become a new fixed selector:
[RegEnumKeyExW](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regenumkeyexw)
does not define a stable subkey ordering. No named SDK product key or raw value
can be reconstructed from these counts. SDK servicing identity remains
unavailable, and no defective SDK installation or complete registry scan is
established.

### Consequence for the next decision

The collector decodes every DisplayName before applying its SDK-name filter.
An unclassified name can therefore block SDK discovery, regardless of whether
that unknown name belongs to an SDK. The retained failure supports reviewing
that classification/admission boundary; it does not justify larger entry, API,
time or artifact budgets, and repeating the same request supplies no known fix.

Do not silently treat a rejected name as a proven non-SDK entry. Before another
evidence request, define whether the goal is a fully classified inventory or a
stable positive record for a source-backed exact SDK selector, and how bounded
unclassified metadata is represented. Neither a new selector nor a relaxed
format policy is accepted here. Any later method needs a separate concrete
scoped decision and finite allowance; the spent workflows remain unchanged.

This completes the source review only. Dispatch remains spent 1/1, every unused
slot remains closed, #1176 stays open and Gate1 remains Investigate. No compiler
pin, co-availability seal, build, runtime profile or production activation follows.

## Positive SDK record proposal (2026-11-27)

The [new positive-record proposal](../superpowers/plans/2026-11-27-pair-sdk-positive-record-observation.md)
defines explicit unclassified DisplayName handling after the source review. It
requests a stable positive SDK record with inventory completeness explicitly
false, strict matching SDK fields, private bounded payload commitments, and one
new finite job under the existing caps. It is awaiting owner approval of the
contract, tooling and exact allowance together. Proposal publication authorizes
no implementation or dispatch. Every earlier allowance remains spent; no new
registry observation or runner qualification occurred for this proposal.

### Accepted positive-record tooling scope

The owner subsequently accepted the proposal's combined contract, isolated
tooling and one exact finite job by replying “keep going” to the explicit
approval request after #2126. Implementation and synthetic verification are
tracked in the linked plan's execution ledger and #1176. The allowance is one
three-minute windows-2022 job, two passes of at most1,024 aggregate entries, with
no retries and unchanged resource caps. Tooling review/merge and a frozen source
record must precede its sole dispatch; no new runner observation has occurred
at this tooling checkpoint. Earlier allowances remain spent.

## Positive SDK record terminal result (2026-11-27)

**Outcome: positive product observation succeeded; allowance spent1/1.** Owner
accepted the [exact proposal](../superpowers/plans/2026-11-27-pair-sdk-positive-record-observation.md)
after #2126. [Tooling #2134](https://github.com/Rambolarsen/orkworks/pull/2134)
merged as93be0f10b83c92ffefaafcb58a0fbd0e359b6fb6 after full local repository
verification, required CI and current-head review. The
[pre-dispatch freeze](https://github.com/Rambolarsen/orkworks/issues/1176#issuecomment-7217216937)
confirmed zero prior runs and source/workflow identity with reviewed
head49e9ede8eb4586f5418c37b51bd4fb586668c554. Existing consumed workflows and
collectors were not changed.

Sole [run45991524648](https://github.com/Rambolarsen/orkworks/actions/runs/45991524648),
attempt1, succeeded at that frozen merged SHA. Its only
[job140480267131](https://github.com/Rambolarsen/orkworks/actions/runs/45991524648/job/140480267131)
ran11:34:06–11:34:27Z; receipt interval11:34:19.236–11:34:21.730Z. Runner:
Windows X64, win22 image20261123.376.1. One isolated CPython3.12.10 X64 collector
ran at C:/hostedtoolcache/windows/Python/3.12.10/x64/python.exe; exit0. No
inspected tool, fetch, build or target execution occurred. Controller count1,
dispatch-history count1, problems empty, owned working scratch removed.

### Scoped positive fact and retained unknowns

Schema2, evidence_kind sdk-positive-product-observation reports success=true and
positive_record_observed=true. **inventory_complete=false, runtime_qualified=false
and replacement_pin_accepted=false.** Both complete passes reported identical
products, counts, root markers and canonical digest
19d4f8f0183282ff6bca181bf3d15509f1d3c7c21e7312d83956d837cdc5f09d.
KitsRoot10 normalized to C:/Program Files (x86)/Windows Kits/10/.

The local HKLM64 roots retained these equal before/after observations in both
passes (paths use the collector's separator convention):

| Root | Count per pass | Before and after write marker |
| --- | ---: | --- |
| SOFTWARE/Microsoft/Windows/CurrentVersion/Uninstall | 257 | 134399551354776438 |
| SOFTWARE/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall | 431 | 134399565468708431 |

All five matching public SDK records were in the second root. The table gives
the immediate child component; prepend that root and a separator for the key.

| Child component | DisplayName | DisplayVersion |
| --- | --- | --- |
| {45e335f3-c807-43b8-9ed1-43666cfcf939} | Windows Software Development Kit - Windows 10.0.26100.9457 | 10.1.26100.9457 |
| {5f4dc51d-f151-4325-8ba1-8b26169529a9} | Windows Software Development Kit - Windows 10.0.19041.5609 | 10.1.19041.5609 |
| {5fe95b9d-9219-4d8b-a031-71323ae48a81} | Windows Software Development Kit - Windows 10.0.17763.132 | 10.1.17763.132 |
| {b72d4013-dd88-405a-9f9d-e88c278dcefe} | Windows Software Development Kit - Windows 10.0.22621.5040 | 10.1.22621.5040 |
| {c0b93b80-d0a4-47fc-a4d1-21a4008a63ca} | Windows Software Development Kit - Windows 10.0.26100.7705 | 10.1.26100.7705 |

Per pass, the688 entries comprised659 accepted ASCII names,27 absent names and
**two unclassified names**. The659 admitted names comprised five literal SDK
predicate matches and654 predicate nonmatches. Unclassified reasons were one
unsupported_type and one non_ascii_or_nonprintable. Those are local admission
categories, not identified keys, exact native types, raw bytes, or declarations
of invalid Windows metadata. No unrelated names or individual value digests
were retained. An unclassified name could describe another SDK record. These
new aggregate counts do not identify the prior batch's rejected25th name or
retroactively resolve its exact format cause.

### Bounded work and independent readback

The receipt reports5,536 registry API calls,1,432,088 charged metadata bytes and
zero open handles. There were1,376 child visits and ten matching-version queries
over two passes; counts reconstruct the API/read ledgers under the reviewed
collector:2×(4 root open/close calls +4 info calls +2 installed-root open/close
calls +1 KitsRoot10 query +688×4 enum/open/name-query/close calls +5 version
queries)=5,536;2×(4×12 +688×516 +688×522 +5×266 +522)=1,432,088 bytes.
All existing caps held; no retry, fallback, repair or quota increase occurred.

Collector hashing charged371,402 bytes: raw values102,104, canonical rows266,518
and pass envelopes2,780. The four physical Windows source files total60,595
bytes; two sealing passes charged121,190. Hashing the retained3,965-byte
observation gives **496,557 total charged hash bytes**, below8 MiB.

| Physical Windows source | Bytes | SHA-256 |
| --- | ---: | --- |
| pair-windows-sdk-positive-record.mjs | 17882 | 7b26e55b3d60d271000a87fc004a7f57405748f4d163cca2eb429168cec32d85 |
| pair-windows-sdk-positive-record.py | 14786 | acca2438deef46c1453a46186853e560b1c7bba03424881b015a36439978f5a8 |
| pair-windows-metadata-evidence.mjs | 13145 | 19aaf7a47a117951820325b33fbeb4206682e00464883990c9f81872d0e20de2 |
| pair-windows-toolchain-inventory.mjs | 14782 | 7d87e22d69ec189474cf4f6f4b6938fdb1501d61c0d71aa0ad0a5cdbeff6aa43 |

Independent readback reconciled all four hashes/sizes with the committed source
at the frozen SHA using CRLF checkout bytes; the freeze separately records LF
identities. It verified the archive and three file hashes, schema/authority
flags, pass equality, exact positive record, counter arithmetic, hash ledgers
and cleanup. The private raw registry inputs were intentionally not retained,
so readback could compare the reported pass digests but could not independently
recompute those private observation digests. No extra registry or tool read was
performed to fill that limitation.

Artifact15194520150, pair-windows-sdk-positive-record, expires2026-12-11T11:34:22Z.
The2,201-byte archive SHA-256 was independently checked against the Actions
digest9a7a4a45fe19ed0f0599d01dbecfc448472f7a0ee7288a8f3dceeb2e40c4a152.
The three uncompressed files total5,860 bytes, below256 KiB:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| result.json | 1895 | 92ba627200b2f5648a46c42f81497aa3b02a92f7447059f63ee06148dfaa6e7a |
| positive-record.json | 3965 | 74aef004c2185f067950141cd525ba60ce211c204051d51dc3c9899b5d5e5f74 |
| diagnostics.txt | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |

### Authority closure

The exact SDK servicing record is now a stable positive observation for this
job. It does not establish complete SDK inventory, absence, package authenticity,
compiler/SDK co-availability with earlier jobs, helper/source closure, accepted
replacement build pins, a successful build or runtime qualification. Image and
root-marker equality with earlier receipts do not supply those missing facts.

This allowance is terminal and spent1/1; every unused slot is closed. No rerun,
replacement selector/image, repair or follow-on job is authorized. Earlier
allowances remain spent. #1176 stays open; Gate1 remains Investigate and later
source/custody/compatibility/recovery/durability/activation gates remain closed.
Next useful work is a source-only reconciliation of the retained toolchain and
SDK facts against the remaining qualification requirements, before proposing
any further acquisition or build allowance.

## Toolchain and SDK source reconciliation (2026-11-27)

Disposition: **positive SDK evidence established; exact-build and mechanism
qualification remain Investigate.** Inspection base c87e020e170b7b288750f2e45955198b6696c894.
This source-only follow-up to #2135 reconciles existing records and committed
policies. It reacquired no raw artifact, queried no runner or installed tool,
executed no qualification program and changed no pin, collector, workflow or
allowance. Every earlier execution allowance remains terminal and spent.

### Evidence ledger and permissible joins

| Retained record | Established fact | Limit on reuse |
| --- | --- | --- |
| [Metadata run45763908423](#one-windows-metadata-attempt-completed--2026-11-26) | Complete target-filtered graph for the frozen default-feature request; SQLite/build.rs identities matched; Cargo/rustc byte identities observed | A resolve graph is not a compiler/unit/link graph. No effective macro or linked object was produced. |
| [Fixed-build run45876634487](#fixed-windows-build-batch-closed-unavailable-2026-11-26) | Four source pins and Cargo/rustc matched; selected cl.exe differed from its approved pin | Preflight stopped before tool version commands, fetch, build and oracle. There is no candidate, object, archive, map or link-custody receipt to qualify. |
| [Component run45889209265](#read-only-inventory-terminal-result-2026-11-26) | Thirteen selected PE identities stable within that job; cl.exe19.44.35230.0 observed at the fixed toolset selector; selected SDK directories existed | Its SDK product enumeration never began. Thirteen files are a bounded subset, not all compiler/Rust/SDK headers, libraries, helpers or actual linked inputs. |
| [Registry-only run45957491349](#registry-only-sdk-terminal-result-2026-11-27) | Bounded partial observation and local DisplayName rejection | Terminal unavailable; neither absence nor a complete inventory. Later success does not rewrite this result or identify its exact rejected value. |
| [Positive-record run45991524648](#positive-sdk-record-terminal-result-2026-11-27) | SDK10.1.26100.9457 product record stable in two passes, with five matching records and two explicitly unclassified names | Positive-only schema: inventory_complete=false. No compiler/PE reads or build; it cannot seal co-availability with the earlier thirteen-file job. |

The metadata/fixed-build image was20261116.370.1; the later component and SDK
observations reported20261123.376.1. Equal later image labels, KitsRoot10, root
counts or write markers provide context, not identity of a shared job, filesystem
contents or loader/helper closure. No historical cause for the compiler change
has been established. Do not transfer the older compiler's macro/link/runtime
results to the newer bytes or combine separate receipts into one build witness.

### Current source and selection comparison

A direct SHA-256 comparison of the four current repository files against
[scripts/qualification/pair-fixed-windows-build.inputs.json](../../scripts/qualification/pair-fixed-windows-build.inputs.json)
found all four equal to the frozen metadata/build baseline:

| Input | Current SHA-256, equal to policy |
| --- | --- |
| crates/orkworksd/Cargo.toml | 0134c524d6e92d0f01f3cf3953f687c1ee0b570e54da879659061238aa615cc8 |
| crates/orkworksd/Cargo.lock | 40845c5bdf0a4ed869e93af1de572220711f0d2fabf31f546488b1cdbab28ded |
| rust-toolchain.toml | d4d89c66d4e571b2f86ef1c5484191acca0465a101b6c46083eea9537e69fc25 |
| crates/orkworksd/build.rs | 1111ea2be9dfa3ddeb9402608723b50b01bd304a81d3de63d7d1427fe7915edf |

This is a comparison of those four source inputs, not a claim that all current
sidecar source matches an earlier tree or that a new runner has the same resolved
inputs. Reuse the frozen graph only within its recorded request; any future
attempt must bind its reviewed source head and actual complete input manifest.

The spent policy still requires cl.exe SHA-256
fe251ef50a1545b1b0835ee17b1e785459712b38d79e45b5c1d3d28970a36619 and the
[spent driver](../../scripts/qualification/pair-fixed-windows-build.mjs) requires
file version19.44.35229.0. The later observation is SHA-256
e87e78b3ef6580b75aebda843f8a22e874612d9f766344046551399f898eb226,
file version19.44.35230.0/product14.44.35230.0. The shared directory label
14.44.35207 does not make these byte identities equivalent. A replacement would
be a new candidate decision, never a policy patch or retry of the consumed build.

### Requirements that remain without actual evidence

The [accepted build/startup provenance contract](../superpowers/specs/2026-11-24-pair-build-startup-design.md#exact-proposed-build-rule-and-provenance)
and [ADR0119](../adr/0119-pair-build-startup-custody.md) remain the requirements.
The existing [fixed-build input audit](#fixed-build-input-audit--2026-11-26) and
driver describe planned checks; they do not report those checks executing after
the preflight denial.

| Requirement | Remaining evidence or decision |
| --- | --- |
| Candidate and complete inputs | Explicit replacement compiler decision if selecting the observed newer bytes; exact Rust/MSVC/SDK selectors and a same-job source/tool/header/library/helper manifest sealed before compilation and rechecked afterward. The separate SDK receipt supplies a known positive record, not a reusable availability token. |
| Build-script/proc-macro/configuration closure | Reconcile the reviewed37 build-script and14 proc-macro entrypoints, all package files/lock checksums and actual child environment/configuration against the selected build. Existing static helper audits are conditional source evidence; no fetch/build stage of the failed fixed attempt reached this verification. |
| Effective SQLite semantics | Actual SQLite compiler/argv/source/environment association and the same-input oracle showing MEMORY_MANAGEMENT absent, THREADSAFE=1 and reviewed override exclusions. LIBSQLITE3_FLAGS text and a source branch are insufficient; the old compiler's result does not qualify a new compiler. |
| Linked-artifact custody | Actual SQLite object, native archive, unique Cargo libsqlite3_sys archive, matching extracted member bytes, final map/import association and unsigned sidecar identity. No such output exists from the spent fixed-build batch. No reproducibility claim follows from a hash alone. |
| Startup and configuration ownership | Actual early owner, stable saved methods/pAppData, no unknown prior writer/cache/lookaside state, sticky failure and unpaired delegation. A built artifact or dormant fake-state tests alone do not prove that history. |
| Ordinary compatibility | Qualify Codex/OpenCode reads and ordinary storage under the changed shared library/actual bootstrap, including changed memory-reclamation behavior, late/partial init, allocation provenance, failure and concurrent unpaired use. Unchanged SQL or absent direct release_memory calls are not compatibility evidence. |
| Controlled mechanism and full profile | Actual EOF/arena/VFS/context owners and their pre-effect/lifetime checks, source authority/newness/cutoff/catalog/initializer participation, original old/new/unknown recovery and paired durability. Toolchain/SDK availability cannot supply these independent witnesses. |

In particular, the reviewed driver would build a complete input manifest, derive
an oracle from the actual SQLite compile command, and compare the captured
SQLite object with members extracted from both archives plus final map rows.
Those branches remain unobserved. Their source and synthetic tests explain what
to demand from a later candidate; they do not mint link custody now.

### Gate disposition and next bounded deliverable

[ADR0118](../adr/0118-pair-resource-assurance.md) still accepts only the logical
EOF/fixed-arena guarantee, with native host costs outside its numeric caps. The
shared macro/startup recipe is already accepted for further qualification; do
not ask the owner to approve those unchanged choices again. Source authority,
ordinary compatibility and actual runtime mechanisms are still unqualified.

Gate1 remains Investigate. Gate2's
[private bootstrap-state plan](../superpowers/plans/2026-11-24-pair-bootstrap-state-tests.md)
still requires its stated source closure and bounded implementation authority;
Gate3 native qualification and Gate4 complete admission/activation remain gated.
The inspected production Connection::open still returns unqualified profile
before path I/O. This reconciliation unlocks no code, SQL/database, native/model,
compiler, acquisition, migration or production route. #1176 remains open.

The next useful deliverable is a **new written replacement-candidate build
proposal**, using the observed compiler identity as a candidate and reusing the
accepted macro/startup recipe. It should resolve same-job SDK/input sealing,
complete helper/configuration custody, exact outputs/oracles and one explicit
finite allowance together. Do not schedule another standalone availability
probe merely because these earlier records are separate. Proposal publication
authorizes no tooling or job; obtain one scoped owner decision before dependent
work. Keep every consumed workflow/pin/history and all unused slots closed.

## Replacement build proposal (2026-11-27)

This paragraph records proposal publication. Owner acceptance, execution and
the spent allowance are recorded in the terminal section below.

The [new replacement-candidate proposal](../superpowers/plans/2026-11-27-pair-replacement-windows-build.md)
selects the observed19.44.35230.0 compiler as a proposed input, uses the retained
exact SDK product key for two bounded reads bracketing the build, and combines
controlled input sealing, macro/link-custody oracles and one45-minute job in one
owner decision. It reuses the accepted macro/startup recipe. Publication is
proposal-only: no pin, tooling, execution or qualification is accepted by its
merge. Every prior allowance remains spent and all existing gates remain binding.


## Replacement build terminal result (2026-11-27)

**Terminal unavailable; allowance 1/1 spent, zero retries.** The owner accepted
the combined [replacement package](../superpowers/plans/2026-11-27-pair-replacement-windows-build.md)
after proposal #2159. [Tooling #2168](https://github.com/Rambolarsen/orkworks/pull/2168)
merged as a37ec6196355b795d1d7fd25a0c9a7c06ad281a4 after full repository
verification, current-head required CI, one bounded two-Luna review round and
Codex review of ab787965d4738975e033da3766878e158f1a6b97 without findings.
The [source/policy/command freeze](https://github.com/Rambolarsen/orkworks/issues/1176#issuecomment-7221453346)
preceded the sole [run 46020662577](https://github.com/Rambolarsen/orkworks/actions/runs/46020662577),
attempt 1, at that exact merged head. Workflow history independently returned
one run and one attempt. No consumed workflow, pin or history was edited.

### Observed stage boundary

The [candidate job](https://github.com/Rambolarsen/orkworks/actions/runs/46020662577/job/140579169638)
ran from 14:58:21Z to 15:00:58Z on image win22/20261123.376.1. Its controller
receipt spans 14:58:34.005Z–15:00:53.035Z. The schema 2
replacement-sidecar-candidate receipt reports stage=sources, reason=time-limit.
The source-verification PowerShell invocation reached its 120-second ceiling;
the 45-minute job ceiling and 35-minute build ceiling were not reached. The
reader combines locked archive/member verification and the initial manifest;
no completed manifest or internal progress record was retained, so the receipt
does not locate the timeout within that combined pass or prove package/helper
closure completed. Do not infer a measured installation size or a larger
required timeout from this result.

| Observation | Retained result |
| --- | --- |
| Installed PE preflight | All twelve selected hashes and x64 machine values match the frozen policy. cl.exe is 19.44.35230.0, SHA e87e78b3ef6580b75aebda843f8a22e874612d9f766344046551399f898eb226. Selected rustc/rc/mt numeric versions also match. These remain batch input observations, not production pins. |
| Controllers | Node v22.23.5; PowerShell 7.6.10; isolated CPython3.12.10/x64. Exact paths and executable hashes are retained in result.json. |
| SDK before pass | One complete exact-key read: SDK 10.1.26100.9457 and KitsRoot10=C:/Program Files (x86)/Windows Kits/10/. Installed/product write markers were stable within this pass. No after pass ran. |
| Acquisition | One locked fetch invocation; the driver advanced to sources only after accepting its zero exit. No metadata query or acquisition retry. |
| Later work | build_invocations=0, oracle_invocations=0, target_executions=0. No SQLite object/archive, candidate sidecar, effective macro result or linked-artifact custody exists from this batch. |
| Termination and cleanup | Two PowerShell readers and one Python SDK invocation; one owned taskkill fallback, child drain completed, cleanup=owned-scratch-removed. No later build or second SDK/manifest pass started. |
| Final authority | success=false, candidate_evidence_complete=false, runtime_qualified=false, production_pin_accepted=false. The incomplete SDK bracket also has success=false, positive_record_observed=false and inventory_complete=false. |

The first SDK pass used 13 API calls and 1,358 charged read bytes, with zero
enumeration and zero open handles. Its raw/framed commitment charged 1,000 hash
bytes; the 753-byte SDK receipt brings the separate SDK hash charge to 1,753.
The full-width Installed Roots/product write markers are 134399460646426989 and
134399614693565521; before/after arrays within the pass are equal. Its digest is
e7a893d99f932760873cc45a2953db960340e0ad1d960491b1025195c1cd1c28.
This partial record supplies no cross-build bracket or SDK inventory completion.

The parent reports 9,497 cumulative child stream bytes and 34,358,706,779 charged
hash/read-for-hash bytes under the 32 GiB ceiling. **The latter is conservative
reservation accounting, not measured hashing throughput:** hash_work_unknown=true
because the timed-out reader supplied no final work report. The parent retained
the entire delegated grant as spent, preserved its failure-publication reserve
and did not refund or reuse the unknown work. SDK work accounting is separately
known. No file/directory/manifest capacity measurement follows from this timeout.

### Source checkout reconciliation and independent readback

The freeze verified byte identity between reviewed and merged Git blobs for all
nine own/imported sources (179,584 Git-blob bytes). The five new own sources
retained those hashes on the runner. The four imported historical files used
Windows checkout bytes: each retained runner hash exactly equals the SHA-256 of
the corresponding frozen Git blob after LF-to-CRLF conversion. This reconciles
the initial hashes; it is not a completed second source seal. The earlier freeze's
179,584 figure describes Git blobs, not a measured Windows source-pass byte total.
All existing attributes and spent source files remain unchanged.

| Imported source | Frozen Git SHA-256 | Runner SHA-256, matching LF-to-CRLF conversion |
| --- | --- | --- |
| scripts/qualification/pair-fixed-windows-build.mjs | 4a8ce2742bf280badf6337b6f715d4739566d1d6a51a4514825dc3b2fb924baa | f3af7ca64074db7ce2bc3c30aff68af046c4147e9c4283045faf03eb288ae529 |
| scripts/qualification/pair-fixed-windows-build.inputs.json | 6870a4482dbec425369aceadeb79837496fe181924accc70362589b15b6d4dd0 | f8eccaafd28e77966f6614c1ec4716c6cd3acb18d5bfc437fd6f3dd216c653db |
| scripts/qualification/pair-windows-metadata-evidence.mjs | 3f4f9a7b00bc2ec49f385de3000db5610b1400f02518e36ba2fc73d734947a79 | 19aaf7a47a117951820325b33fbeb4206682e00464883990c9f81872d0e20de2 |
| scripts/qualification/pair-windows-sdk-positive-record.py | 39fea3b40bef85121aa6dab0c7cd1dcb09910f5573ddc4a6ccdcf4548c1ee8d5 | acca2438deef46c1453a46186853e560b1c7bba03424881b015a36439978f5a8 |

Artifact 15206744794, pair-replacement-windows-build, contains exactly three
allowlisted files and 17,373 uncompressed bytes. Its 5,792-byte ZIP SHA-256 is
478c7ef3db9a3fcda5c03b49a52f3dbbc40f23e31860c7c0b99be9ad8d57ee58,
matching GitHub's artifact digest. Expiry is 2026-12-11T15:00:53Z (14 days).
Independent bounded readback checked ZIP membership/sizes, every retained file
hash, receipt identity/authority, all twelve PE pins, the initial source-hash
reconciliation, partial SDK counters and the zero later-stage counts. Plain
workspace/scratch prefixes are absent from retained diagnostics. No retained program was
executed or loaded as code. The raw ZIP and temporary receipt copies are
removed after this terminal record is published; the hosted artifact retains
its fixed expiry.

| File | Bytes | Independently checked SHA-256 |
| --- | --- | --- |
| result.json | 8503 | 5682d3946b77b7b602375f8a3137c5e54973eae1ffa7fef9d13aa161eaa88cc5 |
| diagnostics.txt | 8117 | 083399452a855f4d77992121085f40881da0d54bdfcb3e852c891ebd2190795f |
| sdk-record.json | 753 | 6939c967ad9fecccccc5b84c2452389890fc821389d0c6326f1d35a2d72af2bf |

### Disposition

Gate 1 remains Investigate. Gate 2/bootstrap, native mechanisms, actual macro/link
custody, ordinary compatibility, source authority/newness/cutoff/catalog/initializer,
recovery/durability, complete profile and activation remain gated. The production
Connection::open refusal is unchanged. All earlier allowances and this allowance
are spent; every unused slot is closed. This record authorizes no retry,
replacement job, timeout/hash-budget increase, dependency/configuration change,
new compiler pin or bootstrap implementation. #1176 remains an open umbrella.

The next useful deliverable is a **source-only diagnosis proposal for the bounded
reader's work and progress evidence**, using this timeout boundary and synthetic
fixtures. A future executable package would need its own concrete reviewed
scope and finite owner allowance. Do not use this failure to silently extend
caps, edit the spent collector or repeat an availability probe.


## Reader timeout source diagnosis (2026-11-27)

The [source-only diagnosis](pair-reader-timeout-diagnosis.md) traces the sealed
reader's archive/member checks, manifest, sorting and serialization under one
120-second deadline. An owned Node-only synthetic experiment confirms that
bytes buffered before timeout are not returned to the reader caller, which
therefore retains the full unknown-work grant. Source inspection also identifies
repeated per-member 1 MiB buffer allocation and ancestry/identity/row-conversion
work. These are cost candidates, not a measured explanation of the Windows
failure. No completed per-phase/root count exists, so runtime cause remains
unproven.

The recommended separate next proposal is a bounded synthetic-only diagnostic
reader contract, preserving admission/custody checks and all existing ceilings.
No spent collector, workflow, policy, allowance or qualification flag changed.
