# Copilot Icon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render the correct GitHub Copilot mark anywhere OrkWorks shows a harness icon instead of the generic terminal fallback.

**Architecture:** Keep icon resolution in `apps/desktop/src/harnessIcons.ts` so every harness-icon surface shares one lookup table. Vendor the official Copilot SVG mark as plain path data, alias all Copilot names the UI can pass into that same entry, and cover the lookup with a focused Node test so both the icon registry and the fallback behavior stay honest.

**Tech Stack:** TypeScript, React, Node built-in test runner.

## Global Constraints

- No changes to session sorting, labels, or provider wiring.
- No visual redesign beyond the Copilot icon itself.

---

### Task 1: Add Copilot icon aliases to the harness icon registry

**Files:**
- Modify: `apps/desktop/src/harnessIcons.ts`

**Interfaces:**
- Consumes: `harnessIconKey(tool: string): string`
- Produces: `HARNESS_ICON_PATHS["gh copilot"]`, `HARNESS_ICON_PATHS["copilot"]`, `HARNESS_ICON_PATHS["github copilot cli"]`, and `HARNESS_ICON_PATHS["github copilot"]` all returning the same Copilot SVG path list.

- [ ] **Step 1: Edit the registry**

Add a `COPILOT` path array to `apps/desktop/src/harnessIcons.ts` using the official Copilot SVG mark, then add these aliases to `HARNESS_ICON_PATHS`: `gh copilot`, `copilot`, `github copilot`, and `github copilot cli`.

- [ ] **Step 2: Keep the key normalizer unchanged**

```ts
export function harnessIconKey(tool: string): string {
  return tool.toLowerCase().replace(/[-_]+/g, " ").trim();
}
```

- [ ] **Step 3: Commit the registry update**

```bash
git add apps/desktop/src/harnessIcons.ts
git commit -m "Add Copilot harness icon aliases"
```

### Task 2: Extend the harness icon regression tests

**Files:**
- Modify: `apps/desktop/tests/harnessIcon.test.ts`

**Interfaces:**
- Consumes: `HARNESS_ICON_PATHS`, `harnessIconKey`
- Produces: coverage that Copilot ids and display names resolve to a vendored icon and that unknown tools still fall back.

- [ ] **Step 1: Add Copilot cases to the builtin coverage test**

```ts
test("every builtin harness display name and id resolves to a vendored mark", () => {
  const builtins: Array<[name: string, id: string]> = [
    ["Claude Code", "claude-code"],
    ["Codex", "codex"],
    ["OpenCode", "opencode"],
    ["Aider", "aider"],
    ["Gemini CLI", "gemini"],
    ["GitHub Copilot CLI", "gh-copilot"],
    ["Copilot", "copilot"],
  ];
  for (const [name, id] of builtins) {
    assert.ok(HARNESS_ICON_PATHS[harnessIconKey(name)], `missing icon path for name ${name}`);
    assert.ok(HARNESS_ICON_PATHS[harnessIconKey(id)], `missing icon path for id ${id}`);
  }
});
```

- [ ] **Step 2: Run the focused test file**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/harnessIcon.test.ts`
Expected: PASS

- [ ] **Step 3: Commit the test update**

```bash
git add apps/desktop/tests/harnessIcon.test.ts
git commit -m "Cover Copilot harness icon lookup"
```
