# Recent Workspace Switcher (Pin/Remove) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a user list, switch to, pin, and remove workspaces OrkWorks has previously opened on this device, instead of only ever silently reopening the last one.

**Architecture:** Extend the existing installation-scoped workspace-history file (`apps/desktop/electron/workspaceMemory.ts`) from schema v1 to v2 by adding a separate, capped `pinnedWorkspacePaths` list alongside the existing `recentWorkspacePaths`. Expose the history through five new IPC channels that reuse the existing, unmodified `workspaceSwitchCoordinator` for the actual open/switch lifecycle. Surface it in the renderer as one `WorkspaceHistoryList` component, shown inside a small dropdown behind both the titlebar's existing switch (⇄) button and the "Open workspace" button shown when no workspace is open.

**Tech Stack:** Electron main process (Node, TypeScript, `fs-ext` advisory locking), React/TypeScript renderer, `node:test` for both electron- and renderer-side tests.

**Spec:** [`docs/superpowers/specs/2026-09-19-recent-workspace-switcher-design.md`](2026-09-19-recent-workspace-switcher-design.md)

## Global Constraints

- Package manager is pnpm only; run all commands from `apps/desktop/`.
- `recentWorkspacePaths` keeps its existing 20-entry cap; `pinnedWorkspacePaths` gets its own 50-entry cap. Both share the file's existing 64 KiB total serialized-size cap.
- A path lives in at most one of `recentWorkspacePaths` / `pinnedWorkspacePaths` at a time.
- Every mutation continues to use the existing short-lived advisory lock (`acquireHistoryLock`), revision reread/increment, and same-directory atomic replace (`writeAndVerify`) — do not change that mechanism.
- Corrupt/unreadable history is left untouched and surfaced as a diagnostic; it is never silently rewritten (existing rule, unchanged).
- The `electron/` (main process) and `src/` (renderer) TypeScript boundary is a hard boundary: IPC contract types are declared independently on each side (`electron/backendLifecycleEvent.ts` vs `src/orkworksWindow.d.ts`), never imported across it.
- New UI copy goes through the `VOCAB` table in `src/labels.ts`, not inline string literals.
- This feature only changes which workspace *this* OrkWorks instance opens next — no cross-instance dashboard, focus switcher, or auto-resume (per `specs/multi-workspace.md`'s non-goals).
- After every task's code changes, run from `apps/desktop/`: `npx tsc --noEmit` and the specific test file(s) touched by that task. Run the full suite (`node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`) at the end of Task 3 and again in Task 7.

---

### Task 1: ADR for the v1 → v2 workspace-history schema

Per the root `AGENTS.md` rule ("Create an ADR before writing any implementation code for a decision that shapes the architecture, stack, or protocol"), this is the first, blocking task — no schema or IPC code is written before this lands.

**Files:**
- Create: `docs/adr/0061-pinned-workspace-history.md`
- Modify: `docs/adr/README.md`

**Interfaces:**
- Produces: none (documentation only). Later tasks reference this ADR by number in code comments where the v1→v2 migration lives.

- [ ] **Step 1: Write the ADR**

Create `docs/adr/0061-pinned-workspace-history.md`:

```markdown
# Pinned and enumerable installation-scoped workspace history

- Status: accepted
- Deciders: owner
- Date: 2026-09-19

## Context

ADR 0060 introduced installation-scoped workspace history
(`recentWorkspacePaths`, a single recency-ordered, 20-entry-capped list) as a
startup hint for `lastWorkspacePath`. Nothing in the UI reads this list —
there is no way for a user to see, switch to, or deliberately retain a
workspace shortcut without re-browsing the filesystem through the native
folder picker every time.

## Decision

Extend the on-disk workspace-memory schema from v1 to v2 by adding a second,
separate list, `pinnedWorkspacePaths`, alongside the existing
`recentWorkspacePaths`:

```json
{
  "version": 2,
  "revision": 43,
  "lastWorkspacePath": "<canonical path or null>",
  "recentWorkspacePaths": ["<canonical path>"],
  "pinnedWorkspacePaths": ["<canonical path>"]
}
```

- A path lives in at most one of the two lists at a time.
- `recentWorkspacePaths` keeps its existing 20-entry cap; both lists share
  the file's existing 64 KiB total serialized-size cap.
- `pinnedWorkspacePaths` is exempt from recency-based eviction but gets its
  own 50-entry cap, enforced as a distinct `pin_limit_reached` diagnostic
  (a count overflow) alongside the existing byte-size diagnostic that the
  generic write path already enforces for either list.
- `lastWorkspacePath`'s validity invariant relaxes from "must be present in
  `recentWorkspacePaths`" to "must be present in `recentWorkspacePaths` OR
  `pinnedWorkspacePaths`," since opening a pinned workspace updates only
  `lastWorkspacePath`, not either list's membership.
- v1 files (no `pinnedWorkspacePaths` key) migrate at read time to v2 with an
  empty pinned list, following the same read-time migration pattern already
  used for the pre-#569 legacy format (three-tier reader: legacy → v1 → v2).
- The renderer gains a `WorkspaceHistoryList` surface (behind the titlebar
  switcher and the no-workspace state) that reads this history over new IPC
  channels and lets a user open, pin, unpin, or remove an entry.

Full design: [`docs/superpowers/specs/2026-09-19-recent-workspace-switcher-design.md`](../superpowers/specs/2026-09-19-recent-workspace-switcher-design.md).

## Consequences

Users can pin workspaces they return to often and switch to any remembered
workspace directly, instead of only ever seeing a single silent
`lastWorkspacePath` startup hint. The on-disk format gains a version; no
cross-version compatibility window applies beyond the one-directional
read-time migration, since this is a single-writer, installation-local file.
The existing lock/revision/atomic-replace write mechanism, byte-size bound,
and corrupt-file-preserved-on-failure behavior are unchanged and apply
uniformly to both lists.
```

- [ ] **Step 2: Add the ADR to the index table**

In `docs/adr/README.md`, after the `0060` row, add:

```markdown
| [0061](./0061-pinned-workspace-history.md) | Pinned and enumerable installation-scoped workspace history | accepted |
```

- [ ] **Step 3: Commit**

```bash
git add docs/adr/0061-pinned-workspace-history.md docs/adr/README.md
git commit -m "docs: add ADR 0061 for pinned workspace history schema v2"
```

---

### Task 2: Rewrite `workspaceMemory.ts` tests for schema v2 (failing)

Update the existing test file to expect the v2 shape and add new tests for pinning, before touching the implementation. Every test in this task is expected to **fail** after this step (the implementation is still v1) — that is the point.

**Files:**
- Modify: `apps/desktop/tests/electronWorkspaceMemory.test.ts` (full rewrite)

**Interfaces:**
- Consumes: `forgetWorkspacePath`, `canonicalWorkspacePath`, `accessibleWorkspaceDirectoryPath`, `readWorkspaceMemory`, `rememberWorkspacePath`, `workspaceMemoryPath` (existing exports) plus two new exports this task's tests require: `pinWorkspacePath(userDataPath: string, workspacePath: string, replaceFile?: WorkspaceHistoryReplacer): AppWorkspaceMemory` and `unpinWorkspacePath(userDataPath: string, workspacePath: string, replaceFile?: WorkspaceHistoryReplacer): AppWorkspaceMemory`.
- Produces: nothing (test file).

- [ ] **Step 1: Replace the whole test file**

```typescript
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { setTimeout as delay } from "node:timers/promises";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  forgetWorkspacePath,
  canonicalWorkspacePath,
  accessibleWorkspaceDirectoryPath,
  readWorkspaceMemory,
  rememberWorkspacePath,
  pinWorkspacePath,
  unpinWorkspacePath,
  workspaceMemoryPath,
} from "../electron/workspaceMemory.ts";

const EMPTY_MEMORY = {
  version: 2,
  revision: 0,
  lastWorkspacePath: null,
  recentWorkspacePaths: [],
  pinnedWorkspacePaths: [],
  diagnostic: null,
} as const;

function withTemporaryUserData(run: (directory: string) => void | Promise<void>): Promise<void> {
  const directory = mkdtempSync(join(tmpdir(), "orkworks-memory-"));
  return Promise.resolve(run(directory)).finally(() => {
    rmSync(directory, { recursive: true, force: true });
  });
}

test("workspace memory round-trips the version and increasing revision", () =>
  withTemporaryUserData((directory) => {
    assert.deepEqual(readWorkspaceMemory(directory), EMPTY_MEMORY);

    const first = rememberWorkspacePath(directory, "/repo/a");
    const second = rememberWorkspacePath(directory, "/repo/b");

    assert.equal(first.version, 2);
    assert.equal(first.revision, 1);
    assert.equal(second.revision, 2);
    assert.deepEqual(readWorkspaceMemory(directory), second);
    assert.deepEqual(JSON.parse(readFileSync(workspaceMemoryPath(directory), "utf8")), {
      version: 2,
      revision: 2,
      lastWorkspacePath: "/repo/b",
      recentWorkspacePaths: ["/repo/b", "/repo/a"],
      pinnedWorkspacePaths: [],
    });
  }));

test("workspace history replacement seam handles a second write over an existing target", () =>
  withTemporaryUserData((directory) => {
    const replacements: boolean[] = [];
    const replace = (source: string, target: string, targetExists: boolean): void => {
      replacements.push(targetExists);
      writeFileSync(target, readFileSync(source));
      rmSync(source, { force: true });
    };

    const first = rememberWorkspacePath(directory, "/repo/a", replace);
    const second = rememberWorkspacePath(directory, "/repo/b", replace);

    assert.equal(first.diagnostic, null);
    assert.equal(second.diagnostic, null);
    assert.deepEqual(replacements, [false, true]);
    assert.deepEqual(readWorkspaceMemory(directory), second);
  }));

test("Windows workspace history replaces an existing target on the second write", {
  skip: process.platform !== "win32",
}, () => withTemporaryUserData((directory) => {
  rememberWorkspacePath(directory, "/repo/a");
  const second = rememberWorkspacePath(directory, "/repo/b");

  assert.equal(second.diagnostic, null);
  assert.equal(second.revision, 2);
  assert.equal(readWorkspaceMemory(directory).lastWorkspacePath, "/repo/b");
}));

test("workspace memory survives a fresh process using the same application-data directory", () =>
  withTemporaryUserData(async (directory) => {
    rememberWorkspacePath(directory, "/repo/a");
    rememberWorkspacePath(directory, "/repo/b");

    const moduleUrl = new URL("../electron/workspaceMemory.ts", import.meta.url).href;
    const script = `import { readWorkspaceMemory } from ${JSON.stringify(moduleUrl)};
process.stdout.write(JSON.stringify(readWorkspaceMemory(process.argv[1])));`;
    const child = spawn(process.execPath, [
      "--experimental-strip-types",
      "--input-type=module",
      "--eval",
      script,
      directory,
    ]);
    let stdout = "";
    child.stdout.on("data", (chunk: Buffer) => { stdout += chunk.toString(); });
    const [code] = await once(child, "close");

    assert.equal(code, 0);
    assert.deepEqual(JSON.parse(stdout), {
      version: 2,
      revision: 2,
      lastWorkspacePath: "/repo/b",
      recentWorkspacePaths: ["/repo/b", "/repo/a"],
      pinnedWorkspacePaths: [],
      diagnostic: null,
    });
  }));

test("rememberWorkspacePath keeps at most 20 newest paths", () =>
  withTemporaryUserData((directory) => {
    for (let index = 0; index < 25; index += 1) {
      rememberWorkspacePath(directory, `/repo/${index}`);
    }

    const memory = readWorkspaceMemory(directory);
    assert.equal(memory.revision, 25);
    assert.equal(memory.lastWorkspacePath, "/repo/24");
    assert.deepEqual(
      memory.recentWorkspacePaths,
      Array.from({ length: 20 }, (_, index) => `/repo/${24 - index}`),
    );
  }));

test("rememberWorkspacePath evicts oldest entries until serialized history is at most 64 KiB", () =>
  withTemporaryUserData((directory) => {
    for (let index = 0; index < 20; index += 1) {
      rememberWorkspacePath(directory, `/repo/${index}-${"x".repeat(4_000)}`);
    }

    const serialized = readFileSync(workspaceMemoryPath(directory), "utf8");
    const memory = readWorkspaceMemory(directory);
    assert.ok(Buffer.byteLength(serialized, "utf8") <= 64 * 1024);
    assert.ok(memory.recentWorkspacePaths.length < 20);
    assert.equal(memory.recentWorkspacePaths[0], `/repo/19-${"x".repeat(4_000)}`);
  }));

test("rememberWorkspacePath deduplicates canonical paths and keeps the newest first", () =>
  withTemporaryUserData((directory) => {
    rememberWorkspacePath(directory, "/repo/a");
    rememberWorkspacePath(directory, "/repo/b");
    const memory = rememberWorkspacePath(directory, "/repo/a");

    assert.equal(memory.revision, 3);
    assert.equal(memory.lastWorkspacePath, "/repo/a");
    assert.deepEqual(memory.recentWorkspacePaths, ["/repo/a", "/repo/b"]);
  }));

test("forgetWorkspacePath removes only the selected shortcut without deleting its workspace", () =>
  withTemporaryUserData((directory) => {
    const workspace = join(directory, "workspace-a");
    const marker = join(workspace, "keep.txt");
    mkdirSync(workspace);
    writeFileSync(marker, "keep");
    rememberWorkspacePath(directory, workspace);
    rememberWorkspacePath(directory, "/repo/other");
    rememberWorkspacePath(directory, workspace);

    const memory = forgetWorkspacePath(directory, workspace);

    assert.equal(memory.revision, 4);
    assert.equal(memory.lastWorkspacePath, null);
    assert.deepEqual(memory.recentWorkspacePaths, ["/repo/other"]);
    assert.equal(readFileSync(marker, "utf8"), "keep");
    assert.equal(existsSync(workspace), true);
  }));

test("forgetWorkspacePath leaves memory and revision untouched for an unknown path", () =>
  withTemporaryUserData((directory) => {
    const remembered = rememberWorkspacePath(directory, "/repo/a");
    const result = forgetWorkspacePath(directory, "/repo/other");

    assert.deepEqual(result, remembered);
    assert.deepEqual(readWorkspaceMemory(directory), remembered);
  }));

test("forgetWorkspacePath removes a pinned path from pinnedWorkspacePaths", () =>
  withTemporaryUserData((directory) => {
    pinWorkspacePath(directory, "/repo/pinned");
    const memory = forgetWorkspacePath(directory, "/repo/pinned");

    assert.equal(memory.diagnostic, null);
    assert.deepEqual(memory.pinnedWorkspacePaths, []);
    assert.deepEqual(memory.recentWorkspacePaths, []);
    assert.equal(memory.lastWorkspacePath, null);
  }));

test("pinWorkspacePath moves a path out of recentWorkspacePaths and unpinWorkspacePath reinserts it at the front", () =>
  withTemporaryUserData((directory) => {
    rememberWorkspacePath(directory, "/repo/a");
    rememberWorkspacePath(directory, "/repo/b");

    const pinned = pinWorkspacePath(directory, "/repo/a");
    assert.equal(pinned.diagnostic, null);
    assert.deepEqual(pinned.pinnedWorkspacePaths, ["/repo/a"]);
    assert.deepEqual(pinned.recentWorkspacePaths, ["/repo/b"]);
    assert.equal(pinned.lastWorkspacePath, "/repo/a");

    const unpinned = unpinWorkspacePath(directory, "/repo/a");
    assert.equal(unpinned.diagnostic, null);
    assert.deepEqual(unpinned.pinnedWorkspacePaths, []);
    assert.deepEqual(unpinned.recentWorkspacePaths, ["/repo/a", "/repo/b"]);
  }));

test("pinWorkspacePath is a no-op when the path is already pinned", () =>
  withTemporaryUserData((directory) => {
    const first = pinWorkspacePath(directory, "/repo/a");
    const second = pinWorkspacePath(directory, "/repo/a");
    assert.deepEqual(second, first);
  }));

test("unpinWorkspacePath is a no-op for a path that is not pinned", () =>
  withTemporaryUserData((directory) => {
    const remembered = rememberWorkspacePath(directory, "/repo/a");
    const result = unpinWorkspacePath(directory, "/repo/a");
    assert.deepEqual(result, remembered);
  }));

test("pinWorkspacePath rejects a 51st pin with a distinct diagnostic instead of evicting", () =>
  withTemporaryUserData((directory) => {
    for (let index = 0; index < 50; index += 1) {
      const result = pinWorkspacePath(directory, `/repo/${index}`);
      assert.equal(result.diagnostic, null);
    }
    const overflow = pinWorkspacePath(directory, "/repo/overflow");
    assert.equal(overflow.diagnostic?.code, "pin_limit_reached");
    assert.equal(overflow.pinnedWorkspacePaths.length, 50);
    assert.equal(overflow.pinnedWorkspacePaths.includes("/repo/overflow"), false);
  }));

test("rememberWorkspacePath updates lastWorkspacePath without duplicating an already-pinned path into recentWorkspacePaths", () =>
  withTemporaryUserData((directory) => {
    rememberWorkspacePath(directory, "/repo/other");
    const pinned = pinWorkspacePath(directory, "/repo/pinned");
    assert.deepEqual(pinned.pinnedWorkspacePaths, ["/repo/pinned"]);
    assert.deepEqual(pinned.recentWorkspacePaths, ["/repo/other"]);

    const remembered = rememberWorkspacePath(directory, "/repo/pinned");

    assert.equal(remembered.diagnostic, null);
    assert.equal(remembered.lastWorkspacePath, "/repo/pinned");
    assert.deepEqual(remembered.pinnedWorkspacePaths, ["/repo/pinned"]);
    assert.deepEqual(remembered.recentWorkspacePaths, ["/repo/other"]);
    assert.equal(remembered.revision, pinned.revision + 1);
  }));

test("a v2 file whose lastWorkspacePath is pinned-only (not in recentWorkspacePaths) reloads without a corrupt diagnostic", () =>
  withTemporaryUserData((directory) => {
    pinWorkspacePath(directory, "/repo/pinned");
    const remembered = rememberWorkspacePath(directory, "/repo/pinned");
    assert.equal(remembered.diagnostic, null);
    assert.equal(remembered.lastWorkspacePath, "/repo/pinned");

    const reloaded = readWorkspaceMemory(directory);
    assert.equal(reloaded.diagnostic, null);
    assert.equal(reloaded.lastWorkspacePath, "/repo/pinned");
    assert.deepEqual(reloaded.pinnedWorkspacePaths, ["/repo/pinned"]);
    assert.deepEqual(reloaded.recentWorkspacePaths, []);
  }));

test("legacy workspace history without version/revision fields is migrated instead of diagnosed as corrupt", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const legacy = {
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a", "/repo/b"],
    };
    writeFileSync(historyPath, JSON.stringify(legacy, null, 2));

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic, null);
    assert.equal(loaded.version, 2);
    assert.equal(loaded.lastWorkspacePath, "/repo/a");
    // Secondary entries are dropped during migration (see the dedicated
    // "drops secondary entries" test below for why).
    assert.deepEqual(loaded.recentWorkspacePaths, ["/repo/a"]);
    assert.deepEqual(loaded.pinnedWorkspacePaths, []);

    const remembered = rememberWorkspacePath(directory, "/repo/c");

    assert.equal(remembered.diagnostic, null);
    assert.equal(remembered.lastWorkspacePath, "/repo/c");
    assert.deepEqual(remembered.recentWorkspacePaths, ["/repo/c", "/repo/a"]);
    assert.deepEqual(JSON.parse(readFileSync(historyPath, "utf8")), {
      version: 2,
      revision: remembered.revision,
      lastWorkspacePath: "/repo/c",
      recentWorkspacePaths: ["/repo/c", "/repo/a"],
      pinnedWorkspacePaths: [],
    });
  }));

test("legacy workspace history with a cleared lastWorkspacePath still migrates", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const legacy = {
      lastWorkspacePath: null,
      recentWorkspacePaths: ["/repo/a", "/repo/b"],
    };
    writeFileSync(historyPath, JSON.stringify(legacy, null, 2));

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic, null);
    assert.equal(loaded.lastWorkspacePath, null);
    assert.deepEqual(loaded.recentWorkspacePaths, []);
    assert.deepEqual(loaded.pinnedWorkspacePaths, []);
  }));

test("legacy workspace history canonicalizes lastWorkspacePath and drops secondary entries", () =>
  withTemporaryUserData((directory) => {
    const realPath = join(directory, "real-workspace");
    const aliasPath = join(directory, "alias-workspace");
    mkdirSync(realPath);
    symlinkSync(realPath, aliasPath, "dir");
    const expectedPath = canonicalWorkspacePath(realPath);

    const historyPath = workspaceMemoryPath(directory);
    const legacy = {
      lastWorkspacePath: aliasPath,
      recentWorkspacePaths: [aliasPath, "/repo/other", "/repo/missing"],
    };
    writeFileSync(historyPath, JSON.stringify(legacy, null, 2));

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic, null);
    assert.equal(loaded.lastWorkspacePath, expectedPath);
    assert.deepEqual(loaded.recentWorkspacePaths, [expectedPath]);
  }));

test("legacy workspace history over the old 64 KiB gate but under the legacy ceiling still migrates", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const paths = Array.from({ length: 5 }, (_, index) => `/repo/${index}-${"x".repeat(15_000)}`);
    const legacy = { lastWorkspacePath: paths[0], recentWorkspacePaths: paths };
    const raw = JSON.stringify(legacy);
    assert.ok(Buffer.byteLength(raw, "utf8") > 64 * 1024);
    assert.ok(Buffer.byteLength(raw, "utf8") <= 2 * 1024 * 1024);
    writeFileSync(historyPath, raw);

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic, null);
    assert.equal(loaded.lastWorkspacePath, paths[0]);
  }));

test("legacy workspace history with more entries than the old writer's 10-entry cap is diagnosed as corrupt", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const legacy = {
      lastWorkspacePath: "/repo/0",
      recentWorkspacePaths: Array.from({ length: 11 }, (_, index) => `/repo/${index}`),
    };
    const raw = JSON.stringify(legacy);
    writeFileSync(historyPath, raw);

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic?.code, "corrupt_history");
    assert.equal(readFileSync(historyPath, "utf8"), raw);
  }));

test("legacy workspace history whose migrated record cannot fit is diagnosed, not silently truncated", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const oversizedPath = `/repo/${"x".repeat(50_000)}`;
    const legacy = {
      lastWorkspacePath: oversizedPath,
      recentWorkspacePaths: [] as string[],
    };
    const raw = JSON.stringify(legacy);
    assert.ok(Buffer.byteLength(raw, "utf8") <= 64 * 1024);
    writeFileSync(historyPath, raw);

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic?.code, "corrupt_history");
    assert.equal(loaded.lastWorkspacePath, null);
    assert.deepEqual(loaded.recentWorkspacePaths, []);
    assert.equal(readFileSync(historyPath, "utf8"), raw);
  }));

test("a v1 file with no pinnedWorkspacePaths key migrates with an empty pinned list", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const v1 = {
      version: 1,
      revision: 5,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a", "/repo/b"],
    };
    writeFileSync(historyPath, JSON.stringify(v1));

    const loaded = readWorkspaceMemory(directory);

    assert.equal(loaded.diagnostic, null);
    assert.equal(loaded.version, 2);
    assert.equal(loaded.revision, 5);
    assert.equal(loaded.lastWorkspacePath, "/repo/a");
    assert.deepEqual(loaded.recentWorkspacePaths, ["/repo/a", "/repo/b"]);
    assert.deepEqual(loaded.pinnedWorkspacePaths, []);

    const pinned = pinWorkspacePath(directory, "/repo/a");
    assert.equal(pinned.diagnostic, null);
    assert.deepEqual(JSON.parse(readFileSync(historyPath, "utf8")), {
      version: 2,
      revision: 6,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/b"],
      pinnedWorkspacePaths: ["/repo/a"],
    });
  }));

test("corrupt workspace history is diagnosed and preserved across mutation attempts", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const corrupt = "{ definitely not valid workspace history";
    writeFileSync(historyPath, corrupt);

    const loaded = readWorkspaceMemory(directory);
    const remembered = rememberWorkspacePath(directory, "/repo/a");
    const forgotten = forgetWorkspacePath(directory, "/repo/a");
    const pinned = pinWorkspacePath(directory, "/repo/a");
    const unpinned = unpinWorkspacePath(directory, "/repo/a");

    assert.deepEqual(loaded, {
      ...EMPTY_MEMORY,
      diagnostic: {
        code: "corrupt_history",
        message: "Workspace history is corrupt or unreadable and was left unchanged.",
      },
    });
    assert.deepEqual(remembered, loaded);
    assert.deepEqual(forgotten, loaded);
    assert.deepEqual(pinned, loaded);
    assert.deepEqual(unpinned, loaded);
    assert.equal(readFileSync(historyPath, "utf8"), corrupt);
  }));

test("malformed UTF-8 workspace history is diagnosed and preserved byte-for-byte", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const corrupt = Buffer.concat([
      Buffer.from('{"version":1,"revision":1,"lastWorkspacePath":"/repo/'),
      Buffer.from([0x80]),
      Buffer.from('","recentWorkspacePaths":["/repo/'),
      Buffer.from([0x80]),
      Buffer.from('"]}\n'),
    ]);
    writeFileSync(historyPath, corrupt);

    const memory = readWorkspaceMemory(directory);

    assert.equal(memory.diagnostic?.code, "corrupt_history");
    assert.equal(memory.revision, 0);
    assert.deepEqual(readFileSync(historyPath), corrupt);
  }));

test("workspace history rejects unknown persisted fields without overwriting the source", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const corrupt = JSON.stringify({
      version: 1,
      revision: 0,
      lastWorkspacePath: null,
      recentWorkspacePaths: [],
      unexpected: true,
    });
    writeFileSync(historyPath, corrupt);

    const memory = readWorkspaceMemory(directory);

    assert.equal(memory.diagnostic?.code, "corrupt_history");
    assert.equal(readFileSync(historyPath, "utf8"), corrupt);
  }));

test("workspace history rejects a valid-looking file whose serialized bytes exceed the bound", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const record = `${JSON.stringify({
      version: 2,
      revision: 1,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a"],
      pinnedWorkspacePaths: [],
    })}${" ".repeat(64 * 1024)}\n`;
    assert.ok(Buffer.byteLength(record, "utf8") > 64 * 1024);
    writeFileSync(historyPath, record);

    const memory = readWorkspaceMemory(directory);

    assert.equal(memory.diagnostic?.code, "corrupt_history");
    assert.equal(memory.revision, 0);
    assert.equal(readFileSync(historyPath, "utf8"), record);
  }));

test("workspace history rejects oversized and duplicate records without normalizing them", () =>
  withTemporaryUserData((directory) => {
    const historyPath = workspaceMemoryPath(directory);
    const cases = [
      {
        version: 2,
        revision: 2,
        lastWorkspacePath: "/repo/a",
        recentWorkspacePaths: Array.from({ length: 21 }, (_, index) => `/repo/${index}`),
        pinnedWorkspacePaths: [],
      },
      {
        version: 2,
        revision: 3,
        lastWorkspacePath: "/repo/a",
        recentWorkspacePaths: ["/repo/a", "/repo/a"],
        pinnedWorkspacePaths: [],
      },
      {
        version: 2,
        revision: 4,
        lastWorkspacePath: "/repo/a",
        recentWorkspacePaths: ["/repo/a"],
        pinnedWorkspacePaths: ["/repo/a"],
      },
    ];

    for (const record of cases) {
      const source = `${JSON.stringify(record)}\n`;
      writeFileSync(historyPath, source);
      const memory = readWorkspaceMemory(directory);
      assert.equal(memory.diagnostic?.code, "corrupt_history");
      assert.equal(memory.revision, 0);
      assert.equal(readFileSync(historyPath, "utf8"), source);
    }
  }));

test("startup workspace validation accepts only accessible directories", () =>
  withTemporaryUserData((directory) => {
    const workspace = join(directory, "workspace");
    const regularFile = join(directory, "workspace.txt");
    mkdirSync(workspace);
    writeFileSync(regularFile, "not a workspace");

    assert.equal(accessibleWorkspaceDirectoryPath(workspace), realpathSync(workspace));
    assert.equal(accessibleWorkspaceDirectoryPath(regularFile), null);
    assert.equal(statSync(regularFile).isDirectory(), false);
  }));

test("a live OS lock is never evicted, even after five seconds; process exit releases it", () =>
  withTemporaryUserData(async (directory) => {
    const lockPath = join(directory, ".workspace-memory.lock");
    const child = spawn(process.execPath, ["--input-type=commonjs", "--eval", `
      const fs = require('node:fs');
      const { flockSync } = require('fs-ext');
      const fd = fs.openSync(process.argv[1], 'a+', 0o600);
      flockSync(fd, 'exnb');
      process.send('locked');
      setInterval(() => {}, 1000);
    `, lockPath], { stdio: ["ignore", "pipe", "pipe", "ipc"] });
    const exited = once(child, "exit");
    try {
      const [message] = await once(child, "message", { signal: AbortSignal.timeout(10_000) });
      assert.equal(message, "locked");
      const fresh = rememberWorkspacePath(directory, "/repo/not-written");
      assert.equal(fresh.diagnostic?.code, "history_lock_timeout");
      await delay(5_100);
      const old = rememberWorkspacePath(directory, "/repo/still-not-written");
      assert.equal(old.diagnostic?.code, "history_lock_timeout");
      assert.equal(existsSync(workspaceMemoryPath(directory)), false);
    } finally {
      child.kill("SIGKILL");
      await exited;
    }
    assert.equal(existsSync(lockPath), true, "the lock inode must be retained");
    const recovered = rememberWorkspacePath(directory, "/repo/recovered");
    assert.equal(recovered.diagnostic, null);
    assert.equal(recovered.revision, 1);
  }));

test("revision overflow rejects mutations before writing and preserves history bytes", () =>
  withTemporaryUserData((directory) => {
    const original = JSON.stringify({
      version: 1,
      revision: Number.MAX_SAFE_INTEGER,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a"],
    });
    writeFileSync(workspaceMemoryPath(directory), original);
    // pinWorkspacePath("/repo/a") is a genuine change here (the migrated v1
    // file has an empty pinnedWorkspacePaths, so pinning is not a no-op) and
    // so reaches the revision-exhausted check like remember/forget do.
    // unpinWorkspacePath is deliberately NOT included in this loop: since
    // "/repo/a" is never pinned in this v1-migrated fixture, unpinning it
    // would return the noChange sentinel and return early *before* the
    // revision check runs at all — see the dedicated pinned-fixture test
    // below for unpin's revision-exhaustion behavior instead.
    for (const mutate of [rememberWorkspacePath, forgetWorkspacePath, pinWorkspacePath]) {
      const result = mutate(directory, "/repo/a");
      assert.equal(result.diagnostic?.code, "history_write_failed");
      assert.equal(result.revision, Number.MAX_SAFE_INTEGER);
      assert.equal(readFileSync(workspaceMemoryPath(directory), "utf8"), original);
      assert.equal(readWorkspaceMemory(directory).diagnostic, null);
    }
    assert.equal(forgetWorkspacePath(directory, "/unknown").diagnostic, null);
  }));

test("revision overflow rejects unpinWorkspacePath before writing and preserves history bytes", () =>
  withTemporaryUserData((directory) => {
    // unpinWorkspacePath only reaches the revision-exhausted check when
    // unpinning is a genuine change, which requires the path to already be
    // pinned — not expressible in a v1 fixture (v1 has no pinned list), so
    // this uses a v2 fixture directly instead of migrating one.
    const original = JSON.stringify({
      version: 2,
      revision: Number.MAX_SAFE_INTEGER,
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: [],
      pinnedWorkspacePaths: ["/repo/a"],
    });
    writeFileSync(workspaceMemoryPath(directory), original);

    const result = unpinWorkspacePath(directory, "/repo/a");

    assert.equal(result.diagnostic?.code, "history_write_failed");
    assert.equal(result.revision, Number.MAX_SAFE_INTEGER);
    assert.equal(readFileSync(workspaceMemoryPath(directory), "utf8"), original);
    assert.equal(readWorkspaceMemory(directory).diagnostic, null);
  }));

test("workspace history canonicalizes an alias before it is remembered", () =>
  withTemporaryUserData((directory) => {
    const realPath = join(directory, "real-workspace");
    const aliasPath = join(directory, "alias-workspace");
    mkdirSync(realPath);
    symlinkSync(realPath, aliasPath, "dir");

    const canonicalPath = canonicalWorkspacePath(aliasPath);
    const expectedPath = canonicalWorkspacePath(realPath);
    assert.equal(canonicalPath, expectedPath);
    const memory = rememberWorkspacePath(directory, canonicalPath!);

    assert.deepEqual(memory.recentWorkspacePaths, [expectedPath]);
    assert.doesNotMatch(readFileSync(workspaceMemoryPath(directory), "utf8"), /alias-workspace/);
  }));

test("workspace history evicts using the actual next revision size", () =>
  withTemporaryUserData((directory) => {
    const revision = 999_999_999;
    const prefix = "/repo/";
    const baseRecord = (padding: number, candidateRevision: number) => ({
      version: 1,
      revision: candidateRevision,
      lastWorkspacePath: `${prefix}${"x".repeat(padding)}`,
      recentWorkspacePaths: [
        `${prefix}${"x".repeat(padding)}`,
        "/repo/oldest",
      ],
    });
    let padding = 0;
    while (
      Buffer.byteLength(`${JSON.stringify(baseRecord(padding, revision), null, 2)}\n`, "utf8") <= 64 * 1024
      && Buffer.byteLength(`${JSON.stringify(baseRecord(padding, revision + 1), null, 2)}\n`, "utf8") <= 64 * 1024
    ) {
      padding += 1;
    }
    while (Buffer.byteLength(`${JSON.stringify(baseRecord(padding, revision), null, 2)}\n`, "utf8") > 64 * 1024) {
      padding -= 1;
    }
    writeFileSync(workspaceMemoryPath(directory), `${JSON.stringify(baseRecord(padding, revision), null, 2)}\n`);

    const memory = rememberWorkspacePath(directory, "/repo/new");

    assert.equal(memory.diagnostic, null);
    assert.equal(memory.revision, revision + 1);
    assert.equal(memory.recentWorkspacePaths[0], "/repo/new");
    assert.ok(Buffer.byteLength(readFileSync(workspaceMemoryPath(directory), "utf8"), "utf8") <= 64 * 1024);
  }));

test("a single path that cannot fit returns a diagnostic without a silent no-op", () =>
  withTemporaryUserData((directory) => {
    const memory = rememberWorkspacePath(directory, `/repo/${"x".repeat(70_000)}`);

    assert.equal(memory.diagnostic?.code, "history_write_failed");
    assert.equal(existsSync(workspaceMemoryPath(directory)), false);
  }));

test("two process writers merge under the history lock without losing revisions", () =>
  withTemporaryUserData(async (directory) => {
    const moduleUrl = new URL("../electron/workspaceMemory.ts", import.meta.url).href;
    const script = `import { rememberWorkspacePath } from ${JSON.stringify(moduleUrl)};
const directory = process.argv[1];
const prefix = process.argv[2];
const startAt = Number(process.argv[3]);
while (Date.now() < startAt) {}
for (let index = 0; index < 40; index += 1) {
  rememberWorkspacePath(directory, \`/repo/\${prefix}-\${index}\`);
}`;
    const startAt = Date.now() + 250;
    const children = ["a", "b"].map((prefix) => spawn(process.execPath, [
      "--experimental-strip-types",
      "--input-type=module",
      "--eval",
      script,
      directory,
      prefix,
      String(startAt),
    ]));
    const codes = await Promise.all(children.map(async (child) => {
      const [code] = await once(child, "close");
      return code;
    }));

    assert.deepEqual(codes, [0, 0]);
    const memory = readWorkspaceMemory(directory);
    assert.equal(memory.revision, 80);
    assert.equal(memory.recentWorkspacePaths.length, 20);
    assert.equal(new Set(memory.recentWorkspacePaths).size, 20);
    assert.equal(memory.diagnostic, null);
  }));

test("two process writers pinning under the history lock without losing revisions", () =>
  withTemporaryUserData(async (directory) => {
    const moduleUrl = new URL("../electron/workspaceMemory.ts", import.meta.url).href;
    const script = `import { pinWorkspacePath } from ${JSON.stringify(moduleUrl)};
const directory = process.argv[1];
const prefix = process.argv[2];
const startAt = Number(process.argv[3]);
while (Date.now() < startAt) {}
for (let index = 0; index < 10; index += 1) {
  pinWorkspacePath(directory, \`/repo/\${prefix}-\${index}\`);
}`;
    const startAt = Date.now() + 250;
    const children = ["a", "b"].map((prefix) => spawn(process.execPath, [
      "--experimental-strip-types",
      "--input-type=module",
      "--eval",
      script,
      directory,
      prefix,
      String(startAt),
    ]));
    const codes = await Promise.all(children.map(async (child) => {
      const [code] = await once(child, "close");
      return code;
    }));

    assert.deepEqual(codes, [0, 0]);
    const memory = readWorkspaceMemory(directory);
    assert.equal(memory.revision, 20);
    assert.equal(memory.pinnedWorkspacePaths.length, 20);
    assert.equal(new Set(memory.pinnedWorkspacePaths).size, 20);
    assert.equal(memory.diagnostic, null);
  }));
```

- [ ] **Step 2: Run the suite and confirm it fails for the expected reason**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/electronWorkspaceMemory.test.ts`
Expected: many failures, all traceable to either a missing export (`pinWorkspacePath`/`unpinWorkspacePath` don't exist yet) or a shape mismatch (`version: 1` vs expected `version: 2`, missing `pinnedWorkspacePaths`). No failure should be an unrelated syntax error — if one is, fix the test file before proceeding.

- [ ] **Step 3: Commit**

```bash
git add tests/electronWorkspaceMemory.test.ts
git commit -m "test: rewrite workspace-memory tests for schema v2 (failing)"
```

---

### Task 3: Implement schema v2 in `workspaceMemory.ts` (make Task 2 pass)

**Files:**
- Modify: `apps/desktop/electron/workspaceMemory.ts` (full rewrite)
- Test: `apps/desktop/tests/electronWorkspaceMemory.test.ts` (from Task 2, unchanged in this task)

**Interfaces:**
- Consumes: nothing new (Node builtins, `fs-ext`, already-imported utilities).
- Produces (exported, consumed by Task 4's IPC handlers):
  - `workspaceMemoryPath(userDataPath: string): string` (unchanged)
  - `readWorkspaceMemory(userDataPath: string): AppWorkspaceMemory` (unchanged signature, new return shape)
  - `canonicalWorkspacePath(workspacePath: string): string | null` (unchanged)
  - `accessibleWorkspaceDirectoryPath(workspacePath: string): string | null` (unchanged)
  - `rememberWorkspacePath(userDataPath: string, workspacePath: string, replaceFile?: WorkspaceHistoryReplacer): AppWorkspaceMemory` (unchanged signature, now pin-aware)
  - `forgetWorkspacePath(userDataPath: string, workspacePath: string, replaceFile?: WorkspaceHistoryReplacer): AppWorkspaceMemory` (unchanged signature, now removes from either list)
  - `pinWorkspacePath(userDataPath: string, workspacePath: string, replaceFile?: WorkspaceHistoryReplacer): AppWorkspaceMemory` (new)
  - `unpinWorkspacePath(userDataPath: string, workspacePath: string, replaceFile?: WorkspaceHistoryReplacer): AppWorkspaceMemory` (new)
  - `interface AppWorkspaceMemory { version: 2; revision: number; lastWorkspacePath: string | null; recentWorkspacePaths: string[]; pinnedWorkspacePaths: string[]; diagnostic: WorkspaceMemoryDiagnostic | null }`
  - `interface WorkspaceMemoryDiagnostic { code: "corrupt_history" | "history_lock_timeout" | "history_write_failed" | "pin_limit_reached"; message: string }`

- [ ] **Step 1: Replace the whole implementation file**

```typescript
import { randomBytes } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  closeSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
  writeSync,
} from "node:fs";
import { join } from "node:path";
import { TextDecoder } from "node:util";
import fsExt from "fs-ext";

export interface WorkspaceMemoryDiagnostic {
  code: "corrupt_history" | "history_lock_timeout" | "history_write_failed" | "pin_limit_reached";
  message: string;
}

export interface AppWorkspaceMemory {
  version: 2;
  revision: number;
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
  pinnedWorkspacePaths: string[];
  diagnostic: WorkspaceMemoryDiagnostic | null;
}

interface StoredWorkspaceMemory {
  version: 2;
  revision: number;
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
  pinnedWorkspacePaths: string[];
}

// The pre-this-feature (ADR 0060) on-disk shape. Read-only: never written by
// this module, only migrated forward. See ADR 0061.
interface V1StoredWorkspaceMemory {
  version: 1;
  revision: number;
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
}

const fileName = "workspace-memory.json";
const lockFileName = ".workspace-memory.lock";
const maximumRecentPaths = 20;
const maximumPinnedPaths = 50;
const maximumSerializedBytes = 64 * 1024;
// The pre-#569 writer's actual cap (see validLegacyStoredMemory) — never 20.
const maximumLegacyRecentPaths = 10;
// Pre-#569 files predate any byte bound: the old writer capped at 10 entries
// with no size limit, so long (e.g. Windows extended-length) paths could
// legitimately exceed maximumSerializedBytes. Gate parsing at a sanity
// ceiling derived from the old writer's actual worst case instead of
// rejecting such files before they're even inspected: up to
// maximumLegacyRecentPaths entries plus one duplicate of lastWorkspacePath,
// each up to the Windows extended-length maximum of 32,767 UTF-16 code
// units (~98,301 bytes worst-case UTF-8) — roughly 1.05 MiB including JSON
// overhead. 2 MiB leaves comfortable headroom. migratedLegacyMemory still
// trims the result to fit maximumSerializedBytes via boundedPaths.
const maximumLegacySourceBytes = 2 * 1024 * 1024;
const lockRetryCount = 50;
const lockRetryDelayMs = 10;
const utf8Decoder = new TextDecoder("utf-8", { fatal: true });

export type WorkspaceHistoryReplacer = (
  temporary: string,
  target: string,
  targetExists: boolean,
) => void;

const corruptDiagnostic: WorkspaceMemoryDiagnostic = {
  code: "corrupt_history",
  message: "Workspace history is corrupt or unreadable and was left unchanged.",
};

const emptyMemory = (): AppWorkspaceMemory => ({
  version: 2,
  revision: 0,
  lastWorkspacePath: null,
  recentWorkspacePaths: [],
  pinnedWorkspacePaths: [],
  diagnostic: null,
});

function withDiagnostic(
  memory: AppWorkspaceMemory,
  diagnostic: WorkspaceMemoryDiagnostic,
): AppWorkspaceMemory {
  return { ...memory, diagnostic };
}

function storedMemory(memory: AppWorkspaceMemory): StoredWorkspaceMemory {
  return {
    version: 2,
    revision: memory.revision,
    lastWorkspacePath: memory.lastWorkspacePath,
    recentWorkspacePaths: memory.recentWorkspacePaths,
    pinnedWorkspacePaths: memory.pinnedWorkspacePaths,
  };
}

function serializedMemory(memory: StoredWorkspaceMemory): string {
  return `${JSON.stringify(memory, null, 2)}\n`;
}

function memoryFits(memory: StoredWorkspaceMemory): boolean {
  return Buffer.byteLength(serializedMemory(memory), "utf8") <= maximumSerializedBytes;
}

// Builds the recentWorkspacePaths list for a write: puts `lastWorkspacePath`
// first (unless it is pinned — a pinned path never lives in both lists),
// dedupes against `paths`, bounds to maximumRecentPaths entries, then trims
// further until the candidate record (including the caller's current
// pinnedWorkspacePaths, which count toward the same byte budget) fits.
function boundedPaths(
  lastWorkspacePath: string | null,
  paths: readonly string[],
  pinnedWorkspacePaths: readonly string[],
  revision: number,
): string[] | null {
  const deduplicated = [
    ...(lastWorkspacePath === null || pinnedWorkspacePaths.includes(lastWorkspacePath) ? [] : [lastWorkspacePath]),
    ...paths,
  ].filter((value, index, values) => values.indexOf(value) === index);
  const bounded = deduplicated.slice(0, maximumRecentPaths);
  const candidate = (recentWorkspacePaths: string[]): StoredWorkspaceMemory => ({
    version: 2,
    revision,
    lastWorkspacePath,
    recentWorkspacePaths,
    pinnedWorkspacePaths: [...pinnedWorkspacePaths],
  });

  while (bounded.length > 1 && !memoryFits(candidate(bounded))) {
    bounded.pop();
  }
  return memoryFits(candidate(bounded)) ? bounded : null;
}

interface LegacyStoredWorkspaceMemory {
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
}

// Pre-#569 files predate the version/revision fields and the invariants
// validV1StoredMemory enforces (deduplication, lastWorkspacePath inclusion).
// Accept the looser shape the old writer actually produced and normalize it
// through boundedPaths rather than rejecting installs' existing history as
// corrupt.
function validLegacyStoredMemory(value: unknown): value is LegacyStoredWorkspaceMemory {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const raw = value as Record<string, unknown>;
  const keys = Object.keys(raw).sort();
  if (JSON.stringify(keys) !== JSON.stringify([
    "lastWorkspacePath",
    "recentWorkspacePaths",
  ])) return false;
  return (raw.lastWorkspacePath === null || typeof raw.lastWorkspacePath === "string")
    && Array.isArray(raw.recentWorkspacePaths)
    && raw.recentWorkspacePaths.length <= maximumLegacyRecentPaths
    && raw.recentWorkspacePaths.every((entry) => typeof entry === "string");
}

// The pre-#569 writer never canonicalized paths (it stored the raw dialog
// selection), while specs/multi-workspace.md requires persisting canonical
// paths only. realpathSync.native is also a synchronous, potentially slow
// syscall (a disconnected UNC share or other unreachable network mount can
// block for the OS network timeout), and readWorkspaceMemory runs on
// Electron's main thread before the window is created — resolving every
// recentWorkspacePaths entry there risks the app appearing hung on launch.
//
// Rather than choosing between an unbounded synchronous cost (canonicalize
// everything) and a spec-violating one (persist raw aliases), secondary
// entries are dropped instead of carried forward: they're low-stakes
// convenience shortcuts (removing one never touches project files or
// session data) that naturally repopulate, correctly canonicalized, as the
// user reopens workspaces going forward. Only lastWorkspacePath — the one
// entry guaranteed to matter immediately, since it's what gets
// auto-restored at startup — is resolved, bounding the worst-case startup
// stall to a single call.
function canonicalizedLegacyPath(path: string): string {
  return canonicalWorkspacePath(path) ?? path;
}

function migratedLegacyMemory(legacy: LegacyStoredWorkspaceMemory): AppWorkspaceMemory {
  const lastWorkspacePath = legacy.lastWorkspacePath === null
    ? null
    : canonicalizedLegacyPath(legacy.lastWorkspacePath);
  // boundedPaths prepends lastWorkspacePath itself, so an empty paths list
  // is enough to produce the single-entry (or empty) result. No pinned
  // paths exist yet at this migration tier.
  const bounded = boundedPaths(lastWorkspacePath, [], [], 0);
  // boundedPaths only returns null when even a single entry can't fit under
  // the serialized-size bound. Silently dropping recentWorkspacePaths would
  // produce a lastWorkspacePath not present in either list, violating the
  // same invariant validStoredMemory enforces for the current format —
  // surface a diagnostic instead, matching how every other "doesn't fit"
  // case here fails loud rather than discarding data quietly.
  if (bounded === null) return withDiagnostic(emptyMemory(), corruptDiagnostic);
  return {
    version: 2,
    revision: 0,
    lastWorkspacePath,
    recentWorkspacePaths: bounded,
    pinnedWorkspacePaths: [],
    diagnostic: null,
  };
}

// ADR 0060's original shape, unchanged: version 1, no pinnedWorkspacePaths,
// lastWorkspacePath must appear in recentWorkspacePaths. Never written by
// this module; only migrated forward to v2. See ADR 0061.
function validV1StoredMemory(value: unknown): value is V1StoredWorkspaceMemory {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const raw = value as Record<string, unknown>;
  const keys = Object.keys(raw).sort();
  if (JSON.stringify(keys) !== JSON.stringify([
    "lastWorkspacePath",
    "recentWorkspacePaths",
    "revision",
    "version",
  ])) return false;
  return raw.version === 1
    && typeof raw.revision === "number"
    && Number.isSafeInteger(raw.revision)
    && raw.revision >= 0
    && (raw.lastWorkspacePath === null || typeof raw.lastWorkspacePath === "string")
    && Array.isArray(raw.recentWorkspacePaths)
    && raw.recentWorkspacePaths.length <= maximumRecentPaths
    && raw.recentWorkspacePaths.every((entry) => typeof entry === "string")
    && new Set(raw.recentWorkspacePaths).size === raw.recentWorkspacePaths.length
    && (raw.lastWorkspacePath === null || raw.recentWorkspacePaths.includes(raw.lastWorkspacePath));
}

function migratedV1Memory(v1: V1StoredWorkspaceMemory): AppWorkspaceMemory {
  return {
    version: 2,
    revision: v1.revision,
    lastWorkspacePath: v1.lastWorkspacePath,
    recentWorkspacePaths: [...v1.recentWorkspacePaths],
    pinnedWorkspacePaths: [],
    diagnostic: null,
  };
}

function validStoredMemory(value: unknown): value is StoredWorkspaceMemory {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const raw = value as Record<string, unknown>;
  const keys = Object.keys(raw).sort();
  if (JSON.stringify(keys) !== JSON.stringify([
    "lastWorkspacePath",
    "pinnedWorkspacePaths",
    "recentWorkspacePaths",
    "revision",
    "version",
  ])) return false;
  if (
    raw.version !== 2
    || typeof raw.revision !== "number"
    || !Number.isSafeInteger(raw.revision)
    || raw.revision < 0
    || (raw.lastWorkspacePath !== null && typeof raw.lastWorkspacePath !== "string")
    || !Array.isArray(raw.recentWorkspacePaths)
    || !Array.isArray(raw.pinnedWorkspacePaths)
    || raw.recentWorkspacePaths.length > maximumRecentPaths
    || raw.pinnedWorkspacePaths.length > maximumPinnedPaths
    || !raw.recentWorkspacePaths.every((entry) => typeof entry === "string")
    || !raw.pinnedWorkspacePaths.every((entry) => typeof entry === "string")
  ) return false;

  const recentSet = new Set(raw.recentWorkspacePaths);
  const pinnedSet = new Set(raw.pinnedWorkspacePaths);
  if (recentSet.size !== raw.recentWorkspacePaths.length) return false;
  if (pinnedSet.size !== raw.pinnedWorkspacePaths.length) return false;
  for (const path of raw.recentWorkspacePaths) {
    if (pinnedSet.has(path)) return false; // a path must live in at most one list
  }
  if (raw.lastWorkspacePath === null) return true;
  return recentSet.has(raw.lastWorkspacePath) || pinnedSet.has(raw.lastWorkspacePath);
}

function readStoredWorkspaceMemory(userDataPath: string): AppWorkspaceMemory {
  const target = workspaceMemoryPath(userDataPath);
  if (!existsSync(target)) return emptyMemory();

  try {
    const source = readFileSync(target);
    if (source.byteLength > maximumLegacySourceBytes) return withDiagnostic(emptyMemory(), corruptDiagnostic);
    const parsed: unknown = JSON.parse(utf8Decoder.decode(source));
    if (validStoredMemory(parsed)) {
      // The v2 writer never produces a file over maximumSerializedBytes, so
      // enforce that tighter bound here now that the shape is confirmed
      // current-format rather than legacy/v1.
      if (source.byteLength > maximumSerializedBytes || !memoryFits(parsed)) {
        return withDiagnostic(emptyMemory(), corruptDiagnostic);
      }
      return {
        version: 2,
        revision: parsed.revision,
        lastWorkspacePath: parsed.lastWorkspacePath,
        recentWorkspacePaths: [...parsed.recentWorkspacePaths],
        pinnedWorkspacePaths: [...parsed.pinnedWorkspacePaths],
        diagnostic: null,
      };
    }
    if (validV1StoredMemory(parsed)) return migratedV1Memory(parsed);
    if (validLegacyStoredMemory(parsed)) return migratedLegacyMemory(parsed);
    return withDiagnostic(emptyMemory(), corruptDiagnostic);
  } catch {
    return withDiagnostic(emptyMemory(), corruptDiagnostic);
  }
}

function sleepForLockRetry(): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, lockRetryDelayMs);
}

function acquireHistoryLock(userDataPath: string): number | null {
  let descriptor: number;
  try {
    descriptor = openSync(join(userDataPath, lockFileName), "a+", 0o600);
  } catch {
    return null;
  }
  // Keep this inode permanently. Unlinking or renaming it could let another
  // process lock a different inode and enter the critical section concurrently.
  // flock/LockFileEx releases ownership on close or process exit, never by age.
  for (let attempt = 0; attempt < lockRetryCount; attempt += 1) {
    try {
      fsExt.flockSync(descriptor, "exnb");
      return descriptor;
    } catch (error) {
      const code = (error as NodeJS.ErrnoException).code;
      if (code !== "EAGAIN" && code !== "EWOULDBLOCK" && code !== "EINTR") break;
      sleepForLockRetry();
    }
  }
  closeSync(descriptor);
  return null;
}

function replaceExistingWorkspaceHistoryOnWindows(temporary: string, target: string): void {
  // PowerShell's [System.IO.File]::Replace delegates to ReplaceFileW, which
  // atomically replaces an existing file on Windows. Passing paths as
  // environment values keeps arbitrary workspace paths out of the command
  // parser and preserves the error if the native operation fails.
  execFileSync("powershell.exe", [
    "-NoLogo",
    "-NoProfile",
    "-NonInteractive",
    "-Command",
    "$ErrorActionPreference = 'Stop'; [System.IO.File]::Replace($env:ORKWORKS_HISTORY_TEMPORARY, $env:ORKWORKS_HISTORY_TARGET, $null, $true)",
  ], {
    env: {
      ...process.env,
      ORKWORKS_HISTORY_TEMPORARY: temporary,
      ORKWORKS_HISTORY_TARGET: target,
    },
    stdio: "ignore",
  });
}

const replaceWorkspaceHistoryFile: WorkspaceHistoryReplacer = (temporary, target, targetExists) => {
  if (process.platform === "win32" && targetExists) {
    replaceExistingWorkspaceHistoryOnWindows(temporary, target);
    return;
  }
  // POSIX rename and the new-target Windows path both publish the fully
  // flushed temporary file in one filesystem operation.
  renameSync(temporary, target);
};

function writeAndVerify(
  userDataPath: string,
  current: AppWorkspaceMemory,
  next: StoredWorkspaceMemory,
  replaceFile: WorkspaceHistoryReplacer,
): AppWorkspaceMemory {
  const target = workspaceMemoryPath(userDataPath);
  const temporary = join(
    userDataPath,
    `.workspace-memory.${process.pid}.${randomBytes(8).toString("hex")}.tmp`,
  );
  let descriptor: number | null = null;
  let replacementCompleted = false;
  try {
    descriptor = openSync(temporary, "wx", 0o600);
    const bytes = Buffer.from(serializedMemory(next), "utf8");
    let offset = 0;
    while (offset < bytes.byteLength) {
      const written = writeSync(descriptor, bytes, offset, bytes.byteLength - offset, null);
      if (written <= 0) throw new Error("Workspace history write made no progress.");
      offset += written;
    }
    fsyncSync(descriptor);
    closeSync(descriptor);
    descriptor = null;
    replaceFile(temporary, target, existsSync(target));
    replacementCompleted = true;
  } catch {
    return withDiagnostic(current, {
      code: "history_write_failed",
      message: "Workspace history could not be saved; the ready workspace was kept.",
    });
  } finally {
    if (descriptor !== null) closeSync(descriptor);
    rmSync(temporary, { force: true });
  }

  if (replacementCompleted) {
    const observed = readStoredWorkspaceMemory(userDataPath);
    if (
      observed.diagnostic === null
      && JSON.stringify(storedMemory(observed)) === JSON.stringify(next)
    ) return observed;
  }
  return withDiagnostic(current, {
    code: "history_write_failed",
    message: "Workspace history could not be confirmed after replacement.",
  });
}

const noChange = Symbol("no workspace history change");
const tooLarge = Symbol("workspace history entry too large");
const pinLimitReached = Symbol("workspace history pin limit reached");
type UpdateResult = StoredWorkspaceMemory | typeof noChange | typeof tooLarge | typeof pinLimitReached;

function updateWorkspaceMemory(
  userDataPath: string,
  update: (current: AppWorkspaceMemory) => UpdateResult,
  replaceFile: WorkspaceHistoryReplacer = replaceWorkspaceHistoryFile,
): AppWorkspaceMemory {
  try {
    mkdirSync(userDataPath, { recursive: true });
  } catch {
    return withDiagnostic(emptyMemory(), {
      code: "history_write_failed",
      message: "Workspace history could not be saved; the ready workspace was kept.",
    });
  }

  const lock = acquireHistoryLock(userDataPath);
  if (lock === null) {
    return withDiagnostic(readStoredWorkspaceMemory(userDataPath), {
      code: "history_lock_timeout",
      message: "Workspace history was busy and could not be updated.",
    });
  }

  try {
    const current = readStoredWorkspaceMemory(userDataPath);
    if (current.diagnostic !== null) return current;
    const next = update(current);
    if (next === noChange) return current;
    if (current.revision === Number.MAX_SAFE_INTEGER) {
      return withDiagnostic(current, {
        code: "history_write_failed",
        message: "Workspace history revision is exhausted; the file was left unchanged.",
      });
    }
    if (next === pinLimitReached) {
      return withDiagnostic(current, {
        code: "pin_limit_reached",
        message: `Only ${maximumPinnedPaths} workspaces can be pinned at a time.`,
      });
    }
    if (next === tooLarge) {
      return withDiagnostic(current, {
        code: "history_write_failed",
        message: "Workspace path is too large to fit in the bounded history file.",
      });
    }
    if (!memoryFits(next)) {
      return withDiagnostic(current, {
        code: "history_write_failed",
        message: "Workspace path is too large to fit in the bounded history file.",
      });
    }
    return writeAndVerify(userDataPath, current, next, replaceFile);
  } finally {
    closeSync(lock);
  }
}

export function workspaceMemoryPath(userDataPath: string): string {
  return join(userDataPath, fileName);
}

export function readWorkspaceMemory(userDataPath: string): AppWorkspaceMemory {
  return readStoredWorkspaceMemory(userDataPath);
}

export function canonicalWorkspacePath(workspacePath: string): string | null {
  try {
    return realpathSync.native(workspacePath);
  } catch {
    return null;
  }
}

export function accessibleWorkspaceDirectoryPath(workspacePath: string): string | null {
  try {
    if (!statSync(workspacePath).isDirectory()) return null;
    return realpathSync.native(workspacePath);
  } catch {
    return null;
  }
}

export function rememberWorkspacePath(
  userDataPath: string,
  workspacePath: string,
  replaceFile: WorkspaceHistoryReplacer = replaceWorkspaceHistoryFile,
): AppWorkspaceMemory {
  return updateWorkspaceMemory(userDataPath, (current) => {
    if (current.pinnedWorkspacePaths.includes(workspacePath)) {
      // Opening an already-pinned workspace only updates lastWorkspacePath —
      // it must never be duplicated into recentWorkspacePaths (a path lives
      // in at most one list; see ADR 0061).
      if (current.lastWorkspacePath === workspacePath) return noChange;
      return {
        version: 2,
        revision: current.revision + 1,
        lastWorkspacePath: workspacePath,
        recentWorkspacePaths: current.recentWorkspacePaths,
        pinnedWorkspacePaths: current.pinnedWorkspacePaths,
      };
    }
    const recentWorkspacePaths = boundedPaths(workspacePath, [
      workspacePath,
      ...current.recentWorkspacePaths.filter((path) => path !== workspacePath),
    ], current.pinnedWorkspacePaths, current.revision + 1);
    if (recentWorkspacePaths === null) return tooLarge;
    return {
      version: 2,
      revision: current.revision + 1,
      lastWorkspacePath: workspacePath,
      recentWorkspacePaths,
      pinnedWorkspacePaths: current.pinnedWorkspacePaths,
    };
  }, replaceFile);
}

export function forgetWorkspacePath(
  userDataPath: string,
  workspacePath: string,
  replaceFile: WorkspaceHistoryReplacer = replaceWorkspaceHistoryFile,
): AppWorkspaceMemory {
  return updateWorkspaceMemory(userDataPath, (current) => {
    const recentWorkspacePaths = current.recentWorkspacePaths.filter((path) => path !== workspacePath);
    const pinnedWorkspacePaths = current.pinnedWorkspacePaths.filter((path) => path !== workspacePath);
    const changed = recentWorkspacePaths.length !== current.recentWorkspacePaths.length
      || pinnedWorkspacePaths.length !== current.pinnedWorkspacePaths.length
      || current.lastWorkspacePath === workspacePath;
    if (!changed) return noChange;
    return {
      version: 2,
      revision: current.revision + 1,
      lastWorkspacePath: current.lastWorkspacePath === workspacePath ? null : current.lastWorkspacePath,
      recentWorkspacePaths,
      pinnedWorkspacePaths,
    };
  }, replaceFile);
}

export function pinWorkspacePath(
  userDataPath: string,
  workspacePath: string,
  replaceFile: WorkspaceHistoryReplacer = replaceWorkspaceHistoryFile,
): AppWorkspaceMemory {
  return updateWorkspaceMemory(userDataPath, (current) => {
    if (current.pinnedWorkspacePaths.includes(workspacePath)) return noChange;
    // Count overflow is distinct from, and checked before, the generic
    // byte-size check below: 50 short paths fit easily in 64 KiB, so the
    // byte check alone would never catch a 51st pin. See ADR 0061.
    if (current.pinnedWorkspacePaths.length >= maximumPinnedPaths) return pinLimitReached;
    const pinnedWorkspacePaths = [workspacePath, ...current.pinnedWorkspacePaths];
    const recentWorkspacePaths = current.recentWorkspacePaths.filter((path) => path !== workspacePath);
    return {
      version: 2,
      revision: current.revision + 1,
      lastWorkspacePath: current.lastWorkspacePath,
      recentWorkspacePaths,
      pinnedWorkspacePaths,
    };
  }, replaceFile);
}

export function unpinWorkspacePath(
  userDataPath: string,
  workspacePath: string,
  replaceFile: WorkspaceHistoryReplacer = replaceWorkspaceHistoryFile,
): AppWorkspaceMemory {
  return updateWorkspaceMemory(userDataPath, (current) => {
    if (!current.pinnedWorkspacePaths.includes(workspacePath)) return noChange;
    const pinnedWorkspacePaths = current.pinnedWorkspacePaths.filter((path) => path !== workspacePath);
    // Re-enters recentWorkspacePaths at the front, as if just opened, rather
    // than being lost.
    const recentWorkspacePaths = boundedPaths(
      workspacePath,
      current.recentWorkspacePaths,
      pinnedWorkspacePaths,
      current.revision + 1,
    );
    if (recentWorkspacePaths === null) return tooLarge;
    return {
      version: 2,
      revision: current.revision + 1,
      lastWorkspacePath: current.lastWorkspacePath,
      recentWorkspacePaths,
      pinnedWorkspacePaths,
    };
  }, replaceFile);
}
```

- [ ] **Step 2: Run the target test file and confirm it passes**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/electronWorkspaceMemory.test.ts`
Expected: all tests pass (one Windows-only test skipped on non-Windows).

- [ ] **Step 3: Type-check and run the full desktop suite**

Run: `cd apps/desktop && npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: no type errors; all tests pass. (`electronSidecarWiring.test.ts` and other files are untouched by this task and must still pass unchanged.)

- [ ] **Step 4: Commit**

```bash
git add electron/workspaceMemory.ts
git commit -m "feat: extend workspace history to schema v2 with pin/unpin support"
```

---

### Task 4: Wire the new IPC channels end-to-end

Adds the five new `ipcMain`/`preload`/renderer-type channels. `main.ts`'s `switchWorkspace(path)` and `getCurrentWorkspacePath()` on `workspaceSwitchCoordinator` already exist and need no changes — `open-remembered-workspace` calls them directly.

**Files:**
- Modify: `apps/desktop/electron/backendLifecycleEvent.ts:37-40, 87-97` (widen `WorkspaceHistoryDiagnostic.code` and its runtime validator)
- Modify: `apps/desktop/electron/main.ts:12` (import), `apps/desktop/electron/main.ts:524-528` (add snapshot helper), `apps/desktop/electron/main.ts:1812-1824` (add five handlers after `open-workspace`)
- Modify: `apps/desktop/electron/preload.ts:110-111` (add five mappings)
- Modify: `apps/desktop/src/orkworksWindow.d.ts:56-59, 140` (widen diagnostic union, add snapshot type, add five window methods)
- Test: `apps/desktop/tests/electronSidecarWiring.test.ts` (append one test)

**Interfaces:**
- Consumes: `pinWorkspacePath`, `unpinWorkspacePath`, `forgetWorkspacePath`, `rememberWorkspacePath`, `readWorkspaceMemory` (from Task 3); `workspaceSwitchCoordinator.getCurrentWorkspacePath(): string | null` and `.switchWorkspace(path: string): Promise<WorkspaceSwitchResult<...>>` (already exist, unmodified).
- Produces (consumed by Task 6/7's renderer code):
  - `window.orkworks.getWorkspaceHistory(): Promise<WorkspaceHistorySnapshot>`
  - `window.orkworks.pinWorkspacePath(path: string): Promise<WorkspaceHistorySnapshot>`
  - `window.orkworks.unpinWorkspacePath(path: string): Promise<WorkspaceHistorySnapshot>`
  - `window.orkworks.forgetWorkspacePath(path: string): Promise<WorkspaceHistorySnapshot>`
  - `window.orkworks.openRememberedWorkspace(path: string): Promise<WorkspaceInfo | null>`
  - `type WorkspaceHistorySnapshot = { pinned: string[]; recent: string[]; diagnostic: WorkspaceHistoryDiagnostic | null }`

- [ ] **Step 1: Widen the electron-side diagnostic type and its validator**

In `apps/desktop/electron/backendLifecycleEvent.ts`, change:

```typescript
export interface WorkspaceHistoryDiagnostic {
  code: "corrupt_history" | "history_lock_timeout" | "history_write_failed";
  message: string;
}
```

to:

```typescript
export interface WorkspaceHistoryDiagnostic {
  code: "corrupt_history" | "history_lock_timeout" | "history_write_failed" | "pin_limit_reached";
  message: string;
}
```

And change:

```typescript
function canonicalizeHistoryDiagnostic(value: unknown): WorkspaceHistoryDiagnostic | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  if (!hasExactKeys(value, ["code", "message"])) return null;
  const diagnostic = value as Record<string, unknown>;
  return (diagnostic.code === "corrupt_history"
    || diagnostic.code === "history_lock_timeout"
    || diagnostic.code === "history_write_failed")
    && typeof diagnostic.message === "string"
    ? { code: diagnostic.code, message: diagnostic.message }
    : null;
}
```

to:

```typescript
function canonicalizeHistoryDiagnostic(value: unknown): WorkspaceHistoryDiagnostic | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  if (!hasExactKeys(value, ["code", "message"])) return null;
  const diagnostic = value as Record<string, unknown>;
  return (diagnostic.code === "corrupt_history"
    || diagnostic.code === "history_lock_timeout"
    || diagnostic.code === "history_write_failed"
    || diagnostic.code === "pin_limit_reached")
    && typeof diagnostic.message === "string"
    ? { code: diagnostic.code, message: diagnostic.message }
    : null;
}
```

- [ ] **Step 2: Add the snapshot helper and handlers in `main.ts`**

Change the import at line 12 from:

```typescript
import { accessibleWorkspaceDirectoryPath, canonicalWorkspacePath, readWorkspaceMemory, rememberWorkspacePath, forgetWorkspacePath, type WorkspaceMemoryDiagnostic } from "./workspaceMemory";
```

to:

```typescript
import { accessibleWorkspaceDirectoryPath, canonicalWorkspacePath, readWorkspaceMemory, rememberWorkspacePath, forgetWorkspacePath, pinWorkspacePath, unpinWorkspacePath, type WorkspaceMemoryDiagnostic } from "./workspaceMemory";
```

Immediately after `toWorkspaceHistoryDiagnostic` (currently lines 524-528), add:

```typescript
interface WorkspaceHistorySnapshot {
  pinned: string[];
  recent: string[];
  diagnostic: WorkspaceHistoryDiagnostic | null;
}

function toWorkspaceHistorySnapshot(memory: ReturnType<typeof readWorkspaceMemory>): WorkspaceHistorySnapshot {
  return {
    pinned: memory.pinnedWorkspacePaths,
    recent: memory.recentWorkspacePaths,
    diagnostic: toWorkspaceHistoryDiagnostic(memory.diagnostic),
  };
}
```

Immediately after the existing `open-workspace` handler (currently lines 1812-1824), add:

```typescript
  ipcMain.handle("get-workspace-history", () =>
    toWorkspaceHistorySnapshot(readWorkspaceMemory(app.getPath("userData"))));

  ipcMain.handle("pin-workspace-path", (_event, path: unknown) => {
    if (typeof path !== "string") throw new Error("Invalid workspace path");
    return toWorkspaceHistorySnapshot(pinWorkspacePath(app.getPath("userData"), path));
  });

  ipcMain.handle("unpin-workspace-path", (_event, path: unknown) => {
    if (typeof path !== "string") throw new Error("Invalid workspace path");
    return toWorkspaceHistorySnapshot(unpinWorkspacePath(app.getPath("userData"), path));
  });

  ipcMain.handle("forget-workspace-path", (_event, path: unknown) => {
    if (typeof path !== "string") throw new Error("Invalid workspace path");
    return toWorkspaceHistorySnapshot(forgetWorkspacePath(app.getPath("userData"), path));
  });

  ipcMain.handle("open-remembered-workspace", async (_event, path: unknown) => {
    if (!workspaceSwitchCoordinator) throw new Error("Workspace lifecycle is unavailable");
    if (typeof path !== "string") return null;
    if (workspaceSwitchCoordinator.getCurrentWorkspacePath() === path) return null;
    const result = await workspaceSwitchCoordinator.switchWorkspace(path);
    return result.ok ? result.workspace : null;
  });
```

- [ ] **Step 3: Mirror the channels in `preload.ts`**

Immediately after line 111 (`openWorkspace: (): Promise<unknown> => ipcRenderer.invoke("open-workspace"),`), add:

```typescript
  getWorkspaceHistory: (): Promise<unknown> => ipcRenderer.invoke("get-workspace-history"),
  pinWorkspacePath: (path: string): Promise<unknown> => ipcRenderer.invoke("pin-workspace-path", path),
  unpinWorkspacePath: (path: string): Promise<unknown> => ipcRenderer.invoke("unpin-workspace-path", path),
  forgetWorkspacePath: (path: string): Promise<unknown> => ipcRenderer.invoke("forget-workspace-path", path),
  openRememberedWorkspace: (path: string): Promise<unknown> => ipcRenderer.invoke("open-remembered-workspace", path),
```

- [ ] **Step 4: Mirror the contract in `src/orkworksWindow.d.ts`**

Change:

```typescript
export type WorkspaceHistoryDiagnostic = {
  code: "corrupt_history" | "history_lock_timeout" | "history_write_failed";
  message: string;
};
```

to:

```typescript
export type WorkspaceHistoryDiagnostic = {
  code: "corrupt_history" | "history_lock_timeout" | "history_write_failed" | "pin_limit_reached";
  message: string;
};

export type WorkspaceHistorySnapshot = {
  pinned: string[];
  recent: string[];
  diagnostic: WorkspaceHistoryDiagnostic | null;
};
```

Immediately after line 140 (`openWorkspace: () => Promise<WorkspaceInfo | null>;`), add:

```typescript
      getWorkspaceHistory: () => Promise<WorkspaceHistorySnapshot>;
      pinWorkspacePath: (path: string) => Promise<WorkspaceHistorySnapshot>;
      unpinWorkspacePath: (path: string) => Promise<WorkspaceHistorySnapshot>;
      forgetWorkspacePath: (path: string) => Promise<WorkspaceHistorySnapshot>;
      openRememberedWorkspace: (path: string) => Promise<WorkspaceInfo | null>;
```

- [ ] **Step 5: Write the wiring test**

Append to `apps/desktop/tests/electronSidecarWiring.test.ts` (this file already defines `mainSource`, `preloadSource`, and `rendererTypes` at the top — reuse them):

```typescript
test("workspace history IPC channels are wired through main, preload, and the renderer contract", () => {
  assert.match(mainSource, /ipcMain\.handle\("get-workspace-history", \(\) =>/);
  assert.match(mainSource, /ipcMain\.handle\("pin-workspace-path", \(_event, path: unknown\) => \{/);
  assert.match(mainSource, /ipcMain\.handle\("unpin-workspace-path", \(_event, path: unknown\) => \{/);
  assert.match(mainSource, /ipcMain\.handle\("forget-workspace-path", \(_event, path: unknown\) => \{/);
  assert.match(mainSource, /ipcMain\.handle\("open-remembered-workspace", async \(_event, path: unknown\) => \{/);
  assert.match(mainSource, /workspaceSwitchCoordinator\.getCurrentWorkspacePath\(\) === path\) return null;/);
  assert.match(mainSource, /await workspaceSwitchCoordinator\.switchWorkspace\(path\);/);

  assert.match(preloadSource, /getWorkspaceHistory: \(\): Promise<unknown> => ipcRenderer\.invoke\("get-workspace-history"\)/);
  assert.match(preloadSource, /pinWorkspacePath: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("pin-workspace-path", path\)/);
  assert.match(preloadSource, /unpinWorkspacePath: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("unpin-workspace-path", path\)/);
  assert.match(preloadSource, /forgetWorkspacePath: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("forget-workspace-path", path\)/);
  assert.match(preloadSource, /openRememberedWorkspace: \(path: string\): Promise<unknown> => ipcRenderer\.invoke\("open-remembered-workspace", path\)/);

  assert.match(rendererTypes, /getWorkspaceHistory: \(\) => Promise<WorkspaceHistorySnapshot>;/);
  assert.match(rendererTypes, /openRememberedWorkspace: \(path: string\) => Promise<WorkspaceInfo \| null>;/);
  assert.match(rendererTypes, /"pin_limit_reached"/);
});
```

- [ ] **Step 6: Run the test and type-check**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/electronSidecarWiring.test.ts && npx tsc --noEmit`
Expected: the new test passes; no type errors (this exercises the widened `WorkspaceHistoryDiagnostic` union compiling through `toWorkspaceHistoryDiagnostic`'s existing body unchanged).

- [ ] **Step 7: Commit**

```bash
git add electron/backendLifecycleEvent.ts electron/main.ts electron/preload.ts src/orkworksWindow.d.ts tests/electronSidecarWiring.test.ts
git commit -m "feat: add workspace history IPC channels (get/pin/unpin/forget/open)"
```

---

### Task 5: `WorkspaceHistoryList` and `WorkspaceHistoryDropdown` components

Builds the reusable list (rows with open/pin/unpin/remove) and a small dropdown wrapper that manages open/close, outside-click, and Escape dismissal. No existing dropdown/popover pattern exists in `apps/desktop/src/` to reuse (confirmed by repo search during design review), so this task builds a minimal one.

**Files:**
- Modify: `apps/desktop/src/labels.ts` (add VOCAB entries)
- Create: `apps/desktop/src/components/WorkspaceHistoryList.tsx`
- Create: `apps/desktop/src/components/WorkspaceHistoryDropdown.tsx`
- Modify: `apps/desktop/src/App.css` (add styles)
- Test: `apps/desktop/tests/workspaceHistoryComponents.test.ts` (new)

**Interfaces:**
- Consumes: `window.orkworks.getWorkspaceHistory/pinWorkspacePath/unpinWorkspacePath/forgetWorkspacePath` (from Task 4); `VOCAB` (extended below).
- Produces (consumed by Task 6):
  - `interface WorkspaceHistoryListProps { currentWorkspacePath: string | null; isSwitching: boolean; onOpenPath: (path: string) => void; onOpenOtherFolder: () => void; onError: (message: string) => void }`
  - `export function WorkspaceHistoryList(props: WorkspaceHistoryListProps): JSX.Element | null`
  - `interface WorkspaceHistoryDropdownProps { currentWorkspacePath: string | null; isSwitching: boolean; onOpenPath: (path: string) => void; onOpenOtherFolder: () => void; onError: (message: string) => void; triggerLabel: string; triggerClassName: string; triggerAriaLabel: string }`
  - `export function WorkspaceHistoryDropdown(props: WorkspaceHistoryDropdownProps): JSX.Element`

- [ ] **Step 1: Add VOCAB entries**

In `apps/desktop/src/labels.ts`, inside the existing `VOCAB` object (after `switchWorkspace: "Switch workspace",`), add:

```typescript
  workspaceHistoryPinnedSection: "Pinned",
  workspaceHistoryRecentSection: "Recent",
  workspaceHistoryEmpty: "No remembered workspaces yet.",
  workspaceHistoryOpenOtherFolder: "Open other folder…",
  workspaceHistoryCurrent: "Current",
  pinWorkspace: "Pin workspace",
  unpinWorkspace: "Unpin workspace",
  removeWorkspace: "Remove from history",
```

- [ ] **Step 2: Create `WorkspaceHistoryList.tsx`**

```tsx
import { useCallback, useEffect, useState } from "react";
import { VOCAB } from "../labels";
import type { WorkspaceHistorySnapshot } from "../orkworksWindow";

export interface WorkspaceHistoryListProps {
  currentWorkspacePath: string | null;
  isSwitching: boolean;
  onOpenPath: (path: string) => void;
  onOpenOtherFolder: () => void;
  onError: (message: string) => void;
}

function workspaceLabel(path: string): string {
  return path.split("/").pop() || path;
}

export function WorkspaceHistoryList({
  currentWorkspacePath,
  isSwitching,
  onOpenPath,
  onOpenOtherFolder,
  onError,
}: WorkspaceHistoryListProps) {
  const [snapshot, setSnapshot] = useState<WorkspaceHistorySnapshot | null>(null);

  useEffect(() => {
    let cancelled = false;
    window.orkworks.getWorkspaceHistory()
      .then((next) => { if (!cancelled) setSnapshot(next); })
      .catch(() => { if (!cancelled) onError("Couldn't load workspace history."); });
    return () => { cancelled = true; };
  }, [onError]);

  const handlePin = useCallback((path: string) => {
    window.orkworks.pinWorkspacePath(path)
      .then(setSnapshot)
      .catch(() => onError("Couldn't pin workspace."));
  }, [onError]);

  const handleUnpin = useCallback((path: string) => {
    window.orkworks.unpinWorkspacePath(path)
      .then(setSnapshot)
      .catch(() => onError("Couldn't unpin workspace."));
  }, [onError]);

  const handleForget = useCallback((path: string) => {
    window.orkworks.forgetWorkspacePath(path)
      .then(setSnapshot)
      .catch(() => onError("Couldn't remove workspace from history."));
  }, [onError]);

  if (!snapshot) return null;

  const renderRow = (path: string, pinned: boolean) => {
    const isCurrent = path === currentWorkspacePath;
    return (
      <li key={path} className="workspace-history-row">
        <button
          type="button"
          className="workspace-history-row-open"
          title={path}
          disabled={isCurrent || isSwitching}
          onClick={() => onOpenPath(path)}
        >
          {workspaceLabel(path)}
          {isCurrent && (
            <span className="workspace-history-current-badge">{VOCAB.workspaceHistoryCurrent}</span>
          )}
        </button>
        <button
          type="button"
          className="workspace-history-row-action"
          aria-label={pinned ? VOCAB.unpinWorkspace : VOCAB.pinWorkspace}
          title={pinned ? VOCAB.unpinWorkspace : VOCAB.pinWorkspace}
          onClick={() => (pinned ? handleUnpin(path) : handlePin(path))}
        >
          {pinned ? "📌" : "📍"}
        </button>
        <button
          type="button"
          className="workspace-history-row-action"
          aria-label={VOCAB.removeWorkspace}
          title={VOCAB.removeWorkspace}
          onClick={() => handleForget(path)}
        >
          ✕
        </button>
      </li>
    );
  };

  const isEmpty = snapshot.pinned.length === 0 && snapshot.recent.length === 0;

  return (
    <div className="workspace-history-list">
      {snapshot.diagnostic && (
        <p role="alert" className="workspace-history-diagnostic">{snapshot.diagnostic.message}</p>
      )}
      {snapshot.pinned.length > 0 && (
        <>
          <p className="workspace-history-section-label">{VOCAB.workspaceHistoryPinnedSection}</p>
          <ul className="workspace-history-rows">{snapshot.pinned.map((path) => renderRow(path, true))}</ul>
        </>
      )}
      {snapshot.recent.length > 0 && (
        <>
          <p className="workspace-history-section-label">{VOCAB.workspaceHistoryRecentSection}</p>
          <ul className="workspace-history-rows">{snapshot.recent.map((path) => renderRow(path, false))}</ul>
        </>
      )}
      {isEmpty && !snapshot.diagnostic && (
        <p className="workspace-history-empty">{VOCAB.workspaceHistoryEmpty}</p>
      )}
      <button type="button" className="workspace-history-other-folder" onClick={onOpenOtherFolder}>
        {VOCAB.workspaceHistoryOpenOtherFolder}
      </button>
    </div>
  );
}
```

- [ ] **Step 3: Create `WorkspaceHistoryDropdown.tsx`**

```tsx
import { useCallback, useEffect, useRef, useState } from "react";
import { WorkspaceHistoryList, type WorkspaceHistoryListProps } from "./WorkspaceHistoryList";

export interface WorkspaceHistoryDropdownProps extends WorkspaceHistoryListProps {
  triggerLabel: string;
  triggerClassName: string;
  triggerAriaLabel: string;
}

export function WorkspaceHistoryDropdown({
  triggerLabel,
  triggerClassName,
  triggerAriaLabel,
  onOpenPath,
  onOpenOtherFolder,
  ...listProps
}: WorkspaceHistoryDropdownProps) {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const close = useCallback(() => setOpen(false), []);

  useEffect(() => {
    if (!open) return;
    function handlePointerDown(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) close();
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") close();
    }
    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open, close]);

  return (
    <div className="workspace-history-dropdown" ref={containerRef}>
      <button
        type="button"
        className={triggerClassName}
        aria-label={triggerAriaLabel}
        title={triggerAriaLabel}
        onClick={() => setOpen((value) => !value)}
      >
        {triggerLabel}
      </button>
      {open && (
        <div className="workspace-history-panel">
          <WorkspaceHistoryList
            {...listProps}
            onOpenPath={(path) => { close(); onOpenPath(path); }}
            onOpenOtherFolder={() => { close(); onOpenOtherFolder(); }}
          />
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Add CSS**

Append to `apps/desktop/src/App.css`:

```css
.workspace-history-dropdown {
  position: relative;
  display: inline-flex;
}

.workspace-history-panel {
  position: absolute;
  top: calc(100% + var(--space-2));
  left: 0;
  z-index: 20;
  min-width: 260px;
  max-width: 360px;
  max-height: 320px;
  overflow-y: auto;
  background: var(--surface-2);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md, 0 4px 12px rgba(0, 0, 0, 0.2));
  padding: var(--space-3);
  -webkit-app-region: no-drag;
}

.workspace-history-section-label {
  margin: var(--space-2) 0 var(--space-1);
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
}

.workspace-history-rows {
  list-style: none;
  margin: 0;
  padding: 0;
}

.workspace-history-row {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}

.workspace-history-row-open {
  flex: 1;
  min-width: 0;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  background: none;
  border: none;
  color: var(--text-primary);
  font: inherit;
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-md);
  cursor: pointer;
}

.workspace-history-row-open:hover:not(:disabled) {
  background: var(--surface-3);
}

.workspace-history-row-open:disabled {
  color: var(--text-muted);
  cursor: default;
}

.workspace-history-current-badge {
  margin-left: var(--space-2);
  font-size: var(--text-sm);
  color: var(--text-muted);
}

.workspace-history-row-action {
  flex-shrink: 0;
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: var(--space-1);
  border-radius: var(--radius-md);
}

.workspace-history-row-action:hover {
  color: var(--text-secondary);
  background: var(--surface-3);
}

.workspace-history-empty,
.workspace-history-diagnostic {
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin: var(--space-2) 0;
}

.workspace-history-other-folder {
  width: 100%;
  margin-top: var(--space-2);
  text-align: left;
  background: none;
  border: none;
  border-top: 1px solid var(--border-default);
  color: var(--text-secondary);
  font: inherit;
  padding: var(--space-2) 0 0;
  cursor: pointer;
}

.workspace-history-other-folder:hover {
  color: var(--text-primary);
}
```

- [ ] **Step 5: Write the source-assertion test**

Following the codebase's existing convention for renderer-component checks that don't warrant a full render harness (see `tests/dockview.test.ts`), create `apps/desktop/tests/workspaceHistoryComponents.test.ts`:

```typescript
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const listSource = readFileSync(new URL("../src/components/WorkspaceHistoryList.tsx", import.meta.url), "utf8");
const dropdownSource = readFileSync(new URL("../src/components/WorkspaceHistoryDropdown.tsx", import.meta.url), "utf8");
const labelsSource = readFileSync(new URL("../src/labels.ts", import.meta.url), "utf8");

test("WorkspaceHistoryList disables opening the current workspace but keeps pin/remove available", () => {
  assert.match(listSource, /disabled=\{isCurrent \|\| isSwitching\}/);
  assert.match(listSource, /onClick=\{\(\) => \(pinned \? handleUnpin\(path\) : handlePin\(path\)\)\}/);
  assert.match(listSource, /onClick=\{\(\) => handleForget\(path\)\}/);
});

test("WorkspaceHistoryList renders pinned and recent as separate sections using VOCAB", () => {
  assert.match(listSource, /VOCAB\.workspaceHistoryPinnedSection/);
  assert.match(listSource, /VOCAB\.workspaceHistoryRecentSection/);
  assert.match(listSource, /VOCAB\.workspaceHistoryEmpty/);
  assert.match(listSource, /VOCAB\.workspaceHistoryOpenOtherFolder/);
  assert.doesNotMatch(listSource, /"Pinned"|"Recent"/);
});

test("WorkspaceHistoryDropdown dismisses on outside click and Escape without a new dependency", () => {
  assert.match(dropdownSource, /document\.addEventListener\("mousedown", handlePointerDown\)/);
  assert.match(dropdownSource, /document\.addEventListener\("keydown", handleKeyDown\)/);
  assert.match(dropdownSource, /event\.key === "Escape"/);
});

test("labels.ts defines the workspace-history VOCAB entries", () => {
  assert.match(labelsSource, /pinWorkspace: "Pin workspace"/);
  assert.match(labelsSource, /unpinWorkspace: "Unpin workspace"/);
  assert.match(labelsSource, /removeWorkspace: "Remove from history"/);
});
```

- [ ] **Step 6: Run the new test and type-check**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/workspaceHistoryComponents.test.ts && npx tsc --noEmit`
Expected: all pass, no type errors.

- [ ] **Step 7: Commit**

```bash
git add src/labels.ts src/components/WorkspaceHistoryList.tsx src/components/WorkspaceHistoryDropdown.tsx src/App.css tests/workspaceHistoryComponents.test.ts
git commit -m "feat: add WorkspaceHistoryList and WorkspaceHistoryDropdown components"
```

---

### Task 6: Wire the dropdown into `App.tsx`'s titlebar

Replaces the plain ⇄ button and the no-workspace "Open workspace" button with `WorkspaceHistoryDropdown`, and adds the `handleOpenRememberedWorkspace` handler.

**Files:**
- Modify: `apps/desktop/src/App.tsx:270-293` (add handler after `handleOpenWorkspace`)
- Modify: `apps/desktop/src/App.tsx:696-735` (replace the two trigger buttons)
- Test: `apps/desktop/tests/electronSidecarWiring.test.ts` (append one test; this file already reads `appSource`)

**Interfaces:**
- Consumes: `WorkspaceHistoryDropdown` (Task 5), `window.orkworks.openRememberedWorkspace` (Task 4), existing `workspace`, `isSwitchingWorkspace`, `pushToast`, `handleOpenWorkspace` from `App.tsx`.
- Produces: nothing further downstream (leaf integration).

- [ ] **Step 1: Add `handleOpenRememberedWorkspace`**

In `apps/desktop/src/App.tsx`, immediately after the closing of `handleOpenWorkspace` (after line 293, `}, []);`), add:

```tsx
  const handleOpenRememberedWorkspace = useCallback(async (path: string) => {
    setIsSwitchingWorkspace(true);
    try {
      await window.orkworks.openRememberedWorkspace(path);
    } catch {
      pushToast("error", "Couldn't open workspace.");
    } finally {
      setIsSwitchingWorkspace(false);
    }
  }, []);

  const handleWorkspaceHistoryError = useCallback((message: string) => {
    pushToast("error", message);
  }, []);
```

- [ ] **Step 2: Import the dropdown component**

Add near the other component imports at the top of `App.tsx`:

```tsx
import { WorkspaceHistoryDropdown } from "./components/WorkspaceHistoryDropdown";
```

- [ ] **Step 3: Replace the titlebar switch button**

Replace:

```tsx
              <button
                className="titlebar-switch-button"
                type="button"
                onClick={handleOpenWorkspace}
                title={VOCAB.switchWorkspace}
                aria-label={VOCAB.switchWorkspace}
              >
                &#x21C4;
              </button>
```

with:

```tsx
              <WorkspaceHistoryDropdown
                triggerLabel="⇄"
                triggerClassName="titlebar-switch-button"
                triggerAriaLabel={VOCAB.switchWorkspace}
                currentWorkspacePath={workspace?.path ?? null}
                isSwitching={isSwitchingWorkspace}
                onOpenPath={handleOpenRememberedWorkspace}
                onOpenOtherFolder={handleOpenWorkspace}
                onError={handleWorkspaceHistoryError}
              />
```

- [ ] **Step 4: Replace the no-workspace "Open workspace" button**

Replace:

```tsx
              <button
                className="titlebar-open-button"
                type="button"
                onClick={handleOpenWorkspace}
              >
                {VOCAB.openWorkspace}
              </button>
```

with:

```tsx
              <WorkspaceHistoryDropdown
                triggerLabel={VOCAB.openWorkspace}
                triggerClassName="titlebar-open-button"
                triggerAriaLabel={VOCAB.openWorkspace}
                currentWorkspacePath={null}
                isSwitching={isSwitchingWorkspace}
                onOpenPath={handleOpenRememberedWorkspace}
                onOpenOtherFolder={handleOpenWorkspace}
                onError={handleWorkspaceHistoryError}
              />
```

- [ ] **Step 5: Write the wiring test**

Append to `apps/desktop/tests/electronSidecarWiring.test.ts` (reuses the file's existing `appSource` constant):

```typescript
test("App wires both workspace-history dropdown triggers to the remembered-open handler", () => {
  assert.match(appSource, /import \{ WorkspaceHistoryDropdown \} from "\.\/components\/WorkspaceHistoryDropdown";/);
  assert.equal(appSource.match(/<WorkspaceHistoryDropdown/g)?.length, 2);
  assert.match(appSource, /onOpenPath=\{handleOpenRememberedWorkspace\}/);
  assert.match(appSource, /onOpenOtherFolder=\{handleOpenWorkspace\}/);
  assert.match(appSource, /await window\.orkworks\.openRememberedWorkspace\(path\);/);
});
```

- [ ] **Step 6: Run the test and type-check**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/electronSidecarWiring.test.ts && npx tsc --noEmit`
Expected: all pass, no type errors.

- [ ] **Step 7: Commit**

```bash
git add src/App.tsx tests/electronSidecarWiring.test.ts
git commit -m "feat: surface the workspace history dropdown in the titlebar"
```

---

### Task 7: Full verification and manual walkthrough

**Files:** none (verification only).

**Interfaces:** none.

- [ ] **Step 1: Run the full desktop suite**

Run: `cd apps/desktop && npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: all pass.

- [ ] **Step 2: Run the API test file separately (per `apps/desktop/AGENTS.md`)**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts`
Expected: passes.

- [ ] **Step 3: Run the repository doc-currency check**

Run: `bash scripts/doc-check.sh` (from the repository root)
Expected: no flagged files. If `AGENTS.md`'s `## Metadata protocol` file listing needs a note about the v2 schema, add it and re-run — but do not add one speculatively; only if the check flags it.

- [ ] **Step 4: Manual walkthrough in the running app**

Run: `cd apps/desktop && pnpm dev`

Walk through, per the root `AGENTS.md` UI-verification bar:
1. With no workspace open, confirm the "Open workspace" dropdown shows an empty state (`VOCAB.workspaceHistoryEmpty`) on first run, plus "Open other folder…".
2. Open two different folders in turn (via "Open other folder…"). Confirm both now appear under "Recent" in both dropdowns (titlebar ⇄ and the no-workspace one).
3. Pin one of them. Confirm it moves to a "Pinned" section and disappears from "Recent".
4. With the pinned workspace as the currently open one, confirm its row in the dropdown is shown as "Current" and is not clickable, but its pin/remove icons still work.
5. Unpin it. Confirm it reappears under "Recent" at the front.
6. Remove a workspace from history via the ✕ button. Confirm it disappears from the dropdown and that the underlying folder on disk is untouched.
7. Click a different, non-current remembered workspace's row. Confirm the app performs the existing close-then-open switch (loading state shows, then the new workspace becomes active) exactly as clicking "Open workspace" → picking that folder does today.
8. Click outside an open dropdown, and press Escape while one is open. Confirm both dismiss it.

- [ ] **Step 5: Run the repository-wide worktree currency check**

Run: `bash .claude/hooks/worktree-check.sh` (from the repository root)

- [ ] **Step 6: Final commit if manual walkthrough required fixes**

If Step 4 surfaced any bug, fix it, re-run Steps 1 and 4, then:

```bash
git add -A
git commit -m "fix: address manual walkthrough findings for workspace history switcher"
```

If no fixes were needed, this task requires no commit — proceed to opening the PR per the root `AGENTS.md` branch/PR workflow (squash-merge by default, `/code-review low` gate before merge since this touches `apps/desktop/`).
