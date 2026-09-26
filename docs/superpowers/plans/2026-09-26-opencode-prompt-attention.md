# OpenCode Prompt Attention Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Show Needs You only for unresolved OpenCode permission or question requests, and keep Peon chat inference from creating or replacing attention for Codex and OpenCode.

**Architecture:** The OpenCode plugin tracks request IDs and reports its effective state through the existing attention route with a session token and event provenance. The sidecar promotes OpenCode's per-session hook authority only after an accepted validated report. A single metadata merge policy lets Peon retain descriptions, use nonprompt fallback before hook activation, and preserve hook-owned attention afterward.

**Tech Stack:** OpenCode plugin JavaScript, Node built-in test runner, Rust sidecar/Axum, repository Markdown.

**Spec:** docs/superpowers/specs/2026-09-26-opencode-prompt-attention-design.md

## Global Constraints

- Needs You requires an unresolved permission or explicit question. Ordinary turn completion is Idle.
- Only events for the OpenCode ID captured from session.created affect the reporter; request IDs are qualified by type.
- OpenCode hook reports carry source opencode_hook, event name, observedAt, and Authorization: Bearer using ORKWORKS_REPORT_TOKEN. The token is never logged or persisted.
- OpenCode's registry capability does not activate active_work_hook. Only an accepted validated report for that live session does.
- An active hook owns all its reported attention states. Peon can still supply summaries, phase, diagnostics, and workflow evidence.
- Before hook activation, Peon may supply nonprompt observed status but cannot create Needs You or prompt fields for Codex/OpenCode from LLM inference. Existing Peon-sourced waiting is cleared to unknown on the next attention reconciliation.
- User overrides, accepted terminal input, and lifecycle transitions retain their existing authority. Claude, Copilot, Aider, and hookless tools retain their present Peon rules.
- The OpenCode integration coverage label remains limited until a live OpenCode process proves end-to-end attention delivery. Lost resolution events and plugin reload with pending requests remain limitations.
- Use rtk for every shell command and pnpm for any Node package-management operation. Work only in the owned opencode-prompt-attention worktree.

---

## Evidence and blind-spot checkpoint

**Least confidence:** A live OpenCode plugin's event/input ordering has not been observed in an OrkWorks-launched process. Versioned schemas and publisher code establish event names and fields, while this plan's sequence tests establish our projection. Keep the integration coverage label limited. The existing input timestamp and hook timestamp guards must reject stale reports without activating hook authority.

**Project blind spot:** The source-priority ladder has one record-wide source and Peon's merge has an early return when an inferred Working would resume a terminal status. Merely dropping a new Peon waiting inference would leave an old Peon-sourced Needs You latched; reconciliation must happen before that early return and must clear durable prompt fields. The attention route currently accepts generic agent reports; after OpenCode activation, an untagged report must not inherit hook authority or replace it.

**Resolved assumptions:** Main owns ADR 0065; this plan uses ADR 0066. The existing in-memory ORKWORKS_REPORT_TOKEN registry can authenticate an OpenCode report for its exact OrkWorks session. active_work_hook is already per-session, so initializing it false for OpenCode avoids a new field and restores pre-hook nonprompt fallback. The validated request tag stays transport-only; persisted metadata retains source agent, avoiding a new UI source value. Check ADR numbering again immediately before Task 1 in case another branch lands first.

## File map

| File | Responsibility |
| --- | --- |
| docs/adr/0066-hook-owned-prompt-attention.md, docs/adr/README.md | Record and index the approved attention authority boundary before code. |
| crates/orkworksd/scripts/opencode-session-reporter.js | Maintain request sets, effective attention, token/event provenance, and ordered microsecond timestamps. |
| crates/orkworksd/scripts/opencode-session-reporter.test.mjs | Exercise the real plugin export against captured HTTP requests and event sequences. |
| crates/orkworksd/src/harness/integrations/opencode.rs | Keep packaged reporter identity coverage while removing obsolete timestamp assertions. |
| crates/orkworksd/src/harness/registry.rs | Initialize OpenCode's per-session work-hook flag as inactive without removing its declared capability. |
| crates/orkworksd/src/http/session_handlers.rs | Forward the attention bearer token while retaining legacy report behavior. |
| crates/orkworksd/src/main.rs | Bind the header-aware attention handler if its exported name changes. |
| crates/orkworksd/src/session_application.rs | Validate OpenCode provenance, promote only accepted reports, select Peon attention policy. |
| crates/orkworksd/src/metadata.rs | Clear old prompt fields and merge Peon under a single attention policy while retaining agent provenance for OpenCode. |
| docs/agents/harness-integration-contracts.md | State verified events, activation rule, limitations, and limited coverage. |

### Task 1: Record the authority decision

**Files:**
- Create: docs/adr/0066-hook-owned-prompt-attention.md
- Modify: docs/adr/README.md

**Interfaces:** The ADR defines the invariant used by Tasks 3-4: Codex/OpenCode Peon inference cannot establish a prompt, and a validated per-session hook owns attention after acceptance.

- [ ] **Step 1: Check the ADR slot.** Run rtk proxy git ls-tree -r --name-only main docs/adr and confirm 0066 is unused. If another change claimed it, update this plan's ADR filename and index entry before writing.
- [ ] **Step 2: Write the decision.** Use docs/adr/template.md. State the stale Peon question incident, why installation/capability is not activation, the Codex/OpenCode prompt evidence rule, the OpenCode token/event validation, per-session hook authority, and limits for lost events and other harnesses. State that metadata.rs remains the status merge owner; the validated OpenCode request tag persists as agent metadata source while runtime activation records its authority.
- [ ] **Step 3: Index and inspect.** Add one accepted row to docs/adr/README.md. Run rtk git diff --check and inspect both files. The existing prose in the approved design is the root guide's ADR pointer, so add no duplicate inline root AGENTS.md bullet.
- [ ] **Step 4: Commit.** Stage only the ADR and index; commit with message "docs: record hook-owned prompt attention".

### Task 2: Make the reporter project explicit prompt lifecycles

**Files:**
- Create: crates/orkworksd/scripts/opencode-session-reporter.test.mjs
- Modify: crates/orkworksd/scripts/opencode-session-reporter.js
- Modify: crates/orkworksd/src/harness/integrations/opencode.rs

**Interfaces:** Consume OpenCode events with event.type and event.properties. Produce POST /sessions/:id/attention with JSON fields status, observedAt, source, event, optional message, plus the bearer token. The first captured session.created reports idle.

- [ ] **Step 1: Write a failing end-to-end reporter test.** Import the actual reporter file as an ESM data URL. Start a node:http receiver on 127.0.0.1 with a random port; collect request path, Authorization header, and parsed body. Set ORKWORKS_PORT, ORKWORKS_SESSION_ID, and a fake ORKWORKS_REPORT_TOKEN for the test, restoring their prior values afterward. Send session.created for ses_1, session.status busy, then question.asked with id que_1 and sessionID ses_1. Assert statuses idle, working, waiting_for_input, matching event names, source opencode_hook, and the fake bearer token. The old reporter lacks the initial idle and question report.

~~~js
const events = [
  { type: 'session.created', properties: { info: { id: 'ses_1' } } },
  { type: 'session.status', properties: { sessionID: 'ses_1', status: { type: 'busy' } } },
  { type: 'question.asked', properties: { id: 'que_1', sessionID: 'ses_1', questions: [] } },
];
for (const event of events) await reporter.event({ event });
assert.deepEqual(attentionPosts.map((post) => post.body.status),
  ['idle', 'working', 'waiting_for_input']);
assert.ok(attentionPosts.every((post) => post.authorization === 'Bearer test-token'));
~~~

- [ ] **Step 2: Run the red test.** Run rtk proxy node --test crates/orkworksd/scripts/opencode-session-reporter.test.mjs. Confirm the failure is a missing lifecycle/provenance assertion, not server setup.
- [ ] **Step 3: Implement effective state.** Track turnStatus, separate permissionIds/questionIds sets, captured session ID, and last reported status/message. Ask events require a nonempty string id; reply/reject events require a matching nonempty requestID. Busy/idle change turnStatus but cannot clear a pending set. A new captured ID resets state and emits idle; a duplicate creation does nothing. A status or message change emits one POST. Only matching sessionID events participate. If ORKWORKS_REPORT_TOKEN is absent, skip the authenticated attention POST rather than sending an unauthenticated claimed hook report.

~~~js
const effective = () => {
  if (permissionIds.size && questionIds.size)
    return { status: 'waiting_for_input', message: 'OpenCode needs an answer or permission decision' };
  if (permissionIds.size)
    return { status: 'waiting_for_input', message: 'OpenCode is asking for a permission decision' };
  if (questionIds.size)
    return { status: 'waiting_for_input', message: 'OpenCode is asking for an answer' };
  return { status: turnStatus };
};
~~~

- [ ] **Step 4: Expand red-green sequences one case at a time.** Cover permission ask/reply, question reply/reject, two requests of one type, overlapping types and message change, busy/idle while pending, duplicate/unknown resolution, malformed IDs, foreign session events, and captured-session replacement. Assert POST bodies and count, not source-text substrings. Run the Node test after each minimal reporter change.
- [ ] **Step 5: Pin event time and ordering.** Add controlled Performance fixtures for distinct events in one millisecond, a nonzero microsecond part after accepted terminal input time, a backward clock step, a failed POST followed by a later event, and overlapping async fetches. Implement a monotonic integer-microsecond sequence from performance.timeOrigin + performance.now(); format exactly six UTC fractional digits. The sidecar's existing observedAt guard, not HTTP completion order, chooses the newer event.

~~~js
const micros = Math.max(
  Math.floor((performance.timeOrigin + performance.now()) * 1000),
  lastMicros + 1,
);
lastMicros = micros;
const fraction = String(micros % 1_000_000).padStart(6, '0');
const observedAt = new Date(Math.floor(micros / 1000))
  .toISOString().replace(/\.\d{3}Z$/, '.' + fraction + 'Z');
~~~

- [ ] **Step 6: Preserve packaging evidence.** In opencode.rs, remove the assertion for millisecond padding; keep the assertion that the installed plugin bytes derive from the source script. Run the Node test and rtk cargo test --manifest-path crates/orkworksd/Cargo.toml plugin_source_.
- [ ] **Step 7: Commit.** Run rtk git diff --check, stage the three Task 2 files, and commit "fix: track OpenCode prompt request lifecycles".

### Task 3: Validate and activate OpenCode hook reports

**Files:**
- Modify: crates/orkworksd/src/harness/registry.rs
- Modify: crates/orkworksd/src/http/session_handlers.rs
- Modify: crates/orkworksd/src/session_application.rs
- Modify: crates/orkworksd/src/metadata.rs

**Interfaces:** AttentionSignal gains report_token: Option<String>. The handler extracts the bearer token for the application layer; only request source opencode_hook requires it. The application returns a bad request for mismatched token, harness, absent observedAt, or event/status pair. An accepted validated request writes metadata_source agent and sets active_work_hook true through an explicit activation flag. Legacy untagged reports retain their pre-activation behavior.

- [ ] **Step 1: Write failing route/application cases.** Add tests near existing report_attention tests using the session test helpers and set_workflow_report_token. Cover missing/wrong token, wrong harness, ended/missing session, malformed observedAt, invalid event/status combinations, valid session.created idle, and a valid question. Assert rejection leaves durable metadata and active_work_hook unchanged. Assert a valid accepted report stores agent provenance and activates the flag; an out-of-order or pre-input stale report does not activate.

~~~rust
let token = "opencode-test-token";
crate::runtime::terminal_runtime::set_workflow_report_token(id, token.into());
let report = AttentionSignal {
    status: "waiting_for_input".into(),
    event: Some("question.asked".into()),
    source: Some("opencode_hook".into()),
    report_token: Some(token.into()),
    observed_at: Some("2026-09-26T12:00:00.123456Z".into()),
    message: Some("OpenCode is asking for an answer".into()),
    plan_path: metadata::PlanPathUpdate::Unchanged,
    cwd: None,
    hook_fingerprint: None,
};
~~~

- [ ] **Step 2: Run the focused red tests.** Use rtk cargo test --manifest-path crates/orkworksd/Cargo.toml opencode_hook_. Confirm failures are validation/activation behavior.
- [ ] **Step 3: Implement route forwarding without changing legacy callers.** Add a report_attention_with_headers handler for the Axum route and retain the existing no-header wrapper under cfg(test). Both call a shared inner function. Extract bearer_token from HeaderMap into AttentionSignal.report_token; never log the token. Update the router import and route binding in main.rs if its symbol changes, and add main.rs to this task's staged files.
- [ ] **Step 4: Validate before mutation.** In SessionApplication::report_attention, validate request source opencode_hook against verify_workflow_report_token, the live session's OpenCode harness, a present observedAt, and this exact event/status matrix: session.created -> idle; session.status -> working; session.idle -> idle; permission.asked/question.asked -> waiting_for_input; permission.replied/question.replied/question.rejected -> waiting_for_input, working, or idle. Reject other pairs. Make validate_codex_hook_signal return false for opencode_hook while still rejecting any other unknown source. Include validated OpenCode reports in supports_active_work so the first working report can pass normalization while active_work_hook is still false. Map validated OpenCode reports to persisted source agent, set activate_work_hook, and require a live handle.

~~~rust
let opencode_hook = signal.source.as_deref() == Some("opencode_hook");
if opencode_hook {
    let token = signal.report_token.as_deref().ok_or(SessionError::EmptyBadRequest)?;
    if !verify_workflow_report_token(id, token) {
        return Err(SessionError::EmptyBadRequest);
    }
    let live_opencode = self.state.sessions.lock().unwrap().get(id).is_some_and(|handle| {
        handle.info.lifecycle == "alive"
            && handle.info.lifecycle_phase == "active"
            && handle.info.harness_id.as_deref() == Some("opencode")
    });
    let event = signal.event.as_deref().ok_or(SessionError::EmptyBadRequest)?;
    if !live_opencode || signal.observed_at.is_none()
        || !opencode_event_allows_status(event, &signal.status)
    {
        return Err(SessionError::EmptyBadRequest);
    }
}

fn opencode_event_allows_status(event: &str, status: &str) -> bool {
    match event {
        "session.created" | "session.idle" => status == "idle",
        "session.status" => status == "working",
        "permission.asked" | "question.asked" => status == "waiting_for_input",
        "permission.replied" | "question.replied" | "question.rejected" => {
            matches!(status, "waiting_for_input" | "working" | "idle")
        }
        _ => false,
    }
}
~~~

- [ ] **Step 5: Promote only after durable acceptance.** Change ResolvedHarness::initial_work_hook_active so Codex and OpenCode begin false while retaining CapabilityName::Attention. Add activate_work_hook: bool to AttentionMergeSignal; set it for validated Codex/OpenCode requests and only update the handle after an accepted durable merge. Use that flag for the live-handle requirement, replacing the Codex-only source check. Persist OpenCode at source agent, Codex at codex_hook. Pass the same flag into MetadataStore::merge_agent_attention_signal_with_plan and clear stored Peon prompt fields in that durable merge, then clear them in the live projection. Existing nonhook callers pass false. Once OpenCode has activated, ignore an untagged generic agent report for that same session; user/process/lifecycle transitions remain possible.

~~~rust
if result == metadata::AttentionMergeResult::Accepted {
    if let Some(handle) = sessions.get_mut(&signal.session_id) {
        if signal.activate_work_hook {
            handle.active_work_hook = true;
            handle.info.needs_user_input = None;
            handle.info.detected_question = None;
            handle.info.suggested_options = None;
        }
    }
}
~~~
- [ ] **Step 6: Run green and negative boundaries.** Run focused auth, stale-event, activation, source-priority, and registry tests. Include a test that a different session's valid token cannot activate this one; a generic agent report before activation does not activate and after activation cannot replace hook attention.
- [ ] **Step 7: Commit.** Format touched Rust files from the crate directory, run rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check and rtk git diff --check, stage only Task 3 files, and commit "fix: activate OpenCode attention from validated reports".

### Task 4: Restrict Peon prompt authority and reconcile old waits

**Files:**
- Modify: crates/orkworksd/src/session_application.rs
- Modify: crates/orkworksd/src/metadata.rs
- Modify: docs/agents/harness-integration-contracts.md

**Interfaces:** MetadataStore's Peon merge receives one explicit policy: Infer for other harnesses, NonPrompt for Codex/OpenCode before activation, or PreserveHook with status/confidence/source after activation. Selection happens under the existing workspace -> sessions lock order. OpenCode's preserved source is agent; Codex's is codex_hook. A preserved process transition retains process provenance.

- [ ] **Step 1: Write failing Peon policy cases.** Near active_codex_hook_keeps_peon_waiting_inference_from_becoming_needs_you, cover Codex and OpenCode before activation: a Peon waiting inference may update summary/diagnostics but leaves attention nonprompt and prompt fields empty. Start another record with Peon-sourced waiting, feed an inference with no status or with Working, and assert durable observed status/prompt fields become unknown or Working respectively; this must happen despite the existing terminal-status early return. Confirm live/API projection follows the durable record.
- [ ] **Step 2: Run the red tests.** Run rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_nonprompt_ and confirm the old code produces Needs You or leaves it latched.
- [ ] **Step 3: Add one metadata merge policy.** Define PeonAttentionPolicy with Infer, NonPrompt, and PreserveHook { status, confidence, source }. Select it in persist_peon_observation_inner from the live handle: Codex/OpenCode with active_work_hook and current hook/process provenance use PreserveHook; other Codex/OpenCode use NonPrompt; all other harnesses use Infer. In the metadata merge, clear prior Peon-sourced waiting and prompt fields before the terminal-status Working guard. Under NonPrompt, ignore incoming waiting status and all incoming prompt fields but keep summary/phase/diagnostics. Under PreserveHook, retain status and actual source unless the newer current source is process. Do not bypass user priority.

~~~rust
pub enum PeonAttentionPolicy<'a> {
    Infer,
    NonPrompt,
    PreserveHook { status: &'a str, confidence: f64, source: &'a str },
}

// In the metadata merge, before the terminal-status early return:
if matches!(policy, PeonAttentionPolicy::NonPrompt)
    && meta.metadata_source == "peon"
    && meta.observed_status.as_deref() == Some("waiting_for_input")
{
    meta.observed_status = None;
    meta.attention = None;
    meta.needs_user_input = None;
    meta.detected_question = None;
    meta.suggested_options = None;
}
~~~

- [ ] **Step 4: Prove hook ownership and negative boundaries.** Add cases for OpenCode hook-owned waiting, working, and idle after the 15-second window; Codex hook-owned Working and after-input process provenance; user override; dead/inactive sessions; a second session; and Claude/Aider/Copilot unchanged. Assert Peon still supplies a new summary or diagnostic when attention is preserved. Run each focused test after its implementation change.
- [ ] **Step 5: Update the integration contract.** Describe OpenCode question and permission request IDs, reply/reject events, per-session token and activation, Peon nonprompt fallback, limited live coverage, and lost-event/plugin-reload limits. State that an existing installation must be reconciled through Settings to receive the new reporter bytes. Describe Codex's no-chat-question rule and the remaining queued-question direct-signal gap in #632; leave other harness entries unchanged except a pointer to #643.
- [ ] **Step 6: Commit.** Format touched Rust files from the crate directory; run rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check and rtk git diff --check. Stage only Task 4 files and commit "fix: keep inferred chat prompts out of attention".

### Task 5: Verify and deliver issue #631

**Files:** No new source file. Update the issue and PR with actual verification and limitations.

**Interfaces:** The combined change keeps the existing attention endpoint for legacy harnesses and the existing priority of user, process, and lifecycle transitions.

- [ ] **Step 1: Run focused behavior checks.** Run rtk proxy node --test crates/orkworksd/scripts/opencode-session-reporter.test.mjs and the focused Rust filters from Tasks 2-4. Record exact counts and failures; fix any failure before proceeding.
- [ ] **Step 2: Run required sidecar validation.** From the repository root run rtk cargo build --manifest-path crates/orkworksd/Cargo.toml, rtk cargo test --manifest-path crates/orkworksd/Cargo.toml, and rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check. Run rtk git diff --check and inspect rtk git status --short. Do not claim live OpenCode delivery from synthetic tests.
- [ ] **Step 3: Review the diff.** Apply requesting-code-review and the repository's explicit /code-review medium gate because attention lifecycle, authentication, and metadata precedence change together. Address findings or document why an intentional behavior remains.
- [ ] **Step 4: Open one PR for #631.** Push the owned branch and open a PR against main with the spec and ADR, actual verification results, limited live-coverage and lost-event limitations, and issue links #631/#632/#643. Follow babysitting-pull-requests through CI and review, then use the documented merge/cleanup path when eligible.
