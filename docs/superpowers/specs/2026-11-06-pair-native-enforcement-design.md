---
type: Design Proposal
title: Pair-local native namespace and resource enforcement
description: Accepted public SQLite VFS and pair-routed fixed-arena direction for further qualification under unchanged limits; complete envelopes and implementation remain gated.
tags: [orkworks, hierarchy, windows, sqlite, ownership, resources]
status: draft
workflow_status: accepted-envelope-qualification-pending
---

# Pair-local native namespace and resource enforcement

Tracking: [#1467](https://github.com/Rambolarsen/orkworks/issues/1467),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Design base:
`19b6a2eb4a98aab1f31a507a903fc6cbe3353a10`.

## Proposed decision

The [namespace/resource audit](../../validation/windows-pair-namespace-resource-audit.md)
finds no pair owner before stock win32 auxiliary operations and no pair-local SQL
scratch allocator. The prior [six-case result](../../validation/windows-pair-handle-component-evidence.md)
remains positive evidence only for its stock-win32 handle/sharing experiment.

**Author recommendation:** keep SQLite 3.46.0, the same ordinary worker, one attached
rollback transaction and the accepted PairSlot/PairCore lifetime; introduce one
private Windows pair VFS implementing the public SQLite file interface, plus a
startup-installed SQLite allocator router with a fixed arena for that worker.
The VFS owns its native operations rather than delegating unknown pathname opens
or deletes to stock win32. No engine-private layout, native handle replacement,
process-wide Windows syscall hook, alternate engine, child process or new durable
application journal is selected.

This explicitly proposes amending the accepted **no custom VFS/shims** restriction
for this one scoped backend. It also proposes a process-wide allocator integration
point with pair-local routing; it is not an already-approved ordinary setting.
Both choices need one scoped owner approval of this concrete design before any
dependent implementation. All earlier owner/logical/source decisions remain
accepted; this proposal does not reopen them or grant another probe budget.

| Alternative | Disposition |
| --- | --- |
| Stock profile plus a complete conservative closed-command/recovery bound | Could satisfy admission, but no such bound exists for all selected commands, existing recovery inputs and scratch. Do not pretend the toy workload supplied it |
| Settings, progress hooks or post-operation samples | Insufficient for the audit's pre-operation requirements |
| Isolated SQL process or filesystem/quota container | Not selected: adds process/IPC/lifecycle and ordinary availability changes beyond the existing worker |
| Public pair VFS plus attributed fixed-arena SQL allocation | Recommend for further qualification: exposes actual I/O and allocation decisions while retaining the owner and engine transaction |

## Unchanged contracts and scope

Reuse the [accepted original owner](2026-11-06-original-pair-owner-design.md#scoped-owner-protocol-acceptance--2026-11-06)
and [joint bounds](2026-11-05-joint-child-preparation-design.md#worker-storage-recovery-and-bounds):
4,096-byte hierarchy pages, 1 GiB hierarchy file, **one shared 2 GiB auxiliary total**
and **separate 8 MiB SQL scratch**, together with all lower queue/intent/retained-
copy/handle/work budgets. The ordinary main file remains under its existing owner;
it is not assigned a new physical quota or absorbed into hierarchy history.
Conservative rejection and fragmentation can reduce availability; no maximum
valid logical record or command is promised to fit.

PairSlot stays the coordinator lifetime root and unique active-or-retiring
position. PairCore still moves once to the ordinary worker before native creation
and becomes Serving in place after a fully qualified CompleteNew. Requests own
none of its release. Created side effects followed by CompleteOld retain the
original core in StoppedOccupied; Unknown retains the original resources. No
restart-to-fresh, failed-reservation maintenance handoff or new serialized source
proof/token/epoch/ledger family is introduced.

The amendment is Windows/local NTFS only. Source authority, genuine-newness,
strict physical acceptance cutoffs, complete initializer/catalog effects,
directory/power-loss durability and paired recovery remain independent gates.
No proposal field or successful VFS operation constructs source permission.

## Private module and interface

Place `NativePairIo` inside the private `session_store::joint_preparation`
implementation. Its internal seams are the public SQLite VFS callbacks, the
native file adapter and the arena/router. Callers use the accepted sealed
original-plan/owner operations; they cannot supply SQL, file handles, filenames,
VFS names, affected lists, budgets or a claimed qualified profile.

A core-owned context joins exactly the original plan, selected root/lease,
ordinary/hierarchy identities, connection generation, inventory, resource
reservations and disposition. It exposes no general Connection. Only the actual
owning worker enters its scoped SQLite-call context; a callback outside that
worker/context, for another generation or during retirement denies. A private
VFS registration name is routing information, never a capability or bearer.

Registration/context storage is stable through every open sqlite3_file and
callback. Each file object references a core-owned inventory entry; SQLite
`xClose` releases that engine reference, not the PairCore or unique position.
The owner cannot unregister/free callback storage while any engine reference,
native operation, arena allocation or callback remains. Reentrancy and all FFI
callbacks obey SQLite's public ABI; no unwind crosses C.

## Rooted namespace and native objects

The original source/namespace owner must provide the actual selected storage
root association and genuine-new reservation. A path string or successful
CREATE_NEW cannot supply that authority. If the actual source owner/profile is
missing, construction remains unavailable.

The native adapter uses documented user-mode
[NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile):
walk one fixed component at a time relative to an already retained directory
handle through OBJECT_ATTRIBUTES.RootDirectory. Reject separators, parent
components, alternate streams and unselected device/volume/path forms at this
native seam. Each directory is opened as a directory/reparse object, checked
for a regular non-reparse local-NTFS association and retained; no intermediate
pathname traversal is delegated after selecting its parent handle. Reject
unsupported attributes/volume changes rather than normalize them into authority.

Open directory guards non-inherited, allowing required reads and omitting write
and delete sharing. This is a proposed stronger guard profile to qualify, not
an inference from the prior leaf rename case. Required native access rights must
support the selected rooted operations; unavailable access/sharing denies before
creation. Revalidate the selected association at the accepted transitions. This
does not add a sandbox against a privileged hostile process.

Create both main files with FILE_CREATE (exclusive new creation) under the held
rooted directories and retain their full FILE_ID_INFO tuples. Their identities
must differ. The new VFS's existing-only main opens bind those original core-owned
native objects to fixed ordinary/hierarchy roles; no replacement SQLite handle
or pathname reopen is used to manufacture the binding. Native file I/O and the
per-schema documented handle query refer to that actual owned object. Recheck
full identities/roles before mutation, disposition and reference publication.
The old stock-VFS experiment does not qualify this changed backend.

SQLite filenames are bounded internal names fixed by the core. `xFullPathname`,
`xOpen`, `xAccess` and `xDelete` recognize only those exact main roles, their fixed
rollback-journal names and the pinned SQLite-generated super-journal grammar.
The mapping supplies only a validated single native leaf relative to its retained
parent; it never forwards an arbitrary SQLite string as an OS path. Grammar,
length and integer overflow deny before native work. Additional attached stores,
WAL/shared memory, VACUUM, extensions or external SQL are not supported.

## Auxiliary ownership and allowed deletion

Reserve an inventory entry, naming bytes, native-handle capacity and applicable
byte headroom before an auxiliary create/probe. New journals/super-journals use
exclusive creation. An existing pathname is not adopted from its suffix or
membership in a directory. Reuse is limited to a still-owned entry of this exact
core/generation after identity/phase validation; an unknown collision denies.

For a generated super-journal candidate, `xAccess` may report absence only after
an admitted rooted existing-only probe proves it. An existing unowned collision
returns an error rather than inviting SQLite to delete it after its collision
loop. Independently, `xDelete` denies every name without an owned deletion entry.
A race after a proved absence still fails exclusive creation; it never opens the
colliding file as the new journal.

Auxiliary native objects are created with the DELETE right needed for their
future owned deletion and retained by the core. Their engine references use that
same owned object, so no later pathname reopen is needed to obtain deletion
permission. The adapter proposes handle-based disposition using documented
[SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle),
with no delete sharing for external reopens. Its exact NTFS/SQLite compatibility
must be qualified; no claim is made that closing a no-delete guard and deleting
by pathname would be equivalent.

| Entry phase | Meaning and release rule |
| --- | --- |
| Reserved/Probing | Resources reserved; no new file may yet be claimed. Failed probes release only resources proved unused |
| Owned/Open | Original handle, identity and live engine references retained; all growth charged before I/O |
| EngineClosed | Engine references drained; owner still retains object, namespace and charges. This is not permission for arbitrary deletion |
| DeletePending | Only a matching SQLite auxiliary-delete operation in the allowed transaction/disposition phase may mark the owned handle for deletion; keep charges through uncertain completion |
| Reclaimed | Engine references are zero, native close/deletion and original name disposition are proved; only then can physical capacity and that inventory slot be reused |
| Unknown | Retain object, maximum reserved growth, context and position; deny new work until supported original-owner disposition |

Main-file deletion is never a VFS operation in this profile. Authorized engine
journal finalization is not application cleanup of a failed fresh pair. Native
initialization side effects still obey StoppedOccupied. Physical capacity reuse
after proven auxiliary reclamation changes no logical ledger, spent read/work
credit, receipt, source permission or epoch.

## Admission before growth and recovery

Let `U` be the conservative charged allocation/high-water total of all owned
rollback and super-journal entries, `P` all pending growth reservations and `H`
retained uncertain capacity. Require `U + P + H <= 2 GiB`, using checked arithmetic
and counting each owned capacity once. Every class/name/handle/operation consumes
its applicable existing admission too; no counter is a new allowance.

Before `xWrite`, `xTruncate` growth or any allocation hint, compute the requested
end and a qualified build/filesystem growth envelope, including allocation
rounding/preallocation and simultaneous representations. Reserve its increase
before native work. Never assume logical EOF equals native allocation or perform
the I/O first and discover excess afterward. A missing conservative envelope,
unsupported attribute or unknown earlier size makes the profile unavailable.
Successful shrinking/refund needs proof of the reduced owned capacity; an error
retains the reserved upper bound unless the original owner proves no side effect.
A full journal cannot trigger fallback, untracked growth or deletion of recovery
evidence to make room.

Recovery is a separate data-only owner, not a path back into this original fresh
owner. It has to obtain its own accepted source/epoch/catalog/membership context
and admitted rooted inventory. Before SQLite reads an existing auxiliary or sizes
a recovery buffer, validate its native length, association, allowed name and
complete conservative read/allocation envelope. All child names later requested
by SQLite remain constrained to that inventory. No journal content may authorize
an arbitrary path or a live source. Oversized/foreign/ambiguous input denies;
there is no truncate-to-fit or hand-delete repair. This proposal does not supply
durable genuine-newness or restart authorization.

## Fixed SQL memory and process-wide routing

The public [sqlite3_mem_methods](https://www.sqlite.org/c3ref/mem_methods.html)
interface is process-wide. Propose installing a router with GETMALLOC/MALLOC
through [sqlite3_config](https://www.sqlite.org/c3ref/config.html) at single-threaded
startup before **any** SQLite initialization. Preserve the original allocator
for unpaired connections and process-global initialization. Installation after
initialization denies this profile; never call shutdown/reset on a live library,
replace another allocator, silently restart the sidecar or change a global heap
limit to force availability.

Before pair creation, reserve the full applicable 8 MiB scratch envelope. A trusted,
version-pinned build manifest must bound scratch outside the arena (`S`), including
any applicable stack/callback/native workspaces; the fixed arena capacity `A`
satisfies `A + S <= 8 MiB`. All arena bookkeeping, alignment, fragmentation,
connection/query caches, temporary buffers and peak realloc copies are inside
`A`. Count native/worker/handle overhead in its existing pools as well wherever
applicable. A manifest with an unknown `S` or allocation escape does not qualify;
this design does not assert that an 8 MiB heap alone bounds total SQL scratch.
The manifest is private qualified build data, not caller-provided evidence or
another numeric quota. No concrete platform fit is claimed here.

Route allocations made inside the actual pair worker's SQLite-call context to
its precommitted fixed arena. It never grows or delegates a failed allocation
to the ordinary allocator. Free/size/realloc classify pointers by registered
arena ownership rather than the current calling thread; arena-to-default realloc
migration is forbidden. Reallocation reserves both old/new storage before copy,
keeps the original allocation on failure, and reuses proven-free blocks only
inside the same admitted arena. Public alignment, xSize, xRoundup and allocator
initialization/free semantics must hold for both arena and delegated allocations.
Callbacks cannot recursively allocate through SQLite to maintain their own router.
An original-allocator pointer presented for growth inside a pair context is not
an escape hatch: deny that growth before allocation, preserving the old pointer.
An arena pointer presented outside its matching core context may be sized/freed
by its retained owner, but cannot grow or migrate into the ordinary allocator.
Such an unexpected escape invalidates qualification and retains its original
arena/position; it does not inherit the new calling thread's budget.

Unpaired allocations continue through the saved original allocator. Initialization
must establish the required global state before entering a pair context; SQL
background workers are disabled, mmap is disabled and only the closed command/
feature set is permitted. Set and verify temp_store=MEMORY, mmap_size=0 and zero
SQLite worker-thread allowance on the actual connection. Any lookaside/page-cache
or other allocation path outside the router must be covered by `A`/`S` or deny
the profile; custom external buffers cannot silently bypass attribution. Native file/SQL callback workspaces use the admitted
arena or the proven `S` allowance, never an untracked growing Rust/OS buffer.
Temporary SQL structures and statement journals must remain inside this memory
allowance; native temp/sort/sub-journal spill requests deny rather than escape
into a global temp directory. This conservative no-disk-spill restriction is
part of the proposed profile and may reject otherwise valid commands.

The router's arena registration remains stable until all engine allocations and
callbacks are gone. A pointer escaping into engine-global or another lifetime
keeps the original arena/PairSlot occupied; it cannot be freed, reused or charged
to the current request. Bounded clean retirement/absence of such escapes is an
explicit qualification requirement, not assumed from TLS. Existing ordinary
allocations must not inherit a pair budget or be reset when a pair retires.

## Compatibility, failure and qualification

This is a new private backend/build profile, not a file-format migration. Keep
schema2/format6, source digests, paired logical effects and the original engine
commit/disposition rules. The VFS must implement the public I/O, file-lock,
error/short-read, sync and callback contracts; a no-op locking/sync adapter or a
return of OK without native evidence is forbidden. OS handle/context closures
and all retirement remain outside core authority latches. Stop/input continues
independently of the SQL worker as already required.

A build/backend mismatch, startup allocator conflict, admission failure before
side effects, unsupported operation or invalid ownership denies. Native side
effects or ambiguous I/O keep the original resources and accepted old/new/unknown
classification; a quota error is not proof of complete rollback. Lost replies
create no new owner, refund, reopening or second connection. Preserve original
user data and existing ordinary storage/installer behavior until an explicitly
qualified profile is selected; install-time availability is not claimed.

The **first qualification gate after scoped owner acceptance** is deriving the
conservative native growth envelope and complete scratch `S` bound for the exact
build/closed feature set. Name every accounted unit, stack/native workspace,
rounding/spill representation and recovery input; distinguish file high-water
limits from any applicable native allocation costs without narrowing the existing
contract. Do not choose arena size or dependent backend implementation steps by
guess. If a supported finite bound cannot be justified within the unchanged
ceilings, record unavailable and stop this candidate; no implementation plan may
assume an invented qualified manifest. Analysis needing native execution requires
a separate explicit bounded plan/authorization, not a reuse of the consumed batch.

After that gate, before any executable publication plan, qualification must cover rooted component
and reparse behavior; exact actual I/O identities; native lock/sync semantics;
legitimate handle-based journal deletion and closed/unknown lifetimes; every
aggregate growth/rounding and recovery-input bound; arena ABI/OOM/realloc/free/
escape/startup behavior and unchanged unpaired allocation; deterministic below/
exact/above-limit denials before side effects; cancellation/lost replies and
retained old/new/unknown; complete paired recovery and real durability. Native
build envelopes must be established, not guessed from small successful runs.

## Owner decision and delivery boundary

Approve or reject this **specific further-qualification amendment**: one public
Windows pair VFS owning rooted native I/O and auxiliary deletion, one early SQLite
allocator router with pair-local fixed scratch arena, and the conservative
no-spill/startup/retirement availability restrictions above. No child process,
new capacity pool, SQL authority, on-disk decision log or native test allowance
is included. Earlier stock-win32 evidence remains valid in its exact scope.

The source-level implementation/verification plan follows only after that scoped
architecture decision. Actual source authority and native build/envelope evidence
still gate an eligible initializer or publisher; no synthetic admitted factory
bypasses them. The prior native batch remains dispatch 1/1 and qualifying process 1/1
consumed. This document writes no code, opens no database, runs no model/native
probe and activates nothing. #1176 remains open; #740 remains closed research.

## Bounded builder check

One fresh read-only builder round examined this proposal. Its sole important
question was whether deriving conservative growth and scratch envelopes can be
the first qualification gate, because guessing them changes arena sizing,
supported operations and potentially feasibility. Author ruling: **yes**, the
explicit first gate above precedes dependent implementation planning; missing
bounds deny rather than create defaults or extra capacity. This is a sequencing
clarification of the proposal, not native evidence or an owner acceptance. No
second post-edit review or approval is claimed. At that pre-acceptance point, the VFS/router/availability
amendments remained pending the one scoped owner decision.

## Scoped architecture acceptance — 2026-11-06

After reviewed [#1469](https://github.com/Rambolarsen/orkworks/pull/1469), head
`e86d447cad43611c770b7f47dcb1ec3c75bfc78b`, merge
`50ba4dc329b9a60b51bc6adf25229b6ff0c98d1b`, the owner answered “keep going” to
the explicit question approving this VFS/router/pair-arena and availability
package **for further qualification only**. Earlier owner-pending wording is
superseded only for those choices. [ADR 0112](../../adr/0112-pair-native-enforcement.md)
records the scoped replacement of the stock/no-custom-VFS selection; all other
accepted original-owner/source/lifetime and resource contracts remain.

Closed #1470's private SQLite copy and extra restrictive defaults are not selected.
Do not reopen unchanged architecture approval or infer an arena size/fit from it.
Complete growth and scratch bounds remain the first qualification gate; the
[initial pinned-source derivation](../../validation/pair-native-envelope-derivation.md)
closes only named component facts and leaves the manifest unavailable. No dependent
backend implementation plan, SQL/database, native/model probe, dispatch/budget
renewal, qualified constructor or runtime activation is authorized by this answer.
#1176 remains open and #740 remains closed research.
