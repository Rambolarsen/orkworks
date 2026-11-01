---
type: Implementation Plan
title: SQLite session storage and history delivery
description: Approved-design delivery sequence for ordinary-session storage, qualified producers, recovery and bounded history.
tags: [orkworks, sessions, sqlite, migration, planning]
status: stable
workflow_status: ready
---

# SQLite session storage and history delivery

Goal: retain ordinary session history without adding history-sized work to live
updates or terminal input. Pass condition: the accepted storage/recovery contract
and its performance budgets have verified evidence before controlled cutover.

Spec: [ordinary session storage](../../../specs/session-storage.md).
Contract and exact limits: [accepted design](../specs/2026-11-01-sqlite-session-storage-design.md).
Decision: [ADR 0103](../../adr/0103-sqlite-session-storage.md).
Parent: [#1240](https://github.com/Rambolarsen/orkworks/issues/1240).
The owner approved the concrete design in conversation on 2026-11-01 after
reviewing draft PR #1253 at `1183221b672aa2850c3916408965aaa00017dbec`.
This plan implements that approval; it does not request a second approval.

## Boundaries and execution

Keep source-priority/scoped provenance, checked revisions, session creation/native
identity, generation/lease/control fences and close-first state semantics.
Preserve content and retention settings. External files and independently owned
stores stay external; no hierarchy/native eligibility or process adoption is
enabled. The existing runtime stays JSON-backed until qualified activation.

One task owner and one branch/PR per independently reviewable deliverable below.
Execute inline within each task using `executing-plans`, TDD for code and the
repository verification/review gates. This plan authorizes no multiwriter fanout.
The dependency graph permits separately owned producer/store work, but delegation
still requires the repository orchestration workflow and isolated writer ownership.
Do not wait on database work under terminal-critical locks. Validate new work
against the then-current source, retaining the accepted interfaces/semantics.

The limits in the design are the acceptance contract: 1 MiB canonical records,
32 queued mutations/8 queued reads/32 MiB admission, separate page/detail/live
bounds, opaque epoch/generation cursors, and the fixed-five-active-session
performance matrix. Link those values rather than duplicating them here.

## Dependencies and deliverables

### 1. Qualify ordinary producers — #1242

Owners: harness definition/launch policy, source-controlled reporter scripts,
canonical APM/agent instructions and their generated artifacts. Start with
`harness/definition.rs`, `harness/registry.rs`, integration owners,
`session-metadata.py`, POSIX/PowerShell reporters and the OpenCode plugin.

- [ ] Inventory supported direct metadata writers/readers and HTTP reporters;
  record installed/source versions and actual effective sandbox/relay profiles.
- [ ] Implement independent ordinary storage-protocol negotiation and mandatory
  mediated writes for migrated workspaces. Do not reuse #789's closed native
  eligibility marker as evidence or weaken its rollout conditions.
- [ ] Preserve source/revision/identity behavior; qualify GET and validation-only
  PATCH from the actual agent context, without file fallback or credential output.
- [ ] Test stale revisions, missing/old helpers, unsupported markers, blocked
  routes and child contexts. An unqualified route remains visibly unsupported.
- [ ] Deliver the compatibility matrix and regressions; run scoped tests,
  `bash scripts/verify-repo.sh` and current-diff review before merge.

Consumes the accepted protocol contract; produces qualified ordinary metadata
writers. No migration is activated by this task. #789 retains native-only scope.

### 2. Private SQL store — #1243

Owners: new private `session_store` module, `metadata.rs` persistence seam,
`metadata/session_transactions.rs`, organization persistence and test fixtures.
Use existing bundled rusqlite rather than another database layer.

- [ ] Write storage-interface regressions for latest-record/revision/source
  arbitration, creation identity, organization, unknown extension preservation,
  maximum records and u64 exhaustion. Pin DELETE/EXTRA and FTS5 capability checks.
- [ ] Implement schema/header/versioning, canonical metadata plus derived columns,
  transactional search indexes and the private bounded worker from the design.
- [ ] Implement typed create/patch/state/read/exists/history/retention/deletion-job
  commands. Preserve no-op and conditional ownership semantics; do not expose SQL.
- [ ] Trace existing synchronous/closure writers and refactor their admission and
  await boundaries; fault tests must prove a stalled store cannot stall PTY input.
- [ ] Test reopen, busy/disk/corruption, queue rejection, rollback and cancellation
  before/after execution without false durable acknowledgement or stale retries.
- [ ] Verify indexed query plans, per-row/page/detail/live limits and explicit
  too-large behavior; preserve canonical content without truncation.
- [ ] Run Rust build/test/fmt, repository verification and current-diff review.

Consumes the schema/worker contract. Produces an isolated tested adapter and
typed query/mutation seam; production remains on legacy storage before task 5.

### 3. Historical facts and native availability — #1248

Owners: `session_view.rs`, `session_projection.rs`, ending/resume mutations and
detail DTOs. Integrate with the store from task 2; do not add attempt history.

- [ ] Preserve last recorded Git/model/outcome/time facts during import/reopen;
  remove current-repository enrichment from historical summaries.
- [ ] Separate retained native references from bounded current eligibility checks
  on detail/action; never fall back to another native session or auto-resume.
- [ ] Test moved/missing worktrees, changed branches, missing replay/plans/native
  records and legitimate resume/close while keeping readable history intact.
- [ ] Complete affected lifecycle/native/view tests, repository verification and
  current-diff review. Task 7 consumes these detail/availability contracts.

### 4. Complete backup and restore — #1250

Owners: a private maintenance/backup module, `workspace_runtime.rs` lease and
format ownership, existing external evidence/image owners; synthetic/copy fixtures.

- [ ] Implement maintenance admission that blocks starts/mutations, drains admitted
  work and pauses all workspace writers. Require explicit quiescence resolution
  after an unclean stop; the workspace lease alone does not prove descendants died.
- [ ] Build an inventory/hash manifest and staged complete backup covering the
  database/marker and required external/related workspace files. Declare global
  settings/native stores/repositories and runtime secrets as excluded.
- [ ] Validate the complete set before success. Restore only into private staging
  and a quiescent destination with existing data preserved until activation.
- [ ] Test interrupted/partial backup and restore, missing optional versus required
  files, corruption, disk-full and restoration including post-cutover writes.
- [ ] Verify supported platform flush/copy behavior and repository checks/review;
  deliver recovery fixtures and instructions to task 5 and task 8.

Consumes the tested store; produces the backup capability required before import.
No online multi-store snapshot or live-user restore is implied.

### 5. Recoverable import, activation and cleanup — #1244

Owners: `session_application.rs` workspace/lifecycle integration, `migration.rs`,
`workspace_runtime.rs`, metadata adapter selection, archive/retention and deletion
owners. Depends on tasks 1–4; producer qualification and backup are hard gates.

- [ ] Stream all validated legacy records/organization into private staging,
  preserving exact identities, extensions, effective provenance and revisions.
  Report malformed/unreadable/oversized/conflicting sources without skipping.
- [ ] Verify source manifest, counts, round trips and references before publishing
  the database/activation record; implement each restart state from the design.
- [ ] Inject failure at every publish/flush boundary. In particular, marker loss
  after later SQL writes must block opening; final/staging evidence must never
  silently enable stale JSON authority.
- [ ] Route current-format operations to SQL, reject incompatible client/protocol
  combinations and diagnose unexpected legacy writes without importing them.
- [ ] Implement identity-bound deletion jobs through current external evidence
  owners; test interrupted cleanup, retry, reused identity, protected archived/
  unknown state and unchanged applied/default retention settings.
- [ ] Support verified same-version backup recovery. Do not claim lossless old-
  binary downgrade without a qualified export of current post-cutover state.
- [ ] Run lifecycle/migration/retention/native tests, full repository checks and
  current-diff review. Keep owner live cutover blocked pending task 8 evidence.

### 6. Bounded transport and complete internal reads — #1245

Owners: `http/session_handlers.rs`, `main.rs` router, `session_projection.rs`,
`workspace_updates.rs`, shared typed read contracts and internal complete consumers.
Build against task 2; production use follows task 5.

- [ ] Deliver negotiated protocol-2 live/history/detail reads and history cursor
  semantics; use indexed columns/FTS5, literal token input and finite budgets.
- [ ] Test byte-limited page progress for large valid records, deterministic ties,
  query/cursor bounds, changed generations and explicit incomplete-read failures.
- [ ] Publish complete live rows/removals plus coalesced history generation; test
  gap/reconnect/fallback and close/resume invalidation without whole-history work.
- [ ] Audit every complete-list consumer: startup unresolved-lifecycle candidates,
  exact recommendation membership, Taskmaster and conflict/evidence queries.
  Test a relevant record beyond page one; never use UI pagination as a complete set.
- [ ] Keep expensive native/Git/plan/payload reads on selected detail/action paths.
  Verify legacy protocol requests fail explicitly on migrated workspaces.
- [ ] Run transport/consumer regressions, repository verification and current-diff
  review. Preserve current read authorization and browser-origin restrictions.

### 7. Desktop integration — #1246

Owners: `api.ts`, `workspaceSessionController.ts`, the existing Sessions view and
exact selected-session detail/terminal owners. Respect the Electron/src split.

- [ ] Consume protocol-2 live data/history pages with the accepted state filters,
  metadata search and bounded loaded-row memory; retain selected detail separately.
- [ ] Test an off-page selected ID, unloaded versus deleted rows, keyboard/context
  actions, stale tokens, archive/restore/delete/resume, reconnect and workspace
  replacement. Paging/filtering must not select or acknowledge another session.
- [ ] Present recorded facts versus current resource availability and explicit
  persistence/read errors; never show empty history/saved success on store failure.
- [ ] Verify real Electron navigation, selected terminal ownership, accessibility
  and memory limits, then repository checks and current-diff review.

### 8. Baseline qualification and controlled rollout — #1247

Owns a reproducible isolated benchmark/recovery harness plus platform evidence.
Prepare the baseline portion before the first controlled cutover; retain/rebuild
the exact legacy reference source so later measurements remain comparable.

- [ ] Retain source SHA, dependency/build/OS/filesystem profile and five identical
  active PTY workloads. Generate 724/10k/100k synthetic histories in temporary
  owned fixtures, never in the user's live metadata directories. Include timestamp
  ties, archived records, maximum previews and missing native resources.
- [ ] Measure legacy and migrated page/search/live/typing/CPU/memory behavior with
  the accepted sample counts, repeated runs, warm/cold conditions and budgets.
  Report storage microbenchmarks separately from end-to-end key latency.
- [ ] Reuse tasks 1–7 failure fixtures and test the complete backup/restore,
  migration interruption and degraded-storage behavior on each claimed profile.
- [ ] Require all budgets, full repository verification, current-head review and
  required CI before a concrete owner-controlled live rollout. No background
  restart, automatic process adoption or destructive user-data trials.
- [ ] Record actual supported behavior in canonical guides; retain explicit
  unsupported-profile/residual-defect dispositions. Close the parent only after
  its required delivery criteria pass. #1249 remains an independent policy task.

## Investigated uncertainty and risk treatment

The least certain seams were external writers, activation-marker loss and
cross-file backup/deletion. Source review confirms direct writers remain in the
MVP while the API helper/native marker is not generally qualified; task 1 cannot
be bypassed. Independent draft reviews caught and resolved missing-marker fallback
and conflicting response limits. SQL cannot make existing external event appends
or payload deletion atomic; tasks 4/5 own explicit maintenance and recovery jobs.
No broader migration of independent stores is needed to obtain bounded display
queries. Runtime/profile and performance evidence remains to be produced.

| Dimension | Rating and evidence / treatment |
| --- | --- |
| Dependencies | 4 — installed reporters, sidecar and renderer require coordinated protocol negotiation; tasks 1/6 fence incompatibility |
| Blast radius | 5 — shared session persistence/lifecycle across consumers; dormant store then maintenance activation and complete-consumer regressions |
| State changes | 5 — authoritative format transition; preserved source/backup and failure-tested single activation |
| Reversibility | Unknown — same-version complete restore and post-cutover data recovery are specified but not yet qualified; task 4/5 evidence gates rollout |
| Uncertainty | 3 — source seams and review findings investigated; platform durability and fixed-workload measurements still unverified |

No total: reversibility evidence is incomplete. Ratings do not grant authority.

## Plan review and readiness

Inline `reviewing-plans` pass on 2026-11-01: **Ready for task-scoped implementation**
under the approved contract; activation remains gated by evidence. Scope, owner
seams, ordering and preservation checks map to the accepted design and linked
issue acceptance criteria. Verification ownership covers stale identity, compound
file failures, oversized rows/page progress, missing-marker recovery, off-page
selection and complete internal consumers. No new architecture choice or second
owner approval is introduced by this plan.

The 725-record live-list samples in the design are the only captured runtime
baseline so far. The complete synthetic/controlled baseline is task 8's first
deliverable and a release-readiness requirement; it is not reported as completed
by this planning PR. Unknown platform/benchmark results cannot be used to waive
the compatibility, backup, performance or live-rollout gates.
