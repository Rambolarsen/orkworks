# Prompt attention authority for Claude Code, Copilot CLI, and Aider

## Status

Proposed for review through [#643](https://github.com/Rambolarsen/orkworks/issues/643).
The proposed product rule is recorded in [the MVP spec](../../../specs/orkworks-mvp.md#deterministic-harness-supplied-signals);
neither proposal is accepted until the review is complete.

## Scope

Review the event and Peon authority boundary for Claude Code, GitHub Copilot
CLI, and Aider. Codex and OpenCode keep their separately reviewed event
contracts. This document sets the signal bar and the implementation follow-up
split; it does not authorize broadening a harness's behavior from its
configuration alone.

The product distinction is between a completed turn, when the coding tool is
idle, and a direct prompt or permission decision that requires the user. A
notification that fires at every completed response does not prove the latter.

## Evidence reviewed

Installed tool versions on 2026-09-27:

| Harness | Installed version | Primary contract | Local live fixture |
| --- | --- | --- | --- |
| Claude Code | 2.1.274 | [Claude Code hooks reference](https://code.claude.com/docs/en/hooks) | Not captured |
| GitHub Copilot CLI | 1.0.83 | [GitHub Copilot hooks reference](https://docs.github.com/en/copilot/reference/hooks-reference) | Not captured |
| Aider | Not installed | [Aider notifications](https://aider.chat/docs/usage/notifications.html) | Unavailable |

The Claude and Copilot event references are primary but are not pinned to the
installed versions. The Aider notification page is also unversioned. These
documents establish what to probe, not a version-verified OrkWorks delivery
fixture. Keep the integration coverage **limited** until version-pinned payload
fixtures exist; mark behavior feature-probed only after the exact events are
exercised. No live prompt was opened during this review. Aider is absent here,
and the repository's task-scoped workflow does not allow launching another
coding harness from this session.

## Current implementation findings

- `ResolvedHarness::initial_work_hook_active` in
  `crates/orkworksd/src/harness/registry.rs` initializes the hook flag from an
  `Attention` capability, except for Codex and OpenCode. Among Claude, Copilot,
  and Aider, only Aider currently declares `Attention`; Claude and Copilot
  declare `NativeSessionId` only. The flag does not check whether Aider's
  workspace integration is installed, active, or has emitted an event.
- Aider's launch-time flag suppresses Peon's inferred `working` status and the
  hookless terminal-output `working` transition. It does **not** apply the
  Codex/OpenCode `NonPrompt` policy, so Peon can still infer
  `waiting_for_input`, `needsUserInput`, and prompt text. Claude and Copilot do
  not receive this static launch-time suppression; their current generic hook
  reports are described separately below. A single static flag therefore
  gives Aider incomplete work-state fallback without proving a hook ran.
- The shared Claude and Copilot hook reporter defaults attention to
  `waiting_for_input` and does not read `notification_type`. Claude installs a
  broad `Notification` matcher plus `PreToolUse` and `PostToolUse`; it does not
  install `UserPromptSubmit` or `Stop`. Its `PreToolUse` reporter writes
  `working` before the tool executes and before a permission decision. The
  shared attention POST carries neither an event name/source for Claude or
  Copilot nor a report-token header, so the proposed event validation and
  session-authentication boundary is not implemented. Copilot installs
  `notification` only.
- Claude's current `PostToolUse` entry is a synchronous `Write|Edit` plan-path
  reporter that deliberately skips generic attention. Do not add a generic
  successful-tool clear: `PostToolUse` includes `tool_use_id`, but
  `PermissionRequest` does not, so completion cannot be reliably matched to the
  permission wait; keep the path-only contract unchanged.
- Aider launch augmentation invokes the reporter when Aider finishes a
  response. That report currently says `waiting_for_input`, although the
  upstream contract describes ordinary response completion and provides no
  structured event payload or native Aider session ID. OrkWorks still correlates
  the reporter to its owning session through the inherited `ORKWORKS_SESSION_ID`
  and `ORKWORKS_PORT`; the missing native ID is not, by itself, a reason to
  reject a session-scoped report.

## Proposed event mapping

| Harness | Opens Needs You | Resolves or clears it | Normal turn | Evidence limits |
| --- | --- | --- | --- | --- |
| Claude Code | `Notification` with `permission_prompt`, `elicitation_dialog`, or `elicitation_url_dialog`. Do not use `PermissionRequest` alone: it runs before the permission flow, including cases where no user-facing prompt appears. | After prompt authority activates, `UserPromptSubmit` clears an earlier wait and marks work. Do not clear a permission wait from generic successful `PostToolUse`: its `tool_use_id` cannot be matched to `PermissionRequest`, which has no such ID. `Stop` is not a completion signal: another configured Stop hook can block stopping, so it must not report `idle` or clear a wait. `idle_prompt` may report `idle` after its documented delay; it does not clear an elicitation. `elicitation_response` and `elicitation_complete` clear an elicitation. `PreToolUse` must not write attention or clear a prompt. Accepted terminal input and session end also clear waits. Before authority activates, turn events do not write attention. | After activation, `UserPromptSubmit` means `working`; `Stop` is a no-op for attention; `idle_prompt` is a delayed idle hint. `PreToolUse` is not an attention or prompt-resolution event. | Permission notification is delayed about six seconds and may be omitted if the user resolves it first. `idle_prompt` fires about 60 seconds after a response, only if no typing occurred. Notification payloads have no event timestamp or turn ID; a delayed report may be stale and cannot be reliably rejected from documented fields alone. |
| GitHub Copilot CLI | `notification` with `permission_prompt` or `elicitation_dialog`. Ignore `agent_idle`, `agent_completed`, and shell-completion notifications for root-session Needs You. | After prompt authority activates, `userPromptSubmitted` clears any earlier wait and marks work. Do not clear a permission wait from generic successful `postToolUse`, whose payload has no invocation ID to correlate with the prompt. `agentStop` can be blocked by another configured hook and force continuation, so it does not report `idle` or clear a wait. Elicitation remains until accepted input or session end. A denied permission may remain waiting until another recognized event arrives. Before authority activates, turn events do not write attention. | After activation, `userPromptSubmitted` means `working`; `agentStop` is a no-op for attention. | `agent_idle` and `agent_completed` describe background subagents. Since `agentStop` may force continuation, no documented hook safely proves root-turn idle; `working` can persist until another authorized status event or session exit. Notifications are asynchronous and fire-and-forget. Order timestamped Copilot reports by event timestamp, reject older reports than the latest accepted event or committed terminal input, and use a stable receive sequence for ties. Missing or invalid timestamps cannot establish authority. Missing delivery can miss or stale a prompt. |
| Aider | No supported event proves an explicit prompt. Do not map its completion notification to `waiting_for_input`. | No prompt-resolution lifecycle exists. Do not write `idle` either: an agent-priority idle report could prevent Peon from recognizing a concrete conversational question. | The completion callback is associated with the owning OrkWorks session by launch environment, but only says the response ended; it does not distinguish a plain completion from an explicit question or carry a matching start/resolution event. | Keep attention writes disabled for this callback and remove Aider's launch-time static hook flag so Peon and terminal fallback remain available. This may miss questions Peon does not recognize, but avoids false waits and avoids suppressing Peon with an idle report. |

## Authority and fallback rule

For Claude and Copilot, an installed hook file or declared capability does not
establish prompt authority. Before prompt authority activates, turn events are
no-ops for attention: they must not write session-wide `agent` metadata, because
the current record-wide source priority would temporarily prevent Peon from
writing prompt fields even though prompt fallback is meant to remain active.
Accepted terminal input continues to clear waits through its existing
sidecar-owned transition. Peon and terminal fallback remain available for
prompt inference until the sidecar accepts a
recognized, session-correlated prompt notification: `permission_prompt`,
`elicitation_dialog`, or Claude's `elicitation_url_dialog`. Recognized
nonprompt notifications, unknown types, and malformed reports do not activate
authority or write a readiness-only state.

After that prompt-notification report, direct events own the attention fields:
`observed_status`/`attention`, `needsUserInput`, `detectedQuestion`, and
`suggestedOptions`. Promotion clears older Peon-sourced values in those fields;
normal source priority still protects user-authored values and newer accepted
direct reports. Peon continues summary, phase, diagnostics, and workflow
evidence, but cannot set or replace attention or prompt fields from
conversational text. This mirrors the existing `NonPrompt` boundary for
Codex/OpenCode while retaining descriptive Peon inference.

Authority lasts for the matching live session. A missing, disabled, or inactive
integration before the first prompt-notification report leaves prompt fallback active.
After promotion, explicit disable/uninstall or an integration reconciliation
that detects the owned notification hook is missing or drifted ends authority,
clears hook-owned attention/prompt fields subject to normal user-source
priority, and returns future attention inference to Peon/terminal fallback.
Silence alone cannot prove that a hook stopped, so authority remains active and
attention can go stale until a recognized event or session lifecycle transition.
Every Claude/Copilot attention report carries the immutable, per-live-session
generation issued by the sidecar at launch when the integration is enabled and
its owned prompt-notification hook is available. Otherwise the environment
variable is absent and reports are rejected. The sidecar passes it as
`ORKWORKS_PROMPT_HOOK_GENERATION`; reporters inherit it and forward
that exact value. A reporter captures it at invocation start and never fetches
the session's current generation while submitting a report. Disable,
uninstall, or detected drift revokes that generation before clearing
hook-owned fields. Reports from a revoked generation cannot reactivate
authority after demotion. Re-enabling does not change an already-running
harness's environment: that session remains on Peon/terminal fallback until it
is relaunched under a new OrkWorks live session with a fresh generation. Reports
without a generation are rejected. This generation fences lifecycle races but
does not authenticate the sending process.

Every Claude/Copilot native-ID registration and attention report must carry
the live OrkWorks session's valid report token and the immutable generation
inherited from that session's launch environment. The identity route accepts
the event's native ID (`session_id` for Claude, `sessionId` for Copilot) only
when both values match the live session and its unrevoked launch generation.
For each event the reporter intends to
send as attention, it must register that native ID through
`POST /sessions/:id/harness-session` first, then submit attention with the same
ID, token, and generation only after registration succeeds. Both requests use
`Authorization: Bearer <ORKWORKS_REPORT_TOKEN>`. This ordering also applies to
the first prompt notification; registration failure or rejection must stop
the attention request. Registration alone does not activate prompt authority.
The first successful registration binds that generation to one native ID;
later registrations with a different ID are rejected and cannot replace or
reset the binding. A different native ID requires a new OrkWorks live session
with a fresh generation. Missing, invalid, stale, unregistered, or mismatched
values are rejected without changing attention or authority.

The report token authenticates a report to its OrkWorks session, and the
generation fences disabled or stale integrations. Because child processes
inherit the token and may access the same session environment, these values do
not prove which operating-system process emitted an event or prevent a
same-session process from spoofing one. Treat the allowlisted
harness/event/status mapping and native-ID match as report consistency checks,
not cryptographic proof of event origin. Aider remains bound by its OrkWorks
launch identity because its callback supplies no native ID.

Although the Aider completion callback is correlated to its owning OrkWorks
session by launch environment, it never activates prompt authority or writes
attention state, including `idle`. Its completion signal is not a prompt and an
agent-priority idle report would prevent Peon from recognizing an explicit
question in the response. Peon remains the attention source. Remove the
launch-time static hook flag for Aider so absent or inactive integration does
not suppress working fallback.

## False-positive and false-negative tradeoffs

- Exact Claude/Copilot prompt event allowlists avoid treating ordinary turn
  completion, background agents, and authentication as prompts.
- Waiting for a delayed Claude permission notification avoids treating a
  pre-permission hook as proof that a visible prompt exists, but misses prompts
  resolved inside the notification delay and may briefly keep a stale wait
  after user action.
- Copilot notification loss and absent resolution events may leave Needs You
  stale until a later recognized event, accepted terminal input, or session
  lifecycle transition. `agentStop` cannot safely clear the wait or mark the
  session idle because another configured hook may block completion and force
  continuation. The documented `agent_idle`/`agent_completed` notifications
  are for background subagents, so a root turn may remain displayed as working
  after ordinary completion until another authorized status event or session
  exit. Reliable root-idle reporting needs a separately reviewed signal or
  turn-state channel.
- Claude's undated, delayed notification cannot be totally ordered against a
  later terminal input from the documented payload alone. A late prompt event
  may reopen stale Needs You; later accepted input, turn events, or lifecycle
  transitions clear it. The implementation and UI must preserve this known
  uncertainty rather than claim stale-event rejection.
- Keeping Peon active before a direct prompt notification handles missing,
  disabled, or broken integrations, but text inference can still create false
  positives until activation. Pre-activation turn reports are attention no-ops
  so record-wide agent priority cannot suppress that fallback.
- A `Stop` handler cannot establish that Claude actually stopped because a
  different configured Stop hook may block the stop. Ignoring it can leave the
  displayed turn status unchanged; the delayed `idle_prompt`, later accepted
  input, or session lifecycle provides subsequent evidence.
- After Claude/Copilot activation, refusing LLM-created prompt state prevents
  speculative Needs You but can miss conversational questions that have no
  direct harness event.
- Leaving Aider on Peon fallback can miss a concrete question. Treating its
  every completed response as Needs You is worse because ordinary completion
  is the common case and Aider has no matching clear event.

## Validation bar and open work

Before #643 is complete, capture exact payloads from pinned Claude and Copilot
versions and reproduce a real prompt, an ordinary completed turn, permission
approval and denial where supported, an unrecognized event, and a missing or
disabled integration. Verify these cases live where the installed harnesses
permit it. Aider has no prompt event to capture: its check is that enabling its
completion callback still leaves the Peon fallback in control, using a
reporter fixture and a real Aider run when available. The Aider installation is
not a prerequisite for reviewing this policy or splitting a narrowly scoped
implementation issue. Record live evidence as unavailable if the tool remains
absent; keep #643 open for the uncompleted real-prompt checks that can be run
with Claude/Copilot and for any later Aider evidence that becomes available.

For each integration, assert that an absent or inactive integration retains
Peon and terminal fallback; pre-activation turn events and nonprompt
notifications do not write attention; only an accepted same-session prompt
notification activates prompt authority. After activation, only allowlisted,
harness-mapped lifecycle events may update or clear attention, and they do not
activate authority. Unknown and malformed reports do not change state; missing,
unregistered, or mismatched native session IDs are rejected; the first ID
registered for a live generation remains immutable and attempts to rebind it
are rejected; a new live session/generation permits a new native ID; missing or
invalid tokens and revoked generations reject both identity registration and
attention without state changes; registering an identity alone does not
activate authority; and the first prompt reporter registers its ID before
sending attention, aborting attention if registration fails. Verify that token
and generation binding identify the OrkWorks session and active integration
only, with same-session process spoofing recorded as outside this protocol's
protection. Completion does not become Needs You; accepted input and specified
turn events clear prompts; unrelated or uncorrelated successful tool-completion
reports do not clear permission waits; reconciliation of a missing or drifted owned
notification hook demotes authority; Peon cannot write attention or prompt
fields after activation; and summary, phase, diagnostics, and workflow evidence
continue independently. For Copilot, timestamped out-of-order reports must not
change state; verify that an
older report is rejected after committed input, remains eligible after raw
uncommitted typing, and that only a committed-work transition advances the
input boundary. An active permission wait must clear on deterministic
single-key `CommittedWorking`; raw typing and single-key input must not clear
free-form elicitation before accepted submitted input, a mapped clear event, or
session end. An `agentStop` report must remain a no-op even when another
configured hook blocks completion and forces continuation. For Claude, verify
receipt-order handling and record the known late-notification case; do not
assert source-time stale-event rejection because the documented payload has no
event timestamp or turn ID. For both harnesses, verify that reports
from a revoked prompt-hook generation cannot restore authority after disable,
uninstall, or detected drift. Verify that existing harness processes retain
their revoked generation after re-enable, while a harness relaunched under a
new OrkWorks live session receives a fresh generation; reporters forward it
unchanged and must not fetch a replacement at POST time. An accepted terminal input is input that
the sidecar commits as work (the
existing Enter-terminated or deterministic single-key `CommittedWorking`
transition), not raw character typing or queued, unsent input.

Live prompt captures were not possible in this review: Claude/Copilot were not
launched, and Aider is not installed. These are open #643 acceptance checks;
the documentation update does not claim they are complete, and integration
coverage remains limited until fixtures and live behavior verify the mappings.

## Follow-up issue split after approval

1. Add per-session, event-validated Claude/Copilot prompt-channel promotion
   without using integration configuration or declared capability as proof
   that a hook executed. Keep pre-activation turn reports from writing the
   session-wide attention record so Peon fallback remains writable; only an
   accepted prompt notification promotes authority. Require session-token and
   unrevoked launch-generation validation when registering Claude/Copilot
   native IDs and when accepting attention. For each event, register the
   event's native ID
   first and send attention only after successful registration, including on
   the first prompt notification; identity registration alone never promotes
   authority. Require the attention report to match the same native ID, token,
   and immutable session-launch generation, reject attempts to rebind that
   generation to another native ID, validate the allowlisted event/status,
   reconcile missing or drifted notification entries, and clear the prompt
   tuple under the explicit record-wide source rule on demotion.
   Reject delayed reports from revoked generations. Document that inherited
   session credentials do not prove process origin or prevent same-session
   process spoofing. Re-enable issues fresh generations only to sessions
   relaunched under a new OrkWorks live session; reporters must never fetch the
   current generation at POST time. Nonprompt notifications do not write a
   readiness-only state.
2. Implement the Claude prompt, elicitation-clear, and turn event mapping with
   receipt-order behavior, permission clear signals, safe non-final `Stop`
   handling, and stale-state coverage. Preserve prompt kind so deterministic
   single-key `CommittedWorking` clears permission waits but not free-form
   elicitation; elicitation clears on accepted submitted input, a mapped clear
   event, or session end. Remove or disable its current `PreToolUse` attention
   write because it is neither an approval nor a prompt-resolution event.
3. Implement Copilot notification type filtering plus prompt and tool-resume
   mapping with async delivery and stale-state coverage. Order reports against
   committed input only, not raw terminal frames; preserve prompt kind so a
   deterministic single-key commit clears permission waits but not free-form
   elicitation. Do not use `agentStop` to mark idle because another configured
   hook can force continuation.
4. Correct Aider completion reporting so it never asserts Needs You and does
   not activate a static hook authority; preserve Peon fallback.

The four issues should be filed only after this policy is accepted in the MVP
spec. Any implementation that changes the session authority boundary must
record an ADR before code is written.
