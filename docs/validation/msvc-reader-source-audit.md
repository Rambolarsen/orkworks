---
type: Source Audit
title: MSVC reader work and the missing within-root checkpoint
description: Traces frozen MSVC manifest operations and observation limits without another installed-input experiment.
tags: [orkworks, qualification, windows, diagnostics]
status: stable
workflow_status: source-audit-runtime-cause-unproven
---

# MSVC reader work and the missing within-root checkpoint

Tracking [#2253](https://github.com/Rambolarsen/orkworks/issues/2253), a prerequisite
of [#1176](https://github.com/Rambolarsen/orkworks/issues/1176). This audit reads Git
source frozen at d870796b634a0f8259d0c418fb0455ea0079a8a1 and the
[merged terminal record](live-reader-progress.md#sole-candidate-attempt-terminal-unavailable-2026-11-28).
It does not reacquire the hosted artifact, inspect installed inputs, import or
replay the collector, execute native tools, change code or renew an allowance.
The reader, controller, codec and policy bytes in this audit checkout equal the
frozen blobs; citations below use that immutable commit.

**Source conclusion:** the MSVC interval contains repeated ancestry/identity
checks, one payload hash per admitted file and per-row JSON conversion. It does
not request PE/version inspection. Its only normal progress boundaries enclose
the whole root, beginning before the root's initial path checks. The recorded
prefix cannot identify which operation or file consumed time, or whether the
first directory/file completed. Buffer reuse leaves these other costs in place;
this audit does not establish their runtime share or a performance fix.

## What the last observation actually binds

The terminal record reports archives-end at parent 37,084 ms with a=257, then
root-begin/msvc at 37,100 ms with h=2,176,513,082 and f=d=0. There is no validated
root-end or terminal tuple. The before reader reached its 120-second child limit;
the parent later records termination, final source work and publication. These
are arrival observations, not CPU/I/O timing or an inventory.

The root selector is exactly
C:/Program Files/Microsoft Visual Studio/2022/Enterprise/VC/Tools/MSVC/14.44.35207.
It selects the whole named toolset root. The manifest loop has no extension,
architecture or subdirectory filter for it. The only special exclusion is .git
when label=source, which does not apply to msvc. No installed subtree membership
or size follows from the selector. Sources:
[frozen policy](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.inputs.json),
[fixed root order and entry handling, lines136–175](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L136).

Before ready, Assert-ReaderRoots already opens identities for all seven roots
and the registry-cache parent. That control-path admission is distinct from
walking their payloads. Archive verification then precedes Get-SealedManifest.
Its first root emits root-begin before Assert-PlainPath, stack creation, the
first directory identity, row encoding and enumeration. Thus the last row does
not prove even that first directory identity completed. Sources:
[root admission, lines295–299](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L295),
[archive-to-manifest call, lines270–279](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L270),
[root entry, lines138–150](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L138).

The file/directory counters increment only after their rows pass the encoded
limit and join the lists. They are transmitted only at the next Emit-Progress
call. Retained tuples are immutable snapshots; later increments do not update
an earlier row. A timeout can kill the child before the cooperative error path
emits counters. Therefore f=d=0 in root-begin is not a zero-work observation.
Sources: [row admission, lines145–173](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L145),
[emission, lines301–308](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L301),
[parent snapshots, lines71–89](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build-protocol.mjs#L71),
[cooperative catch, lines344–352](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L344).

## Operations inside the MSVC interval

The following counts describe successful source paths, not measured disk reads,
cache misses, allocator/RSS effects or elapsed costs. Framework calls may cache
metadata or perform additional internal work. A throw/timeout can stop at any
point; no completed F/D counts are available for this attempt.

| Unit | Explicit source work | Custody/boundary purpose |
| --- | --- | --- |
| Root entry | One Assert-PlainPath ancestry walk before the root stack is populated | Reject non-local/ADS paths and reparse ancestry before traversal |
| Each popped directory | A fresh ancestry walk, one explicit CreateFileW, one Get-HandleInfo with two native queries, then close; one directory row precharge/JSON-size conversion and enumeration | Bind directory identity and reject linked traversal; cap admitted rows before retention |
| Each enumerated entry | Name/relative-path construction, path-length test and one GetAttributes before classification | Reject reparse entries; enforce directory queue or file limits |
| Each completed regular-file sealing call | Two ancestry walks, two FileStream opens, three Get-HandleInfo calls (six explicit native queries), one new IncrementalHash and one exclusive payload-buffer lease | Hash held bytes, compare before/after/current name identity, size and write marker |
| Each completed file row | One row precharge, ConvertTo-Json, UTF-8 byte count and list insertion | Preserve encoded manifest bound and completed-row counters |

Sources: [plain paths and handle queries, lines59–85](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L59),
[sealed file, lines87–134](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L87),
[manifest loop, lines136–175](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L136).

For a completed ordinary file, the manifest obtains one FileInfo.Length before
calling Read-SealedFile. That function creates another FileInfo, tests Exists
and Length, then reads Length again into expected. This is two explicit FileInfo
constructions, one Exists access and three Length accesses across the two
functions; it is not a claim of six OS queries. Its three handle-info calls each
allocate a StringBuilder with capacity 4,096 characters. The reusable 1 MiB byte
array does not eliminate these builders, file rows, hash state, lease wrappers,
path strings or temporary JSON strings. Payload allocation metrics exclude
those allocations. Sources: [length/lease/hash work, lines87–110](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L87),
[builder allocation, lines70–80](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L70),
[pre-call length, lines161–169](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L161).

Let F be files whose ordinary sealing completed, D directories whose identity
checks completed, E entry-classification GetAttributes calls, and d(x) the number
of ancestry-loop iterations for a path. On a fully completed MSVC-root walk,
explicit GetAttributes calls from these sites total
**d(root) + sum over directories d(dir) + E + 2 × sum over files d(file)**.
Explicit native P/Invoke calls are **3D + 6F**, excluding FileStream/framework
internals and all earlier ready/archive/preflight work. Get-HandleInfo builder
constructions are **D + 3F**. These symbolic counts cannot be evaluated from the
retained prefix. They must not be presented as syscall counts, physical reads
or a latency estimate.

For a successfully fully read file of s bytes, requests are at most 1 MiB each:
ceil(s/1 MiB) reads when requests are fulfilled, with more calls possible for
positive short reads. Empty files still create hash state and acquire/release a
lease. MSVC rows call Read-SealedFile without -PE or -Text; consequently the
FileVersionInfo/PE metadata branch and text-capture MemoryStream branch are
outside this walk. The separate preflight uses -PE for twelve selected paths.
Sources: [read loop and optional branches, lines99–127](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L99),
[ordinary manifest call, line164](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L164),
[separate PE preflight, lines336–343](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L336).

## Logical debit, output work and time are different bounds

Each ordinary file length is precharged before its held read. Each directory/file
row also conservatively charges 512 + 6 × the combined .NET string lengths of
root label, relative path and identity before row JSON conversion. For msvc
(length 4) and the 25-character identity, that is **686 + 6L bytes per row**, where
L is the relative path's UTF-16 code-unit length. This is a logical encoding
precharge, not actual UTF-8 length or physical hash throughput. A directory row
has no payload hash. Sources: [encoding rule, lines51–57](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L51),
[file precharge, lines89–95](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L89),
[row charges, lines145–169](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L145).

The input ceilings are shared across all seven manifest roots, not a separate
MSVC budget. With the recorded 257 archive packages, the frozen call passes
file limit 199,743 and directory limit 49,998; these are configured admission
bounds, not observed file/directory totals. Encoded space also subtracts the
archive report's encoded bytes and 4,096 from 64 MiB; the exact archive encoded
size was not retained. Full file bytes remain subject to 12 GiB including archive
bytes and 512 MiB per file. Sources: [call limits, lines270–279](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L270),
[shared lists/limits, lines136–169](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L136).

Per-row JSON conversion is inside the root interval. Global Sort-Object of the
complete file/directory lists is after all root-end emissions. The complete
output-envelope serialization, UTF-8 encoding, exclusive write and flush are
later still. Those later operations cannot be treated as work located inside
the source interval between root-begin/msvc and root-end/msvc; the prefix does
not establish that any of them was reached. Sources:
[end-of-walk sorting, lines173–175](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L173),
[output writer, lines281–293](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.ps1#L281).

The parent computes one child end before spawn, min(job active deadline,
current monotonic time + 120 s), and never resets it at a phase/root boundary.
Startup, root admission, archive verification and the root walks share it.
There is no wall-clock check inside Read-SealedFile; its checks concern work,
format and identity. The parent enforces time asynchronously, latches failure,
uses the bounded owned termination/drain path, and retains the prefix. An arrival
gap or parent end time includes scheduling/transport and later cleanup; it is
not a measured duration for a particular native file or operation. Sources:
[parent runner, lines298–334](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.mjs#L298),
[reader timeout/settlement, lines782–807](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.mjs#L782).

The missing terminal still spends the entire delegated grant. Progress never
admits a partial inventory or refunds unknown work. This audit changes neither
that rule nor the 132 MiB parent share of the same 32 GiB pool. Source:
[closed settlement, lines99–106](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build-protocol.mjs#L99).

## Bounded options for a separate decision

| Option | What source/evidence supports now | What remains gated |
| --- | --- | --- |
| Keep the terminal unavailable result | A valid outcome under the accepted one-attempt package | No new work or admission follows |
| Prepare a within-root diagnostic proposal | The present two-boundary root interval cannot distinguish initial identity, enumeration, file sealing or row conversion | A concrete new representation, retention/debit accounting, refusal fixtures and one scoped owner decision before implementation; any native experiment also needs a new finite allowance |
| Assess an implementation change in a future proposal | Repeated managed/native crossings, ancestry work and row conversion remain visible cost candidates after payload-buffer reuse | No speed/RSS claim without discriminating evidence; preserve complete identities, ancestry, archive/helper closure and all pre-effect/debit checks; removing a pass or caching by path needs its own custody argument |

The current v2 success sequence already uses all 21 records. Its decoder checks
exact phase order and rejects extra records/unknown phases; even a syntactically
valid heartbeat cannot be inserted as an incidental logging change. The same
192-byte line, 4,096-byte child report, 18 KiB retained timeline and 64 KiB result
constraints remain. A separate proposal must explicitly resolve compatibility,
root/subphase meanings, sparse counter sampling and saturation, parent arrivals,
late/failed-close handling, source/output admission before known settlement and
what its bounded evidence could discriminate. It must not invent a successful
root-end or reuse the earlier grant. Sources:
[v2 phases and count/order checks, lines4–9 and71–88](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build-protocol.mjs#L4),
[accepted boundaries](../superpowers/specs/2026-11-27-live-reader-integration-design.md#request-roots-and-reader-wire-domain).

Least confidence remains the runtime distribution inside the unobserved root.
This source audit cannot choose between slow metadata, payload reads/hashing,
PowerShell/JSON overhead, scheduling or other native behavior. It cannot say
that a proposed optimization fits 120 s, that a larger quota is needed, or why the
older uninstrumented attempt timed out. No code, pin, codec, quota, workflow,
installer or production behavior changes here. All allowances remain spent;
Gate 1 is Investigate and all source/compatibility/bootstrap/native/recovery/
durability/activation gates remain open. The audit closes only the source-only
#2253 deliverable; a separate proposal remains the next decision.
