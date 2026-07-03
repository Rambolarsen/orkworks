# Resume Capacity Checking Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an honest transient `checking capacity` state after session resume so the session UI and provider panel stop briefly lying about capped/not-capped status while Codex remains selectable.

**Architecture:** Track a runtime-only `capacity_check_pending` flag on live session handles plus minimal “fresh post-resume output seen” bookkeeping. `list_sessions` owns the lifecycle: it keeps pending true until a visible response has been emitted for a real post-resume scan, then clears the flag so later responses return the reconciled steady state. The provider runtime response reuses the existing `effectiveState` field with a new `checking_capacity` value, and the renderer maps that state into existing session/detail/provider surfaces.

**Tech Stack:** Rust sidecar (`axum`, serde), React + TypeScript renderer, Node built-in test runner, existing OrkWorks provider/session presentation helpers.

## Global Constraints

- Keep this scoped to post-resume transient capacity state only.
- Do not block resume while capped.
- Do not use time-based heuristics.
- Keep `checking_capacity` informational only; it must not make a provider unselectable.
- Provider runtime must reuse the existing `effectiveState` field rather than adding a new provider API field.
- Fresh post-resume capacity evaluation requires new output after the resume call: output-buffer growth or raw `scan_buf` growth.

---

### Task 1: Add runtime-only pending/freshness state for resumed sessions

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/session_types.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Test: `crates/orkworksd/src/session_types.rs`
- Test: `crates/orkworksd/src/http/session_handlers.rs`

**Interfaces:**
- Consumes: existing `SessionHandle`, `SessionInfo`, `resume_session`, and `list_sessions`
- Produces:
  - `SessionInfo.capacity_check_pending: Option<bool>` serialized as `capacityCheckPending`
  - `SessionHandle.capacity_check_pending: bool`
  - `SessionHandle.resume_scan_origin: Option<(usize, usize)>` where tuple = `(output_buffer_len, scan_buf_len)`
  - `SessionHandle.pending_capacity_visible_once: bool`

- [ ] **Step 1: Write failing session-type and resume/list tests**

```rust
#[test]
fn session_info_serializes_capacity_check_pending() {
    let info = SessionInfo {
        capacity_check_pending: Some(true),
        ..test_session_info("test", "Test", "/tmp", "running", "now")
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("\"capacityCheckPending\":true"));
}

#[tokio::test]
async fn list_sessions_keeps_pending_without_fresh_resume_output() {
    // Arrange a resumed live codex session with empty post-resume buffers.
    // Assert first list_sessions response returns capacityCheckPending = true.
}

#[tokio::test]
async fn list_sessions_requires_one_visible_fresh_output_cycle_before_clearing_pending() {
    // Arrange a resumed live codex session whose snapshot grows after resume.
    // Assert first response with fresh output still returns pending = true.
    // Assert next response clears pending.
}
```

- [ ] **Step 2: Run targeted Rust tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_info_serializes_capacity_check_pending list_sessions_keeps_pending_without_fresh_resume_output list_sessions_requires_one_visible_fresh_output_cycle_before_clearing_pending`

Expected: FAIL with missing `capacity_check_pending` fields and missing pending-state behavior in `list_sessions`.

- [ ] **Step 3: Add the session API field to `SessionInfo`**

```rust
#[serde(rename = "capacityCheckPending", skip_serializing_if = "Option::is_none")]
pub(crate) capacity_check_pending: Option<bool>,
```

Also update every `SessionInfo` constructor/default in:

```rust
capacity_check_pending: None,
```

- [ ] **Step 4: Extend `SessionHandle` with runtime-only pending/freshness bookkeeping**

```rust
struct SessionHandle {
    info: SessionInfo,
    kill_tx: tokio::sync::watch::Sender<bool>,
    output_buffer: peon::RingBuffer,
    scan_buf: String,
    command: harness::CommandSpec,
    initial_prompt: Option<String>,
    at_usage_limit_latched: bool,
    capacity_check_pending: bool,
    resume_scan_origin: Option<(usize, usize)>,
    pending_capacity_visible_once: bool,
}
```

Initialize new sessions with:

```rust
capacity_check_pending: false,
resume_scan_origin: None,
pending_capacity_visible_once: false,
```

- [ ] **Step 5: Mark resumed capacity-detecting sessions as pending**

In `resume_session`, after capabilities are resolved and before the session handle is stored:

```rust
let capacity_check_pending = capabilities.detect_capacity;
let resume_scan_origin = if capacity_check_pending { Some((0, 0)) } else { None };
```

Populate the outgoing `SessionInfo` with:

```rust
capacity_check_pending: capacity_check_pending.then_some(true),
```

Store on the handle:

```rust
handle.at_usage_limit_latched = false;
handle.capacity_check_pending = capacity_check_pending;
handle.resume_scan_origin = resume_scan_origin;
handle.pending_capacity_visible_once = false;
```

- [ ] **Step 6: Teach `list_sessions` to detect fresh post-resume output**

When cloning live sessions, capture snapshot lengths alongside the existing tuple:

```rust
sessions.values().map(|h| {
    let snapshot = h.output_buffer.snapshot();
    let scan_buf = h.scan_buf.clone();
    let snapshot_len = snapshot.len();
    let scan_len = scan_buf.len();
    (
        h.info.clone(),
        snapshot,
        scan_buf,
        h.at_usage_limit_latched,
        h.capacity_check_pending,
        h.resume_scan_origin,
        h.pending_capacity_visible_once,
        snapshot_len,
        scan_len,
    )
}).collect()
```

Fresh output predicate:

```rust
let has_fresh_resume_output = pending && origin.map(|(lines0, scan0)| {
    snapshot_len > lines0 || scan_len > scan0
}).unwrap_or(false);
```

- [ ] **Step 7: Return one visible pending response before clearing**

During live-session merge:

```rust
merged.capacity_check_pending = pending.then_some(true);
```

After assembling `infos`, update the matching session handles:

```rust
if handle.capacity_check_pending && has_fresh_resume_output {
    if handle.pending_capacity_visible_once {
        handle.capacity_check_pending = false;
        handle.resume_scan_origin = None;
        handle.pending_capacity_visible_once = false;
        handle.info.capacity_check_pending = None;
    } else {
        handle.pending_capacity_visible_once = true;
        handle.info.capacity_check_pending = Some(true);
    }
}
```

Do not clear pending when `has_fresh_resume_output == false`.

- [ ] **Step 8: Run targeted Rust tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_info_serializes_capacity_check_pending list_sessions_keeps_pending_without_fresh_resume_output list_sessions_requires_one_visible_fresh_output_cycle_before_clearing_pending`

Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add crates/orkworksd/src/main.rs crates/orkworksd/src/session_types.rs crates/orkworksd/src/http/session_handlers.rs
git commit -m "feat: track pending capacity checks after resume"
```

### Task 2: Reuse provider `effectiveState` for transient checking state

**Files:**
- Modify: `crates/orkworksd/src/providers.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Test: `crates/orkworksd/src/providers.rs`

**Interfaces:**
- Consumes:
  - `SessionInfo.capacity_check_pending`
  - `SessionInfo.model_provider_id`
  - `SessionInfo.harness_id`
- Produces:
  - `ProviderManager::update_session_capping(capped, reset_hints, checking)`
  - provider `effectiveState: "checking_capacity" | "healthy" | "degraded" | "unknown" | "capped" | "disabled"`

- [ ] **Step 1: Write failing provider-runtime tests**

```rust
#[test]
fn pending_capacity_overrides_runtime_state_for_enabled_provider() {
    let manager = ProviderManager::new();
    manager.update_session_capping(
        HashMap::from([("codex".into(), false)]),
        HashMap::new(),
        HashSet::from(["codex".into()]),
    );
    let resp = manager.get_providers_response();
    let codex = resp.providers.iter().find(|p| p.id == "codex").unwrap();
    assert_eq!(codex.effective_state, "checking_capacity");
}

#[test]
fn disabled_provider_stays_disabled_when_pending() {
    // Arrange disabled provider settings.
    // Assert effective_state == "disabled", not "checking_capacity".
}
```

- [ ] **Step 2: Run targeted Rust tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml pending_capacity_overrides_runtime_state_for_enabled_provider disabled_provider_stays_disabled_when_pending`

Expected: FAIL because `update_session_capping` does not track pending providers and `get_providers_response()` cannot emit `checking_capacity`.

- [ ] **Step 3: Track pending providers in `ProviderManager`**

Add storage:

```rust
session_checking: Arc<RwLock<HashSet<String>>>,
```

Expand the method signature:

```rust
pub fn update_session_capping(
    &self,
    capped: HashMap<String, bool>,
    reset_hints: HashMap<String, String>,
    checking: HashSet<String>,
) {
    *self.session_capped.write().unwrap() = capped;
    *self.session_reset_hint.write().unwrap() = reset_hints;
    *self.session_checking.write().unwrap() = checking;
}
```

- [ ] **Step 4: Derive pending provider ids in `list_sessions`**

Build a `HashSet<String>` from live sessions:

```rust
let mut provider_checking: HashSet<String> = HashSet::new();
for info in &infos {
    if info.capacity_check_pending == Some(true) {
        if let Some(pid) = &info.model_provider_id {
            provider_checking.insert(pid.clone());
        } else if let Some(hid) = &info.harness_id {
            provider_checking.insert(hid.clone());
        }
    }
}
state.providers.update_session_capping(harness_capped, harness_reset_hint, provider_checking);
```

- [ ] **Step 5: Emit `checking_capacity` through existing `effectiveState`**

In `get_providers_response()`:

```rust
let session_checking = self.session_checking.read().unwrap().clone();
let effective_str = if effective == ProviderEffectiveState::Disabled {
    "disabled"
} else if session_checking.contains(&entry.id) {
    "checking_capacity"
} else if session_is_capped {
    "capped"
} else {
    match effective {
        ProviderEffectiveState::Healthy => "healthy",
        ProviderEffectiveState::Degraded => "degraded",
        ProviderEffectiveState::Capped => "capped",
        ProviderEffectiveState::Unknown => "unknown",
        ProviderEffectiveState::Disabled => "disabled",
    }
};
```

- [ ] **Step 6: Run targeted Rust tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml pending_capacity_overrides_runtime_state_for_enabled_provider disabled_provider_stays_disabled_when_pending`

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/orkworksd/src/providers.rs crates/orkworksd/src/http/session_handlers.rs
git commit -m "feat: expose provider checking-capacity state"
```

### Task 3: Render `Checking capacity` in session and provider UI without affecting selectability

**Files:**
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/providerTypes.ts`
- Modify: `apps/desktop/src/providerPresentation.ts`
- Modify: `apps/desktop/src/labels.ts`
- Modify: `apps/desktop/src/sessionSort.ts`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
- Test: `apps/desktop/tests/api.test.ts`
- Test: `apps/desktop/tests/providersPanel.test.ts`
- Test: `apps/desktop/tests/sessionSort.test.ts`

**Interfaces:**
- Consumes:
  - `SessionInfo.capacityCheckPending?: boolean`
  - `ProviderEffectiveState = ... | "checking_capacity"`
- Produces:
  - session attention status `"checking_capacity"` for live pending sessions
  - provider rows displaying `effectiveState: "checking_capacity"`
  - unchanged provider selectability in `NewSessionDialog.tsx`

- [ ] **Step 1: Write failing TypeScript tests**

```ts
test("SessionInfo type accepts capacityCheckPending", () => {
  const session: SessionInfo = {
    id: "test",
    label: "Test",
    status: "running",
    cwd: "/tmp",
    created_at: "now",
    memoryState: "live",
    resumeStrategy: "none",
    capacityCheckPending: true,
  };
  assert.equal(session.capacityCheckPending, true);
});

test("sessionAttentionStatus prefers checking_capacity over capped while pending", () => {
  assert.equal(sessionAttentionStatus({
    id: "1",
    label: "resumed",
    status: "running",
    cwd: "/tmp",
    created_at: "now",
    memoryState: "live",
    resumeStrategy: "none",
    capacityCheckPending: true,
    atUsageLimit: true,
  }), "checking_capacity");
});

test("buildProviderViewModel preserves runtime checking_capacity state", () => {
  const model = buildProviderViewModel(sampleSettings(), sampleRuntime({
    providers: [{ ...sampleRuntime().providers[0], effectiveState: "checking_capacity" }, sampleRuntime().providers[1]],
  }));
  assert.equal(model.rows[0].effectiveState, "checking_capacity");
});
```

- [ ] **Step 2: Run targeted frontend tests to verify they fail**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/providersPanel.test.ts tests/sessionSort.test.ts`

Expected: FAIL with missing `capacityCheckPending` type support and unknown `"checking_capacity"` provider/session state.

- [ ] **Step 3: Extend API and provider state types**

In `api.ts`:

```ts
capacityCheckPending?: boolean;
```

In `providerTypes.ts`:

```ts
export type ProviderEffectiveState =
  | ProviderCapacityState
  | "disabled"
  | "checking_capacity";
```

- [ ] **Step 4: Prefer runtime provider state in the provider view model**

In `providerPresentation.ts`, stop discarding runtime `effectiveState`:

```ts
effectiveState: rt?.effectiveState ?? deriveEffectiveState(entry),
```

This is required so the backend-emitted `checking_capacity` state reaches the UI.

- [ ] **Step 5: Add copy and attention mapping for `checking_capacity`**

In `labels.ts`:

```ts
case "checking_capacity": return "Checking capacity";
```

and:

```ts
case "checking_capacity":
case "capped":
case "blocked":
  return "blocked";
```

In `sessionSort.ts`:

```ts
checking_capacity: 2,
```

and:

```ts
if (session.capacityCheckPending) return "checking_capacity";
if (session.atUsageLimit) return "capped";
```

- [ ] **Step 6: Render the transient state in session list/detail without special-casing capped text**

Keep existing components simple by relying on `sessionAttentionStatus()` and `attentionLabel()`:

```tsx
const badgeText =
  attn === "capped" && active.usageLimitResetHint
    ? `Capped · ${active.usageLimitResetHint}`
    : attentionLabel(attn);
```

This should remain unchanged if `attn === "checking_capacity"` now resolves correctly. The only required code edits here should be minimal guards or tests if these components assume the old enum set elsewhere.

- [ ] **Step 7: Verify provider selectability stays unchanged**

Do not add any gating for `"checking_capacity"` in `NewSessionDialog.tsx`. Add a source assertion test if needed:

```ts
const source = readFileSync(new URL("../src/components/NewSessionDialog.tsx", import.meta.url), "utf8");
assert.doesNotMatch(source, /checking_capacity.*disabled/);
```

- [ ] **Step 8: Run targeted frontend tests to verify they pass**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/providersPanel.test.ts tests/sessionSort.test.ts`

Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/src/api.ts apps/desktop/src/providerTypes.ts apps/desktop/src/providerPresentation.ts apps/desktop/src/labels.ts apps/desktop/src/sessionSort.ts apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/src/components/SessionListPanel.tsx apps/desktop/tests/api.test.ts apps/desktop/tests/providersPanel.test.ts apps/desktop/tests/sessionSort.test.ts
git commit -m "feat: show checking-capacity state after resume"
```

### Task 4: Full verification and doc currency check

**Files:**
- Modify: none unless verification reveals a defect
- Test: `crates/orkworksd/src/http/session_handlers.rs`
- Test: `crates/orkworksd/src/providers.rs`
- Test: `apps/desktop/tests/api.test.ts`
- Test: `apps/desktop/tests/providersPanel.test.ts`
- Test: `apps/desktop/tests/sessionSort.test.ts`

**Interfaces:**
- Consumes: completed Tasks 1-3
- Produces: verified implementation evidence and doc-currency confirmation

- [ ] **Step 1: Run focused Rust backend tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session list_sessions pending_capacity_overrides_runtime_state_for_enabled_provider disabled_provider_stays_disabled_when_pending`

Expected: PASS

- [ ] **Step 2: Run focused frontend tests**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/providersPanel.test.ts tests/sessionSort.test.ts`

Expected: PASS

- [ ] **Step 3: Run repo-required doc currency check**

Run: `bash .claude/hooks/doc-check.sh`

Expected: either no flagged docs, or only the already-updated design/plan files.

- [ ] **Step 4: Review git diff for scope**

Run: `git diff --stat HEAD~4..HEAD`

Expected: only resume-capacity-checking backend/runtime/frontend files plus the design/plan docs if still uncommitted.

- [ ] **Step 5: Commit any verification follow-up if needed**

```bash
git add <verified-fix-files>
git commit -m "test: finalize resume capacity checking verification"
```

Skip this step if no follow-up changes were needed.

## Self-Review

- Spec coverage: Task 1 covers session pending state, fresh-output rules, and one visible pending cycle. Task 2 covers provider `effectiveState`, precedence, and keying by provider/harness id. Task 3 covers session/detail/provider UI rendering plus unchanged selectability. Task 4 covers verification and doc-check.
- Placeholder scan: no `TODO`/`TBD` placeholders remain; every task names concrete files, commands, and target snippets.
- Type consistency: the same property names are used throughout: Rust `capacity_check_pending`, JSON/TS `capacityCheckPending`, provider `effectiveState: "checking_capacity"`.

Plan complete and saved to `docs/superpowers/plans/2026-07-03-resume-capacity-checking.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
