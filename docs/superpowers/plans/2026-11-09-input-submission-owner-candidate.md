---
type: Implementation Plan
title: Source-only input submission owner candidate
description: One unwired private Rust owner with bounded records, unique phase reporters and independently retained input facts and completion.
tags: [orkworks, sessions, terminal, ownership, qualification]
status: stable
workflow_status: verified-candidate
---

# Source-only input submission owner candidate

Tracking: [#1589](https://github.com/Rambolarsen/orkworks/issues/1589), parent #1250.
The owner approved the [single-candidate exception](../specs/2026-11-09-submission-owner-qualification-sequencing.md)
after #1587 by answering “lets keep going” to the explicit implementation-scope
question. This approval covers a reviewed plan then this one source-only module;
no separate acceptance-only PR or repeat design approval is needed.
Spec: [input truth/completion](../specs/2026-11-08-terminal-input-outcome-interface-design.md).
Pass: actual private handles enforce local accounting/outcome rules with offline
tests; no production callers, physical effects or qualified profile are introduced.

## Scope and interface decisions

Create `crates/orkworksd/src/runtime/input_submission.rs` and its scoped
`input_submission/tests.rs`; declare the private module in `runtime/mod.rs`.
Use only the Rust standard library. Implement inline; no implementer delegation.
No changes to existing writer, terminal, application or HTTP paths.

- `Owner` holds one private mutex-protected fixed-capacity slot vector. Constructor
  requires explicit finite limits for records, observers, declared input length
  and backing-storage bytes. Check multiplication/actual vector capacity before
  use; no per-admission variable allocation or dynamic report history. Backing
  bytes are logical capacity, not allocator/native/RSS fit. Fixed record/result
  fields need no variable result/string budget; no prompts are retained.
- Fixed-size `Context` is opaque data (root/session/incarnation/slot), not native
  identity/authority. Admission rejects duplicate live context, expired deadline,
  oversized declared length or capacity exhaustion atomically. Allocate a checked
  owner-local serial; every handle binds slot+serial so reused storage is stale
  to old observers/reporters.
- Admission reserves four exact reporting roles: preparation, writer, physical
  return and bookkeeping, plus one observer. They are already admitted local
  reporting obligations, not effect permits. Reports after expiry settle that
  original obligation; no future slot/deadline extension exists.
- Each non-cloneable reporter can settle only its own phase. Only preparation
  (known no-input-effect) and writer roles can record input facts. Fixed input
  facts distinguish unresolved, no input effect, full write with flush disposition,
  and partial/unknown reported prefix. Validate prefix against original length.
  Same report is idempotent; conflicting reports preserve prior truth and protect
  the record. A phase failure/lost role is protected, never successful completion.
- Observer snapshot is read-only, explicitly counted and fallible on stale/poisoned
  state. Dropping it releases only observer capacity. Owner retains the record.
  Retirement requires known input truth, all phases known successful/no-effect,
  no protection and no live reporting handles. Each record tracks exactly four live reporters;
  admission reserves all four atomically, bounding reporters by four times the
  fixed record capacity. Drop decrements that handle count and marks an unsettled
  phase lost/protected. Lost records cannot retire or be reused; no force-clear
  or fabricated resolution exists in this candidate. Consuming the result frees only
  that record. Retired observers stay counted until dropped; old serials cannot
  observe or alter new records. Unknown/partial records have no force-clear API.
- No subscriptions, background tasks, I/O, waits, serialization or production
  defaults. Snapshot is one bounded read; no polling loop is introduced. Source-
  only suppression of dead-code warnings documents the deliberate lack of callers.

## One implementation task, three checks

- [x] Write failing behavioral tests first through candidate handles. Start with
  record retention after observer loss and independent writer/full/physical phase
  completion; confirm failure is due to the missing candidate. Add boundary cases
  before their implementation: wrong reporting role/conflicting facts, partial/
  unknown protection, role loss, deadline admission/reporting, duplicate context,
  record/observer/storage/input limits, stale storage reuse and checked overflow.
- [x] Implement the one owner and minimal module declaration; run scoped tests
  red-to-green. Use fixture-sized explicit parameters, not production allowances.
  Include real in-process concurrent reports/observer loss without mocked counters.
  No test mutates internal state to manufacture settled producer evidence.
- [x] Verify formatting/build/scoped tests and full `scripts/verify-repo.sh` with
  pinned Node and frozen desktop/docs dependencies. Existing Rust/desktop test
  fixtures are repository verification, not native/model qualification. Inspect
  any unexpected separately gated execution before running it; no live session,
  real coding harness, model or retained-store experiment is authorized. Run
  docs/close-out checks and fresh current-diff `/code-review low` before handoff.

Source review must verify that only module declaration and scoped tests reference
the candidate, with no production construction. Record exact tests/results and
remaining limitations in this plan and the canonical qualification plan. Update
acceptance status/ADR/index in this same logical task; complete required CI/PR
workflow without claiming production integration or complete physical fit.

## Review focus and risk

All five material classes have behavioral checks above: loss versus settlement;
input truth versus physical return; stale/duplicate reporting; resource exhaustion/
overflow and reuse; expiry versus original completion. Partial/unknown cannot be
retired or reused by sampling zero live handles. Prepared context is fixed-size
and carries no qualified root or input authority.

Dependencies 2 (std-only local owner, real integration deferred); blast radius 1
(unwired module); state changes 1 (in-memory candidate only); reversibility 1
(delete unused declaration/module); uncertainty 2 (bounded concurrency/retention
needs tests). Total 7. No production recovery/physical qualification is implied.
Fresh plan review requested explicit live-reporter accounting. Clarified four
fixed handles per record, tracked drop/loss and retirement exclusion; tests cover
capacity with live/lost reporters. No scope expansion or new production bounds.
Disposition: Ready for this candidate only. First red/green checks may use
`rustc --edition=2021 --test crates/orkworksd/src/runtime/input_submission.rs`
with an output under `/tmp`; it compiles the actual std-only module directly.
Cargo/full-repository checks still follow and are not replaced by that fast loop.


## Execution evidence

Initial tests failed because the candidate types/operations did not exist. The
fast harness nests the real source module (its `pub(super)` interface requires
a parent module); no visibility was widened for tests. Ten initial behavioral
cases passed. Added cross-phase contradiction and observer-visible unknown
protection assertions failed before their fixes, then all 12 cases passed. The
scoped Cargo target compiled and passed; final full-repo verification follows.

All four local reporting obligations are admitted together at record admission;
reports may settle after deadline, while later record admission rejects expiry.
This candidate admits no physical effect or new child slot. It stores fixed context
and results only, not prompt payloads, native handles or production profiles.


## Current-diff review

Fresh whole-code `/code-review medium` completed with no material findings. Medium
was selected for concurrent lifetime rules and roughly 750 code/test lines in
one inseparable candidate (three code files), not for production integration.
The reviewer independently ran all 12 initial fast module tests and all 13
after the admission-clock regression fix. The refreshed review approved the
current code without material findings. Exact reviewed file
SHA-256 values are retained in the PR evidence and checked against final code
before commit; documentation updates do not alter that reviewed code diff.
No production callers or effectful imports were found.


## Final verification and limits

`scripts/verify-repo.sh` passed: formatting/build; Rust unit tests 3,334 passed,
18 ignored, zero failures, plus all integration test binaries; OpenCode reporter
31 passed; desktop type-check; Electron ABI check; desktop tests 1,709 passed,
3 skipped, zero failures; desktop build; all 1,204 docs tests and metadata/index/
site build; whitespace and close-out checks. No ignored/skipped case was added
by this candidate. Fresh code review's exact three-file manifest still matched.

The documentation-currency nudges for user/home/README require no edit: no user
behavior, release availability, installer or runtime production path changed.
Only source-only candidate/approval/plan evidence is added. Existing source-only
SQL unused/dead-code and build chunk/NO_COLOR notices do not concern changed
production behavior; the candidate is deliberately declared with dead-code scope.

Disposition: verified local candidate. No production construction, writer reports,
native/root/input authority, complete resource profile, Q/readiness or restore
qualification follows. Real producer integration is a separate bounded task.


## Admission clock regression

Final owner inspection found that a supplied timestamp could predate transition
lock acquisition. A new actual-past-deadline/stale-caller-time assertion failed
before an actual `Instant::now()` check was added under the lock immediately
before reservation. All 13 fast candidate tests then passed. No effect or scope
was added. Refreshed current-diff review approved the exact code manifest, and
the complete repository verification rerun passed with 3,334 Rust unit tests.
The final verification above describes this corrected code; the earlier full
run remains historical to the preceding head.
