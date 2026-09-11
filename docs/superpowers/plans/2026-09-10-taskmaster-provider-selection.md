# Taskmaster provider selection foundation

Tracking: [#503](https://github.com/Rambolarsen/orkworks/issues/503)
Design: [accepted custom adapters design](../specs/2026-09-10-custom-inference-adapters-design.md).

## Scope and boundaries

Project custom inference capabilities independently of Peon into Recommendations
settings. Preserve stored selections. Distinguish declared capabilities from
runnable built-ins and approved-but-inactive custom executables. Custom runtime
activation, pre-spawn identity validation, cache identity and atomic acceptance
remain a subsequent slice. No provider calls or credential changes.

The picker uses static suggestions/free-text for every provider in this slice.
It must never invoke Peon's dynamic model discovery endpoint: definitions could
change after the picker loads. Existing Peon discovery remains unchanged.

## Tasks (test-first)

- [x] Add a read-only Taskmaster catalog with typed availability states, static
  models and effort support. Test inference without Peon, native overrides and
  clears, approval transitions, missing executables and corrupt trust.
- [x] Expose the catalog through privileged settings and use its runnable gate
  before evaluator context collection/reservation. Preserve native transports.
- [x] Consume that catalog in the renderer, preserve unavailable selections,
  remove automatic discovery and show inactive/approval-required states.
- [x] Verify focused tests, complete repo checks, and review the bounded diff.

## Review topology

Root is the sole writer and integration owner. Reuse two existing read-only
reviewers for one round: correctness/trust boundary and UI/coverage respectively.
These are independent questions on the same completed slice, not parallel
implementation. No external review or additional coding harness.

## Verification

Use local temporary harness/trust fixtures only. Run focused Rust settings and
catalog tests, desktop tests/typecheck, then `scripts/verify-repo.sh`. Review
findings require evidence and regression tests before patches. Leave issue open
until activation and remaining accepted requirements are implemented.

## Execution record

The new custom/no-Peon status regression first failed with unsupported capability
instead of approval required; the renderer capability regression first failed
on the new availability state. Both passed after implementation. Focused catalog
tests cover native override/clear fallback and missing executable/corrupt trust.
An invalid test fixture attempted a `retired` user override (not a supported
patch field); it was removed after tracing the schema rejection, not accommodated
by weakening production validation. Retirement is handled by the projection but
does not have a dedicated new regression test in this slice.

Full `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` passed: Rust formatting,
build/tests, desktop typecheck and 692 tests, desktop/docs builds, diff and currency
checks. Existing warnings remain. No real provider calls or native Windows/GUI
smoke tests were performed.

Both requested read-only reviewers failed with harness usage-limit errors before
returning findings. Independent review is therefore **pending**, not passed.
No commits, pushes or PR changes were made. Remaining integration includes
pre-spawn trust validation, trust-bound cache identity, atomic result acceptance,
selection validation and the full code-owned native capability mapping.

### Resumed review and validation — 2026-09-11

The owner requested continuation. Both reviewers successfully completed the
resumed read-only round. Backend findings were reproduced by failing tests:
execution-affecting native overrides incorrectly reported ready, and Ollama
advertised effort its HTTP transport ignores. Native projection now rejects
launch/Peon overrides while preserving harmless name overrides; Ollama reports
no effort capability. Status and scheduling share a selection-aware gate so
previously stored unsupported effort cannot be treated as ready.

UI feedback identified unsupported persisted effort and missing behavior tests.
The accepted design requires explicit rejection, so the patch rejects unsupported
effort at both save boundaries rather than silently clearing it as suggested.
Component tests exercise real component rendering/effects/save logic with local
state and IPC fixtures: no dynamic discovery, preserved unavailable selections,
inactive custom disclosure, and rejection of unsupported effort. They do not
assert source text or substitute a mock component for the behavior under test.

Shared model/effort validation now checks nonempty values, the 256-byte UTF-8
bound and control characters at settings and custom preparation boundaries,
without trimming or rewriting opaque model identifiers. The new settings test
failed before the shared validator was added. Custom execution is still inactive;
the next slice must introduce a shared serialization boundary for pre-spawn and
result-acceptance trust validation without recursively acquiring the existing
Taskmaster persistence mutex.

Final verification after these fixes: full `scripts/verify-repo.sh` passed with
1,125 Rust unit tests, 3 integration tests, 2 ignored tests, and 695 desktop
tests. Formatting, typechecking, desktop/docs builds, diff and currency checks
passed; existing warnings remain. This completes the resumed review checkpoint,
not custom runtime activation. No commit, push, merge, real model call or native
Windows/GUI smoke test was performed.
