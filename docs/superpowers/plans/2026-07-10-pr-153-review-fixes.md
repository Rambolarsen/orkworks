# PR 153 Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make debug state injection temporary and non-blocking, while hardening its Electron IPC boundary.

**Architecture:** `list_sessions` remains responsible for deriving real capacity and state mutations; it will project debug-only response overlays only after that work. Metadata persistence will use a collected write list outside the session mutex. Electron main will use a small pure payload validator/path builder, enabling direct Node tests without launching Electron.

**Tech Stack:** Rust/Axum/Tokio, TypeScript/Electron, Node built-in test runner.

## Global Constraints

- Preserve the existing `apps/desktop/electron/` and `apps/desktop/src/` import boundary.
- Keep debug injection behind the persisted `Show debug metadata` setting.
- Debug overlays must be replaceable by the next non-debug runtime update and must never mutate real cap/latch state.
- Do not add dependencies or new injection scenarios.

---

### Task 1: Make Session Listing Derive Runtime State Before Debug Projection

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs:1014-1195`
- Test: `crates/orkworksd/src/http/session_handlers.rs` existing `list_sessions_*` tests

**Interfaces:**
- Consumes: `apply_debug_overlay_projection(&mut SessionInfo, Option<&DebugInjectionMetadata>, Option<&SessionMetadata>)`.
- Produces: `list_sessions` responses with debug-only `at_usage_limit` presentation that never changes `SessionHandle.at_usage_limit_latched` or provider state.

- [ ] **Step 1: Write the failing repeated-poll regression test**

```rust
#[tokio::test]
async fn list_sessions_debug_capped_overlay_does_not_latch_or_propagate_capacity() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    // Insert a capped debug session and an uncapped sibling, then call list_sessions twice.
    let first = list_session_json(&state).await;
    let second = list_session_json(&state).await;
    assert_eq!(find_session(&first, "debug-capped")["atUsageLimit"], true);
    assert_eq!(find_session(&second, "sibling")["atUsageLimit"], false);
    assert_eq!(state.sessions.lock().unwrap()["debug-capped"].at_usage_limit_latched, false);
    assert_eq!(opencode_provider_state(&state), "healthy");
    write_metadata_source(&state, "debug-capped", "process");
    let clearing_poll = list_session_json(&state).await;
    // After a non-debug runtime update clears the overlay, that same poll must
    // no longer present debug capacity state.
    assert_eq!(find_session(&clearing_poll, "debug-capped")["atUsageLimit"], false);
}
```

- [ ] **Step 2: Run the test to verify it fails because the overlay is latched as runtime capacity**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_debug_capped_overlay_does_not_latch_or_propagate_capacity`

Expected: FAIL because the second poll or provider state observes the debug overlay as a real cap.

- [ ] **Step 3: Move debug projection after all capacity/latch bookkeeping**

```rust
// First derive and persist only runtime capacity state.
for info in &mut infos {
    // Existing capping propagation and session-handle latch updates stay here.
}

// Then alter only the response copies with post-clear debug state. Do not use
// the pre-clear snapshot captured at the beginning of list_sessions.
for info in &mut infos {
    apply_debug_overlay_projection(
        info,
        post_clear_debug_by_session.get(&info.id),
        metadata_map.get(&info.id),
    );
}
```

- [ ] **Step 4: Collect metadata persistence while locked and write it after releasing `sessions`**

```rust
let metadata_to_write = {
    let mut sessions = state.sessions.lock().unwrap();
    // Clear superseded overlays and clone each changed metadata record.
    // Build post_clear_debug_by_session after clearing so response projection
    // cannot use a stale debug clone.
    changed_metadata
};

if let Some(ws) = state.workspace.lock().unwrap().as_ref() {
    for meta in metadata_to_write {
        ws.metadata.write_session(&meta);
    }
}
```

- [ ] **Step 5: Run the focused Rust test to verify it passes**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_debug_capped_overlay_does_not_latch_or_propagate_capacity`

Expected: PASS.

- [ ] **Step 6: Run adjacent session-handler regressions**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml running_capped_overlay_does_not_cap_sibling_live_sessions_or_provider_state && cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_clears_live_capped_even_when_ring_buffer_length_stays_flat`

Expected: PASS.

- [ ] **Step 7: Commit the Rust fix**

```bash
git add crates/orkworksd/src/http/session_handlers.rs
git commit -m "fix: isolate debug capacity overlays"
```

### Task 2: Validate Electron Debug-Injection IPC Payloads

**Files:**
- Modify: `apps/desktop/electron/main.ts:246-269`
- Create: `apps/desktop/electron/sessionStateInjectionIpc.ts`
- Create: `apps/desktop/tests/sessionStateInjectionIpc.test.ts`

**Interfaces:**
- Produces: `parseSessionStateInjectionPayload(payload: unknown): { sessionId: string; injectionId: string }`.
- Produces: `debugInjectionUrl(port: number, sessionId: string): string`.
- Consumes: validated identifiers in the existing `apply-session-state-injection` IPC handler.

- [ ] **Step 1: Write failing payload and URL tests**

```ts
test("rejects malformed debug injection payloads", () => {
  assert.throws(() => parseSessionStateInjectionPayload(null), /invalid/i);
  assert.throws(() => parseSessionStateInjectionPayload({ sessionId: "id" }), /invalid/i);
  assert.throws(() => parseSessionStateInjectionPayload({ sessionId: "", injectionId: "running-capped" }), /invalid/i);
});

test("encodes session ids in debug injection URLs", () => {
  assert.equal(
    debugInjectionUrl(4312, "session/with spaces"),
    "http://127.0.0.1:4312/sessions/session%2Fwith%20spaces/debug-injection",
  );
});
```

- [ ] **Step 2: Run the new test to verify it fails because the helper does not exist**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/sessionStateInjectionIpc.test.ts`

Expected: FAIL with an import/export error for the missing IPC helper.

- [ ] **Step 3: Implement the smallest pure validator and URL builder**

```ts
export function parseSessionStateInjectionPayload(payload: unknown) {
  if (!payload || typeof payload !== "object") throw new Error("invalid state injection payload");
  const { sessionId, injectionId } = payload as Record<string, unknown>;
  if (typeof sessionId !== "string" || !sessionId || typeof injectionId !== "string" || !injectionId) {
    throw new Error("invalid state injection payload");
  }
  return { sessionId, injectionId };
}

export function debugInjectionUrl(port: number, sessionId: string): string {
  return `http://127.0.0.1:${port}/sessions/${encodeURIComponent(sessionId)}/debug-injection`;
}
```

- [ ] **Step 4: Use the helper from the existing IPC handler**

```ts
const { sessionId, injectionId } = parseSessionStateInjectionPayload(payload);
const resp = await fetch(debugInjectionUrl(await portPromise, sessionId), {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ injectionId }),
});
```

- [ ] **Step 5: Run the focused Electron test to verify it passes**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/sessionStateInjectionIpc.test.ts`

Expected: PASS.

- [ ] **Step 6: Type-check and run related desktop tests**

Run: `cd apps/desktop && npx tsc --noEmit && node --experimental-strip-types --test tests/dockview.test.ts tests/electronSettingsMemory.test.ts tests/sessionStateInjection.test.ts tests/sessionStateInjectionIpc.test.ts`

Expected: all checks PASS.

- [ ] **Step 7: Commit the Electron fix**

```bash
git add apps/desktop/electron/main.ts apps/desktop/electron/sessionStateInjectionIpc.ts apps/desktop/tests/sessionStateInjectionIpc.test.ts
git commit -m "fix: validate debug injection IPC payloads"
```

### Task 3: Final Verification and PR Handoff

**Files:**
- Modify: `docs/superpowers/specs/2026-07-10-pr-153-review-fixes-design.md` only if implementation changes a documented decision.
- Modify: `AGENTS.md`, `README.md`, or `docs/agents/domain-entities.md` only if the documentation currency check identifies a required update.

**Interfaces:**
- Consumes: the completed Rust and Electron changes.
- Produces: verified PR-ready commits; no GitHub comments or thread resolution without explicit user instruction.

- [ ] **Step 1: Run complete applicable validation**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml && cd apps/desktop && npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`

Expected: PASS, except any documented pre-existing failures must be reported with their exact names.

- [ ] **Step 2: Run documentation currency check**

Run: `bash .claude/hooks/doc-check.sh`

Expected: no unaddressed documentation triggers.

- [ ] **Step 3: Inspect the final diff and request code review**

```bash
git diff origin/main...HEAD --check
git status --short
git rev-parse origin/main
git rev-parse HEAD
```

Expected: no whitespace errors and only intended PR changes.

- [ ] **Step 4: Commit any documentation follow-up**

```bash
git add AGENTS.md README.md docs/agents/domain-entities.md
git commit -m "docs: clarify debug injection behavior"
```
