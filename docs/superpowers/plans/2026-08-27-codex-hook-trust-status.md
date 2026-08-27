# Codex Hook Trust Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let OrkWorks change Codex integration status from `needs_trust` to `active` only after the exact installed hook definition reports back.

**Architecture:** Codex's generated command will contain a SHA-256 fingerprint of its hook definition excluding the fingerprint argument itself. The reporter sends that fingerprint with the existing harness-session report; the sidecar persists the last observed fingerprint in `codex-hook-observation.json` and derives activation by comparing it with the currently installed definition.

**Tech Stack:** Rust/Axum sidecar, serde JSON metadata, Bash/PowerShell reporters, React/TypeScript settings UI.

## Global Constraints

- Codex trust remains user-controlled through `/hooks`; OrkWorks only records observed execution.
- Existing non-Codex harness reports and activation states must remain unchanged.
- The renderer receives only the existing `IntegrationStatus` shape; no Codex filesystem or trust-store access is added.
- Use SHA-256 already provided by the sidecar's `sha2` dependency.

---

### Task 1: Add the fingerprint to the Codex hook command and reporter payload

**Files:**
- Modify: `crates/orkworksd/src/harness/integrations/codex.rs`
- Modify: `crates/orkworksd/scripts/report-harness-event.sh`
- Modify: `crates/orkworksd/scripts/report-harness-event.ps1`
- Test: existing Rust integration tests in `codex.rs` and `integrations/mod.rs`

**Interfaces:**
- Produces a deterministic `codex_hook_fingerprint(reporter: &Path) -> Result<String, IntegrationError>` and a command containing `--hook-fingerprint '<sha256>'`.
- Reporter sends optional JSON field `hookFingerprint` only for the Codex marker.

- [ ] **Step 1: Add failing tests** for deterministic fingerprinted command text, changed command fingerprint, and reporter forwarding.
- [ ] **Step 2: Run the focused Rust tests and confirm they fail because the command and payload lack the fingerprint.**
- [ ] **Step 3: Implement the shared command builder and SHA-256 fingerprinting, then parse/forward the argument in both reporters.**
- [ ] **Step 4: Run the focused tests and confirm they pass.**
- [ ] **Step 5: Commit with `feat(codex): report hook definition fingerprints`.**

### Task 2: Persist observed Codex fingerprints and derive activation

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs`
- Modify: `crates/orkworksd/src/harness/integrations/codex.rs`
- Test: metadata, session application, HTTP integration, and Codex integration tests

**Interfaces:**
- `HarnessSessionReport` carries an optional `hook_fingerprint`.
- `codex-hook-observation.json` persists the last valid observed fingerprint and timestamp.
- Codex activation compares the stored observation with the current generated command; other handlers retain their existing activation behavior.

- [ ] **Step 1: Add failing tests** proving a matching observed fingerprint reports `active`, a missing/mismatched fingerprint reports `needs_trust`, and legacy reports without the field remain valid.
- [ ] **Step 2: Run the focused Rust tests and confirm the expected failures.**
- [ ] **Step 3: Persist the fingerprint only after a valid Codex hook report and clear it on Codex uninstall when appropriate.**
- [ ] **Step 4: Derive Codex activation from exact fingerprint equality while preserving `needs_trust` for absent or mismatched evidence.**
- [ ] **Step 5: Run the full Rust test suite.**
- [ ] **Step 6: Commit with `feat(sidecar): surface trusted Codex hook activation`.**

### Task 3: Update the reporter contract, UI copy, and documentation

**Files:**
- Modify: `apps/desktop/src/harnessTypes.ts` only if the serialized status shape changes
- Modify: `apps/desktop/src/components/HarnessIntegrationSection.tsx`
- Modify: `apps/desktop/tests/providersPanel.test.ts`
- Modify: `docs/adr/0035-codex-session-start-hook-not-attention-signal.md`
- Modify: `docs/agents/harness-integration-contracts.md`
- Modify: `AGENTS.md` and `README.md` if the metadata protocol description changes

- [ ] **Step 1: Add a renderer source test for the active Codex copy.**
- [ ] **Step 2: Implement the copy: active means the exact hook has executed; needs trust means no matching execution has been observed.**
- [ ] **Step 3: Update the ADR/evidence register with the observed-execution trust limitation.**
- [ ] **Step 4: Run desktop type-check and tests.**
- [ ] **Step 5: Run `bash scripts/doc-check.sh` and `bash .claude/hooks/worktree-check.sh`.**
- [ ] **Step 6: Commit with `feat(desktop): show active Codex hook trust status`.**

## Self-review

- The plan does not attempt to read Codex's private trust database.
- A stale hook cannot mark the current hook active because the fingerprint is derived from the current command and compared exactly.
- Existing reports without `hookFingerprint` remain accepted for resume capture; they simply cannot prove Codex hook activation.
- The UI remains conservative after config drift, reinstall, or app restart until matching evidence is observed.
