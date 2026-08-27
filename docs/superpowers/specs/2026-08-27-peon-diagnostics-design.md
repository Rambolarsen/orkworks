# Debug-Only Peon Diagnostics

## Context

Taskmaster recommendations depend on workflow observations produced by Peon.
The current desktop UI can show the timestamp of a successful Peon inference,
but it cannot distinguish between a session that has not been selected, one
waiting for an inference slot, one whose provider failed, and one that
completed without producing an observation. This makes the recommendation
feature difficult to troubleshoot.

The diagnostics are intended for development and troubleshooting, not for
normal session monitoring.

## Decision

Expose a compact, per-session `PeonDiagnostics` snapshot through the existing
session-list API. Render it in the selected session's Detail panel, gated by
the existing Debug setting, `showSessionIds` / “Show debug metadata”. No new
setting and no new panel are introduced.

The snapshot reports:

- scheduler state: `idle`, `candidate`, `in_flight`, `completed`, or `failed`;
- the last attempt timestamp;
- the last successful inference timestamp;
- the last provider/error summary, when present; and
- the number of workflow observations persisted for the session.

The diagnostics are read-only and do not alter Peon scheduling, provider
fallback, observation eligibility, or recommendation evaluation.

## Data flow

```text
Peon scheduler/provider
        ↓ per-session runtime bookkeeping
SessionInfo / GET /sessions
        ↓ existing session polling
SessionDetailPanel
        ↓ gated by showDebugMetadata
Peon diagnostics block
```

The sidecar owns scheduler and provider state because it is the only component
that can observe the complete attempt lifecycle. The renderer receives a
serialized snapshot and does not infer state from timestamps.

## State semantics

- `idle`: no Peon attempt is currently pending for the session.
- `candidate`: the scheduler has selected the session and is preparing an
  attempt.
- `in_flight`: provider inference is running.
- `completed`: the most recent attempt returned a valid provider result;
  `lastSuccessfulInferenceAt` is updated even when no workflow observation
  was emitted.
- `failed`: the most recent attempt failed, timed out, or returned unusable
  output; the error summary is retained for diagnosis.

The observation count includes only durably accepted workflow observations for
that session. Duplicate reports do not increase it.

## Error handling and privacy

Diagnostics are best-effort metadata. A missing snapshot must not block session
creation, session polling, Peon inference, or recommendation queries. Provider
error summaries must use the existing bounded/sanitized error-summary path and
must not expose prompts, terminal transcripts, credentials, or raw model
output.

The renderer must not display the block unless the existing debug metadata
toggle is enabled. Toggling the setting should affect the block through the
same `showDebugMetadata` prop already used by the other debug fields.

## Testing

- Sidecar tests verify state transitions for successful, failed, and timed-out
  Peon attempts and observation-count updates.
- API/serialization tests verify the camelCase diagnostics contract.
- Renderer source tests verify the block is inside the existing debug gate and
  renders the diagnostic fields.
- Existing Peon, session polling, and recommendation tests remain green.

## Scope boundary

This change instruments the existing Peon path only. It does not change
recommendation eligibility, add new recommendation types, alter provider
concurrency, or make Peon run more often.
