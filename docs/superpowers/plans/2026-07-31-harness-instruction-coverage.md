# Harness Instruction Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent a scoped-instruction split from silently dropping required guidance for one of OrkWorks' configured coding-agent harnesses.

**Architecture:** Keep `AGENTS.md` authoritative until scoped instruction delivery is proven for Claude, Codex, Copilot, and OpenCode. Add one indexed coverage record that captures each harness's present entry point, scoping evidence, and the exact manual probe needed before moving local rules from the root file.

**Tech Stack:** Markdown, existing APM configuration, existing agent instruction files.

## Global Constraints

- Do not add dependencies, scripts, CI, or a task-state system.
- Keep Electron/renderer and metadata-protocol constraints in root `AGENTS.md`; they are cross-boundary contracts.
- Treat a root link as discovery only, not proof that a harness loads a nested instruction file.
- Preserve the root file as the complete fallback until all configured targets have validated scoped delivery.

---

### Task 1: Record and enforce scoped-instruction coverage

**Files:**
- Create: `docs/agents/harness-instruction-coverage.md`
- Modify: `AGENTS.md`
- Modify: `docs/superpowers/specs/2026-07-31-scoped-agent-instructions-design.md`
- Test: documentation review via `rg`, `git diff --check`, `.claude/hooks/doc-check.sh`, and `.claude/hooks/worktree-check.sh`

**Interfaces:**
- Consumes: `apm.yml` targets and the current root instruction entry points (`AGENTS.md`, `CLAUDE.md`, `.github/copilot-instructions.md`, `opencode.json`).
- Produces: One table defining the required proof before a rule may leave root `AGENTS.md`.

- [ ] **Step 1: Add the coverage record**

Create `docs/agents/harness-instruction-coverage.md` with one row each for Claude, Codex, Copilot, and OpenCode. Record the current entry point, mark nested/path-scoped delivery as `unverified` unless this repository has direct evidence, and define a manual probe: give the harness a path-local task whose answer depends on a unique local instruction, then retain the transcript or PR evidence.

- [ ] **Step 2: Add the root promotion rule**

Add a brief `Instruction scoping` section to `AGENTS.md` that links the coverage record, preserves root completeness as the fallback, and prohibits moving a rule from root until every configured target has validated delivery.

- [ ] **Step 3: Align the approved design**

Keep the design document's safety amendments: cross-boundary constraints remain root-owned, and scoped files are a later promotion after coverage evidence exists.

- [ ] **Step 4: Verify the record**

Run:

```bash
rg -n 'Claude|Codex|Copilot|OpenCode|unverified|AGENTS.md' docs/agents/harness-instruction-coverage.md AGENTS.md
git diff --check
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: all four configured harnesses and the root fallback rule are present; the diff has no whitespace errors; both repository checks exit successfully.

- [ ] **Step 5: Commit**

```bash
git add AGENTS.md docs/agents/harness-instruction-coverage.md docs/superpowers/specs/2026-07-31-scoped-agent-instructions-design.md docs/superpowers/plans/2026-07-31-harness-instruction-coverage.md
git commit -m "docs: gate scoped agent instructions by harness coverage"
```
