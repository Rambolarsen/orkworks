---
type: Qualification Audit
title: Pair native envelope first source derivation
description: Pinned SQLite super-journal component arithmetic and missing growth/scratch inputs after scoped VFS/router acceptance; no complete manifest or platform fit.
tags: [orkworks, hierarchy, sqlite, resources, qualification]
status: draft
workflow_status: complete-envelope-unavailable
---

# Pair native envelope first source derivation

Tracking: [#1472](https://github.com/Rambolarsen/orkworks/issues/1472),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Repository inspection:
`50ba4dc329b9a60b51bc6adf25229b6ff0c98d1b`. The
[accepted VFS/router direction](../superpowers/specs/2026-11-06-pair-native-enforcement-design.md#scoped-architecture-acceptance--2026-11-06)
and [ADR 0112](../adr/0112-pair-native-enforcement.md) permit this read-only
qualification work. They establish no bound by themselves.

## Status and source identity

**The complete growth/scratch manifest remains unavailable.** This audit derives
one component condition and names its missing inputs. It is neither a native
measurement nor a proof that a valid command, recovery case or maximum profile
fits the unchanged shared 2 GiB auxiliary / separate 8 MiB scratch ceilings.
No arena size, native allocation quantum or new count/path/work default is chosen.

The inspected source is bundled SQLite 3.46.0 from libsqlite3-sys 0.30.1,
`sqlite3/sqlite3.c`, SHA-256
`c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d`.
The local file's hash was checked before inspection. The
[pinned crate](https://crates.io/crates/libsqlite3-sys/0.30.1) supplies these
source bytes. No SQLite source was modified, compiled or executed for this audit.

## Unchanged super-journal recovery component

`pager_delsuper`, lines59634–59711 in that amalgamation, first allocates one block
for **two** VFS file objects, then opens the super-journal, obtains its length,
and allocates a second block containing the whole journal plus pathname workspace.
Both blocks coexist while it checks child journals.

| Symbol | Meaning / required provenance |
| --- | --- |
| `F` | Qualified VFS `szOsFile` in bytes, including the chosen file object; positive and safe for the source's signed-int `2*F` expression |
| `P` | Qualified VFS `mxPathname` capacity in bytes for SQLite's internal filenames, excluding their terminator; not a Windows UTF-16 path limit |
| `N` | Original owner-admitted super-journal logical byte length; source obtains this as signed `nSuperJournal` |
| `R(n)` | Complete admitted arena cost of a requested allocation, including allocator rounding/headers/bookkeeping; not assumed equal to `n` |
| `L` | Proved conservative upper bound on all other simultaneous arena allocations on this component path, including VFS/owner callback allocations |
| `S` | Proved complete scratch bound outside the arena for the exact build/call graph, including applicable stack/native/callback workspaces |
| `A` | Admitted arena capacity; not selected until `S` and the complete profile are justified |

The first requested allocation is `2*F`. The second is exactly:

```text
4 + N + (P + 1) + 2 = N + P + 7
```

Consequently a component admission check must cover:

```text
R(2*F) + R(N + P + 7) + L <= A
A + S <= 8,388,608
```

These are conditional component requirements, not evaluated fit results. `L` or
`S` cannot be replaced with zero because no number is currently known. Admission
must validate nonnegative sizes, checked source-expression arithmetic and the
later signed-int read length before entering the path. The source narrows `N` to
`int` for its full read; a qualified implementation must not rely on an unchecked
narrowing. Refusing an oversized allocation does not prove that earlier opens
had no effect or that logical rollback completed; retain the accepted original
owner and actual old/new/unknown disposition.

The source also opens a child rollback journal with the **SUPER_JOURNAL** flag
while checking its backlink (lines59691–59695). Its actual role therefore cannot
be inferred from that flag alone. The new VFS must resolve the exact name and
original admitted inventory/phase, including this core-generated recovery access,
without turning file contents or a class flag into ownership authority.

## Own-write content is not a recovered-input or native-growth bound

`vdbeCommit`, lines87835–87965, writes each participating persistent journal name
and its terminating NUL into the super-journal. TEMP/in-memory entries with no
journal name are skipped. Under the accepted fixed-two-main/closed-attachment
program there are at most two such names. If both names are proved to fit `P`,
the **own-written logical content** satisfies:

```text
N_own <= (P + 1) + (P + 1) = 2*P + 2
```

This does not bound arbitrary recovered bytes, grant custody, select `P`, or
prove the native allocation/high-water growth envelope. Recovery needs exact
accepted input identity, length and contents under its separate owner; a foreign
or oversized journal is unavailable, not truncated or adopted. Native capacity
rounding, pending/uncertain representations and all other journals still belong
to the one aggregate 2 GiB charge. A small name-list formula cannot close that
aggregate proof or the complete arena call graph.

## Missing inputs and the next read-only work

Inspection found no production `NativePairIo`, early SQLite allocator-router
installation or qualified build-envelope manifest at this repository base. The
existing stock handle probe is test-only and cannot supply the new backend's
object size, scratch or native growth behavior.

| Missing input | Required evidence before dependent planning |
| --- | --- |
| `F`, `P`, all bounded object/path/handle storage | Actual private VFS/native-adapter interface and complete public-callback inventory; no guessed default copied from stock win32 or closed #1470 |
| `R`, complete `L`, complete `S` | Exact router/arena ABI and allocation routes; closed call graph and build-specific stack/native/callback buffers, initialization, error and retirement paths; global/foreign/escaped allocation treatment |
| Native growth envelope | Exact rooted I/O operations, allocation/preallocation rounding and allowed attributes for the selected build/volume; all simultaneous/unknown representations |
| Complete coupled program/recovery coverage | Every accepted command and supported recovery input, both main journals, super-journal/collision, no-spill denial, native handle duplication and source-owned custody/read bounds |

The next source task is to inventory the complete allocation/callback paths and
identify exact statically controlled versus build/native-dependent terms. If a
term cannot be justified from the available source/design, record it unknown
and keep the manifest unavailable. Any native/compile-probe execution or new
implementation needed to obtain evidence requires its own concrete bounded plan
and applicable approval; this audit supplies none.

The first envelope gate remains open. No dependent backend implementation plan,
VFS/allocator installation, SQL/database opening, workflow dispatch, native/model
probe, quota increase, profile eligibility, source admission or runtime activation
occurred. The existing consumed native budget is unchanged; #1176 remains open.

## Bounded review and disposition

One bounded read-only round separately checked acceptance fidelity and the new
source arithmetic. The source reviewer independently matched the exact SQLite
hash, both simultaneous allocation expressions, signed-int read narrowing,
flag/role caveat and conditional two-name content bound. No material source or
arithmetic error was found. The scope reviewer confirmed that only #1469's
package was accepted, #1470 was excluded and all remaining gates/budgets stand;
one stale pre-acceptance sentence was labeled historical.

Neither review ran code, SQL/database, native/model probes or a build of SQLite.
They supply factual checks of this component record, not architecture reapproval,
a full envelope, implementation approval or native/profile qualification.
