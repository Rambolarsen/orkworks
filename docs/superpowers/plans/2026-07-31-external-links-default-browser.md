# External Links in Default Browser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Open web links from the Electron UI in the operating system's default browser, without creating an app-owned browser window or navigating the app window away.

**Architecture:** Add one Electron-main helper that installs popup and navigation handlers on the main window's `webContents`. It blocks both Electron paths and delegates only valid `http:` and `https:` URLs to Electron's existing `shell.openExternal` API. Keep the renderer, preload, sidecar, and plan-file opening path unchanged.

**Tech Stack:** Electron 39, TypeScript, Node's built-in test runner and assertions.

## Global Constraints

- Use Electron's existing `shell.openExternal`; add no dependency, IPC channel, setting, or renderer API.
- Prevent every popup and same-window navigation request; open only valid `http:` and `https:` URLs externally.
- Swallow an OS handoff rejection after logging it, so user input cannot create an unhandled rejection.
- Keep Electron-main and renderer imports separate; do not alter `rootDir`.
- Add one focused runnable test before production code and observe it fail first.

---

### Task 1: Redirect Electron web links to the OS browser

**Files:**

- Create: `apps/desktop/electron/externalLinks.ts`
- Modify: `apps/desktop/electron/main.ts`
- Test: `apps/desktop/tests/externalLinks.test.ts`

**Interfaces:**

- Consumes: `Electron.WebContents#setWindowOpenHandler`, `Electron.WebContents#on("will-navigate")`, and a callback compatible with `Electron.Shell["openExternal"]`.
- Produces: `configureExternalLinks(webContents, openExternal): void`, which installs both handlers, plus `openWebLink(url, openExternal): void` for validation and delegation.

- [x] **Step 1: Write the failing test**

```ts
import test from "node:test";
import assert from "node:assert/strict";
import { configureExternalLinks } from "../electron/externalLinks.ts";

test("opens web URLs externally and blocks Electron navigation", async () => {
  let popup!: (details: { url: string }) => { action: "deny" };
  let navigate!: (event: { preventDefault(): void }, url: string) => void;
  const opened: string[] = [];
  configureExternalLinks({
    setWindowOpenHandler(next) { popup = next as typeof popup; },
    on(event, next) { assert.equal(event, "will-navigate"); navigate = next as typeof navigate; },
  } as never, async (url) => { opened.push(url); });

  assert.deepEqual(popup({ url: "https://example.test/docs" }), { action: "deny" });
  const prevented: string[] = [];
  navigate({ preventDefault() { prevented.push("yes"); } }, "http://example.test/");
  navigate({ preventDefault() { prevented.push("yes"); } }, "file:///private/secret");
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(prevented, ["yes", "yes"]);
  assert.deepEqual(opened, ["https://example.test/docs", "http://example.test/"]);
});
```

- [x] **Step 2: Run test to verify it fails**

Run: `node --experimental-strip-types --test tests/externalLinks.test.ts`

Expected: FAIL because `../electron/externalLinks.ts` does not exist.

- [x] **Step 3: Write minimal implementation**

```ts
import type { Shell, WebContents } from "electron";

export function openWebLink(url: string, openExternal: Shell["openExternal"]): void {
  try {
    const protocol = new URL(url).protocol;
    if (protocol === "http:" || protocol === "https:") {
      void openExternal(url).catch((error) => console.error("[main] couldn't open external link", error));
    }
  } catch {}
}

export function configureExternalLinks(webContents: WebContents, openExternal: Shell["openExternal"]): void {
  webContents.setWindowOpenHandler(({ url }) => {
    openWebLink(url, openExternal);
    return { action: "deny" };
  });
  webContents.on("will-navigate", (event, url) => {
    event.preventDefault();
    openWebLink(url, openExternal);
  });
}
```

In `apps/desktop/electron/main.ts`, add `import { configureExternalLinks } from "./externalLinks";` and immediately after constructing `mainWindow`, add `configureExternalLinks(mainWindow.webContents, shell.openExternal);`.

- [x] **Step 4: Run the focused test to verify it passes**

Run: `node --experimental-strip-types --test tests/externalLinks.test.ts`

Expected: PASS with one passing test.

- [x] **Step 5: Run desktop verification**

Run: `npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`

Expected: exit code 0 with no TypeScript errors and all desktop tests passing.

- [x] **Step 6: Commit**

Run: `git add apps/desktop/electron/externalLinks.ts apps/desktop/electron/main.ts apps/desktop/tests/externalLinks.test.ts docs/superpowers/plans/2026-07-31-external-links-default-browser.md && git commit -m "fix: open web links in default browser"`
