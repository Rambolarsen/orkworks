---
type: Design
title: Task actions in an existing session
description: Proposed user-approved task submission and continuation with exact runtime and title ownership.
tags: [orkworks, desktop, sessions, terminal]
status: draft
---

# Task actions in an existing session

Proposed for [#2140](https://github.com/Rambolarsen/orkworks/issues/2140).
Approval of this design is required before product implementation. The
[MVP](../../../specs/orkworks-mvp.md) already specifies Resume and stable
session titles. The two proposed live actions extend the approved terminal
input boundary in [ADR 0034](../../adr/0034-user-approved-session-review-prompt.md).

## First release

The owner chose to proceed with a title-only Start new task action after the
Codex source check. It uses the existing desktop title operation to save an
explicitly user-owned title and returns focus to the same live terminal.
The user submits the instruction through ordinary terminal input. Copy
continuation prompt copies the fixed continuation text for the user to paste.
Ended sessions use the existing Resume flow. Start new task and Copy
continuation prompt use existing desktop authority and send no terminal input.

## Proposed prompt actions

Show the live actions in the selected session's Terminal header, with a compact
menu where space is limited. Availability comes from the sidecar. Explain
unavailability when the session is working, awaiting input, disconnected, or
lacks a qualified prompt-ready signal. Where a session needs the user, its
existing prompt and permission controls remain the way to respond.

| Session state | Action | Result |
| --- | --- | --- |
| Live, at a qualified empty prompt | **Start new task** | Open a composer bound to this session. Submit one user-written instruction in the same conversation. After PTY delivery, adopt the reviewed task title if automatic naming is active. |
| Live, with qualified native admission | **Keep going** | One click asks the current conversation to start a turn with “Keep going with the current task.” The native owner accepts only if the conversation is idle and ready at admission. Keep the existing title. |
| Ended, with an available resume strategy | **Resume** | Use the existing Resume chooser and its exact-identity and fallback confirmation rules. Opening the conversation sends no prompt. |

Start new task accepts one plain-text line of up to 4,096 UTF-8 bytes after
trimming. Show its first 100 Unicode scalar values as a visible, editable
title suggestion. When a manual title is active, show that it will be kept
and omit the title input. Titles use the existing 1–100 scalar, control-free
desktop validation. The explicit Submit button is the delivery authorization.
Escape, selection changes, and workspace changes cancel an unsubmitted
composer. Opening the composer or reopening a conversation has no delivery
effect.

## Delivery authority

Electron main owns the selected-terminal binding and uses its existing
desktop credential and sidecar generation. Preload exposes only the two
named actions. Requests are bound to the expected canonical workspace,
explicit session ID, exact runtime, and a sidecar-issued prompt-ready
generation. Harness hooks and agent reporting credentials cannot invoke
these operations.

The following terminal delivery rules apply to the proposed prompt-bearing
Start new task action. Its payload contains the instruction and reviewed
title. The sidecar rejects other fields, empty or oversized text, controls,
embedded line separators, and leading slash or bang command prefixes after
trimming. Qualification must cover active popup and keymap behavior. Reuse
the bounded terminal delivery path and writer acknowledgement.
Revalidate workspace, runtime, connection, and prompt readiness at admission.
The runtime owner checks and claims the ready generation at the last
sidecar-controlled point before the first write. Order that decision against
competing input and already-received prompt or output transitions. An event
source that cannot be ordered with dispatch cannot grant readiness.

A ready generation permits at most one admitted action, consumed by the
first successful claim. Each adapter must also order its final admission
against ordinary input at the native owner.
If the editor needs separate paste and Enter writes, keep competing writes
out of that sequence and revalidate the same editor target and exclusive draft
before sending Enter. The startup editor path shows why the distinction matters
for Codex paste-burst handling. An interruption can leave a draft. Stop before
Enter when the target or draft changes, and let the user inspect or clear it.

The action owner survives requester cancellation through delivery and title
settlement. Distinguish rejection before input, partial or uncertain delivery,
and confirmed full delivery. A partial, uncertain, or confirmed dispatch
consumes the claim. A future action needs a freshly qualified ready generation.
Never replay a prompt after a timeout, lost response, or title failure. Surface
confirmed delivery and any title failure separately. Temporary delivery
authority is discarded with the runtime.

## Prompt readiness and rollout

The proposed capability requires evidence of a live, connected coding tool at
its root conversation's empty prompt. Busy, a buffered draft, a modal or
permission wait, unknown state, child activity, and an unqualified editor
each make the action unavailable. Invalidate ready evidence on accepted
terminal edits, input admission, a prompt transition, or runtime replacement.
An authenticated Stop/idle report describes turn state. Qualifying a coding
tool also requires evidence that its input editor is ready and empty for the
same runtime. Peon output inference alone cannot grant prompt-write authority.

Qualify an initial Codex integration before enabling a live action.
Retain evidence for a ready prompt, drafts, modal and permission waits, a
pending follow-up, disconnect, and stale queued input. For Start new task,
verify exactly one root submission with short, long, and Unicode text in
real Codex. A successful PTY write establishes byte delivery, so the
qualification must check the editor outcome. Ordinary terminal
typing and existing Resume remain available when the new capability is
unavailable. If the existing observations cannot establish readiness at the
dispatch boundary, keep the live actions unshipped for that integration and
record the missing signal before proposing any new observation source.

### Source check for Codex CLI 0.172.0

The currently installed CLI reports version 0.172.0. Its tagged
[composer source](https://github.com/openai/codex/blob/3d86e43b075adeba2193ec5c22ac92f55afd82e6/codex-rs/tui/src/bottom_pane/chat_composer.rs#L1248)
checks editor text, shell mode and attachments for emptiness. The
[bottom-pane dispatcher](https://github.com/openai/codex/blob/3d86e43b075adeba2193ec5c22ac92f55afd82e6/codex-rs/tui/src/bottom_pane/mod.rs#L949-L1022)
can route input to a modal view. These checks are local to the TUI. Leading
bang can start shell mode in the normal composer. The literal input sequence
must avoid that mode and other command or popup dispatch.

[Codex's hook contract](https://learn.chatgpt.com/docs/hooks#stop) allows a
Stop hook to request continuation. OrkWorks currently maps Stop to idle in
its reporter and forwards turn/attention fields, with no empty-editor
attestation. Its initial-editor handling is only constructed for launch
with a supplied initial prompt. Neither mechanism grants live action
readiness. An idle conversation with an unsent draft and one with an empty
editor can expose the same accepted hook state.

**Qualification result:** Existing Codex CLI signals are insufficient for
either live action. This was a source check of the installed version.
Live submission remains untested. A future CLI solution needs a supported,
orderable editor signal or a native action that atomically validates the
input target. Reopening or attaching another client to the live CLI thread
is outside this proposal. Resume remains available via its existing route.

## Atomic admission for Keep going

Keep going requires an operation on a native client already owned by the
selected runtime. The sidecar supplies the fixed text and accepts no prompt
text or native address from the renderer. An explicit click authorizes one
attempt bound to its application window, workspace, session, live runtime,
native root conversation, and readiness generation. Main validates the
current selection and its authenticated sidecar connection. The sidecar
orders local revocation and dispatch under the same runtime owner. A
workspace or selection change, lost connection, or retired runtime that
revokes the binding before dispatch prevents the send. After dispatch the
request stays bound to the original target. Closing the UI or cancelling a
request after dispatch cannot retract it. The native owner determines
whether to admit that request, and the desktop displays its outcome only
for the original binding.

The sidecar issues a local one-action claim bound to an authoritative
readiness version from the native owner. The native owner advances its
version when editor or turn readiness changes, even if the editor
returns to the same empty, idle state before the operation arrives. A
candidate must prove that the native operation compares the bound version
at admission. A sidecar observation counter alone cannot provide this check.

The native owner checks and starts the new turn in one operation. At that
point the exact conversation and native readiness version must still match. A
busy conversation, pending turn or queued follow-up, user input or permission
wait, nonempty draft, active modal, or unknown state rejects the attempt.
The owner serializes this decision with competing starts and editor state
changes. Rejection leaves the editor, input queue, and running turn untouched.
Success starts one new root turn with the fixed literal prompt. No
continuation work survives a rejection. Keep going leaves the task title
unchanged and never answers a question or permission request.

Correlate each attempt with an identifier scoped to the original binding.
A second click or duplicate request for the same readiness generation
cannot start another turn. A trustworthy, correlated native receipt is
needed to show **Started**. A definite rejection may offer a fresh explicit
click after new readiness is established. A missing or ambiguous
acknowledgement shows that delivery could not be confirmed. Keep automated
submission unavailable for that runtime until the original owner returns an
authoritative outcome for the attempt or the runtime retires. A valid late
receipt from that owner settles the original attempt. Receipt inspection is
read-only. A reconnect, timeout, automatic retry, or later
availability report cannot replay the original click. Receipts from an old
runtime cannot affect a replacement runtime. Record only the outcome and
nonsecret correlation data in OrkWorks diagnostics.

### Qualification and status

The first proposed candidate is a typed exact-thread call, or an equivalent
native editor action, that atomically checks readiness and starts a turn
without a preceding paste or queue write. In Codex 0.172.0, [turn/start can
steer an active turn](https://github.com/openai/codex/blob/3d86e43b075adeba2193ec5c22ac92f55afd82e6/codex-rs/app-server/src/request_processors/turn_processor.rs#L719-L767).
The queue service has an idle-only start for a selected queued item, but
[adding the item persists it and can wake dispatch](https://github.com/openai/codex/blob/3d86e43b075adeba2193ec5c22ac92f55afd82e6/codex-rs/ext/queue/src/service.rs#L275-L291).
The [busy-path test](https://github.com/openai/codex/blob/3d86e43b075adeba2193ec5c22ac92f55afd82e6/codex-rs/app-server/tests/suite/v2/thread_queue.rs#L786-L858)
shows the rejected start leaving an item that later executes. OrkWorks'
[passive Codex client](../../adr/0076-codex-owned-native-approval-observer.md)
provides neither a qualified submission operation nor production listener
ownership. These source checks establish the gap for the inspected version.

| Qualification case | Required observation |
| --- | --- |
| Exact ready root and explicit click | One native turn with the literal continuation and the original title. |
| Competing start, queued follow-up, draft, modal, or permission wait first | Rejection with no change to editor, queue, or active turn and no later execution. |
| Selection or workspace changes and revokes the claim before local dispatch | No native request. After dispatch, a selection change cannot redirect the request or publish its result into the new selection. |
| Stale native readiness, including a draft that appeared and cleared, wrong root, or retired native runtime | No turn. The request cannot target a replacement. |
| Double click, duplicate request, or lost acknowledgement | At most one native turn. A lost acknowledgement allows read-only receipt inspection. A matching late receipt settles the attempt without replay. |

Review the contract before selecting a candidate. Pin the native operation
and version, prove the listener and runtime ownership binding, and validate
the table with repeatable fixtures before enabling the control. A future
native qualification plan must name the exact candidate and get its own
authorization. Until then, Copy continuation prompt is the available
continuation action for live idle sessions.

## Title ownership

The submitted title is display data. After confirmed delivery, record an
explicit task title through the session application and invalidate prior title
work. Fence the commit to the original runtime and conversation generation
and the captured desktop title revision. A manual title or intervening title
action takes precedence. Use the
existing terminal-input provenance and an additive, default-false task-title
marker in the current session record. In a marked session, native title
reports and Peon leave the reviewed title in place. A later Start new task
can replace it. An explicit return to automatic naming or a confirmed
conversation reset clears the marker and restores the existing title policy.
Exact Resume preserves the task title. A successful fallback Resume clears
its task-title ownership when it takes ownership of the new conversation.
Rollback of an unsuccessful fallback preserves the prior title and marker.

This precedence keeps the old Codex conversation name from replacing a
reviewed title for new work in that conversation. Records from earlier
versions default to the existing behavior. Older readers ignore the marker
and can apply their ordinary automatic title policy after downgrade. Use the
current production store and its transaction owner. If the ordinary SQLite
store is production-selected before implementation, recheck the storage
contract before adding a second path. The reviewed title is the only new
durable user text for these actions. Retain ordinary input activity
bookkeeping but bypass generic input-text and title inference for both
actions. Keep going also preserves placeholder titles. Instruction text does
not enter OrkWorks session metadata, Peon title hints, or action diagnostics.
