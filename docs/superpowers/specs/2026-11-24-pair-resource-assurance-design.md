---
type: Design Proposal
title: Pair resource assurance through owned file lengths and fixed arena storage
description: Proposed explicit replacement of total native peak guarantees with enforceable owned file-length and SQL arena budgets, with host costs disclosed and implementation still gated.
tags: [orkworks, hierarchy, windows, sqlite, resources, assurance]
status: draft
workflow_status: proposed-owner-decision-pending
---

# Pair resource assurance through owned file lengths and fixed arena storage

Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176), prerequisite
of #738/#741/#744/#746 and the physical research profile under #1689. Design base:
`65ae7351`. This is a **proposed material reduction in the resource guarantee**,
not a clarification that the existing guarantee always meant logical bytes.
Publication or merge records the proposal; it does not accept it.

## Decision requested and reason

**Author recommendation:** retain the accepted same-worker, attached-transaction,
private Windows/local-NTFS VFS and early allocator-router architecture, but replace
the total native-growth and complete SQL-scratch qualification requirements with
three explicitly accounted capacities. Owned file-length admission and a fixed
SQL arena remain hard checks; total filesystem allocation, native scratch and
process memory are disclosed host costs without a promised numeric ceiling.

The [accepted design](2026-11-06-pair-native-enforcement-design.md#scoped-architecture-acceptance--2026-11-06)
and [ADR 0112](../../adr/0112-pair-native-enforcement.md) currently require the
stronger contract. The [retained input evidence](../../validation/pair-native-envelope-input-closure.md)
closes only component facts. The [source-only checkpoint](https://github.com/Rambolarsen/orkworks/issues/1176#issuecomment-7124345603)
found no supporting whole-operation bound in the inspected API contracts.
It did not establish impossibility. Repeating an inventory or usage sample does
not close the missing pre-operation native peak `E` and complete scratch `S`.

The main uncertainty is whether this weaker resource assurance is acceptable to
the owner. Source inspection cannot answer that product decision. A second risk
is claiming that the proposed accounting is already implemented: it is not.
Neither limit exhaustion nor native failure proves transaction rollback.

| Choice | Consequence |
| --- | --- |
| Keep the existing contract | Valid default. Candidate remains unavailable until supported complete bounds exist; no automatic further probe chain |
| Adopt this proposal | Bound the named application-controlled capacities; explicitly accept host costs outside those capacities, then qualify the actual mechanism before use |
| Isolate SQL in a process or volume container | Requires a different architecture, lifecycle and ordinary-service design; not selected here and not proven to supply the original guarantees |

## Exact change in guarantees

Binary units apply: 1 GiB = 1,073,741,824 bytes; 2 GiB = 2,147,483,648 bytes;
8 MiB = 8,388,608 bytes. Keeping the numbers does **not** preserve their meaning.

| Surface | Existing requirement | Proposed guarantee |
| --- | --- | --- |
| Hierarchy main file | 4,096-byte pages / 1 GiB physical file ceiling | 4,096-byte pages; owned logical file-length high-water at most 1 GiB, enforced before extending operations. No total native allocation ceiling |
| Shared auxiliary files | One 2 GiB total including conservative native peak/rounding and simultaneous or uncertain representations | One 2 GiB total of owned logical file-length high-water charges and pending/uncertain extensions, across both participants and every permitted disk auxiliary. No filesystem allocation peak ceiling |
| SQL scratch | Fixed arena `A` plus all applicable outside-arena scratch `S`, with `A + S <= 8 MiB` | One fixed 8 MiB backing region for pair-routed SQL allocations and owned SQL callback buffers, including arena bookkeeping and peak copies. No bound on total SQL/native scratch outside it |

The hierarchy main-file wording is included because leaving “physical” there
would retain the same unresolved native-peak ambiguity. The ordinary main file
remains under its existing owner and is excluded from the hierarchy and auxiliary
capacities; this proposal creates no ordinary-main quota. All lower ordinary and
hierarchy queue, intent, read/work, retained-copy, source, materializer, handle and
worker-position budgets stay separate and binding. No cap is multiplied by the
number of attached databases or requests. No maximum valid command is promised
to fit; fragmentation and conservative retained charges can deny valid work.

The excluded costs include NTFS cluster rounding, preallocation/internal copies,
filesystem metadata and caches, OS/driver workspaces, thread stacks, native helper
heap/workspaces and unchanged process-global SQLite initialization. Their cost
can exceed the named capacities. Host disk exhaustion, memory pressure, process
failure and collateral effects on unpaired work remain possible. Free-space or
memory observations may explain availability; they cannot prove these costs fit
or turn this profile into host resource isolation.

## File-length admission and retention

Reuse the original PairSlot/PairCore lifetime, rooted non-reparse object identities,
exclusive auxiliary creation, closed filename grammar and legitimate handle-based
deletion from the accepted native design. No stock-VFS delegation, arbitrary
pathname adoption, second connection or new durable application journal is added.
These ownership and native-compatibility requirements still need qualification.

For each disk auxiliary object, retain one charge `h` covering its largest admitted
logical EOF, including the full requested end of any pending or uncertain write.
It is charged once by actual owned object identity, even if engine references
change. Let `T` be the sum of all such retained charges. Before a growth operation,
checked arithmetic computes `end = offset + length` for a write, or the requested
EOF for truncate growth, and `h_new = max(h, end)`. Reserve the increase before
native work and require `T - h + h_new <= 2 GiB`. An unrepresentable offset/length,
negative size, unknown previous charge or unsupported extending operation denies
before I/O. Serial worker ownership does not waive pending native-operation
accounting: retain the reservation until completion is resolved.

Apply the equivalent high-water check to the hierarchy main file with its
separate 1 GiB ceiling. Both ordinary and hierarchy journals, super-journals,
permitted statement journals and all recovery disk auxiliaries share `T`.
In-memory temporary structures use the arena instead; disk temp/sort/sub-journal
spill remains forbidden. A newly observed class is unsupported, not a fresh pool.

A zero-length auxiliary still consumes its existing inventory, naming and handle
admissions before creation. Do not forward allocation hints or chunk-size growth
that bypass the EOF reservation rule: decline optional hints with the correct
public ABI result, or reject the profile if the closed command set requires an
unsupported effect. This does not claim that ordinary native writes cannot
allocate more host storage than their EOFs.

Use a deliberately conservative release policy: shrinking or rewriting an owned
file does not refund its high-water charge. Failed or ambiguous operations retain
the full admitted charge, including after partial writes. Auxiliary charge is
reusable only after the original owner proves engine references drained, native
operations complete, and the owned object and name reclaimed. Unknown deletion,
close or identity keeps the charge and original position occupied. Retrying an
operation does not create another allowance or erase the earlier charge. Main
high-water capacity stays with its original owner until supported retirement.

Recovery must admit the complete validated existing inventory and lengths before
SQL work or recovery reads; it may not discover uncharged child files piecemeal
through journal contents. Recovery has its own accepted data-only authority and
cannot recreate a fresh pair, adopt foreign files, truncate-to-fit or erase hot
journals. Length admission supplies no source, genuine-newness or recovery proof.
An unknown length/association or an aggregate above the cap denies recovery.

## Fixed arena and explicit outside costs

Propose one precommitted backing region of exactly 8 MiB per admitted original
pair, retained through active/retiring/unknown lifetimes. The entire region,
including alignment, bookkeeping and fragmentation, is the allowance; it is not
8 MiB payload plus extra management storage. Any backing-allocation overhead
outside that region is a disclosed host cost. Reserving the region consumes the
existing global single active-or-retiring position, not a new per-request pool.

Keep the accepted early process-wide router with the original allocator preserved
for unpaired work and process-global initialization. Its startup ordering,
pointer provenance, arena ABI, escape handling and no-migration/no-fallback rules
remain binding. Pair SQL heap and application-owned variable SQL callback buffers
must use this region; no unbounded growing Rust buffer is authorized outside it.
Copies during realloc and other simultaneous owned buffers must fit together;
OOM preserves the original pointer and returns the correct engine error.

An exact-build allocation-route inventory must still distinguish pair allocations,
unchanged unpaired/global allocation and documented native helper work. Unknown
routing of an application-controlled allocation denies qualification; it cannot
be relabeled as OS overhead. Deliberate growth of a default-allocator pointer
inside pair context remains forbidden. Native helper/OS allocations and stacks
are explicitly outside the numeric guarantee, not proven absent or charged as
zero. This proposal removes the need for a complete numeric `S`; it does not
remove the need to identify and disclose those paths for the selected build.

Continue to disable mmap and SQLite background workers, verify memory-only temp
storage, reject unsupported operations, and retain arena registration until all
engine references/allocations/callbacks are gone. No smaller platform stack is
selected, no process-wide heap cap is installed, and no router is installed after
SQLite initialization. An installation conflict denies availability and cannot
trigger a restart, allocator replacement or ordinary-memory reset.

## Failure, authority and compatibility

No file format, schema, source codec, bearer, permission, epoch, membership rule,
command proof, approval event or publication cutoff changes. Logical acceptance
of the research catalog/initializer/command proofs is separate from physical
qualification. A capacity reservation supplies no source authority. Complete
old/new/unknown disposition, paired atomicity, cancellation and lost replies
remain owned by the original operation; an OOM/disk-full/error is not proof that
nothing changed. Preserve recovery evidence and fail closed on ambiguity.

The host can still terminate the sidecar or fail outside the arena. A process
loss grants no restart-to-fresh, replacement owner, reclaimed source reservation
or refund. Stop/input must retain their independent responsiveness contract;
weaker resource assurance does not excuse holding core authority latches across
SQL/native work or silently broaden accepted latency guarantees.

Existing ordinary storage/installer behavior and old store bytes are unaffected
by writing or accepting this proposal. No automatic migration/adoption, legacy
qualification, source registration or runtime activation follows. A qualified
profile must identify this assurance level explicitly in its private build data
and any operator-facing eligibility explanation; it must never advertise the
old total-native-peak guarantee. Numeric host-usage observations must be labeled
observations, not remaining guaranteed capacity.

## Acceptance, sequencing and finite checks

One owner decision is requested on the **whole three-row guarantee change and
its disclosed residual risk**. The same-worker/VFS/router/PairCore architecture
is already accepted; approval of it is not requested again. If declined, the
old contract and unavailable result remain authoritative. If accepted, preserve
this dated decision and synchronize the canonical joint/native resource contract,
Taskmaster spec routing and ADR history in a separate acceptance change before
any dependent implementation. Replace the conflicting ADR decision through the
repository supersession procedure; do not rewrite historical evidence or claim
the old stronger proof was obtained.

After that synchronization, a concise implementation plan must close:

1. Exact build and controlled EOF-extension/allocation-route coverage, pending
   and retained charge ownership, and compatibility with the existing budgets.
2. Offline boundary checks below/exactly/above both EOF capacities, checked
   overflow, multiple journals together, failed growth, no refund on shrink,
   unknown deletion, retry and original-owner retirement; arena bookkeeping,
   fragmentation, simultaneous realloc, OOM, escape and startup-conflict checks.
3. A separately approved, bounded native qualification plan for the actual new
   VFS/router, rooted identities/locks/sync/deletion, error outcomes and paired
   recovery/durability. Prior stock-VFS/native/compile evidence is not reused as
   qualification of the new mechanism; all consumed execution budgets stay spent.
4. Independent source admission, physical cutoff, complete initializer/catalog,
   retention/ordinary activation and runtime gates. Capacity tests cannot waive
   them, and meeting the new accounting alone cannot enable production use.

No implementation plan is executed by this proposal. The current candidate
remains unavailable under the existing contract. This document opens no database,
installs no allocator/VFS, runs no SQL/native/model/compiler probe, renews no
budget and closes neither #1176 nor the larger source/physical qualification.

## Review boundary

One bounded round used two fresh `gpt-6-luna` reviewers with separate correctness
and builder/completeness questions. Their command host was unavailable before
file reads. The author supplied the design content and accepted-contract excerpts
as a fallback: the builder found nothing important left before planning; the
correctness reviewer found no material issue in the supplied excerpts. These are
text reviews, not independent file/line or source verification. The author checked
the saved document against the supplied content and linked accepted constraints.
No reviewer finding required a design change. Review or merge cannot accept the
changed product guarantee on the owner's behalf.
