---
type: Qualification Plan
title: Pair file-length and fixed-arena qualification
description: Gated exact-build coverage, offline accounting, native mechanism and production admission sequence for the accepted reduced resource guarantee.
tags: [orkworks, hierarchy, windows, sqlite, resources, qualification]
status: draft
workflow_status: source-coverage-ready-execution-gated
---

# Pair file-length and fixed-arena qualification

Outcome: establish that the selected pair mechanism enforces the owner's
[accepted capacities](../specs/2026-11-24-pair-resource-assurance-design.md#scoped-resource-assurance-acceptance--2026-11-24),
without claiming a bound on total native costs or supplying source authority.
Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176), prerequisite
of #738/#741/#744/#746; research physical qualification remains separately gated
under #1689. [ADR 0118](../../adr/0118-pair-resource-assurance.md) replaces only
ADR 0112's stronger resource meanings and complete numeric E/S first gate.

**Ready for Gate 1 source-only investigation. Investigate before code execution;
Gates 2–4 are not authorized by this plan.** Continue inline when this session
owns the next separately scoped task; use executing-plans only for an authorized
executable slice. No delegation or execution-method choice is required here.

## Baseline and unchanged constraints

Inspection base: `b39fff709530390725382e46c2ce1230aa54bf30`.
[Production hierarchy Connection::open](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs)
still rejects the unqualified profile before I/O. The inspected
[session-store subtree](../../../crates/orkworksd/src/session_store.rs) has no
located NativePairIo/PairSlot/PairCore/joint_preparation implementation; these
names are accepted design owners, not callable qualified factories. The
[retained callback record](../../validation/pair-native-envelope-input-closure.md)
already inventories the 41 public slots: reuse it instead of producing another
callback catalog or treating old compiler frames as mechanism qualification.

The retained evidence pins SQLite 3.46.0 / libsqlite3-sys 0.30.1, amalgamation
SHA-256 `c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d`.
Reconcile the actual dependency/build features with that pin before deriving
implementation coverage; version drift is not silently qualified by old evidence.
Do not fetch/build/run a target as a substitute for an unreadable source.

- Windows/local NTFS, same ordinary worker and attached rollback transaction;
  original PairSlot/PairCore, rooted actual identities and owned deletion.
- 4,096-byte hierarchy pages; 1,073,741,824-byte hierarchy logical EOF high-water;
  2,147,483,648-byte shared auxiliary high-water total; one 8,388,608-byte arena
  including its alignment, bookkeeping, fragmentation and simultaneous copies.
- Ordinary main stays with its existing owner; all lower queue/intent/work/
  retained-copy/source/materializer/handle/position budgets remain binding.
- No shrink refund, fallback/default migration, disk temp spill, late allocator
  installation, reset/restart-to-fresh, fabricated identity or authority proof.
- Native host costs/stacks are outside numeric caps and disclosed as such; unknown
  controlled allocation routing still denies. No maximum valid command fit.
- Independent source/newness/cutoff/catalog/initializer/recovery/durability and
  ordinary activation gates remain. Consumed native and compile batches stay spent.

## Gate 1 — exact-build enforcement closure, source only

- [ ] Reconcile exact dependency source/features and the closed operation set.
  For every existing callback inventory entry, identify its selected native/API
  behavior and whether it can create/extend an object, allocate controlled
  storage, report sizes, retain pending work, release ownership or reject.
  Cite pinned source and distinguish proposed adapter behavior from existing code.
- [ ] Close the operation-to-owner map for both main roles and every permitted
  auxiliary: object identity, checked offset/end, pending/uncertain high-water,
  recovery inventory, no-refund shrink, deletion/reclamation and retry. Optional
  allocation/chunk hints must have a correct public-ABI rejection path, not a
  hidden extension. A missing behavior/owner keeps the map incomplete.
- [ ] Close the allocation-route map: pair heap, controlled callback buffers,
  lookaside/page-cache/temp/realloc copies, startup/global/unpaired allocation,
  pointer provenance, cross-context free/escape, and excluded native/helper/stack
  paths. Name every retained buffer's existing budget and lifetime; do not label
  unknown application allocation as OS overhead or invent a numeric S.
- [ ] Write one bounded implementation slice with concrete private owner/module
  boundaries and tests for the closed routes. NativePairIo remains inside the
  proposed private session_store::joint_preparation owner; no generic Connection,
  supplied SQL/path/budget, public constructor or admitted-source fixture seam.

Deliverable: a source-backed enforcement map and implementation slice appended
to the existing resource evidence concept or a linked canonical concept if its
scope warrants one. Pass: every controlled route has a specified pre-effect
check, original owner and retained outcome; every excluded host path is explicit;
unknowns deny. **An inventory count alone is not a pass.** Root is the single
writer; no build, SQL, database, fixture/code implementation, native/model/compiler
probe or budget renewal is part of this gate. New material choices go to owner
review; unchanged accepted resource/architecture choices need no repeat approval.

## Gate 2 — private offline accounting and arena slice

- [ ] After Gate 1 closes and a bounded executable slice is authorized, use TDD
  for the selected private owner, with no production registration or SQLite
  startup installation. Keep resource mechanisms separate from source permission.
- [ ] Check below/exact/above the two file-length capacities, overflow/negative
  or unknown sizes, multiple participant journals plus super-journal together,
  zero-length creation admissions, outstanding growth, partial/error writes,
  no refund on shrink, stale/wrong identity, retry, uncertain deletion/close and
  release only after complete original-owner reclamation.
- [ ] Check all arena bytes including bookkeeping/fragmentation, old-plus-new
  realloc and simultaneous buffers; OOM preserves old storage, no allocator
  fallback/migration, pointer provenance across contexts, escape retention,
  startup conflict and unpaired/global delegation. Model outcomes are not native
  evidence. No live SQLite initialization or database is needed for pure checks.

Pass: meaningful red/green boundary/error tests for the actual private slice and
its exact diff review; applicable repository checks, including verify-repo after
code implementation. Full-suite test execution that opens databases requires
explicit inclusion in that later executable slice; it is not authorized now.
No mechanism qualifies production by passing an isolated accounting model.

## Gate 3 — separately bounded native qualification

- [ ] Only after the actual mechanism/slice is reviewed, propose the exact build,
  artifact hashes, platform/filesystem, cases, fault/repetition/time budgets,
  target invocations, observations/oracles and owned cleanup. Obtain the
  required explicit execution authorization before tooling, dispatch or probes.
- [ ] Qualify rooted non-reparse namespace and actual I/O identities, locks/sync,
  legitimate handle-based deletion, failed/partial/unknown lifetimes, aggregate
  EOF denials before extending I/O, arena ABI/startup/unpaired behavior, actual
  recovery and paired power-loss/durability outcomes. An observed small peak is
  neither an EOF-admission oracle nor proof of a host allocation ceiling.

Pass: supported actual mechanism evidence for the authorized cases, no ambiguity
promoted to CompleteOld/New, no leak of charges/ownership or source authority.
Stop at the approved budget or unsupported case; retain evidence and unavailable
status. Neither this plan nor old successful stock-VFS/compile evidence grants
another invocation or qualifies the new VFS/router.

## Gate 4 — complete admission and production decision

- [ ] Reconcile complete initializer/catalog/membership/resource effects, actual
  source/newness and commit-authority participation, original old/new/unknown
  recovery, ordinary storage/migration/retention and actual activation evidence.
  Do not infer research physical-format acceptance from this resource decision.
- [ ] Require private qualified-build data and any operator eligibility wording
  to state file-length/fixed-arena assurance and uncapped native host costs.
  Host observations cannot be displayed as guaranteed remaining capacity.

Pass: all independent profile requirements and a separately reviewed executable
publication/activation plan, with compatibility/recovery and rollback evidence.
Until then keep production Connection::open unavailable. No format, schema,
source grant, migration or runtime route is selected by this roadmap.

## One-time plan review and risk disposition

Scope/clarity/coverage review: **Ready for Gate 1; Investigate for Gates 2–4.**
Every accepted resource rule maps to coverage in Gate 1 and a boundary/failure
oracle in Gate 2; actual native semantics belong to Gate 3, independent authority/
activation to Gate 4. The plan deliberately does not prewrite Rust or assume a
qualified parent module exists. It reuses reviewed resource/architecture choices.

Dependencies 4 (ordinary worker, native VFS/router and source owners); blast
radius 4 (shared SQLite startup and linked storage); state changes 1 for Gate 1
(read-only, documentation evidence), Unknown for later execution; reversibility
1 for Gate 1, Unknown for unqualified native persistence; uncertainty 4 (actual
controlled routes and native behavior unqualified). Aggregate execution risk is
incomplete, not a permission threshold. Gate 1 must resolve controlled routing
before executable decomposition; later native rollback remains an explicit gate.

No implementation or probe ran to write this plan. The acceptance record changes
requirements only. #1176 remains open and the selected backend unqualified.
