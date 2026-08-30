# Review Tab Plan File Refresh Design

## Context

The Review tab in the desktop client loads the selected session's `planPath` content only when the active session ID changes. If the underlying plan/spec file on disk is modified by an agent or user while the same session remains selected, the Review tab displays stale content until the session is unselected and reselected or the panel is closed and re-opened.

Issue #379 defines the requirements:
- The Review tab can refresh the current session's associated plan content without changing sessions.
- Refreshes do not display stale responses after switching sessions or workspaces.
- The current `planPath` remains the source of truth; do not turn Review into a latest-file browser.
- Add regression coverage for file/content refresh behavior.

## Decision

1. **Tab Header Action (`DockviewApp.tsx`)**:
   - Generalize the Dockview right header actions (`DockviewHeaderActions`) so that when the active panel is `"review"`, it renders a `"Refresh plan"` button with the `RotateCw` icon from `lucide-react`.
   - The refresh action is active when the selected session has an openable plan (`session?.hasOpenablePlan`).
   - Clicking the refresh button invokes `onRefreshReview()`, which increments `reviewTick`.

2. **Refresh Coordination (`App.tsx` & `DockviewApp.tsx`)**:
   - Introduce `reviewTick: number` in `DockviewAppData` (defaulting to `0`, matching the established `resumeTick` pattern).
   - In `App.tsx`, `handleReviewPlan` activates the `"review"` panel and increments `reviewTick` so that clicking "Review plan" in the Session Detail panel or clicking a terminal plan link also requests a fresh read of the plan file.

3. **Smooth In-Place Refresh (`ReviewPanel.tsx`)**:
   - `ReviewPanel` accepts `{ sessionId: string | null; reviewTick?: number }`.
   - When switching sessions (`sessionId` change), content is reset to `null` to display the loading state.
   - When `reviewTick` changes for the current session, the component retains the currently rendered Markdown while fetching fresh content in the background, updating atomically on response without layout jumps.
   - The incrementing `requestId` guard is preserved to discard any late responses from superseded sessions or workspaces.
   - If the file is deleted or unreadable, the component transitions to the error state (`"This plan is no longer available."`) with a working `"Retry"` button.

## Boundaries

- The session's recorded `planPath` remains the strict source of truth; no speculative file scanning is introduced.
- Existing authenticated IPC bridge `getPlanContent(sessionId)` and sidecar route `/sessions/:id/plan-content` are reused as-is.
- No background disk polling or file watchers are added.

## Testing

- Unit tests in `apps/desktop/tests/dockview.test.ts` verify:
  - Review tab header action renders the refresh control for sessions with openable plans.
  - `ReviewPanel` re-fetches content on `reviewTick` changes.
  - Stale responses across session switches are safely ignored.
- Full desktop test suite (`node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`) and TypeScript type-check (`tsc --noEmit`) pass.
