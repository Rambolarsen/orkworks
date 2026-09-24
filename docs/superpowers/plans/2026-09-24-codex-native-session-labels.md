# Codex Native Session Labels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enrich live Codex session labels from the native saved name or generated title while preserving safe fallback behavior.

**Architecture:** A deep `codex_session_store` adapter owns deterministic path resolution, read-only SQLite access, exact native-ID lookup, normalization, and bounded failure behavior. `SessionApplication` owns label provenance, capability-gated scheduling, runtime/workspace revalidation, and atomic durable/live publication.

**Tech Stack:** Rust 2021, Axum/Tokio, bundled `rusqlite`, serde metadata, existing Codex hook reporter and per-session report capability.

**Spec:** `docs/superpowers/specs/2026-09-24-codex-native-session-labels-design.md` and ADR 0063.

## Global Constraints

- Use only `$CODEX_HOME/state_5.sqlite` or `~/.codex/state_5.sqlite`; never scan by newest mtime/suffix or accept a caller path.
- Query exact native session IDs; do not read `first_user_message`, rollout JSONL, or `rollout_path` for labels.
- Native enrichment requires the live session's `ORKWORKS_REPORT_TOKEN`; unauthenticated identity reporting remains backward-compatible but cannot trigger private reads.
- Native labels may replace automatic label sources only; future explicit user labels win.
- Missing/unsupported/unreadable data preserves the existing label and does not publish candidate text to logs.
- All blocking SQLite work runs outside Tokio/application locks; use read-only bundled SQLite with bounded resources.
- Run `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`, `cargo build --manifest-path crates/orkworksd/Cargo.toml`, and `cargo test --manifest-path crates/orkworksd/Cargo.toml` before completion.

---

### Task 1: Add the deep Codex state-store adapter

**Files:**
- Create: `crates/orkworksd/src/codex_session_store.rs`
- Modify: `crates/orkworksd/Cargo.toml`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/codex_session_store.rs`

**Interfaces:**
- Produces `CodexLabelCandidate`, `CodexStoreError`, and `lookup_label_candidate(native_session_id: &str) -> Result<Option<CodexLabelCandidate>, CodexStoreError>`.
- The adapter hides path resolution, schema validation, read-only connection setup, exact-ID query, normalization, and bounds from callers.

- [ ] **Step 1: Write failing adapter tests** for `name` before `title`, blank-name fallback, blank-title absence, exact-ID matching, missing store, unsupported schema, control rejection, and display bounds using temporary SQLite fixtures.
- [ ] **Step 2: Run the focused tests** with `cargo test --manifest-path crates/orkworksd/Cargo.toml codex_session_store`; confirm failure because the module and dependency are absent.
- [ ] **Step 3: Add bundled `rusqlite` and the module declaration** with no production call sites yet.
- [ ] **Step 4: Implement deterministic store resolution** using `CODEX_HOME` or `dirs::home_dir`, accepting only `state_5.sqlite`; reject absent/ambiguous/unsupported paths without logging text.
- [ ] **Step 5: Implement read-only exact-ID lookup** with fixed parameterized SQL over the verified `threads` shape, bounded busy timeout, no schema/journal mutation, and normalized bounded output.
- [ ] **Step 6: Run the focused tests** and confirm all adapter cases pass with no warnings.
- [ ] **Step 7: Commit** with `git add crates/orkworksd/Cargo.toml crates/orkworksd/Cargo.lock crates/orkworksd/src/main.rs crates/orkworksd/src/codex_session_store.rs && git commit -m "feat(sidecar): read Codex native session labels"`.

### Task 2: Persist label provenance and protect automatic writers

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Test: the existing unit-test modules in those files

**Interfaces:**
- Produces `LabelSource` with placeholder, initial-prompt, terminal-input, peon, codex, user, and legacy states.
- Produces source-aware label persistence that accepts a candidate only when the current source is automatic and the session remains current.

- [ ] **Step 1: Add failing metadata tests** proving legacy records default safely, terminal/Peon writes record their source, Codex can replace automatic sources, and user/legacy labels are preserved.
- [ ] **Step 2: Run the focused metadata/session tests** and confirm the new ownership assertions fail.
- [ ] **Step 3: Add the serialized backward-compatible `labelSource` field** and update all metadata constructors with explicit sources.
- [ ] **Step 4: Update terminal, bootstrap, Peon, and reset writers** to set or clear provenance atomically with the label while preserving existing epoch/runtime guards.
- [ ] **Step 5: Add the source-aware Codex persistence helper** that rechecks the current native ID, live runtime, label source, and workspace before writing durable and live projections.
- [ ] **Step 6: Run focused tests** and confirm stale/lower-authority writers cannot overwrite native or user labels.
- [ ] **Step 7: Commit** with `git add crates/orkworksd/src/metadata.rs crates/orkworksd/src/session_application.rs crates/orkworksd/src/runtime/terminal_runtime.rs crates/orkworksd/src/runtime/peon_runtime.rs && git commit -m "feat(sidecar): track session label provenance"`.

### Task 3: Gate and schedule Codex native-label enrichment

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Modify: `crates/orkworksd/scripts/report-harness-event.sh`
- Modify: `crates/orkworksd/scripts/report-harness-event.ps1`
- Test: `crates/orkworksd/src/http/session_handlers.rs` and integration-script tests

**Interfaces:**
- The harness-session route accepts an optional `Authorization` header for capability-gated enrichment while preserving existing identity-report behavior.
- Accepted Codex reports schedule bounded `spawn_blocking` lookup attempts and call the source-aware application helper only after revalidation.

- [ ] **Step 1: Add failing route/script tests** for token forwarding, missing/wrong token no-enrichment, accepted Codex enrichment, non-Codex no-enrichment, and stale native-ID protection.
- [ ] **Step 2: Run the focused route and script tests** and confirm they fail before integration exists.
- [ ] **Step 3: Forward `ORKWORKS_REPORT_TOKEN` as a bearer header** only when present; preserve the current no-token request shape otherwise.
- [ ] **Step 4: Verify the token and accepted Codex session identity** before spawning any private-state read; never accept a database path from the request.
- [ ] **Step 5: Schedule one bounded lookup plus short retry delays** outside the async/application locks, then revalidate native ID, workspace, runtime, lifecycle, and label source before atomic persistence.
- [ ] **Step 6: Run focused tests** and confirm failures remain fallback-only with no candidate text in logs.
- [ ] **Step 7: Commit** with `git add crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/src/session_application.rs crates/orkworksd/src/runtime/terminal_runtime.rs crates/orkworksd/scripts/report-harness-event.sh crates/orkworksd/scripts/report-harness-event.ps1 crates/orkworksd/src/harness/integrations/mod.rs && git commit -m "feat(sidecar): apply authenticated Codex native labels"`.

### Task 4: Update durable architecture documentation

**Files:**
- Modify: `docs/agents/architecture.md`
- Modify: `docs/agents/harness-integration-contracts.md`
- Modify: `specs/orkworks-mvp.md`

- [ ] **Step 1: Add the accepted label-source and Codex adapter contract** with the exact path, precedence, privacy, fallback, and opportunistic-refresh limits from ADR 0063.
- [ ] **Step 2: Run repository documentation checks** relevant to changed Markdown and confirm links resolve.
- [ ] **Step 3: Commit** with `git add docs/agents/architecture.md docs/agents/harness-integration-contracts.md specs/orkworks-mvp.md && git commit -m "docs: document Codex native session labels"`.

### Task 5: Verify the complete branch

**Files:**
- Test only; no additional source files.

- [ ] **Step 1: Run `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`** and fix any formatting failures.
- [ ] **Step 2: Run `cargo build --manifest-path crates/orkworksd/Cargo.toml`** and confirm the sidecar builds with bundled SQLite.
- [ ] **Step 3: Run `cargo test --manifest-path crates/orkworksd/Cargo.toml`** and record the complete passing output.
- [ ] **Step 4: Run `git diff --check` and the repository documentation checks**.
- [ ] **Step 5: Inspect the final diff against issue #602 and ADR 0063**, then request the required `/code-review low` gate for this Rust change.
