# Codex needs-you clear Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep a Codex session working after accepted input when asynchronous label inference reports a stale prompt.

**Architecture:** `InferenceMode::InputLabel` already has a dedicated label persistence path. Remove its full metadata merge so only `InferenceMode::Output` may change observed status and attention.

**Tech Stack:** Rust, Tokio tests, existing fake Peon provider.

## Global Constraints

- No new dependencies or abstractions.
- Input-label inference may update only the session label.
- Output-triggered inference behavior is unchanged.
- The regression asserts both `SessionInfo` and persisted `SessionMetadata` remain `working`.

---

### Task 1: Restrict input-label inference

**Files:**

- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Test: `crates/orkworksd/src/runtime/peon_runtime.rs`

**Interfaces:**

- Consumes: `InferenceMode::InputLabel`, `MetadataStore::read_session`.
- Produces: input-label inference that updates `SessionInfo.label` and `SessionMetadata.label` without changing attention metadata.

- [ ] **Step 1: Write the failing test**

Add a workspace-backed `#[tokio::test]` beside `input_label_inference_only_updates_the_live_label` that seeds a live and persisted session with `observed_status = "working"`, `attention = "working"`, and `metadata_source = "process"`; queue an input-label inference whose fake provider returns `{"status":"waiting_for_input","summary":"New label","confidence":0.85}`; then assert:

```rust
assert_eq!(info.label, "New label");
assert_eq!(info.attention.as_deref(), Some("working"));
assert_eq!(meta.observed_status.as_deref(), Some("working"));
assert_eq!(meta.attention.as_deref(), Some("working"));
assert_eq!(meta.label, "New label");
```

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml input_label_inference_preserves_committed_working_attention`

Expected: FAIL because `InferenceMode::InputLabel` calls `merge_peon_inference_with_history`, which accepts `waiting_for_input` and restores `needs_you` in metadata.

- [ ] **Step 3: Write minimal implementation**

Delete the `merge_peon_inference_with_history` call in the `InferenceMode::InputLabel` branch of `peon_loop`; retain the following existing label extraction and live/persisted label writes.

- [ ] **Step 4: Run test to verify it passes**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml input_label_inference_preserves_committed_working_attention`

Expected: `1 passed; 0 failed`.

- [ ] **Step 5: Run focused module verification and commit**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_runtime`

Expected: all `peon_runtime` tests pass.

Commit:

```bash
git add crates/orkworksd/src/runtime/peon_runtime.rs
git commit -m "fix: preserve working attention after input label inference"
```
