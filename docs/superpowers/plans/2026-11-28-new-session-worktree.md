---
type: Implementation Plan
title: Opt-in worktree launch for new agent sessions implementation
description: Deliver opt-in New Agent Session worktree provisioning under ~/.orkworks/worktrees/, Git-safe hook reconciliation, agent handoff, and safe archive/delete cleanup.
tags: [orkworks, sessions, git, worktrees, implementation]
status: draft
workflow_status: proposed
---

# Opt-in worktree launch for new agent sessions implementation

Design: [Opt-in worktree launch for new agent sessions](../specs/2026-11-28-new-session-worktree-design.md).
Prerequisite plan (`OrganizationStore` v2 foundation):
[Auto-archive, favorites, and staged session retention](2026-11-28-auto-archive-favorites-retention.md).
Related ADRs: [ADR 0008](../../adr/0008-git-context-detection-not-control.md),
[ADR 0094](../../adr/0094-explicit-session-worktree-binding.md).
Tracking issue: create before runtime implementation (none exists yet).

Pass condition: users in a non-bare Git workspace with at least one commit can
opt into **New worktree** in the New Agent Session composer, pick a local or
remote-tracking base branch, launch directly inside an OrkWorks-managed
worktree under `~/.orkworks/worktrees/` with Git-safe hook reconciliation and
`ORKWORKS_MANAGED_WORKTREE=1` handoff, and have clean managed worktrees removed
automatically on Archive/Delete while dirty worktrees are preserved and flagged
for revisit.

## Boundaries and investigated evidence

Build on `OrganizationStore` v2 (from the prerequisite staged-retention plan),
existing `git.rs` / `session_location.rs` canonicalization, existing owned
subprocess helpers, existing harness integration installers, and Electron-main
desktop authorization (`x-orkworks-open-plan-token`). Do not add `git fetch`,
branch switching in the primary checkout, branch deletion, `--force` worktree
removal, automatic dependency installs, or changes to `#610` orchestration.

Investigated codebase constraints that shape the deliverables:

- All five workspace hook integrations install untracked/ignored local files
  (`.claude/settings.local.json`, `.github/copilot/settings.local.json`,
  `.opencode/plugins/orkworks-session-reporter.js`) or shared files
  (`.codex/hooks.json`, `.agents/hooks.json`) gated by
  `require_local_or_ignored_untracked`. A fresh `git worktree add` omits
  untracked files, so reconciliation must run against `<managed-path>` while
  preserving Git-safety gates (never writing unignored files or dirtying tracked
  shared files) and degrading gracefully to Peon/PTY fallback when a base branch
  is incompatible.
- `create_session_workflow` persists a `creating` record and returns
  asynchronously before background PTY spawn (`publish_startup_failure` records
  spawn errors). Git provisioning runs synchronously before returning `creating`
  so invalid base refs keep the composer draft intact, while background PTY
  spawn failures trigger clean-only worktree + unused-branch rollback inside
  `publish_startup_failure`.
- `agent_patch.rs` currently lets agents overwrite or clear `worktree_binding`
  and `worktree_instructions.rs` injects worktree-reporting prompts. For
  managed-worktree sessions, suppress `worktree-instructions` and reject agent
  `worktreeBinding` mutations so `managedWorktree.root` and `worktreeBinding`
  stay pinned together.

## Delivery 1: governance prerequisites, managed-worktree core, branch listing, and Git-safe hook reconciliation

Files: new ADR amending `ADR 0008` (+ `docs/adr/index.md`);
`specs/orkworks-mvp.md`; new `crates/orkworksd/src/managed_worktree.rs`;
`crates/orkworksd/src/harness/integrations/` (worktree-targeted install and
readiness helper); `crates/orkworksd/src/http/session_handlers.rs` (or git
handler); `crates/orkworksd/src/main.rs`.

- [ ] Open the GitHub tracking issue, record the ADR amending
  [ADR 0008](../../adr/0008-git-context-detection-not-control.md), update
  `specs/orkworks-mvp.md` with the opt-in launch-time worktree convenience and
  clean-only archive cleanup boundary, and index the ADR before runtime code.
- [ ] Write failing temp-Git-repo unit tests and implement local-only branch
  listing (`GET /workspace/git/branches`) returning local branches, remote
  tracking branches (excluding symbolic `*/HEAD`), detached `HEAD`, eligibility
  classification (non-Git / bare / unborn), and default base selection
  (`refs/remotes/origin/HEAD` -> `origin/main` -> `origin/master` -> `main` ->
  `master` -> current branch/`HEAD`), with zero network I/O.
- [ ] Write failing unit tests and implement `managed_worktree.rs` provisioning:
  strict canonical containment under
  `~/.orkworks/worktrees/<workspace-hash>/<worktree-id>/`, prompt-slug branch
  generator (`orkworks/<slug>-<short-id>` / `orkworks/<short-id>`) with Git ref
  validation and collision retry, and bounded owned `git worktree add -b`.
- [ ] Implement Git-safe hook reconciliation for the launched harness when its
  integration is already enabled and owned in the parent workspace: preserve
  `require_local_or_ignored_untracked`, reuse already-present tracked shared
  hook entries without editing tracked files, record exact written files +
  SHA-256 digests in `managedHookFiles`, evaluate prompt-hook readiness against
  `<managed-path>`, and return a non-fatal `hookNote` (degraded fallback) on
  base-branch incompatibility.
- [ ] Run targeted Rust tests (`cargo test --manifest-path crates/orkworksd/Cargo.toml managed_worktree`) and confirm all pass.

Interface: `GET /workspace/git/branches` returns `{ eligible, reason?, defaultRef?,
refs: [{ name, kind }] }`; `managed_worktree::provision(...)` returns
`ManagedWorktreeRecord` plus `base_oid` and rollback handle.

## Delivery 2: desktop-authorized session creation, async rollback, agent handoff, pinned binding, and skill update

Files: `crates/orkworksd/src/http/session_handlers.rs`;
`crates/orkworksd/src/session_application.rs`;
`crates/orkworksd/src/runtime/session_runtime.rs`;
`crates/orkworksd/src/runtime/terminal_runtime.rs`;
`crates/orkworksd/src/metadata/agent_patch.rs`;
`skills/starting-work/SKILL.md`.

- [ ] Extend `CreateSessionRequest` with optional `worktreeMode: "new"` and
  `baseRef` (strict deserialization). Require Electron-main desktop token
  (`authorize_plan_request` / `x-orkworks-open-plan-token`) when
  `worktreeMode == "new"`, matching image-attachment authorization.
- [ ] Run synchronous Git validation, provisioning, hook reconciliation, and
  `OrganizationStore` v2 `managedWorktree` persistence before returning
  `creating` (pre-populating `worktreeBinding` and setting `cwd` to
  `<managed-path>`). Roll back worktree + unused branch immediately on pre-record
  failure, and run clean-only worktree + unused-branch rollback inside
  `publish_startup_failure` on background PTY spawn failure (recording
  `cleanupStatus = "removed"` or `"removal_failed"`).
- [ ] Export `ORKWORKS_MANAGED_WORKTREE=1` and
  `ORKWORKS_MANAGED_WORKTREE_ROOT=<canonical root>` on managed launches and
  resumes while stripping both names case-insensitively on all launches;
  suppress `worktree-instructions` injection on managed sessions.
- [ ] Update `agent_patch.rs` to reject modifying or clearing `worktreeBinding`
  on sessions with an active `managedWorktree`, and update Resume admission to
  refuse cleanly (no primary-checkout fallback) when `cleanupStatus == "removed"`.
- [ ] Update `skills/starting-work/SKILL.md` with the
  `ORKWORKS_MANAGED_WORKTREE=1` preflight check (skip double-worktree creation
  and close-out self-deletion while keeping repo-specific setup).
- [ ] Add Rust tests covering desktop-token gate, pre-record & async spawn
  rollback, image-attachment coexistence, env stripping, pinned `worktreeBinding`,
  and post-removal resume refusal; confirm all pass.

## Delivery 3: fail-closed clean-only cleanup on Archive/Delete, dirty flagging, and retry command

Files: `crates/orkworksd/src/managed_worktree.rs`;
`crates/orkworksd/src/session_archive.rs`;
`crates/orkworksd/src/session_application.rs`;
`crates/orkworksd/src/runtime/retention.rs`;
`crates/orkworksd/src/session_types.rs`;
`crates/orkworksd/src/session_projection.rs`;
`crates/orkworksd/src/http/session_handlers.rs`;
`crates/orkworksd/src/main.rs`.

- [ ] Write failing temp-Git-repo tests and implement fail-closed clean-only
  removal in `managed_worktree.rs`: verify `(sessionId, createdAt)` ownership,
  closed runtime, strict `~/.orkworks/worktrees/<workspace-hash>/` containment,
  no other live session referencing the path, external deletion pruning,
  explicit on-disk SHA-256 verification of every recorded `managedHookFiles`
  entry (modified/symlinked ignored hooks mark the worktree dirty), and
  `git status --porcelain=v1 --untracked-files=all --ignored=traditional`
  preserving ignored user files (such as `.env`) as dirty while excluding only
  exact-matching `managedHookFiles` and the spec's narrow disposable cache
  directory allowlist (`node_modules/`, `target/`, etc.).
- [ ] Wire cleanup into manual Archive and staged auto-archive (from Plan 1):
  clean -> `git worktree remove` (preserving branch/commits) +
  `cleanupStatus = "removed"`; dirty -> complete Archive, keep worktree, set
  `retained_dirty` + `cleanupNote`; OS/Git error -> `removal_failed`.
- [ ] Wire cleanup into Manual Delete (covering `active`, `retained_dirty`, and
  `removal_failed` uniformly after Close-first): clean -> remove worktree then
  delete session; dirty or removal failed -> complete session deletion, leave
  worktree untouched on disk, and return a structured payload
  `{ preservedWorktreePath, reason }` (updating the Delete response contract).
- [ ] Add Electron-main-authorized **Remove worktree** retry endpoint
  (`x-orkworks-open-plan-token` + `sessionActionToken`) for flagged sessions,
  and project `managedWorktree` (`root`, `branch`, `baseRef`, `cleanupStatus`,
  `cleanupNote`, `hookNote`) onto `SessionInfo`.
- [ ] Verify dirty/failed managed worktrees remain exempt from Stage 2
  auto-deletion and `workspace_gc` until cleaned; run all targeted Rust tests.

## Delivery 4: desktop composer UX, Details recovery controls, and Delete notices

Files: `apps/desktop/electron/preload.ts`; `apps/desktop/electron/main.ts`;
`apps/desktop/src/orkworksWindow.d.ts`; `apps/desktop/src/harnessTypes.ts`;
`apps/desktop/src/api.ts`; `apps/desktop/src/workspaceSessionController.ts`;
`apps/desktop/src/components/NewSessionDialog.tsx`;
`apps/desktop/src/components/SessionListPanel.tsx`;
`apps/desktop/src/components/SessionDetailPanel.tsx`;
`apps/desktop/src/components/SessionContextMenu.tsx`;
`apps/desktop/src/App.tsx`; `apps/desktop/src/App.css`; `docs/user/sessions.md`.

- [ ] Add preload/IPC bridges in Electron main for desktop-authorized
  managed-worktree session creation and explicit **Remove worktree** retry
  (reusing `runtime.runReady` generation checks).
- [ ] Update `NewSessionDialog.tsx` to fetch workspace Git branch eligibility,
  show the non-sticky **Current workspace / New worktree** toggle and local base
  branch picker when eligible, and preserve draft prompt/images on launch error.
- [ ] Update `SessionListPanel.tsx` and `SessionDetailPanel.tsx` to display
  **Dirty worktree** / **Worktree cleanup failed** flags, `hookNote` degradation
  warnings, and Details recovery controls (**Open in VS Code**, **Changes**,
  **Restore to Closed**, **Remove worktree**) targeting `managedWorktree.root`.
- [ ] Update Delete confirmation to warn when a non-removed managed worktree
  exists (naming its path and stating uncommitted edits will stay on disk), and
  render a visible post-delete notice/toast whenever Delete returns
  `preservedWorktreePath`.
- [ ] Add desktop unit/component tests and an Electron workflow fixture covering
  composer visibility/reset/error preservation, Details dirty-worktree flag &
  retry, and Delete confirmation + post-delete preserved-path notice; update
  `docs/user/sessions.md`.

## Delivery 5: verify, review, and land

- [ ] Run `bash scripts/verify-repo.sh`, `bash scripts/doc-check.sh`, and
  `bash .claude/hooks/worktree-check.sh`; inspect any warnings against the diff.
- [ ] Complete diff-scoped `/code-review medium` (justified by Git/filesystem
  mutation, Electron authorization boundary, and hook reconciliation changes)
  and address all findings before merge.
- [ ] Push branch, open PR, complete `babysitting-pull-requests` checks/comments
  before authorized merge, and clean up the owned worktree via
  `bash scripts/finish-pr.sh <PR>`.

## Planning review

Disposition: **Ready** (for owner review; execution stays gated on landing
Plan 1's `OrganizationStore` v2 foundation, creating the tracking GitHub issue,
and recording Delivery 1's ADR 0008 amendment and MVP spec update).

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 3 | Builds cleanly on Plan 1's `OrganizationStore` v2 and existing harness installers / `session_location.rs`; no new external crates or packages |
| Blast radius | 4 | Introduces opt-in Git worktree creation/removal and hooks into composer, archive, delete, resume, and `starting-work`; strictly gated to opted-in managed sessions |
| State changes | 3 | Additive `managedWorktree` object in `OrganizationStore` v2 and filesystem directories confined under `~/.orkworks/worktrees/<hash>/`; primary checkout never mutated |
| Reversibility | 3 | Feature is opt-in and non-destructive (preserves branches/commits and dirty trees); rollback leaves existing managed worktrees usable as ordinary linked Git worktrees |
| Uncertainty | 3 | Windows path/file-lock semantics and per-harness tracked hook files require temp-Git-repo tests across platforms; fail-closed degradation and `removal_failed` retry isolate failures |

Total: 16. Key failure treatments: never write unignored hook files or dirty
tracked shared hook files; strict canonical containment under
`~/.orkworks/worktrees/<hash>/`; clean-only removal (no `--force`); dirty
worktrees preserved and flagged on Archive and preserved with explicit path
notice on Delete; async spawn failure rolls back unused worktree and branch.
