---
type: decision
status: accepted
title: "Replace `react-resizable-panels` with `dockview` for draggable panel layout"
---

# Replace `react-resizable-panels` with `dockview` for draggable panel layout

- Status: accepted
- Deciders: user
- Date: 2026-06-17

## Context

The current UI uses `react-resizable-panels` for a fixed three-column layout (left sidebar, center terminal, right sidebar). This is rigid — users cannot reposition panels to suit their workflow. The product goal is session observability, and forcing a single layout works against that.

Additionally, the right sidebar (300px) is underused — it only shows detail for the active session, which would be more useful docked alongside the session list on the left. Capacity and recommendation panels are planned for M8/M9 and need space.

## Decision

Replace `react-resizable-panels` with `dockview` (`dockview-react`). dockview provides:

- Drag-and-drop panel repositioning (left/right/bottom/top/float)
- Native tab support within panels (needed for terminal tabs)
- Persistent layout save/restore
- Dark theme compatible

Five panels registered in the dockview grid:
1. Session list (left, top)
2. Session detail (left, bottom)
3. Terminal — tabbed (center)
4. Capacity — placeholder (right)
5. Recommendations — placeholder (right)

All panels draggable. Default layout ships as described but users can rearrange freely.

## Consequences

- **Easier**: Users can customize their workspace layout. Future panels (capacity, recommendations, webview, markdown viewer) drop in as dockable panels without layout redesign.
- **Easier**: Terminal tabs managed by dockview natively instead of custom tab bar code.
- **Harder**: Adds ~100KB bundle weight (dockview). Slightly more complex initialization code.
- **Harder**: Full panel docking means testing must cover repositioning, not just fixed layout.
- `react-resizable-panels` is removed as a dependency.
