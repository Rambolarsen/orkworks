---
type: Architecture Decision
title: Recommendations in the main view
description: Host workspace Recommendations in the central content pane at every width.
tags: [orkworks, architecture, navigation]
---

# Recommendations in the main view

- Status: accepted
- Deciders: user
- Date: 2026-10-16

## Context

Recommendations need the main content area for reading evidence and proposed
changes. The user approved replacing the terminal with Recommendations, keeping
Sessions available and returning to the previous view. Tracked in
[#890](https://github.com/Rambolarsen/orkworks/issues/890).

## Decision

Supersede only Recommendations placement in [ADR 0078](0078-fixed-desktop-shell-and-central-navigation.md)
and the [shell design](../superpowers/specs/2026-10-05-application-shell-navigation-design.md).
Recommendations uses the existing transient utility page at every width, including
wide windows. It replaces Terminal, Review, or the New session composer; Back,
toggling Recommendations, or Escape within the page restores the underlying
view and invoker. At compact widths Sessions remains a separate temporary page.
Selecting a session explicitly opens its Terminal. Deep links focus the requested
recommendation card in the central page.

Recommendations has no inspector pin or resize handle. Details, Actions, and
Capacity retain their existing inspector/page behavior. Persistent navigation
continues recording only Terminal or Review: Recommendations is transient
presentation state. Workspace changes clear it. PTY ownership, recommendation
actions, approval gates, and Electron authority are unchanged.

## Implementation and checks

1. Amend the MVP shell contract and current architecture/product references.
2. Reuse the renderer utility host, excluding Recommendations from inspector
   width calculations and giving the central header its destination title.
3. Test wide/medium/compact opening, Back/Escape, Review return, explicit
   session selection, and workspace reset in the existing Electron fixture.
4. Run repository verification and diff-scoped code review before PR merge.

## Consequences

Evidence and proposed changes have the full main pane. Existing responsive
return and focus behavior can be reused without a navigation storage migration.
Other utility destinations remain mutually exclusive with Recommendations.
