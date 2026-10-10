---
type: "Implementation Plan"
title: "Live Uncommitted Change Counts Implementation Plan"
description: "Implementation plan: Live Uncommitted Change Counts Implementation Plan."
tags: ["orkworks", "plans"]
---

# Live Uncommitted Change Counts Implementation Plan

> **For agentic workers:** Use executing-plans to implement this approved plan inline, with test-first checks and independent diff review.

**Goal:** Display live file and added/removed line counts in session Details.

**Architecture:** Extend the existing read-only libgit2 Git context and session-list projection with optional `lineChanges: { additions, deletions }`. These statistics are recomputed from the effective working directory, shared per worktree during a listing, and never persisted as session-authored edits.

**Tech Stack:** Rust/git2, existing session JSON DTO, React/TypeScript.

**Spec:** `specs/orkworks-mvp.md`, Git and Worktree Context; issue #723.

## Global Constraints

- Include staged and unstaged changes together against HEAD and non-ignored untracked files; use an empty tree before the first commit.
- Binary files contribute no line totals. Omit unavailable statistics; show clean zero totals.
- Session totals describe their worktree, not individual authorship.
- Preserve existing refresh, cwd precedence, and Electron/renderer boundaries.
- Use pnpm; work only on `live-change-counts` in its sibling worktree.

## Task 1: Compute and project live totals

Files: `crates/orkworksd/src/git.rs`, `session_types.rs`, `session_projection.rs`, and existing DTO/test constructors in `session_view.rs`, `main.rs`, and `http/session_handlers.rs`.

- [x] Add real repository fixtures asserting the serialized session `lineChanges`, including net staged/unstaged changes, deletions, new nested files, ignored files, binary files, unborn HEAD, clean/no repository, and separate worktrees.
- [x] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml git::tests::uncommitted -- --test-threads=4`; confirm missing totals cause failure.
- [x] Add optional `LineChanges { additions: usize, deletions: usize }`, serialized as `lineChanges` on the API DTO only. Initialize absent values in existing constructors.
- [x] Calculate a single `diff_tree_to_workdir_with_index` with `include_untracked`, `recurse_untracked_dirs`, and `show_untracked_content`; obtain insertions/deletions from its statistics. Only unborn/missing HEAD uses an empty tree; other errors leave totals absent.
- [x] Keep recursive changed-file counting consistent with the new untracked file totals. Cache Git context by discovered worktree root within each projection; preserve cwd-based conflict/recommendation logic.
- [x] Run the Git and Git-context projection tests to verify the serialized output and refresh behavior.

## Task 2: Display the counter and document its meaning

Files: `apps/desktop/src/api.ts`, `src/components/SessionDetailPanel.tsx`, `tests/sessionGitChanges.test.mjs`, `docs/user/sessions.md`, `docs/agents/domain-entities.md`.

- [x] Render the actual Details component in tests; require `3 files · +124 −37`, zero totals, singular file count, unborn repository, and no fabricated zero statistics.
- [x] Run `node --test tests/sessionGitChanges.test.mjs`; confirm missing UI counter causes failure.
- [x] Define `lineChanges?: { additions: number; deletions: number }` in `SessionInfo` and render it in the existing Git row with a tooltip explaining the worktree scope. Show the row when the repository exists even before its first commit.
- [x] Update user/session-model documentation; state that this is fresh API projection data, not persisted session authorship.
- [x] Run component tests and `pnpm exec tsc --noEmit`.

## Task 3: Verify, review, and land one PR

- [x] Run Cargo tests, format check, Clippy, desktop build/tests, documentation checks, and `git diff --check`.
- [x] Request an independent diff-scoped code review with an explicit effort level; address findings with evidence.
- [x] Update #723 with approved scope and verification, check active Taskmaster recommendations for an exact match, and open one PR closing #723 (PR #726).
- [ ] Inventory all PR comments, reviews, and checks; handle actionable feedback and merge only after required checks and review pass under the repository maintainer policy.
- [ ] Run the guarded `scripts/finish-pr.sh` cleanup for the concrete merged PR and report fleet worktree flags without touching other owners' work.
