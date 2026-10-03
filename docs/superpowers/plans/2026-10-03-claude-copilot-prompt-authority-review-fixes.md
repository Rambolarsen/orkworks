# Claude and Copilot Prompt Authority Review Fixes Implementation Plan

> **For agentic workers:** Execute inline in this session with test-first steps and review checkpoints.

**Goal:** Resolve the actionable runtime review findings on PR #719 without changing the accepted prompt-authority contract.

**Architecture:** Keep authority process-local. Model an acknowledged reset independently from its optional replacement lifecycle candidate, and serialize reset completion with authority revocation so persistence and the live projection cannot outlive the capability. Enforce the prompt-authority boundary from the target session harness and preserve Peon fallback until authority activates. Apply the same failure-safe demotion rule when active harnesses change.

**Tech Stack:** Rust/Axum sidecar, shared metadata store, existing unit and handler tests.

**Spec:** [`docs/adr/0073-other-harness-prompt-attention-authority.md`](../../adr/0073-other-harness-prompt-attention-authority.md); [`specs/orkworks-mvp.md#deterministic-harness-supplied-signals`](../../../specs/orkworks-mvp.md#deterministic-harness-supplied-signals).

## Global Constraints

- Preserve ADR 0073's Claude/Copilot allowlists, launch generation, and exact reset commands.
- Acknowledged PTY input is the reset boundary; lifecycle registration can arrive before or after it.
- Never clear a user-owned prompt tuple; preserve Peon fallback before prompt authority activates.
- Keep prompt authority in-memory and bound to the live runtime generation.
- Keep unrelated Codex/OpenCode behavior unchanged.

---

### Task 1: Make reset reservation state represent acknowledgement without a candidate

**Files:**
- Modify `crates/orkworksd/src/runtime/prompt_authority.rs` and its tests.

- [ ] Add tests that fail when an acknowledged reset without a candidate cannot be committed, a late exact lifecycle candidate is accepted, or revocation leaves a reset committable.
- [ ] Run each focused test and confirm it fails for the missing state transition.
- [ ] Represent acknowledgment separately from the optional candidate, preserve the reset slot for one later matching registration, reject stale reports after revocation, and fence completion of a revoked generation.
- [ ] Run the registry tests and confirm pre-ack candidate holding, post-ack binding, and revocation behavior.

### Task 2: Commit reset identity and prompt state at PTY acknowledgement

**Files:**
- Modify `crates/orkworksd/src/session_application.rs`, `crates/orkworksd/src/runtime/terminal_runtime.rs`, and related tests.

- [ ] Add tests for acknowledgement-before-lifecycle: advance the epoch, retire the previous durable identity, clear only non-user prompt state, and accept a later matching lifecycle registration.
- [ ] Add a regression test proving a Copilot prompt queued for the replacement conversation survives the reset input timestamp boundary.
- [ ] Run the new tests and confirm they fail for the current early return and timestamp comparison.
- [ ] Implement durable reset transition and late-candidate completion with failure-safe state ordering; apply queued prompt state after identity persistence.
- [ ] Run focused reset and terminal input tests.

### Task 3: Enforce prompt authority from the target session identity

**Files:**
- Modify `crates/orkworksd/src/http/session_handlers.rs`, `crates/orkworksd/src/session_application.rs`, and handler tests.

- [ ] Add tests proving that a caller-selected generic source cannot replace prompt-harness resume identity and that generic attention cannot overwrite active Claude/Copilot prompt state.
- [ ] Run the new tests and confirm they fail against source-based routing.
- [ ] Select protected registration and attention behavior using the target live session's harness; reject generic writes while its prompt authority is active.
- [ ] Run focused HTTP and application tests, including ordinary-session fallback behavior.

### Task 4: Revoke safely and reconcile integration readiness after errors

**Files:**
- Modify `crates/orkworksd/src/session_application.rs`, `crates/orkworksd/src/http/integration_handlers.rs`, and `crates/orkworksd/src/harness/integration.rs` tests.

- [ ] Add tests proving inactive authority preserves Peon-inferred prompt state, active authority clears only after a successful durable write, and a failed write leaves registry/live/durable state consistent for retry through both explicit revocation and active-harness changes.
- [ ] Add tests proving a failed install/uninstall action still revokes authority when the owned prompt hook is no longer ready.
- [ ] Run the new tests and confirm they fail for unconditional clearing and success-only readiness probing.
- [ ] Serialize reset commit against revocation, surface persistence failure, clear only active non-user prompt state, and evaluate prompt-hook readiness after either action result.
- [ ] Run focused integration and revocation tests.

### Task 5: Verify the PR change

**Files:**
- No additional source files unless a failing test exposes a contract gap.

- [ ] Review every changed behavior against ADR 0073 and all open PR review threads.
- [ ] Run Rust formatting, the sidecar build, and the sidecar test suite.
- [ ] Inspect the final diff and confirm no unrelated prompt mappings or metadata fields changed.
- [ ] Complete each addressed inline review thread with its specific fix and verification evidence.
- [ ] Request current-head automated review after this substantial behavior change and rerun the required manual `/code-review low` gate.
