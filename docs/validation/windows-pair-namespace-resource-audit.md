---
type: Qualification Audit
title: Windows pair namespace and auxiliary resource enforcement
description: Source-backed audit of the selected Windows SQLite candidate's namespace, auxiliary-file and scratch enforcement seams after its passing handle component test.
tags: [orkworks, hierarchy, windows, native, resources, qualification]
status: draft
workflow_status: pre-operation-enforcement-missing
---

# Windows pair namespace and auxiliary resource enforcement

Tracking: [#1462](https://github.com/Rambolarsen/orkworks/issues/1462),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Repository inspection:
`3b66b37dc519322b8813dc794c1411064b3c5952`.

## Result and next deliverable

The [passing Windows component attempt](windows-pair-handle-component-evidence.md)
closes the six selected handle/sharing observations on its recorded runtime.
It does not supply a production namespace or aggregate resource owner. The next
missing seam is **ownership and admission before native creation, file growth,
auxiliary deletion/recovery and scratch allocation**. Retained file handles and
post-operation size observations cannot supply that admission retroactively.

The inspected stock win32 paths contain no OrkWorks pair-budget enforcement.
The available settings below do not compose into the accepted shared ceiling.
This is a concrete missing mechanism/proof, not a claim that SQLite cannot be
used safely or that the selected Windows candidate is impossible. A closed,
conservative source-level upper bound could also establish admission, but no
complete bound for the coupled command and recovery paths is supplied here.

The smallest next deliverable is a **pair-local pre-operation reservation and
namespace ownership design**, covering every file/allocation class in the matrix
below. It must identify either a complete conservative bound for the unchanged
profile, or an enforcement boundary that actually sees operations before their
side effects. It must reconcile memory attribution and recovery in the same
design, rather than add another toy success probe or catalog-count document.
No new architecture, VFS, quota, allocator or native execution is selected by
this audit.
The [follow-up proposal](../superpowers/specs/2026-11-06-pair-preoperation-enforcement-design.md)
compares enforcement amendments and identifies one concrete owner-pending package;
its drafting does not change this audit or the accepted candidate.

## Accepted constraints and source pin

Reuse the [accepted original owner](../superpowers/specs/2026-11-06-original-pair-owner-design.md#scoped-owner-protocol-acceptance--2026-11-06)
and [joint resource contract](../superpowers/specs/2026-11-05-joint-child-preparation-design.md#worker-storage-recovery-and-bounds):
hierarchy 4,096-byte pages / 1 GiB file; **one shared 2 GiB rollback/auxiliary total**
for both participants, super-journal, statement journals and recovery;
**separate 8 MiB SQL scratch**. Lower ordinary/hierarchy queue, intent, resource,
retained-copy and lifetime limits remain binding. No per-attached-file allowance,
new ordinary physical quota or maximum-profile fit is inferred.

The accepted candidate excludes custom VFS/shims and private SQLite layout casts.
A public VFS wrapper or allocator/process-isolation amendment would therefore be
a new design decision requiring its own reviewed concrete proposal and owner
approval before implementation. Public API availability does not waive that
existing exclusion or automatically qualify a replacement profile.

[Cargo.lock](../../crates/orkworksd/Cargo.lock) pins libsqlite3-sys 0.30.1 /
SQLite 3.46.0 with bundled/hooks from [Cargo.toml](../../crates/orkworksd/Cargo.toml).
The inspected dependency file is `libsqlite3-sys-0.30.1/sqlite3/sqlite3.c`,
SHA-256 `c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d`.
Line numbers below identify that amalgamation; function names survive splitting
into upstream files. Source can be obtained from the
[pinned crate](https://crates.io/crates/libsqlite3-sys/0.30.1).
Current online documentation is interface context; pinned source controls the
version-specific findings. No source bytes were modified.

## Namespace ownership versus the observed leaf guard

The [probe](../../crates/orkworksd/tests/windows_pair_handle_probe.rs) opens
non-inherited guards with read/write sharing and no delete sharing, checks a
regular/non-reparse final object, and tests one held leaf-directory rename.
[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
documents those sharing and reparse-open controls. The observed rename denial
is positive evidence for that leaf operation; it is no complete component walk,
ancestor pin, mount/reparse-retarget proof or admission of later auxiliary names.

[WorkspaceIdentity::resolve/revalidate](../../crates/orkworksd/src/workspace_runtime.rs)
canonicalizes a requested workspace, retains one directory and compares it with
a fresh resolution. Its Windows opener uses backup semantics; it does not walk
and retain the pair's complete naming-directory chain or provide a pair-owned
reparse policy. This is appropriate existing workspace identity behavior, not
an observed exploit or a bug claim against ordinary storage.

An actual pair namespace owner must bind the selected original root, both fixed
main roles and every allowed auxiliary name to its retained guard set for the
whole operation/disposition lifetime. It needs bounded component/handle admission
and explicit failure for unsupported redirection or identity, including failed
opens. Omitted delete sharing alone is not evidence for every namespace mutation:
[FSCTL_SET_REPARSE_POINT](https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-fsctl_set_reparse_point)
is a separate operation, not exercised by the probe. No additional same-user or
privileged-hostile-process sandbox guarantee is introduced.

## Auxiliary creation, growth and recovery trace

| Class / pinned source | Actual path | Missing pair-level proof or enforcement |
| --- | --- | --- |
| Main rollback journals: pager_open_journal, lines62925–62957; winOpen,51673–51900 | Persistent main journals request READWRITE/CREATE without EXCLUSIVE; stock win32 maps CREATE without EXCLUSIVE to OPEN_ALWAYS. Existing-file-only main/ATTACH does not make journal creation CREATE_NEW | Exact auxiliary ownership, legitimate-existing/hot-file classification, namespace authority and pre-growth shared reservation; never adopt or delete a file merely from a suffix |
| Super-journal: vdbeCommit,87835–87965 | Selects a generated main-derived name, opens EXCLUSIVE, writes participant journal names, syncs and later deletes through the VFS | Super-journal namespace/bytes/count and failed/unknown lifetime belong to the same shared budget. Main handle comparison does not enumerate these operations |
| Super-journal name collision branch: vdbeCommit,87852–87879 | After repeated existing-name collisions, source includes a VFS delete of the selected name before attempting its exclusive open | The namespace owner must justify every permitted deletion or prove the branch unreachable under its closed ownership rules. This is static control-flow evidence, not an observed collision or an exploit claim |
| Statement/sub-journal: openSubJournal,61599–61615; memjrnlCreateFile/memjrnlWrite,106162–106271 | May start in allocated memory and spill into a VFS temporary file with CREATE/EXCLUSIVE/DELETEONCLOSE | Account peak memory, spill, simultaneous retained representations on failure, and temporary-file namespace before allocation/open/write; do not count only surviving files |
| Recovery: pager_delsuper,59635–59711 | Opens a super-journal, obtains its size, allocates space for the whole content plus path workspace, reads child names and inspects related journals | Recovery requires bounded admission of existing auxiliary lengths, reads, allocations and names, with no caller-supplied path authority. An own-write bound alone does not qualify a recovered input |
| Native I/O: winWrite,49451–49554; winTruncate,49560 onward | Delegates to native writes/truncation; chunk sizing can round a requested size. These bodies have no OrkWorks aggregate reservation check | Charge checked end offsets, simultaneous files, growth rounding and unknown outcomes before excess work, or supply a conservative complete upper-bound proof |

SQLite describes additional temporary classes and their OS-specific locations in
[temporary-file documentation](https://www.sqlite.org/tempfiles.html). Setting
`temp_store=MEMORY` does not remove rollback/super-journal disk I/O; it also does
not establish an 8 MiB memory ceiling. WAL/shared-memory and VACUUM are not silently
allowed by the rollback/no-VACUUM profile; a future owner must reject unsupported
operation classes and show the selected command set cannot produce them.

Main-file no-delete guards also cannot simply be copied to journals unchanged:
the selected DELETE profile requires legitimate journal/super-journal deletion,
while a retained handle omitting delete sharing can deny that operation. An
auxiliary-owner design must reconcile exact ownership, allowed deletion and
handle lifetime, without assuming the main-file sharing experiment proved it.

## Available controls and their actual limits

| Control | What it establishes | What it does not establish |
| --- | --- | --- |
| WIN32_GET_HANDLE / FILE_ID_INFO | Borrowed main-file identities, positively observed for both roles | Complete auxiliary inventory, ownership or growth reservation |
| [sqlite3_file_control / JOURNAL_POINTER](https://www.sqlite.org/c3ref/file_control.html) | Documented per-schema control and a current journal-file object query | A callback before every auxiliary open/write/delete, complete super-journal/temp enumeration or private native layout access |
| [journal_size_limit](https://www.sqlite.org/pragma.html#pragma_journal_size_limit) | Limits files retained after transaction/checkpoint finalization; zeroJournalHdr,58497–58529, truncates after commit | Peak journal size during work or the aggregate of both journals, super-journal and recovery files |
| max_page_count / page_size | A selected pager's database page ceiling | The journal/scratch amplification of every coupled command or any independent ordinary physical ceiling |
| temp_store=MEMORY | Selects memory for applicable temporary storage classes | Hard scratch allocation admission or removal of transaction-control disk files |
| [sqlite3_hard_heap_limit64](https://www.sqlite.org/c3ref/hard_heap_limit64.html) | Hard limit for accounted SQLite heap across the process, subject to documented memory-accounting configuration | A separately attributed pair scratch pool. Setting a process-wide 8 MiB limit would constrain other connections and non-scratch SQLite heap; that is no existing per-pair policy |
| Query progress/deadline hooks and after-the-fact file/heap samples | Cooperative cancellation/observation at their actual callback points | Proof that an earlier native allocation, creation or write stayed below its bound |

`winFileControl`,50167–50296, does not implement SIZE_LIMIT; unsupported opcodes
return NOTFOUND. The documented SIZE_LIMIT opcode applies to an in-memory VFS;
it is not a hidden stock-win32 file-quota switch. SIZE_HINT/CHUNK_SIZE are not
aggregate caps either.

[Ordinary Database::configure](../../crates/orkworksd/src/session_store.rs)
configures DELETE/EXTRA/foreign keys and temporary query-token tables, without
these pair-level owners. Its record/intent limits do not establish the maximum
physical effect of every SQL operation. The hierarchy
[mechanical settings](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/schema.rs)
are test-only; production [Connection::open](../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs)
still refuses the unqualified profile before I/O. No located pair VFS registration,
pair allocator or complete paired resource constructor closes the matrix above.

## Required content of the next design

A concrete reservation/namespace design must settle all of these together:

- Actual lifetime root and attribution for the complete guard/file/SQL-memory
  inventory, retained failed opens, rollback/recovery and unknown disposition;
  retain the accepted PairSlot/PairCore and StoppedOccupied policy.
- A checked, conservative admission rule for every native creation/growth/read/
  allocation and ownership-qualified deletion, including recovery inputs and
  copy/spill/rounding costs; ordinary and hierarchy contributors share the
  existing limits. Shrink or deny an unsupported operation rather than raise a cap.
- Either a demonstrated closed-program bound using the existing stock profile,
  or an explicit proposed amendment for the needed public VFS/I/O and allocation
  seam. The [public VFS](https://www.sqlite.org/c3ref/vfs.html) and
  [file methods](https://www.sqlite.org/c3ref/io_methods.html) expose relevant
  operation boundaries, but no wrapper or memory isolation is approved here.
- Finite native validation oracles for the new mechanism only after that design
  is reviewed/accepted and an execution plan/budget is approved. Keep the prior
  passing stock-win32 component evidence distinct from any amended backend.

This audit performs source/documentation reads only: no SQL/database opening,
new native/model probe, custom VFS implementation, allocator change, production
source registration or activation. The prior batch remains dispatch 1/1 and
qualifying native process 1/1 consumed. All accepted decisions remain accepted;
strict source cutoffs, paired durability/restart and full source admission remain
separate gates under #1176. #740 remains closed research.
