# Session List Git Scan Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the complete session history visible while making `/sessions` Git work proportional to unique working directories and preventing overlapping background polls.

**Architecture:** Extract the existing per-session Git projection into a request-local enrichment helper that caches `GitContext` by `cwd`, then call it from `list_sessions`. Extract the renderer's background polling lifecycle into a small single-flight scheduler so React can start an immediate poll and schedule the next one only after settlement.

**Tech Stack:** Rust 2024, Axum, git2, React 19, TypeScript 5.9, Node built-in test runner

## Global Constraints

- Preserve every live and historical session returned by `GET /sessions`.
- Preserve Git context freshness on each completed session poll; do not add a cross-request TTL cache.
- Preserve current Git fields, recommendations, and conflict warnings.
- Keep `apps/desktop/electron/` and `apps/desktop/src/` independent; do not add cross-boundary imports.
- Use pnpm for Node dependency and script operations.
- Do not add dependencies or create an ADR.
- Track implementation against GitHub issue #196.

---

## File Structure

- Modify `crates/orkworksd/src/http/session_handlers.rs`: add request-local Git-context enrichment and its regression test.
- Create `apps/desktop/src/sessionPolling.ts`: own the renderer's single-flight background polling lifecycle.
- Create `apps/desktop/tests/sessionPolling.test.ts`: prove slow refreshes do not overlap and cleanup prevents rescheduling.
- Modify `apps/desktop/src/App.tsx`: replace the interval effect with the polling helper.

### Task 1: Deduplicate Git detection by working directory

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs:1208-1224`
- Test: `crates/orkworksd/src/http/session_handlers.rs` test module

**Interfaces:**
- Consumes: `SessionInfo`, `git::GitContext`, `session_recommendation`, and `git::detect`.
- Produces: `fn enrich_sessions_with_git_context<F>(infos: &mut [SessionInfo], detect_git: F) where F: FnMut(&std::path::Path) -> git::GitContext`.

- [ ] **Step 1: Write the failing request-local deduplication test**

Add this test to the existing `#[cfg(test)] mod tests` in `session_handlers.rs`:

```rust
#[test]
fn session_git_context_is_resolved_once_per_unique_cwd() {
    let shared = "/workspace/shared";
    let separate = "/workspace/separate";
    let mut infos = vec![
        test_session_info("one", "One", shared, "running", "now"),
        test_session_info("two", "Two", shared, "running", "now"),
        test_session_info("three", "Three", separate, "ended", "now"),
    ];
    let mut calls: HashMap<String, usize> = HashMap::new();

    enrich_sessions_with_git_context(&mut infos, |cwd| {
        *calls.entry(cwd.display().to_string()).or_default() += 1;
        git::GitContext {
            repo_root: Some(format!("{}/repo", cwd.display())),
            branch: Some("test-branch".into()),
            dirty: true,
            changed_files: 2,
            is_worktree: cwd == std::path::Path::new(separate),
        }
    });

    assert_eq!(calls.get(shared), Some(&1));
    assert_eq!(calls.get(separate), Some(&1));
    assert_eq!(calls.len(), 2);
    assert_eq!(infos[0].repo_root.as_deref(), Some("/workspace/shared/repo"));
    assert_eq!(infos[1].branch.as_deref(), Some("test-branch"));
    assert_eq!(infos[1].dirty, Some(true));
    assert_eq!(infos[1].changed_files, Some(2));
    assert_eq!(infos[2].is_worktree, Some(true));
    assert!(infos[0].recommendation.is_some());
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml \
  http::session_handlers::tests::session_git_context_is_resolved_once_per_unique_cwd \
  -- --exact
```

Expected: compilation fails because `enrich_sessions_with_git_context` does not exist.

- [ ] **Step 3: Add the minimal enrichment helper**

Add this helper immediately before `list_sessions`:

```rust
fn enrich_sessions_with_git_context<F>(infos: &mut [SessionInfo], mut detect_git: F)
where
    F: FnMut(&std::path::Path) -> git::GitContext,
{
    let mut cwd_counts: HashMap<String, usize> = HashMap::new();
    for info in infos.iter() {
        if info.status == "running" || info.status == "creating" {
            *cwd_counts.entry(info.cwd.clone()).or_default() += 1;
        }
    }

    let mut contexts: HashMap<String, git::GitContext> = HashMap::new();
    for info in infos {
        let ctx = contexts
            .entry(info.cwd.clone())
            .or_insert_with(|| detect_git(std::path::Path::new(&info.cwd)));
        let count = cwd_counts.get(&info.cwd).copied().unwrap_or(1);
        info.recommendation = session_recommendation(ctx, count);
        info.repo_root = ctx.repo_root.clone();
        info.branch = ctx.branch.clone();
        info.dirty = Some(ctx.dirty);
        info.changed_files = Some(ctx.changed_files);
        info.is_worktree = Some(ctx.is_worktree);
    }
}
```

Replace the existing `cwd_counts` and per-session `git::detect` loops near line 1208 with:

```rust
enrich_sessions_with_git_context(&mut infos, git::detect);
```

Leave `detect_conflicts(&infos)` and its assignment loop unchanged after this call.

- [ ] **Step 4: Run focused and module tests and verify GREEN**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml \
  http::session_handlers::tests::session_git_context_is_resolved_once_per_unique_cwd \
  -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml http::session_handlers::tests
```

Expected: the focused test passes, followed by all session-handler tests passing.

- [ ] **Step 5: Format and commit the sidecar change**

Run:

```bash
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
rtk git add crates/orkworksd/src/http/session_handlers.rs
rtk git commit -m "fix(sidecar): deduplicate session Git scans"
```

Expected: formatting is clean and the commit contains only the handler and test change.

### Task 2: Make renderer background polling single-flight

**Files:**
- Create: `apps/desktop/src/sessionPolling.ts`
- Create: `apps/desktop/tests/sessionPolling.test.ts`
- Modify: `apps/desktop/src/App.tsx:1-97`

**Interfaces:**
- Consumes: the existing `refreshSessions: () => Promise<void>` callback from `App.tsx`.
- Produces: `startSessionPolling(refresh, delayMs?, scheduler?): () => void` and `PollScheduler` for deterministic tests.

- [ ] **Step 1: Write failing polling lifecycle tests**

Create `apps/desktop/tests/sessionPolling.test.ts`:

```typescript
import test from "node:test";
import assert from "node:assert/strict";

import {
  startSessionPolling,
  type PollScheduler,
} from "../src/sessionPolling.ts";

function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

test("background session polls wait for the previous refresh to settle", async () => {
  const first = deferred();
  const scheduled: Array<() => void> = [];
  let refreshes = 0;
  const scheduler: PollScheduler = {
    set(callback, delayMs) {
      assert.equal(delayMs, 2_000);
      scheduled.push(callback);
      return callback;
    },
    clear() {},
  };

  const stop = startSessionPolling(async () => {
    refreshes += 1;
    await first.promise;
  }, 2_000, scheduler);

  await flushMicrotasks();
  assert.equal(refreshes, 1);
  assert.equal(scheduled.length, 0);

  first.resolve();
  await flushMicrotasks();
  assert.equal(scheduled.length, 1);

  scheduled.shift()!();
  await flushMicrotasks();
  assert.equal(refreshes, 2);
  stop();
});

test("stopping an unresolved poll prevents it from scheduling again", async () => {
  const first = deferred();
  const scheduled: Array<() => void> = [];
  const scheduler: PollScheduler = {
    set(callback) {
      scheduled.push(callback);
      return callback;
    },
    clear() {},
  };

  const stop = startSessionPolling(() => first.promise, 2_000, scheduler);
  stop();
  first.resolve();
  await flushMicrotasks();

  assert.equal(scheduled.length, 0);
});
```

- [ ] **Step 2: Run the polling test and verify RED**

Run from `apps/desktop`:

```bash
rtk node --experimental-strip-types --test tests/sessionPolling.test.ts
```

Expected: failure with `ERR_MODULE_NOT_FOUND` for `src/sessionPolling.ts`.

- [ ] **Step 3: Implement the minimal polling helper**

Create `apps/desktop/src/sessionPolling.ts`:

```typescript
export interface PollScheduler {
  set(callback: () => void, delayMs: number): unknown;
  clear(handle: unknown): void;
}

const browserScheduler: PollScheduler = {
  set: (callback, delayMs) => window.setTimeout(callback, delayMs),
  clear: (handle) => window.clearTimeout(handle as number),
};

export function startSessionPolling(
  refresh: () => Promise<void>,
  delayMs = 2_000,
  scheduler: PollScheduler = browserScheduler,
): () => void {
  let stopped = false;
  let timer: unknown;

  async function poll(): Promise<void> {
    try {
      await refresh();
    } catch {
      // Background refresh failures remain silent; retry on the next cycle.
    }
    if (!stopped) {
      timer = scheduler.set(() => void poll(), delayMs);
    }
  }

  void poll();
  return () => {
    stopped = true;
    if (timer !== undefined) scheduler.clear(timer);
  };
}
```

- [ ] **Step 4: Wire the helper into `App.tsx`**

Add the import:

```typescript
import { startSessionPolling } from "./sessionPolling";
```

Replace the current interval effect with:

```typescript
useEffect(() => {
  if (backendStatus !== "connected") return;
  return startSessionPolling(refreshSessions);
}, [backendStatus, refreshSessions]);
```

Do not change manual `refreshSessions()` calls used by create, resume, delete, or forget actions.

- [ ] **Step 5: Run focused tests and TypeScript checking and verify GREEN**

Run from `apps/desktop`:

```bash
rtk node --experimental-strip-types --test tests/sessionPolling.test.ts
rtk pnpm exec tsc --noEmit
```

Expected: two polling tests pass and TypeScript reports no errors.

- [ ] **Step 6: Commit the renderer change**

Run:

```bash
rtk git add apps/desktop/src/App.tsx \
  apps/desktop/src/sessionPolling.ts \
  apps/desktop/tests/sessionPolling.test.ts
rtk git commit -m "fix(desktop): serialize session polling"
```

Expected: the commit contains only the polling helper, its test, and the `App.tsx` integration.

### Task 3: Full verification and issue handoff

**Files:**
- Verify: all files changed by Tasks 1-2
- Update: GitHub issue #196 and PR description during publish handoff

**Interfaces:**
- Consumes: the completed sidecar and renderer changes.
- Produces: fresh build/test evidence and a review-ready branch.

- [ ] **Step 1: Run complete sidecar verification**

Run:

```bash
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
rtk cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings
```

Expected: formatting, all Rust tests, and Clippy pass without warnings.

- [ ] **Step 2: Run complete desktop verification**

Run from `apps/desktop`:

```bash
rtk pnpm exec tsc --noEmit
rtk node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
rtk pnpm build
```

Expected: TypeScript, all frontend tests, and the production desktop build pass.

- [ ] **Step 3: Run repository currency and diff checks**

Run from the repository root:

```bash
rtk git diff --check origin/main...HEAD
rtk bash .claude/hooks/doc-check.sh
rtk bash .claude/hooks/worktree-check.sh
rtk git status --short
```

Expected: no whitespace errors; the doc check reports no required doc updates; worktree warnings are reported but only this branch is acted upon; the working tree is clean.

- [ ] **Step 4: Perform the required lightweight code review**

Run `/code-review` at lightweight effort because the change touches code under both `apps/desktop/` and `crates/orkworksd/` but is bounded and does not change concurrency ownership, lifecycle architecture, schemas, protocols, migrations, or security boundaries.

Expected: address correctness findings or document why a finding is intentional before publishing.

- [ ] **Step 5: Publish handoff**

Update issue #196 with the verification summary, then use the repository publish workflow to push `session-list-git-scan` and open a draft PR. Include:

```text
Closes #196

Root cause: `/sessions` performed one full Git status scan per historical
session, and the renderer allowed slow interval polls to overlap.

Fix: reuse Git context per unique cwd within each request and schedule the next
background poll only after the current request settles.
```

Expected: the draft PR links issue #196, contains the design and implementation commits, and records test/review evidence.
