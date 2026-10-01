# Codex Permission Diagnostic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capture a redacted `PermissionRequest` and `PostToolUse` payload pair locally so issue #690 can establish whether Codex exposes turn-and-tool correlation fields.

**Architecture:** The shared shell and PowerShell reporters capture only an allowlist of safe top-level payload key names and the bounded scalar fields `hook_event_name`, `permission_mode`, `turn_id`, `tool_name`, and `tool_use_id` into a private diagnostic record. `tool_use_id` is retained only as a string of at most 128 characters. The diagnostic retains at most the latest record for each of those two events. `PostToolUse` is a capture-only event and skips attention and identity reporting; it becomes the fifth owned Codex event, changing the bundle fingerprint.

**Tech Stack:** Rust unit tests, Bash, PowerShell, Python JSON parsing, Markdown.

**Spec:** `docs/adr/0051-codex-deterministic-attention-hooks.md` (proposed verification gate); `docs/agents/harness-integration-contracts.md`.

## Global Constraints

- Do not change attention behavior.
- Never persist `tool_input`, `transcript_path`, `cwd`, session IDs, tokens, or arbitrary free text.
- Capture only the safe payload key names `hook_event_name`, `model`, `permission_mode`, `turn_id`, `tool_name`, `tool_use_id`, and `tool_response`, plus bounded scalar values `hook_event_name`, `permission_mode`, `turn_id`, `tool_name`, and `tool_use_id`.
- `PostToolUse` must have no attention or harness-session side effects.
- Preserve unrelated Codex hooks; native `/hooks` trust remains user controlled.

---

### Task 1: Pin reporter redaction and capture-only behavior

**Files:**
- Modify: `crates/orkworksd/src/harness/integrations/codex.rs` (unit tests)
- Test: invoke `crates/orkworksd/scripts/report-harness-event.sh` with a fake home and representative JSON payloads.

- [x] Add a failing test that runs `PermissionRequest` then `PostToolUse` and verifies both event records, their allowlisted fields, top-level payload keys, retention of only those event records, absence of forbidden values, and skipped reports for `PostToolUse`.
- [x] Run the focused test and confirm it fails because capture is absent and the fifth event is not installed.

### Task 2: Add bounded redacted diagnostic capture to both reporters

**Files:**
- Modify: `crates/orkworksd/scripts/report-harness-event.sh`
- Modify: `crates/orkworksd/scripts/report-harness-event.ps1`

- [x] Parse the top-level key names and allowlisted scalar values only for `PermissionRequest` and `PostToolUse`.
- [x] Preserve a bounded per-event record in the existing diagnostic JSON using the existing private permissions and atomic replacement pattern.
- [x] Mark `PostToolUse` as capture-only and skip attention and harness-session requests.
- [x] Run the focused Bash reporter test and confirm it passes. The PowerShell twin could not be executed because `pwsh` is not installed in this environment.

### Task 3: Install the fifth owned Codex event

**Files:**
- Modify: `crates/orkworksd/src/harness/integrations/codex.rs`

- [x] Add `PostToolUse` to the owned event bundle and update bundle tests to expect five owned groups.
- [x] Run the focused bundle test and confirm the fingerprint is derived from the updated generated event commands.

### Task 4: Document the verification contract

**Files:**
- Modify: `docs/agents/harness-integration-contracts.md`

- [x] Document the fifth capture-only event, local redacted pair diagnostic, allowlist, exclusions, and unchanged attention behavior.
- [x] Run `bash scripts/doc-check.sh` and `git diff --check`.

### Task 5: Verify and report evidence

- [x] Run `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.
- [x] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml`.
- [x] Run `bash scripts/verify-repo.sh` and `bash scripts/doc-check.sh`.
- [ ] Exercise the capture path in a real auto-review Codex session and confirm both event records appear without forbidden payload data.
- [ ] Post the observed correlation findings to issue #690 before any later step changes attention behavior.
