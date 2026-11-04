---
type: Implementation Plan
title: Dormant evaluation lifecycle close plumbing
description: Conditional plan for exact optional revocation carriers and ordinary lifecycle close ordering without production binding attachment or storage activation.
tags: [orkworks, hierarchy, evaluation, lifecycle, authority]
status: draft
workflow_status: proposed
---

# Dormant evaluation lifecycle close plumbing

Tracking: [#1380](https://github.com/Rambolarsen/orkworks/issues/1380), under #1176.
Spec: [close-only proposal](../specs/2026-11-04-evaluation-publication-authority-design.md#close-only-lifecycle-proposal--2026-11-04).
Source base: `90bc6cb824cfe2125d235a2be406a7847ebb9c2d`.

## Outcome and authority

Pass: every inventoried ordinary terminal root closes its exact injected carrier
before irreversible stop or teardown; replacement/rollback cannot revoke a new
runtime or rearm an old carrier. All production carriers remain None, and no
production creator, attachment, registration, response/claim/publication or store
path is added. Current ordinary-session behavior remains available.

**Owner acceptance of this concrete scope is pending. Do not execute.** Earlier
approval was for the merged non-I/O core only. The conditional plan below does
not relax active source/native/resource or complete-profile qualification.

## One coupled implementation unit

Ownership: one inline implementer owns the facade, call sites and tests together.
No parallel writers. Preserve the core's accepted claim/disposition invariants;
introduce no new dependency, codec, bearer, quota, state migration or wire API.

Files/responsibilities:

- `taskmaster/publication_authority.rs` and a private adjacent lifecycle facade:
  opaque close-only runtime/workspace handles retaining exact owner references;
  fixture-only factories/attachment, no source or settlement API for runtime code.
- `main.rs` WorkspaceState and `runtime/session_runtime.rs` SessionRuntime:
  optional carrier fields initialized to None; capture into startup/driver owners
  with a single non-Clone task guard transferred across successful startup.
  AppState gains no authority initialization in this unit.
- `session_application.rs`, `session_actions.rs`, `runtime/session_runtime.rs` and
  `runtime/terminal_runtime.rs`: cover the spec's full root table, preserving
  existing action reservations, exact generations and bounded ordinary waits.
- Existing scoped test modules or adjacent lifecycle tests: exercise real stop,
  startup/exit, replacement/rollback and workspace entry points with synthetic
  private handles. Update ordinary fixture constructors with None where needed.
- Source-only SQL lifecycle and EndingPersistence: inspect for bypass/backstop
  coverage; do not change storage selection, execution tickets or SQL semantics.

The facade consumes an existing exact core Binding or Scope only through its
cfg(test) factory. Ordinary code can clone/capture and close a handle, never
construct, open, prepare, claim or settle one. Preserve retired owner/claim
references outside every latch and ordinary guard; no callback/I/O/destruction
inside the publication latch.

## Behavioral oracles and execution

- [ ] After scoped owner acceptance, record its exact design/plan revision and
  amend ADR 0090 in a dated section before runtime code: the private-worker/storage
  direction and active-qualification gates remain unchanged; this authorizes only
  dormant revocation routing. Follow the existing ADR lifecycle, not a status
  change or implied approval of a production adapter.

- [ ] Write failing entry-point tests for delete, row Close/Archive/Delete and
  terminal Kill. Propagate the originating attachment identity through delayed
  terminal dispatch. A channel/barrier records close before kill; stale identity
  rejects signaling a replacement. Test both present and None carriers.
- [ ] Add failing pre-spawn cancellation/failure, post-spawn abort, native observer,
  writer/editor failure and natural exit tests. Verify close precedes kill or
  drain/finalization waits even while the ordinary metadata transition is parked.
  Cover duplicate callbacks, driver channel closure/select exhaustion, cancellation
  before first poll and during startup/driver waits, and exit before any possible
  test registration. A task guard closes on teardown outside ordinary guards;
  dropping a cloned carrier alone must not close authority.
- [ ] Add failing resume admission/rollback and creating-record failure tests:
  old binding closes before exchange; abandoned replacement closes before removal;
  restoring an ordinary handle never reopens it; stale rollback leaves a newer
  runtime unchanged. No restored test binding may register after its close.
- [ ] Add failing workspace race tests: close-before-exchange, exact instance
  revalidation, same-path reload, failed adoption, two concurrent adopters and
  old unsettled claim retention. Neither stale completion nor failed ordinary
  persistence clears a newer fence. No Arc/lease destruction under close latch.
- [ ] Implement the smallest facade/carriers and capture→close→revalidate tails
  until those tests pass. Never acquire the latch with sessions/workspace/prompt
  locks held. Reuse accepted projection→publication order where atomic admission
  needs it; keep full metadata and store construction outside the close latch.
- [ ] Prove poisoned-owner denial does not disable ordinary stop/cleanup. Re-run
  the 23 core tests and assert no production constructor/attachment/open/claim
  caller by source inventory and compile privacy boundaries. Every production
  field initializer is None; no entropy/backend/source route is activated.
- [ ] Run focused tests, Rust fmt/build/tests and `bash scripts/verify-repo.sh`
  with frozen dependencies/required Node ABI; docs/worktree checks. Inspect all
  changed lock lifetimes and the complete source-root table against final diff.
- [ ] Complete current-code `/code-review medium` for this cross-cutting lifecycle
  change, then PR/required CI/comment follow-through and owned cleanup. Keep
  #1176 and production-admission gates open; report routing evidence only.

No sleep-based ordering assertion, live model/native probe, actual storage,
physical lease-loss or crash-surviving ownership evidence is part of these tests.
If implementing a listed root needs an unapproved policy or actual attachment,
stop that dependent change and return the concrete scope question; do not make
an eligibility boolean, generic registration API or partial active adapter.

## Review and risk

Review disposition: **Ready for scoped owner decision; not execution.**
Spec coverage: all inventoried ordinary roots have an oracle above. Future run,
namespace, health, shutdown and physical source owners remain activation gates;
None carriers do not pretend to qualify them. One coupled unit is required
because partial close coverage would be misleading even in integration tests.

Dependencies 4 (ordinary admission/driver/workspace and private authority);
blast radius 4 (shared lifecycle lock tails); durable changes 1 (none);
reversibility 2 (source revert with no format/data migration); uncertainty 3
(actual entry-point scheduling remains untested). Sum 14, a planning aid only.
The full active profile's native/recovery reversibility remains Unknown.

One fresh read-only Luna builder check completed. Originating terminal identity
and task-cancellation guard coverage were clarified; the reviewer found no
remaining material builder question or lock-order contradiction. The workspace
lock-tail refactor is explicit, not assumed already implemented. No unchanged
upstream design or active qualification gate was reopened. Self-review maps all
inventoried roots to tests and preserves fixture-only attachment/privacy.

Documentation verification: frozen docs install under Node 22.23.4, 1,202 existing
docs tests, metadata/index/link/site build and doc/whitespace checks pass. Rust
and native tests were not run for this docs-only proposal. Baseline VitePress
chunk warnings do not concern changed content. Worktree warnings include foreign
branches, which are untouched; this owned uncommitted branch initially appeared
merged because it still pointed at the base before its first commit.

