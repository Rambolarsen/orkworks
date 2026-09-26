# OpenCode prompt attention lifecycle

Status: proposed; written-spec review required before implementation
Date: 2026-09-26
Tracking: [#631](https://github.com/Rambolarsen/orkworks/issues/631)

## Purpose and scope

An OrkWorks session should show **Needs You** while its captured OpenCode
session has an unresolved permission request or explicit question. Ordinary
turn completion remains **Idle**, and active work remains **Working**. The
current reporter handles permission asks but ignores question events; later
busy/idle events can also clear a pending permission. Its timestamp formatter
adds three zero digits to millisecond time, so two rapid reports can have the
same `observedAt` and the sidecar discards the later one as stale.

This design changes only the OrkWorks-owned OpenCode reporter and its focused
behavioral coverage and integration documentation. It retains the existing
session-scoped attention route, metadata priority, and shared sidecar ordering
rule. Codex queued questions have a different signal gap and are tracked in
[#632](https://github.com/Rambolarsen/orkworks/issues/632).

## Verified event contract

The supported OpenCode 1.18.18 schema and the locally installed 1.18.32 schema
both define `permission.asked` and `question.asked` with `id` and `sessionID`.
`permission.replied`, `question.replied`, and `question.rejected` carry
`requestID` and `sessionID`. The 1.18.32 question service publishes these
events when it opens, answers, or dismisses a question. Its event bridge
forwards the event data as plugin event `properties`.

Sources: [OpenCode 1.18.18 question schema](https://github.com/anomalyco/opencode/blob/v1.18.18/packages/schema/src/v1/question.ts),
[OpenCode 1.18.32 question schema](https://github.com/anomalyco/opencode/blob/v1.18.32/packages/schema/src/v1/question.ts),
[permission schema](https://github.com/anomalyco/opencode/blob/v1.18.32/packages/schema/src/v1/permission.ts),
[question publisher](https://github.com/anomalyco/opencode/blob/v1.18.32/packages/opencode/src/question/index.ts),
and [event bridge](https://github.com/anomalyco/opencode/blob/v1.18.32/packages/opencode/src/event-v2-bridge.ts).

The reporter continues to capture `event.properties.info.id` from
`session.created` and ignores attention events whose `sessionID` differs from
that captured ID. A new captured session resets the reporter's in-memory
attention state and reports `working` to clear a prompt left by the previous
captured session. The first capture only establishes identity. A duplicate
`session.created` for the same ID does not reset or report attention.

## Reporter state and projection

The reporter maintains three in-memory values for its captured session:

- Latest turn state: `working` initially; `session.status` with `status.type`
  `busy` sets `working`, and `session.idle` sets `idle`.
- Pending permission IDs and pending question IDs, in separate sets. An ask
  adds its `id`; a matching reply or rejection removes its `requestID`.
- Last reported effective state, initially unknown. The first valid attention
  event reports even when its effective state is the initial `working` value.
- Last emitted logical microsecond timestamp.

The effective attention state is `waiting_for_input` whenever either pending
set is nonempty. Otherwise it is the latest turn state. The reporter posts an
attention update when that effective state changes. A new ask that leaves it
already waiting does not need another post. Busy or idle events update the
underlying turn state while a prompt is pending, but cannot clear Needs You.
Resolving the last pending request posts the current underlying turn state;
resolving one of several leaves Needs You in place. An unknown or duplicate
resolution does not change state.

Only events with the captured `sessionID` participate. Asks without a
nonempty string `id` and resolutions without a nonempty string `requestID`
are ignored; malformed events cannot create an unresolvable pending prompt or
clear a real one. Request IDs are qualified by type so a permission and a
question cannot cancel each other even if their raw IDs coincide. The
reporter's state is intentionally process-local: on OpenCode restart, the
existing session restoration and later event flow establish fresh state; this
change does not introduce persistence or query OpenCode's pending queues.

The `waiting_for_input` message identifies whether OpenCode asks for a
permission decision or a question answer. The message is a generic prompt
description, not the question text or permission detail.

## Timestamp and delivery ordering

For each emitted update, calculate integer microseconds as the greater of
`Date.now() * 1000` and `lastEmittedMicroseconds + 1`. Format that logical time
as UTC with exactly six fractional digits. This makes successive posts
strictly ordered even when callbacks run in the same wall-clock millisecond,
and preserves the sidecar's `observedAt <= last_hook_attention_at` stale-event
guard. If asynchronous HTTP requests arrive out of order, that guard keeps
the newer state. Failed POSTs retain their place in the logical sequence;
later reports still have newer timestamps. Report failures remain best effort,
as in the existing reporter.

## Boundaries and validation

No UI or shared Rust attention-policy change is required. The integration
installer continues to publish the stable OrkWorks-owned reporter; an existing
OpenCode integration must be reconciled through Settings to receive new bytes.
The documentation should describe the newly verified question coverage and
state that OpenCode events outside the supported event contract remain
unverified.

Behavioral coverage should drive the change using event sequences for ordinary
turn completion, permission ask/reply, question ask/reply/reject, overlapping
requests, busy/idle while a request is pending, foreign/malformed events,
same-millisecond ordering, and out-of-order HTTP delivery. The existing Rust
integration test should continue to assert that the packaged reporter is the
owned source script. A live OpenCode smoke check is useful when available,
but the event-contract and sequence tests are the deterministic acceptance
evidence.

## Explicit non-goals

This change does not infer Needs You from general terminal text, treat an
ordinary idle turn as a prompt, broaden cross-session attention authority,
or change Codex's hook contract. It does not make OrkWorks answer prompts or
approve permissions.
