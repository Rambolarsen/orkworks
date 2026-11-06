---
type: Qualification Audit
title: Pair native envelope callback and platform-input closure
description: Complete public callback inventory, global helper paths and documented Windows size semantics for the accepted pair VFS/router envelope gate, with numerical fit still unavailable.
tags: [orkworks, hierarchy, sqlite, windows, memory, qualification]
status: draft
workflow_status: complete-envelope-unavailable
---

# Pair native envelope callback and platform-input closure

Tracking: [#1474](https://github.com/Rambolarsen/orkworks/issues/1474),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Inspection base:
`cb1d57c4a88e3b89e1ca2582848c9020f7ea341e`.

## Disposition and qualification method

The VFS/router/pair-arena direction is already
[accepted for further qualification](../superpowers/specs/2026-11-06-pair-native-enforcement-design.md#scoped-architecture-acceptance--2026-11-06),
recorded by [#1473](https://github.com/Rambolarsen/orkworks/pull/1473) and
[ADR 0112](../adr/0112-pair-native-enforcement.md). No repeat architecture decision
is needed. This record advances its first envelope gate without implementing or
executing the proposed backend.

The source inventory covers all 41 function-pointer slots in the pinned public
VFS/file/allocator structures: 16 VFS, 18 file methods and 7 memory methods.
These are interface slots, not active callback counts, runtime samples or new
capacity defaults. An entry may be unreachable under the selected ABI/profile,
but that requires an explicit reachability/configuration argument. No method is
omitted because it is expected to do little work.

**Complete numerical growth/scratch qualification remains unavailable.** This
record closes the callback-class inventory and distinguishes controlled input
sizes from missing build/native envelopes. It does not assign F/P/R/L/S, select
an arena capacity, prove a valid command fits or declare the architecture
impossible. The next evidence must provide the actual callback/native layout and
closed build/call graph; another sample showing small usage cannot replace it.

Method, performed read-only:

1. Extract public callback members from the exact pinned amalgamation and map
   every slot to accepted behavior and pre-operation accounting obligations.
2. Trace allocation and global/default-VFS helper paths that connection selection
   and TLS alone do not close; compare documented Windows size/stack semantics.
3. Preserve the existing source-derived equations and mark numerical/native
   terms unknown when the available source cannot justify their bound. No native
   measurement or guessed numeric profile is substituted.

Source: SQLite 3.46.0 / libsqlite3-sys 0.30.1, `sqlite3/sqlite3.c`, SHA-256
`c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d`.
Names/lines below refer to that file from the [pinned crate](https://crates.io/crates/libsqlite3-sys/0.30.1).
The [first derivation](pair-native-envelope-derivation.md) already owns the
super-journal allocation/name-list arithmetic; it is linked rather than copied
or independently reapproved here.

## Public callback inventory

The [VFS](https://www.sqlite.org/c3ref/vfs.html),
[file-method](https://www.sqlite.org/c3ref/io_methods.html) and
[allocator](https://www.sqlite.org/c3ref/mem_methods.html) interfaces describe
these slots. Grouping shares an accounting obligation, not an implementation.

| Public VFS slots (16) | Accepted-profile obligation / missing envelope input |
| --- | --- |
| xOpen | Reserve object/inventory/name/handle and creation/probe resources before native work; include failed-open and later close lifetime |
| xDelete | Only original-owned auxiliary disposition; include native callback/unknown/deferred-close storage before deletion and until proven reclamation |
| xAccess | Bound exact rooted probes and output; absence is an observed result, not path authority or free probe cost |
| xFullPathname | Bound source and output encoding/conversion/copy workspace; declared mxPathname is not itself its storage proof |
| xDlOpen, xDlError, xDlSym, xDlClose | Extension loading is excluded. Include bounded rejection/error/cleanup behavior for any callable slot; exclusion is not permission to leave an unsafe path |
| xRandomness | Bound supplied output and helper stack/native work; also account for the global default-VFS path below |
| xSleep | Bound callback state and native sleep helper, without blocking authority latches; requested delay is not an allocation ceiling |
| xCurrentTime, xCurrentTimeInt64 | Bound scalar outputs plus helper frames/native work; pin the actual available version/fallback |
| xGetLastError | Bound error-buffer writes and any conversion/storage; no unbounded path/error formatting |
| xSetSystemCall, xGetSystemCall, xNextSystemCall | No runtime syscall-hook selection is accepted. Declare whether slots are absent or bounded/rejecting; the optional v3 surface is not a reason to change global native dispatch |

| Public file slots (18) | Accepted-profile obligation / missing envelope input |
| --- | --- |
| xClose | Drain engine reference only; original native object/charges may remain. Include close after failed xOpen when pMethods is non-NULL |
| xRead | Validate offset/length and original input association before work; caller buffer, short-read handling and native/temporary workspace remain charged |
| xWrite | Checked end offset plus the complete native peak-growth reservation before I/O, with source buffer and failed/partial completion retained |
| xTruncate | Include growth as well as shrink behavior, rounding and uncertain capacity; do not refund merely because requested length decreased |
| xSync | Account callback/native workspace and original in-flight lifetime; successful return is not by itself full paired/directory/power-loss durability |
| xFileSize | Exact reported size and failures, not a fabricated smaller admitted size; any use to size a later allocation needs its own admission |
| xLock, xUnlock, xCheckReservedLock | Actual public locking semantics, native structures and retained operation state; no no-op success based on a lease |
| xFileControl | Closed opcode/argument inventory, including callback allocations and file-size hints; unsupported optional operations cannot bypass admission |
| xSectorSize, xDeviceCharacteristics | Fixed outputs require justified target/build claims; they are not proof of native allocation quantum or peak scratch |
| xShmMap, xShmLock, xShmBarrier, xShmUnmap | WAL/shared-memory is excluded. Prove ABI/reachability or safe rejection/cleanup, without creating uncharged mappings |
| xFetch, xUnfetch | mmap is disabled. Include version/reachability and cleanup evidence; no mapped memory outside the accounting model |

| Public allocator slots (7) | Accepted-profile obligation / missing envelope input |
| --- | --- |
| xMalloc | Validate/round/admit actual arena cost before allocation for every entry path; no fallback to ordinary allocation |
| xRealloc | Classify the old owner and reserve simultaneous old/new/copy cost before growth; preserve the old allocation on failure; no cross-pool migration |
| xFree | Classify by original ownership, including another calling thread; prove block release and retained registration, not request lifetime |
| xSize | Report actual usable allocation according to the ABI; distinguish reported payload from complete arena charge/bookkeeping |
| xRoundup | Checked alignment/overflow and deterministic actual costs; it is not the sole admission point for all source paths |
| xInit, xShutdown | Process-global lifecycle and saved-allocator behavior; prove single initialization, routing context and complete live-reference drain without resetting other connections |

This is exhaustive only for these public structures through their pinned versions.
It is not an exhaustive list of native helpers, SQLite internal call sites,
argument-dependent xFileControl branches or concurrent engine allocations. Their
call graphs and overlap/recursion/error paths must still close L/S.

## Paths not closed by connection-local VFS or TLS

### Process-global randomness

`sqlite3_randomness`, lines33962–34030, initializes its process-global generator
through `sqlite3_vfs_find(0)` and `sqlite3OsRandomness`, not a connection's pVfs.
The seed request on this path is 44 bytes into the global state, not an arena or
complete stack bound. Its reset condition also exists in the pinned function;
initializing once is not a general proof that every later call skips the helper.

The selected default stock `winRandomness`,52537–52587, uses local entropy/time/
process/counter objects and compile-conditional native helpers. Which helpers and
frames exist depends on the pinned build. This is not evidence of uncontrolled
file writes or that the chosen private VFS is bypassed for main-file ownership.
It is evidence that the scratch call graph must include the default-VFS helper
when reachable. Replacing the global default VFS or taking the closed #1470
isolated-library alternative is not selected as an implicit fix.

### Allocator and external cache paths

`mallocWithAlarm` / `sqlite3Malloc`,30464–30545, have configuration-dependent
routes: the memory-statistics path calls xRoundup before xMalloc, while the
non-statistics branch calls xMalloc directly. The router must admit its own
actual aligned/metadata cost rather than assume one upstream rounding path.
No private-field access or forced global memory-statistics change is selected.

`pcache1Alloc`,55497–55530, can obtain a page from a configured external page-cache
pool before falling back to sqlite3Malloc. A successful router installation is
therefore not alone proof that every live SQL page lies in its arena. The accepted
requirement already covers external lookaside/cache buffers: classify/admit the
actual configuration or deny the profile. Callback inventory is not a zero-cost
assumption for globals, native workspaces or cache state.

## Windows growth requests, state and observations

| Documented interface | What the contract supplies | What the complete envelope must additionally establish |
| --- | --- | --- |
| [NtCreateFile AllocationSize](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile) | Initial allocation request for create/overwrite/supersede; a nonzero request has no effect on a plain existing open | Allowed creation class/attributes, native rounded/peak allocation and failed/unknown results; do not treat this parameter as a permanent future-write quota |
| [FILE_ALLOCATION_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_allocation_info) | Sets a new allocation size, normally expressed in device sector/cluster units; setting it below EOF also adjusts EOF | This is an actual mutator, not just observation. Reserve before calling it; prove the selected operation/build's complete peak and rounding envelope, later writes and error/lifetime behavior |
| [FILE_END_OF_FILE_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_end_of_file_info) | Sets the file's EOF | Do not substitute requested EOF for the full native capacity/peak allocation proof |
| [FILE_STANDARD_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_standard_info) | Reports allocation, EOF, links, delete-pending and directory state on the actual handle | Useful for validating known state/disposition, but a later report cannot retroactively admit earlier excess work or prove reclamation solely from a missing name |
| [WriteFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-writefile) | Performs the requested write and defines buffer/completion obligations | Bound checked end, caller/OS workspace, completion/partial failure, simultaneous/uncertain native capacity and selected filesystem semantics before work |

These public contracts identify candidate controls and required observations.
This inspection has not derived a complete conservative upper-bound function for
all the selected operations/attributes and failure/recovery paths. It does not
prove that no such bound or supported sequence can be established. Neither
post-operation sampling nor calling an allocation setter is substituted for the
missing whole-operation proof.

## Stack and remaining manifest inputs

The ordinary [worker launcher](../../crates/orkworksd/src/session_store/worker.rs)
at this base names/spawns its thread without an explicit stack_size selection.
It is not the new backend's closed build image. Rust's
[Builder::stack_size](https://doc.rust-lang.org/std/thread/struct.Builder.html#method.stack_size)
allows an actual size above a request when the platform requires a minimum.
Windows documents [reserved/committed stacks and rounding](https://learn.microsoft.com/en-us/windows/win32/procthread/thread-stack-size).
No Rust request, linker default or small observed peak is silently substituted
for the complete applicable stack/native/callback portion of S.

| Term | Controlled facts now | Evidence still required |
| --- | --- | --- |
| F/P and callback state | Public object/filename interfaces and complete slot inventory | Actual private object/encoding/handle layout, selected ABI versions and checked maxima; no defaults imported from #1470 |
| R and arena state | Accepted no-growth/no-fallback/realloc ownership rules; relevant source entry paths above | Exact allocator block/rounding/bookkeeping layout and peak coexistence, including error/free/retirement paths |
| L | The first derivation's two recovery blocks and this callback/global/cache inventory | All other simultaneous engine/callback/owner allocations on the complete closed program, not a sum or maximum of guessed isolated cases |
| S | Native API/call classes, ordinary launcher and documented stack distinctions | Exact closed build/native helper frames, recursion/stack reservation/rounding, buffers and other applicable out-of-arena workspaces and lifetime |
| Native growth | Allocation setters, explicit offsets/EOF and actual handle observations are available | Qualified pre-operation peak/rounding/unknown/recovery/reclamation envelope for each allowed operation and complete shared aggregate |

The first record's equations remain binding and unevaluated. No unknown term is
zero, no callback count is a new allowance and no arena capacity has been chosen.
The approved architecture is not reopened. #1470's library-copy/count defaults
remain unselected. A dependent implementation plan cannot assume a qualified
manifest from these source facts. Any implementation/compile/native work needed
to produce missing evidence requires its concrete bounded plan and applicable
authorization; the prior native attempt budget remains fully consumed.

No product/backend/allocator code, SQL/database opening, compile/native/model
probe or activation ran. This is source/documentation qualification only;
#1176 remains open and #740 remains closed research.
