# Copilot Label Reset Commands Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Declare Copilot CLI's verified bare fresh-conversation commands so OrkWorks resets the session label for Copilot sessions using the existing harness capability.

**Architecture:** Keep label-reset behavior declarative. Add the three exact commands to the Copilot builtin definition, then extend the existing definition and runtime regression tests; no runtime implementation change is expected because `terminal_runtime` already matches the persisted harness's declared commands.

**Tech Stack:** Rust, serde-backed harness definitions, embedded JSON, Cargo unit tests.

## Global Constraints

- Use the persisted session harness ID when testing reset behavior; do not broaden runtime matching.
- Match only exact trimmed commands; `/new prompt` and other prompt-bearing forms remain outside this change.
- Do not add model, voice, capacity, hook, UI, or terminal-text inference behavior.
- Preserve the existing label-reset semantics from ADR 0040: successful, non-sensitive, delivered commands reset the label and invalidate stale label work.
- Validate with `cargo build --manifest-path crates/orkworksd/Cargo.toml` and `cargo test --manifest-path crates/orkworksd/Cargo.toml`.

---

### Task 1: Add regression coverage for Copilot (RED)

**Files:**
- Modify: `crates/orkworksd/src/harness/definition.rs:781-823`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:2198-2227`

**Interfaces:**
- Consumes: The existing embedded-definition test and the existing
  `each_declared_command_resets_its_own_harness` test matrix.
- Produces: Definition and runtime assertions that initially fail because
  Copilot has no declared commands.

- [ ] **Step 1: Add the failing assertion**

Extend `embedded_builtins_are_complete_and_valid` with:

```rust
assert_eq!(
    resolved.get("copilot").unwrap().definition.label_reset_commands,
    ["/clear", "/new", "/reset"]
);
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests::embedded_builtins_are_complete_and_valid
```

Expected: FAIL because Copilot's current resolved `label_reset_commands` is
empty. The test must compile and fail on the missing declaration, not report a
test or parse error.

- [ ] **Step 3: Add Copilot cases to the runtime test matrix**

Extend the test's array with:

```rust
("copilot /clear", "copilot", "/clear"),
("copilot /new", "copilot", "/new"),
("copilot /reset", "copilot", "/reset"),
```

- [ ] **Step 4: Run the focused runtime test and verify RED**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests::each_declared_command_resets_its_own_harness
```

Expected: FAIL for the first Copilot case because the Copilot definition has
no declared commands. Existing Claude Code and OpenCode cases should still
pass; the failure must be the missing Copilot declaration.

### Task 2: Add Copilot's declared reset commands (GREEN)

**Files:**
- Modify: `crates/orkworksd/resources/harnesses-v2.json:10`

**Interfaces:**
- Consumes: The failing definition and runtime assertions from Task 1.
- Produces: Copilot's resolved builtin definition with `labelResetCommands` equal to `[/clear, /new, /reset]`.

- [ ] **Step 1: Update the Copilot builtin JSON entry**

Add the field to the end of the Copilot object, preserving the existing
single-line resource format:

```json
"labelResetCommands": ["/clear", "/new", "/reset"]
```

- [ ] **Step 2: Run the focused definition test and verify GREEN**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests::embedded_builtins_are_complete_and_valid
```

Expected: PASS with the Copilot list resolved exactly as
`["/clear", "/new", "/reset"]`.

The existing runtime assertions already verify the placeholder label,
persisted label, cleared label hint and pending work, and incremented label
epoch. After the JSON change, run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests::each_declared_command_resets_its_own_harness
```

Expected: PASS for Claude Code, OpenCode, and all three Copilot commands.

- [ ] **Step 3: Confirm exact-match behavior remains covered**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests
```

Expected: PASS, including existing negative coverage for non-exact commands;
do not add prompt-bearing matching to make the new cases pass.

### Task 3: Full verification and handoff

**Files:**
- Verify: `crates/orkworksd/resources/harnesses-v2.json`
- Verify: `crates/orkworksd/src/harness/definition.rs`
- Verify: `crates/orkworksd/src/runtime/terminal_runtime.rs`

**Interfaces:**
- Consumes: The completed Tasks 1–2 changes.
- Produces: A verified, review-ready branch for issue #326.

- [ ] **Step 1: Run Rust formatting and diff checks**

Run:

```bash
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
rtk git diff --check
```

Expected: both commands succeed without formatting or whitespace findings.

- [ ] **Step 2: Run the complete Rust test suite**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: PASS with no new warnings or failures.

- [ ] **Step 3: Build the Rust sidecar**

Run:

```bash
cargo build --manifest-path crates/orkworksd/Cargo.toml
```

Expected: successful debug build.

- [ ] **Step 4: Run repository checks**

Run:

```bash
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: no unaddressed documentation-drift trigger and no owned stale
worktree warning.

- [ ] **Step 5: Review the final diff and commit implementation**

Run:

```bash
rtk git diff --stat
rtk git diff -- crates/orkworksd/resources/harnesses-v2.json crates/orkworksd/src/harness/definition.rs crates/orkworksd/src/runtime/terminal_runtime.rs
rtk git add crates/orkworksd/resources/harnesses-v2.json crates/orkworksd/src/harness/definition.rs crates/orkworksd/src/runtime/terminal_runtime.rs
rtk git commit -m "feat(harness): declare Copilot label reset commands"
```

Expected: only the three implementation files are committed in this step;
the design and documentation reconciliation remain in commits `8a937ae` and
`ac38520`.
