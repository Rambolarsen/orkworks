# Dockview Header Unification — Design

> **Date:** 2026-06-18
> **Scope:** Sessions panel header cleanup

## Goal

Keep Dockview's built-in header as the single visible header row for the Sessions panel, style it to match the current subheader treatment, and remove the duplicate inner `Sessions` row from the panel content.

The user-facing outcome is a cleaner panel with the same tab/docking behavior, less repeated chrome, and no new information density beyond what already exists today.

## Why This Change

The current Sessions panel shows two stacked headers:

1. Dockview's native panel header
2. The custom inner `.panel-header` rendered by `SessionListPanel`

This duplicates the `Sessions` label, consumes vertical space, and makes the panel feel heavier than the surrounding UI. The second row does not provide enough additional context to justify the duplication.

## Recommended Approach

Adopt the "single unified header" approach:

- Keep Dockview's native header row.
- Restyle that row so it visually matches the current inner subheader language.
- Move the Sessions panel create action (`+`) into the Dockview header area.
- Remove the inner `.panel-header` from `SessionListPanel`.

This preserves Dockview's built-in affordances while simplifying the panel structure.

## Non-Goals

- Do not introduce a richer contextual header with additional badges or status text.
- Do not redesign all panel headers beyond the minimum needed to establish a consistent visual treatment.
- Do not change session list ordering, session item behavior, or terminal selection behavior.
- Do not change Dockview layout persistence or panel registration.

## Architecture

The structural ownership stays the same:

- `DockviewReact` owns the actual panel header and tab behavior.
- `SessionListPanel` owns only the list content.
- `DockviewApp` remains the place where panel-level Dockview integrations live.

The cleanup is therefore split across presentation responsibilities:

- `DockviewApp.tsx`: attach Sessions-specific header actions to Dockview
- `SessionListPanel.tsx`: remove the redundant content header
- `App.css`: align Dockview header styles with the existing subheader visual language

## Component Changes

### `SessionListPanel.tsx`

Remove the inner header row:

- delete the `.panel-header` wrapper
- remove the inline `Sessions` label from the panel body
- remove the inline `+` button from the panel body

After this change, the component should render:

- panel content wrapper
- empty state or session list

No panel-title chrome should remain inside the content area.

### `DockviewApp.tsx`

Add a Sessions-specific Dockview header action:

- render a lightweight `+` action in the Dockview header for the Sessions panel
- wire it to the existing `onCreateSession` callback
- keep the action scoped to the Sessions panel only

This should use Dockview's supported header-actions extension point rather than overlaying ad hoc DOM into the panel body.

### `App.css`

Restyle Dockview's header row to match the current subheader treatment as closely as practical:

- muted header text color
- uppercase label styling
- tighter spacing
- same border rhythm
- transparent/minimal tab background treatment

The Sessions panel should feel visually equivalent to today's subheader row, but now as the Dockview-owned header.

The existing `.panel-header` rule can be removed if it is no longer used anywhere else.

## Visual Rules

The resulting Sessions panel should read like this:

- one visible header row
- `Sessions` label in the Dockview header
- `+` action aligned to the right in that same row
- session list starts immediately below

The design should stay intentionally restrained:

- no new icons beyond the existing add affordance
- no new metadata in the header
- no extra accent colors beyond existing Dockview/session palette

## Behavior

Behavior remains unchanged except for header ownership:

- clicking `+` still creates a session
- clicking a session still selects it
- Dockview tabs and drag behavior remain intact
- layout persistence remains intact

The cleanup is presentational, not behavioral.

## Risks

### Global Header Styling Spillover

Dockview header CSS is shared across panels, so style updates may affect non-Sessions panels.

Mitigation:

- prefer scoped selectors under `.orkworks-dockview`
- keep the styling close to the existing visual system rather than introducing a Sessions-only special case
- keep Sessions-specific logic in header actions, not in global CSS hacks

### Header Action Placement

If the add action is implemented too generically, it may appear on other panels.

Mitigation:

- gate the rendered action on the active Dockview panel/group context so it only appears for the Sessions panel

## Testing

Add or update tests to cover:

- `SessionListPanel` no longer renders the inner `Sessions` header text/button chrome
- Sessions panel still exposes a create-session action through the Dockview layer
- existing Dockview panel registration remains intact

Manual verification should confirm:

- only one Sessions header row is visible
- the `+` action still works
- the session list sits directly under the Dockview header
- other panels still render correctly

## Rollout Notes

This is a safe UI cleanup and does not require backend changes, protocol changes, or ADR work. It can be implemented independently of the broader Dockview session-detail work as long as it preserves the existing panel IDs and callbacks.
