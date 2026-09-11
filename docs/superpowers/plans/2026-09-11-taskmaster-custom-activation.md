# Taskmaster custom activation implementation plan

> **For agentic workers:** Use executing-plans; root is the sole writer.

**Goal:** Activate approved custom inference through the existing guarded evaluator.

**Architecture:** Extract the evaluator's filesystem-root dependency into a private
function used by production and tests. Local executable fixtures exercise catalog
readiness, context, reservation, transport and recommendation persistence together.
Only approved custom catalog entries change from inactive to ready.

**Tech Stack:** Rust, existing tempfile/process fixtures and real local stores.

**Spec:** [accepted adapter design](../specs/2026-09-10-custom-inference-adapters-design.md),
[ADR 0055](../../adr/0055-json-taskmaster-inference-adapters.md), issue #503.

## Constraints

- No real providers, credentials, user settings or external writes.
- Explicit executable approval and enabled analysis remain mandatory.
- No custom discovery, native fallback, or changes to Peon or active sessions.
- Root sole writer; two independent read-only reviewers, one round, no publish.
- Windows process validation remains required before release; Unix fixtures do
  not constitute Windows verification.

## Task 1: Full evaluation fixtures and activation

Files: `taskmaster/evaluator.rs`, new `evaluator/activation_tests.rs`,
`taskmaster/provider_catalog.rs`, `http/taskmaster_settings_handlers.rs`.

- [x] Write a local executable fixture that records stdin and literal model
  argv, then returns a grounded proposal in a strict success envelope. Run the
  actual evaluation path against a temporary runtime root:
  `run_model_evaluation_at(state.clone(), runtime_root.clone())`.
  Assert unapproved means no child marker, no reservation and no recommendation;
  approval means one reservation and one grounded recommendation without sessions.
- [x] Observe the missing root-injection interface, then extract only the root
  lookup from `run_model_evaluation`. Observe approved evaluation still produces
  no recommendation because readiness is inactive.
- [x] Change approved custom catalog state to `Ready`; update the settings
  contract test to require ready only after explicit approval. Keep unknown,
  revoked, corrupt and unsupported-effort selections gated.
- [x] Add full-path failure cases: malformed/nonzero output spends one reserved
  evaluation but writes no recommendation; revoked/disabled/unsupported effort
  spends none and starts no process. Pending revocation rejects output and
  prevents a subsequent call. Keep fixtures bounded and isolated.
- [x] Run focused Taskmaster/provider tests and full repository verification
  using `RUST_TEST_THREADS=4` (known one-second process fixture sensitivity).

## Task 2: Review and checkpoint

- [x] Correctness reviewer checks activation authorization and integration;
  coverage reviewer checks untested state transitions and no-fallback assertions.
  Both inspect the completed slice independently; root applies verified findings.
- [x] Update architecture and user documentation for approved execution,
  preserving trust/revocation caveats. Record test evidence and Windows limitations.
- [x] Verify final changes, then return a checkpoint without commit or publication.

## Initial permission checkpoint (resolved below)

The full-path test first failed because `run_model_evaluation_at` did not exist.
After extracting the private runtime-root dependency, it failed at the intended
assertion: approved custom execution produced zero recommendations instead of one.

The permission reviewer then rejected changing approved custom readiness to
`Ready`: persistent background execution with workspace context and existing
credential access requires explicit owner authorization for activation. No part
of that rejected patch was applied. The catalog and settings still report
`execution_inactive`; the user-facing documentation is unchanged.

The positive activation test is explicitly ignored with that permission reason;
it is a pending regression, not verified activation coverage. A separate active
test verifies that the unapproved evaluator starts no child, spends no usage,
and writes no recommendation. No fixture provider was executed in this checkpoint.
Remaining failure-path tests, activation, and the planned review round are pending.
Do not bypass the permission decision by enabling a test-only readiness path.

## Explicit authorization and resumed execution

The owner subsequently authorized: “Enable approved custom providers for
background execution using permitted workspace context and existing CLI logins.”
The production readiness change was then permitted. The ignored activation test
was restored to the normal test suite; no test-only readiness bypass was added.

All five full-path fixture tests pass: unapproved denial, approved grounded output,
malformed/nonzero output, revoked/disabled/unsupported-effort/corrupt-trust/changed-
definition denial, and rejection of output revoked during a running process.
Tests use temporary settings/trust stores and fixture executables only. The
production root lookup delegates to the same private evaluator exercised by tests.

### Review fixes

Correctness review found no concrete defect. Coverage review identified missing
explicit no-fallback observations, recovery transitions, unknown-provider denial,
and a context-collection claim that was only indirect. The fixture now registers
counters for all native/Peon alternatives and asserts zero invocations. Recovery
tests cover revoke/reapprove and disable/re-enable before successful evaluation.
Unknown providers join the denied-state table.

The private evaluation function now accepts its repository collector as an
explicit dependency. Production supplies the existing collector; denial tests
supply a panic-on-call collector, proving those states never reach collection.
The missing dependency seam first failed compilation before implementation.
The pending-revocation fixture also records successful output completion so a
timeout cannot masquerade as rejection of valid pending output. Its immediate
second call remains interval-gated too; fresh revoked fixtures independently
verify revocation denial without a prior reservation.

All six activation fixture tests pass after these changes. The first full
repository verification passed before the coverage additions; the final
post-review verification result is recorded at completion. The renderer retains
defensive support for an older sidecar's `execution_inactive` state, but the
current backend no longer emits it. No additional review round was dispatched.

Final post-review `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` exited zero:
1,157 Rust unit tests passed (three ignored helpers), three Rust integration tests
passed, and all 695 desktop tests passed. Rust formatting/build, desktop typecheck
and build, docs build, diff and documentation/worktree checks passed. Existing
warnings remain. Native Windows execution is unverified and remains a release
gate. Close-out investigation found no new issue beyond that already specified
requirement; no external issue writes, commits, pushes or releases were performed.
