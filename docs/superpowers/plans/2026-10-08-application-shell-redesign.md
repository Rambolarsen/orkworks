# Application Shell Redesign Implementation Plan

> **Before continuing:** Use [reviewing-plans](../../../skills/reviewing-plans/SKILL.md), recheck the delivery status below, and follow the repository execution workflow. Completed tasks are delivery history, not instructions to recreate their files.

**Goal:** Replace Dockview's user-arranged panel workspace with the approved fixed, resizable OrkWorks shell and explicit navigation between one central Terminal or Review surface. Workflow navigation is a separately gated extension.

**Architecture:** Keep Electron, React, TypeScript, and the existing Dockview dependency. Configure Dockview with drag-and-drop disabled and group headers hidden; retain its fixed-region split resizing. A renderer-owned navigation reducer describes the currently available destinations and focus, while Electron main owns bounded, versioned presentation and per-workspace navigation records. Existing session selection, terminal lifetime, Review authorization, and Taskmaster approval remain the authority sources. Workflow reducer state, entry points, projection and tree/timeline presentation belong to the separately gated Workflow issue.

**Tech Stack:** Electron, React, TypeScript, Dockview 8.3.1, existing `fs-ext` lock support, Node test runner, pnpm.

**Spec:** `docs/superpowers/specs/2026-10-05-application-shell-navigation-design.md` (approved 2026-10-08); `specs/orkworks-mvp.md`; `specs/session-plan-review.md`; `specs/taskmaster.md`; `specs/multi-workspace.md`; ADR 0078.

## Delivery status — checked 2026-10-09

| Slice | Evidence | Next action |
| --- | --- | --- |
| Tasks 1–2: reducer and persistence | [#778](https://github.com/Rambolarsen/orkworks/issues/778) closed by merged [PR #784](https://github.com/Rambolarsen/orkworks/pull/784), commit `6739461b88c79451f4578d6ea654b5e8359a3792` | Reuse the delivered modules and tests. Verify their integration in the remaining slices. |
| Task 3: fixed shell | [#779](https://github.com/Rambolarsen/orkworks/issues/779), draft [PR #796](https://github.com/Rambolarsen/orkworks/pull/796), head `e7e4aa4703751b5711c392113589bfd316402ff6` | Coordinate amendments with that branch's owner. Manual walkthroughs and `/code-review low` remain pending in its PR body. |
| Task 4: central Review | [#780](https://github.com/Rambolarsen/orkworks/issues/780) open; depends on #778 and #779 | Resolve the compact/medium Review boundary recorded in PR #796 before dependent integration. An open issue alone does not establish that nobody is working on it. |
| Task 5: ordinary-shell issue split | #778, #779 and #780 already created under [#755](https://github.com/Rambolarsen/orkworks/issues/755) | Keep these issues in sync; do not create duplicates. Workflow remains separately gated. |

Check live issue/PR state and current code before resuming. Checked boxes below
record merged delivery, not a fresh test run. PR #784 reports focused checks and
type checks, plus three Electron-fixture failures in its local full-suite run;
this amendment does not claim those checks were rerun.

## Plan review

**Outcome:** Complete the approved ordinary-session shell without changing session,
PTY, Review or approval authority. Keep Workflow behind its existing prerequisites.

| Dimension | Rating | Evidence and response |
| --- | --- | --- |
| Dependencies | 3 | Renderer, Electron storage, menus and terminal/Review interfaces must agree. Reuse #784 and coordinate #779/#780 ownership before changing their boundary. |
| Blast radius | 4 | The shell affects session navigation, focus, terminal attachment and responsive views. Verify exact-session returns and input isolation in each affected view. |
| State changes | 3 | New bounded presentation records coexist with legacy `layout.json`. Keep the accepted storage protocol and its existing preservation/concurrency tests. |
| Reversibility | Unknown | Storage tests preserve legacy and invalid bytes; an application downgrade/recovery rehearsal is not recorded here. Verify recovery before declaring rollout ready. |
| Uncertainty | 4 | Installed Dockview interaction/accessibility evidence and the compact Review boundary remain incomplete. Close the bounded checks below before dependent integration or merge. |

**Total: Incomplete — Reversibility is Unknown.**

**Plan quality: Investigate.** The remaining work needs two decisions: where the
approved ordinary Actions behavior will be delivered, and how #779/#780 divide
compact/medium Review navigation. Resolve these with the active owners under #755;
update the affected issue criteria and this plan before implementing that work.
This amendment does not authorize changes to PR #796's branch or waive its gates.

The review's least certain point was which requirements were already delivered.
Inspection confirms that `shellNavigation.ts` already includes `actions`, storage
tests preserve legacy bytes, and PR #796's Electron fixture exercises real
Dockview instance replacement/unmount. Those facts do not establish Actions UI
coverage, split interaction/accessibility, or end-to-end PTY continuity. The
previous all-unchecked task list also hid completed work; the status table fixes
that before anyone resumes execution.

## Global Constraints

- Preserve Electron/React/TypeScript, the existing Dockview package, native window chrome, and existing keyboard chords.
- Show only one central context at a time; never render multiple terminals or use Dockview tabs, floats, docking, or drag gestures as navigation.
- Dockview region resizing remains available. Do not set its whole-layout `locked` option, which also disables resizing.
- A session is selected only by an explicit session-selection command; overview inspection, restoring a surface, and opening an inspector do not select or acknowledge a session.
- The sidecar remains the sole authority for workspace/session state and PTY lifetime. Hiding Terminal detaches presentation only; it does not kill the runtime.
- Review reads the currently validated artifact for the explicitly selected session. The existing user click remains the only approval for the fixed review prompt.
- Electron main owns all persistent shell writes. Renderer IPC carries typed desired presentation changes, never filesystem paths, commands, prompt text, or approval grants.
- Preserve the existing workspace history lock inode and use bounded advisory locking for the new records. A storage failure leaves the current UI usable and does not gate ordinary sessions.
- Workflow-specific reducer transitions, navigation controls, projection, and tree/timeline presentation are excluded from Tasks 1–4. They may be implemented only by the separate Workflow issue after #610 and accepted #746/#741/#743/#744 contracts permit them. This plan does not implement orchestration commands or fabricate a Workflow for ordinary sessions.
- Use pnpm for desktop package commands. Do not add a dependency unless the approved implementation plan is revised and reviewed.

## Planning evidence and unresolved validation

- The checked-in desktop lockfile pins both Dockview packages to 8.3.1. Upstream Dockview documentation describes `disableDnd` and `panel.group.header.hidden`; it also says whole-layout locking disables resizing. This supports retaining Dockview while removing pane drag-and-drop and header tabs. Before #779 merges, verify this behavior against the installed 8.3.1 React component: panel dragging and header navigation are unavailable, mouse and keyboard split resizing work, and responsive changes preserve these constraints. Reuse the real Electron/Dockview fixture; source-text assertions alone do not prove interaction or accessibility. If the installed component cannot meet the accepted design, stop and seek design review before replacing the library.
- The largest product risk is confusing navigation restoration with session or workflow authority. These shell-slice reducer and IPC tests must prove that restoring Review never creates/resumes/selects a session, acknowledges attention, or restores an approval/artifact grant; the separately gated Workflow issue must prove the same for Workflow restoration.
- Current `layout.json` contains arbitrary Dockview JSON. It must be retained untouched for downgrade safety; the new shell must not deserialize it or infer region widths from its dock graph.
- The hierarchy projection and skill/role state remain outside this plan until #746 and its tracked prerequisites have accepted contracts. Ordinary-session shell navigation must work without those projections.

## File map

- `apps/desktop/src/shellNavigation.ts`: pure destination, inspection, return-target, and focus transitions; no Electron or backend calls.
- `apps/desktop/src/App.tsx`: connect reducer events to existing session selection, Review loading, and backend/workspace lifecycle.
- `apps/desktop/src/components/DockviewApp.tsx`: fixed region registration, one central surface, explicit shell navigation, Dockview drag/header configuration, and view synchronization.
- `apps/desktop/src/App.css`: fixed shell regions, separators, responsive/zoom behavior, focus indicators, and removal of Dockview tab affordances.
- `apps/desktop/electron/shellLayoutMemory.ts`: bounded installation-level layout preferences and atomic read/write/reset.
- `apps/desktop/electron/workspaceNavigationMemory.ts`: bounded per-canonical-workspace last-surface records and concurrency-safe transactions.
- `apps/desktop/electron/main.ts`, `electron/preload.ts`, and `src/orkworksWindow.d.ts`: Electron-owned shell-memory IPC, independently typed across the Electron/renderer boundary.
- `apps/desktop/electron/menuTemplate.ts`: preserve existing accelerator identities while routing them to navigation/inspector commands instead of panel visibility toggles.
- `apps/desktop/src/components/SessionListPanel.tsx`, `SessionDetailPanel.tsx`, and `ReviewPanel.tsx`: compact switcher, visible-subject details, and central Review entry/return.
- `apps/desktop/tests/shellNavigation.test.ts`, `tests/dockview.test.ts`, and `tests/menuTemplate.test.ts`: pure transitions, disabled drag/header behavior, and menu mapping.
- `apps/desktop/tests/shellLayoutMemory.test.ts` and `tests/workspaceNavigationMemory.test.ts`: storage bounds, corruption/future-version handling, locking, concurrent writers, reset/deletion ordering, and atomic-save failures.

## Implementation tasks

### Tasks 1–2: Navigation reducer and persistence — delivered in PR #784

- [x] Added the pure navigation reducer, wired App-owned state, and tested selection, inspection, return targets, restoration and workspace-generation fences.
- [x] Added Electron-owned bounded shell preferences and per-workspace Terminal/Review records, typed IPC and separately confirmed rebuild operations.
- [x] Added storage validation, legacy/invalid-byte preservation, retained locking, revision/epoch ordering, writer queues, reset/deletion barriers and failure tests.

Reuse `shellNavigation.ts`, `shellLayoutMemory.ts`, `workspaceNavigationMemory.ts`
and their tests listed above. The accepted design's storage bounds and ordering
contract remain mandatory: 16 KiB shell preferences, at most 20 workspace entries
and 64 KiB navigation data; no persisted session IDs, documents, paths or grants.
See [Saved layout migration and layout library](../specs/2026-10-05-application-shell-navigation-design.md#saved-layout-migration-and-layout-library)
and PR #784 for the full delivered contract and verification record. Do not
remove this compatibility work to reduce the complexity score.

### Task 3: Fixed, resizable shell — active in PR #796

**Files:**
- Modify `apps/desktop/src/components/DockviewApp.tsx`, `src/App.tsx`, `src/App.css`, `src/components/SessionListPanel.tsx`, and `src/components/SessionDetailPanel.tsx`.
- Modify `apps/desktop/electron/menuTemplate.ts` and the existing menu/Dockview tests.

- [ ] Before changing the active slice, agree ownership under #755 for the approved ordinary Actions overview, persistent source-separated action counts, and Show next action. The reducer destination exists; #779 and #780 do not currently name the complete UI behavior. Update the relevant issue criteria or create a scoped follow-up after agreement. Do not mark the ordinary shell complete without this coverage. Keep run-specific Actions sources behind the Workflow gates.
- [ ] Build compact Sessions, central content, and optional contextual inspector regions. Set Dockview `disableDnd`; hide each group's Dockview header; keep split resizing enabled and expose keyboard-operable separators.
- [ ] Route existing Sessions, Detail, Terminal, Capacity, Recommendations, and Reset Layout commands to the approved meanings without assigning new global accelerators. Repeated commands must not hide the only central surface.
- [ ] Add labeled Terminal/Review navigation affordances and contextual inspector controls. Do not expose a Workflow entry point or tree/timeline control in these ordinary-shell tasks.
- [ ] Make `SessionDetailPanel` follow the visible subject. Inspecting a task never changes selected session or focus in xterm; opening that session's terminal is a separate explicit action.
- [ ] Apply the approved responsive rules at 1180px, 860px, and effective 200% text-zoom width; below the compact threshold, expose one temporary content page at a time with an explicit return target.
- [ ] Add focused tests for Dockview drag/drop disabled, hidden headers, resize availability, one central context, command mappings, and no selection/unread change during inspection. For ordinary Actions, verify canonical session attention followed by recommendations in displayed order, labeled unavailable/partial sources, visible counts when Sessions/inspector are hidden, and inspection/Show next action without selection, acknowledgement or approval. Closing Actions restores the prior valid subject and invoker focus.
- [ ] Verify Terminal → compact utility page → Terminal with the same selected runtime: output continues while hidden, history is preserved, attachment refits, no second runtime starts, and navigation keystrokes never reach the PTY. Coordinate the corresponding Review check with Task 4.
- [ ] Run the full desktop Node suite and `pnpm exec tsc --noEmit` from `apps/desktop/`; perform keyboard, screen-reader, reduced-motion, 200% zoom, narrow-width, and Windows/macOS/Linux chrome walkthroughs before merging.

### Task 4: Central Review — tracked by #780

**Files:**
- Modify `apps/desktop/src/components/ReviewPanel.tsx`, `SessionDetailPanel.tsx`, `DockviewApp.tsx`, and `App.tsx`.
- Modify the Review and terminal-link tests in `apps/desktop/tests/`.

- [ ] Before integration, agree the #779/#780 boundary for Review controls and compact/medium destinations. Record which slice provides each usable entry/return path; do not merge a control that leads to an unsupported destination.
- [ ] Make Review replace Terminal in the central region and store a bounded return destination for the exact selected session.
- [ ] Keep the current artifact path validation, changed/unreadable states, refresh behavior, and one reusable document. Opening a different session must invalidate the old Review subject.
- [ ] Keep Request independent review as the existing explicit button action. Read, refresh, close, and restore do not submit the prompt.
- [ ] Test Review-to-Terminal returns, target drift, session forgetting, workspace switching, and exactly-once explicit prompt submission. Verify Terminal → Review → Terminal on the same selected runtime: output continues while hidden, history survives, reattachment refits, no second runtime starts, and document/navigation keys do not reach the PTY. Exercise wide, medium and compact entry/return paths.
- [ ] Run the full desktop Node suite and TypeScript check.

### Task 5: Issue tracking — ordinary split delivered; Workflow gated

**Files:**
- No runtime files in this task.
- Keep #755 and the affected implementation issues aligned with reviewed amendments. Verify the acceptance gates for #610, #741, #743, #744, and #746 before creating any Workflow issue.

- [x] Created #778 for Tasks 1–2, #779 for Task 3, and #780 for Task 4, with dependencies and references to #755. Preserve these separate delivery scopes.
- [ ] Create a separate Workflow issue only after #610 and the accepted #746/#741/#743/#744 contracts are verified. That issue owns Workflow reducer transitions, entry points, projection, tree/timeline presentation, and related persistence; reference #755 and #746 and record the evidence/ownership contracts and checkbox criteria.
- [ ] Keep workflow inspection separate from session selection and launch approval. No shell navigation code may create a run, launch a task, approve a proposal, or fabricate missing projection data.

## Verification and handoff

- Before ordinary-shell rollout, rehearse recovery using a disposable profile: record legacy `layout.json` bytes, run the fixed shell and save/reset its presentation, reopen the prior layout-reading version, then return to the fixed shell. Verify legacy bytes are unchanged before reopening the prior version, that it can read its layout, and that sessions remain usable after returning to the fixed shell. New presentation records must not grant session selection or approval authority. Record versions, steps and results; never use the live user profile for this check. Reuse existing corrupt/future-record and confirmed-rebuild tests for failure recovery. If compatibility fails, hold rollout and review the recovery approach.

- Run `git diff --check`, `bash scripts/doc-check.sh`, and `cd apps/desktop && pnpm exec tsc --noEmit` after each implementation PR.
- Run the full desktop suite (`cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`) for shell/navigation integration.
- Do not claim assistive-technology or usability validation from static checks; record the platform, zoom, keyboard, and screen-reader walkthrough evidence separately.
- Review each code PR with `/code-review low`; split work if it exceeds the documented review threshold.
- Review amendments and accept their scoped issue changes before dependent implementation. Existing approved, merged work stays delivered; unresolved findings hold the affected remaining steps. Workflow implementation remains separately gated by its contracts and issue.
