---
type: Design Proposal
title: Prevent false Codex permission waits with a native approval-route signal
description: Record the original approval-route proposal and the preferred upstream human-approval lifecycle follow-up.
tags: [orkworks, codex, hooks, attention, upstream]
status: draft
---

# Prevent false Codex permission waits with a native approval-route signal

Status: original route-field proposal not adopted; prefer the human-approval
lifecycle follow-up below. No runtime implementation is authorized by this document.
The owner requested a revised design after the human-run hook failure tests.
Tracks [#690](https://github.com/Rambolarsen/orkworks/issues/690).
The original preventative proposal is retained below as history. This follow-up
does not amend an accepted runtime contract, activate a capability, or qualify
the installed binary.

## Follow-up: observe actual human approval requests

On 2026-10-25 the owner accepted the recommendation to pursue a narrow upstream
human-approval lifecycle instead of further completion-queue work or a predicted
exclusive-automatic route. The current scope is the proposal update, not runtime activation or adoption
of another project's full design. Further upstream communication requires
explicit approval of the message and any project attribution.
The original proposal below is retained as history; its route-field issue draft
must not be submitted as the current recommendation.

Existing upstream requests already cover the gap:

- [openai/codex#39426](https://github.com/openai/codex/issues/39426) requests an
  output-only human-input lifecycle with opaque request identity and resolution.
- [openai/codex#28833](https://github.com/openai/codex/issues/28833) describes the
  same false notifications during automatic review; its comments include a
  second terminal manager's identical PermissionRequest-to-waiting mapping.
- [openai/codex#19921](https://github.com/openai/codex/issues/19921) requests
  external notifications for approval and question prompts.

Track #39426 without opening a duplicate. A project-specific comment was
removed by the owner; do not repost it. The preferred technical question is
whether upstream supports an observation-only projection of the existing native
approval request/resolution lifecycle. Do not prescribe a new transport, fork
Codex, or implement guessed hook names. The minimum useful semantics are:

1. Emit pending only once human approval is actually required, after automatic
   routing. Automatic allow/deny produces no human-pending event; an automatic
   fallback to human approval does.
2. Pair pending and resolution with an opaque request identity and enough native
   session/source identity to separate concurrent requests and process restarts.
   Scope includes the native root/child attribution contract; the parent session
   ID alone must not be assumed to distinguish child requests.
3. Resolve when that request ceases to need a human decision, including decline,
   cancellation and native cleanup, before an accepted long tool completes.
   Resolution means no longer pending, never human approval, tool success or
   session-wide working. Resolving A must leave pending B intact.
4. Keep the surface passive and content-minimal: no approval-control responses,
   approval decisions, command text, prompt content, user answers or credentials. Observer failure must not
   change native approval policy or tool execution.
5. Ask how missed events, reconnect and startup are represented. Paired IDs alone
   do not guarantee complete delivery. Without a documented fresh authoritative
   baseline, gaps/disconnects/restarts are unknown, not evidence that a wait ended.

The source-loss default in #39426 is not adopted: clearing an observer's own
visual notification on EOF is different from clearing OrkWorks' persisted
attention tuple. Likewise, broader questions/heartbeat/transport proposals in
that issue are upstream discussion, not new OrkWorks implementation scope.
Existing attention ownership and competing-writer fences remain binding.

### Smaller existing alternatives and limits

[Codex TUI notifications](https://learn.chatgpt.com/docs/config-file/config-advanced#notify-vs-tuinotifications)
support an approval-requested filter, OSC 9 output, and notification_condition
set to always. At the pinned 0.164.0 commit,
[tool request handlers](https://github.com/openai/codex/blob/6aaac694803004a63c6f393d153c96d22521a6b9/codex-rs/tui/src/chatwidget/tool_requests.rs)
emit notifications for command/edit approvals and MCP elicitation, while the
explicit request_permissions handler does not emit one.
[Notification coalescing](https://github.com/openai/codex/blob/6aaac694803004a63c6f393d153c96d22521a6b9/codex-rs/tui/src/chatwidget/notifications.rs)
and [OSC output](https://github.com/openai/codex/blob/6aaac694803004a63c6f393d153c96d22521a6b9/codex-rs/tui/src/notifications/osc9.rs)
do not supply paired identity/resolution. Terminal bytes are not an authenticated
native status channel. These are source findings, not a live qualification;
notifications remain a possible limited experiment, not an authoritative fix.

The [app-server protocol](https://learn.chatgpt.com/docs/app-server#approvals)
already exposes request identity and serverRequest/resolved. Its use by an
independent observer remains subject to ADR 0076's connection and launch gates.
The [Unix-socket alternative](2026-10-15-codex-listener-ownership-design.md#evidence-and-alternatives)
was already rejected because it does not retain exclusion of same-user tool
processes from native approval control; it is not an unexamined shortcut.

### Delivery boundary

After upstream identifies a supported contract, review a concrete OrkWorks
consumer design before implementation. Qualification must cover real manual
holds, automatic decisions and fallback, overlapping requests, long tools,
source loss and existing attention ownership. No native/completion capability,
API-only launch marker, installed hook or session state changes in this update.
#690 remains open. No claim is made that upstream has accepted or shipped this
contract. The historical acceptance/review sections below apply only to the
original route-field proposal.

## Original route-field proposal (historical)

## Problem and evidence

Codex PermissionRequest currently means an approval workflow is starting; it
can start automatic review without a human prompt. OrkWorks immediately writes
Needs You. Long automatic tools can retain that false wait throughout execution.

The human-started macOS arm64 Codex 0.164.0 experiment passed serial manual
hold/approval/completion. Injected exit-1 and timeout failures before source
publication both let the native tool execute without its pre-event. The queue
remained contiguous and unpoisoned. No late pre-event was observed after an
additional explicit ten-second timeout observation. These are bounded native
experiments using an injected wrapper, not claims that the normal reporter
routinely fails or that all timed-out descendants are killed.
[Sanitized issue evidence](https://github.com/Rambolarsen/orkworks/issues/690#issuecomment-6387330708).

Replays through the merged pure reducer reject the observed isolated missing-pre
posts. A synthetic incomplete-stream counterexample still permits a clear:
A pre is recorded; overlapping same-tool B loses its pre; ID-less Permission B
is associated with A; Post A offers a clear while B remains waiting. This is an
unqualified-input counterexample, not an observed production clear. The dormant
queue and completion application have no production caller. More publication
counters cannot prove native callbacks that never reached the publisher.

Pinned installed-source audit: annotated tag `rust-v0.164.0`
`fe3b29455488c2e189e43ad5526a107900c5e1b7` resolves to commit
`6aaac694803004a63c6f393d153c96d22521a6b9`. The experimental binary SHA-256 is
`d8990e3ee7acfbc26f91265b12a1a83f0fee40d0e0f8f1f40a37d4ec21f499cf`.

- [PermissionRequest request type](https://github.com/openai/codex/blob/6aaac694803004a63c6f393d153c96d22521a6b9/codex-rs/hooks/src/events/permission_request.rs#L35-L48)
  has no public invocation or reviewer-route field. Its internal run suffix is
  not a documented payload guarantee.
- [Approval dispatch](https://github.com/openai/codex/blob/6aaac694803004a63c6f393d153c96d22521a6b9/codex-rs/core/src/tools/approvals.rs#L454-L590)
  runs permission hooks first, then reviewer approval; a missing automatic
  decision can select the user route. MCP action configuration can override
  the review context. A global `approvals_reviewer` setting is insufficient.
- [Review decision host](https://github.com/openai/codex/blob/6aaac694803004a63c6f393d153c96d22521a6b9/codex-rs/core/src/guardian/decision.rs#L44-L121)
  documents `None` as requesting the existing user flow and combines effective
  requirements, model policy, extensions and cancellation. Do not label the
  request automatic by copying one configuration field.
- [Official hooks documentation](https://learn.chatgpt.com/docs/hooks)
  documents default synchronous hooks and fail-open pre-tool error behavior.
  [App-server documentation](https://learn.chatgpt.com/docs/app-server)
  provides explicit item/request identities; it does not establish security
  of the existing native TUI connection.

These source observations do not qualify a distributed installer or a future
upstream contract. The installed 0.164.0 has no proposed signal.

## Options and recommendation

| Option | Result and dependency |
| --- | --- |
| More hook ordering/timeout inference | Reject: cannot prove absent callbacks or safely correlate an ID-less permission under hidden overlap. |
| Add only a permission invocation ID | Useful upstream improvement, but insufficient by itself: another missing permission callback could conceal a separate held prompt. It does not certify absence of pending prompts. |
| Use the owned native observer | Retain the accepted long-term path. Native item/request identity and pending state address the signal problem; all-client connection ownership, initialization and exact-version/runtime gates remain unresolved. |
| Native exclusive-automatic route signal; suppress only its new wait | Recommended next upstream proposal. It prevents this false wait at its source and never clears or replaces existing attention. Requires a new upstream semantic guarantee, not a local configuration heuristic. |

The recommendation is the owner's implementer judgement, not a previously
approved protocol. It targets future source-qualified launches; there is no
safe retrofit claimed for installed 0.164.0.

## Proposed upstream contract

Conceptual new PermissionRequest input field:

```json
{"approval_route": "automatic"}
```

Spelling is proposed and requires upstream agreement. Values are `automatic`,
`user`, or `unknown`; absence is equivalent to unknown for consumers.

`automatic` is a native engine commitment for this particular approval request:
all remaining outcomes are machine allow/deny/cancel/error, and none can enter
or fall back to a human approval prompt. It does not mean approved, successful,
finished, idle, or safe. It cannot be supplied by tool input, hook output,
user configuration, command-line interpolation, or an integration override.

Compute it from the fully resolved action and routing policy, including strict
review requirements, model rules, MCP-specific reviewer selection, contributor
availability and every fallback branch. Freeze the decision for that request.
If an extension can return no decision and route to a human, or the engine
cannot commit before invoking permission hooks, emit `unknown`. Do not change
review/denial behavior merely to manufacture the automatic value. It is valid
for no existing route to qualify until upstream can establish this contract.

`user` means human routing remains possible, not that a prompt definitely
appeared: other policy hooks or caches may still resolve it. `unknown` makes no
promise. Both retain today's conservative OrkWorks behavior. Later configuration
changes apply to later requests. A request marked automatic cannot silently
change route after a retry or configuration refresh; a retry with possible human
routing is a new request with its own hook input. The old automatic request
must be terminal before such a retry. Upstream review must establish this
lifecycle; it is not an assumed property of 0.164.0.

Keep all existing PermissionRequest policy hooks and their allow/deny semantics.
This field is informational, cannot approve/deny a tool, and is not a new hook
controller. A hook's error/timeout behavior stays native policy. Do not add
full inputs, outputs, commands, transcripts or credentials to diagnostic capture.

## Proposed OrkWorks behavior

Only a source-owned exact version/platform/protocol entry with installed-native
behavioral evidence may interpret `automatic`. The first candidate scope is
macOS arm64. Qualification cannot be enabled by user overrides, hook arguments,
ambient environment, a config checkbox, a version lower-bound or field presence.
Unknown versions always use existing behavior. The table starts empty.

After normal native-identity, bundle-trust and report acceptance checks:

| Accepted PermissionRequest | Attention operation |
| --- | --- |
| Qualified exact native contract, strict scalar `approval_route == automatic` | Do not send/apply this event's waiting-for-input attention mutation. |
| `user`, `unknown`, missing, malformed, duplicated, unsupported, or unqualified field | Use the existing accepted PermissionRequest wait behavior. |
| Invalid ordinary identity/report | Use existing rejection behavior; the new field cannot grant authority. |

An automatic event is an attention no-op: no working/idle write, prompt-field
clear, ownership-token advance, authority promotion or demotion. It cannot
resolve an earlier manual permission, explicit question, user override or child
prompt. Identity-only and existing bounded diagnostics may continue under their
current contracts; they cannot smuggle an attention mutation into this path.
All existing trusted input, turn and lifecycle transitions remain separate.

Thus a late automatic report cannot clear a newer prompt; a missing automatic
callback cannot introduce a new false-clear path. Simultaneous automatic child
work cannot clear a parent/other-child manual wait. This needs no inference
that the observed stream is complete or that no other request exists. Normal
manual callback loss remains an existing limitation, not a reliability claim
removed by this proposal. No failure or silence synthesizes working.

The coordinated-writer requirements remain binding on native/completion clears.
This no-mutation path does not enable either clear mechanism or advertise an
API-only launch marker, so it does not waive #789 or borrow its qualification.
Legacy agent reporting and all other attention writers remain as before.
A pre-existing false wait may remain until an existing authorized transition.

## Upstream issue draft

Title: Expose a conservative per-request no-human-approval route in PermissionRequest input

PermissionRequest fires before both automatic and human approval workflows.
A status observer cannot tell whether a user prompt is possible. Treating every
request as human input reports false waits during automatic review; treating a
global auto-review setting as authoritative misses per-action routing/fallback.

Could the native hook input expose an engine-owned `approval_route` whose
`automatic` value guarantees no human approval prompt or fallback for that
particular request? Missing/unknown should preserve conservative behavior.
The signal must describe routing, never grant approval or imply completion.
If the engine cannot make that commitment before policy hooks run, report
unknown rather than predict the reviewer. Policy hooks and reviewer behavior
must remain unchanged. Proposed acceptance cases are listed below.

Evidence: 0.164.0 permission request type and approval dispatcher above; real
manual hold succeeds, while injected pre-publication hook error/timeout can
continue the tool without a complete source stream. A downstream consumer can
use the proposed automatic classification solely to avoid creating its own
false wait, without clearing another prompt or reconstructing native order.

Questions for upstream: Is a non-predictive exclusive route available at this
boundary? Which built-in actions can guarantee it? What request/retry lifecycle
prevents later human fallback? If none can, should a supported independent
native status contract be used instead? No existing public support is assumed.

This is a draft for owner review. No upstream message, issue or PR was sent.

## Acceptance and delivery

1. Owner reviews this proposal and separately authorizes upstream communication.
   Post the scoped question only after that authorization. Do not implement a
   guessed field in OrkWorks or fork/distribute Codex under this proposal.
2. Upstream confirms a realizable routing contract, actual schema/version and
   retry/fallback semantics. If it cannot commit before the hook boundary,
   stop this path and retain the separately gated observer route. Do not add
   another speculative queue or payload classifier as a substitute.
3. After contract acceptance, amend MVP/ADR 0051 and canonical harness/runtime
   instructions, then write the scoped implementation plan. Runtime work needs
   owner approval of that concrete design; material upstream revisions return
   for review. This document alone is not implementation authorization.
4. Upstream tests must cover human, automatic allow/deny/error/cancel, unsupported
   contributors, per-MCP reviewer overrides, model/managed requirements, cached
   decisions, permission-hook error/timeout, retry, configuration changes and a
   long running tool. Any path
   reaching human input cannot carry automatic. Tests must exercise the real
   dispatcher, not only serialization or a mocked chosen reviewer.
5. OrkWorks tests prove zero attention/ownership mutation for automatic in working,
   already-waiting, question/options, user-owned and overlapping child/manual
   states; strict fallbacks and old-version behavior; duplicate JSON keys tested at the
   actual raw report parser before map decoding; delayed/duplicate events;
   unchanged policy/identity/capture and installer reconciliation behavior.
6. Qualify the exact installed upstream artifact and effective configuration in
   a human-owned diagnostic. Observe manual holds and automatic long tools;
   include fallback and simultaneous manual/automatic cases. Hook trust stays
   human controlled. Enable only that source-owned entry after review/CI pass.

## Review routing and open risks

Owner is the sole writer in an isolated worktree. One round, at most two fresh
read-only lower-cost reviewers (three active agents including owner): one checks
source claims/contract consistency, the other asks builder questions and hunts
missing cases. No worker edits or upstream communication. Owner reconciles the
results before handoff; unchanged artifacts are not repeatedly re-reviewed.

The main uncertainty is whether upstream can make a route commitment before
hooks without changing approvals. That is a feasibility gate, not deferred
implementation detail. A second risk is overbroad version qualification; only
exact installed evidence can create a row. Existing direct launches remain
available, and the new consumer can be disabled for future events without
rewriting any wait. There is no metadata migration or replay recovery here.

The accepted observer and completion designs remain intact and dormant under
their own gates. #763 is currently closed as issue tracking, but its linked
client-identity proposal and code still lack qualifying all-client proof; issue
closure is not startup authorization. Current 0.164.0 TCP connector still sends
a bearer through URL-based connect without that expected-instance parameter:
[source](https://github.com/openai/codex/blob/6aaac694803004a63c6f393d153c96d22521a6b9/codex-rs/app-server-client/src/remote.rs#L719-L783).
No native process, conversation, or installed diagnostic was launched for this
read-only design audit. #690 remains unresolved.


## Historical route-field review disposition

The independent builder check found no important accidental omissions. It
confirmed that exclusive-route feasibility and retry/configuration lifecycle
are explicit upstream gates, not assumptions a local builder may fill in.
The separate source/contract reviewer checked the pinned request/dispatch/
reviewer code and the accepted native/completion contracts, and found no
consequential contradiction in the no-mutation proposal.

Their two bounded clarifications are incorporated in acceptance: exercise
permission-hook error/timeout at the real dispatcher, and test duplicate-key
fallback at the raw reporter parser before map decoding. No scope or runtime
gate changed. Neither reviewer ran an installed Codex session or approved
upstream support. Documentation checks and PR lifecycle are owner-executed.
