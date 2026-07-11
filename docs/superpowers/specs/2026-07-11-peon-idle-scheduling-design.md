# Peon idle scheduling design

## Goal

Prevent Peon from repeatedly invoking a provider for an idle session. After
Peon reaches `idle`, it must remain dormant until OrkWorks records new user
input for that session. This avoids keeping an expensive local provider, such
as Ollama, resident while preserving the session's terminal history and
runtime.

## Scope

This design changes ongoing per-session Peon scheduling only. It does not
change PTY lifetime, terminal-output persistence, session lifecycle, metadata
priority, or repository-level Peon behavior. The bounded final Peon scan when
a session exits remains in place.

## Scheduling model

The session Peon scheduler exposes one ephemeral state for every live session:

| State | Meaning | Transition out |
| --- | --- | --- |
| `waiting_for_output` | No inference is currently requested. | Terminal output or recorded user input requests observation. |
| `debouncing` | An observation is waiting for output to settle. | Start inference after the configured debounce interval. |
| `inferring` | A provider inference is in flight. | Apply its result, failure, or timeout. |
| `idle_waiting_for_user_input` | Peon has classified the session as idle. It is unscheduled. | A user-input event only. |
| `final_scan` | A bounded observation runs while the session exits. | Completion or the existing final-scan timeout. |

`idle_waiting_for_user_input` is a scheduling hold, not a session lifecycle or
attention state. The session remains live or running according to its runtime;
its existing `observedStatus: idle` continues to describe the observed state.

## Event rules

1. Every transition to effective `observedStatus: idle` sets
   `idle_waiting_for_user_input` and removes that session from normal periodic
   scheduling. This includes a persisted idle inference and the
   `PEON_IDLE_TIMEOUT` timer path. An idle inference that the metadata-priority
   rules reject does not create a hold.
2. While held, terminal output continues to be drained and persisted exactly
   as it is today. That output must not schedule Peon or clear the hold.
3. Every completed, non-sensitive user-input line recorded by OrkWorks clears
   the hold and moves the scheduler to `debouncing`, including short input that
   is not eligible to update the descriptive label. Partial, control-sequence,
   and sensitive input are not qualifying events. Repeated qualifying input
   coalesces into one pending observation while debouncing or inferring.
4. Outside an idle hold, new terminal output and qualifying user input both
   move `waiting_for_output` to `debouncing`. Existing debounce configuration
   remains the delay; descriptive input keeps its existing immediate hint
   behavior. The latest retained user-input hint is included in the next
   inference context.
5. A non-idle success, failed inference, or timeout returns to
   `waiting_for_output` using the existing retry cadence.
6. On session exit, scheduling first atomically cancels pending normal work,
   prevents an already-running normal inference from persisting after the
   lifecycle becomes `ending`, and then performs the existing bounded
   finalization behavior. If the final terminal snapshot is nonempty, it may
   make one provider scan; otherwise the existing fallback applies. This does
   not restart ongoing observation.

The scheduler must not use a periodic retry or terminal-output event to revive
an idle-held session. A user-input event is the sole revival signal.

## Debug visibility

The scheduler state is runtime-only: it is not written to session metadata,
event logs, or workspace settings. The sidecar always includes an optional
`peonSchedulerState` field in its live-session DTO; the sidecar does not know
or enforce the renderer's debug preference.

The existing Settings → Debug → “Show debug metadata” toggle gates rendering
of this field. When enabled, Session Details displays a `Peon scheduler` value
using the state names above. When disabled, it is omitted from the UI; persisted
metadata remains unchanged.

When a session finalizes, is forgotten, or is deleted by retention, all of its
Peon scheduler state and related runtime-map entries are removed. `final_scan`
is used only while a provider scan is actually in flight for an `ending`
session.

## Acceptance criteria

- An idle session, including one made idle by `PEON_IDLE_TIMEOUT`, makes no
  additional provider calls as time passes.
- New terminal output for an idle-held session is persisted but does not invoke
  Peon or alter the hold.
- Only completed, non-sensitive user input resumes ongoing session Peon
  observation after idle; short input does so, while partial/control/sensitive
  input does not.
- Exiting during a normal inference produces no post-finalization metadata
  mutation and follows one bounded finalization path.
- An idle-held exit preserves existing bounded finalization: it makes one
  provider scan only when the final snapshot is nonempty, otherwise it uses
  the existing fallback.
- The live session API exposes optional `peonSchedulerState` without
  persisting it, and the UI displays it only when debug metadata is enabled.
- Tests cover each state transition, including no-repeat behavior with a real
  `active` lifecycle phase, input coalescing, runtime-state cleanup, and
  debug-off/debug-on rendering, so the idle hold is not accidentally bypassed.
