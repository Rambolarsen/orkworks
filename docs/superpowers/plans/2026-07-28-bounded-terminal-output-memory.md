# Bounded Terminal Output Memory Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate full-file line-vector allocations while reading or trimming persisted terminal output.

**Architecture:** `metadata.rs` gains one private buffered-tail helper that scans a terminal file in order, retains only the newest requested lines in a `VecDeque<String>`, and then drops oldest retained entries until the supplied byte budget fits. `read_terminal_output` consumes the helper with the 1 MiB replay budget; `trim_terminal_output` consumes it with the 768 KiB trim target and rewrites only when the helper discarded content.

**Tech Stack:** Rust 2021 standard library (`BufReader`, `BufRead`, `VecDeque`), existing `tempfile` unit-test support.

## Global Constraints

- Implement GitHub issue #192 only; renderer profiling is tracked separately in #247.
- Keep raw replay bounded to the existing newest 1,000 lines and 1 MiB contract.
- Preserve the 768 KiB trim target that supplies append headroom.
- Do not add dependencies or modify Electron/renderer APIs.
- Reading an oversized dormant replay file must never rewrite it; append-triggered trimming remains the mutation path.

---

## File Structure

- Modify: `crates/orkworksd/src/metadata.rs` — terminal-output tail selection, read/trim callers, and metadata-store regression tests.
- Modify: `docs/superpowers/specs/2026-07-28-terminal-output-bounded-memory-design.md` only if implementation discovers a contradiction; otherwise leave the approved design unchanged.

### Task 1: Pin the dormant-file and bounded-tail contract

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs:3155-3275`

**Interfaces:**
- Consumes: `MetadataStore::terminal_output_path(&self, id: &str) -> PathBuf` and `MetadataStore::read_terminal_output(&self, id: &str, max_lines: usize) -> Vec<String>`.
- Produces: Regression evidence that replay returns the newest requested lines without mutating an oversized dormant `.terminal` file.

- [ ] **Step 1: Write the failing test**

Add this unit test beside `terminal_output_round_trip_and_trim`:

```rust
#[test]
fn terminal_output_read_keeps_oversized_dormant_file_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(dir.path());
    let path = store.terminal_output_path("dormant-session");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let payload = "x".repeat(1_024);
    let original = (0..1_500)
        .map(|index| format!("line-{index}-{payload}"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(&path, &original).unwrap();

    let replay = store.read_terminal_output("dormant-session", 3);

    assert_eq!(
        replay,
        vec![
            format!("line-1497-{payload}"),
            format!("line-1498-{payload}"),
            format!("line-1499-{payload}"),
        ],
    );
    assert_eq!(fs::read_to_string(path).unwrap(), original);
}
```

- [ ] **Step 2: Run the test to verify the current behavior still passes for the wrong implementation shape**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_output_read_keeps_oversized_dormant_file_unchanged`

Expected: PASS. Record that this is an invariant test, not the RED test for the allocation fix; it protects ADR 0024 while the implementation is refactored.

- [ ] **Step 3: Add a failing helper-level bounded-retention test**

After introducing the planned helper signature in the test module import scope, add:

```rust
#[test]
fn terminal_output_tail_keeps_only_newest_lines_before_byte_trimming() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.terminal");
    fs::write(&path, "zero\none\ntwo\nthree\nfour\n").unwrap();

    let tail = read_terminal_output_tail(&path, 3, 1_024).unwrap();

    assert!(tail.discarded);
    assert_eq!(tail.lines, VecDeque::from(["two".into(), "three".into(), "four".into()]));
}
```

- [ ] **Step 4: Run the helper test to verify it fails correctly**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_output_tail_keeps_only_newest_lines_before_byte_trimming`

Expected: FAIL because `read_terminal_output_tail` does not exist.

- [ ] **Step 5: Commit the red-test contract**

```bash
rtk git add crates/orkworksd/src/metadata.rs
rtk git commit -m "test: cover bounded terminal output tail"
```

### Task 2: Implement buffered bounded-tail selection

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs:1-20, 1497-1539`
- Test: `crates/orkworksd/src/metadata.rs:3155-3275`

**Interfaces:**
- Consumes: a terminal `Path`, a line cap, and a byte cap.
- Produces: `TerminalOutputTail { lines: VecDeque<String>, discarded: bool }`, where `discarded` is true whenever the line or byte budget dropped input.

- [ ] **Step 1: Add the minimal private tail type and helper**

Replace the `HashMap` import with `use std::collections::{HashMap, VecDeque};`, remove `terminal_output_retain_start`, and add the helper below `impl MetadataStore`:

```rust
struct TerminalOutputTail {
    lines: VecDeque<String>,
    discarded: bool,
}

fn read_terminal_output_tail(
    path: &Path,
    max_lines: usize,
    max_bytes: u64,
) -> std::io::Result<TerminalOutputTail> {
    let file = fs::File::open(path)?;
    let mut lines = VecDeque::new();
    let mut retained_bytes = 0_u64;
    let mut discarded = false;

    for line in BufReader::new(file).lines() {
        let line = line?;
        if max_lines == 0 {
            discarded = true;
            continue;
        }
        if lines.len() == max_lines {
            let removed = lines.pop_front().expect("non-empty at line limit");
            retained_bytes -= removed.len() as u64 + 1;
            discarded = true;
        }
        retained_bytes += line.len() as u64 + 1;
        lines.push_back(line);
    }
    while retained_bytes > max_bytes {
        let Some(removed) = lines.pop_front() else { break };
        retained_bytes -= removed.len() as u64 + 1;
        discarded = true;
    }

    Ok(TerminalOutputTail { lines, discarded })
}
```

- [ ] **Step 2: Route the public read and trim paths through the helper**

Replace both full-file reads with:

```rust
pub fn read_terminal_output(&self, id: &str, max_lines: usize) -> Vec<String> {
    read_terminal_output_tail(
        &self.terminal_output_path(id),
        max_lines,
        TERMINAL_OUTPUT_MAX_BYTES,
    )
    .map(|tail| tail.lines.into_iter().collect())
    .unwrap_or_default()
}

pub fn trim_terminal_output(&self, id: &str, max_lines: usize) {
    let path = self.terminal_output_path(id);
    let Ok(tail) = read_terminal_output_tail(
        &path,
        max_lines,
        TERMINAL_OUTPUT_TRIM_TARGET_BYTES,
    ) else { return };
    if !tail.discarded { return; }
    let content = tail.lines.into_iter().collect::<Vec<_>>().join("\n") + "\n";
    if let Err(error) = fs::write(&path, content) {
        warn!("failed to trim terminal output for {id}: {error}");
    }
}
```

Keep the current trim serialization exactly: when content was discarded, write the retained lines joined by `\n` followed by one trailing `\n`. This also preserves the current single-newline result when the byte budget discards every input line.

- [ ] **Step 3: Run focused metadata tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_output_`

Expected: PASS, including existing line-limit, byte-limit, and trim-headroom tests.

- [ ] **Step 4: Format and run the full Rust suite**

Run: `rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`

Expected: PASS.

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit the implementation**

```bash
rtk git add crates/orkworksd/src/metadata.rs
rtk git commit -m "fix: bound terminal output read memory"
```

### Task 3: Verify the issue and repository handoff conditions

**Files:**
- Verify only: `crates/orkworksd/src/metadata.rs`, `docs/superpowers/specs/2026-07-28-terminal-output-bounded-memory-design.md`

**Interfaces:**
- Consumes: the completed bounded-tail helper and test suite.
- Produces: evidence that #192 acceptance criteria and documentation checks are satisfied.

- [ ] **Step 1: Inspect the final diff against the branch base**

Run: `rtk git diff aa6c4afe8fbfe5677b59c153cfbdcc6f4e6a2d91...HEAD -- crates/orkworksd/src/metadata.rs`

Expected: only the terminal-output allocation strategy and its tests change.

- [ ] **Step 2: Run documentation and worktree currency checks**

Run: `bash .claude/hooks/doc-check.sh`

Expected: no unaddressed documentation trigger.

Run: `bash .claude/hooks/worktree-check.sh`

Expected: report all worktrees; act only on this branch.

- [ ] **Step 3: Request the required lightweight code review**

Review the final diff for #192 acceptance criteria, ADR 0024's dormant-file rule, error behavior, and test honesty. Address Critical and Important findings before PR handoff.

- [ ] **Step 4: Commit any review-driven correction separately**

```bash
rtk git add crates/orkworksd/src/metadata.rs
rtk git commit -m "fix: address terminal output review findings"
```
