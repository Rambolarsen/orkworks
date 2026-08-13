# Session Label Reset Commands Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reset a session's one-shot topic label when the active coding tool receives one of its documented fresh-conversation commands.

**Architecture:** Add an optional `labelResetCommands` array to resolved harness definitions and use the persisted session harness ID to gate exact, trimmed command matches. Terminal input resets the live and durable labels to the established placeholder and invalidates queued label work. A per-session label epoch travels with every `InputLabel` hint so an already-running Peon inference cannot restore a title from the previous conversation.

**Tech Stack:** Rust, Serde JSON registry, Tokio, existing metadata store and Rust unit tests.

## Global Constraints

- Preserve ADR 0029's one-shot label lifecycle: a reset command is never itself a label; only the next descriptive input may seed one.
- Apply a reset only after a completed input line was successfully delivered and classified non-sensitive.
- Match only the exact trimmed command configured for the persisted session harness ID; absent or unknown IDs never reset.
- Ship only verified initial built-ins: Claude Code `/clear`, `/reset`, `/new`; OpenCode `/clear`, `/new`; every other built-in declares none.
- Existing custom `harnesses.json` documents without the field must continue to deserialize; sparse built-in overrides may replace the list or clear it with `null`.
- Keep Electron and renderer untouched; this is sidecar-only behavior.
- Run Rust formatting, build, and tests plus the repository doc/worktree checks before handoff.

---

## File structure

- `crates/orkworksd/resources/harnesses-v2.json` — source-of-truth reset command declarations for supported built-ins.
- `crates/orkworksd/src/harness/definition.rs` — Serde schema, sparse override semantics, and registry tests.
- `crates/orkworksd/src/harness/store.rs` — safe migration of v1 definitions into the expanded schema.
- `crates/orkworksd/src/main.rs` — small concurrency state types (`LabelHint` and per-session epoch) and all state constructors.
- `crates/orkworksd/src/runtime/terminal_runtime.rs` — accepted-input reset detection, atomic label reset, re-seeding, and terminal-runtime regression coverage.
- `crates/orkworksd/src/runtime/peon_runtime.rs` — epoch-aware `InputLabel` work and deterministic stale-result coverage.

### Task 1: Make reset commands a resolved harness capability

**Files:**
- Modify: `crates/orkworksd/resources/harnesses-v2.json`
- Modify: `crates/orkworksd/src/harness/definition.rs:11-24,130-177,240-280,432-550,743-940`
- Modify: `crates/orkworksd/src/harness/store.rs:435-470`

**Interfaces:**
- Produces: `HarnessDefinition::label_reset_commands: Vec<String>`.
- Produces: `HarnessPatch::label_reset_commands: Option<Option<Vec<String>>>`, where `None` means unchanged, `Some(Some(commands))` replaces, and `Some(None)` clears to `[]`.
- Consumed by Task 2 through `ResolvedHarness.definition.label_reset_commands`.

- [ ] **Step 1: Write the failing registry tests**

Add tests beside the existing sparse-patch tests which pin the entire contract:

```rust
#[test]
fn label_reset_commands_default_for_legacy_custom_documents() {
    let document: HarnessUserDocument = serde_json::from_str(
        r#"{"version":2,"custom":[{
          "id":"company-tool","name":"Company Tool",
          "launch":{"kind":"command-template","command":"company-tool","args":[],"modelPrefix":null},
          "defaultModel":null,"resume":null,"models":null,"peon":null,
          "capacity":null,"sessionSignals":null,"integration":null,"voice":null
        }]}"#,
    ).unwrap();
    assert!(document.custom[0].label_reset_commands.is_empty());
}

#[test]
fn label_reset_command_patch_replaces_or_clears_the_builtin_list() {
    let original = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()
        .builtins.into_iter().find(|h| h.id == "claude-code").unwrap();
    assert_eq!(original.label_reset_commands, ["/clear", "/reset", "/new"]);

    let replacement: HarnessPatch = serde_json::from_str(
        r#"{"labelResetCommands":["/fresh"]}"#,
    ).unwrap();
    assert_eq!(original.apply_patch(&replacement).unwrap().label_reset_commands, ["/fresh"]);

    let cleared: HarnessPatch = serde_json::from_str(r#"{"labelResetCommands":null}"#).unwrap();
    assert!(original.apply_patch(&cleared).unwrap().label_reset_commands.is_empty());
}
```

Extend `embedded_builtins_are_complete_and_valid` with assertions for Claude Code, OpenCode, and Codex so the registry cannot silently change the supported command map.

- [ ] **Step 2: Run the registry tests and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests`

Expected: compilation errors that `HarnessDefinition` and `HarnessPatch` do not expose `label_reset_commands`.

- [ ] **Step 3: Implement the schema and data declarations**

Add the defaulted definition field and the optional sparse-patch boundary:

```rust
#[serde(default)]
pub label_reset_commands: Vec<String>,

#[serde(skip_serializing_if = "Option::is_none")]
pub label_reset_commands: Option<Option<Vec<String>>>,
```

In `HarnessPatch::deserialize`, add `"labelResetCommands"` to the strict allow-list and populate it with:

```rust
label_reset_commands: optional_boundary_field(&fields, "labelResetCommands")?,
```

In `HarnessDefinition::apply_patch`, apply replacement/null semantics without merging:

```rust
if let Some(commands) = &patch.label_reset_commands {
    result.label_reset_commands = commands.clone().unwrap_or_default();
}
```

Set `label_reset_commands: Vec::new()` in `legacy_definition`. Add the JSON fields only to the two verified built-ins:

```json
"labelResetCommands": ["/clear", "/reset", "/new"]
```

for `claude-code`, and:

```json
"labelResetCommands": ["/clear", "/new"]
```

for `opencode`. Keep every other entry absent so its Serde default represents no declared support.

- [ ] **Step 4: Run the focused registry tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests`

Expected: PASS, including validation of built-ins, legacy custom parsing, sparse replacement, and explicit null clearing.

- [ ] **Step 5: Commit the capability layer**

```bash
git add crates/orkworksd/resources/harnesses-v2.json crates/orkworksd/src/harness/definition.rs crates/orkworksd/src/harness/store.rs
git commit -m "feat: declare harness label reset commands"
```

### Task 2: Reset and re-arm the terminal label lifecycle

**Files:**
- Modify: `crates/orkworksd/src/main.rs:79-101,158-174` and every test `PeonState` literal
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:313-484,1153-1820,2030-2780`

**Interfaces:**
- Consumes: `ResolvedHarness.definition.label_reset_commands` from Task 1 and persisted `SessionMetadata.harness`.
- Produces: `pub(crate) LabelHint { text: String, epoch: u64 }` and `PeonState::label_epochs: RwLock<HashMap<String, u64>>`.
- Produces: `reset_label_for_declared_command(state: &Arc<AppState>, id: &str, line: &str) -> bool` called only after sensitivity classification.
- Consumed by Task 3 through epoch-carrying label hints and `label_epochs`.

- [ ] **Step 1: Write failing terminal-runtime regression tests**

In the terminal runtime test module, build a Claude-backed metadata fixture by setting `meta.harness = "claude-code"`, then add tests that submit accepted completed input:

```rust
#[test]
fn declared_reset_replaces_live_and_persisted_label_and_rearms_seeding() {
    let id = "label-reset";
    let (state, _dir) = prompted_session_state(id);
    set_harness(&state, id, "claude-code");
    set_label(&state, id, "Old conversation title");
    queue_label_hint(&state, id, "old topic", 0);

    record_terminal_input(&state, id, "  /new\r");

    let placeholder = crate::session_types::placeholder_label(id);
    assert_eq!(live_label(&state, id), placeholder);
    assert_eq!(stored_label(&state, id), placeholder);
    assert!(state.peon.label_hint.read().unwrap().get(id).is_none());
    assert!(!state.peon.label_pending.read().unwrap().contains(id));
    assert_eq!(label_epoch(&state, id), 1);

    record_terminal_input(&state, id, "fix the next login bug\r");
    assert_eq!(live_label(&state, id), "fix the next login bug");
    assert_eq!(stored_label(&state, id), "fix the next login bug");
}
```

Add table-driven negative coverage for `"/new extra\r"`, `"new\r"`, and `"/NEW\r"`; each must leave an old label, hint, pending flag, and epoch untouched. Add separate tests proving `/new` is inert for `codex`, empty/missing harness IDs, and an unknown persisted harness ID; one proving a pre-dispatch sensitive `/new` leaves label, hint, pending state, and epoch unchanged; and one confirming `/new` is not stored as `last_user_input`.

- [ ] **Step 2: Run the terminal-runtime tests and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests`

Expected: FAIL because no reset path exists and hints have no epoch.

- [ ] **Step 3: Add epoch-bearing label work state**

In `main.rs`, define the small payload beside `PeonState`:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LabelHint {
    text: String,
    epoch: u64,
}
```

Change `label_hint` to `StdRwLock<HashMap<String, LabelHint>>` and add:

```rust
label_epochs: StdRwLock<HashMap<String, u64>>,
```

Initialize the new map in application startup and every explicit test `PeonState` literal. Update existing hint assertions to compare `hint.text` and its expected starting epoch instead of comparing raw strings.

- [ ] **Step 4: Implement exact harness-scoped reset and safe re-seeding**

Add helpers in `terminal_runtime.rs` with these responsibilities:

```rust
fn reset_command_for_persisted_harness(state: &Arc<AppState>, id: &str, line: &str) -> bool;
fn reset_label_for_declared_command(state: &Arc<AppState>, id: &str, line: &str) -> bool;
pub(crate) fn queue_label_hint(state: &Arc<AppState>, id: &str, line: String);
```

`reset_command_for_persisted_harness` must read the session metadata, use its non-empty `harness` value to look up the resolved catalog, and return true only when `line.trim()` equals one command in that definition's array. Do not fall back to `SessionInfo.harness_id`, an arbitrary harness, prefix matching, or case folding.

`reset_label_for_declared_command` must first return false when the command is not declared. For a declared command, acquire the per-session `label_epochs` write lock, increment the epoch with `saturating_add(1)`, remove the session from `label_hint` and `label_pending`, then update both copies to `placeholder_label(id)`: durable metadata first (when present), then `SessionInfo`. Hold the epoch write guard until both updates are complete, so Task 3's stale inference check and this reset serialize. Do not record the command as `last_user_input`.

In `record_terminal_input_impl`, after `is_sensitive` is known and before calculating `label_worthy`, branch as follows:

```rust
if !is_sensitive && reset_label_for_declared_command(state, id, &line) {
    return Some(());
}
```

For ordinary descriptive seeding, keep the current metadata-first fallback behavior, then call `queue_label_hint`. That helper must hold an epoch read guard while it captures the current epoch and inserts both `LabelHint { text: line, epoch }` and the pending ID. This prevents a reset from interleaving between epoch capture and hint insertion.

- [ ] **Step 5: Run the terminal-runtime tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests`

Expected: PASS, including immediate placeholder visibility, exact matching, harness scoping, sensitive-input protection, stale-work clearing, and next-topic re-seeding.

- [ ] **Step 6: Commit the reset lifecycle**

```bash
git add crates/orkworksd/src/main.rs crates/orkworksd/src/runtime/terminal_runtime.rs
git commit -m "feat: reset session labels for declared commands"
```

### Task 3: Reject stale Peon label inference after a reset

**Files:**
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs:20-150,340-1000`
- Modify: `crates/orkworksd/src/http/session_handlers.rs:1155-1185,3750-3840` only as required to enqueue initial-prompt hints with epoch zero/current epoch
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:2160-2200` only as required to enqueue its existing label hint with epoch zero/current epoch

**Interfaces:**
- Consumes: `LabelHint` and `PeonState::label_epochs` from Task 2.
- Produces: `input_label_epoch_is_current(captured: u64, current: u64) -> bool` and epoch-gated live/durable label writes.

- [ ] **Step 1: Write a deterministic stale-inference test**

Refactor the InputLabel write decision into a small pure helper and first pin it directly:

```rust
#[test]
fn input_label_epoch_is_stale_after_a_reset() {
    assert!(input_label_epoch_is_current(4, 4));
    assert!(!input_label_epoch_is_current(4, 5));
}
```

Then add an async regression using a delayed fake provider: queue `LabelHint { text: "old conversation".into(), epoch: 0 }`, let `peon_loop` take it into flight, issue the terminal `/new` reset while the provider is blocked, release the provider, and assert the live and metadata labels remain the placeholder. Submit a new descriptive line and assert an inference carrying epoch 1 may update that new fallback title. This test must use a synchronization barrier/channel, not a timing sleep, to make the old-result race reproducible.

- [ ] **Step 2: Run the Peon tests and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::peon_runtime::tests`

Expected: failure because the current worker treats a hint as a bare string and writes its result unconditionally.

- [ ] **Step 3: Carry the epoch through scheduling and gate both writes**

Keep the drained `label_pending` IDs, but when `InferenceMode::InputLabel` removes a hint, retain the complete `LabelHint`. Construct its prompt from `hint.text` and validate returned labels against `hint.text`.

Before changing a label after inference, acquire a read guard on `label_epochs` and require:

```rust
fn input_label_epoch_is_current(captured: u64, current: u64) -> bool {
    captured == current
}
```

Keep that read guard through both writes, taking locks in the same order as Task 2: workspace metadata then sessions. If the session epoch differs, skip both writes entirely. If a worker acquired the read guard before reset, it may finish its old write, but reset then waits for and supersedes it with the placeholder; if reset acquired its write guard first, the worker sees the mismatch and writes nothing. Always remove `in_flight` on every return path.

Update all other producers of `label_hint` (initial prompt creation and session-runtime fallback) to store `LabelHint` with the current per-session epoch under the same epoch read-lock discipline used by `queue_label_hint`.

- [ ] **Step 4: Run the Peon test module**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::peon_runtime::tests`

Expected: PASS, including normal label refinement, pending re-queue behavior, and the delayed old-epoch result being rejected.

- [ ] **Step 5: Commit the stale-result guard**

```bash
git add crates/orkworksd/src/runtime/peon_runtime.rs crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/src/runtime/session_runtime.rs
git commit -m "fix: reject stale session label inference"
```

### Task 4: Verify the complete sidecar change and prepare review

**Files:**
- Verify: `crates/orkworksd/resources/harnesses-v2.json`
- Verify: `crates/orkworksd/src/harness/definition.rs`
- Verify: `crates/orkworksd/src/harness/store.rs`
- Verify: `crates/orkworksd/src/main.rs`
- Verify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Verify: `crates/orkworksd/src/runtime/peon_runtime.rs`

**Interfaces:**
- Verifies: all contracts introduced by Tasks 1-3 and ADR 0040.

- [ ] **Step 1: Format the Rust changes**

Run: `rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`

Expected: either PASS or formatting changes required. If formatting is required, run `rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml`, then rerun the check until it passes.

- [ ] **Step 2: Run complete sidecar verification**

Run:

```bash
rtk cargo build --manifest-path crates/orkworksd/Cargo.toml
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
git diff --check
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: build and tests pass; no whitespace errors; doc check has no required update; worktree output is reviewed without modifying worktrees not owned by this branch.

- [ ] **Step 3: Inspect the final diff and request the required code review**

Run:

```bash
git diff origin/main...HEAD -- crates/orkworksd
rtk codex review --base origin/main
```

Expected: the diff contains only harness declarations/schema and label lifecycle changes; review findings are addressed or documented in the PR description.

- [ ] **Step 4: Commit any verification-only adjustment**

```bash
git add crates/orkworksd
git commit -m "test: cover label reset lifecycle"
```

Run this commit only if Tasks 1-3 did not already include every final test and formatting change; otherwise leave history at the three focused commits.
