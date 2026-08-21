# Copilot Label Reset Commands Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Declare Copilot CLI's verified bare fresh-conversation commands so OrkWorks resets the session label for Copilot sessions using the existing harness capability.

**Architecture:** Keep label-reset behavior declarative. Add Copilot's two exact declared commands to the builtin definition, then extend the existing definition and runtime regression tests; no runtime implementation change is expected because `terminal_runtime` already matches the persisted harness's declared commands.

**Tech Stack:** Rust, serde-backed harness definitions, embedded JSON, Cargo unit tests.

## Global Constraints

- Use the persisted session harness ID when testing reset behavior; do not broaden runtime matching.
- Match only exact trimmed commands; `/new prompt` and other prompt-bearing forms remain outside this change.
- Copilot currently declares only `/clear` and `/new`. `/reset` is intentionally deferred because the existing `minVersion` gate controls integration-status probing only, not terminal label-reset matching; do not add a runtime version-capability mechanism for it.
- Do not add model, voice, capacity, hook, UI, or terminal-text inference behavior.
- Preserve the existing label-reset semantics from ADR 0040: successful, non-sensitive, delivered commands reset the label and invalidate stale label work.
- Use the [GitHub Copilot CLI command reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference) as the command authority. Match only bare commands: optional prompt-bearing forms remain ordinary input.
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

- [x] **Step 1: Add the failing assertion**

Extend `embedded_builtins_are_complete_and_valid` with:

```rust
assert_eq!(
    resolved.get("copilot").unwrap().definition.label_reset_commands,
    ["/clear", "/new"]
);
```

- [x] **Step 2: Run the focused test and verify RED**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests::embedded_builtins_are_complete_and_valid
```

Expected: FAIL before the declaration is added because Copilot's resolved
`label_reset_commands` is empty. The test must compile and fail on the missing
declaration, not report a test or parse error.

- [x] **Step 3: Add Copilot cases to the runtime test matrix**

Extend the test's array with:

```rust
("copilot /clear", "copilot", "/clear"),
("copilot /new", "copilot", "/new"),
```

- [x] **Step 4: Run the focused runtime test and verify RED**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests::each_declared_command_resets_its_own_harness
```

Expected: FAIL for the first Copilot case before the declaration is added.
Existing Claude Code and OpenCode cases should still pass; do not add a
`/reset` case.

### Task 2: Add Copilot's declared reset commands (GREEN)

**Files:**
- Modify: `crates/orkworksd/resources/harnesses-v2.json:10`

**Interfaces:**
- Consumes: The failing definition and runtime assertions from Task 1.
- Produces: Copilot's resolved builtin definition with `labelResetCommands` equal to `[/clear, /new]`.

- [x] **Step 1: Update the Copilot builtin JSON entry**

Add the field to the end of the Copilot object, preserving the existing
single-line resource format:

```json
"labelResetCommands": ["/clear", "/new"]
```

- [x] **Step 2: Run the focused definition tests and verify GREEN**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests::embedded_builtins_are_complete_and_valid
```

Expected: PASS with Copilot's command list resolved exactly as `["/clear",
"/new"]`; the existing min-version test continues to cover Codex only.

The existing runtime assertions already verify the placeholder label,
persisted label, cleared label hint and pending work, and incremented label
epoch. After the JSON change, run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests::each_declared_command_resets_its_own_harness
```

Expected: PASS for Claude Code, OpenCode, and Copilot's `/clear` and `/new`
commands.

- [x] **Step 3: Confirm exact-match behavior remains covered**

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

- [x] **Step 1: Run Rust formatting and diff checks**

Run:

```bash
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
rtk git diff --check
```

Expected: the diff check succeeds without whitespace findings. Cargo fmt may
report pre-existing formatting drift outside this change; do not reformat
unrelated sidecar code as part of this issue.

- [x] **Step 2: Run the complete Rust test suite**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: PASS; the verified run completed 645 tests. Any pre-existing
environment-only failures or warnings must be recorded in the handoff report.

- [x] **Step 3: Build the Rust sidecar**

Run:

```bash
cargo build --manifest-path crates/orkworksd/Cargo.toml
```

Expected: successful debug build.

- [x] **Step 4: Run repository checks**

Run:

```bash
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: no unaddressed documentation-drift trigger and no owned stale
worktree warning.

- [x] **Step 5: Review the final diff and commit implementation**

Run:

```bash
rtk git diff --stat
rtk git diff -- crates/orkworksd/resources/harnesses-v2.json crates/orkworksd/src/harness/definition.rs crates/orkworksd/src/runtime/terminal_runtime.rs
rtk git add crates/orkworksd/resources/harnesses-v2.json crates/orkworksd/src/harness/definition.rs crates/orkworksd/src/runtime/terminal_runtime.rs
rtk git commit -m "feat(harness): declare Copilot label reset commands"
```

Expected: the implementation files and the required design, ADR, plan, and
verification documentation are committed on the feature branch; the branch
is ready for review and squash merge.
