# Task 7 report — fix round 1 evidence consolidation

Status: evidence/spec/ADR scope only; ADR 0056 remains proposed and no
production runtime files changed.

## Review corrections

- The canonical 48-row matrix is asserted as 16 scenarios × 3 platforms with
  result totals of 5 `pass`, 9 `accepted`, 1 `unresolved`, and 33
  `unsupported`. Every row has a non-empty `evidence_ref`; production seam
  rows cite this Task 6 report.
- Task 4 is recorded as vacuous/re-scoped candidate coverage: the six macOS
  tests fail closed before native launch, so they do not prove the descendant
  matrix or launchd lifecycle. Candidate labels are not generalized Unix
  ownership evidence. Admission-substitution isolation/reliability remains
  fixture-bounded, and portable-Linux descriptor runtime evidence is missing.
- Task 5's 14 launch-dependent skips remain explicit. The acknowledgement-
  concurrency gap remains parked: the paused-launch latch test is preserved,
  but it is not treated as closing independent cleanup-acknowledgement proof.
  The escaped-JSON 64 KiB framing gap also remains parked.
- The requested `npx tsc --noEmit` check is failed/unverified: the equivalent
  local typecheck produced no output for 90 seconds and was stopped with exit
  130; desktop focused tests remained 24/24.

## Verification

The exact fix-round commands and results are recorded here after execution:

- `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test evidence`
  — 1 passed.
- `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml -- --nocapture`
  — 48 passed.
- fixture `cargo fmt --check`, `git diff --check`, and `bash scripts/doc-check.sh`
  — pass; the documentation hook may emit its advisory README reminder for
  ADR changes without failing.
- `npx tsc --noEmit` — failed/unverified as described above; no production
  code was changed to address it.
- `node --experimental-strip-types --test tests/sidecarLifecycle.test.ts tests/backendRestoration.test.ts`
  — 24 passed.
- Rust sidecar baseline remains the known Task 6 failure: 1266 passed, 1
  failed, 3 ignored in the ProcessRunner prompt-write test.

## Issue status

Issue #545 remains open. The fix-round comment attempt and exact API result are
recorded in the evidence document. The earlier retry after `cc72e0d` exited 1
with `error connecting to api.github.com`; the later successful comment is
[issuecomment-5688102947](https://github.com/Rambolarsen/orkworks/issues/545#issuecomment-5688102947).
