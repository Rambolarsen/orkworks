# Antigravity CLI Harness Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Gemini CLI as a selectable built-in with Antigravity CLI while preserving Gemini history and settings compatibility.

**Architecture:** Add an `antigravity` built-in definition using only documented `agy` launch/resume commands. Keep the legacy `gemini` definition readable but mark it retired, then filter retired definitions at the new-session boundary. No Antigravity integration, provider, capacity, voice, or model-argument capability is introduced.

**Tech Stack:** Rust sidecar harness registry and React/TypeScript desktop renderer.

## Global Constraints

- Harness ID is `antigravity`; command is `agy`.
- Exact resume is `agy --conversation={harnessSessionId}`; latest resume is `agy --continue` in the session cwd.
- Do not migrate Gemini metadata, IDs, hooks, settings, or conversations.
- Gemini remains available only for persisted settings and historical sessions; new sessions must not select it.
- No undocumented capability may be inferred or installed.

---

### Task 1: Define retired and Antigravity built-ins

**Files:**
- Modify: `crates/orkworksd/src/harness/definition.rs`
- Modify: `crates/orkworksd/src/harness/registry.rs`
- Test: existing definition/registry tests in those modules

- [x] Add a serializable `retired: bool` definition field with a false default.
- [x] Add the `antigravity` built-in: `agy`, no default model, exact conversation resume, cwd-scoped latest resume, and no integration/provider/voice/capacity bindings.
- [x] Mark `gemini` retired without altering its ID or existing integration binding.
- [x] Add tests proving Antigravity launch/exact/latest rendering and that Gemini still resolves through persisted definitions.
- [ ] Commit: `feat: add Antigravity CLI harness definition`.

### Task 2: Hide retired built-ins from new sessions

**Files:**
- Modify: `crates/orkworksd/src/http/harness_handlers.rs`
- Modify: `apps/desktop/src/newSessionDialogState.ts`
- Modify: `apps/desktop/tests/newSessionDialogState.test.ts`

- [x] Add a `retired` API field and filter retired built-ins from selectable harness results while preserving custom harnesses and historical display resolution.
- [x] Make an invalid remembered Gemini new-session draft fall back to the first selectable harness.
- [x] Add focused Rust and renderer tests.
- [ ] Commit: `feat: hide retired harnesses from new sessions`.

### Task 3: Update presentation and documentation

**Files:**
- Modify: `apps/desktop/src/harnessIcons.ts`
- Modify: `apps/desktop/tests/harnessIcon.test.ts`
- Modify: `docs/user/getting-started.md`
- Modify: `docs/agents/architecture.md`
- Modify: `docs/agents/harness-integration-contracts.md`

- [x] Add Antigravity icon fallback/presentation while retaining Gemini’s historical icon.
- [x] Document `agy` launch/resume behavior, Gemini retirement, and unsupported capabilities.
- [ ] Commit: `docs: document Antigravity CLI migration`.

### Task 4: Verify and review

- [x] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml`.
- [x] Run `pnpm exec tsc --noEmit` and `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` from `apps/desktop`.
- [x] Run `bash .claude/hooks/doc-check.sh`, `bash .claude/hooks/worktree-check.sh`, and `git diff --check`.
- [x] Request code review and address findings before opening a PR for issue #297.
