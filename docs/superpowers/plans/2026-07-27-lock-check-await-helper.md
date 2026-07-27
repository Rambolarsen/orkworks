# Lock-check await helper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract one shared primitive for the integration handler’s lock-check-drop-await-relock flow without changing existing 409 semantics or widening scope into unrelated `spawn_blocking` readers.

**Architecture:** Keep `run_integration_action` as the public integration entrypoint, but carve out a private helper that snapshots identity, probes outside `!Send` guards, revalidates, then hands a still-valid workspace guard + harness clone to the caller. Keep runtime `spawn_blocking` lock readers untouched and document that as an intentional boundary in ADR + tests.

**Tech Stack:** Rust, Axum handlers, `std::sync::{Mutex,RwLock}`, Tokio async tests, cargo test, clippy.

## Global Constraints

- Do not change `409 Conflict` semantics or identity-key choice in `run_integration_action`.
- Keep scope to the integration TOCTOU pattern; do not refactor unrelated runtime `spawn_blocking` lock readers.
- Reuse the primitive across the three integration handler entrypoints via `run_integration_action`.
- Add an ADR under `docs/adr/` documenting primitive shape and rejected alternatives.
- Add independent tests that pin primitive behavior (not only end-to-end handler assertions).
- Keep existing crate behavior stable and run existing checks (`cargo test`, `cargo clippy`).

---

### Task 1: Record architecture + complete call-site survey

**Files:**
- Create: `docs/adr/0029-integration-lock-check-await-helper.md`
- Modify: `docs/adr/README.md`
- Modify: `docs/superpowers/specs/2026-07-27-lock-check-await-helper-design.md`

**Interfaces:**
- Consumes: issue context from `#235` and PR `#229` review thread.
- Produces: ADR decision text used by code tasks to enforce helper boundary.

- [ ] **Step 1: Write the failing documentation test (survey completeness check)**

Add this checklist block near the top of the ADR draft and leave one unchecked item first:

```markdown
## Call-site survey checklist

- [x] `http/integration_handlers.rs::run_integration_action` (target pattern)
- [ ] `spawn_blocking` lock readers listed and marked out-of-scope
```

- [ ] **Step 2: Run a docs grep to verify the checklist is still incomplete**

Run:  
`rg "\- \[ \] .*spawn_blocking" docs/adr/0029-integration-lock-check-await-helper.md`

Expected: one unchecked line is returned.

- [ ] **Step 3: Fill in the final ADR content and mark checklist complete**

Replace the ADR body with concrete content like:

```markdown
# Integration lock-check-drop-await-relock helper

- Status: accepted
- Date: 2026-07-27
- Deciders: Copilot CLI

## Context
Issue #235 (split from PR #229 review) flagged drift risk in the integration TOCTOU choreography.

## Decision
Extract a private helper from `run_integration_action` that:
1. snapshots workspace path + harness definition identity,
2. probes outside `!Send` guards,
3. revalidates both identities,
4. executes caller logic under a still-valid workspace guard.

## Call-site survey checklist
- [x] `http/integration_handlers.rs::run_integration_action` (target pattern)
- [x] `spawn_blocking` lock readers in `runtime/terminal_http.rs`, `runtime/session_runtime.rs`, `http/harness_handlers.rs`, `http/session_handlers.rs`, `http/provider_handlers.rs`, `runtime/peon_runtime.rs`, `runtime/terminal_runtime.rs` (out-of-scope by design)

## Rejected alternatives
- Broad helper over all `std::sync` + async adjacency shapes (mixes distinct safety problems)
- Keep ad hoc logic (drift risk)

## Consequences
Centralizes the only lock-drop-await-revalidate path while preserving current 409 conflict behavior.
```

- [ ] **Step 4: Run docs grep to verify checklist completion**

Run:  
`rg "\- \[ \] " docs/adr/0029-integration-lock-check-await-helper.md`

Expected: no output.

- [ ] **Step 5: Index the new ADR**

Add this row at the end of `docs/adr/README.md`:

```markdown
| [0029](./0029-integration-lock-check-await-helper.md) | Integration lock-check-drop-await-relock helper | accepted |
```

- [ ] **Step 6: Commit**

```bash
git add docs/adr/0029-integration-lock-check-await-helper.md docs/adr/README.md docs/superpowers/specs/2026-07-27-lock-check-await-helper-design.md
git commit -m "docs: add ADR for integration lock-check await helper" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 2: Extract primitive with independent unit coverage

**Files:**
- Modify: `crates/orkworksd/src/http/integration_handlers.rs`

**Interfaces:**
- Consumes: `AppState`, `ResolvedHarness`, `IntegrationError`, existing `integration_error_response`.
- Produces:
  - `async fn with_revalidated_integration_target<R>(state: &Arc<AppState>, harness_id: &str, action: impl FnOnce(&ResolvedHarness, &crate::WorkspaceState, Option<&crate::harness::integration::DetectedTool>) -> R) -> Result<R, axum::response::Response>`

- [ ] **Step 1: Write failing primitive-focused tests (no handler action body)**

Inside `#[cfg(test)] mod tests` in `integration_handlers.rs`, add tests that call the new helper directly:

```rust
#[tokio::test]
async fn with_revalidated_integration_target_rejects_workspace_switch() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    let result = with_revalidated_integration_target(&state, "claude-code", |_h, _ws, _tool| ())
        .await;
    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
}

#[tokio::test]
async fn with_revalidated_integration_target_rejects_harness_definition_change() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    let result = with_revalidated_integration_target(&state, "copilot", |_h, _ws, _tool| ())
        .await;
    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
}
```

- [ ] **Step 2: Run targeted tests to confirm failure before implementation**

Run:  
`cargo test --manifest-path crates/orkworksd/Cargo.toml with_revalidated_integration_target_ -- --nocapture`

Expected: FAIL (helper symbol missing / tests not yet wired).

- [ ] **Step 3: Implement minimal helper**

Add helper code above `run_integration_action`:

```rust
async fn with_revalidated_integration_target<R>(
    state: &Arc<AppState>,
    harness_id: &str,
    action: impl FnOnce(
        &ResolvedHarness,
        &crate::WorkspaceState,
        Option<&crate::harness::integration::DetectedTool>,
    ) -> R,
) -> Result<R, axum::response::Response> {
    let workspace_path_at_start = {
        let ws_guard = state.workspace.lock().unwrap();
        let Some(ws) = ws_guard.as_ref() else {
            return Err(integration_error_response(IntegrationError::NoWorkspace));
        };
        ws.path.clone()
    };

    let harness = {
        let registry = state.harness_catalog.read().expect("harness catalog lock poisoned");
        let Some(harness) = registry.get(harness_id) else {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: format!("unknown harness id \"{harness_id}\"") }),
            ).into_response());
        };
        harness.clone()
    };

    let detected_tool = crate::harness::detect::resolve_tool_gate(
        &state.integration_probe_cache,
        &harness.definition.id,
        &harness.launch_command(),
        harness.definition.min_version.as_ref(),
    ).await;

    {
        let registry = state.harness_catalog.read().expect("harness catalog lock poisoned");
        match registry.get(harness_id) {
            Some(current) if current.definition == harness.definition => {}
            _ => {
                return Err((
                    StatusCode::CONFLICT,
                    Json(ErrorResponse { error: "harness definition changed during this request; retry".into() }),
                ).into_response());
            }
        }
    }

    let ws_guard = state.workspace.lock().unwrap();
    let Some(ref ws) = *ws_guard else {
        return Err(integration_error_response(IntegrationError::NoWorkspace));
    };
    if ws.path != workspace_path_at_start {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse { error: "workspace changed during this request; retry".into() }),
        ).into_response());
    }

    Ok(action(&harness, ws, detected_tool.as_ref()))
}
```

- [ ] **Step 4: Update tests to assert success path shape**

Add:

```rust
#[tokio::test]
async fn with_revalidated_integration_target_returns_harness_and_probe_data() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    let result = with_revalidated_integration_target(&state, "claude-code", |h, _ws, _tool| {
        h.definition.id.clone()
    })
    .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "claude-code");
}
```

- [ ] **Step 5: Run targeted tests to verify pass**

Run:  
`cargo test --manifest-path crates/orkworksd/Cargo.toml with_revalidated_integration_target_ repeated_status_polls_reuse_one_version_probe_within_ttl workspace_switch_forces_a_fresh_version_probe harness_edit_forces_a_fresh_version_probe -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/http/integration_handlers.rs
git commit -m "refactor: extract integration probe/revalidate helper" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 3: Rewire run_integration_action to consume the helper without behavior drift

**Files:**
- Modify: `crates/orkworksd/src/http/integration_handlers.rs`

**Interfaces:**
- Consumes: `with_revalidated_integration_target(...) -> Result<R, Response>`.
- Produces: unchanged public handlers:
  - `pub(crate) async fn get_integration_status(...) -> impl IntoResponse`
  - `pub(crate) async fn install_integration(...) -> impl IntoResponse`
  - `pub(crate) async fn uninstall_integration(...) -> impl IntoResponse`

- [ ] **Step 1: Add failing regression assertions for conflict payload stability**

In existing tests `a_workspace_switch_during_the_probe_is_rejected_instead_of_targeting_the_new_one` and `a_harness_definition_change_during_the_probe_is_rejected_instead_of_using_stale_data`, assert exact payload strings:

```rust
assert_eq!(payload["error"], "workspace changed during this request; retry");
assert_eq!(payload["error"], "harness definition changed during this request; retry");
```

- [ ] **Step 2: Run those two tests first (baseline)**

Run:  
`cargo test --manifest-path crates/orkworksd/Cargo.toml a_workspace_switch_during_the_probe_is_rejected_instead_of_targeting_the_new_one a_harness_definition_change_during_the_probe_is_rejected_instead_of_using_stale_data -- --nocapture`

Expected: PASS before refactor.

- [ ] **Step 3: Rewire run_integration_action**

Replace the inline snapshot/probe/revalidate block with helper usage:

```rust
match with_revalidated_integration_target(state, harness_id, |harness, ws, detected_tool| {
    let ctx = IntegrationContext {
        workspace: &ws.path,
        workspace_metadata: Some(&ws.metadata),
        orkworks_root: &orkworks_root,
        enabled: true,
        detected_tool,
        reporter_assets: &reporter_assets,
    };

    match action(harness, &ctx) {
        Ok(status) => Json(status).into_response(),
        Err(error) => integration_error_response(error),
    }
})
.await
{
    Ok(response) => response,
    Err(response) => response,
}
```

- [ ] **Step 4: Run focused integration handler tests**

Run exact unit tests:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml \
  repeated_status_polls_reuse_one_version_probe_within_ttl \
  workspace_switch_forces_a_fresh_version_probe \
  harness_edit_forces_a_fresh_version_probe \
  a_slow_version_probe_does_not_block_a_concurrent_workspace_request \
  a_workspace_switch_during_the_probe_is_rejected_instead_of_targeting_the_new_one \
  a_harness_definition_change_during_the_probe_is_rejected_instead_of_using_stale_data \
  -- --nocapture
```

Expected: PASS with unchanged payload/status behavior.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/http/integration_handlers.rs
git commit -m "refactor: route integration action through shared guard helper" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 4: Full verification + completion checks

**Files:**
- Modify (if needed): `AGENTS.md`, `README.md` (only if process/docs drift is discovered)
- No planned code file creation

**Interfaces:**
- Consumes: outputs from Tasks 1-3.
- Produces: verified branch ready for review/PR.

- [ ] **Step 1: Run crate-wide tests**

Run:  
`cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: PASS.

- [ ] **Step 2: Run clippy on the crate**

Run:  
`cargo clippy --manifest-path crates/orkworksd/Cargo.toml --all-targets -- -D warnings`

Expected: PASS.

- [ ] **Step 3: Run doc/worktree currency hooks**

Run:

```bash
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: no blocking findings for this branch.

- [ ] **Step 4: Capture review notes for issue acceptance criteria**

Add a short PR/issue note including:

```markdown
- Survey completed for lock/await/revalidate and spawn_blocking-adjacent sites.
- ADR 0029 records primitive choice and rejected broader alternatives.
- Primitive extracted and covered by direct helper tests + existing handler regressions.
- `cargo test` and `cargo clippy -- -D warnings` clean.
```

- [ ] **Step 5: Commit any final doc-only deltas (if present)**

```bash
git add -A
git commit -m "docs: finalize lock-check helper validation notes" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```
