# Cap PTY Partial-Line Persistence Buffer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bound in-memory PTY persistence buffering when a process emits output without newline characters, while preserving normal terminal history.

**Architecture:** Extract the existing partial-line persistence logic into a pure helper in `session_runtime.rs`. It first drains all newline-delimited records, then repeatedly flushes a capped synthetic record through a valid UTF-8 boundary when the remaining suffix is at least 64 KiB. The PTY's live WebSocket/replay path remains untouched.

**Tech Stack:** Rust, Tokio, portable-pty, Cargo tests

## Global Constraints

Keep ordinary newline and CRLF terminal-history records unchanged.
Keep the live PTY WebSocket and replay byte stream unchanged.
Do not drop partial terminal output or change the metadata persistence format.
Flush synthetic records at valid UTF-8 boundaries so a valid character split across PTY chunks persists as one character.
Do not solve on-disk terminal-history byte retention in this change; it is tracked by #160.

---

### Task 1: Specify bounded persistence extraction with failing unit tests

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:17-18`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:670-930`
- Test: `crates/orkworksd/src/runtime/session_runtime.rs`

**Interfaces:**
- Consumes: `&mut Vec<u8>` containing PTY bytes not yet persisted.
- Produces: `fn drain_persist_records(buffer: &mut Vec<u8>) -> Vec<String>`.
- Produces: `const MAX_PARTIAL_PERSIST_BYTES: usize = 64 * 1024`.

- [ ] **Step 1: Write failing tests for normal records, capped records, and boundaries**

```rust
#[test]
fn persist_records_keep_newline_delimited_output_unchanged() {
    let mut buffer = b"first\nsecond\r\npartial".to_vec();
    assert_eq!(
        drain_persist_records(&mut buffer),
        vec!["first".to_string(), "second".to_string()],
    );
    assert_eq!(buffer, b"partial");
}

#[test]
fn persist_records_flush_a_newline_free_suffix_at_the_byte_cap() {
    let mut buffer = vec![b'x'; MAX_PARTIAL_PERSIST_BYTES];
    assert_eq!(drain_persist_records(&mut buffer), vec!["x".repeat(MAX_PARTIAL_PERSIST_BYTES)]);
    assert!(buffer.is_empty());
}

#[test]
fn persist_records_keep_a_split_utf8_character_for_the_next_chunk() {
    let mut buffer = vec![b'x'; MAX_PARTIAL_PERSIST_BYTES - 1];
    buffer.extend_from_slice(&[0xE2, 0x82]);
    let records = drain_persist_records(&mut buffer);
    assert_eq!(records, vec!["x".repeat(MAX_PARTIAL_PERSIST_BYTES - 1)]);
    buffer.push(0xAC);
    assert!(drain_persist_records(&mut buffer).is_empty());
    assert_eq!(String::from_utf8(buffer).unwrap(), "€");
}
```

Add tests for CRLF split across two appends, a complete line followed by a capped suffix, and reconstructing a multi-flush newline-free stream from returned records plus the remaining buffer.

- [ ] **Step 2: Run the new tests and verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml persist_records`

Expected: compilation failure because `drain_persist_records` and `MAX_PARTIAL_PERSIST_BYTES` do not exist.

### Task 2: Implement lossless bounded record extraction

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:17-18`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:355-445`
- Test: `crates/orkworksd/src/runtime/session_runtime.rs`

**Interfaces:**
- Consumes: `persist_buffer: Vec<u8>` in the PTY driver task.
- Produces: `raw_persist_lines: Vec<String>` for the existing output buffer and bounded persistence queue.

- [ ] **Step 1: Add the helper and cap constant**

```rust
const MAX_PARTIAL_PERSIST_BYTES: usize = 64 * 1024;

fn drain_persist_records(buffer: &mut Vec<u8>) -> Vec<String> {
    let mut records = Vec::new();
    while let Some(nl) = buffer.iter().position(|&byte| byte == b'\n') {
        let line: Vec<u8> = buffer.drain(..=nl).collect();
        let end = if line.ends_with(b"\r\n") { line.len() - 2 } else { line.len() - 1 };
        records.push(String::from_utf8_lossy(&line[..end]).into_owned());
    }
    while buffer.len() >= MAX_PARTIAL_PERSIST_BYTES {
        let flush_end = utf8_flush_boundary(buffer, MAX_PARTIAL_PERSIST_BYTES);
        records.push(String::from_utf8_lossy(&buffer[..flush_end]).into_owned());
        buffer.drain(..flush_end);
    }
    records
}
```

Implement `utf8_flush_boundary` so it returns the greatest valid UTF-8 character boundary at or below the cap; when the cap ends inside a valid multibyte sequence, leave its incomplete prefix in `buffer` for the next input chunk. Keep existing lossy decoding for invalid terminal bytes.

- [ ] **Step 2: Replace the inline newline-drain loop in the driver output branch**

```rust
persist_buffer.extend_from_slice(&data);
let raw_persist_lines = drain_persist_records(&mut persist_buffer);
```

Leave all subsequent `raw_persist_lines` handling, live output forwarding, Peon updates, and queue backpressure unchanged.

- [ ] **Step 3: Run the focused tests and verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml persist_records`

Expected: all new `persist_records_*` tests pass.

- [ ] **Step 4: Commit the implementation**

```bash
git add crates/orkworksd/src/runtime/session_runtime.rs
git commit -m "fix(sidecar): cap partial PTY persistence buffer"
```

### Task 3: Verify runtime behavior and repository guardrails

**Files:**
- Verify only: `crates/orkworksd/src/runtime/session_runtime.rs`
- Verify only: `.claude/hooks/doc-check.sh`

- [ ] **Step 1: Run the full Rust test suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: exit 0 with no failed tests.

- [ ] **Step 2: Run the Rust linter**

Run: `cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings`

Expected: exit 0 with no warnings.

- [ ] **Step 3: Run the documentation currency check**

Run: `bash .claude/hooks/doc-check.sh`

Expected: exit 0 with no additional documentation changes required.

- [ ] **Step 4: Commit the implementation plan**

```bash
git add docs/superpowers/plans/2026-07-10-cap-persist-buffer.md
git commit -m "docs: plan capped PTY persistence buffer"
```
