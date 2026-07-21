# Session Plan Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a session that needs user attention safely open its reported Markdown plan in the operating system's configured handler.

**Architecture:** An agent supplies `planPath` in the existing attention report. The Rust sidecar persists that path atomically with the attention update, derives `hasOpenablePlan` without exposing the path to the renderer, and validates it again for an Electron-main handoff endpoint. The selected session's Details panel calls a session-ID-only preload bridge.

**Tech Stack:** Rust/Axum/serde, Electron IPC and `shell.openPath`, React/TypeScript, Node built-in test runner.

## Global constraints

- No queue, inbox, digest, automatic discovery, document renderer/editor, review state, or revision tracking.
- The existing `needs_you` session state is the only review prompt.
- Accept a workspace-relative `.md` extension case-insensitively; reject absolute paths, missing/non-file paths, `..` escapes, and symlink escapes.
- Canonicalize workspace and target, check containment, then repeat validation immediately before opening.
- The renderer sends only a session ID. It never receives or sends an absolute plan path.
- Do not add dependencies or cross the `electron/` ↔ `src/` import boundary.
- The plan action is the selected session's sole Details action, preserving the single-active-context principle.

## File map

| File | Change |
| --- | --- |
| `crates/orkworksd/src/metadata.rs` | Persist `plan_path` and atomically apply three-state updates. |
| `crates/orkworksd/src/plan_handoff.rs` | New pure canonical-path validator. |
| `crates/orkworksd/src/session_types.rs`, `session_view.rs` | Project `hasOpenablePlan`, never raw paths. |
| `crates/orkworksd/src/http/session_handlers.rs`, `main.rs` | Accept `planPath` and add the open-plan route. |
| `apps/desktop/electron/planOpener.ts` | New testable Electron-main sidecar/OS handoff helper. |
| `apps/desktop/electron/main.ts`, `preload.ts`, `src/orkworksWindow.d.ts` | Session-ID-only IPC bridge. |
| `apps/desktop/src/api.ts`, `labels.ts`, `components/SessionDetailPanel.tsx` | Project and render the one plan action. |
| Rust/desktop tests and `docs/agents/{domain-entities,architecture}.md` | Pin behavior and document the contract. |

### Task 1: Add the agent report and persisted metadata contract

**Files:**

- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Test: unit tests in both files

**Interfaces:**

- Produces `SessionMetadata.plan_path: Option<String>`.
- Produces `metadata::PlanPathUpdate::{Unchanged, Clear, Set(String)}`.
- Extends `POST /sessions/:id/attention` with `planPath`.

- [ ] **Step 1: Write failing parsing and merge tests**

Add tests that deserialize all three request states and prove one attention merge sets, replaces, clears, or preserves the stored value as appropriate. Retain the existing user-priority test and prove it does not mutate `plan_path`.

```rust
let set: AttentionReportRequest = serde_json::from_str(
    r#"{"status":"waiting_for_input","planPath":"docs/plan.md"}"#,
).unwrap();
assert_eq!(set.plan_path, metadata::PlanPathUpdate::Set("docs/plan.md".into()));

let clear: AttentionReportRequest = serde_json::from_str(
    r#"{"status":"waiting_for_input","planPath":null}"#,
).unwrap();
assert_eq!(clear.plan_path, metadata::PlanPathUpdate::Clear);
```

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml attention
```

Expected: compile or assertion failure because the metadata field and update type do not exist.

- [ ] **Step 3: Implement the minimal atomic contract**

Add `plan_path` to `SessionMetadata` using `#[serde(rename = "planPath", skip_serializing_if = "Option::is_none")]`. Implement a defaulted `PlanPathUpdate` deserializer: omitted uses `Unchanged`; a present `null` uses `Clear`; a string uses `Set`. Pass `&req.plan_path` to `merge_agent_attention_signal` and apply it to `meta.plan_path` before the existing single `try_write_session(&meta)` call:

```rust
match plan_path {
    PlanPathUpdate::Unchanged => {}
    PlanPathUpdate::Clear => meta.plan_path = None,
    PlanPathUpdate::Set(path) => meta.plan_path = Some(path.clone()),
}
```

Add `plan_path: None` to every explicit `SessionMetadata` literal and fixture. Do not update in-memory state on a persist failure.

- [ ] **Step 4: Re-run focused tests**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml attention
```

Expected: all existing and new attention tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/metadata.rs crates/orkworksd/src/http/session_handlers.rs
git commit -m "feat(sidecar): persist session plan paths"
```

### Task 2: Validate paths and expose only openability

**Files:**

- Create: `crates/orkworksd/src/plan_handoff.rs`
- Modify: `crates/orkworksd/src/session_types.rs`
- Modify: `crates/orkworksd/src/session_view.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `plan_handoff.rs`, `session_view.rs`, and handler unit tests

**Interfaces:**

```rust
pub(crate) fn resolve_openable_plan(
    workspace_root: &std::path::Path,
    relative_path: &str,
) -> Result<std::path::PathBuf, PlanHandoffError>;
```

- Produces `SessionInfo.has_openable_plan: Option<bool>`, serialized as `hasOpenablePlan`.
- Produces `POST /sessions/:id/open-plan -> { path: String }`.

- [ ] **Step 1: Write failing resolver, projection, and handler tests**

Use `tempfile::tempdir()` to accept `docs/plan.MD`, reject absolute, `../outside.md`, absent, directory, and `.txt` targets, and on Unix reject an in-workspace symlink to outside. Assert only an approved target serializes `"hasOpenablePlan":true`. Assert endpoint outcomes: 404 unknown session, 409 missing/unsafe plan, 200 canonical valid path.

```rust
assert!(resolve_openable_plan(workspace.path(), "docs/plan.MD").is_ok());
assert!(resolve_openable_plan(workspace.path(), "../outside.md").is_err());
assert!(resolve_openable_plan(workspace.path(), "docs/plan.txt").is_err());
```

- [ ] **Step 2: Run tests and confirm they fail**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml plan_handoff
```

Expected: failure because the module, projection, and route are absent.

- [ ] **Step 3: Implement safe validation, projection, and route**

Create `plan_handoff.rs`. Reject `Path::is_absolute()`; canonicalize the workspace root and `workspace_root.join(relative_path)`; require `candidate.starts_with(&canonical_workspace)`, `candidate.is_file()`, and `candidate.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"))`. Return only the canonical candidate.

Add `has_openable_plan` to `SessionInfo` with:

```rust
#[serde(rename = "hasOpenablePlan", skip_serializing_if = "Option::is_none")]
pub(crate) has_openable_plan: Option<bool>,
```

Derive it from persisted metadata in both live and remembered projections; do not serialize `plan_path`.

Implement `open_session_plan`: obtain and clone workspace root plus the session's persisted `plan_path` under existing locks, release locks, call `resolve_openable_plan` immediately before responding, and return `Json(OpenPlanResponse { path: canonical_path })`. Return 409 for no workspace/no valid plan and 404 for no session. Register `mod plan_handoff;` and this route in both router constructors:

```rust
.route("/sessions/:id/open-plan", post(open_session_plan))
```

- [ ] **Step 4: Run the focused Rust tests**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml plan_handoff
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml open_session_plan
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml session_view
```

Expected: all pass; symlink coverage is Unix-gated.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/plan_handoff.rs crates/orkworksd/src/main.rs crates/orkworksd/src/session_types.rs crates/orkworksd/src/session_view.rs crates/orkworksd/src/http/session_handlers.rs
git commit -m "feat(sidecar): validate session plan handoff"
```

### Task 3: Add a narrow Electron-main handoff

**Files:**

- Create: `apps/desktop/electron/planOpener.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Test: `apps/desktop/tests/planOpener.test.ts`

**Interfaces:**

```ts
export async function openSessionPlan(
  baseUrl: string,
  sessionId: string,
  fetchImpl: typeof fetch,
  openPath: (path: string) => Promise<string>,
): Promise<void>;
```

- Produces `window.orkworks.openPlan(sessionId): Promise<void>`.

- [ ] **Step 1: Write a failing injected-dependency helper test**

Assert the helper POSTs only to `/sessions/<encoded-id>/open-plan`, calls the injected opener with the response path, rejects for non-OK sidecar responses, and rejects a non-empty `shell.openPath` error.

```ts
await openSessionPlan("http://127.0.0.1:4444", "session 1", fetchImpl, openPath);
assert.deepEqual(calls, ["http://127.0.0.1:4444/sessions/session%201/open-plan"]);
assert.deepEqual(opened, ["/workspace/docs/plan.md"]);
```

- [ ] **Step 2: Run test and confirm it fails**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/planOpener.test.ts
```

Expected: module-not-found failure.

- [ ] **Step 3: Implement helper and IPC/preload declarations**

The helper POSTs the encoded session ID, parses `{ path: string }` only on success, calls its injected opener, and throws a stable `open plan failed: <status>` error or the non-empty opener error. It takes no renderer path.

In `main.ts`, import `shell` and the helper. Register `ipcMain.handle("open-plan", ...)`, reject a non-string/empty session ID, await `portPromise`, then call the helper with the loopback base URL and `(filePath) => shell.openPath(filePath)`. In `preload.ts`, expose only:

```ts
openPlan: (sessionId: string): Promise<void> => ipcRenderer.invoke("open-plan", sessionId),
```

Duplicate that signature in `orkworksWindow.d.ts`.

- [ ] **Step 4: Verify helper and TypeScript**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/planOpener.test.ts
cd apps/desktop && npx tsc --noEmit
```

Expected: helper test and type-check pass.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/electron/planOpener.ts apps/desktop/electron/main.ts apps/desktop/electron/preload.ts apps/desktop/src/orkworksWindow.d.ts apps/desktop/tests/planOpener.test.ts
git commit -m "feat(desktop): open session plans through Electron"
```

### Task 4: Show one selected-session plan action

**Files:**

- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/labels.ts`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/tests/api.test.ts`
- Modify: `apps/desktop/tests/dockview.test.ts`
- Modify: `apps/desktop/tests/labels.test.ts`

**Interfaces:**

- Consumes `SessionInfo.hasOpenablePlan?: boolean`.
- Produces `detailActionZone(...): { kind: "plan" }` for a `needs_you` session with an openable plan.

- [ ] **Step 1: Write failing API and Details action tests**

Add a `SessionInfo` test sample with `hasOpenablePlan: true`. Extend Details tests to require the `plan` variant and `window.orkworks.openPlan(active.id)`, and assert `detailActionZone` selects the plan variant before buttons/resume so there remains one action.

```ts
assert.deepEqual(
  detailActionZone({ ...base, attention: "needs_you", hasOpenablePlan: true }, "needs-you"),
  { kind: "plan" },
);
```

- [ ] **Step 2: Run tests and confirm they fail**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/dockview.test.ts tests/labels.test.ts
```

Expected: missing DTO field and plan-action failures.

- [ ] **Step 3: Implement the focused action**

Add `hasOpenablePlan?: boolean` to `SessionInfo`. In `detailActionZone`, return `{ kind: "plan" }` first when `session.attention === "needs_you" && session.hasOpenablePlan === true`. Add this action branch in `SessionDetailPanel.tsx` using existing button styles:

```tsx
{actionZone.kind === "plan" && (
  <button className="detail-button detail-button--primary" type="button"
    onClick={() => void window.orkworks.openPlan(active.id).catch((error: unknown) => {
      pushToast("error", error instanceof Error ? error.message : "Couldn’t open plan.");
    })}>
    Open plan
  </button>
)}
```

Do not add a new panel, preview, raw path, or second review action.

- [ ] **Step 4: Verify frontend behavior**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/dockview.test.ts tests/labels.test.ts
cd apps/desktop && npx tsc --noEmit
```

Expected: tests pass and the two independent TypeScript projects pass.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/api.ts apps/desktop/src/labels.ts apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/tests/api.test.ts apps/desktop/tests/dockview.test.ts apps/desktop/tests/labels.test.ts
git commit -m "feat(desktop): surface session plan action"
```

### Task 5: Document and verify the integrated slice

**Files:**

- Modify: `docs/agents/domain-entities.md`
- Modify: `docs/agents/architecture.md`

- [ ] **Step 1: Update required documentation**

In `domain-entities.md`, document persisted advisory `plan_path` and derived/non-persisted `hasOpenablePlan`; state that only workspace-contained Markdown files become openable. In `architecture.md`, update the attention payload to `{ status, message?, planPath? }` with set/clear/omitted semantics, add `POST /sessions/:id/open-plan`, and document the session-ID-only `openPlan` preload bridge and Electron `shell.openPath` handoff.

- [ ] **Step 2: Run full verification**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
cd apps/desktop && npx tsc --noEmit
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
rtk git diff --check
```

Expected: Rust and desktop tests pass, TypeScript passes, doc/worktree checks identify no task-owned actionable issue, and whitespace is clean.

- [ ] **Step 3: Commit documentation**

```bash
git add docs/agents/domain-entities.md docs/agents/architecture.md
git commit -m "docs: document session plan handoff"
```

## Plan self-review

- Task 1 covers report, persistence, and atomic set/replace/clear/omit semantics.
- Task 2 covers canonical validation, symlink protection, derived availability, repeat validation, and the sidecar endpoint.
- Task 3 covers the Electron-only OS opening boundary.
- Task 4 covers the selected-session, single-action UI and error feedback.
- Task 5 covers the required docs and final verification.
- No task creates a queue, reader, editor, or second context surface; field names are consistent: HTTP `planPath`, metadata `plan_path`, projection `hasOpenablePlan`, preload `openPlan(sessionId)`.
