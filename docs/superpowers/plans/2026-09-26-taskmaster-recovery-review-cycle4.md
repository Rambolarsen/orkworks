# Taskmaster Recovery Review Fixes

> **For agentic workers:** Execute this plan inline in the current PR worktree.

**Goal:** Make manual Brain analysis revalidate cached blockers and recover only truly orphaned recommendations.

**Architecture:** Keep recovery in the existing recommendation store. The HTTP handler supplies currently live runtime IDs and IDs whose prompt delivery is protected by the workspace-scoped delivery guard. The renderer always asks the backend to revalidate a blocker, so local cached state cannot prevent recovery.

**Tech Stack:** Rust, Axum, React/TypeScript, Node test runner.

**Spec:** `specs/taskmaster-knowledge.md` (manual analysis gate and recovery behavior); `specs/taskmaster.md` (manual Analyze now flow).

## Blind-spot checkpoint

- **Least confident:** Whether a session-map entry is sufficient evidence that a recommendation target is still live. `complete_session_ending` was inspected and leaves dead handles in the map. The lifecycle vocabulary shows `stopping`/`ending` are transitional states before `dead`/`ended`, so recovery will treat `alive`/`active` and `stopping`/`ending` handles as live, and exclude terminal handles.
- **Project blind spot:** Renderer cache and map membership are projections, not lifecycle authority. The backend must revalidate on each explicit Analyze request, and its recovery pass must preserve records protected by an in-flight delivery. The accepted specs already cover this; no scope change or ADR is needed.

## Global Constraints

- Manual analysis remains available with Background discovery disabled.
- An active Brain recommendation blocks a new analysis until it is completed or dismissed.
- In-flight Fix with AI delivery cannot be reset by orphan recovery.
- Keep Electron main and renderer boundaries intact.

---

### Task 1: Make orphan recovery lifecycle-aware and delivery-safe

**Files:**
- Modify: `crates/orkworksd/src/session_types.rs`
- Modify: `crates/orkworksd/src/http/taskmaster_handlers.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/taskmaster/store.rs`

- [x] Add `SessionInfo::has_live_runtime` tests for terminal and transitional states, and a store test proving protected in-flight recommendations remain `Executing` during recovery.
- [x] Run the focused Rust and renderer tests before implementation; confirm the missing lifecycle method/store parameter and cached-blocker early return fail as expected.
- [x] Filter runtime IDs to nonterminal lifecycle phases at both recovery call sites; pass delivery-guarded recommendation IDs into store recovery and skip those records.
- [x] Run focused checks: `cargo test --offline --manifest-path crates/orkworksd/Cargo.toml taskmaster::store::tests::orphan -- --test-threads=1`, `cargo test --offline --manifest-path crates/orkworksd/Cargo.toml session_types::tests::live_runtime_excludes_terminal_lifecycle_states -- --exact`, and `cargo test --offline --manifest-path crates/orkworksd/Cargo.toml session_application::tests::dismiss_recommendation_can_recover_a_stuck_execution -- --exact`.
- [x] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml`; result: 1,421 passed, 3 ignored.

### Task 2: Let the backend revalidate cached blockers

**Files:**
- Modify: `apps/desktop/src/components/RecommendationsPanel.tsx`
- Modify: `apps/desktop/tests/taskmaster.test.ts`

- [x] Add a regression assertion that Analyze now reaches `requestTaskmasterAnalysis` even when a local blocked recommendation is cached.
- [x] Run `pnpm --dir apps/desktop exec node --experimental-strip-types --test --test-name-pattern='Analyze now asks the backend to revalidate' tests/taskmaster.test.ts` before implementation and confirm the existing early return fails it.
- [x] Remove the local short-circuit; display the backend's active-recommendation or recovery result as the authoritative response.
- [x] Run `pnpm --dir apps/desktop exec node --experimental-strip-types --test tests/electronSidecarWiring.test.ts tests/taskmaster.test.ts` (76 passed) and `pnpm --dir apps/desktop exec tsc --noEmit`.

### Task 3: Verify and hand off the review cycle

- [x] Confirm `origin/main` contains the deterministic nested-process test fix in #644; keep that unrelated fix out of this PR's diff.
- [x] Run `cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check`, `git diff --check`, and `bash scripts/doc-check.sh` (passed; docs check emitted its standard advisory).
- [ ] Review the final diff and push one commit to PR #642.
- [ ] Trigger exactly one fresh Codex review and request one Copilot re-review for the new head.
- [ ] Refresh the full PR inventory and report CI, review, and merge state without merging past the required `/code-review low` gate.
