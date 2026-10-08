# Application Shell Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Dockview's user-arranged panel workspace with the approved fixed, resizable OrkWorks shell and explicit navigation between one central Terminal or Review surface. Workflow navigation is a separately gated extension.

**Architecture:** Keep Electron, React, TypeScript, and the existing Dockview dependency. Configure Dockview with drag-and-drop disabled and group headers hidden; retain its fixed-region split resizing. A renderer-owned navigation reducer describes the currently available destinations and focus, while Electron main owns bounded, versioned presentation and per-workspace navigation records. Existing session selection, terminal lifetime, Review authorization, and Taskmaster approval remain the authority sources. Workflow reducer state, entry points, projection and tree/timeline presentation belong to the separately gated Workflow issue.

**Tech Stack:** Electron, React, TypeScript, Dockview 8.3.1, existing `fs-ext` lock support, Node test runner, pnpm.

**Spec:** `docs/superpowers/specs/2026-10-05-application-shell-navigation-design.md` (approved 2026-10-08); `specs/orkworks-mvp.md`; `specs/session-plan-review.md`; `specs/taskmaster.md`; `specs/multi-workspace.md`; ADR 0078.

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

- The checked-in desktop lockfile pins both Dockview packages to 8.3.1. Upstream Dockview documentation describes `disableDnd` and `panel.group.header.hidden`; it also says whole-layout locking disables resizing. This supports retaining Dockview while removing pane drag-and-drop and header tabs. The first implementation task must pin this behavior against the installed 8.3.1 React component and verify that fixed split resizing and keyboard/accessibility behavior remain usable.
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

### Task 1: Add a pure shell-navigation reducer

**Files:**
- Create `apps/desktop/src/shellNavigation.ts`.
- Create `apps/desktop/tests/shellNavigation.test.ts`.
- Modify `apps/desktop/src/App.tsx` only to consume the reducer after the module is independently verified.

- [ ] Define the discriminated `CentralSurface` (`terminal`, `review`), temporary utility destinations, exact inspected subject, focus target, and bounded return descriptor. Do not add Workflow state in this slice.
- [ ] Add reducer events for explicit session selection, open/close Review, open/close inspector, workspace generation changes, missing targets, and restored preferences. Do not add Workflow entry/inspection events.
- [ ] Add tests proving inspection leaves `activeSessionId` and unread acknowledgements unchanged; explicit session selection changes only that exact session; workspace-generation change clears stale destinations; target loss falls back visibly; Review return preserves the exact session without resubmitting its prompt; restoration cannot issue selection, resume, launch, or approval events.
- [ ] Run `pnpm exec tsc --noEmit` from `apps/desktop/` and `node --experimental-strip-types --test tests/shellNavigation.test.ts`.

### Task 2: Implement bounded Electron-owned shell persistence

**Files:**
- Create `apps/desktop/electron/shellLayoutMemory.ts` and `apps/desktop/tests/shellLayoutMemory.test.ts`.
- Create `apps/desktop/electron/workspaceNavigationMemory.ts` and `apps/desktop/tests/workspaceNavigationMemory.test.ts`.
- Modify `apps/desktop/electron/main.ts`, `electron/preload.ts`, and `apps/desktop/src/orkworksWindow.d.ts`.
- Modify the desktop main-process test files that cover layout IPC and `fs-ext` locking.

- [ ] Persist only validated version-1 shell widths, Sessions visibility, and density in `shell-layout.json`, capped at 16 KiB. Reject unknown fields, non-finite values, invalid bounds, corrupt records, and future versions without overwriting their bytes.
- [ ] Keep legacy `layout.json` untouched and never call Dockview `fromJSON()` with its contents. First use creates defaults; ordinary Reset Layout resets presentation only and does not delete the legacy record or unrelated settings. Corrupt/future-version bytes remain untouched unless the user confirms a distinct reset/rebuild choice.
- [ ] Persist at most 20 canonical workspace entries and 64 KiB total in a separate navigation record. In this shell slice, each entry stores only its last central surface (`Terminal` or `Review`); run IDs, overview anchors, session IDs, paths, documents, credentials, action versions, and grants are not stored. Workflow restoration, persistence fields, or UI require the separately gated issue.
- [ ] Use a retained installation lock file, monotonic revision, opaque creation epoch, per-instance writer queue, enqueue-time revision checks, local-successor proof, and reset/deletion barriers as specified in the accepted design. Atomic replacement and read-back must succeed before dependent writes are admitted.
- [ ] Expose narrow Electron IPC operations for read, save, reset, workspace-entry deletion, and validated navigation completion. Duplicate contract types in Electron and renderer directories; do not introduce a cross-boundary import.
- [ ] Add tests for byte/entry limits; first-use races; corrupt/future data preservation and the separately confirmed reset/rebuild path; cross-instance stale revisions; rapid same-instance writes; lock contention; workspace-generation invalidation; and protected-entry oversize rejection proving the prior bytes and all entries remain intact.
- [ ] Prove successful navigation transactions, in lock/revision order, move their workspace entry to the front; reads/restores, polling, attention changes, and temporary-page inspection leave recency unchanged; and least-recent eviction preserves every entry except the protected entry when limits require it.
- [ ] Test reset/deletion barriers, failed-save successor invalidation, and read-back mismatch.
- [ ] Run the focused main-process storage tests and `pnpm exec tsc --noEmit` from `apps/desktop/`.

### Task 3: Replace panel toggles with the fixed, resizable shell

**Files:**
- Modify `apps/desktop/src/components/DockviewApp.tsx`, `src/App.tsx`, `src/App.css`, `src/components/SessionListPanel.tsx`, and `src/components/SessionDetailPanel.tsx`.
- Modify `apps/desktop/electron/menuTemplate.ts` and the existing menu/Dockview tests.

- [ ] Build compact Sessions, central content, and optional contextual inspector regions. Set Dockview `disableDnd`; hide each group's Dockview header; keep split resizing enabled and expose keyboard-operable separators.
- [ ] Route existing Sessions, Detail, Terminal, Capacity, Recommendations, and Reset Layout commands to the approved meanings without assigning new global accelerators. Repeated commands must not hide the only central surface.
- [ ] Add labeled Terminal/Review navigation affordances and contextual inspector controls. Do not expose a Workflow entry point or tree/timeline control in these ordinary-shell tasks.
- [ ] Make `SessionDetailPanel` follow the visible subject. Inspecting a task never changes selected session or focus in xterm; opening that session's terminal is a separate explicit action.
- [ ] Apply the approved responsive rules at 1180px, 860px, and effective 200% text-zoom width; below the compact threshold, expose one temporary content page at a time with an explicit return target.
- [ ] Add focused tests for Dockview drag/drop disabled, hidden headers, resize availability, one central context, command mappings, and no selection/unread change during inspection.
- [ ] Run the full desktop Node suite and `pnpm exec tsc --noEmit` from `apps/desktop/`; perform keyboard, screen-reader, reduced-motion, 200% zoom, narrow-width, and Windows/macOS/Linux chrome walkthroughs before merging.

### Task 4: Move Review into the central surface

**Files:**
- Modify `apps/desktop/src/components/ReviewPanel.tsx`, `SessionDetailPanel.tsx`, `DockviewApp.tsx`, and `App.tsx`.
- Modify the Review and terminal-link tests in `apps/desktop/tests/`.

- [ ] Make Review replace Terminal in the central region and store a bounded return destination for the exact selected session.
- [ ] Keep the current artifact path validation, changed/unreadable states, refresh behavior, and one reusable document. Opening a different session must invalidate the old Review subject.
- [ ] Keep Request independent review as the existing explicit button action. Read, refresh, close, and restore do not submit the prompt.
- [ ] Test Review-to-Terminal returns, target drift, session forgetting, workspace switching, and exactly-once explicit prompt submission.
- [ ] Run the full desktop Node suite and TypeScript check.

### Task 5: Split reviewed work into implementation issues

**Files:**
- No runtime files in this task.
- Update the issue board only after the owner reviews this plan. Verify the acceptance gates for #610, #741, #743, #744, and #746 before creating any Workflow issue.

- [ ] After plan review, create three ordinary-shell implementation issues with checkbox acceptance criteria: (1) navigation state and bounded presentation persistence (Tasks 1–2); (2) fixed, resizable shell regions, compact Sessions, and command mapping (Task 3, depending on 1); (3) central Review navigation and return behavior (Task 4, depending on 1 and 2). Reference #755 in each issue and keep their scopes separately reviewable.
- [ ] Create a separate Workflow issue only after #610 and the accepted #746/#741/#743/#744 contracts are verified. That issue owns Workflow reducer transitions, entry points, projection, tree/timeline presentation, and related persistence; reference #755 and #746 and record the evidence/ownership contracts and checkbox criteria.
- [ ] Keep workflow inspection separate from session selection and launch approval. No shell navigation code may create a run, launch a task, approve a proposal, or fabricate missing projection data.

## Verification and handoff

- Run `git diff --check`, `bash scripts/doc-check.sh`, and `cd apps/desktop && pnpm exec tsc --noEmit` after each implementation PR.
- Run the full desktop suite (`cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`) for shell/navigation integration.
- Do not claim assistive-technology or usability validation from static checks; record the platform, zoom, keyboard, and screen-reader walkthrough evidence separately.
- Review each code PR with `/code-review low`; split work if it exceeds the documented review threshold.
- Preserve the issue dependency gates and do not begin implementation before this plan is reviewed and the scoped issue is accepted. Workflow implementation remains separately gated by its contracts and issue.
