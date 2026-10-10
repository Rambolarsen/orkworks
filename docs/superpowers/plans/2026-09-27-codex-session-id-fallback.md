---
type: "Implementation Plan"
title: "Codex Session ID Fallback Implementation Plan"
description: "Implementation plan: Codex Session ID Fallback Implementation Plan."
tags: ["orkworks", "plans"]
---

# Codex Session ID Fallback Implementation Plan

> **For agentic workers:** Execute this plan inline in the current session. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capture a missing Codex native session ID from a later trusted hook without weakening identity replacement rules.

**Architecture:** The Codex reporter will extract `session_id` from every Codex hook event, but will attach `SessionStart` source metadata only to the actual `SessionStart` event. The sidecar keeps its existing first-accepted-ID behavior and authenticated recorded-clear replacement guard. Update the accepted ADR and integration reference to describe these separate rules.

**Tech Stack:** Rust, Bash, PowerShell, Markdown.

**Spec:** `specs/orkworks-mvp.md`, “Harness-native session ID and Codex label enrichment” and “Codex identity and resume integrity”.

## Global Constraints

- Keep the first accepted native Codex session ID.
- A different ID may replace it only on an authenticated root `SessionStart(source=clear)` after OrkWorks records the explicit reset.
- Do not infer a native ID from terminal output or Codex rollout JSONL.
- Keep the reporter silent and bounded when OrkWorks reporting environment variables are absent.

---

### Task 1: Reproduce and fix late-hook ID capture

**Files:**
- Modify: `crates/orkworksd/tests/report_harness_event_script.rs`
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs`
- Modify: `crates/orkworksd/scripts/report-harness-event.sh`
- Modify: `crates/orkworksd/scripts/report-harness-event.ps1`

**Interfaces:**
- Consumes: Existing reporter flags `--event` / `-Event` and Codex common hook payload field `session_id`.
- Produces: `harnessSessionId` on the existing `POST /sessions/:id/harness-session` payload for any Codex hook event; `sessionStartSource` and `sessionStartEvent` only for `SessionStart`.

- [x] Add an end-to-end shell reporter test using a fake `curl`: pass a `UserPromptSubmit` payload with `session_id`, then assert the captured JSON contains `harnessSessionId` and omits both SessionStart-only fields.
- [x] Run that focused test and confirm it fails because the reporter currently produces no harness-session payload for `UserPromptSubmit`.
- [x] Update the reporter assertion to prove a configured later event can provide the owning ID.
- [x] Change Bash and PowerShell extraction so all Codex events read `session_id`, while source/event metadata remains gated on `SessionStart`.
- [x] Run the focused reporter tests and relevant Codex identity merge tests.

### Task 2: Synchronize the identity decision record

**Files:**
- Modify: `docs/adr/0068-codex-subagents-share-owning-session-identity.md`
- Modify: `docs/agents/harness-integration-contracts.md`

**Interfaces:**
- Consumes: The Codex Hooks common payload contract and the unchanged sidecar replacement guard.
- Produces: Documentation distinguishing initial first-ID capture from the stricter replacement authority.

- [x] Amend ADR 0068 with the Codex hook payload evidence and clarify that any trusted hook may supply the owning ID for initial capture; a replacement still requires authenticated root `SessionStart(source=clear)` plus the recorded reset.
- [x] Update the Codex row in the harness integration contract and the user-facing docs to match the amended decision.
- [x] Run formatting, focused Rust tests, `git diff --check`, `bash scripts/doc-check.sh`, and the full sidecar test suite.
