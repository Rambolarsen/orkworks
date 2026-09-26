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

This design changes the OrkWorks-owned OpenCode reporter and the sidecar's
attention-source policy for Codex and OpenCode, plus focused behavioral coverage
and integration documentation. Peon continues to read terminal output for
summaries and diagnostics. Its reading of conversational questions must not
create **Needs You** for either of these hook-capable harnesses. The existing
attention route and shared stale-event guard remain. Before implementation,
record the revised authority boundary in an ADR. Codex queued questions have
a different direct-signal gap and are tracked in
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

## Sidecar attention authority

For Codex and OpenCode, a conversational question in captured terminal output
is not evidence that the coding tool is currently prompting for input. Peon
must not set `waiting_for_input`, `needsUserInput`, `detectedQuestion`, or
`suggestedOptions` for either harness from an LLM inference alone. An accepted,
session-scoped permission or explicit-question lifecycle event may set Needs
You. A future deterministic terminal prompt recognizer would require its own
reviewed contract; none is part of this design. While a session has no accepted
attention hook event, Peon may still supply nonprompt observed status
and descriptive fields. Thus installed hook files or a registry attention
capability alone never assert that a hook executed. When hooks are absent,
unapproved, or broken, a real prompt may be missed until a direct signal is
available; the UI must not replace that uncertainty with an inferred Needs You.
If a live session already carries Peon-sourced `waiting_for_input` when this
policy is applied, the next attention reconciliation clears that status and
its prompt fields to unknown unless a newer direct event or user override has
arrived. It must not relabel the old inference as Idle or leave Needs You
latched simply because the next Peon result contains no status.

Once a valid attention hook event is accepted for a live Codex or OpenCode
session, that session's hook stream owns its reported attention state. Peon
continues to update summary, phase, diagnostics, and workflow evidence, but
cannot replace hook-owned `waiting_for_input`, `working`, or `idle` or its
prompt fields after the ordinary staleness window. A later accepted hook event,
user status override, accepted terminal input transition, or session lifecycle
transition can change attention through its existing rules. Hook authority is
per session, not inherited from another session or from an integration setting.
It is retained across a later `process` transition caused by accepted user
input, as Codex already does. User-sourced status retains its higher priority.

Codex already promotes a live session only after validating an owned hook
event's bundle fingerprint, harness, and event type. OpenCode currently starts
with `active_work_hook` from its registry capability and accepts ordinary
`agent` attention POSTs; neither proves that its plugin ran. Add a distinct
per-session observed-hook state for OpenCode. Its reporter marks attention
reports as OpenCode hook reports and carries the session's existing
`ORKWORKS_REPORT_TOKEN`. The sidecar accepts that hook provenance and activates
its authority only when the token, live session, harness, and event/status
contract match; a generic `agent` POST, debug injection, installed plugin file,
or capability flag cannot activate it. Keep the existing local reporter trust
boundary: the token proves possession of the session capability, not the
identity of code inside the OpenCode process. Other attention reporters retain
their present route behavior. Generalize the Peon-preserving merge helper so
it retains each harness's actual provenance instead of writing `codex_hook`
for OpenCode.

The OpenCode report includes `source: "opencode_hook"`, the originating event
name, and `Authorization: Bearer <ORKWORKS_REPORT_TOKEN>`. Only
`session.created` may report the initial `idle`; `session.status` with `busy`
may report `working`; `session.idle` may report `idle`; a permission or question
ask may report `waiting_for_input`; and a reply/rejection may report the
remaining effective state (`waiting_for_input`, `working`, or `idle`). The
sidecar rejects a claimed OpenCode hook report whose event/status pair is
outside that contract. It stores the report at the existing agent-priority
tier while retaining the validated hook provenance needed for later Peon
merges. The reporter posts no attention for a duplicate `session.created`.

OpenCode prompt authority depends on delivery of the matching reply/reject
report. If that report is lost while its process remains alive, Needs You can
remain stale; Peon cannot resolve a request ID reliably. The reporter sends
the latest effective state on later recognized events, accepted terminal input
can advance attention, and session death clears live attention through the
existing lifecycle. A plugin reload while a request remains open cannot
reconstruct the request from `session.created` alone. The supported plugin
client does not expose the pending-question and pending-permission list methods
in its [legacy SDK surface](https://github.com/anomalyco/opencode/blob/v1.18.32/packages/sdk/js/src/gen/sdk.gen.ts),
so this design does not claim restart recovery. Codex's hook bundle does not
report queued-question opening or resolution; [#632](https://github.com/Rambolarsen/orkworks/issues/632)
must establish a direct signal for those prompts. This design does not make a
Peon guess a substitute for that signal.

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
and out-of-order HTTP delivery. Sidecar tests should prove Peon cannot create
Needs You from chat prose for Codex or OpenCode, can still provide nonprompt
fallback before a hook executes, and cannot overwrite any hook-owned attention
state afterward. Cover Codex's existing validated-hook path, OpenCode's token
and event validation, accepted terminal input, user overrides, separate
sessions, existing Peon-sourced waiting state, and other harnesses' unchanged
authority. Peon should still update descriptive fields. The existing Rust
integration test should continue to assert that
the packaged reporter is the owned source script. A live OpenCode smoke check
is useful when available, but source-contract and sequence tests do not
upgrade the integration's live coverage label.

## Explicit non-goals

This change does not infer Needs You from general terminal text, treat an
ordinary idle turn as a prompt, give one session authority over another, or
change Codex's hook event contract. It does not make OrkWorks answer prompts
or approve permissions. It does not guarantee recovery from lost OpenCode
lifecycle events or a plugin reload with an already pending prompt. Claude
Code, Copilot CLI, Aider, and hookless tools keep their current Peon policy;
their event coverage must be reviewed separately before applying this rule.
