# Dim dead sessions design

## Goal

Make terminally finished sessions easier to distinguish from live sessions in
the Sessions list.

## Design

The session row already derives `remembered` from `s.lifecycle === "dead"` and
applies `session-row--remembered`. Reuse that existing semantic and styling
hook, strengthening its opacity from the current `0.78` to `0.62`. The row
remains clickable and keyboard-selectable; existing hover/focus affordances
remain in place even though the dead row's contents are dimmed.

Only dead sessions are affected. Creating, alive, and stopping sessions keep
their current emphasis. No session data, sorting, labels, status logic, or
terminal behavior changes.

## Testing

Add a source-level renderer regression assertion that the session list applies
`session-row--remembered` from the dead-session condition and that the CSS
defines the exact `0.62` dimmed treatment. Run the desktop type-check, test
suite, doc currency check, and worktree currency check.
