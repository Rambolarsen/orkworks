# Codex Session ID Integrity Implementation Plan

> **For agentic workers:** Execute this plan inline, one task at a time. Use test-driven development for every behavior change.

**Goal:** Preserve the Codex conversation ID owned by an OrkWorks session and resume it only while Codex has that exact thread saved locally.

**Architecture:** Validate Codex identity replacement at the authenticated session-report boundary, using Codex's `SessionStart.source` to allow an explicit `clear` transition. Before launching an exact Codex resume, verify the matching `state_5.sqlite` thread and rollout file still exist. Remove Codex's `resume --last` capability so missing identity never resumes another conversation.

**Tech Stack:** Rust, Axum, rusqlite, POSIX/PowerShell hook reporters, embedded JSON harness definitions, Cargo tests.

**Spec:** `specs/orkworks-mvp.md` (harness-native session ID capture); clarified by ADR 0067, which supersedes ADR 0066's process-binding requirement.

## Global Constraints

- Codex hook trust remains an explicit user action through `/hooks`.
- Never substitute another Codex conversation when the exact native ID is missing or no longer saved.
- Do not log or expose native session IDs as diagnostics.
- Keep workspace hook mutation opt-in and ownership-aware.

### Scope clarification (2026-09-26)

The child agents in scope are Codex CLI subagents running inside the owning
Codex session, not separate Codex CLI processes or OrkWorks sessions. They do
not receive independent OrkWorks session IDs. The reporter captures native
identity only on the root `SessionStart`; other events, including a subagent
event if one is supplied, do not submit native identity. As approved by the
user, reset replacement is authorized by the authenticated root `SessionStart`
and OrkWorks' explicit reset record; reporter-supplied process IDs are not
treated as process authentication.

### Follow-up: remove untrusted process identity from clear authorization

- [x] Test that an authenticated `SessionStart(source=clear)` replaces the
  native ID only after OrkWorks records the reset; unauthenticated, unrecorded,
  or non-`SessionStart` reports cannot replace it.
- [x] Capture native identity only for root `SessionStart`; subagent events do
  not report an independent ID.
- [x] Remove caller-supplied process IDs and process-owner state from the
  reporter, HTTP request, reset grant, and session lifecycle cleanup.
- [x] Update ADR 0067, the MVP spec, and harness integration docs with the
  subagent ownership and reset authorization rules.
- [x] Run reporter, session identity, lifecycle, full sidecar, formatting, and
  documentation checks.
- [ ] Push and request the single approved fresh review.

---

### Task 1: Protect Codex identity and exact resume

**Files:**
- Modify: `crates/orkworksd/src/codex_session_store.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs`
- Modify: `crates/orkworksd/scripts/report-harness-event.sh`
- Modify: `crates/orkworksd/scripts/report-harness-event.ps1`
- Modify: `crates/orkworksd/resources/harnesses-v2.json`
- Modify: `crates/orkworksd/src/harness/registry.rs`
- Modify: `crates/orkworksd/src/session_view.rs`
- Modify: `docs/agents/harness-integration-contracts.md`
- Modify: `docs/agents/architecture.md`
- Create: `docs/adr/0066-codex-exact-session-identity-and-resume.md`
- Create: `docs/adr/0067-codex-subagents-share-owning-session-identity.md`
- Modify: `docs/adr/0066-codex-exact-session-identity-and-resume.md` to mark its process-binding decision superseded
- Modify: `docs/adr/README.md`
- Test: focused module tests in the Rust files above and reporter-script integration tests.

**Interfaces:**
- Codex harness-session reports may include `sessionStartSource`, accepted only for a Codex `SessionStart` event.
- Codex store lookup returns whether an exact thread row points to an existing rollout file; unavailable, malformed, or absent local state returns false.
- Metadata merges preserve an existing Codex ID unless a validated `SessionStart` report has `source: clear`.

- [x] **Step 1: Write failing tests** for an existing Codex ID rejecting a different startup ID, accepting a different ID only for `clear`, exact Codex resume being unavailable without an ID, and saved-thread lookup requiring both the thread row and rollout file.
- [x] **Step 2: Run each focused test** and confirm it fails on the current behavior.
- [x] **Step 3: Implement identity-source validation and exact-ID-only Codex configuration.** The reporter forwards the SessionStart source; the request handler passes it only for the identity report; the metadata merge rejects unapproved identity changes.
- [x] **Step 4: Implement exact saved-thread preflight.** Read only `state_5.sqlite` in read-only mode, resolve the matching `rollout_path`, and check the file before spawning `codex resume <id>`.
- [x] **Step 5: Run focused Rust tests** for metadata merge, Codex store, harness registry, and resume workflow; run reporter integration tests for POSIX and PowerShell payload handling.
- [x] **Step 6: Update ADR 0066/0067 and harness/architecture docs** to state the exact ID ownership, explicit-clear replacement, saved-rollout requirement, and no `--last` fallback.
- [x] **Step 7: Run `cargo fmt --check`, relevant `cargo test`, `git diff --check`, `bash scripts/doc-check.sh`, and `bash .claude/hooks/worktree-check.sh`.**
