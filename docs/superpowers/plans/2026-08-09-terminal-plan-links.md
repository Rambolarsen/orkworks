# Terminal Plan Links Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make printed plan/spec paths clickable in live and historical terminals, safely associate the chosen artifact, and open it in Review.

**Architecture:** The sidecar owns an anchored, source-aware plan reference and revalidates it for selection, rendering, and fixed review handoff. xterm only detects visible paths; Electron forwards an explicit click through a token-authenticated route.

**Tech Stack:** Rust/Axum/serde/git2, Electron IPC, React/TypeScript, xterm.js, node:test.

## Global Constraints

- Allow Markdown only under `docs/superpowers/plans/`, `docs/superpowers/specs/`, or `specs/`.
- Relative paths use the immutable launch worktree; absolute paths require the same Git common directory.
- Authority is `user_selected > hook_reported > terminal_fallback`; a user clear suppresses automatic sources.
- No `file:` URL, generic file opener, generic terminal-write API, watcher, or dependency.

---

### Task 1: Record the revised privileged contract

**Files:** Create `docs/adr/0039-terminal-plan-link-selection.md`; modify `docs/adr/README.md`, `specs/session-plan-review.md`, `docs/agents/architecture.md`, and `docs/agents/domain-entities.md`.

- [ ] Add ADR 0039 before code: a dedicated preload method may send only string `sessionId` and string `printedPath`; Electron uses the sidecar secret; Rust remains authoritative. It supersedes ADR 0025 only for this method.
- [ ] Document `PlanReference { worktreeRoot, relativePath, source }`, legacy-string reads, click-to-associate, and failure preservation.
- [ ] Run `bash .claude/hooks/doc-check.sh` (expect 0) and commit `docs: define terminal plan link selection`.

### Task 2: Implement the anchored plan reference and resolver

**Files:** Modify `crates/orkworksd/src/metadata.rs` and `crates/orkworksd/src/plan_handoff.rs`; add inline Rust tests.

**Interfaces:** `PlanSource`, `PlanReference`, `resolve_openable_plan_ref(session, reference) -> Result<PathBuf, String>`, and `select_terminal_plan(session, printed_path) -> Result<PlanReference, String>`.

- [ ] Write failing tests for legacy `"planPath":"specs/p.md"`, object serialization, precedence, clear tombstones, same relative sibling names, absolute same-common-dir acceptance, unrelated-repo rejection, symlink/removed-anchor rejection, and post-print-CWD-change determinism.
- [ ] Implement untagged serde decoding:

```rust
#[serde(untagged)]
enum StoredPlanReference { Legacy(String), Anchored(PlanReference) }
```

New writes use `worktreeRoot`, `relativePath`, and `source`; hook input stays string-compatible and normalizes server-side.

- [ ] Implement one `git2::Repository::discover` resolver: canonicalize roots and target, enforce regular Markdown + allowed root, compare Git common directories for absolute sibling worktrees, and use it before availability, content reads, and review prompt submission.
- [ ] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml plan_handoff metadata` (expect PASS) and commit `feat(sidecar): anchor session plan references`.

### Task 3: Add selection route and Electron bridge

**Files:** Modify `crates/orkworksd/src/http/session_handlers.rs`, `crates/orkworksd/src/main.rs`, `apps/desktop/electron/{planOpener,main,preload}.ts`, and `apps/desktop/src/orkworksWindow.d.ts`; create `apps/desktop/tests/planOpener.test.ts`.

**Interfaces:** `POST /sessions/:id/select-terminal-plan` accepts `{ "printedPath": string }`; preload exposes `selectTerminalPlan(sessionId, printedPath): Promise<void>`.

- [ ] Write failing tests: missing token → 401; invalid/unrelated path → 409; valid path → 204, persisted `UserSelected`, and `session.plan_selected_by_user`; wrong IPC types make no request.
- [ ] Add the handler, route, token header, and Electron type checks. Refactor plan-content and request-review handlers to call Task 2’s shared resolver immediately before I/O/PTY input. Do not expose the token.
- [ ] Run `pnpm --dir apps/desktop exec tsx --test tests/planOpener.test.ts` and `cargo test --manifest-path crates/orkworksd/Cargo.toml select_terminal_plan` (expect PASS); commit `feat: select terminal plans through Electron`.

### Task 4: Link terminal paths and activate Review

**Files:** Modify `apps/desktop/src/{terminalLinks,terminalStore,App}.ts`, `apps/desktop/src/components/{HistoricalTerminal,DockviewApp,ReviewPanel}.tsx`; modify `apps/desktop/tests/{terminalLinks,dockview}.test.ts`; create `apps/desktop/tests/reviewPanel.test.ts`.

**Interfaces:** `createTerminalPlanLinkProvider(sessionId, onPlanPath)` and `onTerminalPlanSelected(sessionId): Promise<void>`.

- [ ] Write parser/provider tests for quoted, trailing-punctuation relative/absolute allowed paths; reject ordinary docs/malformed fragments; prove exactly-once click and preserved HTTPS behavior.
- [ ] Keep `terminalLinkHandler` for HTTP(S); add the custom provider and register/dispose it in both live `terminalStore` and historical `HistoricalTerminal`. A successful callback invokes preload then emits a plan-selected event.
- [ ] Write and implement UI sequencing tests: refresh sessions before Review opens, failed clicks retain the current Review artifact, and changing active session does not replace it. Store `reviewSessionId` independently from `activeSessionId`; Details actions set it for existing references.
- [ ] Run `pnpm --dir apps/desktop exec tsx --test tests/terminalLinks.test.ts tests/dockview.test.ts tests/reviewPanel.test.ts` (expect PASS); commit `feat(desktop): review terminal plan links`.

### Task 5: Verify and review

**Files:** Only scoped verification fixes.

- [ ] Run:

```bash
pnpm --dir apps/desktop exec tsc --noEmit
pnpm --dir apps/desktop exec tsx --test tests/*.test.ts tests/*.test.mjs
cargo test --manifest-path crates/orkworksd/Cargo.toml
git diff --check
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: every command exits 0.

- [ ] Manually verify live Codex and dead replay links; a sibling-worktree click opens Review and Details; unrelated paths preserve Review; independent review writes the fixed prompt exactly once.
- [ ] Run the required lightweight code review, address findings, and commit `test: verify terminal plan review flow`.
