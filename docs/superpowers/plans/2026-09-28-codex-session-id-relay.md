# Codex Session ID Relay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Execute inline in the active session because the task is already approved and repository policy prohibits delegation in this session.

**Goal:** Capture Codex's native session ID when Codex's command sandbox blocks the hook's localhost POST.

**Architecture:** For Codex sessions only, the sidecar creates a private temporary mailbox and passes its path to the child environment. The existing reporter writes bounded, uniquely named JSON reports atomically to that mailbox; the session runtime consumes them through a pinned directory handle and invokes the same authenticated harness-session application path as HTTP. Unix reads and removals are relative to the original directory handle; Windows holds a directory handle that prevents path replacement. The token remains in memory and is not written into the mailbox. The relay retains the Codex hook fingerprint and identity reset guard and does not enable sandbox networking.

**Tech Stack:** Rust sidecar, Tokio, Serde, the existing shell and PowerShell Codex reporters, tempfile, existing session handler and metadata merge.

**Spec:** `specs/orkworks-mvp.md` deterministic harness-supplied signals; ADR 0068.

## Global Constraints

- Keep the OrkWorks session ID and runtime generation authoritative.
- Retain the first accepted native Codex ID; replace only after the recorded explicit reset and an authenticated root `SessionStart(source=clear)`.
- Relay the harness-session report only. Attention and all other report routes retain existing behavior.
- The reporting capability authenticates the OrkWorks session but does not prove operating-system process identity.
- Do not broaden ordinary Codex command network access.

---

### Task 1: Specify the relay and record its architecture

**Files:**
- Modify: `specs/orkworks-mvp.md`
- Create: `docs/adr/0069-codex-session-id-hook-report-mailbox.md`
- Modify: `docs/adr/README.md`

- [x] Document the Codex-only temporary mailbox fallback and its security boundary in the MVP signal section.
- [x] Record atomic report publication, bounded single-use consumption, runtime cleanup, existing authentication and identity validation, and the lack of process-level proof in ADR 0069.
- [x] Add ADR 0069 to the historical index.

### Task 2: Add a real reporter-to-mailbox regression test

**Files:**
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs` reporter tests
- Modify: `crates/orkworksd/scripts/report-harness-event.sh`
- Modify: the matching PowerShell reporter if needed for parity

**Interface:** The reporter reads `ORKWORKS_CODEX_SESSION_REPORT_DIR`; for a Codex harness-session report it atomically writes one JSON envelope containing the existing report fields. The mailbox filename is a fresh UUID and only completed `.json` names are consumable.

- [x] Run the reporter with a Codex hook payload, a temporary mailbox, and a fake `curl` executable that records calls.
- [x] Assert the native ID, event provenance, fingerprint, token and reset fields survive in one parseable envelope, and no HTTP attempt is made for the identity report.
- [x] Assert non-Codex reports and Codex attention handling retain their existing HTTP behavior.
- [x] Run the focused test and confirm it fails because the reporter currently attempts HTTP instead of creating the mailbox report.

### Task 3: Consume reports through the existing authenticated identity path

**Files:**
- Create: `crates/orkworksd/src/runtime/codex_hook_report_relay.rs`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs` for Codex-only environment setup if needed
- Modify: `crates/orkworksd/src/http/session_handlers.rs` to expose the existing report inner path to the local consumer
- Modify: `crates/orkworksd/src/main.rs` only if module registration is needed

**Interface:** `CodexHookReportEnvelope { report: HarnessSessionReportRequest }`; `CodexHookReportRelay::new() -> io::Result<Self>`; `consume_ready(&self, state: Arc<AppState>, session_id: &str, report_token: &str, runtime_generation: u64) -> RelayOutcome`. Consume only exact UUID `.json` files through a pinned mailbox handle, enforce a 4 KiB per-report and 32-entry scan bound, deserialize before use, revalidate the in-memory report token after filesystem scanning, and call the same hook observation, metadata merge and Codex reset logic as the HTTP handler. Remove each file after one processing attempt. The PTY driver's existing select loop polls the relay only for its owning live Codex runtime. Cleanup removes the runtime-owned directory when its path is still the original mailbox and preserves a replacement path rather than deleting an unrelated directory.

- [x] Add a failing consumer test proving a valid report with the active token updates the session's persisted native ID while the HTTP route is not involved.
- [x] Add rejection coverage for malformed JSON, oversized files, invalid token, stale runtime, invalid source/fingerprint, revoked token, and mailbox path replacement.
- [x] Implement the relay and expose the shared harness-session logic without duplicating its authorization or merge rules.
- [x] Run the focused tests and confirm accepted files update metadata, invalid files do not, and each file is consumed once.

### Task 4: Verify hook behavior and document the operating contract

**Files:**
- Modify: `docs/agents/architecture.md`
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs` only for remaining test support

- [x] Document that the mailbox is ephemeral and Codex identity only; temp-directory creation failure leaves the reporter's existing HTTP fallback in place and emits a redacted diagnostic.
- [x] Confirm the helper and shell/PowerShell reporter diagnostics never include the native ID, report token, or full payload.
- [x] Run focused Rust tests, `cargo build`, `cargo fmt --check`, and `git diff --check`. The full Rust suite was attempted but a network-dependent test could not reach `models.opencode.ai` inside the sandbox.
- [ ] Run the required manual `/code-review low` gate and review the final diff against issue #673 acceptance criteria and ADR 0068 before merge.
