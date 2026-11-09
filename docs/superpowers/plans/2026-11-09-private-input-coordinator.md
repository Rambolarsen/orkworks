---
type: Implementation Plan
title: Private finite input coordinator
description: Approved private coordinator and closed recipe enforcement using the existing input Entry/Core and actual scope/writer lifetime.
tags: [orkworks, terminal, ownership, qualification]
status: stable
workflow_status: locally-verified
---

# Private finite input coordinator

Tracking: [#1644](https://github.com/Rambolarsen/orkworks/issues/1644), parent
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
Spec: [approved coordinator recipe](../specs/2026-11-09-input-coordinator-recipe-design.md).
After #1641 the owner answered “keep going” to the explicit design/private-code
question. Acceptance, plan and actual code are one task; no repeat approval.

## Pass and boundaries

One existing Entry remains the truth owner. A private coordinator retains bounded
original routes, scope reservations, resource charges, unique job/slot handles
and fixed continuation state under that Core's transition lock. Closing races
admission atomically; late reports never renew a deadline. Unavailable production
registration/producer/profile adapters remain closed. Actual writer/join checks
leave real preparation/bookkeeping pending, not fabricated Completed.

No production factory/caller, AppState activation, real RootKey/readiness token,
command/graph/reset/model adapter, shipping default or native/SQL/model/resource
experiment; no maintenance/restore qualification. Local transition inputs are
cfg(test) only and do not qualify physical producers.

## Decisions and files

- Extend `runtime/input_submission.rs`: optional recipe in Entry/snapshot,
  optional coordinator registry in State, one shared locked admission primitive,
  retirement checks and charge release before resource Drop outside the lock.
  Legacy fixture admission cannot bypass a configured coordinator.
- Add `runtime/input_submission/coordinator.rs`: private Profile/Charges,
  bounded route registry, original scope pool, exact local Route references,
  coordinator admission/Closing/removal and one noncloneable job capability.
  Actual registration remains unavailable: only cfg(test) fixture registration
  can open a non-authoritative route. Default constructor refuses jobs.
- Add `runtime/input_submission/recipe.rs`: closed slot kinds, finite state,
  unique in-flight slot tickets, actual-clock admission, dependency and rich
  result selection. Completion inputs for absent adapters remain cfg(test) only.
  One original attempt per reset/tuple/disposal group in this schema; adapters
  needing a different/internal recipe are Unsupported, never retried implicitly.
  Initial graph claim, one notification and final claim have distinct slots.
- Extend immutable scope with exact same-binding and originating-budget checks;
  coordinator captures and failed admissions keep the same finite scope permit.
- Scoped tests in coordinator/recipe modules and actual writer reporting tests;
  canonical design/ADR/architecture/qualification docs record exact evidence.

All routing and slot identifiers are local fixture bookkeeping, not native root
or effect authority. Actual domain adapters must still provide their own captured
currentness/grant/physical-return contract before future production wiring. No
public completion API accepts arbitrary caller claims; test-only transitions are
explicitly local. Actual writer/return reporters remain the implemented owners.

Profile explicitly declares record/root/scope capacities and per-job/aggregate
byte/handle/notification charges. Check arithmetic and actual record/root vector
backing. Scope uses its existing real finite pool. Every admitted Entry retains
its full logical job charge until safe retirement; failed/protected jobs cannot
be evicted. This is local declared bookkeeping, not a physical fit measurement.
Shared allocator/native/payload/descendant evidence remains unqualified.

## Delivery and expected checks

- [x] TDD missing coordinator/recipe APIs; implement shared atomic admission with
  default Unsupported, exact route/scope/pool/serial checks, capacity/overflow,
  refusal preserving inputs, Closing admission race and no stale-route reuse.
- [x] TDD finite unique slot claims and dependency ordering, no clone/repeat or
  renewal, one-notification/final-claim graph, rich full/no-effect selection,
  expiry/cancellation/loss and retirement blocked by pending/protected work.
  State tests may supply local cfg(test) return values; they cannot discharge
  actual preparation/bookkeeping reporter obligations.
- [x] Exercise actual retained scope and optional writer/join through coordinator
  handles. Drop waiter/coordinator/job while flush held; old lease and late facts
  survive via actual reporters, with unresolved continuation state retained.
- [x] Run scoped cases and full `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh`,
  docs/worktree close-out and fresh current-code review. Required CI and complete
  PR/comment/Taskmaster inventory, merge and cleanup tracked by #1644/PR.

Expected red: new types/APIs absent, then guard tests exposing bypass or missing
slot/deadline/retirement checks. Expected green: meaningful local cases and full
repository checks; production capture/reporting call sites stay unchanged.

## Plan review and execution record

Self-reviewed once against all approved handoff checks. One inline writer; work
is sequential across the same Core/Entry, so no implementer fanout. New shared
interfaces are locked admission, recipe retirement and scope-pool binding.
Closing/admission use the same lock; no AppState/module call or resource Drop
under that lock. Fixture support cannot be a production Open constructor.

Risk: dependencies 2 (existing Core/scope/writer), blast radius 2 (private unwired),
state 2 (additive), reversibility 1 (no production callers), uncertainty 3 (new
finite transitions and lifecycle): 10/25. Physical/production rollback Unknowns
remain excluded and blocking for future activation.

Initial red build failed on absent coordinator/recipe APIs. Initial eight
coordinator cases then passed; runtime-only visibility was corrected before that
green build. The complete owner/scope/coordinator filter passed 40 cases, and
actual held-flush/join coordinator-loss test passed with real preparation and
bookkeeping pending.

Two self-found boundaries have observed assertion-red regressions: removed route
references could outlive reusable routing slots, and a committed reservation with
permanently rejected graph action remained only Pending. Route removal now drops
scope outside the lock but retains its finite slot until the unique routing handle
drops; active route loss closes admission and retains its original registration.
Rejected/no-effect graph action after committed reserve is protected for explicit
resolution. Scope pool/route serials remain exact. The route and graph fixes are
within approved finite-handle/protection semantics, not expanded scope.

Host Node22 prerequisite found fs-ext ABI mismatch in this new checkout; existing
dependency rebuilt under the required Node before full verification. No product
source, probe assertions or sandbox policy changed for setup. Full verification
and current-code review pending.


After both self-found fixes, all 40 owner/scope/coordinator cases passed; the actual
held-flush/join loss test also passed. There are 13 new coordinator cases and one
new actual-writer case. Fresh /code-review medium APPROVE found no findings and
verified the exact six-file source manifest. Medium is justified by >500 code/test
lines and coupled admission/slot/retirement lifetime across inseparable Core and
recipe modules; splitting these gates would leave an incomplete transition.
The review was read-only and ran no builds/tests. Production activation, actual
root/readiness authority and unsupported producer adapters were explicitly outside
the approved private scope, not silently qualified. Full verification pending.


Full `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` passed on the reviewed code:
3,459 Rust unit tests (18 existing ignored), all integration groups (1, 1, 4, 1),
31 OpenCode reporter tests, desktop type-check/Electron ABI/build, 1,721 desktop
tests (three existing skips), all 1,204 docs tests/OKF/index/site build and
whitespace/doc/worktree close-out. All 64 related cases passed (14 new).

Currency warnings need no public-doc/README edit: this private unwired change
adds no public behavior, milestone completion or installer claim; canonical
architecture/ADR/spec/qualification notes were updated. The worktree hook called
the new branch merged because its current base ref is in main before the first
commit; the uncommitted owned implementation is not merged and will be retained
until its actual PR terminates. Existing Rust and VitePress/NO_COLOR/chunk warnings
are unrelated to the diff. Current-code manual review passed; required CI, final
comment inventory, Taskmaster tie-off if applicable and cleanup still gate merge.
