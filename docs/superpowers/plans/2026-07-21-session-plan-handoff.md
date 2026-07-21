# Session Plan Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a session safely offer its repo-relative Markdown plan for opening in the user's configured OS handler.

**Architecture:** Session metadata already persists the optional `planPath` supplied by the attention endpoint. The sidecar resolves that path from the active workspace and exposes only a boolean projection plus a validated-path handoff endpoint. Electron owns the OS `shell.openPath` call behind a narrow session-ID-only preload API; the renderer only renders the resulting action.

**Tech Stack:** Rust/Axum/Serde sidecar; Electron IPC; React/TypeScript; Rust and Node built-in test runners.

## Global Constraints

- Accept only repo-relative, existing regular Markdown files; canonicalize the workspace and candidate before containment checks.
- The renderer must never receive, construct, or submit a filesystem path.
- Electron and renderer remain separate TypeScript boundaries; duplicate bridge contract types rather than cross-importing.
- Do not add a review queue, Markdown viewer, discovery watcher, or review-state model.

---

### Task 1: Validate and project a reported plan path

**Files:**
- Create: `crates/orkworksd/src/plan_handoff.rs`
- Modify: `crates/orkworksd/src/session_types.rs`, `crates/orkworksd/src/session_view.rs`, `crates/orkworksd/src/http/session_handlers.rs`, `crates/orkworksd/src/main.rs`
- Test: inline Rust tests in `plan_handoff.rs` and `session_view.rs`

**Interfaces:**
- Produces: `resolve_openable_plan(workspace_root: &Path, relative_path: &str) -> Result<PathBuf, String>` and `SessionInfo.has_openable_plan: Option<bool>` serialized as `hasOpenablePlan`.

- [ ] **Step 1: Write failing sidecar tests** for a valid `.md` path, absolute path, `..` escape, missing file, non-Markdown file, and a symlink escaping the workspace.
- [ ] **Step 2: Run the focused test** with `cargo test --manifest-path crates/orkworksd/Cargo.toml plan_handoff`; confirm the missing resolver/projection fails.
- [ ] **Step 3: Implement the smallest resolver**: reject absolute input, canonicalize workspace and candidate, require candidate containment, regular-file status, and a case-insensitive `.md` extension.
- [ ] **Step 4: Project openability into `SessionInfo`** from metadata using the active workspace root; keep `None` for creation/resume responses that cannot safely evaluate it.
- [ ] **Step 5: Run the focused tests** and confirm they pass.

### Task 2: Add the validated open-plan HTTP and Electron handoff

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs`, `crates/orkworksd/src/main.rs`
- Modify: `apps/desktop/electron/main.ts`, `apps/desktop/electron/preload.ts`, `apps/desktop/src/orkworksWindow.d.ts`
- Test: inline Rust handler tests; `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- Consumes: `resolve_openable_plan`.
- Produces: `POST /sessions/:id/open-plan` returning `{ "path": "<canonical path>" }`; `window.orkworks.openPlan(sessionId: string): Promise<{ error?: string }>`.

- [ ] **Step 1: Write failing tests** proving the handler returns a canonical path only for a known session with a valid `planPath`, otherwise returns a non-success response; prove the preload accepts only a session ID and Electron calls `shell.openPath` with the returned path.
- [ ] **Step 2: Run the focused Rust and Node tests**; confirm they fail for the absent route and bridge.
- [ ] **Step 3: Implement the route and handler**: read metadata for the requested session, resolve against the selected workspace root immediately before responding, and never mutate session state.
- [ ] **Step 4: Implement the narrow IPC bridge**: Electron posts the session ID, passes only the validated response path to `shell.openPath`, and returns a readable error for HTTP or OS-handler failures.
- [ ] **Step 5: Run the focused tests** and confirm they pass.

### Task 3: Surface Open plan in session details

**Files:**
- Modify: `apps/desktop/src/api.ts`, `apps/desktop/src/components/SessionDetailPanel.tsx`
- Test: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- Consumes: `SessionInfo.hasOpenablePlan` and `window.orkworks.openPlan(sessionId)`.
- Produces: an **Open plan** button shown only when `hasOpenablePlan === true`.

- [ ] **Step 1: Write a failing renderer-source test** that checks `hasOpenablePlan` is represented in `SessionInfo`, the button is conditionally rendered, and its handler passes `active.id` to the bridge.
- [ ] **Step 2: Run the Node test**; confirm it fails because the API field and action are absent.
- [ ] **Step 3: Add the API field and action**: call the bridge with the active session ID alone; surface a non-empty returned error through the existing toast mechanism.
- [ ] **Step 4: Run the focused Node test** and confirm it passes.

### Task 4: Verify the complete safety boundary

**Files:**
- Modify only files required by failures from the commands below.

- [ ] **Step 1: Run Rust formatting and tests**: `cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check` then `cargo test --manifest-path crates/orkworksd/Cargo.toml`.
- [ ] **Step 2: Run desktop validation**: `cd apps/desktop && npx tsc --noEmit` then `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`.
- [ ] **Step 3: Inspect the diff** with `git diff --check` and verify no renderer file contains a filesystem path argument for plan opening.
- [ ] **Step 4: Commit** using `git add crates/orkworksd apps/desktop docs/superpowers/plans/2026-07-21-session-plan-handoff.md && git commit -m "feat: open session plans safely"`.
