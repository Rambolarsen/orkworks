# Taskmaster Recommendation Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Fix with AI hand off a stable Taskmaster recommendation ID to the current session, let the agent resolve its evidence through the sidecar, and let the agent report verified completion through an authenticated callback.

**Architecture:** Reuse each recommendation's existing stable `id`. The desktop draft names the repo skill and local sidecar lookup contract; the sidecar owns reverse token lookup, target-session authorization, lifecycle transitions, and structured event provenance. Session history exposes the recommendation ID and lets the user reopen the corresponding recommendation record.

**Tech Stack:** Rust/Axum sidecar, Tokio tests, serde/NDJSON metadata, React/TypeScript desktop, Node test runner, committed Agent Skills `SKILL.md`.

**Spec:** `docs/superpowers/specs/2026-09-07-taskmaster-recommendation-handoff-design.md`

## Global Constraints

- Keep Fix with AI targeting the user's current live session; never create or resume a session.
- Agents must retrieve recommendation truth from `GET /taskmaster/recommendations/:id`, not from copied prompt prose or direct metadata-file edits.
- Completion authentication uses the existing per-session `ORKWORKS_REPORT_TOKEN`; never accept a caller-supplied target session or lifecycle status.
- `accepted` means prompt delivered; `completed` means the target agent explicitly reported verified work.
- Existing persisted recommendations and events must remain readable; do not infer completion for historical accepted work.
- Use pnpm for desktop package commands and the repository's existing Rust validation commands.
- Keep the Electron-main/renderer boundary unchanged.

---

### Task 1: Update the authoritative protocol documentation and add the agent skill

**Files:**
- Modify: `specs/taskmaster.md` in the Workflow-improvement recommendations, API, lifecycle, and acceptance-criteria sections.
- Modify: `specs/orkworks-mvp.md` in the explicit agent-reporting protocol section.
- Modify: `docs/agents/architecture.md` in the endpoint and Taskmaster implementation inventory.
- Modify: `AGENTS.md` in the repo-level skills list.
- Modify: `docs/agents/apm.md` in the committed repo-skills table.
- Create: `skills/working-on-recommendation/SKILL.md`.
- Test: `scripts/doc-check.sh` and `/Users/froomiebot/.codex/skills/.system/skill-creator/scripts/quick_validate.py`.

**Interfaces:**
- The skill consumes `ORKWORKS_PORT`, `ORKWORKS_SESSION_ID`, `ORKWORKS_REPORT_TOKEN`, and a recommendation ID included in the task prompt.
- The skill produces a verified recommendation completion report through `POST /taskmaster/recommendations/:id/complete`.

- [ ] **Step 1: Write the failing documentation/skill checks**

Add the new skill table entry and protocol text to the listed documentation files. Create the skill test invocation before the skill exists:

```bash
python3 /Users/froomiebot/.codex/skills/.system/skill-creator/scripts/quick_validate.py skills/working-on-recommendation
```

Expected: FAIL because the skill directory/file does not exist.

- [ ] **Step 2: Run the checks to verify the failure**

Run from the repository root:

```bash
python3 /Users/froomiebot/.codex/skills/.system/skill-creator/scripts/quick_validate.py skills/working-on-recommendation
```

Expected: a missing-path failure, not a Python or shell error.

- [ ] **Step 3: Write the skill and documentation contract**

Create a concise skill with this frontmatter and body:

```markdown
---
name: working-on-recommendation
description: Use when work starts from an OrkWorks Taskmaster recommendation or a prompt contains a recommendation ID.
---

# Working on a Taskmaster recommendation

1. Read the recommendation ID from the prompt and fetch the authoritative record:
   `curl --fail-with-body "http://127.0.0.1:${ORKWORKS_PORT}/taskmaster/recommendations/${RECOMMENDATION_ID}"`.
2. Inspect `workflowImprovement.targetSurface`, `workflowImprovement.proposedImprovement`, `evidence`, and `sourceSessionIds` before editing.
3. Follow the repository's normal branch, testing, and review instructions. Keep changes scoped to the target surface and do not edit recommendation files directly.
4. Run focused verification, then the desktop/Rust checks required by this repository for the change.
5. Only after verification succeeds, report completion:
   `curl --fail-with-body -X POST "http://127.0.0.1:${ORKWORKS_PORT}/taskmaster/recommendations/${RECOMMENDATION_ID}/complete" -H "Authorization: Bearer ${ORKWORKS_REPORT_TOKEN}" -H "Content-Type: application/json" --data '{"summary":"..."}'`.
6. If verification or the callback fails, explain the failure and do not claim the recommendation is completed.
```

Document that `sourceSessionIds` are evidence context, not permission to resume or modify those sessions, and that the callback has no caller-supplied session ID because the sidecar derives it from the token.

- [ ] **Step 4: Run the checks to verify they pass**

Run:

```bash
python3 /Users/froomiebot/.codex/skills/.system/skill-creator/scripts/quick_validate.py skills/working-on-recommendation
bash scripts/doc-check.sh
```

Expected: the skill validator succeeds and the documentation check reports no drift/dead-link errors.

- [ ] **Step 5: Commit**

```bash
git add AGENTS.md docs/agents/apm.md docs/agents/architecture.md specs/taskmaster.md specs/orkworks-mvp.md skills/working-on-recommendation/SKILL.md
git commit -m "docs: add recommendation work skill and protocol"
```

### Task 2: Add the sidecar lifecycle, capability, and event domain seams

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`.
- Modify: `crates/orkworksd/src/taskmaster/store.rs`.
- Modify: `crates/orkworksd/src/metadata.rs`.
- Modify: `crates/orkworksd/src/session_application.rs`.
- Modify: `crates/orkworksd/src/runtime/terminal_http.rs`.
- Test: the unit-test modules in those same files.

**Interfaces:**
- `workflow_report_session_for_token(candidate: &str) -> Option<String>` returns the live session ID for a valid reporting token using constant-time token comparison.
- `RecommendationStore::complete_accepted(id: &str, updated_at: String) -> Result<Option<Recommendation>, StoreError>` transitions only `Accepted` to `Completed`, returns `None` for unknown IDs, and returns `InvalidTransition` for other states.
- `SessionApplication::complete_recommendation(id: &str, session_id: &str, summary: Option<String>) -> Result<Option<Recommendation>, RecommendationCompleteError>` validates recommendation type/status/target session, appends a structured completion event, and persists the transition; a repeated request from the same target session returns the existing completed record without a duplicate event.
- `metadata::Event.recommendation_id: Option<String>` serializes as `recommendationId` and defaults to `None` for old event records.
- `SummaryLogQueryEntry.recommendation_id: Option<String>` carries the structured event field to the HTTP adapter.

- [ ] **Step 1: Write the failing tests**

Add tests for:

```rust
#[test]
fn workflow_report_session_for_token_returns_only_the_matching_live_session() {}

#[test]
fn complete_accepted_transitions_to_completed_and_preserves_target_session() {}

#[test]
fn complete_accepted_rejects_proposed_executing_dismissed_and_failed_states() {}

#[test]
fn old_events_without_recommendation_id_deserialize_as_none() {}

#[test]
fn complete_recommendation_records_id_and_summary_in_session_history() {}

#[test]
fn completed_recommendation_is_not_resurfaced_by_evaluation() {}
```

Use the existing token registry helpers and `test_app_state_with_workspace`; do not mock the recommendation store or token registry.

- [ ] **Step 2: Run the focused Rust tests to verify they fail**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store::tests::complete_accepted -- --exact
cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests::workflow_report_session_for_token -- --exact
```

Expected: compile/test failures because the new interfaces and fields do not exist.

- [ ] **Step 3: Implement the minimal domain behavior**

Add the reverse capability lookup beside `verify_workflow_report_token`, preserving constant-time comparison. Add the store transition with the same atomic write path as existing transitions. Extend `Event` and summary projection with optional recommendation IDs. In `SessionApplication`, lock the active workspace, validate `ImproveWorkflow + Accepted + targetSessionId == session_id`, call `complete_accepted`, and append:

```rust
metadata::Event {
    event_type: "taskmaster_fix_completed".into(),
    timestamp: iso_now(),
    status: "working".into(),
    observed_status: Some("working".into()),
    confidence: None,
    summary: Some(summary.unwrap_or_else(|| "Taskmaster fix completed.".into())),
    source: Some("agent".into()),
    recommendation_id: Some(id.to_string()),
}
```

Also add `recommendation_id: Some(id.to_string())` to the existing `taskmaster_fix_requested` event. Update every Rust `metadata::Event` constructor to set `recommendation_id: None` unless it is one of these Taskmaster events, and update `runtime/terminal_http.rs` to serialize the optional field as `recommendationId`. Keep the NDJSON event format backward-compatible.

- [ ] **Step 4: Run the focused Rust tests to verify they pass**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store::tests::complete_accepted
cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::complete_recommendation_records_id_and_summary_in_session_history
cargo test --manifest-path crates/orkworksd/Cargo.toml metadata::tests::old_events_without_recommendation_id_deserialize_as_none
```

Expected: all focused tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/runtime/terminal_runtime.rs crates/orkworksd/src/taskmaster/store.rs crates/orkworksd/src/metadata.rs crates/orkworksd/src/session_application.rs
git commit -m "feat: add authenticated recommendation completion state"
```

### Task 3: Expose and test the authenticated completion route

**Files:**
- Modify: `crates/orkworksd/src/http/taskmaster_handlers.rs`.
- Modify: `crates/orkworksd/src/main.rs`.
- Modify: `crates/orkworksd/src/session_application.rs` only if the error mapping needs a narrowly typed completion error.
- Test: `crates/orkworksd/src/http/taskmaster_handlers.rs`.

**Interfaces:**
- `POST /taskmaster/recommendations/:id/complete` accepts `Authorization: Bearer <ORKWORKS_REPORT_TOKEN>` and an optional JSON body `{ "summary": "..." }`.
- Successful completion returns the updated `Recommendation` JSON with `status: "completed"`.
- Missing/invalid token returns `401`; unknown recommendation returns `404`; wrong target session, wrong lifecycle, malformed/oversized summary, or stale workspace returns the existing conflict/validation status without mutation.

- [ ] **Step 1: Write the failing handler tests**

Add tests for:

```rust
#[tokio::test]
async fn complete_requires_a_valid_reporting_token() {}

#[tokio::test]
async fn complete_uses_the_token_session_and_rejects_a_different_target() {}

#[tokio::test]
async fn complete_returns_completed_recommendation_and_is_idempotent() {}

#[tokio::test]
async fn complete_rejects_unknown_or_non_accepted_recommendations() {}

#[tokio::test]
async fn complete_rejects_unknown_json_fields_and_overlong_summaries() {}
```

Create a proposed accepted recommendation in the test workspace, install a token with `set_workflow_report_token`, and exercise the handler directly with `State`, `Path`, `HeaderMap`, and `Json` as the existing workflow-observation handler tests do.

- [ ] **Step 2: Run the handler tests to verify they fail**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml http::taskmaster_handlers::tests::complete_requires_a_valid_reporting_token
```

Expected: compile/test failure because the route handler and router registration are absent.

- [ ] **Step 3: Implement the route and router registration**

Add a `#[serde(deny_unknown_fields)]` `CompleteRequest` with an optional summary and a bounded Unicode length. Parse the bearer token, resolve it to a session ID with `workflow_report_session_for_token`, then call `SessionApplication::complete_recommendation`. Add the handler import and route in `main.rs`:

```rust
.route(
    "/taskmaster/recommendations/:id/complete",
    post(complete_recommendation),
)
```

Map errors without leaking token values or recommendation contents in authentication failures. For an already completed recommendation, allow the same token/session to return the existing completed record without appending a duplicate completion event.

- [ ] **Step 4: Run the handler tests to verify they pass**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml http::taskmaster_handlers::tests::complete_
```

Expected: all completion-handler tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/http/taskmaster_handlers.rs crates/orkworksd/src/main.rs crates/orkworksd/src/session_application.rs
git commit -m "feat: expose recommendation completion endpoint"
```

### Task 4: Make the desktop handoff recommendation-aware

**Files:**
- Modify: `apps/desktop/src/taskmaster.ts`.
- Modify: `apps/desktop/src/App.tsx` only if the acceptance request needs the recommendation ID outside the prompt draft.
- Modify: `apps/desktop/src/api.ts` only for summary-log typing and any shared status label.
- Test: `apps/desktop/tests/taskmaster.test.ts` and `apps/desktop/tests/api.test.ts`.

**Interfaces:**
- `buildFixPromptDraft(recommendation)` includes the exact stable `recommendation.id`, the `working-on-recommendation` skill name, the local GET endpoint, and the authenticated completion instructions without embedding the bearer token.
- `SummaryLogEntry.recommendationId: string | null` is optional for old events.

- [ ] **Step 1: Write the failing desktop tests**

Extend the existing prompt test:

```ts
test("Taskmaster fix prompt identifies the recommendation and invokes the repo skill", () => {
  const prompt = buildFixPromptDraft(recommendation);
  assert.match(prompt, /rec-1/);
  assert.match(prompt, /working-on-recommendation/);
  assert.match(prompt, /taskmaster\/recommendations\/rec-1/);
  assert.match(prompt, /ORKWORKS_REPORT_TOKEN/);
  assert.match(prompt, /sourceSessionIds/);
});
```

Add an API/typing assertion that `SummaryLogEntry` accepts an omitted recommendation ID for old responses.

- [ ] **Step 2: Run the focused desktop tests to verify they fail**

```bash
cd apps/desktop
node --experimental-strip-types --test tests/taskmaster.test.ts tests/api.test.ts
```

Expected: the new prompt assertions fail because the recommendation ID and skill instructions are absent.

- [ ] **Step 3: Implement the prompt and API contract**

Keep the existing target-surface and no-other-session restrictions. Add a clear block like:

```text
Taskmaster recommendation ID: rec-1
Use the working-on-recommendation skill first. Treat the sidecar record as authoritative:
GET http://127.0.0.1:$ORKWORKS_PORT/taskmaster/recommendations/rec-1
Inspect workflowImprovement, evidence, and sourceSessionIds before acting.
After making and verifying the change, report completion with the authenticated completion endpoint using ORKWORKS_REPORT_TOKEN.
```

Use a literal placeholder in the displayed draft that the agent can resolve (`${RECOMMENDATION_ID}` in the sent prompt should be replaced with the actual recommendation ID by `buildFixPromptDraft`; do not send an unresolved placeholder). Preserve the existing `\r` append in `App.tsx`.

- [ ] **Step 4: Run the focused desktop tests to verify they pass**

```bash
cd apps/desktop
node --experimental-strip-types --test tests/taskmaster.test.ts tests/api.test.ts
npx tsc --noEmit
```

Expected: all focused tests and the TypeScript check pass.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/taskmaster.ts apps/desktop/src/api.ts apps/desktop/tests/taskmaster.test.ts apps/desktop/tests/api.test.ts
git commit -m "feat: include recommendation context in fix prompts"
```

### Task 5: Render session provenance and reopen recommendation history

**Files:**
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`.
- Modify: `apps/desktop/src/components/DockviewApp.tsx`.
- Modify: `apps/desktop/src/App.tsx`.
- Modify: `apps/desktop/src/components/RecommendationsPanel.tsx`.
- Modify: `apps/desktop/src/App.css`.
- Test: `apps/desktop/tests/taskmaster.test.ts` and the existing Dockview/session-detail tests.

**Interfaces:**
- `SessionDetailPanel` receives `onOpenRecommendation(recommendationId: string) => void`.
- `DockviewApp` passes that callback through its context to the detail panel.
- `App` activates the Recommendations panel and records a focused recommendation ID.
- `RecommendationsPanel` accepts an optional focused ID and keeps that record visible long enough to show its status, including `accepted` or `completed` history records.

- [ ] **Step 1: Write the failing UI wiring tests**

Add source-level regression assertions that:

```ts
test("session history exposes recommendation IDs through an open-recommendation callback", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");
  assert.match(source, /recommendationId/);
  assert.match(source, /onOpenRecommendation/);
});

test("the focused recommendation opens the recommendations panel", () => {
  const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
  assert.match(app, /setFocusedRecommendationId/);
  assert.match(app, /recommendations/);
});
```

- [ ] **Step 2: Run the focused UI tests to verify they fail**

```bash
cd apps/desktop
node --experimental-strip-types --test tests/taskmaster.test.ts tests/dockview.test.ts
```

Expected: the new assertions fail because summary entries and callback wiring do not exist.

- [ ] **Step 3: Implement provenance rendering and focused navigation**

Render a session-history row's `recommendationId` as a button labeled with the stable ID and call `onOpenRecommendation`. In `App.tsx`, activate the existing `recommendations` Dockview panel and pass the focused ID. In `RecommendationsPanel`, retain the current proposed-card behavior for normal polling, but include a focused non-proposed record as read-only history so a completed recommendation remains inspectable. Do not add mutation actions for completed history records.

- [ ] **Step 4: Run the focused UI tests to verify they pass**

```bash
cd apps/desktop
node --experimental-strip-types --test tests/taskmaster.test.ts tests/dockview.test.ts
npx tsc --noEmit
```

Expected: all focused tests and TypeScript validation pass.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/App.css apps/desktop/src/components/DockviewApp.tsx apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/src/components/RecommendationsPanel.tsx apps/desktop/tests/taskmaster.test.ts apps/desktop/tests/dockview.test.ts
git commit -m "feat: link session history to recommendations"
```

### Task 6: Full verification and handoff review

**Files:**
- Modify: `docs/superpowers/specs/2026-09-07-taskmaster-recommendation-handoff-design.md` only if implementation reveals a contract correction.
- Test: repository-wide desktop and Rust validation commands.

- [ ] **Step 1: Run documentation and skill validation**

```bash
bash scripts/doc-check.sh
python3 /Users/froomiebot/.codex/skills/.system/skill-creator/scripts/quick_validate.py skills/working-on-recommendation
git diff --check
```

- [ ] **Step 2: Run desktop validation**

```bash
cd apps/desktop
npx tsc --noEmit
node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

- [ ] **Step 3: Run Rust validation**

```bash
cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
cargo build --manifest-path crates/orkworksd/Cargo.toml
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

- [ ] **Step 4: Inspect the final diff and requirements**

Confirm:

- existing recommendation IDs are reused, not migrated or regenerated;
- the prompt targets the current session and names the skill;
- the completion route derives the target session from the bearer token;
- completion cannot be spoofed by a caller-supplied session or status;
- old events/recommendations deserialize;
- completed recommendations do not resurface;
- no new session creation, arbitrary terminal input, direct metadata-file writes, or Git workflow control was added.

- [ ] **Step 5: Run the required low-effort code review before handoff**

Invoke the repository's explicit review gate with `/code-review low`, address findings that change the diff, and document intentional deviations.

- [ ] **Step 6: Commit any final corrections**

```bash
git add -u
git commit -m "chore: verify recommendation handoff"
```

Then report the branch, commits, verification commands, and any remaining integration step. Do not claim completion until the fresh command output is read and all required checks pass.
