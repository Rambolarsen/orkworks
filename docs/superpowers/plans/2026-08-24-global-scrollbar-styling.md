# Global Scrollbar Styling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply a subtle dark scrollbar treatment to all native scroll containers in the desktop renderer.

**Architecture:** Add a single global CSS rule block to the existing renderer stylesheet. Use standards-based scrollbar properties with Chromium pseudo-elements for Electron's native scrollbar rendering; preserve native scrolling and all existing overflow behavior.

**Tech Stack:** React renderer, shared CSS, Electron/Chromium, pnpm desktop checks.

## Global Constraints

- Use pnpm for Node.js package-management tasks.
- Do not add a dependency for this visual-only change.
- Preserve the Electron-main/renderer boundary.
- Keep the active terminal single-context; do not introduce parallel terminal surfaces.

---

### Task 1: Add the shared scrollbar theme

**Files:**
- Modify: `apps/desktop/src/App.css` near the global `html, body, #root` rules

**Interfaces:**
- Consumes: Existing dark-theme CSS variables and native overflow containers.
- Produces: Consistent scrollbar appearance across renderer scroll containers.

- [ ] **Step 1: Add standards-based scrollbar colors**

Add `scrollbar-width: thin` and muted `scrollbar-color` to the global `html, body, #root` rule.

- [ ] **Step 2: Add Chromium scrollbar pseudo-elements**

Add global `::-webkit-scrollbar`, `::-webkit-scrollbar-track`, `::-webkit-scrollbar-thumb`, and hover rules with a narrow width, transparent track, rounded thumb, and existing border/surface colors.

- [ ] **Step 3: Verify the source diff**

Run `git diff --check` and inspect that only the intended CSS and design/plan files changed.

- [ ] **Step 4: Run desktop verification**

From `apps/desktop/`, run `npx tsc --noEmit` and `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`. Both commands must exit successfully.

- [ ] **Step 5: Run repository closeout checks**

From the repository root, run `bash .claude/hooks/doc-check.sh` and `bash .claude/hooks/worktree-check.sh`; address any actionable findings before handoff.

- [ ] **Step 6: Commit the scoped change**

Commit only `apps/desktop/src/App.css`, the design spec, and this plan with:

```bash
git add apps/desktop/src/App.css docs/superpowers/specs/2026-08-24-global-scrollbar-styling-design.md docs/superpowers/plans/2026-08-24-global-scrollbar-styling.md
git commit -m "style(desktop): soften global scrollbars"
```
