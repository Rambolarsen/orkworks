# Claude Windows Hooks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make project and installed Claude Code hooks use supported Windows hook execution semantics.

**Architecture:** Project-maintained shell hooks explicitly select Bash. The Rust Claude integration continues to choose the platform-specific reporter asset and emits PowerShell for Windows; tests pin that contract.

**Tech Stack:** Claude Code JSON settings, Rust unit tests, PowerShell validation.

## Global Constraints

- Use pnpm for Node package-management tasks.
- Do not modify the untracked machine-local `.claude/settings.local.json`.
- Preserve the Electron/renderer boundary.
- Keep the change scoped to Claude hook compatibility.

---

### Task 1: Pin the supported Windows command shape

**Files:**
- Modify: `.claude/settings.json`
- Test: `crates/orkworksd/src/harness/integrations/claude.rs`

- [ ] **Step 1: Add a failing assertion** that the Windows reporter invocation uses `powershell.exe`, the `.ps1` asset, and `-Marker`/`-Status`/`-ReportPlanPath` parameters.
- [ ] **Step 2: Run the focused Rust tests** and confirm the new assertion fails against the current generated shape if it exposes a missing requirement.
- [ ] **Step 3: Update `.claude/settings.json`** to remove unsupported `commandWindows` fields and set the supported shell field for each hook command.
- [ ] **Step 4: Run the focused Rust tests and JSON parse check** and confirm they pass.

### Task 2: Repository verification

**Files:**
- No additional production files.

- [ ] **Step 1: Run the Claude integration test subset.**
- [ ] **Step 2: Run `bash .claude/hooks/doc-check.sh`.**
- [ ] **Step 3: Run `bash .claude/hooks/worktree-check.sh`.**
- [ ] **Step 4: Inspect `git diff` and report the commit limitation caused by `.git/index.lock` if it remains.
