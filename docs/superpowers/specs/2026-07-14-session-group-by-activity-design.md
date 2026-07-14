# Session Grouping by Recent Activity Design

## Goal

Place a session that is resumed today in the **Today** group without changing its original creation timestamp.

## Decision

Session-list date groups use `lastActivityAt` when it is present and valid. Older metadata without that field, or an invalid activity timestamp, falls back to `created_at`.

## Data flow

The resume endpoint already updates `lastActivityAt` to the resume time. The renderer's `groupForSession` function will select that timestamp for its calendar-day and seven-day comparisons. No API, metadata schema, or sidecar change is required.

## Error handling

Malformed timestamps remain safely classified as **Earlier** when neither activity nor creation time parses.

## Testing

Add a focused unit test proving that a session created yesterday but active today is grouped as **Today**. Existing tests continue to cover creation-time fallback and invalid dates.
