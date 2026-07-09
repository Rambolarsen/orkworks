# Session Row Meta Compaction Design

## Goal

Reduce the visual footprint of the `session-row-meta` area in the desktop session list so the right side of each row feels less bulky, while keeping unread state from shifting the row layout.

## Scope

This change is limited to:

- `apps/desktop/src/components/SessionListPanel.tsx`
- `apps/desktop/src/App.css`
- the existing source-based UI regression test in `apps/desktop/tests/dockview.test.ts`

## Design

- Keep the existing single-line session row design.
- Add a `session-row-leading` cluster on the left side that contains a dedicated unread slot before `session-row-primary`.
- Move the unread dot from the right-side actions cluster into that leading slot.
- Render the slot for every row, even when no unread dot is present, so the row never shifts based on unread state.
- Set the unread slot width to `6px` and the gap between the slot and `session-row-primary` to `6px`.
- Change `.session-row` left padding from `var(--space-5)` to `var(--space-2)` so the label start remains within about `2px` of its current position while still reducing left-side padding.
- Preserve the compact right-side metadata/actions footprint and keep the existing font sizes and interaction targets for the kill/delete controls.

## Non-Goals

- No broader renderer/component restructuring
- No changes to session sorting, row behavior, or action visibility
- No token-scale changes in `tokens.css`

## Verification

- Update the existing source assertion that currently expects `session-row-unread-dot` inside `session-row-actions` so it instead asserts:
  - a dedicated unread slot exists before `session-row-primary`
  - the unread dot renders in that slot
  - the unread dot no longer renders inside `session-row-actions`
- Keep explicit CSS assertions for the compact spacing values that must remain true:
  - `.session-row-secondary { gap: var(--space-1); }`
  - `.session-row-meta { grid-template-columns: 12px 6ch; column-gap: var(--space-2); }`
  - `.session-row-actions { gap: 0; }`
  - `.session-row-kill` and `.session-row-forget` keep `padding: 0 2px;`
- Add a source assertion that the kill/delete buttons still call `e.stopPropagation()` so moving the unread slot does not change row-selection behavior.
- Run the focused `dockview` test file after the markup/CSS change.
