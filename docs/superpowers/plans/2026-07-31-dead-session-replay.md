# Dead-session replay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve terminal record delimiters so dead-session replay does not inject cursor-moving CRLF bytes.

**Architecture:** New terminal history uses collision-detectable versioned JSONL records that store text and its original `LF`, `CRLF`, or empty terminator. The sidecar returns raw records for that format and legacy strings for historic files; both renderer replay paths use `write()` only for raw records and retain `writeln()` for legacy output.

**Tech Stack:** Rust, serde JSON, Axum, React/TypeScript, xterm.js, Node test runner.

## Global Constraints

- Retain original LF, CRLF, or empty delimiters for newly persisted terminal output.
- Replay raw records with xterm `write()` without adding line endings.
- Existing `.terminal` files remain readable through the legacy `writeln()` path.
- Do not add dependencies, migrate old files, or change the 1,000-line/1 MiB retention contract.

---

### Task 1: Preserve and replay terminal record delimiters

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:367-387`
- Modify: `crates/orkworksd/src/metadata.rs:1498-1594`
- Modify: `crates/orkworksd/src/runtime/terminal_http.rs:15-52`
- Modify: `crates/orkworksd/src/metadata.rs` tests near `terminal_output_round_trip_and_trim`
- Modify: `crates/orkworksd/src/runtime/terminal_http.rs` tests near `get_terminal_output_reads_persisted_terminal_history_for_dead_session`
- Modify: `apps/desktop/src/api.ts:201-208`
- Modify: `apps/desktop/src/terminalReplay.ts:1-23`
- Modify: `apps/desktop/src/terminalStore.ts:180-189`
- Modify: `apps/desktop/tests/terminalReplay.test.ts`

**Interfaces:**
- Consumes: PTY output buffered by `drain_persist_records`.
- Produces: `TerminalOutputResponse` records represented in TypeScript as legacy text or raw replay text.

- [ ] **Step 1: Write focused failing tests**

Add a Rust test that drains `"one\\r\\ntwo\\nthree"` into records retaining `"\\r\\n"`, `"\\n"`, and `""`; add a metadata/API test that a new-format history returns raw records while a manually written legacy file returns strings. Add TypeScript tests that raw records call `write("one\\r\\n")` in both replay paths and legacy strings call `writeln("one")`.

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_output -- --nocapture` and `node --experimental-strip-types --test tests/terminalReplay.test.ts` from `apps/desktop`.

Expected: the new delimiter/raw-replay assertions fail because the current implementation discards delimiters and exposes only `string[]`.

- [ ] **Step 3: Implement the minimum compatible format and replay path**

Make `drain_persist_records` return text-plus-terminator records. Persist new records as RS-prefixed versioned JSONL while preserving the existing bounded trim behavior and encoded records during rewrites. Detect old, malformed, or unknown records and return their entries as legacy strings. Serialize raw records through `TerminalOutputResponse`; update the API type and make `terminalReplay.ts` and `terminalStore.ts` call `write()` for raw data while preserving `writeln()` for strings.

- [ ] **Step 4: Run focused tests and type-check**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_output -- --nocapture`, `node --experimental-strip-types --test tests/terminalReplay.test.ts`, and `npx tsc --noEmit` from `apps/desktop`.

Expected: all commands exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/runtime/session_runtime.rs crates/orkworksd/src/metadata.rs crates/orkworksd/src/runtime/terminal_http.rs apps/desktop/src/api.ts apps/desktop/src/terminalReplay.ts apps/desktop/src/terminalStore.ts apps/desktop/tests/terminalReplay.test.ts
git commit -m "fix: preserve dead-session terminal replay delimiters"
```
