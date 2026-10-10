---
type: "Architecture Decision"
title: "Fixed desktop shell with central navigation"
description: "Architecture decision record: Fixed desktop shell with central navigation."
tags: ["orkworks", "architecture"]
---

# Fixed desktop shell with central navigation

- Status: accepted; library choice superseded by [ADR 0082](0082-react-grid-desktop-shell.md)
- Deciders: user
- Date: 2026-10-08

## Context

The Dockview shell has grown into a tabbed, user-arranged panel grid. The
approved application-shell design places session awareness in a compact
switcher and workflow awareness in a central overview. Users need deliberate
navigation between that overview and one selected terminal, plus one reusable
Review surface. Details, Actions, Recommendations, and Capacity should follow
the currently visible subject or workspace without competing for a permanent
column.

The product must continue to show one central context and one terminal. The
shell cannot turn session inspection into session selection, let a restored
view resume work, or change any existing approval or Electron authority.

## Decision

- Keep Electron with React and TypeScript.
- Use compact Sessions, one central surface (Terminal, Review, or eligible
  Workflow), and one optional contextual inspector. At narrow widths, the
  inspector and Sessions become temporary pages with explicit return targets.
- Use the branching timeline as the default Workflow presentation and provide
  a tree presentation over the same validated projection. Tree/Timeline changes
  presentation only.
- Restore the last valid central surface for the workspace after revalidating
  its subject. Selecting an actual session explicitly opens its Terminal.
  Restoration never selects, resumes, launches, acknowledges, approves, or
  recreates a session, run, task, artifact, or grant.
- Retain Dockview 8.3.1 for resizable fixed regions and programmatic view
  hosting. Disable drag-and-drop, hide group headers, and do not expose floating,
  docking, tab reordering, or tab-based central navigation. Leave region
  resizing enabled. The native operating-system window remains draggable.
- Keep Dockview's legacy `layout.json` unchanged for downgrade safety. Electron
  owns bounded installation-level shell preferences and canonical-workspace
  navigation records. Navigation records are presentation state, not session,
  workspace, execution, approval, or peer-instance authority. Use retained
  installation-scoped advisory locking and revision-checked atomic writes for
  concurrent instances.
- Review remains one central document for the exact selected session's current
  validated Markdown artifact. Request independent review remains a separate,
  explicit user action with the existing authenticated fixed-prompt handoff.

The approved interaction, responsive, accessibility, lifecycle, migration, and
storage contracts are in the
[application-shell/navigation design](../superpowers/specs/2026-10-05-application-shell-navigation-design.md).
This ADR does not claim that the shell or Workflow surface has shipped. Runtime
implementation requires the reviewed execution plan and separately accepted
Workflow contracts.

## Consequences

- **Easier:** Session switching, Workflow inspection, Review, and utility
  details have explicit transitions and return behavior without draggable
  panel arrangements or hidden tab destinations.
- **Easier:** Dockview supplies fixed-region resizing without replacing the
  dependency or changing Electron/React/TypeScript.
- **Harder:** The renderer must own explicit navigation, focus restoration,
  responsive temporary pages, and subject binding instead of relying on Dockview
  panel visibility.
- **Harder:** Shell preferences and workspace navigation need bounded,
  concurrency-safe Electron persistence. The old docking layout cannot be
  replayed into the new shell and remains stored for downgrade safety.
- One visible terminal remains mandatory. Hidden Terminal presentation may
  detach its renderer attachment; sidecar PTY lifetime and output draining
  remain unchanged under ADR 0022.
- The Workflow overview remains gated by #610 and the accepted #746 projection
  and evidence contracts. Ordinary sessions continue to work without it.
