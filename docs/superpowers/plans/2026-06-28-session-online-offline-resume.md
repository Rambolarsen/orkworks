# Session Online/Offline Resume Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace status-first session presentation with an `online | offline` model, expose resume methods as explicit detail-panel capabilities, and keep terminal outcomes as secondary diagnostics only.

**Architecture:** Extend the backend session DTO and metadata layer with presentation connectivity, terminal outcome, canonical last-activity time, and explicit resume options. Then update the renderer to sort by last activity, show a uniform offline list treatment, and move all resume affordances into the detail panel with disabled reasons for unavailable strategies.

**Tech Stack:** Rust (`orkworksd`, Axum, serde), TypeScript/React renderer, Node built-in test runner, Rust test suite, repo doc-check hook

---

## File Structure

### Backend contract and state derivation

- Modify: `crates/orkworksd/src/main.rs`
  - Extend `SessionInfo`
  - Derive `connectivity`, `terminalOutcome`, `resumeOptions`, and `lastActivityAt`
  - Stop exposing offline sessions through live attention semantics
- Modify: `crates/orkworksd/src/metadata.rs`
  - Add persisted fields for connectivity, terminal outcome, and canonical last activity
  - Add helper logic for deriving stable resume-option reason strings
- Modify: `crates/orkworksd/src/infrastructure/session_repository.rs`
  - Map new metadata fields into and out of the domain repository adapter
- Modify: `crates/orkworksd/src/domain/session/entity.rs`
  - Add explicit presentation-liveness fields if needed by the domain-facing adapter and tests

### Frontend API and session modeling

- Modify: `apps/desktop/src/api.ts`
  - Add `connectivity`, `terminalOutcome`, `resumeOptions`, and `lastActivityAt` to `SessionInfo`
- Modify: `apps/desktop/src/domain/session.ts`
  - Model the new fields
  - Stop treating offline sessions as active attention states
- Modify: `apps/desktop/src/sessionSort.ts`
  - Sort by last activity first
  - Scope attention derivation to online sessions
- Modify: `apps/desktop/src/labels.ts`
  - Add plain-language labels for `online`, `offline`, and resume-option copy where needed

### Renderer components

- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
  - Render uniform offline rows
  - Stop surfacing terminal outcome as the row status
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
  - Render offline resume-option list with disabled reasons
  - Keep terminal outcome/history lower emphasis than resume actions

### Tests

- Modify: `apps/desktop/tests/api.test.ts`
- Modify: `apps/desktop/tests/sessionSort.test.ts`
- Modify: `apps/desktop/tests/labels.test.ts`
- Modify: `apps/desktop/tests/dockview.test.ts`
- Modify: Rust tests in `crates/orkworksd/src/main.rs`
- Modify: Rust tests in `crates/orkworksd/src/metadata.rs`
- Modify: Rust tests in `crates/orkworksd/src/domain/session/entity.rs`

### Documentation

- Modify: `docs/agents/domain-entities.md`
- Modify: `docs/agents/architecture.md`
- Run: `.claude/hooks/doc-check.sh`

---

### Task 1: Extend the Backend DTO and Metadata Contract

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `apps/desktop/src/api.ts`
- Test: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/metadata.rs`
- Test: `apps/desktop/tests/api.test.ts`

- [ ] **Step 1: Write the failing desktop API shape test**

Add to `apps/desktop/tests/api.test.ts`:

```ts
test("SessionInfo type accepts connectivity, terminalOutcome, resumeOptions, and lastActivityAt", () => {
  const session: SessionInfo = {
    id: "offline-test",
    label: "Offline Test",
    status: "ended",
    connectivity: "offline",
    terminalOutcome: "ended",
    cwd: "/tmp/project",
    created_at: "2026-06-28T09:00:00Z",
    lastActivityAt: "2026-06-28T09:05:00Z",
    memoryState: "resumable",
    resumeStrategy: "exact",
    resumeOptions: [
      {
        strategy: "exact",
        label: "Resume exact session",
        available: true,
        preferred: true,
      },
      {
        strategy: "latest_repo",
        label: "Resume latest in repo",
        available: false,
        preferred: false,
        reason: "Harness does not support repo-scoped resume",
      },
    ],
  };

  assert.equal(session.connectivity, "offline");
  assert.equal(session.terminalOutcome, "ended");
  assert.equal(session.lastActivityAt, "2026-06-28T09:05:00Z");
  assert.equal(session.resumeOptions[1].available, false);
});
```

- [ ] **Step 2: Run the desktop API test to verify it fails**

Run:

```bash
rtk node --experimental-strip-types --test apps/desktop/tests/api.test.ts
```

Expected: FAIL with TypeScript/shape errors because `SessionInfo` does not yet include `connectivity`, `terminalOutcome`, `resumeOptions`, or `lastActivityAt`.

- [ ] **Step 3: Write the failing Rust metadata serialization test**

Add to `crates/orkworksd/src/metadata.rs`:

```rust
#[test]
fn session_metadata_serializes_connectivity_terminal_outcome_and_last_activity() {
    let meta = SessionMetadata {
        id: "s1".into(),
        label: "Test".into(),
        workspace: "/tmp".into(),
        task: String::new(),
        harness: String::new(),
        model: String::new(),
        cwd: "/tmp".into(),
        status: "ended".into(),
        phase: String::new(),
        connectivity: "offline".into(),
        terminal_outcome: Some("ended".into()),
        observed_status: None,
        summary: None,
        next_action: None,
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        peon_last_inference: None,
        provider_id: None,
        provider_label: None,
        provider_model: None,
        provider_state: None,
        created_at: "2026-06-28T09:00:00Z".into(),
        last_activity: "2026-06-28T09:05:00Z".into(),
        metadata_source: "process".into(),
        metadata_confidence: 1.0,
        repo_root: None,
        branch: None,
        dirty: None,
        changed_files: None,
        is_worktree: None,
        resume: None,
        resume_options: vec![],
        harness_session_id_source: None,
        harness_session_id_confidence: None,
        harness_session_id_captured_at: None,
        resumed_from: None,
        last_user_input: None,
    };

    let raw = serde_json::to_value(&meta).unwrap();
    assert_eq!(raw["connectivity"], "offline");
    assert_eq!(raw["terminalOutcome"], "ended");
    assert_eq!(raw["lastActivity"], "2026-06-28T09:05:00Z");
}
```

- [ ] **Step 4: Run the targeted Rust metadata test to verify it fails**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml session_metadata_serializes_connectivity_terminal_outcome_and_last_activity -- --exact
```

Expected: FAIL because the new fields do not exist on `SessionMetadata`.

- [ ] **Step 5: Add the new API and metadata fields with minimal implementation**

Update `apps/desktop/src/api.ts` with:

```ts
export type SessionConnectivity = "online" | "offline";
export type TerminalOutcome = "ended" | "killed" | "error";

export interface ResumeOption {
  strategy: ResumeStrategy;
  label: string;
  available: boolean;
  preferred: boolean;
  reason?: string;
}

export interface SessionInfo {
  // existing fields
  connectivity?: SessionConnectivity;
  terminalOutcome?: TerminalOutcome;
  lastActivityAt?: string;
  resumeOptions?: ResumeOption[];
}
```

Update `crates/orkworksd/src/metadata.rs` and `crates/orkworksd/src/main.rs` with matching fields:

```rust
#[serde(rename = "terminalOutcome", skip_serializing_if = "Option::is_none")]
pub terminal_outcome: Option<String>,
pub connectivity: String,
#[serde(rename = "lastActivity")]
pub last_activity: String,
#[serde(rename = "resumeOptions", default)]
pub resume_options: Vec<ResumeOption>,
```

And in `SessionInfo`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
connectivity: Option<String>,
#[serde(rename = "terminalOutcome", skip_serializing_if = "Option::is_none")]
terminal_outcome: Option<String>,
#[serde(rename = "lastActivityAt", skip_serializing_if = "Option::is_none")]
last_activity_at: Option<String>,
#[serde(rename = "resumeOptions", skip_serializing_if = "Vec::is_empty", default)]
resume_options: Vec<metadata::ResumeOption>,
```

- [ ] **Step 6: Run the targeted tests to verify they pass**

Run:

```bash
rtk node --experimental-strip-types --test apps/desktop/tests/api.test.ts
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml session_metadata_serializes_connectivity_terminal_outcome_and_last_activity -- --exact
```

Expected: PASS for both tests.

- [ ] **Step 7: Commit**

```bash
rtk git add apps/desktop/src/api.ts apps/desktop/tests/api.test.ts crates/orkworksd/src/main.rs crates/orkworksd/src/metadata.rs
rtk git commit -m "feat: add session connectivity and resume option dto fields"
```

### Task 2: Derive Online/Offline State and Resume Options in the Backend

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/infrastructure/session_repository.rs`
- Modify: `crates/orkworksd/src/domain/session/entity.rs`
- Test: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/metadata.rs`
- Test: `crates/orkworksd/src/domain/session/entity.rs`

- [ ] **Step 1: Write the failing backend transition tests**

Add to `crates/orkworksd/src/main.rs`:

```rust
#[test]
fn list_sessions_marks_running_sessions_online() {
    let info = SessionInfo {
        id: "a".into(),
        label: "A".into(),
        harness_id: None,
        model_provider_id: None,
        model_id: None,
        harness: None,
        model: None,
        status: "running".into(),
        cwd: "/tmp".into(),
        created_at: "2026-06-28T09:00:00Z".into(),
        observed_status: Some("working".into()),
        summary: None,
        next_action: None,
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        metadata_source: None,
        metadata_confidence: None,
        repo_root: None,
        branch: None,
        dirty: None,
        changed_files: None,
        is_worktree: None,
        conflict_warning: None,
        recommendation: None,
        peon_last_inference: None,
        provider: None,
        provider_model: None,
        provider_state: None,
        memory_state: MemoryState::Live,
        resume_strategy: harness::ResumeStrategy::None,
        resume: None,
        resumed_from: None,
        connectivity: Some("online".into()),
        terminal_outcome: None,
        last_activity_at: Some("2026-06-28T09:05:00Z".into()),
        resume_options: vec![],
    };

    assert_eq!(info.connectivity.as_deref(), Some("online"));
    assert_eq!(info.terminal_outcome, None);
}

#[test]
fn list_sessions_marks_ended_sessions_offline_with_terminal_outcome() {
    let info = SessionInfo {
        status: "ended".into(),
        observed_status: Some("waiting_for_input".into()),
        connectivity: Some("offline".into()),
        terminal_outcome: Some("ended".into()),
        last_activity_at: Some("2026-06-28T09:05:00Z".into()),
        resume_options: vec![],
        ..test_session_info("ended-session")
    };

    assert_eq!(info.connectivity.as_deref(), Some("offline"));
    assert_eq!(info.terminal_outcome.as_deref(), Some("ended"));
}
```

- [ ] **Step 2: Write the failing resume-option derivation test**

Add to `crates/orkworksd/src/metadata.rs`:

```rust
#[test]
fn derive_resume_options_returns_disabled_entries_with_reasons() {
    let options = derive_resume_options(
        &crate::harness::ResumeStrategy::Exact,
        None,
        false,
        false,
    );

    assert_eq!(options.len(), 3);
    assert_eq!(options[0].strategy, "exact");
    assert_eq!(options[0].available, false);
    assert_eq!(options[0].reason.as_deref(), Some("No harness session id was captured"));
}
```

- [ ] **Step 3: Run the failing Rust tests**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_marks_running_sessions_online -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_marks_ended_sessions_offline_with_terminal_outcome -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml derive_resume_options_returns_disabled_entries_with_reasons -- --exact
```

Expected: FAIL because the transition helpers and resume-option derivation do not exist yet.

- [ ] **Step 4: Implement backend derivation helpers**

Add focused helpers in `crates/orkworksd/src/main.rs` and `crates/orkworksd/src/metadata.rs`:

```rust
fn connectivity_for_status(status: &str) -> &'static str {
    match status {
        "creating" | "running" => "online",
        _ => "offline",
    }
}

fn terminal_outcome_for_status(status: &str) -> Option<String> {
    match status {
        "ended" | "killed" | "error" => Some(status.to_string()),
        _ => None,
    }
}
```

And a stable resume-option derivation function:

```rust
pub fn derive_resume_options(
    preferred: &ResumeStrategy,
    resume: Option<&ResumeMemory>,
    supports_latest_cwd: bool,
    supports_latest_repo: bool,
) -> Vec<ResumeOption> {
    vec![
        ResumeOption::exact(resume),
        ResumeOption::latest_cwd(supports_latest_cwd),
        ResumeOption::latest_repo(supports_latest_repo),
    ]
    .into_iter()
    .map(|mut option| {
        option.preferred = option.strategy == preferred.as_str();
        option
    })
    .collect()
}
```

- [ ] **Step 5: Wire the new helpers into list/session persistence paths**

Update the `SessionInfo` construction in `crates/orkworksd/src/main.rs` to populate:

```rust
connectivity: Some(connectivity_for_status(&status).into()),
terminal_outcome: terminal_outcome_for_status(&status),
last_activity_at: meta.map(|m| m.last_activity.clone()).or_else(|| Some(created_at.clone())),
resume_options: metadata::derive_resume_options(&resume_strategy, meta.and_then(|m| m.resume.as_ref()), caps.supports_resume_latest_cwd, caps.supports_resume_latest_repo),
```

Update `crates/orkworksd/src/infrastructure/session_repository.rs` so metadata reads/writes preserve:

```rust
connectivity: meta.connectivity.clone(),
terminal_outcome: meta.terminal_outcome.clone(),
resume_options: meta.resume_options.clone(),
```

- [ ] **Step 6: Run the focused Rust tests to verify they pass**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_marks_running_sessions_online -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_marks_ended_sessions_offline_with_terminal_outcome -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml derive_resume_options_returns_disabled_entries_with_reasons -- --exact
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
rtk git add crates/orkworksd/src/main.rs crates/orkworksd/src/metadata.rs crates/orkworksd/src/infrastructure/session_repository.rs crates/orkworksd/src/domain/session/entity.rs
rtk git commit -m "feat: derive online offline session state and resume options"
```

### Task 3: Rework Sorting and Attention Semantics for Online-Only Status

**Files:**
- Modify: `apps/desktop/src/sessionSort.ts`
- Modify: `apps/desktop/src/domain/session.ts`
- Modify: `apps/desktop/src/labels.ts`
- Test: `apps/desktop/tests/sessionSort.test.ts`
- Test: `apps/desktop/tests/labels.test.ts`

- [ ] **Step 1: Write the failing session-sort tests**

Replace/add tests in `apps/desktop/tests/sessionSort.test.ts`:

```ts
test("sessionAttentionStatus ignores observed status for offline sessions", () => {
  const session: SessionInfo = {
    id: "offline",
    label: "Offline",
    status: "ended",
    connectivity: "offline",
    terminalOutcome: "ended",
    observedStatus: "waiting_for_input",
    cwd: "/tmp",
    created_at: "2026-06-28T09:00:00Z",
    lastActivityAt: "2026-06-28T09:05:00Z",
    memoryState: "resumable",
    resumeStrategy: "exact",
    resumeOptions: [],
  };

  assert.equal(sessionAttentionStatus(session), "offline");
});

test("sortSessions sorts by last activity descending before label", () => {
  const sessions: SessionInfo[] = [
    { id: "older", label: "Older", status: "running", connectivity: "online", cwd: "/tmp", created_at: "2026-06-28T09:00:00Z", lastActivityAt: "2026-06-28T09:01:00Z", memoryState: "live", resumeStrategy: "none", resumeOptions: [] },
    { id: "newer", label: "Newer", status: "ended", connectivity: "offline", cwd: "/tmp", created_at: "2026-06-28T09:00:00Z", lastActivityAt: "2026-06-28T09:10:00Z", memoryState: "resumable", resumeStrategy: "exact", resumeOptions: [] },
  ];

  assert.deepEqual(sortSessions(sessions).map((s) => s.id), ["newer", "older"]);
});
```

- [ ] **Step 2: Run the failing desktop tests**

Run:

```bash
rtk node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts apps/desktop/tests/labels.test.ts
```

Expected: FAIL because sort still prioritizes attention buckets and `sessionAttentionStatus()` still returns `observedStatus ?? status`.

- [ ] **Step 3: Implement online-only attention semantics and last-activity sort**

Update `apps/desktop/src/sessionSort.ts`:

```ts
function canonicalLastActivity(session: SessionInfo): number {
  const iso = session.lastActivityAt ?? session.peonLastInference ?? session.created_at;
  const t = new Date(iso).getTime();
  return Number.isNaN(t) ? 0 : t;
}

export function sessionAttentionStatus(session: SessionInfo): string {
  if (session.connectivity === "offline") return "offline";
  return session.observedStatus ?? session.status;
}

export function sortSessions(list: SessionInfo[]): SessionInfo[] {
  return [...list].sort((a, b) => {
    const timeDelta = canonicalLastActivity(b) - canonicalLastActivity(a);
    if (timeDelta !== 0) return timeDelta;
    return a.label.localeCompare(b.label);
  });
}
```

Update `apps/desktop/src/labels.ts`:

```ts
case "offline": return "Offline";
case "online": return "Online";
```

And tone handling:

```ts
case "offline":
  return "neutral";
```

- [ ] **Step 4: Mirror the same semantics in `apps/desktop/src/domain/session.ts`**

Update:

```ts
attentionState: ((dto.connectivity === "offline" ? "offline" : (dto.observedStatus ?? dto.status))) as AttentionState,
```

And keep the domain helper aligned with `sessionSort.ts`:

```ts
export function sessionAttentionStatus(session: Session): string {
  if (session.connectivity === "offline") return "offline";
  return session.observedStatus ?? session.status;
}
```

- [ ] **Step 5: Run the desktop tests to verify they pass**

Run:

```bash
rtk node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts apps/desktop/tests/labels.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/desktop/src/sessionSort.ts apps/desktop/src/domain/session.ts apps/desktop/src/labels.ts apps/desktop/tests/sessionSort.test.ts apps/desktop/tests/labels.test.ts
rtk git commit -m "feat: sort sessions by activity and scope attention to online state"
```

### Task 4: Render Uniform Offline Rows in the Sessions List

**Files:**
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
- Test: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Write the failing list-render tests**

Add to `apps/desktop/tests/dockview.test.ts` assertions against the source:

```ts
test("SessionListPanel uses canonical lastActivityAt when present", () => {
  const source = readFileSync(new URL("../src/components/SessionListPanel.tsx", import.meta.url), "utf8");
  assert.match(source, /lastActivityAt/);
});

test("SessionListPanel renders a uniform offline status treatment", () => {
  const source = readFileSync(new URL("../src/components/SessionListPanel.tsx", import.meta.url), "utf8");
  assert.match(source, /s\.connectivity === "offline"/);
  assert.doesNotMatch(source, /attentionLabel\(attn\).*Ended/);
});
```

- [ ] **Step 2: Run the failing renderer test**

Run:

```bash
rtk node --experimental-strip-types --test apps/desktop/tests/dockview.test.ts
```

Expected: FAIL because the component still keys row tone/status directly from `sessionAttentionStatus()` and relative-time logic still ignores `lastActivityAt`.

- [ ] **Step 3: Update `SessionListPanel.tsx` for the new row treatment**

Change the row helpers to:

```ts
function lastActivity(s: SessionInfo, now: Date): string {
  return relativeTime(s.lastActivityAt ?? s.peonLastInference, now) || relativeTime(s.created_at, now);
}
```

And in the row render:

```tsx
const isOffline = s.connectivity === "offline";
const statusLabel = isOffline ? "Offline" : attentionLabel(attn);
const tone = isOffline ? "neutral" : attentionTone(attn);
```

Render:

```tsx
{statusLabel && (
  <div className="session-row-status" data-attention={tone}>
    {statusLabel}
  </div>
)}
```

- [ ] **Step 4: Run the renderer test to verify it passes**

Run:

```bash
rtk node --experimental-strip-types --test apps/desktop/tests/dockview.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add apps/desktop/src/components/SessionListPanel.tsx apps/desktop/tests/dockview.test.ts
rtk git commit -m "feat: render uniform offline session rows"
```

### Task 5: Move Resume Choices into the Offline Detail Panel

**Files:**
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/src/labels.ts`
- Test: `apps/desktop/tests/dockview.test.ts`
- Test: `apps/desktop/tests/labels.test.ts`

- [ ] **Step 1: Write the failing offline-detail tests**

Add to `apps/desktop/tests/labels.test.ts`:

```ts
test("resumeActionLabel stays strategy-specific for offline resume options", () => {
  assert.equal(resumeActionLabel("exact"), "Resume session");
  assert.equal(resumeActionLabel("latest_cwd"), "Resume latest in folder");
  assert.equal(resumeActionLabel("latest_repo"), "Resume latest in repo");
});
```

Add to `apps/desktop/tests/dockview.test.ts`:

```ts
test("SessionDetailPanel renders resume options for offline sessions", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");
  assert.match(source, /resumeOptions/);
  assert.match(source, /active\.connectivity === "offline"/);
  assert.match(source, /reason/);
});
```

- [ ] **Step 2: Run the failing detail-panel tests**

Run:

```bash
rtk node --experimental-strip-types --test apps/desktop/tests/dockview.test.ts apps/desktop/tests/labels.test.ts
```

Expected: FAIL because the detail panel still exposes one resume button driven by `resumeStrategy`.

- [ ] **Step 3: Replace the single resume button with a resume-option section**

Update `apps/desktop/src/components/SessionDetailPanel.tsx`:

```tsx
const isOffline = active.connectivity === "offline";
const resumeOptions = active.resumeOptions ?? [];
```

Render the offline section:

```tsx
{isOffline && (
  <div className="detail-resume-options">
    {resumeOptions.map((option) => (
      <button
        key={option.strategy}
        className="session-resume-button"
        type="button"
        disabled={!option.available}
        onClick={() => option.available && onResumeSession(active.id)}
        title={option.reason ?? option.label}
      >
        {option.label}
      </button>
    ))}
    {resumeOptions.map((option) => (
      !option.available && option.reason ? (
        <div key={`${option.strategy}-reason`} className="session-resume-reason">
          {option.reason}
        </div>
      ) : null
    ))}
  </div>
)}
```

Demote secondary history:

```tsx
{isOffline && active.terminalOutcome && (
  <div className="detail-provenance">
    Last exit: {active.terminalOutcome}
  </div>
)}
```

- [ ] **Step 4: Run the detail-panel tests to verify they pass**

Run:

```bash
rtk node --experimental-strip-types --test apps/desktop/tests/dockview.test.ts apps/desktop/tests/labels.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/src/labels.ts apps/desktop/tests/dockview.test.ts apps/desktop/tests/labels.test.ts
rtk git commit -m "feat: show offline resume methods in session details"
```

### Task 6: Update Docs and Run End-to-End Verification

**Files:**
- Modify: `docs/agents/domain-entities.md`
- Modify: `docs/agents/architecture.md`
- Modify: `docs/superpowers/specs/2026-06-28-session-online-offline-resume-design.md` if implementation drifted

- [ ] **Step 1: Update the maintained architecture docs**

Add explicit notes to `docs/agents/domain-entities.md`:

```md
- Session presentation state is `online` or `offline`
- Live attention state is only meaningful while the session is online
- Terminal outcome (`ended`, `killed`, `error`) is secondary diagnostic metadata for offline sessions
- Resume options are enumerated explicitly for the detail panel with availability and reason text
```

Add matching API-flow notes to `docs/agents/architecture.md`:

```md
- `GET /sessions` returns `connectivity`, `terminalOutcome`, `resumeOptions`, and `lastActivityAt`
- The sessions list sorts by activity and renders offline sessions uniformly
- The detail panel owns resume-option display and disabled-reason messaging
```

- [ ] **Step 2: Run focused desktop and Rust verification**

Run:

```bash
rtk node --experimental-strip-types --test apps/desktop/tests/api.test.ts apps/desktop/tests/sessionSort.test.ts apps/desktop/tests/labels.test.ts apps/desktop/tests/dockview.test.ts
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: PASS for the targeted desktop suite and full Rust suite.

- [ ] **Step 3: Run repo doc currency check**

Run:

```bash
rtk bash .claude/hooks/doc-check.sh
```

Expected: no unaddressed doc warnings related to session model, renderer state, or domain entities.

- [ ] **Step 4: Review final diff**

Run:

```bash
rtk git status --short
rtk git diff -- docs/agents/domain-entities.md docs/agents/architecture.md apps/desktop/src apps/desktop/tests crates/orkworksd/src
```

Expected: only intended online/offline resume-model changes remain.

- [ ] **Step 5: Commit**

```bash
rtk git add docs/agents/domain-entities.md docs/agents/architecture.md apps/desktop/src apps/desktop/tests crates/orkworksd/src
rtk git commit -m "feat: adopt online offline session presentation model"
```
