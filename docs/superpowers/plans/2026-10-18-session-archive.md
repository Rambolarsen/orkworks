---
type: Implementation Plan
title: Session archive and context-menu implementation
description: Deliver the approved three-state session model with safe close-first archive and delete actions.
tags: [orkworks, sessions, archive, implementation]
status: draft
---

# Session archive and context-menu implementation

Issue: [#439](https://github.com/Rambolarsen/orkworks/issues/439).
Design: [Active, Closed and Archived sessions](../specs/2026-10-18-session-archive.md).
State: reviewed draft, awaiting owner design approval; runtime work is gated.

Pass condition: the real app supports Active/Closed/Archived, Archive/Restore and
row context-menu commands, and no history mutation runs before a requested
Active Close completes. Preserve existing resume, history, attention and
workspace/runtime ownership contracts.

## Boundaries and investigated evidence

Use the existing Rust sidecar, MetadataStore, SessionApplication and renderer
Sessions surface. No new dependency, coding-tool archive mutation, state-machine
rewrite, automatic session selection or result acceptance.

Current close (`SessionApplication::delete_session`) requests kill and begins
ending; `complete_session_ending` finalizes metadata and the live mirror later.
Current forget/retention test only selected status strings and must reject all
unresolved current runtime states. The metadata API has sidecar session-write
lanes but external snapshot writers remain uncoordinated; organization state
therefore uses the proposed separate store. SessionListPanel currently has Kill
and remembered-session Delete buttons inside its list. Current grouping is
separate in sessionGroups. Reuse the real Electron shell test harness, stating
fixture boundaries. TDD applies to production behavior changes.

## Delivery 1: state ownership and lifecycle-safe mutations

Files: new crates/orkworksd/src/session_archive.rs (organization storage and pure
state rules); metadata.rs ownership/construction; session_application.rs command
orchestration; session_projection.rs and session_types.rs projections;
runtime/session_runtime.rs and terminal_runtime.rs only for the minimal
finalization completion signal; runtime/retention.rs; http/session_handlers.rs
and main.rs routes. Keep detailed lifecycle and cleanup owners intact.

- [ ] Record the next available ADR and synchronize MVP, domain entities and
  metadata/retention architecture descriptions with the approved design before
  runtime code. Index the ADR and changed documentation correctly.
- [ ] Write failing storage/state tests for legacy missing state, identity-bound
  archive/restore, malformed/read failure and atomic publication failures. Implement
  bounded sidecar-owned records and additive sessionState/actionToken projection.
- [ ] Write deterministic failing close-first tests using controlled finalizer
  completion. Implement per-session operation fencing and a bounded wait for the
  captured runtime's completed cleanup/finalization, with locks released while
  awaiting. No stale token can stop or mutate a replacement runtime.
- [ ] Add state-command handler tests, archived resume rejection and direct-forget
  eligibility tests. Preserve compatibility meanings of legacy close/forget routes;
  update their shared guards rather than relying solely on UI sequencing.
- [ ] Add retention/delete failure/race tests. Exclude archived/unknown records
  from automatic deletion/count budget, preserve last activity, and remove orphan
  organization data only through identity-safe deletion without resurrection.
- [ ] Verify targeted backend/storage/runtime tests and normal resume/close/retention
  coverage. Pin the no-prompt/no-native-tool-archive/no-Git-effects boundary.

Interface: the list exposes sessionState and an opaque stale-action token plus
bounded state-read diagnostic if unavailable. State commands carry intent and
the token; authority, current workspace/runtime state and cleanup are validated
server-side. Response returns current projected state or a specific conflict/error.
The existing direct forget route never closes an Active session implicitly.

## Delivery 2: state-aware Sessions and accessible row actions

Files: src/api.ts; workspaceSessionController.ts; App.tsx; ApplicationShell.tsx;
SessionListPanel.tsx; a focused SessionContextMenu component and pure action/state
helpers as justified; App.css and existing token styles. Electron files only
if required by current transport contracts; preserve the import boundary.

- [ ] Add failing controller/API tests for command/token transport, pending state,
  stale workspace responses and refresh after success. Implement targeted handlers
  without optimistic state changes or fallback to destructive legacy commands.
- [ ] Add renderer state-filter tests retaining current grouping and exact selection.
  Provide All unarchived/Active/Closed/Archived in the existing Sessions header;
  changing filters never acknowledges/selects a session.
- [ ] Add row-action tests for the three states. Implement right-click, visible
  More actions and keyboard invocation with exact row identity, menu focus/return,
  outside/Escape dismissal and conflict disabling. Resolve listbox/interactive-row
  structure accessibly; do not add a generic menu framework.
- [ ] Require destructive confirmation before Active close/delete; show Archive
  close-first semantics, progress and useful failure states. Wire Archived Restore
  to Closed without resume. Preserve selected history across Archive/Restore and
  clear only a successfully deleted selection.
- [ ] Add a real Electron workflow fixture exercising state transitions and menu
  behavior, including finalization delay/error, keyboard actions, narrow/zoom and
  archived history. Distinguish controlled runtime fixtures from live tool evidence.
- [ ] Update session/archive/retention user docs with native-tool and downgrade
  limitations. Mark handled remains unimplemented/out of this feature.

## Delivery 3: verify, review and land

- [ ] Run focused checks after each changed behavior. Run bash scripts/verify-repo.sh
  after implementation; inspect all warnings against the diff. Run doc-check and
  worktree-check at close-out. Test manual native menu/focus behavior where fixtures
  do not establish it; record exact build and results.
- [ ] Complete the applicable diff-scoped /code-review effort for current code;
  use medium if the final diff retains concurrency/schema/runtime changes. Tie off
  an active matching Taskmaster recommendation through the authenticated API if any.
- [ ] Commit/push the owned branch, open/update the PR, run bounded PR monitoring
  and satisfy required checks/review before any authorized maintainer merge. Use
  scripts/finish-pr.sh on the merged PR for owned-worktree cleanup.

## Planning review

Disposition: **Ready for owner design review**, not ready for runtime execution
until that approval. No subagent work or separate coding harness is required.

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 3 | Known runtime/store/UI owners; producer JSON writes remain external, so separate owned state |
| Blast radius | 4 | Session list, resume, forget and retention; shared guards plus targeted regression checks |
| State changes | 3 | Additive identity-bound organization records; legacy defaults and explicit restore |
| Reversibility | 3 | Feature can stop while retaining records; old binaries cannot honor archive retention |
| Uncertainty | 3 | Close/finalizer ordering inspected; proposed completion/fencing changes require deterministic tests |

Total: 16. This is a review aid, not permission. Key failure treatment: request
acceptance is not cleanup completion; timeout preserves data; archive/retention
serialize; stale workspace/runtime tokens cannot mutate replacements. Required
manual approval covers the concrete state/retention/protocol design once.
