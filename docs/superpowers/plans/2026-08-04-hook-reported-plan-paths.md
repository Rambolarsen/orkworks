# Hook-reported plan paths Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist the actual Claude-written plan/spec without changing session attention.

**Architecture:** Claude's `PostToolUse` hook forwards its raw file path to a path-only route. Rust validates and normalizes it, then replaces any fallback association. Terminal scanning stays as the hookless fallback.

**Tech Stack:** Rust, Axum, Bash, PowerShell, Claude Code hook JSON.

## Global Constraints

- No renderer contract, arbitrary terminal input, or new dependency.
- Validate and canonicalize all hook-supplied paths in Rust.
- Codex remains on the fallback until it exposes a canonical file path.

---

### Task 1: Sidecar path-only report

**Files:**

- Modify: `crates/orkworksd/src/plan_handoff.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`

**Interfaces:**

- Consumes: `POST /sessions/:id/plan-path` with `{ "planPath": string }`.
- Produces: normalized `SessionMetadata.plan_path`, preserving attention.

- [ ] Write failing route tests for a valid absolute plan, workspace escape, control character, non-Markdown file, and symlink escape. Assert a valid report stores `docs/superpowers/plans/plan.md` and leaves `attention == "working"`.
- [ ] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml report_session_plan_path`; expect failure because the route is absent.
- [ ] Add `normalize_reported_plan_path` plus `report_session_plan_path`. Canonicalize the raw input, require a regular Markdown file below an accepted root, derive its workspace-relative representation, store it, and append `session.plan_path_hooked` without calling attention merge code.
- [ ] Re-run the focused test; expect pass.
- [ ] Commit with `git commit -m "feat(sidecar): accept validated hook plan paths"`.

### Task 2: Claude hook transport

**Files:**

- Modify: `crates/orkworksd/src/harness/integrations/claude.rs`
- Modify: `crates/orkworksd/scripts/report-harness-event.sh`
- Modify: `crates/orkworksd/scripts/report-harness-event.ps1`
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs`

**Interfaces:**

- Consumes: Claude `PostToolUse` `Write|Edit` payload with `tool_input.file_path`.
- Produces: one bounded `POST /sessions/:id/plan-path`; no attention update.

- [ ] Write failing tests for the installed `PostToolUse` matcher, probe/remove symmetry, and a real reporter trace containing the path-only URL but no generic attention request.
- [ ] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml report_harness_event --lib`; expect failure because the hook and path request are absent.
- [ ] Add the owned synchronous `PostToolUse` `Write|Edit` hook with a `--plan-path` reporter mode. In both scripts, extract only Claude's raw `tool_input.file_path`, post it with existing request timeouts, and skip attention in that mode.
- [ ] Re-run the focused test; expect pass.
- [ ] Update `specs/session-plan-review.md` and `docs/agents/harness-integration-contracts.md`, then commit with `git commit -m "feat(claude): report written plan paths"`.

### Task 3: Verify the complete change

- [ ] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml`; expect pass.
- [ ] Run `bash .claude/hooks/doc-check.sh` and `bash .claude/hooks/worktree-check.sh`; resolve only drift owned by this branch.
