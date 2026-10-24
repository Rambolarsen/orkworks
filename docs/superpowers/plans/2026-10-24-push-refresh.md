---
type: Implementation Plan
title: Push session and recommendation refresh
description: Deliver changed-row synchronization and shared recommendation refresh with authoritative recovery.
tags: [orkworks, desktop, sidecar, synchronization]
status: draft
workflow_status: executing
---

# Push refresh implementation plan

Goal: eliminate recurring full session downloads on a healthy stream and duplicate
recommendation list polls, preserving authoritative recovery and session behavior.

Authority: [approved design](../specs/2026-10-24-push-refresh-design.md),
[MVP](../../../specs/orkworks-mvp.md), [Taskmaster](../../../specs/taskmaster.md),
[issue #22](https://github.com/Rambolarsen/orkworks/issues/22).
Implementation is inline in the owned `push-refresh-design` worktree.

## Contracts and producer inventory

One workspace controller owns the v1 socket. Snapshot, session delta and
recommendation invalidation share a global cursor and workspace/instance identity.
Complete rows preserve the entire REST pipeline, including live Peon diagnostics.
No deletion from an incomplete query. Snapshot on reconnect; current REST polling
while disconnected. Two-second projection throttle, 30-second reconciliation,
10-second heartbeat, 30-second heartbeat timeout, reconnect 1–30 seconds with
jitter. Bounded socket send timeout disconnects slow consumers.

| Producer | Invalidation boundary |
| --- | --- |
| Create/resume/stop/archive/restore/delete/settings/providers/harness mutations | Router mutation middleware after response; filesystem publication catches background completion |
| PTY output/exit and terminal input | Runtime publication and exit boundary mark sessions dirty |
| Hook and Peon metadata, external metadata helpers | Nonrecursive active workspace metadata watcher catches atomic publication/deletion |
| Peon diagnostics and observation count | Diagnostic publication boundary plus observation filesystem events |
| Taskmaster evaluation/recommendation/assessment/run status | Recommendation directory and global Taskmaster durable-state watcher |
| Retention cleanup | Session/organization/recommendation deletion events |
| Clock/cwd/Git/native saved-state/missed filesystem events | Thirty-second reconciliation; focus and explicit actions request immediate refresh |
| Workspace close/replacement | Identity fence and stream closure; controller lifecycle cancels old work |

Watch only metadata directories and their parent for directory recreation; never
terminal logs. Watcher errors invalidate all domains and rearm on reconciliation.
Do not publish read/access-only filesystem events. Existing complete metadata
scanner already fails on listed unreadable/disappeared records; retain it.

## Tasks

### Task 1: Sidecar stream and authoritative projection

Own `workspace_updates.rs`, `main.rs`, `http/session_handlers.rs`, runtime
publication points and Cargo dependency. Extract `query_sessions` for REST/stream.
Add a coalesced dirty watch signal to AppState, a workspace-bound socket endpoint,
serialized row diff and ordering, heartbeat, watcher, throttle and recovery.
Audit origin behavior: existing CORS is permissive; new socket rejects non-loopback
browser origins, supports packaged opaque origin and nonbrowser clients, and does
not accept reporting tokens. No change to existing routes' authorization.

- [x] RED: test changed rows/removals/order, cursor progression, no-op diff,
  incomplete query, wrong origin/workspace and lifecycle replacement.
- [x] Implement server and producer boundaries. A query failure sends unavailable
  without advancing the row baseline; complete recovery sends a snapshot.
- [x] GREEN: targeted Rust unit and real socket tests, formatting/build.

### Task 2: Renderer stream recovery and controller adoption

Own `workspaceUpdates.ts`, controller and tests. Validate messages before atomic
application to a private stream baseline. Controller uses the existing merge,
pending-create and prune path for complete lists reconstructed from deltas.
REST results started before socket adoption must be rejected. Explicit foreground
refresh invalidates the socket baseline so subsequent deltas cannot undo it.

- [x] RED: cursor/identity gaps, malformed rows, trailing changes, REST/adoption
  race, old workspace, reconnection/heartbeat, disposal and pending creation.
- [x] Implement injected socket/scheduler transport and default browser adapter;
  healthy stream suspends polling, disconnect restores it, unsupported retains it.
- [x] GREEN: protocol and workspace controller tests plus desktop type check.

### Task 3: Shared recommendation source and UI integration

Own `recommendationSource.ts`, `useActionsRecommendations.ts`, shell/panel wiring
and component tests. One workspace-scoped list refresh supplies both consumers.
Stream invalidations refresh the list plus mounted status/details. Failed list
refresh preserves scope-correct rows and reports unavailable. Local mutations
invalidate immediately. No overlapping list refresh; changes during requests
schedule one trailing refresh. Fallback uses one five-second timer.

- [x] RED: two subscribers make one request; invalidation during fetch trails;
  stale workspace/error/disposal; healthy stream removes polling; fallback resumes.
- [x] Implement shared source and migrate both consumers without changing action
  approval or focused/historical recommendation detail behavior.
- [x] GREEN: source/controller/component tests and type check.

### Task 4: End-to-end verification and PR

- [x] Update architecture/specs/ADR and issue acceptance criteria; document evidence.
- [x] Install frozen desktop dependencies with host Node ABI; run
  `bash scripts/verify-repo.sh`, documentation and worktree close-out checks.
- [x] Real-socket integration proves snapshot plus changed row rather than full
  retransmission; inspect measured bytes and report active projection cost limit.
- [ ] Complete required `/code-review low` on current code diff; open and follow
  PR checks/reviews within the repository budget, merge only with gates satisfied.

## Review and risks

Review disposition: Ready. All design requirements map to tasks above. No storage
migration; reverting renderer wiring restores existing behavior. Dependencies 2
(one release, coupled server/client), blast radius 4 (core lists), state changes 2
(additive transient protocol), reversibility 1 (polling fallback), uncertainty 3
(watcher/platform and producer coverage): 12/25. These are review aids, not gates.

Review focus: reconnect gaps cannot imply deletion; partial metadata cannot remove
history; foreground creation cannot be overwritten by an older stream; multiple
recommendation consumers cannot double polling; old workspace data cannot cross
replacement. Each has an explicit test owner above. Watcher qualification and
origin tests are required before completion, not assumed from protocol unit tests.

## Verification and review evidence

`bash scripts/verify-repo.sh` passed on 2026-10-24 under Node 22: 2,227 Rust
unit tests and all seven integration tests passed (10 intentionally ignored);
1,431 desktop tests passed (two intentionally skipped); reporter tests, native
ABI repair, type checks, desktop/docs builds and close-out checks passed. Seven
workspace socket/protocol tests exercise native updates, removals, origin and
workspace fences, corruption, and directory disappearance/recreation. The
32-record socket fixture verifies that one changed row sends less than a quarter
of the snapshot bytes; production active-projection CPU is not claimed improved.
Windows CI now runs the same socket/watcher tests with required discovery.

Independent `/code-review medium` found one P2: a missing established session
directory could look empty. Its regression failed before the shared directory
witness fix and passed afterward. Follow-up review approved the current code
with no remaining P1/P2 findings. Reviewed app/sidecar diff SHA-256:
`d5f4079fb2ab970add7abf200008aa9b4c06451a6849d4a151106a1be848a7de`.

Documentation-currency warnings were reviewed: no public onboarding or installer
claim changed, existing README HTTP/WebSocket architecture remains accurate, and
the new dependency/ADR adds no universal agent rule. Detailed contracts live in
the architecture concept and ADR. Existing bundle-size/circular-chunk and Rust
dead-code warnings do not require unrelated edits. The pre-commit worktree hook
called the new branch merged because its HEAD was still the fork point; staged
changes were retained and require the owned PR before cleanup.

## PR feedback cycle 1 of 3

PR #1034's automated Codex review found a P2 startup-restoration race: the first
stream snapshot fenced initial REST before its restoration continuation ran.
Four regressions (open/adopt and intervening navigation) reproduced two failures;
restoration now completes once from the first accepted authoritative list, with
foreground/explicit-selection fencing. Two behavioral REST tests also verify
list-before-active publication, replacing an implementation-specific regex.

Windows CI exposed the socket fixture's assumption that every next frame is a
session delta. The fixture now validates and advances a shared cursor through
recommendation invalidations and leaves heartbeat/unavailable cursors unchanged.
Its explicit deletion uses the projection lock; corruption still tests the
incomplete-query recovery path. The navigation fixture now rejects corrupt or
unsupported saved-state writes until rebuild, matching the real persistence
contract. Delayed and pending rebuild Electron regressions passed.

Fresh medium review approved these changes with no remaining material findings.
Reviewed current app/sidecar diff SHA-256:
`a6e7836407b858da6caf8da66ac60353cfbeb26be949027bda7b418d7e58218b`.
The verifier's Rust suites, reporter tests, type check and native ABI repair
passed on this production code; after the fixture corrections, the full desktop
rerun passed 1,436 tests with two intentional skips. Seven native socket tests
and 98 controller/shell tests passed separately. Hosted Windows revalidation
remains a merge gate for this revision.
