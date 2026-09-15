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
- an optional reason: an eligibility reason when the state is `idle`,
  `candidate`, or no diagnostic state has been established; the failure
  reason when `failed`; or a terminal outcome (`inference_succeeded`) when
  `completed`. Always absent while `in_flight`;
- the last attempt timestamp;
- the last successful inference timestamp;
- the last provider ID and model, when present;
- the last provider fallback step and attempt count, when present;
- the last bounded provider/error summary, when present; and
- the number of workflow observations persisted for the session, or an
  unavailable/null value if that store read fails.

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

## State semantics and transitions

- `idle`: no Peon attempt is currently pending. The optional reason identifies
  known cases such as `disabled`, `no_new_silent_output`, `not_active`, or
  `waiting_for_retry`.
- `candidate`: the scheduler has selected the session and is preparing an
  attempt. This state is written when the candidate is selected, before the
  provider task is spawned.
- `in_flight`: provider inference is running. This state is written when the
  provider task starts and includes an attempt generation/identity internally
  so late completions cannot overwrite a newer attempt.
- `completed`: the most recent attempt returned a valid provider result;
  `lastSuccessfulInferenceAt` is updated even when no workflow observation
  was emitted, and the reason is set to the terminal outcome
  `inference_succeeded` rather than left absent.
- `failed`: the most recent attempt failed, timed out, or returned unusable
  output; the error summary is retained for diagnosis. A timeout becomes
  failed immediately, releases the scheduler lease, and any detached provider
  completion is ignored if its attempt identity is stale. Once the scheduler
  leaves `failed` for `idle` or `candidate`, the failure's error summary,
  provider ID/model, and fallback step are cleared so they don't linger
  against an unrelated later state; a `completed` snapshot's provider
  ID/model/fallback step are preserved through a later `idle` instead, since
  they describe the last successful attempt rather than a resolved failure.

The scheduler updates these fields at candidate selection, provider start,
provider completion, provider failure, and timeout. A session that has never
been considered has no established state and exposes a reason rather than
pretending that it completed successfully. The snapshot is current runtime
state and is not persisted across a sidecar restart; the existing persisted
`peonLastInference` remains the historical successful-inference signal.

The observation count is obtained through a per-session observation-store query
that counts only durably accepted workflow observations. Duplicate reports do
not increase it. A store read failure leaves the count null and does not fail
the session-list request.

## Error handling and privacy

Diagnostics are best-effort metadata. A missing snapshot must not block session
creation, session polling, Peon inference, or recommendation queries. Provider
error summaries must use the existing bounded/sanitized error-summary path and
must not expose prompts, terminal transcripts, credentials, or raw model
output.

`peonDiagnostics` is an optional/null field in the session API so partially
initialized sessions and mixed-version callers remain readable.

The renderer must not display the block unless the existing debug metadata
toggle is enabled. Toggling the setting should affect the block through the
same `showDebugMetadata` prop already used by the other debug fields.

## Testing

- Sidecar tests verify state transitions for successful, failed, and timed-out
  Peon attempts and observation-count updates.
- A scheduler test holds two fake provider calls open and verifies two distinct
  sessions enter `in_flight` before either call completes; this guards against
  accidental global serialization.
- API/serialization tests verify the camelCase diagnostics contract.
- Renderer source tests verify the block is inside the existing debug gate and
  renders the diagnostic fields.
- Existing Peon, session polling, and recommendation tests remain green.

## Scope boundary

This change instruments the existing Peon path only. It does not change
recommendation eligibility, add new recommendation types, alter provider
concurrency, or make Peon run more often.
