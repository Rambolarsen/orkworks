# Read-only terminal replay for remembered sessions

## Problem

The sidecar preserves bounded terminal output for sessions after their PTY has
ended, but the renderer only constructs a terminal for `lifecycle === "alive"`.
Selecting a remembered session therefore hides output that is still available
from `GET /sessions/:id/terminal-output`.

## Decision

The Terminal panel will retain one active context. For an alive session, it
continues to attach an interactive xterm instance to the session WebSocket.
For any remembered session, it instead renders the persisted replay in a new,
read-only xterm instance populated through the existing HTTP endpoint.

The historical instance must disable stdin and not create a WebSocket. It is
disposed when the selected session changes or the panel unmounts, so the
live-session cache remains bounded by the existing `pruneTerminals` behavior.

## Data flow

```text
remembered session selected
  -> TerminalPanel selects historical replay path
  -> GET /sessions/:id/terminal-output
  -> read-only xterm writes returned lines
```

The endpoint already returns an empty list when no replay exists. In that case
the panel shows an explicit empty state rather than implying that a process can
be resumed or attached.

## Error handling

If the replay request fails, the panel shows a non-interactive unavailable
message. The request is cancelled/ignored if selection changes before it
finishes, preventing stale output from being written into the next session.

## Constraints

- Historical replay is limited by the existing 1,000-line / 1 MiB persistence
  policy.
- No PTY, WebSocket, terminal input, lifecycle transition, or terminal-cache
  behavior changes.
- The single-active-context product principle remains intact.

## Testing

Source-level renderer regression tests will assert that remembered sessions
use the read-only replay component and that this component calls the existing
HTTP replay endpoint without using `ensureTerminal`. Existing live-terminal
and pruning tests remain green.
