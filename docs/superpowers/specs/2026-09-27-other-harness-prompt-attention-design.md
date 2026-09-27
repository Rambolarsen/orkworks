# Prompt attention authority for Claude Code, Copilot CLI, and Aider

## Status

Proposed for review through [#643](https://github.com/Rambolarsen/orkworks/issues/643).
The normative product rule is in [the MVP spec](../../../specs/orkworks-mvp.md#deterministic-harness-supplied-signals).

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
  attention capability for Claude, Copilot, and Aider. It does not check
  whether the workspace integration is installed, active, or has emitted an
  event.
- That launch-time flag suppresses Peon's inferred `working` status and the
  hookless terminal-output `working` transition. It does **not** apply the
  Codex/OpenCode `NonPrompt` policy to these harnesses, so Peon can still infer
  `waiting_for_input`, `needsUserInput`, and prompt text. A single static flag
  therefore gives incomplete work-state fallback without proving a hook ran.
- The shared Claude and Copilot hook reporter defaults attention to
  `waiting_for_input` and does not read `notification_type`. Claude installs a
  broad `Notification` matcher plus `PreToolUse` and `PostToolUse`; it does not
  install `UserPromptSubmit` or `Stop`. Copilot installs `notification` only.
- Aider launch augmentation invokes the reporter when Aider finishes a
  response. That report currently says `waiting_for_input`, although the
  upstream contract describes ordinary response completion and provides no
  event payload or native session ID.

## Reviewed event mapping

| Harness | Opens Needs You | Resolves or clears it | Normal turn | Evidence limits |
| --- | --- | --- | --- | --- |
| Claude Code | `Notification` with `permission_prompt`, `elicitation_dialog`, or `elicitation_url_dialog`. Do not use `PermissionRequest` alone: it runs before the permission flow, including cases where no user-facing prompt appears. | `elicitation_response` and `elicitation_complete` clear their elicitation prompt. A permission decision has no paired resolution event; clear on later `UserPromptSubmit`, `PreToolUse` after allowed work resumes, `Stop`, accepted terminal input, or session lifecycle. | `UserPromptSubmit` and `PreToolUse` mean `working`; `Stop` means `idle`. `idle_prompt` is a delayed completion notification, not Needs You. | Permission notification is delayed about six seconds and may be omitted if the user resolves it first. The documented notification payload has no event timestamp, so reporter ordering against terminal input needs an explicit rule. Missing clear events can leave stale attention. |
| GitHub Copilot CLI | `notification` with `permission_prompt` or `elicitation_dialog`. Ignore `agent_idle`, `agent_completed`, and shell-completion notifications for root-session Needs You. | No matching resolution notification is documented. `userPromptSubmitted`, `postToolUse` after an approved tool, `agentStop`, accepted terminal input, or session lifecycle clear the wait. A denied permission may remain waiting until the agent stops or another recognized event arrives. | `userPromptSubmitted` means `working`; `agentStop` means `idle`. | Notifications are asynchronous and fire-and-forget. Payloads contain `sessionId` and millisecond timestamp; event ordering must use that timestamp with stale-event guards. Missing delivery can miss or stale a prompt. |
| Aider | No supported event proves an explicit prompt. Do not map its completion notification to `waiting_for_input`. | No prompt-resolution lifecycle exists. | A response-completed notification may describe `idle`, but it must not suppress Peon inference for a concrete question or authorize attention globally. | No native session ID or event payload is documented. Peon remains the attention fallback even when the optional notification command is installed. This can miss a question Peon does not recognize; mapping every completion to Needs You creates a false positive on every ordinary turn. |

## Authority and fallback rule

For Claude and Copilot, an installed hook file or declared capability does not
establish authority. The sidecar promotes only the matching live session after
accepting a recognized, session-correlated direct event with the expected
harness and status. Before then, Peon and terminal fallback remain available
for work and attention. Once promoted, direct events own attention for that
session; Peon still contributes summary, phase, diagnostics, and workflow
evidence, but cannot set or replace Needs You from conversational text. A
future implementation must define event provenance, report-token validation,
native session correlation, and timestamp ordering before it activates this
authority.

Aider never activates prompt authority from the completion notification. Since
the notification has no native session ID or matching lifecycle, it must not
write attention state, including `idle`. Peon remains the attention source.
The implementation must also remove the launch-time static hook flag for Aider
so absent or inactive integration does not suppress working fallback.

## False-positive and false-negative tradeoffs

- Exact Claude/Copilot prompt event allowlists avoid treating ordinary turn
  completion, background agents, and authentication as prompts.
- Waiting for a delayed Claude permission notification avoids treating a
  pre-permission hook as proof that a visible prompt exists, but misses prompts
  resolved inside the notification delay and may briefly keep a stale wait
  after user action.
- Copilot notification loss and absent resolution events may leave Needs You
  stale until a later recognized turn event, accepted terminal input, or
  session lifecycle transition.
- Keeping Peon active before a direct hook event handles missing, disabled, or
  broken integrations, but text inference can still create false positives
  until the first accepted event.
- After Claude/Copilot activation, refusing LLM-created prompt state prevents
  speculative Needs You but can miss conversational questions that have no
  direct harness event.
- Leaving Aider on Peon fallback can miss a concrete question. Treating its
  every completed response as Needs You is worse because ordinary completion
  is the common case and Aider has no matching clear event.

## Validation bar and open work

Before implementation is split into harness issues, capture exact payloads
from pinned Claude and Copilot versions and add Aider fixture evidence when
available. Verify an explicit real prompt, an ordinary completed turn, a
permission approval and denial where supported, an unrecognized event, and a
missing or disabled integration for each harness. Assert that missing or
inactive integrations retain Peon and terminal fallback, only a recognized
same-session event activates authority, completion does not become Needs You,
accepted input and turn events clear prompts, and descriptive Peon updates
continue after activation.

Live prompt captures were not possible in this review: Claude/Copilot were not
launched, and Aider is not installed. The documentation update therefore does
not claim these acceptance checks are complete; integration coverage remains
limited until fixtures and live behavior verify the mappings.

## Follow-up issue split after approval

1. Add per-session, event-validated Claude/Copilot authority promotion without
   using integration configuration or declared capability as proof that a
   hook executed.
2. Implement the Claude prompt, elicitation-clear, and turn event mapping with
   event ordering and stale-state coverage.
3. Implement Copilot notification type filtering plus prompt, tool-resume, and
   turn event mapping with async delivery and stale-state coverage.
4. Correct Aider completion reporting so it never asserts Needs You and does
   not activate a static hook authority; preserve Peon fallback.

The four issues should be filed only after this policy is accepted in the MVP
spec. Any implementation that changes the session authority boundary must
record an ADR before code is written.
