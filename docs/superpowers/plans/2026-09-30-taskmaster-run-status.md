# Taskmaster Analysis Run Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist and display Taskmaster's active analysis attempt and latest outcome for the selected workspace, including actionable failure details.

**Architecture:** Store one active attempt and one latest terminal outcome per canonical workspace in the existing installation-wide evaluation ledger. Mutations use existing process/file locking and snapshot revalidation; crash recovery first acquires the global analysis lease. An authenticated sidecar route exposes only the current workspace projection through narrow Electron IPC, and both desktop surfaces poll it while mounted.

**Tech Stack:** Rust/Serde/Axum, existing Taskmaster evaluation and file locks, Electron IPC, React/TypeScript, Node's built-in test runner.

**Spec:** `docs/superpowers/specs/2026-09-29-taskmaster-model-refresh-status-design.md`, “Analysis run status”.

## Global Constraints

- Provider/configuration availability remains separate from runtime outcome.
- Each workspace record contains at most one active attempt and one latest terminal outcome; total records grow with workspaces used.
- Mark attempts running before context collection or prompt construction; failures after that point become failed outcomes even without provider invocation.
- Pre-evaluation skips clear the queued attempt and preserve the prior outcome.
- Recovery acquires the same installation-wide analysis lease as evaluation; if busy, it leaves the record unchanged and retries on workspace open or evaluation admission.
- Status reads are read-only and return only the current instance's selected workspace projection.
- The legacy installation-wide `lastError` is deserialization-only and is not displayed as analysis status.
- Knowledge-update errors remain independent; no popups, focus changes, peer-instance status, or analysis coordination.

---

### Task 1: Record the protocol decision before implementation

**Files:**
- Create: `docs/adr/0071-taskmaster-model-refresh-and-run-status.md`
- Modify: `docs/adr/README.md`
- Modify: `docs/superpowers/specs/2026-09-29-taskmaster-model-refresh-status-design.md`
- Modify: `specs/taskmaster-knowledge.md`
- Modify: `docs/agents/architecture.md`

- [ ] **Step 1: Add ADR 0071.** Record the typed Codex/Ollama discovery seam, Taskmaster-only Electron authority, current-workspace run-status route, durable `activeAttempt`/`latestOutcome` record, error commit semantics, and analysis-lease-gated recovery. State that it does not introduce peer-instance coordination or aggregate record retention.
- [ ] **Step 2: Add ADR 0071 to the historical index and synchronize the accepted spec, design, and architecture concept with the lease-gated recovery rule.**
- [ ] **Step 3: Run `rtk git diff --check` and `rtk bash scripts/doc-check.sh`.** Resolve all documentation findings before code.

### Task 2: Add durable workspace run-status types

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/runtime.rs`
- Modify: `crates/orkworksd/src/taskmaster/runtime/inference.rs`
- Test: runtime unit tests in `crates/orkworksd/src/taskmaster/runtime.rs`

**Interface:**
- Add `TaskmasterRunStatus { workspace_path, active_attempt, latest_outcome }`.
- Add serialized `TaskmasterRunAttempt { id, state, queued_at, started_at, trigger, provider, model }` and `TaskmasterRunOutcome { state, started_at, completed_at, trigger, provider, model, error_summary }`.
- Add `TaskmasterRunTrigger::{Manual, Background}` and terminal/active state enums serialized as snake_case.
- Add `EvaluationLedger.workspace_runs: BTreeMap<String, WorkspaceRunRecord>` with `#[serde(default)]`.
- Add `TaskmasterRuntime::run_status(workspace: Option<&Path>) -> Result<TaskmasterRunStatus, String>`, a read-only projection that canonicalizes the selected key. No selected workspace returns an unavailable error; a missing record in a readable ledger returns idle; an unreadable ledger returns unavailable rather than idle.

- [ ] **Step 1: Write failing serialization and projection tests.** Cover no selected workspace (unavailable), an unreadable ledger (unavailable), a missing record (idle), canonical workspace key, two different workspace records, safe error-summary bounds, and loading a legacy ledger with `lastError` while the current serialization omits that field.

```rust
let status = runtime.run_status(Some(&workspace_a));
let status = runtime.run_status(Some(&workspace_a)).expect("readable ledger");
assert_eq!(status.workspace_path.as_deref(), Some(canonical_a.as_str()));
assert_eq!(status.active_attempt, None);
assert_eq!(status.latest_outcome, None);
```

- [ ] **Step 2: Run the focused runtime tests and confirm they fail on the missing types/field.**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_run_status
```

- [ ] **Step 3: Implement the types and read-only projection.** Keep `last_error` deserializable with `skip_serializing`; remove it from `TaskmasterStatus` so Settings cannot expose the legacy global analysis error. Normalize run error summaries to at most 512 UTF-8 bytes and replace control characters before persistence.
- [ ] **Step 4: Run the focused runtime tests and existing ledger compatibility tests.**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_run_status
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml corrupted_ledger
```

### Task 3: Add locked run transitions and legacy-error reconciliation

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/runtime.rs`
- Modify: `crates/orkworksd/src/taskmaster/runtime/inference.rs`
- Modify: `crates/orkworksd/src/taskmaster/evaluator.rs`
- Test: runtime tests and `crates/orkworksd/src/taskmaster/evaluator/identity_tests.rs`

- [ ] **Step 1: Add failing tests for queue, running, terminal failure, success, and skip transitions.** Assert failure summaries are bounded; success clears only the selected workspace's run error; stale snapshots cannot finish a newer attempt; and clearing an attempt with no prior outcome removes the empty record.
- [ ] **Step 2: Add a failing regression test for the existing `record_evaluation_error(None)` call.** A valid provider response must leave the previous failure visible until result application and successful cache persistence both commit.
- [ ] **Step 3: Run the focused tests and confirm they fail.**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_run_transition
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml successful_result_clears_workspace_run_error
```

- [ ] **Step 4: Implement locked transition methods.** Queue assigns a monotonically increasing ledger run ID and captures provider/model/trigger. `mark_running` sets the start timestamp before context collection. `finish_failed` and `finish_succeeded` move the matching attempt into `latest_outcome` and clear `active_attempt`. `clear_active_attempt` only clears the matching run ID and preserves prior outcome. All writes reload under the existing process mutex and OS file lease.
- [ ] **Step 5: Keep identity validation read-only.** Replace the current pre-apply `record_evaluation_error(..., None)` use with a no-write `with_current_evaluation` validation. Clear the prior run error only inside `record_evaluation_success`, after the result has been applied and the cache update is accepted.
- [ ] **Step 6: Run the focused runtime/identity tests and existing stale-snapshot tests.**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_run_transition
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml successful_result_clears_workspace_run_error
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml stale_apply_is_invalidated
```

### Task 4: Tie evaluator lifecycle and crash recovery to the global lease

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/evaluator.rs`
- Modify: `crates/orkworksd/src/http/taskmaster_handlers.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Test: evaluator tests and `crates/orkworksd/src/http/taskmaster_handlers.rs`

- [ ] **Step 1: Add failing lifecycle tests.** Cover accepted manual request queued state, evaluator-start running state, context/prompt failure to failed, cache/cooldown/eligibility early return clearing only active state, and process restart converting running to interrupted while queued is cleared.
- [ ] **Step 2: Add failing lease tests.** Hold the global analysis lock and assert workspace-open recovery leaves the run unchanged. Release it and assert recovery marks the matching run interrupted. Assert recovery never changes another workspace's projection.
- [ ] **Step 3: Run focused tests and confirm the lifecycle behavior is missing.**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_run_recovery
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml evaluator_run_status
```

- [ ] **Step 4: Implement `ScheduleResult::{Scheduled, AlreadyRunning, Unavailable}` and persist `queued` before spawning the blocking evaluator.** Pass the run ID and already-acquired analysis lease to the worker. Map ledger/write failures to the existing `unavailable` manual response.
- [ ] **Step 5: Add an evaluator lifecycle guard.** Create it before workspace/snapshot early returns so `Drop` clears a matching queued/running attempt when a non-run exit occurs. Transition to running before the context collector; record context, prompt, provider, response, and result-application errors as terminal failures.
- [ ] **Step 6: Recover on workspace open and before a later evaluation, only while the caller holds the global analysis lease.** If `try_analysis_lease` reports contention, preserve the record. Keep status reads mutation-free.
- [ ] **Step 7: Run focused evaluator, handler, and workspace-open tests.**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_run_recovery
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml evaluator_run_status
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml analyze_taskmaster
```

### Task 5: Expose authenticated run status to Electron

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/http/taskmaster_settings_handlers.rs`
- Modify: `apps/desktop/electron/taskmasterSettings.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Modify: `apps/desktop/src/taskmasterSettings.ts`
- Test: sidecar route tests and `apps/desktop/tests/taskmasterSettings.test.ts`

**Interface:**
- Add `GET /taskmaster/run-status`, guarded by `authorize_taskmaster_request`, returning the current workspace's `TaskmasterRunStatus` only.
- Add `getTaskmasterRunStatus(): Promise<TaskmasterRunStatus>` through Electron preload. Electron uses the active sidecar port and existing open-plan token; renderer never receives the token or chooses a URL.

- [ ] **Step 1: Add failing auth/scope tests for the route and a failing fixed-loopback/token test for Electron's `run-status` resource.** Cover no selected workspace and unreadable-ledger responses as unavailable, and confirm the renderer cannot provide a workspace path.
- [ ] **Step 2: Run the focused tests and confirm the route/resource are missing.**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_run_status_route
cd apps/desktop && rtk node --experimental-strip-types --test tests/taskmasterSettings.test.ts
```

- [ ] **Step 3: Implement the read-only route and fixed Electron request resource.** The response uses the workspace already owned by the sidecar; it does not accept a workspace path from the renderer. Keep the status request separate from the broader Settings payload.
- [ ] **Step 4: Add preload declarations and run focused tests plus TypeScript validation.**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_run_status_route
cd apps/desktop && rtk node --experimental-strip-types --test tests/taskmasterSettings.test.ts && rtk pnpm exec tsc --noEmit
```

### Task 6: Display status in Recommendations and Settings

**Files:**
- Modify: `apps/desktop/src/components/RecommendationsPanel.tsx`
- Modify: `apps/desktop/src/components/TaskmasterSettings.tsx`
- Modify: `apps/desktop/src/taskmasterSettings.ts`
- Modify: `apps/desktop/src/App.css`
- Test: `apps/desktop/tests/taskmasterSettingsComponent.test.mjs`
- Test: `apps/desktop/tests/taskmaster.test.ts`

- [ ] **Step 1: Add failing display tests.** Verify active states and latest outcomes render with provider/model, timestamps, and failure detail; legacy `status.lastError` is not displayed as an analysis error; knowledge-update errors remain visible; no-workspace and not-ready states clear previous workspace data.
- [ ] **Step 2: Add polling tests/source assertions.** Recommendations refreshes run status alongside its existing five-second recommendation poll. Settings polls status every five seconds while mounted and refreshes on window focus; stale IPC responses after unmount are ignored.
- [ ] **Step 3: Run the focused desktop tests and confirm they fail.**

```bash
cd apps/desktop && rtk node --experimental-strip-types --test tests/taskmasterSettingsComponent.test.mjs tests/taskmaster.test.ts
```

- [ ] **Step 4: Implement a shared pure status projection/format helper and render it in both surfaces.** Recommendations shows compact queued/running/latest outcome text beside Analyze now. Settings shows the complete active/latest record and keeps provider readiness and knowledge-update status independent.
- [ ] **Step 5: Run the component/helper tests and TypeScript check.**

```bash
cd apps/desktop && rtk node --experimental-strip-types --test tests/taskmasterSettingsComponent.test.mjs tests/taskmaster.test.ts tests/taskmasterSettings.test.ts && rtk pnpm exec tsc --noEmit
```

### Task 7: Verify the run-status slice

- [ ] Run `rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.
- [ ] Run focused Rust and desktop tests from Tasks 2–6.
- [ ] Run the complete Rust and desktop test suites after both feature plans are integrated.
- [ ] Run `rtk git diff --check` and `rtk bash scripts/doc-check.sh`.
- [ ] Confirm status GET is read-only, status is current-workspace-only, all writes are lease/identity guarded, and neither a stale error nor a live cross-instance analysis is reported as the current failure/interruption.
