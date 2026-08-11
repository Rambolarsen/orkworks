# Remove Stale Codex Ponytail Hooks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop Codex prompt-hook failures by retaining only the tracked Codex Stop hook and proving that it remains executable from a subdirectory.

**Architecture:** `apm.yml` restricts Ponytail to Copilot. The generated Agent Skills and Superpowers hook assets are not present in Codex’s committed runtime layout, so `.codex/hooks.json` must retain only its explicitly tracked Stop wrapper. A Node test will execute that wrapper from `apps/desktop/` and assert its JSON protocol for quiet, diagnostic, and error cases.

**Tech Stack:** JSON hook configuration; APM-generated dependency layout; Node.js command validation.

## Global Constraints

- Ponytail remains scoped to Copilot as declared in `apm.yml`.
- Keep the sole `hooks.Stop[0].hooks[0]` command unchanged.
- Do not modify `.codex/hooks/doc-check-stop.sh`, `.codex/hooks/doc-check.sh`, `apm.yml`, or `.claude/settings.json`.
- No dependency, product behavior, or application-source changes.

---

### Task 1: Remove broken Codex hook registrations and protect the Stop contract

**Files:**
- Modify: `.codex/hooks.json:13-85`
- Create: `apps/desktop/tests/codexStopHook.test.mjs`
- Test: `apps/desktop/tests/codexStopHook.test.mjs`

**Interfaces:**
- Consumes: `apm.yml`, which scopes `DietrichGebert/ponytail` to `[copilot]`, and the JSON protocol emitted by `.codex/hooks/doc-check-stop.sh`.
- Produces: a valid Codex hook configuration containing only the Stop wrapper, plus executable regression coverage for its command and output.

- [ ] **Step 1: Write and run failing regression coverage**

Create a Node test using `node:test` and `node:assert/strict`. It must parse `.codex/hooks.json`, recursively collect each `command`, `commandWindows`, `bash`, and `powershell` value, and fail while any command references Ponytail, Agent Skills, or Superpowers generated hook scripts. It must execute the Stop command through `bash -lc` while its current directory is `apps/desktop/`, then assert JSON output of `{}` when `ORKWORKS_DOC_CHECK_OUTPUT` is empty, `{ "systemMessage": message }` for a diagnostic message, and JSON `systemMessage` values for numeric and invalid `ORKWORKS_DOC_CHECK_EXIT_CODE` values.

Run: `cd apps/desktop && node --test tests/codexStopHook.test.mjs`

Expected before configuration removal: the stale-command assertion fails.

- [ ] **Step 2: Remove all unsupported hook registrations**

Delete the legacy lowercase `sessionStart` and `userPromptSubmitted` entries and all `SessionStart`, `UserPromptSubmit`, and `SubagentStart` groups. Preserve the `Stop` object byte-for-byte.

- [ ] **Step 3: Verify the focused regression test**

Run: `cd apps/desktop && node --test tests/codexStopHook.test.mjs`

Expected: all tests pass.

- [ ] **Step 4: Check required closeout signals**

Run: `bash .claude/hooks/doc-check.sh` and `bash .claude/hooks/worktree-check.sh`

Expected: each command finishes successfully; address any documentation flag caused by this change.

- [ ] **Step 5: Activate the corrected project hook**

In Codex, run `/hooks` and approve or re-trust the changed project hook configuration.
