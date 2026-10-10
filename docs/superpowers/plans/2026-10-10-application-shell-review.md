# Delivery plan: Review and return to the same terminal

**Issue:** [#780](https://github.com/Rambolarsen/orkworks/issues/780)  
**Depends on:** [#779](https://github.com/Rambolarsen/orkworks/issues/779), the shared shell

[Back to the delivery map](2026-10-08-application-shell-redesign.md)

## What changes for the user

From a selected session, the user can open its current, validated plan in the
main work area. A clearly labeled return action takes them back to that exact
session's terminal. The path works in wide, medium, and compact windows and at
200% text zoom.

Review belongs to the selected session and its current document. Opening,
reading, refreshing, or closing it does not send a review request. The existing
explicit request button remains the only way to send one.

## How to tell it works

- From a session, a user can open Review and return to the same terminal at all
  supported widths.
- Changing width, switching sessions, losing the document, or changing
  workspace never leaves an old document attached to the wrong session.
- Returning to Terminal preserves the same running session and output.
- Reading or closing Review never submits a prompt; the explicit request button
  still works once when clicked.
- Keyboard focus returns to the control that opened Review, or to a safe visible
  destination if that control no longer exists.

Use #780 for exact responsive thresholds and required automated cases. The
shared shell host and temporary-page behavior are provided by #779; this issue
owns Review's entry, rendering, subject binding, and return behavior. Workflow
navigation is excluded.

## Complexity and review

| Dimension | Rating | Evidence |
| --- | --- | --- |
| Dependencies | 3 | Needs the shared shell from #779 and the delivered navigation state from #778. |
| Blast radius | 3 | Review is one central surface, but stale session/document binding would expose the wrong work. |
| State changes | 2 | Adds navigation behavior around an existing Review document and explicit request action. |
| Reversibility | 1 | The UI path can be reverted without changing stored session data. |
| Uncertainty | 3 | Wide, medium, compact, focus, and PTY-continuity behavior need integration evidence. |

**Total: 12/25. Plan quality: Ready.** The owner boundary and return behavior are
explicit; the issue names the checks that establish them.
