---
type: "Implementation Plan"
title: "Codex Permission Diagnostic Implementation Plan"
description: "Implementation plan: Codex Permission Diagnostic Implementation Plan."
tags: ["orkworks", "plans"]
---

# Codex Permission Diagnostic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capture redacted `PreToolUse`, `PermissionRequest`, and `PostToolUse` events locally under both auto-review and manual-prompt modes so issue #690 can establish whether Codex exposes turn-and-tool correlation fields and whether `PreToolUse` IDs correlate with permission events.

**Architecture:** The shared shell and PowerShell reporters capture only an allowlist of safe top-level payload key names and the bounded scalar fields `hook_event_name`, `permission_mode`, `turn_id`, `tool_name`, and `tool_use_id` into a private diagnostic record. `tool_use_id` is retained only as a string of at most 128 characters. The diagnostic retains a bounded ordered sequence of at most 16 records for `PreToolUse`, `PermissionRequest`, and `PostToolUse`, preserving event order without storing session IDs. `PreToolUse` and `PostToolUse` are capture-only events and skip attention and identity reporting; together they change the Codex bundle to six owned events and change its fingerprint.

**Tech Stack:** Rust unit tests, Bash, PowerShell, Python JSON parsing, Markdown.

**Spec:** `docs/adr/0051-codex-deterministic-attention-hooks.md` (proposed verification gate); `docs/agents/harness-integration-contracts.md`.

## Global Constraints

- Do not change attention behavior.
- Never persist `tool_input`, `transcript_path`, `cwd`, session IDs, tokens, or arbitrary free text.
- Capture only the safe payload key names `hook_event_name`, `model`, `permission_mode`, `turn_id`, `tool_name`, `tool_use_id`, and `tool_response`, plus bounded scalar values `hook_event_name`, `permission_mode`, `turn_id`, `tool_name`, and `tool_use_id`.
- `PreToolUse` and `PostToolUse` must have no attention or harness-session side effects.
- Preserve unrelated Codex hooks; native `/hooks` trust remains user controlled.

---

### Initial experiment (completed): `PermissionRequest` and `PostToolUse`

Tasks 1–5 below record the initial two-event experiment that was implemented
first. The follow-up experiment at the end adds `PreToolUse` without changing
the earlier attention contract.

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
- [x] Preserve a bounded ordered event sequence in the existing diagnostic JSON using private permissions and atomic replacement in both reporters.
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
- [ ] Exercise the capture path in a real auto-review Codex session and a real manual-prompt session; confirm both event records appear without forbidden payload data.
- [ ] Post the observed correlation findings from both modes to issue #690 before any later step changes attention behavior.

### Follow-up experiment: Capture `PreToolUse` invocation IDs

The user approved a capture-only experiment to test whether `PreToolUse`'s
`tool_use_id` can be connected to `PermissionRequest` by event order and to the
matching `PostToolUse`. This expands the Codex bundle to six events and changes
its fingerprint. It does not change attention behavior.

- [x] Add a failing reporter test requiring redacted `PreToolUse`, `PermissionRequest`, and `PostToolUse` captures, matching IDs only on the pre/post pair, and no attention or identity POSTs for the capture-only events.
- [x] Add `PreToolUse` to the owned Codex event bundle and both reporters' bounded capture allowlist.
- [ ] Observe real auto-review and manual-prompt event ordering before proposing any attention behavior change.
