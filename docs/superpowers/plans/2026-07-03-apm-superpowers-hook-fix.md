# APM And Superpowers Hook Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Update APM and Superpowers, regenerate hook artifacts, and verify Codex no longer fails on SessionStart.

**Architecture:** Refresh the CLI layer first, then refresh the repo dependency lock and generated hook artifacts, then verify the exact failing hook path. If upstream still emits stale Codex hook wiring, remove only that wiring locally.

**Tech Stack:** APM CLI, APM lockfile, Codex hook config JSON, shell verification

---

### Task 1: Refresh The Toolchain

**Files:**
- Modify: `apm.lock.yaml`
- Verify: `.codex/hooks.json`

- [ ] **Step 1: Check for newer APM CLI and dependency revisions**

Run: `apm self-update --check`
Run: `apm outdated -v`
Expected: A newer APM CLI and/or `obra/superpowers` revision is reported, or the command explains why not.

- [ ] **Step 2: Update the APM CLI**

Run: `apm self-update`
Expected: The local `apm` binary is replaced with a newer version.

- [ ] **Step 3: Refresh repo dependencies**

Run: `apm update -y obra/superpowers`
Expected: `apm.lock.yaml` resolves `obra/superpowers` to a newer upstream revision.

- [ ] **Step 4: Regenerate installed artifacts**

Run: `apm install`
Expected: Codex and Claude hook artifacts are regenerated from the updated package set.

### Task 2: Verify And Apply Narrow Fallback If Needed

**Files:**
- Modify: `.codex/hooks.json`
- Verify: `.codex/hooks/superpowers/hooks/run-hook.cmd`

- [ ] **Step 1: Inspect generated Codex hook wiring**

Run: `sed -n '1,240p' .codex/hooks.json`
Expected: Superpowers is absent from Codex `SessionStart`, or the stale hook entry is clearly visible.

- [ ] **Step 2: Reproduce the direct hook invocation**

Run: `bash .codex/hooks/superpowers/hooks/run-hook.cmd session-start`
Expected: No exit code `127`.

- [ ] **Step 3: Apply narrow local fallback only if needed**

If `.codex/hooks.json` still contains the Superpowers `SessionStart` entry, remove only that entry and leave unrelated hook integrations intact.

- [ ] **Step 4: Re-run verification**

Run: `bash .codex/hooks/superpowers/hooks/run-hook.cmd session-start`
Expected: No exit code `127`, or the hook is no longer present because Codex no longer ships it.

- [ ] **Step 5: Run repo completion checks**

Run: `bash .claude/hooks/doc-check.sh`
Expected: No additional doc updates are required.
