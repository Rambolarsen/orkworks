# Taskmaster inference guard implementation plan

> **For agentic workers:** Use executing-plans to implement this sequential
> slice; root owns all edits, with two read-only reviewers at the checkpoint.

**Goal:** Capture a trusted custom evaluation identity and revalidate it under
the same serialization boundary as Taskmaster generation-guarded application.

**Architecture:** A private persistence guard proves ownership of the existing
process mutex and filesystem lease. Runtime checks reuse that guard rather than
recursively locking trust storage. Harness mutation lock comes first, persistence
locks second, runtime data last; the supplied synchronous action stays inside
those locks. Execution wiring remains inactive until the runner can hold the
guard through spawn but release it before waiting for process completion.

**Tech Stack:** Rust, serde, existing fs2 locks, local temporary test fixtures.

**Spec:** [accepted adapter design](../specs/2026-09-10-custom-inference-adapters-design.md),
[ADR 0055](../../adr/0055-json-taskmaster-inference-adapters.md), issue #503.

## Global constraints

- No real provider calls, credential changes, implicit approval or provider fallback.
- Identity contains definition digest, resolved executable path and trust generation.
- Revoke/reapprove invalidates an old identity even when the definition is identical.
- Approved metadata alone does not make a custom adapter runnable.
- Keep callbacks synchronous and short; never wait for model completion under locks.
- Root is the only writer; maximum two read-only reviewers, one review round.
- No commits, push, PR or merge without the owner's separate approval.

## Task 1: Shared lock proof and checked custom action

Files: `taskmaster/runtime.rs`, new `taskmaster/runtime/inference.rs`,
`taskmaster/inference_trust.rs`, `harness/store.rs`.

- [x] Write local fixture tests that capture an approved definition, invalidate
  it by revocation/reapproval or definition edits, and assert the action never runs:

  ```rust
  let evaluation = runtime.evaluation_snapshot(&workspace).unwrap();
  let identity = runtime.capture_custom_inference(&harnesses, &workspace, &evaluation)?.unwrap();
  trust.revoke("custom", trust.generation()?)?;
  assert!(!runtime.with_current_custom_inference(&harnesses, &identity, || panic!("stale action"))?);
  ```

- [x] Run the focused test and observe the missing guard behavior.
- [x] Add `PersistenceGuard` with private mutex/file ownership and root binding;
  refactor ordinary generation application and trust inspection to use it.
- [x] Implement `capture_custom_inference` and `with_current_custom_inference`:
  resolve from a locked harness snapshot, validate exact selection/capability,
  inspect trust using the same guard, compare digest/path/trust generation,
  reload the Taskmaster generation, then run the callback while locks are held.
- [x] Include a stable cache identity representation, but do not wire the
  scheduler/runner or mark custom adapters ready in this slice.
- [x] Run focused runtime/trust tests; retain existing approval/persistence behavior.

## Task 2: Concurrency and fail-closed evidence

Files: new `taskmaster/runtime/inference/tests.rs` and the modules above.

- [x] Test unavailable/unapproved definitions, corrupt trust, wrong settings
  generation and changed executable resolution with no callback side effects.
- [x] Test that trust mutation and harness mutation block while an accepted
  callback is inside the critical section, then complete after it exits. Use
  channels with bounded receive timeouts and always release before asserting.
- [x] Verify normal callbacks complete without recursive mutex acquisition.
- [x] Run `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh`.

## Review and handoff

Reuse the two existing reviewers: correctness examines lock order and identity
binding; coverage examines whether tests pin stale-output rejection and mutation
serialization. They read code, do not edit or spawn others. Root independently
verifies findings, patches with regression tests, updates docs and issue #503.
Subsequent runner/cache integration must consume this guard before activation.

## Execution and review record

The initial test could not compile until the capture/guard interface existed.
After implementation, the reapproval test was mutation-checked: deliberately
omitting the durable trust-generation comparison triggered its stale-action
panic. The comparison was restored and all focused tests passed.

The correctness review and a second-store regression exposed an actual gap:
HarnessStore had only an instance-local mutex, so another store could edit the
definition during a callback. That regression failed with “mutation crossed an
active guarded callback.” Both snapshots and mutations now acquire a retained
document file lease after the instance mutex and before Taskmaster locks.
The regression passes, including a separate-process test whose helper blocks
until the guarded callback returns. Symlinked lock targets fail closed on Unix.

Coverage feedback added workspace-specific selection/disablement, missing
executable and corrupt-trust capture, a historical grant on a retired tool, and
cache determinism/individual-field tests. The focused suite passes 11 tests;
one ignored helper is invoked explicitly by the cross-process test.

The guard remains inert at the scheduler/runner seam. No real provider calls,
credential changes, commits, pushes or merges were made. Native Windows and
GUI smoke testing remain unverified. Remaining integration must hold this guard
through actual spawn, release it before process waiting, include cache identity,
and use it around the existing workspace/evidence/result-commit checks.

Final full verification passed after the file-lease fix: 1,136 Rust unit tests,
3 integration tests, 3 ignored helper tests, and 695 desktop tests. Rust formatting,
build, desktop typecheck/build, docs build, diff checks and currency checks passed.
Existing warnings remain. Both read-only reviews completed and their findings
were addressed within this one review round.
