---
type: Implementation Plan
title: Dormant evaluation lifecycle close plumbing
description: Conditional plan for exact optional revocation carriers and ordinary lifecycle close ordering without production binding attachment or storage activation.
tags: [orkworks, hierarchy, evaluation, lifecycle, authority]
status: draft
workflow_status: active
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

The owner answered “go ahead” to the exact inline implementation request after
merged PR #1386 (head f4775d3032ff78f29efae0386bc0894bb5a2ede0; merge
9d67ee0f6ae6898c9dd7b6b33ce06156cdb32551). This approves this close-only unit,
tracked by [#1389](https://github.com/Rambolarsen/orkworks/issues/1389), with
[ADR 0090's dated amendment](../../adr/0090-workspace-hierarchy-sqlite-transactions.md#amendment--dormant-lifecycle-revocation-routing-2026-11-04)
recorded before runtime code. Active source/native/resource and complete-profile
qualification remain unchanged.

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

- [x] After scoped owner acceptance, record its exact design/plan revision and
  amend ADR 0090 in a dated section before runtime code: the private-worker/storage
  direction and active-qualification gates remain unchanged; this authorizes only
  dormant revocation routing. Follow the existing ADR lifecycle, not a status
  change or implied approval of a production adapter.

- [x] Write failing entry-point tests for delete, row Close/Archive/Delete and
  terminal Kill. Propagate the originating attachment identity through delayed
  terminal dispatch. A channel/barrier records close before kill; stale identity
  rejects signaling a replacement. Test both present and None carriers.
- [x] Add failing pre-spawn cancellation/failure, post-spawn abort, native observer,
  writer/editor failure and natural exit tests. Verify close precedes kill or
  drain/finalization waits even while the ordinary metadata transition is parked.
  Cover duplicate callbacks, driver channel closure/select exhaustion, cancellation
  before first poll and during startup/driver waits, and exit before any possible
  test registration. A task guard closes on teardown outside ordinary guards;
  dropping a cloned carrier alone must not close authority.
- [x] Add failing resume admission/rollback and creating-record failure tests:
  old binding closes before exchange; abandoned replacement closes before removal;
  restoring an ordinary handle never reopens it; stale rollback leaves a newer
  runtime unchanged. No restored test binding may register after its close.
- [x] Add failing workspace race tests: close-before-exchange, exact instance
  revalidation, same-path reload, failed adoption, two concurrent adopters and
  old unsettled claim retention. Neither stale completion nor failed ordinary
  persistence clears a newer fence. No Arc/lease destruction under close latch.
- [x] Implement the smallest facade/carriers and capture→close→revalidate tails
  until those tests pass. Never acquire the latch with sessions/workspace/prompt
  locks held. Reuse accepted projection→publication order where atomic admission
  needs it; keep full metadata and store construction outside the close latch.
- [x] Prove poisoned-owner denial does not disable ordinary stop/cleanup. Re-run
  the 23 core tests and assert no production constructor/attachment/open/claim
  caller by source inventory and compile privacy boundaries. Every production
  field initializer is None; no entropy/backend/source route is activated.
- [x] Run focused tests, Rust fmt/build/tests and `bash scripts/verify-repo.sh`
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



## Execution evidence — 2026-11-04

Implementation base: f11e325f6c442f4bf17b1b7e35c76a14d6587d74; preserve its
namespace-response fence fix. One inline owner; every production runtime/workspace
initializer remains None and all source factories/attachment are cfg(test).

Observed RED→GREEN: initial 7 tests had 5 behavioral failures/2 controls passed;
expanded 11 had 8 failures/3 controls. Exact stop/row/terminal snapshots, workspace
and resume tails made all 11 pass. Actual startup cancellation before first poll,
during wait and failed spawn then failed, as did a delayed attachment under an
ID-only kill mutation; restored exact identity/owned-future capture made 15 pass.
Further deterministic tests cover held-latch lock release, stale workspace tail,
retained old claim, unissued late registration and creating-record failure.
Three deliberate missing-close/lock-order mutations failed exactly their three
oracles and were restored. Exact retained-lease replacement exposed a real
reader-only revalidation gap; the fixed snapshot compares both reader and lease
Arc identities without claiming physical lease qualification.

The local PTY fixtures exercise natural exit before notification/finalization,
driver abort, post-spawn ownership loss, writer panic, initial-editor rejection
and a synthetic native-server failure. Writer/editor and native omission
mutations each failed their cutoff-before-kill oracle and were restored. Fixtures
disable background inference; generated local processes are explicitly cleaned.
These are unit/integration tests, not hosted calls or native platform/profile
qualification. Existing core claim/settlement semantics are unchanged.

Ruling: preserve the ordinary workspace guard while opening/recovering stores,
then release it before projection and the publication cutoff. RecommendationStore
open recovers/migrates files; moving that work outside existing writer fencing
could introduce same-path recovery/write races. This changes the draft's proposed
preparation lock lifetime, preserving existing I/O fencing while keeping every
publication latch outside workspace/sessions/prompt guards. Cost if wrong is the
existing workspace-preparation I/O responsiveness, not new authority. Actual
selected workspace capture occurs under projection after preparation, allowing
concurrent prepared adopters to serialize their exchange.

Full repository verification and current-code manual review are recorded below
when complete; no success is inferred from this execution ledger.


Focused final result: **28 passed, none ignored**. A writer fixture was corrected
because portable-pty 0.8.1 writer Drop sends newline/EOF; it now retains the real
writer until test cleanup. With that correction, omitting either writer/editor
cutoff again produces exactly its two expected failures. Restored source SHA-256
matches the 28-pass source bytes before full verification. The synthetic native
failure omission likewise failed its dedicated first-cutoff test. No mutation
remains in the implementation.


Independent whole-branch `/code-review medium` completed: **APPROVE, no findings**.
Reviewed base f11e325f6c442f4bf17b1b7e35c76a14d6587d74 and staged tree
c46c5a686aad454148cbb4bee88764f346946d8d in a fresh read-only Luna context.
The reviewer inspected actual code/tests and relevant caller edges; tests/build
were not rerun independently. Parent focused tests passed 28/28; full repository
verification is still pending at this review checkpoint. Production source/
attachment, claim/response/publication and run/shutdown/native capability gates
were explicitly left outside this dormant unit, as approved. No optional or
minor findings were deferred.


Full-suite correction: the first repository run passed fmt/build but exposed an
HTTP deadlock: set_workspace_inner already held projection while the new
application adoption tail reacquired it. A bounded real HTTP regression failed
at its five-second deadline. Application admission now owns that mutex exactly
once; HTTP no longer wraps it. Early/final poison checks preserve HTTP 500 and
prevent preparation on an already poisoned projection. The application retains
the former HTTP fence through ordinary recovery, then releases it before retired
resources are destroyed.

A separate three-second channel regression proved terminal Kill was unnecessarily
waiting on unrelated projection work. It retains its original no-projection
control path: exact captured identity is rechecked under sessions after close.
Delete/row/replacement operations keep their existing projection admission.
These are fixes within the approved ordering/ordinary-availability contract.
Focused result after both corrections: **31 passed, none ignored**. The original
c46 review is historical for the earlier tree; the current delta and full suite
are reviewed/verified again before commit or merge.


Updated current-code `/code-review medium`: **APPROVE, no findings**, staged tree
63a176d24b184622e5958dac3e8515df146876de. The fresh whole-branch reviewer then
checked the four-file correction delta against c46 and its HTTP/workspace/terminal
caller edges; all other reviewed Rust blobs are unchanged. The reviewer also
checked the synchronous transition backstop against actual caller lock lifetimes
and found no material executor-blocking risk. Tests/build were not independently
rerun by the reviewer. The parent reruns the full repository check after the
observed regression fixes; this current approval does not turn the interrupted
first run into a passing result.


Full `bash scripts/verify-repo.sh` passed on the corrected code: Rust fmt/build,
2,895 Rust unit tests passed (17 existing ignored) plus all integration suites,
20 OpenCode reporter tests, desktop type/native/tests/build (1,690 passed, three
existing skipped), and 1,202 docs tests plus metadata/index/link/site build.
Diff/doc/worktree checks pass. Baseline compiler/Node/VitePress warnings are
unrelated. Doc-check's user/onboarding and README currency warnings require no
additional edit: the UI/metadata schema/milestone and published capability claims
are unchanged; source architecture documents dormant routing and exact terminal
identity. No runtime/profile or installer activation is claimed.

Code blobs remain identical to reviewed tree63; only documentation result records
changed afterward. PR CI and owned branch/worktree cleanup remain delivery steps.
