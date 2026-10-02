# Claude and Copilot Prompt Authority Implementation Plan

> **For agentic workers:** Execute inline in this session using the approved issue scope; keep each step test-first and independently reviewable.

**Goal:** Implement the shared Claude/Copilot prompt-attention authority protocol in issue #681, preserving Peon fallback and limiting authority to the active, launch-bound integration.

**Architecture:** The sidecar keeps authority, generation, native identity, conversation epoch, and reset reservations in process-local state. Authenticated HTTP routes validate identity and attention reports against that state, while shared reporters synchronously register native identity before each attention report. PTY input delivery acknowledgement commits or cancels reset reservations; metadata writes are limited to the four-field prompt tuple and its existing record-wide source rule.

**Tech Stack:** Rust/Axum sidecar, serde JSON, Bash and PowerShell hook reporters, pnpm-managed desktop workspace for setup.

**Spec:** `specs/orkworks-mvp.md#deterministic-harness-supplied-signals`; `docs/superpowers/specs/2026-09-27-other-harness-prompt-attention-design.md`; `docs/adr/0073-other-harness-prompt-attention-authority.md`; issue #681.

## Global Constraints

- Keep Claude and Copilot coverage **feature-probed/limited**, not version-verified; #643 remains open.
- Only reviewed prompt types activate authority; turn and lifecycle events never activate it.
- The report token and launch generation authenticate an OrkWorks session capability, not the emitting process.
- Do not revive authority after sidecar restart, or let integration re-enable mutate an existing process environment.
- Only exact persisted Claude `SessionStart(source=clear)` and Copilot bare `/clear` or `/new` reset flows can replace native IDs.
- Promotion and demotion update only `observed_status`/`attention`, `needsUserInput`, `detectedQuestion`, and `suggestedOptions`, preserving the entire tuple only when metadata source is `user`.
- Keep Peon summaries, phase, diagnostics, workflow evidence, and pre-activation fallback independent of hook attention.

---

### Task 1: Launch-bound authority state and generation

**Files:**
- Create `crates/orkworksd/src/runtime/prompt_authority.rs` as the process-local authority state machine; export it from `runtime/mod.rs`.
- Modify `crates/orkworksd/src/runtime/terminal_runtime.rs` to seed and clean up per-session authority alongside the reporting capability.
- Modify `crates/orkworksd/src/session_application.rs` for fresh generation setup at new runtime launch and resume, and stale capability cleanup.
- Modify `crates/orkworksd/src/harness/integrations/claude.rs` and `copilot.rs` to report integration launch readiness only when the owned prompt notification hook is exact and present.
- Test in the matching Rust module tests.

**Interfaces:**
- Create a launch capability containing the existing report token, random immutable generation, harness ID, and initial runtime identity; leave generation absent when the supported owned hook is unavailable.
- Provide atomic APIs to read a launch environment snapshot, validate generation/harness/token, bind one native ID, activate on an accepted prompt, revoke, and remove ended-session state.

- [ ] Add tests proving disabled, absent, drifted, or ambiguous hooks cannot issue a generation, while exact installed Claude/Copilot notification hooks can.
- [ ] Run each focused Rust test to confirm the expected failure before implementation.
- [ ] Implement the in-memory capability and exact hook readiness checks without persisting generation or authority.
- [ ] Run the focused tests and confirm new runtime launches receive immutable `ORKWORKS_PROMPT_HOOK_GENERATION` only when eligible; stale/revoked capabilities fail closed.

### Task 2: Registration-before-attention reporter transport

**Files:**
- Modify `crates/orkworksd/scripts/report-harness-event.sh` and `.ps1`.
- Modify `crates/orkworksd/src/harness/integrations/mod.rs`, `claude.rs`, and `copilot.rs` only where shared reporter invocation configuration needs the launch generation field.

**Interfaces:**
- Both reporters capture `ORKWORKS_PROMPT_HOOK_GENERATION` at invocation start, include it in the identity registration payload, await a successful registration response before every attention POST, and exit without retry/buffering when registration fails.
- Registration-only lifecycle events remain non-attention reports; existing event/status allowlists remain unchanged.

- [ ] Add shell and PowerShell reporter tests for missing generation, missing token, successful registration then attention, failed registration stopping attention, and reset lifecycle fields.
- [ ] Run those reporter tests to establish the intended failing cases.
- [ ] Add generation forwarding and synchronous registration gating to the shared reporters; retain existing Codex/OpenCode contracts unless their call sites explicitly opt into this Claude/Copilot protocol.
- [ ] Run focused reporter tests and confirm no attention POST follows a rejected or failed identity registration.

### Task 3: Authenticated identity and prompt-state merge boundary

**Files:**
- Modify `crates/orkworksd/src/http/session_handlers.rs` for generation-bearing request parsing and route validation.
- Modify `crates/orkworksd/src/session_application.rs` and `metadata.rs` for atomic registration/attention acceptance, authority activation, and prompt-tuple-only promotion/demotion.
- Test in the corresponding session handler, application, and metadata test modules.

**Interfaces:**
- Registration validates live session, session harness, bearer report token, exact immutable generation, native ID syntax, and allowed harness/event/source pair.
- Attention validates those same bindings plus the accepted current native ID, exact event/status pair, live runtime generation, and revoke state before merge.
- A recognized prompt event is the sole authority activation point; registration and all pre-activation nonprompt reports leave attention and metadata source unchanged.

- [ ] Add failing cases for missing/invalid/revoked generation, wrong token, wrong harness, cross-session ID, unregistered/mismatched native ID, invalid event/status, and inactive/dead sessions.
- [ ] Add failing merge cases for activation/demotion clearing a non-user tuple, preserving a user tuple, and leaving summary/phase/diagnostics/workflow evidence untouched.
- [ ] Run the focused tests and confirm each fails for the contract violation.
- [ ] Implement validation under the existing workspace/session lock order and use the shared metadata merge boundary for the four-field tuple only.
- [ ] Run focused tests and verify existing Codex/OpenCode validation and Peon fallbacks remain intact.

### Task 4: PTY-acknowledged native-ID reset reservations

**Files:**
- Modify `crates/orkworksd/src/session_application.rs` with per-session reservation/candidate transitions.
- Modify `crates/orkworksd/src/runtime/terminal_runtime.rs` to open reservations before dispatch and commit/cancel them from the existing PTY write acknowledgement.
- Modify `crates/orkworksd/src/http/session_handlers.rs` so early lifecycle registration and following reports are held until successful delivery.

**Interfaces:**
- Prepare only an exact persisted Claude clear or Copilot bare `/clear` or `/new` while the session is live and already bound to a native ID.
- Before acknowledgement, hold a matching replacement lifecycle event and subsequent reports without changing binding, epoch, prompt tuple, or attention.
- On successful write acknowledgement, advance epoch, retire the old ID, clear the previous non-user prompt tuple, and apply the queued replacement binding/reports; on error or cancellation discard candidate state and leave the current binding intact.

- [ ] Add failing tests for pre-ack candidate holding, successful acknowledgement, failed/canceled write, stale/retired ID, wrong lifecycle source, repeated reset, and same-ID idempotent registration.
- [ ] Run focused reservation and terminal input tests to establish failures.
- [ ] Implement reservation transitions using the existing capture-before-dispatch and accepted-input acknowledgement seam; keep Codex reset authorization behavior unchanged.
- [ ] Run focused tests and verify failed delivery leaves the original identity and prompt tuple unchanged.

### Task 5: Revocation, end, and restart boundaries

**Files:**
- Modify the integration mutation/reconciliation path in `crates/orkworksd/src/harness/integration.rs` and applicable Claude/Copilot handlers.
- Modify `crates/orkworksd/src/session_application.rs` and `runtime/terminal_runtime.rs` for disable/uninstall/drift revocation and terminal/deletion cleanup.
- Add Rust lifecycle tests near those paths.

**Interfaces:**
- Explicit disable, uninstall, or detected drift revokes the current generation before clearing non-user prompt fields.
- Re-enable affects only future launches; it never replaces the inherited generation of an existing runtime.
- Orphan recovery/dead runtime cleanup removes live capabilities so restart cannot revive authority; Peon fallback remains available after revocation.

- [ ] Add failing tests for revoke-before-demote ordering, post-revoke reports from a still-running process, re-enable with stale generation, runtime end, and sidecar restart/orphan state.
- [ ] Run those tests to verify they fail against current lifecycle behavior.
- [ ] Implement ordered revoke/demote and lifecycle cleanup using process-local capability state.
- [ ] Run focused tests and verify summaries/diagnostics continue to update after prompt authority ends.

### Task 6: Contract documentation and full verification

**Files:**
- Update `docs/agents/harness-integration-contracts.md` only where the implemented shared protocol, launch readiness, and feature-probed evidence status differ from current documentation.
- Keep event-specific schema/mapping changes for follow-up issues #682 and #683.

- [ ] Review the diff against all #681 acceptance criteria and ADR 0073; remove any event mapping expansion or unrelated metadata mutation.
- [ ] Run Rust formatting, focused Rust tests, sidecar build, and the repo's required checks for changed files.
- [ ] Confirm no recommendation completion is due; this implementation does not target an existing Taskmaster recommendation.
- [ ] Prepare a PR body linking #681 and noting that #643 remains open and both integrations are feature-probed/limited.
