---
type: Implementation Plan
title: Selected-session Git changes
description: Deliver distinct uncommitted and committed branch inspection in the existing right panel after design approval.
tags: [orkworks, desktop, git, implementation]
status: draft
workflow_status: awaiting-owner-approval
---

# Selected-session Git changes implementation plan

Outcome: **Review changes** opens a read-only inspector for the selected
session's worktree, with clearly distinct Uncommitted changes and Branch changes.
Pass condition: real panel navigation displays the correct fixture diff for
each comparison, rejects stale or unauthorized content, and preserves the PTY.

Tracking: [#96](https://github.com/Rambolarsen/orkworks/issues/96).
Design: [selected-session Git changes](../../../specs/session-git-changes.md).
This is preparation only; implementation awaits approval of both documents.
No dependency on delivering Markdown #18 or app preview #19; preserve their
shared single-inspector direction. Execute inline in the owning worktree.

## 1. Bounded Git inspection and authenticated read contract

Owners: `crates/orkworksd/src/git.rs` retains Git context; new
`git_changes.rs` owns bounded captures/ref enumeration and opaque file results;
new `http/git_changes_handlers.rs` owns thin authenticated routes. Wire router
and bounded, generation-scoped snapshot state in `main.rs`. Reuse the current
session application's effective-CWD authority; do not add a second precedence.

Produce the three design routes, exact subject/baseline/capture identity,
typed failures and opaque file lookup. Preserve existing session-list totals and
metadata. No persisted migration. Extend existing plan-content desktop credential
validation through a shared internal helper without broadening unauthenticated
routes. Use git2 only, including safe bounded worktree access; inspect its actual
no-follow behavior before relying on it. Record the accepted new read capability
in an ADR before implementation; reconcile runtime/security references with the
approved design without changing central document Review.

- [ ] Write failing real temporary-repository tests: mixed staged/unstaged/new
  changes; a branch commit plus dirty edits appears only in its intended view;
  diverged base uses the merge base rather than base tip; base default never
  selects feature upstream. Cover rename/delete/binary/conflict/mode/symlink,
  unborn/detached HEAD, missing/shallow/unrelated/multiple-base histories and
  linked-worktree isolation. Preserve the existing totals tests.
- [ ] Write failing router/reader tests for missing/wrong credentials, unknown
  session, changed effective root/workspace, expired/foreign snapshot/file,
  traversal and ancestor/leaf symlink races, administrative paths, special
  files, oversized text/lines, candidate/file/output limits and busy admission.
  Confirm aborted/partial captures never report clean or complete totals.
- [ ] Implement capture, bounds and snapshots until these tests pass. Verify
  HEAD/base revalidation and the non-atomic capture notice. No generic read API.

## 2. Desktop transport and reachable Changes inspector

Owners: `apps/desktop/electron/main.ts`, `preload.ts` and corresponding per-side
contract declarations add narrow Changes methods through the existing trusted
transport. `src/api.ts` wraps them. New `src/components/GitChangesPanel.tsx`
owns selection, request generations, file list, unified patch and notices.
`ApplicationShell.tsx`, `shellNavigation.ts`, `SessionDetailPanel.tsx`, labels
and CSS integrate the alternative inspector and always-reachable Git-facts entry.
Keep transport work on its own side of the Electron/renderer boundary.

- [ ] Add failing transport tests proving credentials stay in main, validation
  rejects arbitrary paths/revisions, and backend lifecycle changes invalidate
  responses. Pin corresponding wire types using the existing test convention.
- [ ] Add failing rendered interaction tests: Review changes reachable for idle,
  live, historical and clean repositories; opening retains the selected terminal;
  both labeled comparisons show their correct baselines and categories. Verify
  empty/unavailable/partial/binary/errors and base selection without a fetch.
- [ ] Add controllable-promise tests for session/workspace/comparison/base/file
  switches and out-of-order successes/errors; old results never paint beneath a
  new identity. Verify explicit Refresh, preserved-file selection, failure labels,
  close/reopen resets and Back to Details focus.
- [ ] Implement transport and panel to pass these tests, replacing the old diff
  placeholder with this reachable action. Leave Mark handled semantics to #439.

## 3. Integration, documentation and delivery

- [ ] Exercise the real Electron host at wide overlay/pinned widths, below 1180px,
  below 860px and 200% zoom: keyboard-only entry/select/refresh/back/close,
  accessible non-color labels, long paths and horizontally scrollable patches.
  Confirm one inspector and correct focus restoration when an invoker disappears.
- [ ] Use a live output-producing PTY fixture to verify opening, pinning, changing
  comparisons, responsive page transitions and closing preserve runtime identity,
  output/replay and input behavior; navigation keystrokes do not reach the PTY.
- [ ] Update current architecture/product/API references and user docs only after
  implementation; retain source versus published-installer distinction. Run the
  full Rust build/test/fmt, desktop suite and type check, `bash scripts/verify-repo.sh`,
  docs validation/build, `bash scripts/doc-check.sh` and worktree check.
- [ ] Complete current-diff `/code-review low` (escalate under repository size/risk
  rules if needed), open/update the feature PR, pass required checks and follow
  the bounded PR workflow. Keep #96 open until delivered and verified.

## Plan review and risk disposition

Self-review: **Ready for owner design/plan review; runtime work remains gated**.
Each #96 acceptance item maps to design sections and tasks 1–3. No unresolved
product choice is delegated to implementation; unsafe/unbounded filesystem access
is an explicit blocker. Reuse ADR 0009/0078/0083 geometry and security constraints.

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 3 | Rust, Electron transport and shell; coupled delivery, no #18/#19 runtime prerequisite |
| Blast radius | 3 | New authenticated read surface and inspector navigation; existing PTY invariant tests |
| State changes | 1 | Additive ephemeral snapshots, no persisted schema migration |
| Reversibility | 1 | Revert routes/bridge/panel together; no user repository mutation |
| Uncertainty | 3 | git2 safe bounded capture needs behavioral proof before exposure |

Total: 11/25; judgment aid only. The security design and scoped plan require
owner approval under #96. After approval, author the scoped ADR and execute this
plan; fail closed if filesystem containment or resource bounds cannot be proven.

## Proposal verification

The one-pass read-only builder check found no important unanswered questions.
Documentation metadata/index validation, all 20 OKF validator tests and the
VitePress build passed. Existing build warnings concern circular framework/theme
chunks and bundle size; this diff changes no build configuration. The doc-drift
hook recommends reviewing README because specs changed: no README edit is
needed because this is explicitly proposed behavior, with no availability or
installer claim to update. Runtime tests have not run; no runtime code changed.
