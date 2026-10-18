---
type: Implementation Plan
title: Session archive and context-menu implementation
description: Deliver the approved three-state session model with safe close-first archive and delete actions.
tags: [orkworks, sessions, archive, implementation]
status: stable
workflow_status: in_progress
---

# Session archive and context-menu implementation

Issue: [#439](https://github.com/Rambolarsen/orkworks/issues/439).
Design: [Active, Closed and Archived sessions](../specs/2026-10-18-session-archive.md).
State: owner approved on 2026-10-18; implementation in progress.

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

- [x] Record the next available ADR and synchronize MVP, domain entities and
  metadata/retention architecture descriptions with the approved design before
  runtime code. Index the ADR and changed documentation correctly.
- [x] Write failing storage/state tests for legacy missing state, identity-bound
  archive/restore, malformed/read failure and atomic publication failures. Implement
  bounded sidecar-owned records and additive sessionState/actionToken projection.
- [x] Write deterministic failing close-first tests using controlled finalizer
  completion. Implement per-session operation fencing and a bounded wait for the
  captured runtime's completed cleanup/finalization, with locks released while
  awaiting. No stale token can stop or mutate a replacement runtime.
- [x] Add state-command application/API tests, archived resume rejection and direct-forget
  eligibility tests. Preserve compatibility meanings of legacy close/forget routes;
  update their shared guards rather than relying solely on UI sequencing.
- [x] Add retention/delete failure/race tests. Exclude archived/unknown records
  from automatic deletion/count budget, preserve last activity, and remove orphan
  organization data only through identity-safe deletion without resurrection.
- [x] Verify targeted backend/storage/runtime tests and normal resume/close/retention
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

- [x] Add failing controller/API tests for command/token transport, pending state,
  stale workspace responses and refresh after success. Implement targeted handlers
  without optimistic state changes or fallback to destructive legacy commands.
- [x] Add renderer state-filter tests retaining current grouping and exact selection.
  Provide All unarchived/Active/Closed/Archived in the existing Sessions header;
  changing filters never acknowledges/selects a session.
- [x] Add row-action tests for the three states. Implement right-click, visible
  More actions and keyboard invocation with exact row identity, menu focus/return,
  outside/Escape dismissal and conflict disabling. Resolve listbox/interactive-row
  structure accessibly; do not add a generic menu framework.
- [x] Require Delete confirmation before any Active close/delete; show Archive
  close-first semantics, progress and useful failure states. Wire Archived Restore
  to Closed without resume. Preserve selected history across Archive/Restore and
  clear only a successfully deleted selection.
- [x] Add a real Electron workflow fixture exercising state transitions and menu
  behavior, including finalization delay/error, keyboard actions, narrow/zoom and
  archived history. Distinguish controlled runtime fixtures from live tool evidence.
- [x] Update session/archive/retention user docs with native-tool and downgrade
  limitations. Mark handled remains unimplemented/out of this feature.

## Delivery 3: verify, review and land

- [x] Run focused checks after each changed behavior. Run bash scripts/verify-repo.sh
  after implementation; inspect all warnings against the diff. Run doc-check and
  worktree-check at close-out. Test manual native menu/focus behavior where fixtures
  do not establish it; record exact build and results.
- [x] Complete the applicable diff-scoped /code-review effort for current code;
  use medium if the final diff retains concurrency/schema/runtime changes. Tie off
  an active matching Taskmaster recommendation through the authenticated API if any.
- [ ] Commit/push the owned branch, open/update the PR, run bounded PR monitoring
  and satisfy required checks/review before any authorized maintainer merge. Use
  scripts/finish-pr.sh on the merged PR for owned-worktree cleanup.

## Planning review

Disposition: **Approved for implementation** by the owner on 2026-10-18. No subagent work or separate coding harness is required.

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

## Implementation evidence — 2026-10-18

The owner approved the concrete design. ADR 0088 records it; 0087 was taken by
concurrently merged work. The implementation reuses persisted `ended` plus the
runtime command-closure/attachment/PID guards, checking the captured generation
in a bounded 25 ms state-based wait. This avoids adding a parallel lifecycle
completion signal. No application lock survives an await.

The initial projection and retention tests were observed failing, then passing.
Backend fixtures cover close-first finalizer delay, timeout/cancellation,
stale workspace/runtime tokens, archived resume admission, count/age retention,
and partial deletion failure after metadata removal. Renderer/controller/API
checks cover captured token transport, conflict diagnostics, stale workspace
settlement and selection preservation. The real Electron fixture uses the real
SessionListPanel/menu with controlled session data and a delayed action callback;
it does not run a live coding-tool process. It exercises filters, exact-row
pointer/keyboard actions, cancellation, successful Delete, focus return, disabled
admission and 150% zoom. Visual capture was inspected at narrow width.

Diff-scoped medium reviews used separate read-only backend and UI seats. UI
focus/removal and availability findings were fixed and re-reviewed; backend
partial-deletion cleanup was additionally tested and re-reviewed. Both reviews
approve the current code. Full repository verification and PR checks remain
required before landing.

Repository verification passed Rust formatting/build, 2,098 unit tests (5 ignored),
6 integration tests, 9 OpenCode reporter tests, desktop type-check, native ABI
repair, 1,350 desktop tests (1 skipped), and the desktop build. Its docs stage
caught an OKF status vocabulary error; that metadata was corrected and the full
docs build, diff check, doc-check and worktree-check passed separately. Existing
VitePress circular/chunk-size warnings are unrelated to this diff. The authenticated
Taskmaster API returned no active recommendations, so no tie-off was needed.

Final integration verification: merged current `origin/main` (7e6a9264), resolving
only two documentation index conflicts. The archive code diff remained unchanged
(SHA-256 `6e19bbceffd2c0149f9ec9d5ccf74949428dde28fe5b0b82186b0b8dab8e1b70`).
A clean full `bash scripts/verify-repo.sh` run under repository-pinned Node
22.23.3 passed every stage, including 2,139 Rust unit tests (9 ignored), the
integration suites, desktop tests/build, docs build, diff, docs and worktree checks.
The host shell's Node 26 was not used for that final run.

## PR feedback cycle 1/3

Native Codex review on `f637566a` identified four concrete gaps despite passing
CI: directory-entry durability after rename; selected-row cleanup after a
partial Delete; keyboard menu targeting when selection is filtered out; and
plain-text sidecar error parsing. Each was reproduced by a focused failing
regression, then corrected. Archive publication now syncs the metadata parent
and session-state directory on Unix, propagating sync failures. Failed Delete
reconciles a row proven absent by a successful refresh while preserving the
cleanup error and any different current selection. Keyboard invocation requires
a visible selected row. State-action transport preserves text or JSON errors.
Fresh focused reviews and full Node 22 verification precede the next push.

Focused review additionally required the real in-flight selection race test and
Windows publication durability. The regression now changes selection while a
Delete request is pending. Windows publication uses `MoveFileExW` with
`MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH`, after closing and syncing
the staged file; Unix syncs the containing directories. The existing Windows
Rust job also runs the archive storage suite to exercise native create/replace
and restore behavior. See the [Microsoft API contract](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw).

Cycle 1 verification: full Node 22 repository script passed (2,140 Rust tests,
9 ignored; 1,357 desktop tests, 1 skipped; integration/build/docs/check stages).
The final platform-aware publication helper then passed the focused storage and
state-action suites; the added deferred selection race passed under Node 22.
Both focused manual review seats approve the final cycle changes. Windows native
publication execution is additionally gated by the new existing-job CI step.

## PR feedback cycle 2/3

Native Codex on `a3828bf6` identified filesystem reads while application state
locks were held, fresh workspace guidance on archived history, and missing
organization state/action tokens in successful create/resume responses.
Focused regressions reproduced all three, then passed after correction.
Organization projection now snapshots the retained store and runtime authority
under locks, releases those locks before storage reads, and is shared with
create/resume response construction. Creation uses one identity timestamp for
the returned info, runtime handle and durable metadata. Archived projections
keep read-only Git context but suppress current recommendations/conflict warnings.

Cycle 2 full verification passed under Node 22.23.3: 2,142 Rust unit tests
(9 ignored), 6 integration tests, 1,357 desktop tests (1 skipped), reporter
tests, type-check, ABI checks, desktop/docs builds and repository checks.
Fresh focused manual medium review approves this code; its optional lock
comment clarification was applied. The existing user docs already describe
source archive behavior and release availability; no onboarding claim changed.

## PR feedback cycle 3/3

Native Codex on `c62f51dc` identified producer lifecycle regression overriding
archive projection, a missing creation-identity recheck in retention, and late
create/resume responses overwriting completed row actions. Focused regressions
reproduced all three. Organization derivation now separates actual runtime
authority from producer lifecycle evidence: a matching archive remains Archived
without a live admitted runtime, while real runtime authority still wins.
Retention retains the captured candidate identity across age/count selection and
compares the final metadata reread before deletion. Row-action admission fences
older foreground responses without changing its workspace/admission guards.
Four deferred frontend tests cover create/resume against both Archive and Delete.
This is the final permitted substantial review cycle; further substantial
feedback requires an explicit finite owner extension before another review.

The focused backend review also tested recovery after a regressed snapshot.
Archived remains visible, but Restore/Delete/legacy forget preserve the existing
current persisted-finalization gate and fail without mutating state/evidence
until ended/dead is restored; recovery is covered by explicit action tests.
A stored Closed choice can predate a resumed run and is not substituted for
current finalization proof. No producer-owned JSON is rewritten by organization.

Cycle 3 full Node 22 verification passed: 2,144 Rust unit tests (9 ignored),
6 integrations, 1,361 desktop tests (1 skipped), reporter tests, native ABI,
type-check, builds/docs and repository checks. The two additional conservative
recovery regressions then passed, as did the final storage suite. Existing
user documentation already covers the unchanged close-first/failure behavior.

## Owner-approved extension: PR feedback cycle 4/4

The owner approved one additional substantial review cycle after the three-cycle
cap. The owner also approved manual review plus native Codex as the alternative
to unavailable Copilot; no further Copilot request is required for this path.

The final cycle-3 review found eight actionable gaps. Red/green regressions now
cover unreadable metadata blocking legacy forget, final-guard deadline expiry,
protected metadata-only rows not consuming count-deletion slots, archived stale
rows not affecting live Git guidance or current Actions/attention, same-path
generation resets, and Delete/Restore consequence copy in real Electron.
The implementation retains current-finalization and identity guards. Both fresh
focused manual medium review seats approve these changes. Further substantial
automated review beyond cycle 4 requires a new finite owner authorization.

Cycle 4 verification passed the full repository script under Node 22.23.3
with RUST_TEST_THREADS=4: 2,149 Rust unit tests (9 ignored), 6 integrations,
1,367 desktop tests (1 skipped), reporter/native ABI/type-check, builds/docs
and repository checks. The initial run exposed two legacy expectations for
deleting unreadable metadata; those tests now require a conflict and retained
history. A subsequent unconstrained parallel run hit three unrelated native
Codex fixture-probe failures; each passed in isolation, and the complete run
with four workers passed without modifying those tests or production code.
Fresh focused backend/UI review approves the final code and amended regressions.

## Owner-approved extension: PR feedback cycle 5/5

The owner authorized the two cycle-4 findings and one additional cycle.
A shared archive-aware historical/alive predicate now governs terminal replay,
attachment pruning, workspace selection restoration, independent-review and
Fix with AI entry points, Details live controls, grouping and unread/attention.
Archived rows keep read-only plan/history access even if producer metadata
claims alive. Actual App Electron and TerminalPanel regressions reproduce the
live-attachment bug and verify historical rendering and disabled live controls.
Post-action refresh reconciles any confirmed-absent target, including retention
following Restore, without disturbing a newer selection or inferring absence
from an unavailable refresh. Deferred success/error/selection regressions were
observed failing then passing. The already-approved manual-plus-Codex path
continues; no sixth cycle is assumed.

Cycle 5 full verification passed under Node 22.23.3 with four Rust workers:
2,149 Rust unit tests (9 ignored), 6 integrations, 1,375 desktop tests
(1 skipped), reporter/native ABI/type-check, desktop/docs builds and repository
checks. Existing source-contract assertions now verify the shared predicate;
Electron generation tests wait for observable reset rather than a single frame.
Fresh focused backend/UI medium reviews approve the code. The documentation
currency warning is covered by the existing archive user-guide/home updates.
