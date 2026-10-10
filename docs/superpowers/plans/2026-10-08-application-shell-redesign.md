# Application shell: delivery map

This is the short guide to what the shell work gives users and how the pieces
fit together. Each outcome has its own small plan. The linked GitHub issue is
the source for detailed engineering acceptance criteria; this guide avoids
repeating them.

## What users will get

The current window relies on movable panels and tabs. The approved redesign
gives users a compact session list, one main work area, and optional details.
Users can switch deliberately between a terminal, a plan for review, and a
read-only overview of ordinary work without losing track of the session they
were using.

## Delivery status — checked 2026-10-10

| User-facing piece | Issue | Status |
| --- | --- | --- |
| Remember where the shell was and which central view was open | [#778](https://github.com/Rambolarsen/orkworks/issues/778) | Delivered in [PR #784](https://github.com/Rambolarsen/orkworks/pull/784) |
| Replace movable panels with a fixed, resizable shell | [#779](https://github.com/Rambolarsen/orkworks/issues/779) | Open; [PR #796](https://github.com/Rambolarsen/orkworks/pull/796) is an open draft |
| Open Review and return to the same terminal | [#780](https://github.com/Rambolarsen/orkworks/issues/780) | Open |
| See ordinary actions, separate counts, and the next item to inspect | [#805](https://github.com/Rambolarsen/orkworks/issues/805) | Open |

The shell foundation in #779 comes first. After that, Review (#780) and Actions
(#805) are separate deliveries and can be completed independently. The ordinary
shell is complete only when #779, #780, and #805 each pass their issue criteria.
Keep [#755](https://github.com/Rambolarsen/orkworks/issues/755) open
through the reviewed execution handoff and remaining #746 gates.

## The three delivery plans

1. [Fixed shell and shared navigation](2026-10-10-application-shell-fixed-shell.md) — #779
2. [Review and return to the same terminal](2026-10-10-application-shell-review.md) — #780
3. [Ordinary Actions overview](2026-10-10-application-shell-actions.md) — #805

The detailed implementation requirements stay in those issues. The approved
[navigation design](../specs/2026-10-05-application-shell-navigation-design.md),
[ADR 0078](../../adr/0078-fixed-desktop-shell-and-central-navigation.md), and
[MVP Actions overview](../../../specs/orkworks-mvp.md#actions-overview) remain
the product sources.

## Rules that apply to every piece

- Inspecting a session or action never changes the selected session, marks
  attention as handled, or moves keyboard focus into the terminal.
- Hiding the terminal only hides its view; the same session keeps running.
- Reading Review never requests a review. Only its existing explicit button
  sends that request.
- Actions is an overview. It does not approve or run work.
- Workflow navigation and its tree/timeline view stay outside this delivery
  until the existing #610 and #741/#743/#744/#746 gates are accepted.

## Plan review

**Outcome:** deliver the approved ordinary shell through three independently
reviewable user outcomes, while preserving the existing session and approval
rules and leaving Workflow gated.

**Plan quality: Ready.** The completed foundation is separated from remaining
work; every remaining outcome maps to an open issue; and the two follow-on
pieces have no dependency on each other. Open usability and accessibility
walkthroughs remain delivery gates in the relevant issues, not hidden scope.

Before resuming any piece, refresh its issue and PR status. The issues carry the
current checklists; completed checkboxes in them are the delivery record.
