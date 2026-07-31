# Cross-Harness Scoped Instructions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce irrelevant subsystem guidance while keeping equivalent instructions reachable by Claude, Codex, Copilot, and OpenCode.

**Architecture:** Root `AGENTS.md` remains the concise universal entry point and routes work to canonical scoped files. Codex and Copilot discover nested `AGENTS.md`; Claude path rules import the same files; OpenCode follows an explicit root routing instruction and reads the applicable file on demand.

**Tech Stack:** Markdown, existing APM-managed harness configuration, Claude Code rules.

## Global Constraints

- Preserve product scope, issue/PR policy, skills, documentation checks, Electron/renderer boundaries, and metadata-protocol invariants in root `AGENTS.md`.
- Keep one canonical copy of each subsystem rule: `apps/desktop/AGENTS.md` or `crates/orkworksd/AGENTS.md`.
- Do not change `opencode.json`; its `instructions` list is session-wide, not path-conditional.
- Claude rules may import canonical scoped files but must not duplicate their body.
- Do not add dependencies, scripts, CI, or product behavior.

---

### Task 1: Create canonical scoped instructions and route every harness to them

**Files:**
- Create: `apps/desktop/AGENTS.md`
- Create: `crates/orkworksd/AGENTS.md`
- Create: `.claude/rules/desktop-agent-instructions.md`
- Create: `.claude/rules/rust-agent-instructions.md`
- Modify: `AGENTS.md`
- Modify: `docs/agents/harness-instruction-coverage.md`
- Modify: `README.md`
- Test: Markdown/link inspection, `git diff --check`, doc/worktree currency checks

**Interfaces:**
- Consumes: current root instruction sections, `apm.yml` target list, and the coverage record's validated delivery mechanisms.
- Produces: a root routing contract plus two canonical subsystem instruction files.

- [ ] **Step 1: Extract scoped instructions without duplication**

Create `apps/desktop/AGENTS.md` with desktop validation commands, the Electron/renderer import boundary, duplicated IPC-contract ownership, and desktop-specific architecture/documentation references. Keep the repository-wide pnpm-only rule at root because `docs/` is also a Node workspace. Create `crates/orkworksd/AGENTS.md` with Rust-sidecar module-layout guidance, Rust validation commands, and links back to root metadata-protocol constraints and agent docs. Move—not copy—only subsystem-local details out of root `AGENTS.md`.

- [ ] **Step 2: Turn the root file into the router**

Add a short directory map to root `AGENTS.md`. Require agents to read `apps/desktop/AGENTS.md` before changing desktop code, `crates/orkworksd/AGENTS.md` before changing sidecar code, and both for cross-component work. Keep the global constraints named above in root.

- [ ] **Step 3: Add Claude-native path rules**

Create one `.claude/rules/*.md` file per subsystem with only YAML `paths` frontmatter and an `@` import of its canonical scoped `AGENTS.md`. Use `apps/desktop/**` and `crates/orkworksd/**` paths. Do not duplicate rules.

- [ ] **Step 4: Record the production targets**

Update `docs/agents/harness-instruction-coverage.md` with the exact scoped-file paths and observed mechanism for Codex, Copilot, Claude, and OpenCode's root-router fallback. Retain production-shaped, read-only results for both scoped files: the desktop type-check command and the Rust `SessionMetadata` owner. Update `README.md` only with the root-to-scoped instruction map needed to keep the repository entry point current.

- [ ] **Step 5: Verify**

Run:

```bash
rg -n 'apps/desktop/AGENTS.md|crates/orkworksd/AGENTS.md|desktop-agent-instructions|rust-agent-instructions' AGENTS.md README.md .claude/rules docs/agents/harness-instruction-coverage.md
git diff --check
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: the root routes both scoped files; Claude rules import them; coverage records all four harnesses; no whitespace errors or currency-check findings.

- [ ] **Step 6: Commit**

```bash
git add AGENTS.md README.md apps/desktop/AGENTS.md crates/orkworksd/AGENTS.md .claude/rules/desktop-agent-instructions.md .claude/rules/rust-agent-instructions.md docs/agents/harness-instruction-coverage.md docs/superpowers/plans/2026-07-31-cross-harness-scoped-instructions.md
git commit -m "docs: scope agent instructions by subsystem"
```
