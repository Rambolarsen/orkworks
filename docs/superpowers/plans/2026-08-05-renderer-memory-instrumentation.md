# Renderer Memory Instrumentation & Lifecycle Invariants Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** For issue #247 (Electron renderer reached 7.6 GB / ~5s periodic stalls after 27h), land evidence-gathering instrumentation and fix the one deterministic lifecycle-invariant violation visible from static analysis — without speculating aheap fix before a heap snapshot identifies the retaining path.

**Architecture:** Three deliverables, each independently shippable as a testable unit:

1. A pure `TerminalRegistry<T>` bookkeeping module that owns the live-terminal map and a per-id `disposed` flag, with `prune(keepSet)` semantics. `terminalStore.ts` delegates to it so the lifecycle invariants are unit-testable without DOM/WebSocket.
2. A disposed-guard in the `ws.onclose` `getTerminalOutput().then(writeTerminalReplay)` path so a mid-flight fetch cannot write into an xterm the registry already marked disposed (this was the one deterministic leak visible from static reading).
3. A pure renderer-side `rendererHealthProbe` aggregator (heap size from `performance.memory`, live-terminal count, dead-session DOM node count, dockview panel count) wired behind a new `DebugSettings.rendererHealthLogMs` opt-in, which console-logs a sample on a `setInterval` and exposes `window.__orkworksCaptureRendererHealth()` for ad-hoc DevTools capture. No automatic heap snapshots; the user triggers V8's `_HEAP_SNAPSHOT` from DevTools once the probe shows growth.

**Tech Stack:** TypeScript + React in `apps/desktop/` (renderer); Electron preload diagnostics bridge; node:test for unit tests; no new npm dependencies; no Rust changes.

## Global Constraints

- This is **Phase 1 evidence-gathering + one deterministic invariant fix**. Do not add speculative "free terminal on session-switch" changes — ADR 0022 (renderer terminal attachment is detachable from PTY lifetime) is load-bearing; the static pruning rules in `terminalStore.ts` already dispose dead/forget/kill/resume sessions.
- pnpm-only. Run renderer-side commands from `apps/desktop/`.
- TypeScript validation commands (run from `apps/desktop/`):
  - Type-check: `npx tsc --noEmit`
  - Whole renderer test suite: `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
  - Single test file: `node --experimental-strip-types --test tests/<file>.test.ts`
- Existing terminalStore tests use source-regex assertions (see `tests/terminalDetachSource.test.ts`) — the same style is used here for the disposed-guard test, since `terminalStore.ts` instantiates `WebSocket` and xterm at module load and is not unit-instantiable without a refactor this plan deliberately does not include.
- No Electron IPC additions. The probe is computed and consumed entirely in the renderer; the main/preload changes are limited to plumbing one synchronous debug boolean (`rendererHealthLogMs`) through the existing settings round-trip.
- Every step that changes code must leave `npx tsc --noEmit` and the affected test file green before moving to the next step.
- Work happens on branch `fix-247-renderer-memory` in the primary checkout at `/Users/froomiebot/workspace/orkworks`. No worktree (no parallel agent is in flight on this path).
- Docs: update `AGENTS.md`/`docs/agents/domain-entities.md` only if a session-status/lifecycle vocabulary is touched — this plan touches none, so skip.

### Out of scope (explicitly deferred until after a heap snapshot identifies the retaining path)

- Refactoring `terminalStore.ts` to be WebSocket-injectable (real e2e lifecycle test).
- Any change to the detachable-attachment rule (ADR 0022).
- Disposing live terminals on session-switch-away.
- Replacing `WebglAddon` or restructuring xterm disposal.
- Moving the 2s poll cadence or changing `mergeSessionsById`.
- Reducing terminal-output replay size (that's #244, not #247).

---

### Task 1: Pure `TerminalRegistry<T>` bookkeeping module

**Files:**
- Create: `apps/desktop/src/terminalRegistry.ts`
- Create: `apps/desktop/tests/terminalRegistry.test.ts`

**Interfaces:**
- Produces: `createTerminalRegistry<T>()` returning `TerminalRegistry<T>` with the public methods consumed by Task 2:
  - `get(id: string): T | undefined` — returns the handle if the id is live (not disposed); `undefined` after `remove(id)` or `prune` chose to dispose it.
  - `set(id: string, handle: T): void` — register a handle for an id; no-op overwrite is the caller's contract (caller already dedups via `get` before `set`).
  - `remove(id: string): T | undefined` — marks the id disposed and removes the handle from the live map; returns the removed handle so the caller can run synchronous teardown (`terminal.dispose()` etc.) on it. Idempotent: a second `remove` returns `undefined`.
  - `prune(keep: ReadonlySet<string>): T[]` — for every id in the registry that is **not** in `keep`, mark it disposed, remove it from the live map, and collect the handle. Returns the list of removed handles in insertion order. Idempotent. `keep` may contain ids that have never been registered — they are ignored.
  - `isDisposed(id: string): boolean` — `true` once `remove`/`prune` marked the id disposed, **false** if the id was never seen, **false** while the id is still live. (The distinction "never seen" vs "disposed" doesn't matter for the guard; post-await we only care "did this id get torn down while I was in flight".)
  - `get size(): number` — count of live handles, for the health probe (Task 3).
  - `liveIds(): readonly string[]` — snapshot of currently-live ids, for the health probe (Task 3).
- Designed as a closure over a private `Map` and a private `Set<string>` of disposed ids; no class needed.

- [ ] **Step 1: Write the failing tests**

Create `apps/desktop/tests/terminalRegistry.test.ts`:

```typescript
import test from "node:test";
import assert from "node:assert/strict";
import { createTerminalRegistry } from "../src/terminalRegistry.ts";

test("set then get returns the handle", () => {
  const reg = createTerminalRegistry<{ x: number }>();
  reg.set("a", { x: 1 });
  assert.equal(reg.size, 1);
  assert.deepEqual(reg.get("a"), { x: 1 });
  assert.equal(reg.isDisposed("a"), false);
});

test("remove returns the handle and marks disposed; second remove returns undefined", () => {
  const reg = createTerminalRegistry<{ x: number }>();
  reg.set("a", { x: 1 });
  const removed = reg.remove("a");
  assert.deepEqual(removed, { x: 1 });
  assert.equal(reg.get("a"), undefined);
  assert.equal(reg.size, 0);
  assert.equal(reg.isDisposed("a"), true);
  assert.equal(reg.remove("a"), undefined);
});

test("prune keeps the keep-set and returns the disposed handles in insertion order", () => {
  const reg = createTerminalRegistry<number>();
  reg.set("a", 1);
  reg.set("b", 2);
  reg.set("c", 3);
  const removed = reg.prune(new Set(["a", "never-seen"]));
  assert.deepEqual(removed, [2, 3]);
  assert.equal(reg.size, 1);
  assert.deepEqual(reg.get("a"), 1);
  assert.equal(reg.get("b"), undefined);
  assert.equal(reg.get("c"), undefined);
  assert.equal(reg.isDisposed("b"), true);
  assert.equal(reg.isDisposed("c"), true);
  assert.equal(reg.isDisposed("never-seen"), false);
});

test("prune with empty keep-set disposes everything", () => {
  const reg = createTerminalRegistry<number>();
  reg.set("a", 1);
  reg.set("b", 2);
  assert.deepEqual(reg.prune(new Set()), [1, 2]);
  assert.equal(reg.size, 0);
});

test("prune is idempotent and ignores ids in keep that were never registered", () => {
  const reg = createTerminalRegistry<number>();
  reg.set("a", 1);
  assert.deepEqual(reg.prune(new Set(["a", "z"])), []);
  assert.deepEqual(reg.prune(new Set()), [1]);
});

test("liveIds returns a snapshot of currently-live ids", () => {
  const reg = createTerminalRegistry<number>();
  reg.set("a", 1);
  reg.set("b", 2);
  const ids = reg.liveIds();
  assert.deepEqual([...ids].sort(), ["a", "b"]);
  reg.remove("a");
  assert.deepEqual([...reg.liveIds()], ["b"]);
});

test("isDisposed is false for ids the registry has never seen", () => {
  const reg = createTerminalRegistry<number>();
  assert.equal(reg.isDisposed("never-seen"), false);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/terminalRegistry.test.ts
```

Expected: FAIL with `Error: Cannot find module '../src/terminalRegistry.ts'` (or similar resolution error).

- [ ] **Step 3: Implement the registry**

Create `apps/desktop/src/terminalRegistry.ts`:

```typescript
export interface TerminalRegistry<T> {
  get(id: string): T | undefined;
  set(id: string, handle: T): void;
  remove(id: string): T | undefined;
  prune(keep: ReadonlySet<string>): T[];
  isDisposed(id: string): boolean;
  liveIds(): readonly string[];
  readonly size: number;
}

export function createTerminalRegistry<T>(): TerminalRegistry<T> {
  const handles = new Map<string, T>();
  const disposed = new Set<string>();
  return {
    get(id) {
      return handles.get(id);
    },
    set(id, handle) {
      handles.set(id, handle);
    },
    remove(id) {
      const h = handles.get(id);
      if (h === undefined) return undefined;
      handles.delete(id);
      disposed.add(id);
      return h;
    },
    prune(keep) {
      const removed: T[] = [];
      for (const [id, handle] of handles) {
        if (!keep.has(id)) {
          handles.delete(id);
          disposed.add(id);
          removed.push(handle);
        }
      }
      return removed;
    },
    isDisposed(id) {
      return disposed.has(id);
    },
    liveIds() {
      return [...handles.keys()];
    },
    get size() {
      return handles.size;
    },
  };
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/terminalRegistry.test.ts
```

Expected: 7 tests pass.

- [ ] **Step 5: Type-check**

Run from `apps/desktop/`:

```bash
npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/terminalRegistry.ts apps/desktop/tests/terminalRegistry.test.ts
git commit -m "feat(desktop): pure TerminalRegistry with disposed-flag enforcement"
```

---

### Task 2: `terminalStore` delegates to `TerminalRegistry`; disposed-guard in the post-close replay-fetch

**Files:**
- Modify: `apps/desktop/src/terminalStore.ts` (replace the module-level `Map` with a registry; thread `markDisposed` through `disposeTerminal`, `pruneTerminals`, `disposeAllTerminals`; add the disposed-guard after the `getTerminalOutput` await; export `terminalRegistrySize`/`terminalRegistryLiveIds` for Task 3 to read)
- Create: `apps/desktop/tests/terminalReplayDisposeGuard.test.ts`

**Interfaces:**
- Consumes: `TerminalRegistry<T>` from Task 1.
- Produces:
  - `getLiveTerminalCount(): number` — needed by the health probe (Task 3). Reads `registry.size`.
  - `getLiveTerminalIds(): readonly string[]` — needed by the health probe (Task 3). Reads `registry.liveIds()`.
- Public API surface stays otherwise identical: `getTerminal`, `ensureTerminal`, `disposeTerminal`, `pruneTerminals`, `disposeAllTerminals`, `TerminalHandle`.

- [ ] **Step 1: Write the failing source-pattern test**

Create `apps/desktop/tests/terminalReplayDisposeGuard.test.ts`:

```typescript
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const source = readFileSync(
  new URL("../src/terminalStore.ts", import.meta.url),
  "utf8",
);

test("ws.onclose replay-fetch guards handle.disposed before writing replay", () => {
  const block = source.match(/getTerminalOutput\(baseUrl, id\)\.then\(\([\s\S]*?\}\)\.catch\(/)?.[0]
    ?? "";
  assert.match(
    block,
    /handle\.disposed/,
    "the post-fetch path must check handle.disposed before writing replay into the terminal",
  );
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/terminalReplayDisposeGuard.test.ts
```

Expected: FAIL with the assertion message about `handle.disposed`.

- [ ] **Step 3: Refactor `terminalStore.ts` to use the registry**

In `apps/desktop/src/terminalStore.ts`:

1. Add import at the top with the other imports:

```typescript
import { createTerminalRegistry, type TerminalRegistry } from "./terminalRegistry";
```

2. Replace the module-level `const terminals = new Map<string, TerminalHandle>();` (line 31) with:

```typescript
const terminals: TerminalRegistry<TerminalHandle> = createTerminalRegistry<TerminalHandle>();
```

3. Replace the body of `getTerminal` (lines 43–45) with:

```typescript
export function getTerminal(id: string): TerminalHandle | undefined {
  return terminals.get(id);
}
```

(this is unchanged behavior — `registry.get` returns `undefined` for a disposed id, and `ensureTerminal` already guards with `if (existing) return existing;`).

4. In `ensureTerminal`, replace the `terminals.set(id, handle)` line (line 116) — it stays as `terminals.set(id, handle)` (now the registry method).

5. Replace the body of `disposeTerminal` (lines 234–251) with:

```typescript
export function disposeTerminal(id: string): void {
  const handle = terminals.remove(id);
  if (!handle) return;
  handle.disposed = true;
  handle.resizeObserver.disconnect();
  try {
    handle.ws.close();
  } catch {
    /* ignore */
  }
  try {
    handle.terminal.dispose();
  } catch {
    /* ignore */
  }
  handle.wrapper.remove();
}
```

(Note: `handle.disposed = true` is set **after** the registry has removed the id, so `registry.isDisposed(id)` is already true; this preserves the existing `handle.disposed` flag for any code that reads it directly.)

6. Replace `pruneTerminals` (lines 253–257) with:

```typescript
export function pruneTerminals(keepLiveSessionIds: ReadonlySet<string>): void {
  for (const handle of terminals.prune(keepLiveSessionIds)) {
    handle.disposed = true;
    handle.resizeObserver.disconnect();
    try {
      handle.ws.close();
    } catch {
      /* ignore */
    }
    try {
      handle.terminal.dispose();
    } catch {
      /* ignore */
    }
    handle.wrapper.remove();
  }
}
```

7. Replace `disposeAllTerminals` (lines 259–261) with:

```typescript
export function disposeAllTerminals(): void {
  pruneTerminals(new Set());
}
```

8. Add the registry accessors right after `disposeAllTerminals`:

```typescript
export function getLiveTerminalCount(): number {
  return terminals.size;
}

export function getLiveTerminalIds(): readonly string[] {
  return terminals.liveIds();
}
```

- [ ] **Step 4: Add the disposed-guard in `ws.onclose`**

In `apps/desktop/src/terminalStore.ts`, find the `ws.onclose = () => { ... }` block (lines 176–194). Replace the inner `if (shouldReplayTerminalOutputOnClose(...)) { ... }` block with:

```typescript
    if (
      shouldReplayTerminalOutputOnClose({
        disposed: handle.disposed,
        receivedData,
      })
    ) {
      getTerminalOutput(baseUrl, id).then((payload) => {
        if (handle.disposed) return;
        writeTerminalReplay(term, payload.lines);
      }).catch(() => {
        /* silently ignore fetch failures */
      });
    }
```

- [ ] **Step 5: Run the source-pattern test to verify it passes**

Run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/terminalReplayDisposeGuard.test.ts
```

Expected: PASS.

- [ ] **Step 6: Run the whole renderer test suite + type-check**

Run from `apps/desktop/`:

```bash
npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: type-check clean; all tests pass, including the existing `terminalDetachSource.test.ts` and `dockview.test.ts`.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src/terminalStore.ts apps/desktop/tests/terminalReplayDisposeGuard.test.ts
git commit -m "fix(desktop): guard dead-session replay writes after terminal dispose (#247)"
```

---

### Task 3: Pure `rendererHealthProbe` aggregator

**Files:**
- Create: `apps/desktop/src/rendererHealthProbe.ts`
- Create: `apps/desktop/tests/rendererHealthProbe.test.ts`

**Interfaces:**
- Consumes (passed in by Task 4):
  - `getLiveTerminalCount(): number` — from `terminalStore` (Task 2).
  - `getLiveTerminalIds(): readonly string[]` — from `terminalStore` (Task 2).
  - The global `performance.memory` (Chrome-only, available in Electron): `{ usedJSHeapSize: number; totalJSHeapSize: number; jsHeapSizeLimit: number } | undefined`.
  - The global `document`: count of `.xterm` DOM nodes inside `.terminal-shell` containers (each dead-session view mounts an xterm `.xterm` inside a `.terminal-shell`).
  - A `dockviewApiRef` for panel count — passed in by Task 4 via an injected `panelCountProvider: () => number`. Task 3 does **not** import dockview-react; the provider closure is supplied at the call site.
- Produces: `captureRendererHealth(deps): RendererHealthSample`, an object snapshot for one tick.

```typescript
export interface RendererHealthSample {
  capturedAt: number;          // Date.now() at capture
  usedJSHeapSize?: number;     // bytes, undefined if performance.memory absent
  totalJSHeapSize?: number;
  jsHeapSizeLimit?: number;
  liveTerminalCount: number;
  liveTerminalIds: readonly string[];
  historicalTerminalNodeCount: number;  // count of `.xterm` nodes inside `.terminal-shell`
}

export interface RendererHealthProbeDeps {
  panelCountProvider: () => number;
  liveTerminalCountProvider: () => number;
  liveTerminalIdsProvider: () => readonly string[];
}

export function captureRendererHealth(deps: RendererHealthProbeDeps): RendererHealthSample { ... }
```

`captureRendererHealth` is **pure of network/DOM-globals as much as possible**: it reads the procided providers and `document.querySelector` for the historical-node count, and reads `performance.memory` defensively (try/catch). It is callable from both the renderer and a unit test that stubs `document`/`performance`/`globalThis.window` via dependency injection — keep the function free of module-level singletons.

- [ ] **Step 1: Write the failing tests**

Create `apps/desktop/tests/rendererHealthProbe.test.ts`:

```typescript
import test from "node:test";
import assert from "node:assert/strict";
import { captureRendererHealth, type RendererHealthSample } from "../src/rendererHealthProbe.ts";

function makeDeps({
  liveTerminalCount = 0,
  liveTerminalIds = [] as readonly string[],
  panelCount = 0,
} = {}) {
  return {
    panelCountProvider: () => panelCount,
    liveTerminalCountProvider: () => liveTerminalCount,
    liveTerminalIdsProvider: () => liveTerminalIds,
  };
}

test("captureRendererHealth reports live-terminal count and ids from the providers", () => {
  const sample = captureRendererHealth(makeDeps({ liveTerminalCount: 2, liveTerminalIds: ["a", "b"], panelCount: 3 }));
  assert.equal(sample.liveTerminalCount, 2);
  assert.deepEqual([...sample.liveTerminalIds], ["a", "b"]);
});

test("captureRendererHealth reports zero historical-terminal xterm nodes when document has none", () => {
  const sample = captureRendererHealth(makeDeps());
  assert.equal(sample.historicalTerminalNodeCount, 0);
});

test("captureRendererHealth reports performance.memory when available", () => {
  const original = (globalThis as any).performance;
  (globalThis as any).performance = {
    ...original,
    memory: { usedJSHeapSize: 1000, totalJSHeapSize: 2000, jsHeapSizeLimit: 4000 },
  };
  try {
    const sample = captureRendererHealth(makeDeps());
    assert.equal(sample.usedJSHeapSize, 1000);
    assert.equal(sample.totalJSHeapSize, 2000);
    assert.equal(sample.jsHeapSizeLimit, 4000);
  } finally {
    (globalThis as any).performance = original;
  }
});

test("captureRendererHealth omits memory fields when performance.memory is unavailable", () => {
  const original = (globalThis as any).performance;
  (globalThis as any).performance = { ...original, memory: undefined };
  try {
    const sample = captureRendererHealth(makeDeps());
    assert.equal(sample.usedJSHeapSize, undefined);
    assert.equal(sample.totalJSHeapSize, undefined);
    assert.equal(sample.jsHeapSizeLimit, undefined);
  } finally {
    (globalThis as any).performance = original;
  }
});

test("captureRendererHealth counts `.xterm` nodes inside `.terminal-shell` containers", () => {
  const original = (globalThis as any).document;
  const shell = {
    querySelectorAll: (sel: string) =>
      sel === ".xterm" ? [{}, {}] : [],
  };
  (globalThis as any).document = {
    ...original,
    querySelectorAll: (sel: string) => (sel === ".terminal-shell" ? [shell] : []),
  };
  try {
    const sample = captureRendererHealth(makeDeps());
    assert.equal(sample.historicalTerminalNodeCount, 2);
  } finally {
    (globalThis as any).document = original;
  }
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/rendererHealthProbe.test.ts
```

Expected: FAIL with `Cannot find module '../src/rendererHealthProbe.ts'`.

- [ ] **Step 3: Implement `rendererHealthProbe.ts`**

Create `apps/desktop/src/rendererHealthProbe.ts`:

```typescript
export interface RendererHealthSample {
  capturedAt: number;
  usedJSHeapSize?: number;
  totalJSHeapSize?: number;
  jsHeapSizeLimit?: number;
  liveTerminalCount: number;
  liveTerminalIds: readonly string[];
  historicalTerminalNodeCount: number;
}

export interface RendererHealthProbeDeps {
  panelCountProvider: () => number;
  liveTerminalCountProvider: () => number;
  liveTerminalIdsProvider: () => readonly string[];
}

function readMemory(): { usedJSHeapSize?: number; totalJSHeapSize?: number; jsHeapSizeLimit?: number } {
  try {
    const mem = (globalThis as { performance?: { memory?: { usedJSHeapSize: number; totalJSHeapSize: number; jsHeapSizeLimit: number } } }).performance?.memory;
    if (!mem) return {};
    return {
      usedJSHeapSize: mem.usedJSHeapSize,
      totalJSHeapSize: mem.totalJSHeapSize,
      jsHeapSizeLimit: mem.jsHeapSizeLimit,
    };
  } catch {
    return {};
  }
}

function countHistoricalTerminalNodes(): number {
  try {
    const doc = globalThis as { document?: Document };
    if (!doc.document) return 0;
    const shells = doc.document.querySelectorAll(".terminal-shell");
    let count = 0;
    shells.forEach((shell) => {
      const el = shell as Element;
      count += el.querySelectorAll(".xterm").length;
    });
    return count;
  } catch {
    return 0;
  }
}

export function captureRendererHealth(deps: RendererHealthProbeDeps): RendererHealthSample {
  return {
    capturedAt: Date.now(),
    ...readMemory(),
    liveTerminalCount: deps.liveTerminalCountProvider(),
    liveTerminalIds: deps.liveTerminalIdsProvider(),
    historicalTerminalNodeCount: countHistoricalTerminalNodes(),
  };
}
```

(`panelCountProvider` is accepted for future use; reading it is omitted for now to avoid an unused-variable lint, but the field stays on the deps interface so Task 4's wiring signature is stable.)

- [ ] **Step 4: Wire `panelCountProvider` use so it isn't unused**

Avoid shipping an unused-parameter lint. Replace the body of `captureRendererHealth` so it consumes `panelCountProvider` by storing it in a `panelCount` field on the sample. Update the `RendererHealthSample` interface to include `panelCount: number`, and add a test that asserts the value surfaces. Concretely:

Add to `RendererHealthSample`:

```typescript
  panelCount: number;
```

Replace the returned object with:

```typescript
  return {
    capturedAt: Date.now(),
    ...readMemory(),
    liveTerminalCount: deps.liveTerminalCountProvider(),
    liveTerminalIds: deps.liveTerminalIdsProvider(),
    historicalTerminalNodeCount: countHistoricalTerminalNodes(),
    panelCount: deps.panelCountProvider(),
  };
```

Append to `tests/rendererHealthProbe.test.ts`:

```typescript
test("captureRendererHealth surfaces the dockview panel count", () => {
  const sample = captureRendererHealth(makeDeps({ panelCount: 4 }));
  assert.equal(sample.panelCount, 4);
});
```

- [ ] **Step 5: Run the tests to verify they pass**

Run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/rendererHealthProbe.test.ts
```

Expected: all tests pass.

- [ ] **Step 6: Type-check**

Run from `apps/desktop/`:

```bash
npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src/rendererHealthProbe.ts apps/desktop/tests/rendererHealthProbe.test.ts
git commit -m "feat(desktop): pure rendererHealthProbe aggregator for #247 instrumentation"
```

---

### Task 4: `DebugSettings.rendererHealthLogMs` opt-in + App wiring + ad-hoc DevTools hook

**Files:**
- Modify: `apps/desktop/src/appSettingsTypes.ts` (extend `DebugSettings`)
- Modify: `apps/desktop/electron/settingsMemory.ts` (`DEFAULT_DEBUG_SETTINGS`, `normalizeDebugSettings`)
- Modify: `apps/desktop/src/orkworksWindow.d.ts` (no change — the probe is renderer-only and exported via `window.__orkworksCaptureRendererHealth`, not the preload bridge)
- Modify: `apps/desktop/src/App.tsx` (start/stop logging on a `setInterval` keyed on the debug setting; expose `window.__orkworksCaptureRendererHealth`)
- Create: `apps/desktop/tests/appRendererHealthWiring.test.ts` (source-pattern assertion that `App.tsx` reads `settings.debug.rendererHealthLogMs` and calls `setInterval`, plus cleans up)

**Interfaces:**
- Consumes: `captureRendererHealth` from Task 3, `getLiveTerminalCount`/`getLiveTerminalIds` from Task 2, the `dockviewApiRef` already owned by `App` for the panel-count provider.
- Produces: a global `window.__orkworksCaptureRendererHealth?: () => RendererHealthSample` that the user can call from DevTools while reproducing the 7.6 GB stall.

- [ ] **Step 1: Write the failing source-pattern test**

Create `apps/desktop/tests/appRendererHealthWiring.test.ts`:

```typescript
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appSource = readFileSync(
  new URL("../src/App.tsx", import.meta.url),
  "utf8",
);

test("App reads settings.debug.rendererHealthLogMs to start the health probe", () => {
  assert.match(appSource, /rendererHealthLogMs/);
});

test("App uses setInterval for the health probe and clear it in cleanup", () => {
  assert.match(appSource, /setInterval\([\s\S]*?clearInterval/);
});

test("App exposes window.__orkworksCaptureRendererHealth for ad-hoc DevTools capture", () => {
  assert.match(appSource, /__orkworksCaptureRendererHealth/);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/appRendererHealthWiring.test.ts
```

Expected: FAIL on all three assertions (none of the symbols exist in `App.tsx` yet).

- [ ] **Step 3: Extend `DebugSettings`**

In `apps/desktop/src/appSettingsTypes.ts`, replace the `DebugSettings` interface with:

```typescript
export interface DebugSettings {
  showSessionIds: boolean;
  rendererHealthLogMs: number;
}
```

In `apps/desktop/electron/settingsMemory.ts`:

1. Replace `DEFAULT_DEBUG_SETTINGS` (lines 105–107) with:

```typescript
export const DEFAULT_DEBUG_SETTINGS: DebugSettings = {
  showSessionIds: false,
  rendererHealthLogMs: 0,
};
```

2. Replace `normalizeDebugSettings` (lines 226–237) with:

```typescript
export function normalizeDebugSettings(value: unknown): DebugSettings {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return { ...DEFAULT_DEBUG_SETTINGS };
  }
  const raw = value as Record<string, unknown>;
  const showSessionIds =
    typeof raw.showSessionIds === "boolean"
      ? raw.showSessionIds
      : DEFAULT_DEBUG_SETTINGS.showSessionIds;
  const rendererHealthLogMs =
    typeof raw.rendererHealthLogMs === "number" && Number.isFinite(raw.rendererHealthLogMs) && raw.rendererHealthLogMs >= 0
      ? Math.floor(raw.rendererHealthLogMs)
      : DEFAULT_DEBUG_SETTINGS.rendererHealthLogMs;
  return { showSessionIds, rendererHealthLogMs };
}
```

- [ ] **Step 4: Wire the probe into `App.tsx`**

In `apps/desktop/src/App.tsx`:

1. Add imports near the existing terminal-store import (line 28):

```typescript
import { getLiveTerminalCount, getLiveTerminalIds } from "./terminalStore";
import { captureRendererHealth, type RendererHealthSample } from "./rendererHealthProbe";
```

2. Inside `function App()` (after the existing refs/useStates declared at the top of the function), add an effect that starts/stops the logging interval and exposes the ad-hoc capture hook. Insert immediately after the `dockviewApiRef`/`sessionsHiddenLayoutRef` declarations (lines 47–48) and before the first `useEffect` (line 50):

```typescript
  useEffect(() => {
    const intervalMs = settings?.debug?.rendererHealthLogMs ?? 0;
    if (!intervalMs || intervalMs < 1) {
      (window as unknown as { __orkworksCaptureRendererHealth?: unknown }).__orkworksCaptureRendererHealth = undefined;
      return;
    }
    const deps = {
      panelCountProvider: () => {
        const api = dockviewApiRef.current;
        if (!api) return 0;
        try { return api.size; } catch { return 0; }
      },
      liveTerminalCountProvider: () => getLiveTerminalCount(),
      liveTerminalIdsProvider: () => getLiveTerminalIds(),
    };
    const timer = window.setInterval(() => {
      const sample = captureRendererHealth(deps);
      // eslint-disable-next-line no-console
      console.info("[orkworks:health]", sample);
    }, intervalMs);
    (window as unknown as { __orkworksCaptureRendererHealth?: () => RendererHealthSample }).__orkworksCaptureRendererHealth =
      () => captureRendererHealth(deps);
    return () => {
      window.clearInterval(timer);
      (window as unknown as { __orkworksCaptureRendererHealth?: unknown }).__orkworksCaptureRendererHealth = undefined;
    };
  }, [settings?.debug?.rendererHealthLogMs, dockviewApiRef]);
```

- [ ] **Step 5: Run the wiring test + whole renderer suite + type-check**

Run from `apps/desktop/`:

```bash
npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: type-check clean; all tests pass.

- [ ] **Step 6: Manually verify the probe end-to-end**

This is the only manual verification step in the plan. Run from `apps/desktop/`:

```bash
pnpm dev
```

In a running app, open Settings → Debug, set `rendererHealthLogMs` to `5000`, save, and watch the renderer DevTools console. Confirm a `[orkworks:health]` sample appears every 5s with `liveTerminalCount` and (in Electron) `usedJSHeapSize` populated. Open DevTools console and run `window.__orkworksCaptureRendererHealth()` to confirm an on-demand sample is returned. Set `rendererHealthLogMs` back to `0` and confirm the interval logs stop.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src/appSettingsTypes.ts apps/desktop/electron/settingsMemory.ts apps/desktop/src/App.tsx apps/desktop/tests/appRendererHealthWiring.test.ts
git commit -m "feat(desktop): opt-in rendererHealthLogMs + DevTools capture hook (#247)"
```

---

### Task 5: Verification and PR

**Files:**
- No new files; this task compiles the verification evidence required by `verification-before-completion`.

- [ ] **Step 1: Full validation from `apps/desktop/`**

Run from `apps/desktop/`:

```bash
npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: type-check clean; all tests pass.

- [ ] **Step 2: Doc currency check from repo root**

```bash
bash .claude/hooks/doc-check.sh
```

Expected: no doc files flagged. (This plan touches `DebugSettings` but adds no new lifecycle vocabulary, ADR, or runtime dependency, so `AGENTS.md`/`docs/agents/domain-entities.md`/`README.md` need no edits.)

- [ ] **Step 3: Worktree currency check from repo root**

```bash
bash .claude/hooks/worktree-check.sh
```

Expected: only the branches/worktrees the user already owns are listed; the `fix-247-renderer-memory` branch is in flight with an open PR (Step 5).

- [ ] **Step 4: Push and open a PR**

```bash
git push -u origin fix-247-renderer-memory
```

Then open the PR via `gh pr create` with a body containing:

- A "Root-cause status" section stating the Phase 1 static mapping outcomes (the suspect surfaces listed in this plan's Architecture section) and that a heap snapshot is still required before any speculative `free terminal on session-switch` fix is attempted (ADR 0022 boundary).
- A "What this PR lands" section listing the three deliverables: `TerminalRegistry`, the disposed-guard, and the renderer-health probe.
- A "What this PR deliberately does not do" section listing the Out-of-scope bullets.
- A "Next step" section instructing the next reproducer run to set `DebugSettings.rendererHealthLogMs` high enough to observe growth and capture a V8 `Heap snapshot` from DevTools once the probe shows the heap climbing, before opening a follow-up issue with the snapshot-named retaining path.

- [ ] **Step 5: Run `/code-review`**

Per the root `AGENTS.md` review gate, code under `apps/desktop/src/` requires a `/code-review` run before merge. Trigger a **lightweight** review (this PR is bounded, no concurrency/protocol/schema/security-sensitive changes; just under the 8-code-file threshold). Address any findings inline or note in the PR description why each is intentional.

---

## Self-Review

**1. Spec coverage (#247 acceptance criteria):**

- "Reproduce or instrument renderer memory growth across repeated polling, session switches, terminal attachment/detachment, and historical replay." → Task 3 + Task 4 ship the instrumentation. ✓
- "Identify the retaining allocation path with a renderer heap/profile capture." → Defers to follow-up; Task 4 Step 4 manual verification step and Task 5 Step 4 PR body explicitly instruct the next reproducer to capture a V8 heap snapshot. This plan produces the evidence tool, not the snapshot itself. The plan is honest about this in its Goal line and the Out-of-scope section.
- "Add a regression test or deterministic diagnostic that demonstrates the fixed lifecycle." → Task 1 registry tests + Task 2 disposed-guard source-pattern test. ✓
- "Keep renderer memory bounded during an extended session with normal polling and terminal use." → Cannot be verified in this PR's scope; deferred until the retaining path is identified.
- "Verify UI responsiveness no longer degrades into periodic stalls." → Same as above; deferred.

Two of the five acceptance criteria are intentionally not closed by this PR because the systematic-debugging skill forbids a speculative fix without root-cause evidence. The PR description names these as out-of-scope and proposes a follow-up once the heap snapshot is captured.

**2. Placeholder scan:** No "TBD", "appropriate error handling", "implement later", "similar to Task N". Each code step shows the actual code.

**3. Type consistency:**
- `TerminalRegistry<T>` API is identical between Task 1 (definition) and Task 2 (consumption): `get`/`set`/`remove`/`prune`/`isDisposed`/`liveIds`/`size`.
- `getLiveTerminalCount`/`getLiveTerminalIds` are introduced in Task 2 Step 3 step 8 and consumed in Task 4 Step 4 (`liveTerminalCountProvider: () => getLiveTerminalCount()`).
- `RendererHealthSample.panelCount` is added in Task 3 Step 4 along with the matching test.
- `DebugSettings.rendererHealthLogMs` is the field name used in Task 4 Step 3 (settingsMemory), Step 4 (App), and the wiring test in Step 1.
- Source-pattern test imports use `.ts` extensions matching the existing `terminalDetachSource.test.ts` pattern.