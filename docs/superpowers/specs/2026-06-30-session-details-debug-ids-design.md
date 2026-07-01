# Session Details Debug IDs Design

- Date: 2026-06-30
- Status: proposed

## Summary

Keep session identifiers out of the default Details surface, but make them available when debugging. Add a persisted Settings toggle named `Show debug metadata`. When enabled, the selected session's Details panel shows:

- `OrkWorks session ID`
- `Harness session ID`

If no harness-native session ID has been captured, Details should show `Not captured`.

## Goals

- Keep the normal Details view focused on user-facing session facts.
- Make internal identifiers available without requiring code inspection or backend logs.
- Reuse the existing app settings flow rather than adding a special-case developer flag.

## Non-Goals

- No inline editing of either identifier.
- No new debug drawer, disclosure widget, or tooltip system in Details.
- No backend metadata shape changes.

## UX

### Settings

Add a checkbox under Settings:

- Label: `Show debug metadata`
- Behavior: persisted app-wide setting, saved immediately

### Details

When `Show debug metadata` is off:

- Do not render either ID field.

When `Show debug metadata` is on:

- Render `OrkWorks session ID` with the selected session's `id`
- Render `Harness session ID` with `resume.harnessSessionId` when present
- Otherwise render `Not captured`

## Data Flow

- `SessionInfo.id` already exists in the renderer payload.
- `SessionInfo.resume?.harnessSessionId` already exists in the renderer payload.
- Add a new app settings field for renderer/electron state:

```ts
debug: {
  showSessionIds: boolean;
}
```

No Rust or HTTP API changes are required.

## Testing

- Settings normalization should default `showSessionIds` to `false`.
- Persisted settings should round-trip the debug flag.
- Settings UI should expose the checkbox and save through a dedicated debug-settings IPC path.
- Details should gate the ID fields behind the debug flag and show `Not captured` for missing harness IDs.
