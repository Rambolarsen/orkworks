# PR-aware session labels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep explicit PR numbers in Peon-generated session labels and reject generic harness-control labels.

**Architecture:** A pure validator in `peon.rs` decides whether an `InputLabel` inference may replace its synchronous typed-input fallback. The runtime calls it before updating live and persisted labels; prompt guidance biases the model toward task topics but the validator enforces the critical invariant.

**Tech Stack:** Rust 2021, Tokio, existing sidecar unit-test harness.

## Global Constraints

- Keep `label` a one-shot Peon-authored topic under ADR 0029.
- Do not add dependencies or alter normal terminal-output inference.
- Preserve a rejected inference’s raw typed-input fallback in memory and metadata.

---

### Task 1: Validate InputLabel replacements

**Files:**
- Modify: `crates/orkworksd/src/peon.rs:387-600`
- Test: `crates/orkworksd/src/peon.rs:603-1080`

**Interfaces:**
- Produces: `pub fn is_usable_input_label(label: &str, input_hint: &str) -> bool`.
- Consumes: a candidate summary and original submitted input.

- [ ] **Step 1: Write failing validator tests**

```rust
assert!(is_usable_input_label("Monitoring PR #249", "keep watching PR #249"));
assert!(!is_usable_input_label("Monitoring pull request", "keep watching PR #249"));
assert!(!is_usable_input_label(
    "Instructing system to continue current task execution",
    "keep watching PR #249",
));
```

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml input_label_requires_each_explicit_pr_number`

Expected: FAIL because the validator does not yet exist.

- [ ] **Step 3: Implement validator and prompt guidance**

Add local character scanning for `PR #<digits>` and `pull request #<digits>`, generic-instruction normalization, and prompt language requiring an InputLabel task topic plus PR-number retention.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml input_label_`

Expected: PASS.

### Task 2: Keep rejected inference from overwriting fallback

**Files:**
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs:123-145`
- Test: `crates/orkworksd/src/runtime/peon_runtime.rs:672-800`

**Interfaces:**
- Consumes: `peon::is_usable_input_label(&candidate, &hint)`.
- Produces: an unchanged live and persisted fallback label for an invalid candidate.

- [ ] **Step 1: Write a failing workspace-backed runtime test**

Set the fake provider output to `{"summary":"Monitoring pull request"}` for `keep watching PR #249`, then assert both live and persisted labels remain `keep watching PR #249`.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml input_label_inference_rejects_a_pr_number_dropping_label`

Expected: FAIL because the current path accepts the number-dropping label.

- [ ] **Step 3: Gate replacement**

Pass the consumed hint to the validator; update `SessionInfo` and `SessionMetadata` only for valid inferred labels.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml input_label_inference_`

Expected: PASS.

### Task 3: Verify and hand off

**Files:**
- Modify: `docs/superpowers/specs/2026-07-28-pr-aware-session-labels-design.md` only if implementation changes the approved design.

- [ ] **Step 1: Format and run sidecar tests**

Run: `cargo fmt --check --manifest-path crates/orkworksd/Cargo.toml` and `cargo test --manifest-path crates/orkworksd/Cargo.toml`.

- [ ] **Step 2: Run repository currency checks**

Run: `bash .claude/hooks/doc-check.sh` and `bash .claude/hooks/worktree-check.sh`.

- [ ] **Step 3: Commit implementation and plan**

```bash
git add crates/orkworksd/src/peon.rs crates/orkworksd/src/runtime/peon_runtime.rs docs/superpowers/plans/2026-07-28-pr-aware-session-labels.md
git commit -m "fix(peon): preserve PR references in session labels"
```
