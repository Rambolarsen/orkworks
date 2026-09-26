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

This design changes the OrkWorks-owned OpenCode reporter and a narrow sidecar
Peon merge rule, plus focused behavioral coverage and integration documentation.
It retains the existing attention route and its shared stale-event guard.
Before implementation, record the new OpenCode prompt-authority rule in an ADR
because it changes the attention ownership boundary. Codex queued questions
have a different signal gap and are tracked in
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
attention state and reports `idle` to clear a prompt left by the previous
captured session without claiming that the new session is working. A later
`session.status: busy` reports `working`. A duplicate `session.created` for
the same ID does not reset or report attention.

## Reporter state and projection

The reporter maintains four in-memory values for its captured session:

- Latest turn state: `idle` initially; `session.status` with `status.type`
  `busy` sets `working`, and `session.idle` sets `idle`.
- Pending permission IDs and pending question IDs, in separate sets. An ask
  adds its `id`; a matching reply or rejection removes its `requestID`.
- Last reported effective status and message, initially unknown. The first
  valid attention event reports even when its effective state is `idle`.
- Last emitted logical microsecond timestamp.

The effective attention state is `waiting_for_input` whenever either pending
set is nonempty. Otherwise it is the latest turn state. The reporter posts an
attention update when the effective status **or message** changes. A question
alone says OpenCode is asking for an answer; a permission alone says it needs a
permission decision; both pending types use a generic message saying OpenCode
needs an answer or permission decision. Adding or resolving a request reports
the new message even when the status remains `waiting_for_input`. Busy or idle
events update the underlying turn state while a prompt is pending, but cannot
clear Needs You. Resolving the last pending request posts the current turn
state with no prompt message. An unknown or duplicate resolution changes
neither the pending sets nor the reported state.

Only events with the captured `sessionID` participate. Asks without a
nonempty string `id` and resolutions without a nonempty string `requestID`
are ignored; malformed events cannot create an unresolvable pending prompt or
clear a real one. Request IDs are qualified by type so a permission and a
question cannot cancel each other even if their raw IDs coincide. The
reporter's state is intentionally process-local. Prompt messages are generic;
they contain neither question text nor permission details.

## Sidecar authority while a prompt is pending

The current source-priority rule permits Peon to overwrite an `agent` status
after 15 seconds. That can clear a real OpenCode prompt while its reporter is
quiet. For a live, active OpenCode session whose resolved harness has an
attention capability (`active_work_hook` is true) and whose current observed
status is `agent`-sourced `waiting_for_input`, the sidecar preserves that status
during Peon merges. Peon may still update summary, phase, and diagnostics, but
not the status or prompt fields. The reporter emits
`waiting_for_input` only while at least one recognized request ID is pending.
The next accepted `agent` attention report of `working` or `idle` for that
OpenCode session removes this protection. This rule applies to no other harness
and does not protect OpenCode's ordinary `working` or `idle` status from the
existing Peon fallback.
User-sourced status retains its current higher priority.

The existing Peon-preserving merge helper is Codex-specific internally: it
writes `codex_hook` provenance. Generalize that helper's preserved source as
part of this change, keep Codex's behavior unchanged, and retain `agent`
provenance for OpenCode. The rule relies on the same local attention-report
trust boundary as the current OpenCode integration: `agent` provenance and an
attention capability do not attest that the installed plugin sent a particular
POST. It must not be extended to other harnesses or other
OpenCode statuses by inference.

This protection depends on delivery of the matching reply/reject report. If
that report is lost while the OpenCode process remains alive, Needs You can
remain stale; Peon cannot resolve an OpenCode request ID reliably. The reporter
will report the latest state on a later recognized event, and session death
clears live attention through the existing lifecycle. A plugin reload while a
request remains open cannot reconstruct the request from `session.created`
alone. The supported plugin client does not expose the pending-question and
pending-permission list methods in its
[legacy SDK surface](https://github.com/anomalyco/opencode/blob/v1.18.32/packages/sdk/js/src/gen/sdk.gen.ts),
so this design does not claim restart recovery. These cases remain explicit
limitations rather than inferred prompt resolutions.

## Timestamp and delivery ordering

For each emitted update, take high-resolution Unix time from
`performance.timeOrigin + performance.now()`, convert it to integer
microseconds, and use the greater of that value and
`lastEmittedMicroseconds + 1`. Format the result as UTC with exactly six
fractional digits. Node and Bun implement both Performance APIs. Unlike
padding `Date.now()` with zeroes, this retains the sub-millisecond event time
needed by the sidecar's separate `observedAt <= accepted_input_at` guard when
terminal input and a new prompt occur in the same millisecond. The logical
increment also preserves the `observedAt <= last_hook_attention_at` guard for
rapid event sequences. If asynchronous HTTP requests arrive out of order, the
later event's timestamp wins. Failed POSTs retain their place in the logical
sequence; later reports still have newer timestamps. Report failures remain
best effort, as in the existing reporter. Cross-process clock comparison can
still be affected by a system clock step; this design does not claim that the
timestamp alone proves causality across processes.

Clock references: [Bun Performance support](https://bun.com/docs/runtime/nodejs-compat),
[Node Performance timing](https://nodejs.org/api/perf_hooks.html).

## Boundaries and validation

No UI change is required. The integration installer continues to publish the
stable OrkWorks-owned reporter; an existing OpenCode integration must be
reconciled through Settings to receive new bytes. The integration contract
must retain **limited** coverage: the versioned schemas, publisher code, and
reporter sequence tests establish the event mapping, but a live OpenCode
process has not yet confirmed end-to-end attention delivery. Events outside
the supported contract remain unverified.

Behavioral coverage should drive the change using event sequences for ordinary
turn completion, permission ask/reply, question ask/reply/reject, overlapping
requests and message changes, busy/idle while a request is pending,
foreign/malformed events, same-millisecond ordering across terminal input,
and out-of-order HTTP delivery. Sidecar tests should prove Peon cannot clear a
live OpenCode prompt, can still update its descriptive fields, and can overwrite
ordinary OpenCode `working`/`idle` after the normal staleness window. They
should also prove other harnesses and user overrides retain their current
authority. The existing Rust integration test should continue to assert that
the packaged reporter is the owned source script. A live OpenCode smoke check
is useful when available, but source-contract and sequence tests do not
upgrade the integration's live coverage label.

## Explicit non-goals

This change does not infer Needs You from general terminal text, treat an
ordinary idle turn as a prompt, give one OpenCode session authority over
another, or change Codex's hook contract. It does not make OrkWorks answer
prompts or approve permissions. It does not guarantee recovery from lost
OpenCode lifecycle events or a plugin reload with an already pending prompt.
