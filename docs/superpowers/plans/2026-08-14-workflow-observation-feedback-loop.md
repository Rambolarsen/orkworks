# Workflow Observation Feedback Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace unused summary checkpoint history with durable workflow observations written by Peon or an authenticated session agent, then have Taskmaster present deterministic, dismissible workflow-improvement recommendations.

**Architecture:** A deep `workflow_observations` module owns validation, provenance, idempotency, sequencing, bounded per-session persistence, and cleanup. Taskmaster reads that module through a workspace snapshot, writes the canonical recommendation contract through its own store, and exposes thin list/get/dismiss/refresh handlers; the desktop renders only passive `improve_workflow` cards. The latest session summary remains a field-level-provenance snapshot for Taskmaster handoffs, while summary checkpoint history and its UI/API are removed.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio, Serde/serde_json, SHA-256, `getrandom` 0.3, React 19, TypeScript 5.9, Dockview, Node test runner, pnpm 11.9.

## Global Constraints

- Read all authoritative specs and both scoped `AGENTS.md` files before editing; stop if any spec is missing.
- Invoke `skills/starting-work/` before edits, use an agent-owned branch/worktree, and open one PR for this logical feature.
- Before Task 2, confirm ADR 0040's `labelResetCommands` runtime implementation is on the base branch; if its owner is still landing that work, rebase after it merges instead of duplicating or overwriting it.
- Update the authoritative specs and write ADR 0041 before implementation code; create or update the GitHub implementation issue before code begins.
- Peon writes observations and never recommendations; Taskmaster reads observations and never silently changes repository files.
- Explicit reporting is capability-authenticated, limited to 8 KiB, and cannot set workspace, source, confidence, fingerprint, sequence, or recommendation state.
- V1 correlation is exact fingerprint matching only: `v1:<kind>:<trimmed-lowercased-collapsed-description>`.
- Two observations at confidence `>= 0.6`, or one high-impact observation at confidence `>= 0.8`, qualify.
- Observations are per-session bounded to 1,000 evidence records and 2 MiB including idempotency tombstones; Taskmaster reads the newest 10,000 by durable workspace sequence.
- Explicit idempotency lasts 15 minutes; a completed Peon revision range is suppressed for the lifetime of its runtime regardless of wall-clock age.
- `improve_workflow` is passive: `requiresApproval` is `false`, the desktop exposes `Dismiss` only, and OrkWorks creates no issue/session/file edit from it.
- Session forgetting and automatic retention remove observation segments and every recommendation snapshot containing that session's evidence.
- Use pnpm for all desktop/docs package work and prefix repository shell commands with `rtk`.
- Run Rust build/tests, desktop type-check/tests, doc currency, worktree currency, and `/code-review` before PR handoff.

---

## File map

- `crates/orkworksd/src/workflow_observations.rs` — deep module for observation types, validation, sequence allocation, idempotency, bounded segment storage, diagnostics, and session deletion.
- `crates/orkworksd/src/http/workflow_observation_handlers.rs` — thin authenticated explicit-report adapter.
- `crates/orkworksd/src/taskmaster/mod.rs` — canonical recommendation types, workspace snapshot, coordinator facade, and debounce entry point.
- `crates/orkworksd/src/taskmaster/evaluator.rs` — exact clustering, eligibility, deterministic templates, dismissal watermarks, and successor rules.
- `crates/orkworksd/src/taskmaster/store.rs` — atomic recommendation persistence, list/get/dismiss, orphan scrubbing, and embedded evidence snapshots.
- `crates/orkworksd/src/http/taskmaster_handlers.rs` — thin list/get/dismiss/refresh API adapter.
- `apps/desktop/src/taskmaster.ts` — Taskmaster API/domain types and pure presentation helpers.
- `apps/desktop/src/components/RecommendationsPanel.tsx` — polling, diagnostics, cards, evidence expansion, and dismissal.
- Existing metadata/runtime/view files — current-summary projection, Peon adapter, capability injection, cleanup, router wiring, and removal of summary checkpoints.

### Task 1: Make the feature authoritative and tracked

**Files:**
- Modify: `specs/orkworks-mvp.md`
- Modify: `specs/taskmaster.md`
- Create: `docs/adr/0041-workflow-observations-replace-summary-checkpoints.md`
- Modify: `docs/adr/0024-bounded-terminal-replay-durable-summary-checkpoints.md`
- Modify: `docs/adr/0029-session-label-topic-vs-activity-summary.md`
- Modify: `docs/adr/README.md`
- Modify: `docs/agents/architecture.md`
- Modify: `docs/agents/domain-entities.md`
- Modify: `AGENTS.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: approved design `docs/superpowers/specs/2026-08-14-workflow-observation-feedback-loop-design.md`.
- Produces: authoritative protocol/type/route definitions and ADR 0041; a GitHub issue whose acceptance criteria mirror Tasks 2–11.

- [ ] **Step 1: Invoke the branch guardrail and verify ownership**

Run the `skills/starting-work/` skill. Use its inspection commands and create the branch `feat/workflow-observation-feedback-loop` in an isolated worktree if the primary checkout is dirty or on a foreign branch.

Expected: the implementation checkout is on `feat/workflow-observation-feedback-loop`; unrelated changes from the original checkout are absent.

- [ ] **Step 2: Update the authoritative product contracts**

Add the observation schema, storage paths, limits, reporting route, summary snapshot fields, Taskmaster `improve_workflow` variant, thresholds, dismissal watermark, and passive UI behavior to the two specs. Remove statements that define durable summary checkpoints or “Task history” as required behavior. Keep terminal replay bounds and stable one-shot labels.

- [ ] **Step 3: Record the architecture decision and supersessions**

Write ADR 0041 with `Status: accepted`, `Date: 2026-08-14`, and these decisions: current-summary snapshot; workflow observations as durable improvement evidence; exact deterministic Taskmaster correlation; embedded recommendation evidence; removal of summary checkpoints. Mark ADR 0024 and ADR 0029 `superseded by ADR 0041`, while ADR 0041 restates bounded replay and stable label behavior. Add 0041 to the ADR index and update the curated ADR bullets in `AGENTS.md`.

- [ ] **Step 4: Synchronize architecture/domain/user entry docs**

Document `summarySource`, `summaryConfidence`, `summaryObservedAt`, `WorkflowObservation`, the sequence file, session segments, recommendation snapshots, and the three Taskmaster API routes in `docs/agents/architecture.md` and `docs/agents/domain-entities.md`. Update `README.md` and root metadata-protocol bullets so no document still advertises the summary-log route or Task history.

- [ ] **Step 5: Build docs and run drift checks**

Run: `rtk pnpm --dir docs build`

Expected: VitePress exits 0 with no dead links.

Run: `rtk bash .claude/hooks/doc-check.sh`

Expected: exit 0 and no unaddressed drift warning.

- [ ] **Step 6: Create or update the implementation issue**

Run: `rtk gh issue list --state open --search 'workflow observation feedback loop in:title'`

Expected: either one matching issue or no output. If absent, run:

```bash
rtk gh issue create --title "Implement workflow observation feedback loop" --body $'Implement the approved workflow-observation feedback-loop design.\n\n- [ ] Persist field-level current-summary provenance and remove summary checkpoints\n- [ ] Record bounded, sequenced workflow observations from authenticated agents and Peon\n- [ ] Delete session evidence and derived recommendations during retention/forget\n- [ ] Generate deterministic passive Taskmaster improve_workflow recommendations\n- [ ] Persist dismissal watermarks and evidence snapshots\n- [ ] Render evidence-backed cards with Dismiss only\n- [ ] Pass Rust, desktop, docs, drift, and code-review gates\n\nSpec: docs/superpowers/specs/2026-08-14-workflow-observation-feedback-loop-design.md'
```

Expected: GitHub prints the new issue URL. If a match exists, edit that issue to contain the same checklist instead of creating a duplicate.

- [ ] **Step 7: Commit the contract slice**

```bash
rtk git add specs/orkworks-mvp.md specs/taskmaster.md docs/adr docs/agents AGENTS.md README.md
rtk git commit -m "docs: specify workflow observation feedback loop"
```

Expected: one docs commit containing ADR 0041 and no implementation code.

### Task 2: Make summary a current Taskmaster snapshot and remove checkpoint history

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/session_types.rs`
- Modify: `crates/orkworksd/src/session_view.rs`
- Modify: `crates/orkworksd/src/runtime/observed_status.rs`
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_http.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/domain/session.ts`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/src/App.css`
- Test: inline Rust tests in the files above
- Test: `apps/desktop/tests/api.test.ts`
- Test: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- Consumes: existing `SessionMetadata.summary`, Peon inference merge, attention merge, descriptive-input detection, and label-reset branch.
- Produces: flat JSON fields `summarySource?: "agent" | "peon"`, `summaryConfidence?: number`, `summaryObservedAt?: string`; `set_summary_snapshot` and `clear_summary_snapshot`; no `/sessions/:id/summary-log` route.

- [ ] **Step 1: Write failing summary atomicity and compatibility tests**

Add tests that deserialize a legacy summary without provenance, set all four fields from Peon, keep all four on an attention signal without a message, set all four from a non-empty agent message, clear all four on descriptive input and harness-declared label reset, and preserve all four on hotkeys/non-descriptive confirmation.

Use this field contract in both `SessionMetadata` and `SessionInfo`:

```rust
#[serde(rename = "summarySource", skip_serializing_if = "Option::is_none")]
pub summary_source: Option<String>,
#[serde(rename = "summaryConfidence", skip_serializing_if = "Option::is_none")]
pub summary_confidence: Option<f64>,
#[serde(rename = "summaryObservedAt", skip_serializing_if = "Option::is_none")]
pub summary_observed_at: Option<String>,
```

- [ ] **Step 2: Run the focused Rust tests and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml summary_snapshot -- --nocapture`

Expected: FAIL because the provenance fields/helpers do not exist.

- [ ] **Step 3: Implement one atomic summary helper and use it from both adapters**

Add these exact helpers in `metadata.rs` and call them only for `agent`/`peon` non-empty summaries:

```rust
pub(crate) fn set_summary_snapshot(
    meta: &mut SessionMetadata,
    summary: &str,
    source: &str,
    confidence: f64,
    observed_at: &str,
) {
    let value = summary.trim();
    if value.is_empty() || !matches!(source, "agent" | "peon") { return; }
    meta.summary = Some(value.to_string());
    meta.summary_source = Some(source.to_string());
    meta.summary_confidence = Some(confidence);
    meta.summary_observed_at = Some(observed_at.to_string());
}

pub(crate) fn clear_summary_snapshot(meta: &mut SessionMetadata) {
    meta.summary = None;
    meta.summary_source = None;
    meta.summary_confidence = None;
    meta.summary_observed_at = None;
}
```

Mirror the same assignment in live `SessionInfo` only after persistence succeeds. In `record_peon_input_side_effects`, clear persisted/live fields only when the submitted input is non-sensitive and `peon::is_descriptive_input` is true. In the existing exact harness label-reset branch, clear them in the same persisted/live critical section.

- [ ] **Step 4: Remove summary checkpoint production and transport**

Delete `history_summary`/`work_history_summary` arguments from the Peon metadata merge, stop writing `Event.summary`/`Event.source`, remove `SummaryLogEntry`, `SummaryLogResponse`, `get_summary_log`, and the router entry. Keep historical event deserialization tolerant by leaving optional legacy fields on `Event`.

- [ ] **Step 5: Remove Task history from the desktop**

Delete `SummaryLogEntry`, `getSummaryLog`, component state/effects/rendering, and `.detail-task-history*` styles. Add the three summary provenance fields to DTO/domain mappings without changing the selected-session headline fallback.

- [ ] **Step 6: Run focused Rust and desktop tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml summary_snapshot -- --nocapture`

Expected: PASS.

Run: `rtk node --experimental-strip-types --test apps/desktop/tests/api.test.ts apps/desktop/tests/dockview.test.ts`

Expected: PASS with tests asserting the summary-log API/rendering are absent and summary provenance maps through.

- [ ] **Step 7: Commit**

```bash
rtk git add crates/orkworksd/src apps/desktop/src apps/desktop/tests
rtk git commit -m "refactor: keep only the current session summary"
```

### Task 3: Build the workflow-observation recording module

**Files:**
- Create: `crates/orkworksd/src/workflow_observations.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Test: inline `workflow_observations::tests`

**Interfaces:**
- Consumes: active workspace metadata root and session IDs.
- Produces: `WorkflowObservationStore::open`, `record_observation`, `workspace_observations`, `delete_session_observations`, `diagnostics`; `RecordOutcome::{Accepted, Duplicate}` and typed `RecordError`.

- [ ] **Step 1: Write failing domain, persistence, idempotency, ordering, and corruption tests**

Define test fixtures around this public crate-level contract:

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservationKind {
    Repetition, Obstacle, MissingContext, Assumption,
    Correction, Workaround, VerificationGap,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Impact { Low, Medium, High }
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservationSource { Agent, Peon }
pub(crate) enum ObservationOrigin { Agent, Peon }
pub(crate) struct ObservationCandidate {
    pub kind: ObservationKind,
    pub description: String,
    pub evidence: String,
    pub reported_impact: Impact,
    pub confidence: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowObservation {
    pub id: String,
    pub sequence: u64,
    pub session_id: String,
    pub observed_at: String,
    pub kind: ObservationKind,
    pub description: String,
    pub evidence: String,
    pub reported_impact: Impact,
    pub source: ObservationSource,
    pub confidence: f64,
    pub fingerprint: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredObservation {
    #[serde(flatten)]
    observation: WorkflowObservation,
    idempotency_key_hash: String,
    payload_hash: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ObservationDiagnostic {
    pub code: String,
    pub message: String,
    pub session_id: Option<String>,
}
pub(crate) enum RecordOutcome {
    Accepted(WorkflowObservation),
    Duplicate { observation_id: String, sequence: u64, accepted_at: String },
}
```

Cover all seven kinds; empty/oversized input; server-owned source/confidence/fingerprint; exact normalization; concurrent same-key calls; same key/different payload; 15-minute explicit expiry; 1,000-record/2-MiB trim; 10,000 workspace read; partial tail recovery; interior corruption diagnostics; and monotonic sequences across equal clocks, restart, deletion, and a deliberate counter gap.

- [ ] **Step 2: Run the module test target and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml workflow_observations::tests -- --nocapture`

Expected: FAIL because the module and types do not exist.

- [ ] **Step 3: Implement the deep module with a small external surface**

Use these constants and method signatures:

```rust
const MAX_DESCRIPTION_CHARS: usize = 500;
const MAX_EVIDENCE_CHARS: usize = 2_000;
const MAX_SEGMENT_OBSERVATIONS: usize = 1_000;
const MAX_SEGMENT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_WORKSPACE_OBSERVATIONS: usize = 10_000;
const IDEMPOTENCY_WINDOW_SECS: i64 = 15 * 60;
const MAX_TOMBSTONES: usize = 1_024;
const MAX_ACCEPTED_PER_SESSION_MINUTE: usize = 60;

impl WorkflowObservationStore {
    pub(crate) fn open(root: PathBuf) -> Result<Self, StoreError>;
    pub(crate) fn record_observation(
        &self, session_id: &str, origin: ObservationOrigin,
        idempotency_key: &str, candidate: ObservationCandidate,
    ) -> Result<RecordOutcome, RecordError>;
    pub(crate) fn workspace_observations(&self) -> Result<Vec<WorkflowObservation>, StoreError>;
    pub(crate) fn delete_session_observations(&self, session_id: &str) -> Result<(), StoreError>;
    pub(crate) fn diagnostics(&self) -> Vec<ObservationDiagnostic>;
}
```

Hold one internal mutex across key lookup, atomic counter advance, append-or-rewrite, cache publication, and trimming. Persist raw observations/internal tombstones as tagged NDJSON, hash keys/payloads with SHA-256, preserve unexpired tombstones before newest evidence during a rewrite, and reject writes when the counter is malformed. Use temp-file sync, atomic replace, and parent-directory sync for counter/trim rewrites.

- [ ] **Step 4: Attach one store to each `WorkspaceState`**

Create it from the global metadata root in `set_workspace` and test fixtures:

```rust
struct WorkspaceState {
    path: PathBuf,
    metadata: metadata::MetadataStore,
    workflow_observations: workflow_observations::WorkflowObservationStore,
    watcher: watcher::MetadataWatcher,
}
```

Do not expose paths to Peon or Taskmaster; both must call the store methods.

- [ ] **Step 5: Run tests and commit**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml workflow_observations::tests -- --nocapture`

Expected: PASS.

```bash
rtk git add crates/orkworksd/src/workflow_observations.rs crates/orkworksd/src/main.rs crates/orkworksd/src/http/session_handlers.rs
rtk git commit -m "feat: persist bounded workflow observations"
```

### Task 4: Add capability-authenticated explicit agent reporting

**Files:**
- Modify: `crates/orkworksd/Cargo.toml`
- Modify: `Cargo.lock`
- Create: `crates/orkworksd/src/http/workflow_observation_handlers.rs`
- Modify: `crates/orkworksd/src/http/mod.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Test: inline handler/router/runtime tests

**Interfaces:**
- Consumes: `WorkflowObservationStore::record_observation(..., ObservationOrigin::Agent, ...)`.
- Produces: `POST /sessions/:id/workflow-observations`; `ORKWORKS_REPORT_TOKEN`; `{ observationId, sequence, acceptedAt, duplicate }`.

- [ ] **Step 1: Write failing security and protocol tests**

Cover: 64-hex-character token generated from 32 random bytes; token injected with session ID/port; replacement on resume; known live session only; correct bearer accepted; missing/wrong token rejected; no token persistence/serialization/logging; required visible-ASCII 1–128-byte `Idempotency-Key`; 8-KiB body; fixed fields/vocabulary; `409` conflict; `429` after 30 attempts/60 seconds; total-store cap; same-origin browser request without capability rejected.

- [ ] **Step 2: Run focused tests and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml workflow_observation_report -- --nocapture`

Expected: FAIL because the route/capability do not exist.

- [ ] **Step 3: Add minimal OS-random capability support**

Add `getrandom = "0.3"` and implement:

```rust
fn new_workflow_report_token() -> Result<String, getrandom::Error> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes)?;
    Ok(hex::encode(bytes))
}
```

Add `workflow_report_token: String` and a rolling attempt deque to `SessionHandle`; initialize every live/test handle, regenerate on resume, and pass the token to `session_env_overrides(session_id, port, token)`. Update dependency documentation in `README.md` and `AGENTS.md` as required by the repo guide.
If OS randomness fails, fail session creation/resume without spawning the child
and return a scoped `500`; never start a reportable session with an empty or
predictable token.

- [ ] **Step 4: Implement the thin handler and route limits**

Deserialize only this request:

```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkflowObservationReport {
    kind: ObservationKind,
    description: String,
    evidence: String,
    #[serde(rename = "reportedImpact")]
    reported_impact: Impact,
}
```

Authenticate before parsing/persisting, enforce the per-session attempt window, map the request to an agent candidate with module-owned confidence `0.9`, and attach `DefaultBodyLimit::max(8 * 1024)` to this route only.
Run the synchronous durable store call inside `tokio::task::spawn_blocking`; do
not hold the sessions mutex or workspace mutex across `.await`.

- [ ] **Step 5: Run tests and commit**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml workflow_observation_report -- --nocapture`

Expected: PASS.

```bash
rtk git add crates/orkworksd/Cargo.toml Cargo.lock crates/orkworksd/src README.md AGENTS.md
rtk git commit -m "feat: accept authenticated workflow observations"
```

### Task 5: Let Peon infer observations without duplicating evidence windows

**Files:**
- Modify: `crates/orkworksd/src/peon.rs`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Test: inline Peon/runtime tests

**Interfaces:**
- Consumes: ring-buffer revisions and `WorkflowObservationStore::record_observation(..., ObservationOrigin::Peon, ...)`.
- Produces: `PeonInference.workflow_observations: Vec<ObservationCandidate>`; runtime-instance-bound Peon keys; two-minute pass deadline.

- [ ] **Step 1: Write failing Peon schema and range tests**

Use a strict candidate payload:

```rust
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PeonWorkflowObservation {
    pub kind: ObservationKind,
    pub description: String,
    pub evidence: String,
    #[serde(rename = "reportedImpact")]
    pub reported_impact: Impact,
    pub confidence: f64,
}
```

Test situation-only, observation-only, and combined inference; per-candidate confidence; maximum five candidates; snapshot first/last revisions; retry after transient store failure; no retry after accepted/deduplicated/permanently rejected range; same completed range still suppressed after a fake 16-minute advance; later revisions create a distinct occurrence with the same fingerprint.

- [ ] **Step 2: Run focused tests and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_workflow_observation -- --nocapture`

Expected: FAIL because inference candidates/range keys do not exist.

- [ ] **Step 3: Extend the inference schema and prompt**

Add `#[serde(default)] workflow_observations: Vec<PeonWorkflowObservation>` to `PeonInference`. In the inference prompt, enumerate the seven kinds, require concrete evidence, forbid ordinary progress/redraw/speculation, and cap output at five candidates. Validate `0.0..=1.0` confidence before crossing the recording seam.

- [ ] **Step 4: Implement revision-bound recording**

Add a random `runtime_instance_id` to `SessionRuntime` and expose a ring snapshot carrying `(first_revision, last_revision, lines)`. Derive each key as SHA-256 over:

```text
peon-v1 | runtime-instance-id | session-id | input-generation |
first-revision | last-revision | candidate-index
```

Wrap each pass in `tokio::time::timeout(Duration::from_secs(120), ...)`. Advance `min_peon_output_revision` past the captured last revision only after every candidate is accepted, duplicate, or permanently invalid; keep it unchanged on transient store failure.
Move synchronous observation writes to `spawn_blocking`, then reacquire runtime
state only to publish the cursor outcome.

- [ ] **Step 5: Run tests and commit**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_workflow_observation -- --nocapture`

Expected: PASS.

```bash
rtk git add crates/orkworksd/src/peon.rs crates/orkworksd/src/runtime
rtk git commit -m "feat: record Peon workflow observations"
```

### Task 6: Make forgetting and retention remove derived evidence

**Files:**
- Modify: `crates/orkworksd/src/runtime/retention.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/workflow_observations.rs`
- Test: inline retention/forget/store tests

**Interfaces:**
- Consumes: `delete_session_observations(session_id)`; Task 7 later adds recommendation cleanup to the same coordinator.
- Produces: one cleanup coordinator function whose failure prevents a false successful forget response.

- [ ] **Step 1: Write failing deletion tests**

Create metadata, event, terminal, and observation files for two sessions. Verify forgetting/retaining one deletes only its files, preserves the global sequence counter, and reports failure if observation cleanup fails. Reserve a callback parameter for recommendation cleanup:

```rust
pub(crate) fn delete_session_evidence(
    workspace: &WorkspaceState,
    session_id: &str,
    delete_recommendations: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String>
```

- [ ] **Step 2: Run tests and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml delete_session_evidence -- --nocapture`

Expected: FAIL because coordinated cleanup does not exist.

- [ ] **Step 3: Implement and use coordinated cleanup**

Delete recommendation snapshots first through the callback, then the observation segment, then existing session/event/terminal artifacts. Hold the workspace cleanup lock so list/read handlers cannot expose an orphan during deletion. Use the same function from explicit forget and retention cleanup.

- [ ] **Step 4: Run tests and commit**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml delete_session_evidence -- --nocapture`

Expected: PASS.

```bash
rtk git add crates/orkworksd/src/runtime/retention.rs crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/src/workflow_observations.rs
rtk git commit -m "fix: remove retained workflow evidence with sessions"
```

### Task 7: Add the canonical Taskmaster recommendation contract and store

**Files:**
- Create: `crates/orkworksd/src/taskmaster/mod.rs`
- Create: `crates/orkworksd/src/taskmaster/store.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/runtime/retention.rs`
- Test: inline Taskmaster/store tests

**Interfaces:**
- Consumes: immutable `WorkflowObservation` values and session summary snapshots.
- Produces: canonical `Recommendation`, passive `WorkflowImprovement`, embedded `WorkflowObservationEvidence`, `RecommendationStore::{open,list,get,put,dismiss,delete_referencing_session,scrub_orphans}`.

- [ ] **Step 1: Write failing serialization, atomic persistence, and orphan tests**

Pin every canonical field, `type: "improve_workflow"`, `requiresApproval: false`, nullable action fields, reachable passive statuses, evidence snapshots, dismissal watermark, restart loading, one proposed recommendation per dedupe family, and deletion/scrub by embedded session ID.

- [ ] **Step 2: Run focused tests and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store::tests -- --nocapture`

Expected: FAIL because Taskmaster types/store do not exist.

- [ ] **Step 3: Implement the shared contract**

Use this canonical base and passive payload shape:

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecommendationType { ImproveWorkflow }
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecommendationStatus {
    Proposed, Accepted, Executing, Completed, Dismissed, Superseded, Expired, Failed,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecommendationConfidence { Low, Medium, High }
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetSurface { Instructions, Skill, Test, Tooling, Documentation }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DismissalWatermark {
    pub dismissed_at: String,
    pub dismissed_through_sequence: u64,
    pub observation_ids: Vec<String>,
    pub qualifying_count: usize,
    pub highest_impact: Impact,
    pub affected_session_ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowObservationEvidence {
    pub observation_id: String,
    pub sequence: u64,
    pub session_id: String,
    pub kind: ObservationKind,
    pub description: String,
    pub evidence: String,
    pub reported_impact: Impact,
    pub source: ObservationSource,
    pub confidence: f64,
    pub observed_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Recommendation {
    pub id: String,
    pub workspace_id: String,
    pub chain_id: String,
    pub chain_depth: u32,
    #[serde(rename = "type")]
    pub recommendation_type: RecommendationType,
    pub status: RecommendationStatus,
    pub priority: Impact,
    pub title: String,
    pub summary: String,
    pub reason: Vec<String>,
    pub evidence: Vec<WorkflowObservationEvidence>,
    pub source_session_ids: Vec<String>,
    pub target_session_id: Option<String>,
    pub suggested_harness_id: Option<String>,
    pub suggested_model: Option<String>,
    pub suggested_working_directory: Option<String>,
    pub suggested_prompt: Option<String>,
    pub confidence: RecommendationConfidence,
    pub requires_approval: bool,
    pub dedupe_key: String,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>,
    pub workflow_improvement: WorkflowImprovement,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowImprovement {
    pub proposed_improvement: String,
    pub target_surface: TargetSurface,
    pub observation_ids: Vec<String>,
    pub recurrence_count: usize,
    pub affected_session_ids: Vec<String>,
    pub impact: Impact,
    pub expected_benefit: String,
    pub supersedes_recommendation_id: Option<String>,
    pub dismissal_watermark: Option<DismissalWatermark>,
}
```

`WorkflowObservationEvidence` must copy ID, sequence, session ID, kind, description, evidence text, impact, source, confidence, and observed time. Recommendation confidence is `high` only when every qualifying cited observation is at least `0.8`; otherwise `medium`.

- [ ] **Step 4: Implement atomic workspace recommendation storage**

Persist one JSON file per recommendation under `recommendations/`, using temp-file sync, atomic replace, and parent-directory sync. Dismiss in place with `dismissedAt` and `dismissedThroughSequence`; retain immutable evidence. Add the store to `WorkspaceState`, call `delete_referencing_session` from Task 6's callback, and run `scrub_orphans` before recommendations are readable after workspace startup.
The scrub compares embedded evidence session IDs with retained session metadata;
it must not delete a recommendation merely because ordinary size trimming
removed the original observation record.

- [ ] **Step 5: Run tests and commit**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store::tests -- --nocapture`

Expected: PASS.

```bash
rtk git add crates/orkworksd/src/taskmaster crates/orkworksd/src/main.rs crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/src/runtime/retention.rs
rtk git commit -m "feat: persist passive Taskmaster recommendations"
```

### Task 8: Evaluate exact clusters and persist dismissal-safe successors

**Files:**
- Create: `crates/orkworksd/src/taskmaster/evaluator.rs`
- Modify: `crates/orkworksd/src/taskmaster/mod.rs`
- Modify: `crates/orkworksd/src/workflow_observations.rs`
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Modify: `crates/orkworksd/src/http/workflow_observation_handlers.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Test: inline evaluator/coordinator tests

**Interfaces:**
- Consumes: ordered `workspace_observations()`, canonical recommendation store, valid current summaries.
- Produces: `evaluate_workflow_improvements`, `schedule_evaluation`, `refresh_now`, `TaskmasterWorkspaceSnapshot`, and `append_current_work_to_handoff`.

- [ ] **Step 1: Write failing rule and lifecycle tests**

Cover: one ordinary observation quiet; two exact fingerprints qualify; different fingerprints never combine; one confident high-impact event qualifies; low confidence excluded; all seven kind/target/template mappings; counts/sessions derived only from evidence; proposed records update; dismissed records stay quiet on count-only change; increased impact resurfaces; two post-watermark observations including a new session create exactly one successor; 1,001st source record can trim while proposed/dismissed snapshots remain expandable; restart produces no duplicate.

- [ ] **Step 2: Run focused tests and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::evaluator::tests -- --nocapture`

Expected: FAIL because the evaluator does not exist.

- [ ] **Step 3: Implement deterministic rules and templates**

Use one exhaustive mapping:

```rust
fn rule(kind: ObservationKind) -> (TargetSurface, &'static str) {
    match kind {
        ObservationKind::Repetition => (TargetSurface::Tooling, "Automate or remove repeated work"),
        ObservationKind::Obstacle => (TargetSurface::Tooling, "Remove or document the obstacle"),
        ObservationKind::MissingContext => (TargetSurface::Instructions, "Add missing repository context"),
        ObservationKind::Assumption => (TargetSurface::Instructions, "Make the required assumption explicit"),
        ObservationKind::Correction => (TargetSurface::Instructions, "Prevent this recurring correction"),
        ObservationKind::Workaround => (TargetSurface::Tooling, "Replace the workaround with a supported path"),
        ObservationKind::VerificationGap => (TargetSurface::Test, "Add reliable verification for"),
    }
}

fn expected_benefit(target: TargetSurface) -> &'static str {
    match target {
        TargetSurface::Instructions => "Agents receive the required context before acting.",
        TargetSurface::Skill => "Agents follow one repeatable workflow for this task.",
        TargetSurface::Test => "The workflow gains repeatable verification.",
        TargetSurface::Tooling => "Agents spend less time on avoidable manual recovery.",
        TargetSurface::Documentation => "The supported workflow becomes discoverable.",
    }
}
```

Group only by fingerprint, filter qualifying evidence before counting/copying, derive dedupe as `improve_workflow:v1:<target>:<fingerprint>`, and compare resurfacing with sequence watermarks rather than time/UUID order.
Build `proposedImprovement` as `<rule prefix>: <normalized description>`, set
the title to `Improve <target surface>`, set the summary to the proposed
improvement, and build reason strings only from the computed qualifying count,
affected session count, highest impact, and source mix.

- [ ] **Step 4: Add debounce, restart reconstruction, and summary handoff input**

Increment a coordinator generation on each accepted observation, sleep five seconds, and evaluate only if the generation is still current. `refresh_now` bypasses the debounce. On workspace open, scrub then evaluate persisted observations. Build `TaskmasterWorkspaceSnapshot.current_work` only when all summary provenance fields exist, and implement:

```rust
pub(crate) fn append_current_work_to_handoff(prompt: &mut String, current: Option<&CurrentWork>) {
    if let Some(current) = current {
        prompt.push_str("\nCurrent work: ");
        prompt.push_str(&current.summary);
    }
}
```

Test that provenance-less legacy summaries are omitted, a current summary is included with source/observed-time evidence, and an ended session retains its last current-work handoff context.

- [ ] **Step 5: Trigger evaluation from both recording adapters and run tests**

Call `schedule_evaluation` only after `RecordOutcome::Accepted`, not duplicates/rejections. Run:

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
rtk git add crates/orkworksd/src/taskmaster crates/orkworksd/src/workflow_observations.rs crates/orkworksd/src/runtime/peon_runtime.rs crates/orkworksd/src/http
rtk git commit -m "feat: recommend deterministic workflow improvements"
```

### Task 9: Expose Taskmaster read, dismiss, and refresh APIs

**Files:**
- Create: `crates/orkworksd/src/http/taskmaster_handlers.rs`
- Modify: `crates/orkworksd/src/http/mod.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: inline router/handler tests

**Interfaces:**
- Consumes: Taskmaster coordinator/store facade only.
- Produces: `GET /taskmaster/recommendations`, `GET /taskmaster/recommendations/:id`, `POST /taskmaster/recommendations/:id/dismiss`, `POST /taskmaster/recommendations/:id/refresh`.

- [ ] **Step 1: Write failing API tests**

Pin list response `{ recommendations, diagnostics }`; detail `404`; dismiss proposed recommendation with optional bounded reason; repeated dismiss idempotency; refresh reevaluation; workspace conflict; and absence of an accept/execute operation for `improve_workflow`.

- [ ] **Step 2: Run tests and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_http -- --nocapture`

Expected: FAIL because routes/handlers do not exist.

- [ ] **Step 3: Implement thin handlers and router entries**

Use this list envelope so corruption is visible without breaking cards:

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecommendationListResponse {
    recommendations: Vec<Recommendation>,
    diagnostics: Vec<ObservationDiagnostic>,
}
```

Handlers must obtain the active workspace, delegate to Taskmaster, and map typed errors to `404`, `409`, `422`, or `500`; no handler opens metadata files or reimplements eligibility.
Run synchronous store/evaluator work through `spawn_blocking` and never hold a
standard mutex across an `.await`.

- [ ] **Step 4: Run tests and commit**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_http -- --nocapture`

Expected: PASS.

```bash
rtk git add crates/orkworksd/src/http crates/orkworksd/src/main.rs
rtk git commit -m "feat: expose Taskmaster workflow recommendations"
```

### Task 10: Render evidence-backed recommendations with Dismiss only

**Files:**
- Create: `apps/desktop/src/taskmaster.ts`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/components/RecommendationsPanel.tsx`
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/App.css`
- Create: `apps/desktop/tests/taskmaster.test.ts`
- Modify: `apps/desktop/tests/api.test.ts`
- Modify: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- Consumes: Taskmaster list/dismiss/refresh HTTP responses.
- Produces: typed API client, presentation helpers, polling cards, evidence disclosure, diagnostics, and optimistic-safe dismissal refresh.

- [ ] **Step 1: Write failing API and presentation tests**

Define TypeScript discriminants matching Rust:

```ts
export type Impact = "low" | "medium" | "high";
export type RecommendationConfidence = "low" | "medium" | "high";
export type RecommendationStatus =
  | "proposed" | "accepted" | "executing" | "completed"
  | "dismissed" | "superseded" | "expired" | "failed";
export type TargetSurface = "instructions" | "skill" | "test" | "tooling" | "documentation";
export interface WorkflowObservationEvidence {
  observationId: string;
  sequence: number;
  sessionId: string;
  kind: "repetition" | "obstacle" | "missing_context" | "assumption" |
    "correction" | "workaround" | "verification_gap";
  description: string;
  evidence: string;
  reportedImpact: Impact;
  source: "agent" | "peon";
  confidence: number;
  observedAt: string;
}
export interface RecommendationBase {
  id: string;
  workspaceId: string;
  chainId: string;
  chainDepth: number;
  status: RecommendationStatus;
  priority: Impact;
  title: string;
  summary: string;
  reason: string[];
  evidence: WorkflowObservationEvidence[];
  sourceSessionIds: string[];
  targetSessionId: string | null;
  suggestedHarnessId: string | null;
  suggestedModel: string | null;
  suggestedWorkingDirectory: string | null;
  suggestedPrompt: string | null;
  confidence: RecommendationConfidence;
  dedupeKey: string;
  createdAt: string;
  updatedAt: string;
  expiresAt: string | null;
}
export interface DismissalWatermark {
  dismissedAt: string;
  dismissedThroughSequence: number;
  observationIds: string[];
  qualifyingCount: number;
  highestImpact: Impact;
  affectedSessionIds: string[];
}
export interface WorkflowImprovement {
  proposedImprovement: string;
  targetSurface: TargetSurface;
  observationIds: string[];
  recurrenceCount: number;
  affectedSessionIds: string[];
  impact: Impact;
  expectedBenefit: string;
  supersedesRecommendationId: string | null;
  dismissalWatermark: DismissalWatermark | null;
}
export interface WorkflowRecommendation extends RecommendationBase {
  type: "improve_workflow";
  requiresApproval: false;
  workflowImprovement: WorkflowImprovement;
}
export interface ObservationDiagnostic {
  code: string;
  message: string;
  sessionId: string | null;
}
export interface RecommendationListResponse {
  recommendations: WorkflowRecommendation[];
  diagnostics: ObservationDiagnostic[];
}
```

Test camelCase decoding, target/impact/confidence labels, recurrence copy for one vs multiple sessions, sorted evidence by sequence, and API failure messages. Update Dockview source tests to require card/evidence/dismiss rendering and forbid accept/start/edit controls.

- [ ] **Step 2: Run desktop tests and verify failure**

Run: `rtk node --experimental-strip-types --test apps/desktop/tests/taskmaster.test.ts apps/desktop/tests/api.test.ts apps/desktop/tests/dockview.test.ts`

Expected: FAIL because types/client/cards do not exist.

- [ ] **Step 3: Implement typed API functions and pure helpers**

Add:

```ts
export async function getTaskmasterRecommendations(baseUrl: string): Promise<RecommendationListResponse>;
export async function dismissTaskmasterRecommendation(baseUrl: string, id: string, reason?: string): Promise<void>;
export async function refreshTaskmasterRecommendation(baseUrl: string, id: string): Promise<void>;
```

Throw status-bearing errors on non-2xx responses. Keep formatting functions in `taskmaster.ts` so they can be tested without rendering.

- [ ] **Step 4: Implement the Recommendations panel**

Resolve the backend URL on mount; fetch immediately and every five seconds while mounted; cancel state updates after unmount; show empty state only when there are no cards/diagnostics; render proposed cards with proposal, target, reason, impact, confidence, expected benefit, recurrence/sessions, `<details>` evidence rows, and `Dismiss`. Disable only the card being dismissed, refetch after success, and keep the card with an inline error after failure. Do not render accept, execute, start-session, issue, or edit controls.
Change `RecPanel` in `DockviewApp.tsx` to read `onSelectSession` from
`DockviewContext` and pass it to `RecommendationsPanel`; affected-session and
evidence-session buttons call that callback so they use the existing single
active-context switch rather than opening another terminal.

```tsx
function RecPanel() {
  const ctx = useContext(DockviewContext);
  return <RecommendationsPanel onSelectSession={ctx.onSelectSession} />;
}
```

- [ ] **Step 5: Run type-check/tests and commit**

Run: `rtk pnpm --dir apps/desktop exec tsc --noEmit`

Expected: exit 0.

Run: `rtk node --experimental-strip-types --test apps/desktop/tests/taskmaster.test.ts apps/desktop/tests/api.test.ts apps/desktop/tests/dockview.test.ts`

Expected: PASS.

```bash
rtk git add apps/desktop/src apps/desktop/tests
rtk git commit -m "feat: present workflow improvement recommendations"
```

### Task 11: Prove the feedback loop end to end and hand it off

**Files:**
- Modify if drift is reported: `docs/agents/architecture.md`
- Modify if drift is reported: `docs/agents/domain-entities.md`
- Modify if drift is reported: `AGENTS.md`
- Modify if drift is reported: `README.md`
- Test: all Rust and desktop suites

**Interfaces:**
- Consumes: all prior tasks.
- Produces: verified feature branch and review evidence suitable for a PR.

- [ ] **Step 1: Add the end-to-end Rust acceptance test**

In the Taskmaster HTTP test module, create two live sessions in one workspace, record the same `missing_context` description once through the agent adapter and once through the Peon adapter, refresh Taskmaster, and assert exactly one proposed `improve_workflow` recommendation with two evidence snapshots/two session IDs and target `instructions`. Dismiss it, restart stores, refresh, and assert it stays dismissed.

- [ ] **Step 2: Run the complete verification matrix**

Run: `rtk cargo build --manifest-path crates/orkworksd/Cargo.toml`

Expected: exit 0.

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: all tests pass.

Run: `rtk pnpm --dir apps/desktop exec tsc --noEmit`

Expected: exit 0.

Run: `rtk node --experimental-strip-types --test apps/desktop/tests/*.test.ts apps/desktop/tests/*.test.mjs`

Expected: all desktop tests pass.

Run: `rtk pnpm --dir docs build`

Expected: VitePress exits 0.

- [ ] **Step 3: Run repository completion guardrails**

Run: `rtk git diff --check`

Expected: no output, exit 0.

Run: `rtk bash .claude/hooks/doc-check.sh`

Expected: no unaddressed doc drift.

Run: `rtk bash .claude/hooks/worktree-check.sh`

Expected: this branch is current; report unrelated owner worktrees without modifying them.

- [ ] **Step 4: Request the required code review**

Invoke `requesting-code-review` and run a medium-effort `/code-review` because this changes protocol/schema, lifecycle, persistence, concurrency, and security-sensitive capability handling. Address every finding or record the evidence-backed reason it is intentional.

- [ ] **Step 5: Commit review/doc corrections and prepare the PR**

```bash
rtk git add AGENTS.md README.md docs crates/orkworksd apps/desktop
rtk git commit -m "test: verify workflow observation feedback loop"
```

If there are no final corrections or new acceptance-test changes, do not create an empty commit. Invoke `finishing-a-development-branch`, push the branch, and open a draft PR linked to the implementation issue with the verification commands and `/code-review` result in its description.
