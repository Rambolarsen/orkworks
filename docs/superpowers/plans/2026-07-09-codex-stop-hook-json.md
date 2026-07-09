# Codex Stop Hook JSON Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Codex stop hook always emit valid stop-hook JSON while preserving the existing doc-check message content.

**Architecture:** Add a small shell wrapper under `.codex/hooks/` that converts doc-check output into Codex stop-hook JSON, then point `.codex/hooks.json` at that wrapper. Guard it with a focused Node test.

**Tech Stack:** Bash, JSON, Node test runner

## Global Constraints

Keep `.claude/hooks/doc-check.sh` behavior unchanged.
Keep the existing `.codex/hooks/doc-check.sh` diff rules as the source of truth.
Do not expand the fix beyond the Codex stop-hook path.

---

### Task 1: Add failing regression coverage for the wrapper contract

**Files:**
- Create: `apps/desktop/tests/codexStopHook.test.mjs`
- Test: `apps/desktop/tests/codexStopHook.test.mjs`

**Interfaces:**
- Consumes: `.codex/hooks/doc-check-stop.sh`, `.codex/hooks.json`
- Produces: A failing test that expects valid JSON stop-hook output and wrapper-based hook wiring

- [ ] **Step 1: Write the failing test**

```js
test("codex stop hook wrapper emits {} when doc-check is quiet", () => {
  const stdout = execFileSync("bash", [wrapper], {
    cwd: repoRoot,
    env: { ...process.env, ORKWORKS_DOC_CHECK_OUTPUT: "" },
    encoding: "utf8",
  });

  assert.deepEqual(JSON.parse(stdout), {});
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/desktop && node --test tests/codexStopHook.test.mjs`
Expected: FAIL because the wrapper file and hook wiring do not exist yet.

### Task 2: Implement the wrapper and rewire Stop

**Files:**
- Create: `.codex/hooks/doc-check-stop.sh`
- Modify: `.codex/hooks.json`
- Test: `apps/desktop/tests/codexStopHook.test.mjs`

**Interfaces:**
- Consumes: `.codex/hooks/doc-check.sh`
- Produces: Valid JSON stdout for Codex stop hooks

- [ ] **Step 1: Write minimal implementation**

```bash
if [ -z "$output" ]; then
  printf '{}\n'
else
  printf '{\n  "systemMessage": "%s"\n}\n' "$escaped"
fi
```

- [ ] **Step 2: Repoint the Stop hook**

```json
"command": "bash '.codex/hooks/doc-check-stop.sh'"
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cd apps/desktop && node --test tests/codexStopHook.test.mjs`
Expected: PASS

- [ ] **Step 4: Run the repo checks**

Run: `bash .codex/hooks/doc-check.sh`
Expected: exit 0 or a normal doc-check message, with no invalid JSON issue because Codex is no longer consuming this script directly.
