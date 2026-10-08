# Taskmaster privacy prerequisite implementation plan

> **For agentic workers:** Execute inline using the executing-plans skill. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close #782's unsafe existing Brain analysis admission until #529 supplies reviewed, privacy-qualified signed knowledge.

**Architecture:** Add one server-owned availability check in Taskmaster runtime and use it at scheduling, worker entry, and manual HTTP admission. Keep the gate closed in all builds: the current knowledge transport has no signed policy/manifest attestation, and no request field can establish that proof. Preserve the evaluator's private post-admission engine so its existing transport, identity and output tests remain meaningful while production admission is closed. No new protocol, authority, dependency, or verification bypass is introduced. #529 owns verified activation and its positive-path proof.

**Tech Stack:** Existing Rust/Axum sidecar, React status presentation, pnpm desktop validation.

**Spec:** `specs/taskmaster-knowledge.md`, Analysis lines 131–142 and Knowledge distribution; #782 and #529.

## Global constraints

- Block background and manual Brain calls for native and custom providers, including absent or legacy knowledge and forged unsigned policy fields.
- Fail before collection, provider preparation, queueing or usage reservation. A previously queued worker must also fail without dispatch.
- Keep deterministic observation correlation, stored recommendations and explicit Fix with AI working.
- Do not implement Assess workflow or the #529 publisher in this fix.
- The prerequisite is closed in tests as well as production; dormant engine fixtures explicitly test the private continuation after admission, never a production bypass.

## Investigated uncertainty

The unsigned starter and Electron-validated legacy cache carry no matching signed privacy-policy attestation. Accepting a plain policy field would not satisfy the contract. The accepted spec explicitly requires all existing Brain analysis unavailable until #529. The largest blind spot is testing only HTTP admission: an already-queued worker could still run, so independent worker regression coverage is required.

### Task 1: Reproduce and close production admission

**Files:** `crates/orkworksd/src/taskmaster/runtime.rs`, `taskmaster/evaluator.rs`, `taskmaster/evaluator/activation_tests.rs`, `http/taskmaster_handlers.rs`, `http/taskmaster_settings_handlers.rs`.

- [x] Run the new `legacy_knowledge_` tests and observe provider dispatch / scheduling before the gate.
- [x] Add `pub(crate) fn brain_knowledge_availability() -> Result<(), &'static str>` to runtime, returning the explicit unavailable prerequisite until #529 implements verified activation.
- [x] In the scheduler, preserve lease-protected stale run recovery, then return `ScheduleResult::Unavailable` before queue creation when that check fails. In the production worker, return before collection/transport preparation when it fails.
- [x] Keep transport and identity fixtures entering the private post-admission continuation and identify that scope in their helpers. New regression fixtures enter production admission.
- [x] Make Analyze now return `unavailable` with the prerequisite reason, independently of whether background discovery is disabled. Preserve the existing active-recommendation response.
- [x] After provider availability resolves to ready, project `knowledge_unavailable` in settings; render it as “Verified reference knowledge required”. Keep provider capability/approval diagnostics intact.
- [x] Run the focused regression tests, dormant evaluator tests, HTTP settings tests, and desktop settings component tests.

### Task 2: Document and integrate

**Files:** `docs/agents/architecture.md`, `docs/user/taskmaster-knowledge.md`, `docs/user/taskmaster.md`; issue #529 and issue #782.

- [x] Document that background Brain analysis and Analyze now are currently unavailable pending compliant signed knowledge; distinguish loaded legacy knowledge from inference eligibility.
- [x] Add #529 acceptance criteria for signed supported policy fields in manifest and payload, matching signed starter attestation, sidecar-owned eligibility/revalidation, and positive verified-bundle integration coverage. Record #782's current fail-closed scope without claiming #529 complete.
- [x] Run `bash scripts/verify-repo.sh`, inspect every result, and run the required review gate on the verified diff before opening a PR.
- [x] Check matching live Taskmaster recommendations through the API and tie off only a verified matching recommendation before the PR reaches terminal state.
- [ ] Open one PR, babysit required checks and comments, merge via the documented maintainer path only after review/checks pass, and clean up using `scripts/finish-pr.sh`.

Local verification: `scripts/verify-repo.sh` passed on 2026-10-08; Rust suites 1815 + 4 passed, 5 ignored; desktop 1111 passed, 1 skipped. Independent medium-effort diff review found no actionable issues. The live recommendation API returned no matching recommendation.

PR review follow-up: confirmed deferred stale run recovery must remain reachable under the installation-wide lease while Brain admission is closed. Added queued/running recovery regression; observed failure before moving the scheduling gate after recovery. No analysis is queued, dispatched, or charged during recovery.

Follow-up verification: full `scripts/verify-repo.sh` passed again after the recovery fix; independent medium-effort manual review found no actionable issues. Automated re-review cycle 1/3 will cover the updated head.

Second PR review follow-up: confirmed the manual HTTP unavailable response also must retry lease-protected recovery. Added a real HTTP queued/running test with background discovery disabled and a provider call counter; observed failure before the fix and success after it. The handler preserves live attempts while the lease is held and recovers stale attempts after release without dispatch or usage reservation. Review cycle 2/3 will cover this updated head.

Final HTTP follow-up verification: `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` passed (Rust 1815 + 4; desktop 1111). The first default-concurrency attempt hit an unchanged Ollama fixture two-second socket read timeout; that test passed immediately in isolation and in the complete bounded rerun. A fresh medium-effort manual review found no actionable issues.
