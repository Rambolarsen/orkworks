# PR 526 review corrections

Starting revision: `b7ab2534ac846dffdf65acc13ae31d6007a277fc`.
Authority: owner request to validate outstanding findings, add regression tests,
fix confirmed issues, reconcile addressed threads, verify and formally review
before merge. Product contract: `specs/taskmaster-knowledge.md`.

## Execution checklist

- [x] Verify and reconcile existing disclosure/environment fixes and CodeQL paths.
- [x] Reproduce and fix control characters in generated proposal text and newest-observation knowledge ranking (`taskmaster/evaluator.rs`, evaluator tests).
- [x] Reproduce and fix reservation-versus-success caching (`taskmaster/runtime.rs`, `runtime/inference.rs`, evaluator acceptance and tests). Failed attempts must continue consuming durable budget; only accepted results suppress retries.
- [x] Reproduce and fix exclusion normalization and file-open races (`taskmaster/context.rs` and tests). Preserve bounded collection; never follow substituted links outside the workspace.
- [x] Reproduce and fix native model validation mismatch and Windows subprocess cleanup (`providers.rs`, `providers/inference.rs`, process fixtures). Preserve explicit trust, literal arguments, and bounded execution.
- [x] Reproduce and fix canonical workspace override keys and knowledge refresh recovery (Electron Taskmaster integration and tests). Preserve renderer/main boundaries and stale-sidecar guards.
- [ ] Run focused regression tests, full repository verification, required formal review, and reconcile review threads with evidence.
- [x] Preserve #525 for native Windows and owner desktop validation; preserve #503 for remaining integration/publication work.

Each correction follows failing regression → minimal fix → passing targeted test.
No provider accounts, sessions, signing secrets, or publisher activation are needed.
The named formal review gate remains a separate requirement from automated PR
comments and ordinary subagent reviews; merge is blocked until it is satisfied.

## Review routing

One review round, capped at two read-only Luna/high reviewers. The correctness
reviewer checks the completed correction diff and platform/identity behavior.
The coverage reviewer independently checks the regression tests against the
outstanding findings and accepted contract. Neither needs the other's result.
The root agent alone owns reconciliation, code changes, commits, and PR updates.
These reviews do not represent execution of the unavailable named `/code-review`
command. No other coding harness will be launched from this session.

## Review disposition and verification limits

The correctness pass caught an over-broad first fix: built-in provider IDs can
resolve to custom adapters. A failing endpoint regression confirmed opaque model
IDs were wrongly rejected. Native constraints now use the resolved transport;
the endpoint regression passes for both Codex and Claude custom overrides.
The coverage pass prompted added native success/stale-revision cache assertions,
canonical-key override application, and stale knowledge-refresh boundary tests.

Context substitution tests exercise the production no-link open helper with
both leaf and ancestor replacements. Collection deliberately aborts on an
unreadable/raced file rather than risking disclosure; retry/partial collection
is not added to this correction. These tests do not claim exhaustive filesystem
race or Windows reparse-point execution coverage.

The full macOS verification suite passed again after independent review
corrections: `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` exited 0 with
1,168 Rust unit tests, 3 integration tests, 3 intentionally ignored Rust tests,
708 desktop tests, builds, typechecks, formatting, docs and currency checks.
Windows process ownership
also passed a standalone Windows-target compile check, which is not native
execution. PR CI now discovers and runs path-normalization/canonical-key tests
alongside the descendant-holding-pipes timeout fixture. Native Windows execution
and owner desktop smoke evidence remain tracked in #525. The blind-spot closeout
found these gaps already covered by #525 and #503; neither issue is closed or
duplicated. The formal `/code-review medium` gate remains pending and blocks merge.
