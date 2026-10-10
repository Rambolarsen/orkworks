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
