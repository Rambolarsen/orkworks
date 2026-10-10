---
type: "Implementation Plan"
title: "Delivery plan: fixed shell and shared navigation"
description: "Implementation plan: Delivery plan: fixed shell and shared navigation."
tags: ["orkworks", "plans"]
---

# Delivery plan: fixed shell and shared navigation

**Issue:** [#779](https://github.com/Rambolarsen/orkworks/issues/779)  
**Depends on:** [#778](https://github.com/Rambolarsen/orkworks/issues/778), delivered in PR #784  
**Current status:** open; draft [PR #796](https://github.com/Rambolarsen/orkworks/pull/796) is owned on its existing branch.

[Back to the delivery map](2026-10-08-application-shell-redesign.md)

## What changes for the user

Sessions stay in a compact list. The main work area shows one thing at a time,
with an optional details area beside it when there is room. Users can resize the
areas, but cannot rearrange the app into a collection of floating or tabbed
panels. At smaller widths, a page has a clear way back.

This piece builds the shared layout and navigation support. Review-specific
controls belong to #780. The ordinary Actions view belongs to #805. This piece
does not add Workflow controls.

## How to tell it works

- A user can resize the shell, move between its available destinations, and
  return to the main work area at wide, medium, narrow, and 200% text-zoom sizes.
- Existing menu commands still have clear, predictable results; commands do not
  hide the only main view.
- Inspecting a session leaves the selected session and terminal focus alone.
- Hiding and reopening Terminal brings back the same running session and its
  output.
- Keyboard and assistive-technology use, reduced motion, narrow width, and
  native window behavior have been walked through and recorded before merge.

Use #779's acceptance criteria for the exact responsive rules, commands, and
automated checks. PR #796 still lists its manual walkthroughs and `/code-review
low` as merge gates. This plan does not amend that PR or its owner's branch.

## Complexity and review

| Dimension | Rating | Evidence |
| --- | --- | --- |
| Dependencies | 3 | Reuses delivered navigation state and needs the Review and Actions slices to plug into the shared host. |
| Blast radius | 4 | The shell is the frame for session switching, focus, and every central view. |
| State changes | 2 | This slice changes presentation behavior; bounded persistence was delivered in #778. |
| Reversibility | 1 | The UI change can be reverted; legacy layout data is retained. |
| Uncertainty | 4 | The existing PR still needs real keyboard, assistive-technology, zoom, width, and platform walkthroughs. |

**Total: 14/25. Plan quality: Ready for the current execution.** The outcome and
boundary are clear. The listed walkthroughs are required evidence before merge.

## Dockview removal amendment — approved 2026-10-10

The owner authorized this session to adopt PR #796 and requested Dockview's
removal. The concrete replacement is accepted in [ADR 0082](../../adr/0082-react-grid-desktop-shell.md).
The owner approved the replacement on 2026-10-10 after reviewing its visual preview. This amendment
replaces the Dockview-specific delivery mechanics; existing user outcomes and
#779/#780/#805 ownership remain binding.

### Deliverable and boundaries

Replace `DockviewApp` with `ApplicationShell` and a small accessible region
separator. `App.tsx` uses explicit shell destinations and the existing navigation
reducer instead of a `DockviewApi` ref. Existing session/terminal/utility bodies
are reused. Remove library imports, CSS, package dependencies and lock entries.
Use the existing Electron persistence APIs and menu command identities.
Review controls remain unavailable until #780 supplies a working destination.

### Execution and checks

1. Reconcile ADR 0078 and the accepted shell design after owner approval;
   update #779's library-specific criteria and PR scope. Preserve historical
   ADR text with the replacement pointer.
2. Replace imperative panel operations with explicit renderer state. Cover
   repeated utility commands, Sessions focus/hide, labeled returns, scoped
   Escape, subject loss, workspace changes, and delayed preference hydration.
   Preserve selected session and unread state during inspection.
3. Implement fixed CSS Grid regions and pointer/keyboard separators. Cover
   bounds, available-width clamping, conditional separator visibility, resize
   cancellation, and 1180/860px transitions at effective 200% zoom.
4. Replace Dockview-specific tests with meaningful React/Electron behavior
   checks. Exercise Terminal → utility → Terminal using the retained runtime,
   output while hidden, attachment refit, and navigation-input isolation.
   Keep Review and Actions follow-up tests in their owning slices.
5. Update architecture/source-user guidance and remove obsolete library styles
   and dependencies. Verify there are no production Dockview references and
   legacy layout bytes remain unchanged in a disposable profile.
6. Run desktop tests/type-check, `bash scripts/verify-repo.sh`, documentation
   and worktree checks, then review the final code diff at the required effort.
   Record actual keyboard, screen-reader, reduced-motion, zoom, narrow-width,
   and Windows/macOS/Linux chrome evidence; unavailable evidence remains a
   merge blocker rather than a passing claim.

### Amendment review

| Dimension | Rating | Evidence |
| --- | --- | --- |
| Dependencies | 2 | Existing React, terminal bodies and Electron store; no new library. |
| Blast radius | 4 | Replaces the shell and its command/focus routing. |
| State changes | 2 | Renderer presentation changes; persisted schema is retained. |
| Reversibility | 1 | Git revert restores dependencies and renderer; old layout bytes retained. |
| Uncertainty | 4 | Native resize, accessibility, zoom and platform walkthroughs remain. |

**Total: 13/25. Plan quality: Ready for execution.** Coverage includes
navigation, sizing, persistence races and terminal continuity. The replacement architecture is owner-approved; implementation may proceed.
