---
type: Qualification Audit
title: Pair native envelope callback and platform-input closure
description: Public callback/native-input inventory plus the completed Windows SQLite compiler component evidence; complete growth and scratch qualification remains unavailable.
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

Initial source-only audit method:

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

At that initial audit, no product/backend/allocator code, SQL/database opening,
compile/native/model probe or activation ran. That phase was source-only;
#1176 remains open and #740 remains closed research.

## Compile-only component result — 2026-11-07

The owner approved [#1480's exact plan](../superpowers/plans/2026-11-06-pair-sqlite-compile-evidence.md#scoped-execution-approval--2026-11-07).
[#1485](https://github.com/Rambolarsen/orkworks/issues/1485) tracks this attempt;
tooling [#1486](https://github.com/Rambolarsen/orkworks/pull/1486) merged as
24fbfc12ffc6571db98aa484b880aac13381ccc3. The reviewed workflow/driver bytes match
tooling head 8f8ffbfe6b5ff6a59c843d6d57080e3dbabd9e73. Compiler-free Windows tests,
four local workflow/argv checks and exact-head Codex review passed before dispatch.
Required CI passed after one ordinary failed-jobs rerun for two unchanged tests;
that CI rerun was not a qualification-workflow retry.

**The single attempt supplied useful compiler component evidence.** Run
[42352195449](https://github.com/Rambolarsen/orkworks/actions/runs/42352195449)
completed successfully: one Windows-2022 job, one compiler driver invocation,
zero target executions and compiler exit 0. Job time was 07:39:39–07:42:54 UTC
(195 seconds); the script recorded 07:39:48.4529531–07:42:43.1385396 UTC.
Within this qualification job, no linked executable, SQLite call, database,
model call, VFS/router installation or filesystem-growth experiment ran. The owned frozen run branch pointed to the
exact merged tooling head; no alternate ref/configuration or second dispatch ran.

### Build and evidence identity

| Item | Recorded value |
| --- | --- |
| Runner image | windows-2022 / 20261101.353.1 |
| Compiler file version | MSVC 19.44.35229.0, x64 host / x64 target |
| Compiler SHA-256 | fe251ef50a1545b1b0835ee17b1e785459712b38d79e45b5c1d3d28970a36619 |
| Windows SDK | 10.0.26100.0 |
| Source/configuration | Same pinned archive/amalgamation hashes above and in the plan; downloaded report argv exactly matches the approved 29 literal arguments, including source filename and 19 defines |
| Object | 4,602,277 bytes; reported SHA-256 95ab1c5638a33847c8f1e09e13081653f2cba6d8f0e7d8db52a0dfeca596b248; object was not uploaded or executed |
| Sanitized listing | 202,650,720 bytes; SHA-256 5dfd859e068c6690cc12c06fc6283e07cf8cf83d1dab75a86cc5c4183a651245 |
| JSON report | 2,052 bytes; SHA-256 1fe542872b4f50740104e4f0121b07d540964e5feaf7362c01577ddc9e7ce52a |
| Diagnostics | 11 bytes (source filename only); SHA-256 590d897d7a74113c659eb55273051f75edf017a333d4de6961e8b0b6fb4140e2 |
| GitHub artifact | [13565809579](https://github.com/Rambolarsen/orkworks/actions/runs/42352195449/artifacts/13565809579), 53,048,506 archive bytes, one-day remote retention |
| GitHub-reported artifact digest | sha256:fc251b62ab3292a726c88504554d5f82e6e28a700c1e0dff197ae905554fc2cf |

Only result.json, diagnostics.txt and sqlite3.asm were downloaded. Their byte
sizes and SHA-256 values were recomputed locally; report argv/counts/outcome and
identity shapes were checked. Compiler/object hashes are the controlled driver's
records, not a local rehash of retained binaries. No object or whole runner
environment was collected. The artifact archive digest is GitHub-reported;
the independent checks above apply to its three extracted files. This is a
candidate object compilation, not a linked production/backend image.

### Emitted helper bodies and own stack adjustments

All four standalone bodies are present. Line numbers below identify the exact
sanitized listing hash above, counting lines from 1. The byte column counts
only the body's explicit downward RSP adjustment from its entry value: 8 bytes
per pushed x64 register plus its fixed sub rsp instruction. It excludes
call-pushed return addresses, caller-provided home area and callee execution.

| Body / listing lines | Explicit prologue | Own RSP decrement |
| --- | --- | --- |
| sqlite3_randomness, 5009900–5010196 | push rdi; sub rsp,32 | 40 bytes |
| winRandomness, 4178624–4179064 | push rbp/rdi/r14; sub rsp,64 | 88 bytes |
| xorMemory, 4179073–4179127 | No push/sub of RSP; saves rbx/rsi/rdi at incoming rsp+8/+16/+24 and restores them | 0 additional bytes; uses 24 bytes of caller-provided home storage |
| chacha_block, 4212049–4212325 | push rbp/rbx/rsi/rdi/r12/r13/r14/r15; sub rsp,104 | 168 bytes |

The corresponding epilogues restore these adjustments. Inspection of each
complete emitted body found no variable RSP adjustment, outgoing tail transfer
or self-recursive edge. xorMemory and chacha_block have no outgoing calls in
these bodies. Their local loops do not repeatedly allocate another frame.

These numbers are **not S**, total stack consumption or a zero-scratch claim
for xorMemory. Microsoft documents the distinct roles of
[prologue/epilogue allocation](https://learn.microsoft.com/en-us/cpp/build/prolog-and-epilog?view=msvc-170)
and [caller parameter/home storage and call-pushed return addresses](https://learn.microsoft.com/en-us/cpp/build/stack-usage?view=msvc-170).
Some nonvolatile saves use the incoming home area, so counting every store as a
new allocation would double-count caller storage. Callee work, calling ancestry,
alignment/stack reservation and non-stack workspaces remain separate obligations.

### Actual call edges, inlining and remaining boundaries

| Emitted body | Outgoing edges / relevant observations |
| --- | --- |
| sqlite3_randomness | Calls sqlite3_initialize, sqlite3_vfs_find, memcpy (two sites), chacha_block, mutex callbacks through sqlite3Config+112/+128/+144, and an indirect VFS callback. Lines5010058–5010061 load the callback from rcx+104, set edx=44 and call rax: the default-VFS seed path remains in this candidate |
| winRandomness | Calls memset, four aSyscall slots and __security_check_cookie. The source entropy-mixing operations are emitted inline here; there is no call to the separate xorMemory body |
| xorMemory | Standalone body retained, although this winRandomness uses inline copies; register-only loop state plus its caller-home saves above |
| chacha_block | Called by sqlite3_randomness; memcpy(x,in,64) is emitted as vector loads/stores, with no outgoing memcpy or other call in this body |

The four indirect Windows slot offsets are 704, 416, 800 and 1232, at listing
lines4178664,4178787,4178869,4178972. In the pinned source, win_syscall has three
pointer fields and pCurrent is its second field. The x64 24-byte entries map
these offsets to indices29,17,33,51: default GetSystemTime, GetCurrentProcessId,
GetTickCount and QueryPerformanceCounter respectively (source macros at
lines47406,47305,47439,47577). This identifies emitted dispatch slots and their
source defaults; no Windows API or future runtime callback target was executed
or qualified. In particular, native import and CRT/helper workspace costs were
not measured or bounded by this compilation.

Inlining is therefore observed in specific call sites; no requested standalone
helper disappeared. The UUID-gated source branch (lines52575–52585) emits no
UUID calls in this candidate; the approved define set did not enable
SQLITE_WIN32_USE_UUID. This records the candidate's conditional branch, not a
new feature-disable setting. Conditional seed/mutex paths are still present. None of this
proves that initialization, PRNG reset, VFS selection, global mutex dispatch or
the native helpers can be omitted from a later closed-program analysis.

### Terminal disposition

This completes the approved compiler-evidence attempt: dispatch **1/1** and
compiler invocation **1/1**, with no retry/fallback or automatic next probe.
The prior Windows native handle-probe budget remains separately consumed.

The new evidence replaces guessed source-local stack sizes for these four
emitted bodies. It leaves the complete call graph/native callees, Rust/private
backend frames, F/P/R/L, arena layout/lifetimes and pre-operation native-growth
envelope unresolved. No numerical manifest, arena capacity, maximum-profile fit,
eligible backend or runtime activation follows. The unchanged first gate remains
open and the candidate remains unavailable; #1176 stays open and #740 stays
closed research. Further work must justify an identified missing bound within
the accepted gate; this result grants no new execution budget.

One bounded independent read-only review rehashed the three downloaded files and
the pinned source, checked the 29 arguments, complete helper bodies, stack
arithmetic, call/inlining facts and native slot mapping. It found no factual
error or promotion to full qualification; its small conditional-branch coverage
omission is resolved by the explicit UUID note above. No second compilation or
probe ran for review.

## NTFS allocation-rounding component — 2026-11-07

Tracking: [#1496](https://github.com/Rambolarsen/orkworks/issues/1496). Inspection
base: c2af02f9c7f047e65fc7ed74575500d594af0238. This source-only follow-up derives
an alignment component of file growth. **The complete pre-operation growth
envelope remains unavailable.** No native/compile/model probe or database ran.

### Contracts and the quantity they describe

[MS-FSCC FileAllocationInformation](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/0201c69b-50db-412d-bab3-dd97aeede13b)
describes cluster alignment for nonresident allocations. Its
[NTFS product note102](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/d4bc551b-7aaf-4b4f-ba0e-3a75e7c528f0)
specifies 8-byte resident rounding, retention of residency on allocation shrink,
and conversion to cluster-aligned nonresident storage when resident allocation
is extended. Its last-handle-close rule also means unused preallocation is not
a permanent reservation after closing all handles.

The [MS-FSA allocation-set algorithm](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fsa/3820daf2-e512-4730-8cce-2d0435ae6ad2)
computes a cluster-aligned requested size, checks storage reservation before
assigning the abstract allocation field, and includes early-success paths for
an unchanged allocation or a deleted stream. Status alone therefore cannot
establish a new field value or physical reclamation. Its
[BlockAlign definition](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fsa/6e365e98-7cb0-471b-a50e-5b56461b33d8)
requires a power-of-two boundary and gives the exact alignment calculation.

These are useful operation/state contracts. The
[Microsoft protocol catalog](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-winprotlp/e36c976a-6263-42a8-b119-7a3cc41ddd2a)
identifies FSA as an abstract object-store model supporting SMB, and FSCC as
network representations of native structures. FSCC's product note88 maps Windows
processing to NtQueryInformationFile/NtSetInformationFile but explicitly leaves
the processing function's implementation outside the protocol. Neither abstract
field assignment nor the absence of an intermediate model state proves the
local NTFS implementation's peak physical work/allocation.

Let C be the positive cluster size of the **actual admitted volume**, not the
SQLite page size. The native
[volume-query interface](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntqueryvolumeinformationfile)
takes an existing file/directory/volume handle;
[FILE_FS_SIZE_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddk/ns-ntddk-_file_fs_size_information)
provides sectors per allocation unit and bytes per sector. Their checked product
can supply geometry associated with that handle. This supplies neither source
authority nor a disk-space reservation; no query was executed in this audit.

### Conditional alignment and shared-limit arithmetic

For nonnegative allocation request q and validated power-of-two C divisible by8:

    Q_C(q) = C * (q div C + indicator(q mod C != 0))
    Q_8(q) = 8 * (q div 8 + indicator(q mod 8 != 0))

All input conversion, multiplication, addition and requested I/O end calculations
must be checked in the eventual native ABI's integer range. The division form
avoids blindly computing q+C-1 near an integer limit.

For the documented nonresident alignment component, Q_C is the rounded size.
Resident shrink uses Q_8; resident growth converts to the nonresident case.
Since C is a multiple of8, Q_8(q) <= Q_C(q): Q_C is itself an 8-aligned value
at least q. Thus Q_C conservatively covers these **rounding components** without
guessing which residency class is present. It does not bound all representations,
native overhead or transient work.

Let B=2,147,483,648 bytes be the existing shared auxiliary ceiling. Let K include
all other owned/pending/uncertain charges, excluding this file's entire charge.
Let u be this file's entire previously retained charge, counted once. A
conservative rounding-and-retention allowance, not a native-peak bound, is:

    D(q,C,u) = max(u, Q_C(q))
    K + D(q,C,u) <= B

This preserves the old charge while covering the rounding component. Passing
covers only those components; failure permits conservative refusal, not a claim
that every resident-shrink case physically exceeds B. For illustration only,
C=4096, u=4096 and K=B-4096 give:

| q (bytes) | Q_C(q)=D here | K + D | Rounded-allowance check |
| --- | --- | --- | --- |
| 4095 | 4096 | 2,147,483,648 | Fits this component only |
| 4096 | 4096 | 2,147,483,648 | Fits this component only |
| 4097 | 8192 | 2,147,487,744 | Allowance exceeds the shared ceiling |

Likewise, a hypothetical resident allocation8 extended by requesting9 converts
to a 4096-byte nonresident component on that illustrative volume; using resident
rounding16 for that extension would be wrong. These calculations were recomputed
as pure integer arithmetic. They select no cluster default, per-file quota,
residency threshold or qualified volume/build. The ordinary main file is not
assigned this auxiliary ceiling.

### Why this does not close the first gate

Admission still needs K+E(q,state,operation)<=B, where E conservatively covers
the current file's retained charge and **whole-operation peak** under the
accepted accounting contract. The reviewed sources do not establish that
E=D(q,C,u) or E=Q_C(q), nor justify a fixed old-plus-new copy count for conversion, creation,
write/preallocation, recovery or failed/uncertain completion. A smaller settled
allocation cannot refund the original retained charge without reclamation proof.

A successful allocation setter also supplies no permanent cap on later writes:
[NtWriteFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/content/ntifs/nf-ntifs-ntwritefile)
can extend EOF. A future closed I/O sequence would still need its pre-operation
bound for every allowed write/truncate/hint and ownership/error transition.
No such sequence or backend implementation is selected by this derivation.

The finite result is a closed conditional rounding calculation and an identified
missing bridge from abstract allocation state to the applicable native peak.
Under the accepted first gate, growth qualification stays unavailable on this
evidence. It is not a proof that a supported bound is impossible. Another small
measurement cannot substitute for that missing upper-bound argument.

The shared2GiB auxiliary / separate8MiB scratch ceilings, complete-S gate,
accepted architecture and owner/recovery rules remain unchanged. Both earlier
execution batches stay consumed. No new probe budget, dependent implementation
plan, quota, profile eligibility or activation follows; #1176 remains open and
#740 remains closed research.
