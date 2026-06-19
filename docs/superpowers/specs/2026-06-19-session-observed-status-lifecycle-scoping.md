# Session Observed Status — Lifecycle Scoping

> **Date:** 2026-06-19
> **Scope:** Restrict Peon-observed status to actively running sessions only

## Goal

Make `observedStatus` a substatus of the "running" lifecycle phase. Observed status only drives attention, sorting, and display when `status === "running"`. When a session ends, the final observed status is preserved as a historical marker in the detail panel but does not trigger attention.

## Why This Change

Today `sessionAttentionStatus()` returns `observedStatus ?? status` unconditionally. An ended session with a stale `observedStatus: "waiting_for_input"` still appears as needing attention, which is misleading — the session is gone, there is no input to provide.

The lifecycle status (`running`, `ended`, `killed`, `error`) is the source of truth for session existence. Peon-observed status is only meaningful while the session is actively running.

## Behavior

### Core rule

`observedStatus` only drives attention, sorting, and display when `status === "running"`.

### sessionAttentionStatus

```
Before: observedStatus ?? status
After:  status === "running" ? (observedStatus ?? "running") : status
```

### Sorting

Unchanged. Live sessions (`memoryState === "live"`) already sort above remembered sessions. Non-running live sessions (e.g. a just-ended session still in the live map) get their lifecycle status priority, which maps lower than any attention-worthy observed status.

### Display

| Lifecycle | Observed | Session list shows | Attention? | Detail panel |
|-----------|----------|-------------------|------------|-------------|
| running   | waiting_for_input | waiting_for_input | yes (red) | Status: waiting_for_input |
| running   | done              | done              | no        | Status: done |
| running   | null              | running           | no        | Status: running |
| ended     | waiting_for_input | ended             | no        | Status: ended · Final state: waiting for input |
| ended     | done              | ended             | no        | Status: ended · Final state: done |
| ended     | null              | ended             | no        | Status: ended |
| killed    | blocked           | killed            | no        | Status: killed · Final state: blocked |
| error     | any               | error             | no        | Status: error |
| creating  | any               | creating          | no        | Status: creating |

### Final observed state

When a non-running session has an `observedStatus`, the detail panel renders it as:

```
Final state: <observedStatus>
```

Styled with the existing muted `session-detail-value` class. No attention color, no border, no icon.

## Implementation

### Files changed

| File | Change |
|------|--------|
| `apps/desktop/src/components/RightSidebarHelpers.ts` | Guard `sessionAttentionStatus()`: only return observedStatus when status === "running" |
| `apps/desktop/src/components/SessionDetailPanel.tsx` | When status !== "running" and observedStatus exists, render "Final state" line |
| `crates/orkworksd/src/metadata.rs` | Peon gate: skip writing observedStatus when session lifecycle is not "running" |

### sessionAttentionStatus

```ts
export function sessionAttentionStatus(session: SessionInfo): string {
  if (session.status === "running" && session.observedStatus) {
    return session.observedStatus;
  }
  return session.status;
}
```

### Detail panel addition

```tsx
{active.observedStatus && active.status !== "running" && (
  <div className="session-detail-section">
    <span className="session-detail-label">Final state</span>
    <span className="session-detail-value">{active.observedStatus}</span>
  </div>
)}
```

### Peon gate (Rust)

In the Peon inference merge function, before writing `observed_status`:

```rust
if session.status != "running" {
    return; // observed status is only meaningful for running sessions
}
```

## Non-goals

- Do not add a session phase state machine (tracked in [#26](https://github.com/Rambolarsen/orkworks/issues/26))
- Do not change the set of observed status values
- Do not add new attention states
- Do not redesign the session list or detail panel layout
- Do not change sorting behavior beyond what the guard naturally produces

## Testing

- `sessionAttentionStatus()` returns `observedStatus` when status is "running", returns `status` otherwise
- `needsAttention()` does not trigger for non-running sessions regardless of observedStatus
- Sort order: non-running sessions sort below all running sessions regardless of observedStatus
- Detail panel shows "Final state" only for non-running sessions with an observedStatus

## Future

[Issue #26](https://github.com/Rambolarsen/orkworks/issues/26) tracks the phase-based state machine approach: explicit `Creating | Active | Ending | Ended` phases, atomic freeze of observed status on transition, and phase-aware Peon behavior. That is deferred to a later milestone.
