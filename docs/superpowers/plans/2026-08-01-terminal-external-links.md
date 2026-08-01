# Terminal External Links Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Open HTTP(S) terminal links in the default browser without a renderer popup.

**Architecture:** Both xterm terminal constructors receive an OSC-8 `linkHandler` that invokes a narrow preload IPC method. Electron main validates its untrusted argument with `openWebLink` before calling `shell.openExternal`; the general popup/navigation deny policy remains unchanged.

**Tech Stack:** Electron IPC, xterm.js 6, Node test runner, TypeScript.

## Global Constraints

- Accept only `http:` and `https:` URLs in Electron main.
- Keep Electron navigation and renderer popups denied.
- Add no dependencies and do not change local plan/file opening.

---

### Task 1: Route xterm terminal links through Electron main

**Files:**

- Create: `apps/desktop/src/terminalLinks.ts`
- Modify: `apps/desktop/src/terminalStore.ts`
- Modify: `apps/desktop/src/components/HistoricalTerminal.tsx`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Test: `apps/desktop/tests/terminalLinks.test.ts`

**Interfaces:**

- Consumes: `openWebLink(url, shell.openExternal)` from `electron/externalLinks.ts`.
- Produces: `window.orkworks.openExternalLink(url: string): Promise<void>` and `terminalLinkHandler(openExternal)`.

- [ ] **Step 1: Write the failing test**

```ts
test("forwards an activated terminal link to Electron", async () => {
  const opened: string[] = [];
  terminalLinkHandler(async (url) => { opened.push(url); }).activate({} as MouseEvent, "https://example.test", {} as never);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(opened, ["https://example.test"]);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --experimental-strip-types --test tests/terminalLinks.test.ts`

Expected: FAIL because `terminalLinks.ts` does not exist.

- [ ] **Step 3: Write minimal implementation**

```ts
export function terminalLinkHandler(openExternal: (url: string) => Promise<void>) {
  return { activate(_event: MouseEvent, url: string) { void openExternal(url).catch(console.error); } };
}
```

Use the handler in the live and historical `Terminal` constructors. Expose `openExternalLink` through preload and its renderer declaration. Register `open-external-link` in main, reject non-string values, and pass strings to `openWebLink`.

- [ ] **Step 4: Run focused and complete validation**

Run: `node --experimental-strip-types --test tests/terminalLinks.test.ts tests/externalLinks.test.ts && npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`

Expected: all tests and the type-check pass.

- [ ] **Step 5: Commit**

Stage the task files and commit with `fix: open terminal links externally`.
