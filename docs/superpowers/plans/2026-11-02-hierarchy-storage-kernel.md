---
type: Implementation Plan
title: Hierarchy storage owner kernel for the child-source slice
description: A dormant SQLite-backed hierarchy_store module owning rows/receipts/ledgers for the accepted child-source slice, with strict reload and one-transaction publication, wired only to tests.
tags: [plans, hierarchy, storage, sqlite, child-sources]
status: draft
---

# Hierarchy storage owner kernel for the child-source slice

Outcome: one private, dormant Rust owner validates and publishes the accepted
child-source storage rows — objects, dependencies, the four child metadata
tables, quota rows and the commit chain — behind typed producer commands, with
strict reload semantics and no production caller.
Pass condition: local SQLite round-trip and recovery tests against merged
child-source fixture bytes pass, all out-of-scope tables/catalog kinds are
rejected, and no route/validation/launch/migration surface changes.
Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176),
[#1183](https://github.com/Rambolarsen/orkworks/issues/1183), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).

## Accepted basis and honored offsets

Use only what is already owner-accepted:

- Architecture direction: one selected-workspace SQLite database under one
  private storage worker, DELETE journal mode, `synchronous=EXTRA` ([accepted
  scoped direction](../specs/2026-10-23-hierarchy-transaction-storage-design.md#scoped-architecture-direction-acceptance--2026-10-24)).
- Row shapes and closed catalog for the child-source slice: [strict storage
  representation](../specs/2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation)
  (objects/dependencies/commit_proof/quota_usage) plus the accepted child
  additions ([child-source reading](../specs/2026-10-24-hierarchy-storage-contract-design.md#scoped-child-source-choices-accepted--2026-11-02):
  `child_plan_approvals`/`child_reservations`/`child_allocations`/
  `child_attachments`, the `child-source-provenance` dependency purpose, and the
  8 MiB/run / 64 MiB/workspace source pool).
- Component bytes and row joins: the [child-source component packet](../../validation/child-source-components.md)
  supplies fixture canonical bytes/digests; the root package
  (`root-bootstrap-components.mjs`, `root-preparation-components.mjs`,
  `hierarchy-claim-proof-vectors.json`) supplies shared byte conventions.
- Transition/preparation effects and consumer/ledger duties: the accepted
  [effects derivation](../specs/2026-10-31-child-review-source-preparation-design.md#transition-ownership-and-preparation-effects--2026-11-02)
  (zero initialization order, exactly-required effects, unavailable-on-missing).
- Authority/commit ordering gaps: [no explicit no-go](../specs/2026-10-24-hierarchy-storage-contract-design.md#authority-and-commit-ordering-explicit-no-go)
  machinery is inherited; this kernel contains no #744 receipt-slot machinery
  where that no-go lives.

Known unresolved items that deliberately remain open after this kernel (they do
not block it, but they gate later slices): the #1184 read-work physical-commit
cutoff; worst-case whole-command accounting beyond these small rows; research
report/#743 ledger shapes; payloads/heads/summary machinery; native durability
evidence beyond journal/`EXTRA` defaults; producer routes and UI.

## Scope decisions and boundaries

In the first (and only) implementable unit, the storage owner owns and refuses
everything outside:

1. **Connection/config owner** `hierarchy_store::Connection`: open `<workspace
   metadata>/hierarchy/v1.sqlite` with validated `page_size=4096`,
   `max_page_count=262144`, DELETE journal, `synchronous=EXTRA`, foreign keys on,
   `temp_store=MEMORY`, one retained worker connection, no `ATTACH`, no
   extension loading, no caller SQL.
2. **Schema owner**: create/verify exactly the accepted tables
   `store_header`, `objects`, `dependencies`, `child_plan_approvals`,
   `child_reservations`, `child_allocations`, `child_attachments`, `quota_usage`,
   `commit_proof`. No other table may exist after open; unknown tables/columns/
   versions make the store unavailable. Storage epoch and
   `store_header.latest` initialize only on a fresh empty database.
3. **Row validation**: canonical JSON BLOB keys with byte equality (no
   collation), strict child-source `ObjectKey` registry (child source kinds
   only), closed field checks against the owning contracts, u64 `Counter` as
   checked canonical decimal TEXT in Rust, per-kind body limits (plan ≤64 KiB,
   configuration ≤1 MiB, input snapshot ≤256 KiB per owning contracts), the
   16 KiB complete key/value pair wrapper cap for the four `child_*` metadata
   tables only, 2 MiB per-row ceiling, domain digest verification on
   admission/reload (row commitment domain `orkworks.hierarchy-storage-row.v4`
   where the accepted format-4 catalog names one), immutable objects never
   overwritten with different bytes.
4. **Commit chain and atomic publication**: one transaction per producer
   command covering every changed row + its `lastCommit` CommitRef, dependency
   changes, quota deltas and the new `commit_proof` entry; manifest identity
   over [`orkworks.hierarchy-storage-change.v4`](../specs/2026-10-30-evaluation-artifact-producer-design.md#accepted-format-4-row-and-commitment-domains)
   per the accepted format-4 row/manifest domains (row commitment domain
   `orkworks.hierarchy-storage-row.v4`) — small payloads do not make the older
   v1 commitment format interchangeable; monotonic commit sequence; exhaustion
   fences rather than wraps.
5. **Reload/recovery semantics**: every open revalidates epoch/workspace, chain
   continuity, all retained proofs and every row's shape/digests; any
   missing/ambiguous/corrupt state fences writes and reports unavailable —
   never repairs, truncates, migrates or recomputes from latest rows.
6. **Quota rows**: `child-source-retained-bytes` only, as `(owner, resource)`
   rows with checked Counter arithmetic in Rust, `reserved="0"` semantics per
   the accepted fresh-preparation order, missing/ambiguous counts fence writes.

Explicitly out of scope (reject or refuse; never implement here): payloads,
heads, summary_state, pending_material, receipt_slots, candidate_work; #743
research-report ledgers; `artifact_read_debits` kernel rows (owner handoff
recorded instead); any HTTP route, renderer/Electron surface, producer
integration with `create_session_workflow`, `WorkspaceLease` enforcement,
armored claim/authority ordering, WAL mode, backups, migration of any older
store, model/native probes and any production caller at all. The module is
wired only to Rust tests.

## Deliverable map

All new under `crates/orkworksd/src/taskmaster/hierarchy_storage/` plus one
`mod.rs` registration in `crates/orkworksd/src/taskmaster/mod.rs`; existing
modules are not modified (mirroring the dormant `root_bootstrap` precedent):

| Item | Deliverable |
| --- | --- |
| `connection.rs` | DDL-free `Connection::open`/config verification per scope 1; unavailable reasons instead of repairs |
| `schema.rs` | Accepted-SQLite-DDL table creation/verification; unknown-object refusal guard |
| `rows.rs` | `Counter` TEXT, canonical BLOB key/value validation, child kind registry, byte caps, domain digest and reference joins |
| `publication.rs` | Typed producer commands for the accepted transitions: publish plan/configuration/input/assignment-source objects, child metadata rows, provenance edges, quota deltas (one transaction each, change manifest + proof entry) |
| `reload.rs` | Full reopen recovery: epoch/workspace/chain/proof/row revalidation, write fence on any unavailable state |
| `quota.rs` | `child-source-retained-bytes` checked counters (`used`/`reserved`) with 8 MiB run / 64 MiB workspace rejections and no-refund semantics |
| `tests.rs` (+ fixtures) | The acceptance cases below, reusing `docs/validation/child-source-vectors.json` bytes as golden inputs |

## Acceptance criteria and checks

1. **Open/verify (scope 1–2)** — fresh empty path creates the accepted
   schema; reopen verifies and continues; presence of any foreign table/column
   or unknown schema version returns an unavailable error, never repairs.
   Verify with a `#[test]` using a tempdir: nonempty file, foreign table, and
   corrupt page-count cases.
2. **Row admission (scope 3)** — each of the nine golden row kinds from
   `child-review-source-vectors.json` plus wrapper rows parses, reconstructs its
   key, verifies its domain digest and is accepted; each of the rejection
   classes (unknown/extra/duplicate fields, wrong kind root substitution,
   invalid counter TEXT, oversized wrapper/row, digest mismatch, broken join,
   noncanonical stored bytes) rejects before persistence with the merged
   checker's reasons. The 16 KiB wrapper cap applies to child metadata rows
   only — an above-16-KiB valid source object body (e.g. a 32 KiB plan body)
   is an acceptance case, not a rejection. Verify with parametrized
   positive/negative `#[test]`s.
3. **Publication (scope 4)** — publishing the worker chain: objects + edges +
   the four metadata rows + quota deltas + `commit_proof` land in one
   transaction; a second publish of the same object body with different bytes
   conflicts; a mid-command failure (injected via test seam) rolls back every
   row, leaving header/latest unchanged. Verify with round-trip + rollback
   `#[test]`s and exact format-4 manifest digest recomputation.
4. **Reload/recovery (scope 5)** — after every publish, reopen revalidates and
   continues; deleted `commit_proof` row, truncated body or mismatched epoch
   opens unavailable and rejects a subsequent write. Verify with reopen/
   corruption `#[test]`s.
5. **Quota (scope 6)** — a publish charging the owning run beyond 8 MiB and the
   workspace beyond 64 MiB in two separate increment steps rejects visibly with
   complete old state; `reserved` rows deactivate per accepted effects; missing
   quota rows fence rather than autofix. Verify with arithmetic `#[test]`s.
6. **Dormancy/no-go** — the production surface gains no callers: a recorded
   check in the PR body shows `hierarchy_storage` is referenced only by tests;
   `cargo test -p orkworksd` passes with the module dormant.

Level checks: `rustfmt`/`clippy -D warnings`, `cargo test` (full suite),
`bash scripts/doc-check.sh`, `pnpm --dir docs docs:check` and `docs:build` for
the plan/handoff documents, and the format-4 change-manifest byte fixture test
using the merged storage-contract oracle bytes where present.

## Dependencies and sequencing

Sequencing inside the unit: connection/schema → rows → quota → publication →
reload → tests, with each step built green before the next. External
dependencies: none new (SQLite available via `git2`'s linked sqlite3 or a
dependency decision recorded below).

Material decisions left to implementation, bounded here:
- SQLite driver: the existing `rusqlite = { version = "0.32", features =
  ["bundled", "hooks"] }` (`crates/orkworksd/Cargo.toml:37`) is reused — no new
  runtime dependency is introduced. The `hooks` feature stays unused; enabling
  extra features is out of scope.
- Concurrency: the retained connection follows the `CoordinatorStore` precedent
  (`mutation: Mutex` around a private connection, `crates/orkworksd/src/taskmaster/
  coordinator_store.rs:108`); the module exposes no parallel writer. A later
  worker slice introduces the private worker if needed; this unit does not.

## Risks, recovery and no-go boundary

- **Canonical-byte drift between Rust and fixture checkers**: mitigated by
  reusing the merged packet vectors as golden inputs; any mismatch fails the
  build rather than silently normalizing.
- **Unexpected SQLite behavior** (page/journal/locking): out of this unit's
  claims — no native durability or concurrency evidence is asserted; the kernel
  only follows the accepted DELETE/EXTRA configuration and defers qualification
  evidence to a later slice.
- **Recovery repair temptation**: recovery never heals; every ambiguous state
  pins the store unavailable. Tests must show recovery leaves bytes untouched.
- **Scope creep into claim/receipt machinery**: the #1184 ordering and #744
  receipt-slot machinery stay out; a PR adding them is out of scope by default.
- **Old-store migration**: never present here; unknown older formats remain
  unsupported forever within this slice (accepted).
- Recovery/rollback decisions use the store's own durable journal; no extra
  external cleanup path is introduced.

## Coordination and user-acceptance separation

This kernel supplies storage mechanics only. It cannot launch work, retry,
widen permissions, advance dependencies, accept results, or weaken the
accepted producer/qualification gates. Producer-side authority and native
fences stay separately gated; getting rows into storage proves nothing about
eligibility. Component ownership stays with the module; no session/parent state
writes, no YAML/JSON mirrors, no UI.

## Handoff

Following the [local plan policy](../../agents/development-workflow.md#plan-complexity-and-quality),
run the acceptance checks at PR time and record results in the PR body; the
defined owner-review gate is #1176's acceptance of this scoped plan before the
merge is attempted; official acceptance wording is recorded on the PR.
Implementation issues (if any) link to #1176/#1183. Keep the
[qualification handoff](2026-10-31-child-review-source-qualification.md)
narrative aligned — update it only after the kernel lands, not before.

Owner decisions requested at review: (a) confirm the driver choice (reuse the
existing `rusqlite`); (b) confirm reduction of scope to these nine tables;
(c) confirm that the manifest metadata envelope may be adapted
byte-for-byte from the accepted format-4 contract for these small,
uniform payloads (the domain versions stay v4; no new bump is proposed). Everything else is mechanical.

## Plan review (inline)

Recorded per the [local plan policy](../../agents/development-workflow.md#plan-complexity-and-quality);
a completed review is not an approval and authorizes nothing.

- **Scope/simplicity.** One dormant module, no production callers, no new
  dependencies; the boundary table refuses out-of-scope machinery explicitly
  (payloads/heads/summary/receipt slots, routes, probes). Splits that ship
  ledger-only or publication-only slices were rejected because they leave the
  commit chain untested end-to-end.
- **Clarity and decisions.** Named sources back every config/config-table/cap/
  effect choice; three items still requiring an owner call are explicit. The
  one ambiguity found — `commit_proof` retention breadth — is resolved in favor
  of the merged contract's wording (latest plus retained-named proofs); no new
  rule is invented.
- **Requirement/gate coverage.** Tables, caps and joins trace to accepted
  clauses; the merged design's hard no-gos (unknown products, collation keys,
  WAL mode, caller SQL) are carried verbatim into row/admission rules. #1184
  and native durability evidence remain named open gates rather than silently
  dropped.
- **Verification.** Golden bytes come from the merged packet (independent
  generator, not this plan); rejection classes mirror the checker's reasons;
  dormancy is mechanically checkable. The 16 MiB/4,096-row whole-command
  accounting is re-bounded before any larger payload slice.
- **Open risks.** Recovery fencing depends on never repairing; tests must prove
  bytes are left untouched. Two upstream proposals (child launch controls;
  #1184) remain unaccepted — if their final fields differ, that is a new
  domain-version bump for the closed registry, never an in-place change.

Status: ready for owner review. Implementation remains unauthorized until the
owner accepts the reviewed plan.
