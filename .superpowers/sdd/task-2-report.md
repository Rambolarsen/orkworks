# Task 2 report — Keep rejected inference from overwriting fallback

## Scope

Changed only `crates/orkworksd/src/runtime/peon_runtime.rs`:

- Added a workspace-backed runtime regression test for an input hint and fallback label of `keep watching PR #249` when the fake provider returns `Monitoring pull request`.
- Passed the consumed input hint into `peon::is_usable_input_label` before updating either live `SessionInfo.label` or persisted `SessionMetadata.label`.

## TDD evidence

1. Added `input_label_inference_rejects_a_pr_number_dropping_label` before changing runtime behavior.
2. Red run:

   ```text
   cargo test --manifest-path crates/orkworksd/Cargo.toml input_label_inference_rejects_a_pr_number_dropping_label
   FAILED
   left: "Monitoring pull request"
   right: "keep watching PR #249"
   ```

3. Added the smallest gate: candidates are accepted only when the consumed hint exists and `is_usable_input_label(label, hint)` returns true.
4. Green run:

   ```text
   cargo test --manifest-path crates/orkworksd/Cargo.toml input_label_inference_
   4 passed; 0 failed
   ```

## Behavior

An invalid inference no longer changes the live or persisted fallback label. Valid inference behavior stays on the existing path, including its durable metadata write.

## Verification notes

- `git diff --check` passed.
- `cargo fmt --check` was attempted, but reports pre-existing formatting differences in several unrelated Rust files (including `http/`, `metadata.rs`, `peon.rs`, and `terminal_runtime.rs`); no mass-formatting was applied.

## Concerns

None for Task 2 behavior. The unrelated formatter drift should be addressed separately by its owners.

## Review follow-up — deterministic completion

`input_label_inference_rejects_a_pr_number_dropping_label` now waits up to five
seconds for the fake provider call counter to reach one and for the session to
leave `in_flight`. Together these conditions prove the input-label inference
ran and completed before the fallback-label assertions; the prior fixed 2.3s
sleep is removed.

Focused verification:

```text
cargo test --manifest-path crates/orkworksd/Cargo.toml input_label_inference_rejects_a_pr_number_dropping_label
cargo test: 1 passed, 470 filtered out (1 suite, 1.03s)
```
