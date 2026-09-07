# Codex Deterministic Attention Signals Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Make Codex turn state deterministic through trusted UserPromptSubmit, PermissionRequest, and Stop hooks while preserving Peon fallback when those hooks are unavailable.

**Architecture:** Extend the existing Codex workspace-hook adapter from identity-only SessionStart coverage to a four-event bundle. Add event/source/fingerprint provenance to attention reports, validate it against the current Codex hook definition, and promote only that live session to hook-authoritative scheduling after an accepted event. Keep the static capability registry descriptive while making initial runtime authority binding-aware.

**Tech Stack:** Rust sidecar with Axum handlers, serde/serde_json, Tokio tests, POSIX shell and PowerShell reporter scripts, JSON hook integration fixtures, Cargo test.

**Spec:** docs/superpowers/specs/2026-09-07-codex-attention-signals-design.md

## Global Constraints

- Codex hook installation remains explicit and requires Codex /hooks trust; never auto-enable hooks.
- SessionStart captures native identity only and never writes attention.
- Codex supports NativeSessionId and Attention, but not Lifecycle.
- Codex starts with Peon/terminal fallback active; a valid current hook event promotes only that live session.
- Hook reports use source codex_hook; generic attention clients retain existing behavior.
- Stale events cannot overwrite a newer accepted input or hook event.
- Preserve unrelated entries in .codex/hooks.json; ownership ambiguity must fail closed.
- Do not add dependencies, change other harness semantics, infer Codex plan paths, or add session lifecycle handling.
- Run Rust formatting and targeted tests after each task; use pnpm for any desktop command.

---

## Task 1: Make the Codex integration own a four-event hook bundle

**Files:**
- Modify: crates/orkworksd/src/harness/integrations/codex.rs
- Modify: crates/orkworksd/src/harness/integrations/mod.rs
- Test: crates/orkworksd/src/harness/integrations/codex.rs
- Test: crates/orkworksd/src/harness/integrations/mod.rs

**Interfaces:**
- Consumes: the existing JsonHookHandler, ReporterInvocation, marker extraction, and Codex portable path rules.
- Produces: exact Codex event fragments for SessionStart, UserPromptSubmit, PermissionRequest, and Stop; one canonical bundle fingerprint used by installation and runtime validation.

- [ ] Step 1: Write failing bundle-shape tests.

Add tests that merge an empty document and assert each event contains exactly one OrkWorks command with the event argument, marker, and hook fingerprint. Add a preservation assertion for an unrelated Stop command and a probe assertion that an incomplete four-event bundle is Drifted rather than Installed.

~~~rust
#[test]
fn merge_writes_the_codex_four_event_bundle_without_dropping_foreign_hooks() {
    let mut document = json!({
        "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "user-stop" }] }] }
    }).as_object().unwrap().clone();
    merge(&mut document, &reporter_path(tempdir.path())).unwrap();
    assert_eq!(document["hooks"]["Stop"].as_array().unwrap().len(), 2);
    for event in ["SessionStart", "UserPromptSubmit", "PermissionRequest", "Stop"] {
        let command = document["hooks"][event][0]["hooks"][0]["command"].as_str().unwrap();
        assert!(command.contains("--event"));
        assert!(command.contains("--hook-fingerprint"));
    }
}
~~~

- [ ] Step 2: Run the focused test and confirm it fails.

Run:
~~~bash
cargo test --manifest-path crates/orkworksd/Cargo.toml codex::tests::merge_writes_the_codex_four_event_bundle -- --exact
~~~
Expected: FAIL because the current merge writes only SessionStart and has no event-aware fingerprint bundle.

- [ ] Step 3: Implement event-specific invocation and bundle ownership.

Add a private event list and an invocation builder that appends --event Event to both POSIX and PowerShell forms. Generate the four commands from one canonical list. Make groups, marker_state, probe, merge, and remove iterate the four named event arrays while retaining foreign entries. Treat duplicate OrkWorks ownership in any event as ambiguous.

Compute the fingerprint from the canonical event-command bundle, excluding the self-referential fingerprint argument, then embed that fingerprint in every generated command. Keep the fingerprint format as a 64-character SHA-256 value.

Update the handler contract metadata to describe native session ID plus deterministic attention. Replace the generic handler’s Codex-specific attention-summary branch with an explicit contract flag; preserve current values for other handlers and set Codex to true.

- [ ] Step 4: Run integration tests and format.

~~~bash
cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
cargo test --manifest-path crates/orkworksd/Cargo.toml harness::integrations::codex
cargo test --manifest-path crates/orkworksd/Cargo.toml harness::integrations::tests
~~~

Expected: all Codex ownership, portability, and integration-status tests pass; unrelated handler tests remain green.

- [ ] Step 5: Commit the integration bundle.

~~~bash
git add crates/orkworksd/src/harness/integrations/codex.rs crates/orkworksd/src/harness/integrations/mod.rs
git commit -m "feat: install Codex attention hook bundle"
~~~

## Task 2: Make both reporters event-aware

**Files:**
- Modify: crates/orkworksd/scripts/report-harness-event.sh
- Modify: crates/orkworksd/scripts/report-harness-event.ps1
- Test: crates/orkworksd/src/harness/integrations/mod.rs

**Interfaces:**
- Consumes: --marker, --status, --hook-fingerprint, Codex JSON payload session_id, and the new --event argument.
- Produces: deterministic attention POSTs for the three turn events and harness-session POSTs for all four events.

- [ ] Step 1: Add failing script-contract assertions.

Extend the embedded-script tests to require --event, all four event names, codex_hook, /attention, and event-specific status mapping. Assert the scripts still contain the current non-Codex default status and timeout behavior.

~~~rust
#[test]
fn codex_reporter_contract_maps_events_without_treating_session_start_as_attention() {
    let script = include_str!("../../../scripts/report-harness-event.sh");
    assert!(script.contains("--event"));
    assert!(script.contains("UserPromptSubmit"));
    assert!(script.contains("PermissionRequest"));
    assert!(script.contains("Stop"));
    assert!(script.contains("SessionStart"));
    assert!(script.contains("codex_hook"));
}
~~~

- [ ] Step 2: Run the contract tests and confirm they fail.

~~~bash
cargo test --manifest-path crates/orkworksd/Cargo.toml report_harness_event -- --nocapture
~~~

Expected: FAIL because the scripts currently suppress every Codex attention report and have no event argument.

- [ ] Step 3: Implement the POSIX reporter mapping.

Parse --event while preserving existing non-Codex behavior. For the Codex marker, accept only the four generated event names. Set status=working for UserPromptSubmit, status=waiting_for_input for PermissionRequest and Stop, and skip attention for SessionStart. Include source, event, and hookFingerprint in Codex attention JSON; continue posting the native session-ID report with the same confidence. Invalid JSON, empty IDs, missing environment, and curl failures remain no-ops.

- [ ] Step 4: Implement the equivalent PowerShell reporter mapping.

Mirror the POSIX event validation and payload fields in report-harness-event.ps1. Keep -TimeoutSec 5, -NoProfile, and current exception swallowing. Do not change Claude, Copilot, or plan-path branches.

- [ ] Step 5: Run script and shell checks.

~~~bash
cargo test --manifest-path crates/orkworksd/Cargo.toml report_harness_event
bash -n crates/orkworksd/scripts/report-harness-event.sh
git diff --check
~~~

Expected: embedded contract tests pass, the shell script parses, and no whitespace errors are reported.

- [ ] Step 6: Commit reporter support.

~~~bash
git add crates/orkworksd/scripts/report-harness-event.sh crates/orkworksd/scripts/report-harness-event.ps1 crates/orkworksd/src/harness/integrations/mod.rs
git commit -m "feat: report Codex hook event attention"
~~~

## Task 3: Advertise Codex attention without enabling it prematurely

**Files:**
- Modify: crates/orkworksd/src/harness/registry.rs
- Modify: crates/orkworksd/src/session_application.rs
- Test: crates/orkworksd/src/harness/registry.rs
- Test: crates/orkworksd/src/session_application.rs

**Interfaces:**
- Consumes: SessionSignalBinding::Codex, CapabilityName, and ResolvedSessionLaunch.
- Produces: Codex NativeSessionId plus Attention capability metadata and an initial active_work_hook=false policy for Codex sessions.

- [ ] Step 1: Write failing capability and launch tests.

Change the registry evidence test to require Codex Attention. Add a launch test that resolves a Codex session and asserts its initial active_work_hook is false despite the advertised attention capability. Add a control assertion for the existing always-authoritative integration behavior.

~~~rust
assert!(codex.contains(&CapabilityName::NativeSessionId));
assert!(codex.contains(&CapabilityName::Attention));
assert!(!codex.contains(&CapabilityName::Lifecycle));
~~~

- [ ] Step 2: Run the focused tests and confirm they fail.

~~~bash
cargo test --manifest-path crates/orkworksd/Cargo.toml signal_capabilities_follow_the_conservative_contract_evidence -- --exact
cargo test --manifest-path crates/orkworksd/Cargo.toml codex_launch_starts_without_hook_authority -- --exact
~~~

Expected: the capability assertion fails because Codex currently lacks Attention; the launch assertion fails unless the resolver policy is changed with the capability update.

- [ ] Step 3: Implement the binding-aware initial authority policy.

Add Codex to the Attention capability branch in capability_names. Add ResolvedHarness::initial_work_hook_active() and use it from both create and resume launch resolution. It must return false for SessionSignalBinding::Codex until an accepted event and preserve current behavior for existing integrations. Do not alter resume commands or harness definitions.

- [ ] Step 4: Run registry and launch tests.

~~~bash
cargo test --manifest-path crates/orkworksd/Cargo.toml signal_capabilities_follow_the_conservative_contract_evidence
cargo test --manifest-path crates/orkworksd/Cargo.toml codex_launch
cargo test --manifest-path crates/orkworksd/Cargo.toml resolve_session_launch
~~~

Expected: Codex advertises the contract but starts in fallback mode; existing launch and resume behavior remains unchanged.

- [ ] Step 5: Commit capability policy.

~~~bash
git add crates/orkworksd/src/harness/registry.rs crates/orkworksd/src/session_application.rs
git commit -m "feat: gate Codex hook authority per session"
~~~

## Task 4: Add provenance validation and dynamic attention authority

**Files:**
- Modify: crates/orkworksd/src/http/session_handlers.rs
- Modify: crates/orkworksd/src/session_application.rs
- Modify: crates/orkworksd/src/metadata.rs to add the focused current-observation read seam used by attention validation
- Test: crates/orkworksd/src/http/session_handlers.rs
- Test: crates/orkworksd/src/session_application.rs
- Test: crates/orkworksd/src/metadata.rs for current-observation validation

**Interfaces:**
- Consumes: attention JSON fields status, observedAt, cwd, plus optional Codex source, event, and hookFingerprint.
- Produces: validated AttentionSignal provenance, codex_hook metadata source, atomic promotion of SessionHandle.active_work_hook, and stale-event rejection.

- [ ] Step 1: Write failing HTTP/application tests.

Add tests for these exact cases:

1. A Codex UserPromptSubmit with the current fingerprint is accepted as working when the session starts with active_work_hook=false, and the live handle becomes authoritative.
2. SessionStart with the same provenance captures identity but does not write attention or promote authority.
3. PermissionRequest and Stop produce waiting_for_input.
4. A missing, unknown, mismatched, or malformed Codex event is rejected without metadata mutation.
5. A stale Stop is ignored after a newer prompt timestamp.
6. A generic non-Codex attention request remains accepted with its current behavior.

Use a valid 64-character fixture fingerprint and explicit timestamps:

~~~rust
let prompt_at = "2026-09-07T08:00:02.000000Z";
let stop_at = "2026-09-07T08:00:01.000000Z";
~~~

- [ ] Step 2: Run the new tests and confirm they fail.

~~~bash
cargo test --manifest-path crates/orkworksd/Cargo.toml codex_hook -- --nocapture
cargo test --manifest-path crates/orkworksd/Cargo.toml report_attention -- --nocapture
~~~

Expected: the request type lacks provenance, Codex working is rejected while authority is false, and no dynamic promotion exists.

- [ ] Step 3: Extend the request and application signal types.

Add optional serde fields to AttentionReportRequest and matching fields to AttentionSignal: source, event, and hook_fingerprint using the existing camelCase wire naming for the fingerprint. Keep all fields optional so existing callers serialize unchanged. Pass them through the HTTP-to-application seam.

- [ ] Step 4: Add current-Codex-bundle validation.

Implement one application helper that accepts Codex provenance only when the session harness is codex, source is codex_hook, event is one of the four verified names, and the fingerprint is a valid current bundle fingerprint. Use the stable reporter path and the same canonical fingerprint builder exposed as codex::current_hook_fingerprint(). Do not treat the observation file alone as proof that a stale hook is current; the reported fingerprint must match the currently generated OrkWorks bundle. Leave generic reports on their existing path.

- [ ] Step 5: Implement atomic merge and promotion.

Refactor the attention application seam so validation, stale ordering, metadata merge, and Codex promotion occur in one ordered session operation. Normalize Codex event statuses as follows: UserPromptSubmit to working, PermissionRequest and Stop to waiting_for_input, and SessionStart to identity-only. Set metadata source codex_hook and confidence 1.0 for accepted Codex attention. Set active_work_hook=true only after the event is accepted; never promote on an ignored or rejected event.

Preserve existing checks against accepted_input_at, last_hook_attention_at, terminal lifecycle, metadata priority, and pending work buffers. A newer user-authored state remains authoritative.

- [ ] Step 6: Run application and HTTP tests.

~~~bash
cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
cargo test --manifest-path crates/orkworksd/Cargo.toml codex_hook
cargo test --manifest-path crates/orkworksd/Cargo.toml report_attention
cargo test --manifest-path crates/orkworksd/Cargo.toml session_handlers::tests
~~~

Expected: all provenance, promotion, stale-ordering, and backward-compatibility tests pass.

- [ ] Step 7: Commit the application protocol.

~~~bash
git add crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/src/session_application.rs crates/orkworksd/src/metadata.rs
git commit -m "feat: accept trusted Codex attention events"
~~~

## Task 5: Prove runtime fallback and hook-authoritative scheduling

**Files:**
- Modify: crates/orkworksd/src/runtime/peon_runtime.rs only where the current authority guard needs the new transition covered
- Modify: crates/orkworksd/src/runtime/terminal_runtime.rs only where the current authority guard needs the new transition covered
- Test: crates/orkworksd/src/runtime/peon_runtime.rs
- Test: crates/orkworksd/src/runtime/terminal_runtime.rs
- Test: crates/orkworksd/src/session_application.rs

**Interfaces:**
- Consumes: the live SessionHandle.active_work_hook transition from Task 4.
- Produces: no Peon downgrade after accepted Codex hook authority, while unhooked Codex sessions continue using existing inference.

- [ ] Step 1: Add failing scheduler tests.

Cover both branches: a session with active_work_hook=false and ongoing output can still receive existing Peon working inference; a session promoted by UserPromptSubmit does not get downgraded by subsequent output scanning or terminal heuristics. Add a test that a prompt after Stop restores hook working state.

- [ ] Step 2: Run the scheduler tests and confirm the regression boundary.

~~~bash
cargo test --manifest-path crates/orkworksd/Cargo.toml peon_runtime
cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_runtime
~~~

Expected: the fallback test passes against existing behavior; the authority test fails until the dynamic transition is recognized by all scheduler paths.

- [ ] Step 3: Apply the smallest runtime change.

Reuse the existing active_work_hook guards rather than adding a second authority flag. Ensure the attention merge updates the same handle observed by Peon and terminal runtime tasks under the existing session lock. Do not change idle thresholds or terminal parsing in this task.

- [ ] Step 4: Run all runtime tests.

~~~bash
cargo test --manifest-path crates/orkworksd/Cargo.toml peon_runtime
cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_runtime
cargo test --manifest-path crates/orkworksd/Cargo.toml session_application
~~~

Expected: fallback, promotion, and existing timing tests pass without changing unrelated inference behavior.

- [ ] Step 5: Commit runtime coverage.

~~~bash
git add crates/orkworksd/src/runtime/peon_runtime.rs crates/orkworksd/src/runtime/terminal_runtime.rs crates/orkworksd/src/session_application.rs
git commit -m "test: protect Codex hook authority from Peon downgrades"
~~~

## Task 6: Update contract documentation and run the full verification sweep

**Files:**
- Modify: docs/agents/harness-integration-contracts.md
- Modify: docs/superpowers/specs/2026-07-22-harness-capability-system-design.md to align its Codex signal table with the final fingerprint and activation wording
- Test: repository Rust validation commands from the root instructions

**Interfaces:**
- Consumes: the implemented four-event Codex contract and dynamic activation rule.
- Produces: documentation matching the shipped adapter and an evidence-backed verification record.

- [ ] Step 1: Update the harness contract documentation.

Document Codex’s four events, normalized status mapping, source/confidence, fingerprint trust prerequisite, explicit /hooks approval, and Peon fallback. Preserve the existing statement that Codex plan paths remain terminal-text fallback.

- [ ] Step 2: Run formatting, focused tests, and repository checks.

~~~bash
cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
cargo test --manifest-path crates/orkworksd/Cargo.toml harness::integrations
cargo test --manifest-path crates/orkworksd/Cargo.toml registry
cargo test --manifest-path crates/orkworksd/Cargo.toml session_application
cargo test --manifest-path crates/orkworksd/Cargo.toml http::session_handlers
cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::peon_runtime
cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime
git diff --check
~~~

If desktop files remain unchanged, do not invent a desktop test run; report that the change is sidecar/scripts/docs only. If the full Rust suite is affordable, run cargo test --manifest-path crates/orkworksd/Cargo.toml after the focused suite.

- [ ] Step 3: Inspect the final diff for scope and contract drift.

Confirm that only the planned sidecar, reporter, integration, tests, and documentation files changed; that no Codex hook is enabled automatically; and that .codex/hooks.json fixtures preserve foreign hooks.

- [ ] Step 4: Commit documentation and final test updates.

~~~bash
git add docs/agents/harness-integration-contracts.md docs/superpowers/specs/2026-07-22-harness-capability-system-design.md
git commit -m "docs: record Codex attention hook contract"
~~~

- [ ] Step 5: Prepare review handoff.

~~~bash
git status --short --branch
git log --oneline --decorate -8
~~~

Then invoke the required low-effort /code-review low gate for the code diff, address findings, and report exact test commands and outputs before opening or handing off the PR.
