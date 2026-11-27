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
The user submits the instruction through ordinary terminal input. This sends
no PTY data, adds no sidecar authority, and uses the existing Resume flow.

## Proposed prompt actions

Show the live actions in the selected session's Terminal header, with a compact
menu where space is limited. Availability comes from the sidecar. Explain
unavailability when the session is working, awaiting input, disconnected, or
lacks a qualified prompt-ready signal. Where a session needs the user, its
existing prompt and permission controls remain the way to respond.

| Session state | Action | Result |
| --- | --- | --- |
| Live, at a qualified empty prompt | **Start new task** | Open a composer bound to this session. Submit one user-written instruction in the same conversation. After PTY delivery, adopt the reviewed task title if automatic naming is active. |
| Live, at a qualified empty prompt | **Keep going** | Send the fixed prompt “Keep going with the current task.” once on click. Keep the existing title. |
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
generation. The server supplies Keep going's text and owns the
qualified literal-input and submit sequence. The Start new task payload is the instruction and reviewed title.
The sidecar rejects other fields, empty or oversized text, controls, embedded
line separators, and leading slash or bang command prefixes after trimming.
Qualification must also cover active popup and keymap behavior. Harness hooks and
agent reporting credentials cannot invoke this operation.

Reuse the bounded terminal delivery path and writer acknowledgement.
Revalidate workspace, runtime, connection, and prompt readiness at admission.
The runtime owner checks and claims the ready generation at the last
sidecar-controlled point before the first write. Order that decision against
competing input and already-received prompt or output transitions. An event
source that cannot be ordered with dispatch cannot grant readiness.

A ready generation permits at most one write sequence across both actions.
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

Qualify an initial Codex integration before enabling either live action.
Retain evidence for a ready prompt, drafts, modal and permission waits, a
pending follow-up, disconnect, and stale queued input. In real Codex,
verify exactly one root submission for both actions with short, long, and
Unicode text. A successful PTY write alone establishes byte delivery, so the
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
input target. A first-party client that already owns the same App Server
connection can investigate a typed turn submission through the
[App Server API](https://learn.chatgpt.com/docs/app-server#start-a-turn).
That API does not establish readiness for an independently running CLI
editor. Reopening or attaching another client to the live CLI thread is
outside this proposal. Resume remains available via its existing route.

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
