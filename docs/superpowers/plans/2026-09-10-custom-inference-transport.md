# Custom Inference Transport Implementation Plan

> **For agentic workers:** Use executing-plans inline. Root owns all edits; preserve the existing dirty feature worktree.

**Goal:** Implement the approved custom command protocol as an isolated, locally testable transport, without enabling background custom execution prematurely.

**Architecture:** A private providers transport prepares literal argv from validated capability data and a caller-resolved absolute executable. Its owning value retains private prompt files through the existing bounded process runner. Strict UTF-8 stdout decoding is opt-in so existing Peon/native transports retain behavior. Trust eligibility, pre-spawn revalidation, cache identity, and atomic acceptance must be connected before production activation; no production caller is added in this slice.

**Tech Stack:** Rust, serde, tempfile, existing ProcessRunner.

**Spec:** [Accepted adapter design](../specs/2026-09-10-custom-inference-adapters-design.md), ADR 0055; issue #503.

## Constraints

- At most 256 KiB prompt, 64 KiB stdout/stderr, existing definition timeout 1–120 seconds.
- Opaque nonempty model, at most 256 UTF-8 bytes, no controls; no trimming, routing, or fallback. Explicit effort requires declared effort arguments.
- Expand each template once; inserted model text containing placeholders is literal, never expanded again. No shell evaluation or prompt-in-argv.
- Preserve login/config environment; remove all ORKWORKS_* plus BASH_ENV/ENV. No probes/discovery, managed-policy override, real credentials, or configured provider invocation.
- Strict single version-1 success envelope, no duplicate/unknown keys, nonempty nested JSON result with no duplicate keys. Return text for subsequent Taskmaster evidence validation, never recommendations directly.
- One root writer; at most two read-only reviewers (correctness and missing coverage), one round. No external review, commits, pushes or PR in this slice.

## Task 1: Prepared transport and protocol

Files: new `crates/orkworksd/src/providers/custom_inference.rs`; read-only getters in `harness/inference.rs`; private module registration and opt-in strict stdout path in `providers.rs`.

Interfaces: `prepare(capability: &InferenceCapability, resolved_path: &Path, model: &str, effort: Option<&str>, prompt: String) -> Result<PreparedCustomInference, ProviderOperationError>`; consuming `PreparedCustomInference::run(self) -> Result<String, ProviderOperationError>`. Preparation is not authorization; caller must perform runtime checks before using this private module.

- [x] Write protocol and construction tests first. Example: `assert!(decode(r#"{"version":1,"status":"success","result":"{\"x\":1,\"x\":2}"}"#).is_err())`; inspect argv with model `literal;$(x){promptFile}` and require exactly unchanged substitution.
- [x] Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml custom_inference -- --test-threads=4`, observe missing behavior, then implement preparation/strict decode. Use a single-pass template scan, typed input access, RAII directory/file ownership and safe fixed errors.
- [x] Exercise real local subprocess fixtures for stdin/file, literal argv, inherited fake login/config, stripped capability/startup variables, success/nonzero/malformed/invalid-UTF8 output, timeout and output bounds, and cleanup. Fail tests before adding missing runner behavior.
- [x] Run focused and full Rust validation. Retain no production call path until runtime trust integration.

## Task 2: Review and documentation

- [x] Update architecture and issue #503 with the implemented transport boundary and remaining activation work.
- [x] Review the merged slice using two read-only reviewers with distinct questions. Root verifies and patches findings; one round only.
- [x] Run repository verification, record native Windows/GUI limitations and any observed failures. Keep the active worktree; no merge/cleanup while the feature is unfinished.

## Execution and review record

- Initial tests failed for missing prepare/decode and dispatch interfaces. Construction/protocol passed after implementation. Real subprocess fixtures exposed lossy invalid-UTF-8 repair; the corrected fixture failed before opt-in strict decoding and passed afterward. Fixed fixture-only assumptions about macOS physical versus symlinked temporary paths and shell printf quoting.
- Correctness review reproduced two defects: a newly added process-global capability leaked after preparation; provider stderr exactly `timed out` was confused with a runner deadline. Snapshot the filtered environment explicitly (preserving all other login/config variables), and return typed `ProcessOutcome::TimedOut` internally. Existing runner callers retain their original textual result through the compatibility boundary.
- Coverage review strengthened overflow fixtures with otherwise-valid output and exact error codes, distinct config paths/PATH/generic capability sentinels, spawn-failure cleanup, and omitted effort coverage. Startup-variable removal is asserted directly in the child environment; no claim is made about the policy semantics of arbitrary shell startup files. Native Windows subprocess execution is unverified and remains required release/CI work under #503.
- One review round used a new correctness reviewer and the existing read-only coverage reviewer (new-thread capacity was exhausted). Neither reviewer edited artifacts or launched external review. Root owns all patches.
- Close-out uncertainty: tests that could succeed for the wrong failure reason were investigated and strengthened; the actual environment and timeout defects were fixed on the spot. No duplicate issues filed for intentionally pending trust/scheduling/cache/acceptance integration or already-required Windows verification. No real providers, login files, or user trust grants were used. Custom execution stays inactive and the owned feature worktree remains in progress.
- A subsequent full run exposed a fixture-only startup assumption: timeout can precede the child's first record write. Reproduced deterministically by delaying fixture startup beyond the unchanged one-second deadline, then asserted timeout type and owned-path cleanup in the helper instead of requiring startup output. No production timeout was increased. Focused suite after correction: 9 passed.
- Final `rtk env RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` exited 0: 1,118 Rust unit tests and 3 integration tests passed (2 ignored), 692 desktop tests passed, formatting/typecheck/build/docs/diff/currency checks passed. No native Windows or GUI smoke test, commit, push, PR, or manual `/code-review low` gate was performed. Issue #503 retains the remaining activation checklist.
