# Peon work-history implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist only concrete, terminal-grounded Peon work updates in task history.

**Architecture:** `peon.rs` classifies an inference window into an optional history summary from descriptive user input or a command/result pair. The Peon runtime passes that separately from the model inference, and `MetadataStore` uses it as the only Peon-provided live/durable summary. Status and all non-summary inference fields retain their current behavior.

**Tech Stack:** Rust, existing sidecar unit tests, Cargo.

## Global Constraints

- Do not add dependencies, event fields, APIs, or UI changes.
- Preserve existing explicit user and agent history checkpoints.
- Treat ANSI/redraw/spinner text and unpaired error/loading words as ineligible.
- Do not migrate existing event logs.

---

### Task 1: Classify grounded terminal work

**Files:**
- Modify: `crates/orkworksd/src/peon.rs:297-405,485-505,709-1300`

**Interfaces:**
- Produces: `pub fn work_history_summary(output: &[String], inference_summary: Option<&str>) -> Option<String>`.
- Consumes: existing `is_descriptive_input` and `is_usable_input_label` helpers.

- [ ] **Step 1: Write the failing classifier tests**

```rust
#[test]
fn work_history_summary_accepts_a_descriptive_user_task() {
    let output = vec!["[User input]: fix task history noise".into()];
    assert_eq!(
        work_history_summary(&output, Some("Fixing task history noise")),
        Some("Fixing task history noise".into())
    );
}

#[test]
fn work_history_summary_uses_canonical_test_outcomes() {
    let output = vec!["$ cargo test".into(), "test result: ok. 42 passed".into()];
    assert_eq!(work_history_summary(&output, Some("Terminal is healthy")), Some("Tests passed".into()));
}

#[test]
fn work_history_summary_rejects_terminal_state_guesses() {
    let output = vec!["\u{1b}[2K⠋ loading".into()];
    assert_eq!(work_history_summary(&output, Some("Session is loading")), None);
}
```

- [ ] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml work_history_summary`

Expected: FAIL because `work_history_summary` does not exist.

- [ ] **Step 3: Add the minimal classifier and prompt constraint**

```rust
pub fn work_history_summary(output: &[String], inference_summary: Option<&str>) -> Option<String> {
    let input = output.iter().rev().find_map(|line| line.strip_prefix("[User input]:").map(str::trim));
    if let Some(input) = input.filter(|input| is_descriptive_input(input)) {
        return inference_summary
            .filter(|summary| is_usable_input_label(summary, input))
            .map(normalize_summary);
    }
    command_outcome_summary(output)
}
```

Implement `command_outcome_summary` as a private helper that recognizes only a `cargo`/`pnpm`/`npm` test or build command and its matching success/failure marker in the same output window. Return fixed labels (`Tests passed`, `Tests failed`, `Build passed`, or `Build failed`); otherwise return `None`. Update `SYSTEM_PROMPT` to direct the model to omit `summary` unless it sees the same evidence.

- [ ] **Step 4: Run the targeted tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml work_history_summary`

Expected: PASS with all three classifier tests green.

- [ ] **Step 5: Commit the classifier**

```bash
git add crates/orkworksd/src/peon.rs
git commit -m "fix: ground Peon work summaries"
```

### Task 2: Persist only the classified work summary

**Files:**
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs:120-226`
- Modify: `crates/orkworksd/src/metadata.rs:1435-1583,1804-1824,1982-2023`

**Interfaces:**
- Consumes: `peon::work_history_summary(&output_snapshot, inf.summary.as_deref()) -> Option<String>`.
- Changes: `MetadataStore::merge_peon_inference` receives `history_summary: Option<&str>` after `provider`.
- Produces: Peon history/checkpoints and `SessionMetadata.summary` only from `history_summary`.

- [ ] **Step 1: Write the failing metadata test**

```rust
#[test]
fn merge_peon_inference_uses_only_the_classified_history_summary() {
    let dir = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(dir.path());
    store.write_session(&test_metadata("grounded-summary"));
    let inference = peon_inference_with_summary(Some("Session appears stuck"), 0.7);

    store.merge_peon_inference("grounded-summary", &inference, "t1", None, None).unwrap();

    let meta = store.read_session("grounded-summary").unwrap();
    assert_eq!(meta.summary, None);
    assert!(store.read_events("grounded-summary").iter().all(|event| event.summary.is_none()));
}
```

- [ ] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml merge_peon_inference_uses_only_the_classified_history_summary`

Expected: FAIL because the raw inference summary is currently persisted.

- [ ] **Step 3: Make summary persistence use the classified value**

```rust
// runtime/peon_runtime.rs, immediately before merge
let history_summary = peon::work_history_summary(&output_snapshot, inf.summary.as_deref());
ws.metadata.merge_peon_inference(&id, &inf, &now_iso, provider_result.observation.as_ref(), history_summary.as_deref())

// metadata.rs
meta.summary = history_summary.map(str::to_string).or(meta.summary);
let checkpoint = self.summary_checkpoint(id, history_summary);
```

Update every existing `merge_peon_inference` caller and test to pass its intended grounded summary explicitly. Keep all status/phase/question fields sourced from `inf` exactly as before.

- [ ] **Step 4: Run focused and complete sidecar tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml work_history_summary`

Expected: PASS.

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml merge_peon_inference`

Expected: PASS.

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit the persistence gate**

```bash
git add crates/orkworksd/src/runtime/peon_runtime.rs crates/orkworksd/src/metadata.rs
git commit -m "fix: keep noisy Peon summaries out of history"
```

### Task 3: Record the reviewed design and plan

**Files:**
- Modify: `docs/superpowers/specs/2026-08-01-peon-work-history-design.md`
- Create: `docs/superpowers/plans/2026-08-01-peon-work-history.md`

**Interfaces:**
- Produces: implementation record with no runtime behavior.

- [ ] **Step 1: Confirm the design states the evidence boundary and no-migration decision**

```markdown
The runtime classifies the exact observed output before it merges Peon's inference.
Previously persisted history is not rewritten; this prevents future noise only.
```

- [ ] **Step 2: Check documentation-only changes**

Run: `git diff --check`

Expected: no output and exit code 0.

- [ ] **Step 3: Commit the reviewed documentation**

```bash
git add docs/superpowers/specs/2026-08-01-peon-work-history-design.md docs/superpowers/plans/2026-08-01-peon-work-history.md
git commit -m "docs: plan grounded Peon history"
```
