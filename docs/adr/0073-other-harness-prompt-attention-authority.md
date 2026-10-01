# Prompt-attention authority is event-validated for Claude and Copilot; Aider stays on Peon fallback

- Status: accepted
- Deciders: repository owner
- Date: 2026-10-01

## Context

[ADR 0066](0066-hook-owned-prompt-attention.md) established hook-owned prompt
attention for Codex and OpenCode and left Claude Code, Copilot CLI, Aider, and
hookless tools on their prior Peon policy pending separate review
([#643](https://github.com/Rambolarsen/orkworks/issues/643)). The review of the
Claude, Copilot, and Aider boundary landed in
[PR #652](https://github.com/Rambolarsen/orkworks/pull/652), which merged the
reviewed policy into the MVP spec's
[deterministic harness-supplied signals](../../specs/orkworks-mvp.md#deterministic-harness-supplied-signals)
section and recorded the evidence in
[the other-harness attention review](../superpowers/specs/2026-09-27-other-harness-prompt-attention-design.md).

The current implementations do not yet meet that policy: the shared Claude and
Copilot reporter defaults every notification to `waiting_for_input` without
reading `notification_type` and carries no event name, report-token header, or
launch generation; Claude installs broad `PreToolUse` and `PostToolUse`
attention writes; and Aider declares a static launch-time `Attention` flag
that suppresses Peon's `working` fallback without proving any hook executed.
Until this boundary is recorded as a decision before the follow-up runtime
issues
([#681](https://github.com/Rambolarsen/orkworks/issues/681),
[#682](https://github.com/Rambolarsen/orkworks/issues/682),
[#683](https://github.com/Rambolarsen/orkworks/issues/683),
[#684](https://github.com/Rambolarsen/orkworks/issues/684)) change code, an
unreviewed event could turn an ordinary completed turn, a background subagent,
or a plain response completion into **Needs You**.

## Decision

Record the reviewed product rule as an accepted ADR before any follow-up
runtime behavior changes:

1. **Shared session-authority protocol (Claude and Copilot).** An installed
   hook file or declared capability never establishes prompt authority. Before
   activation, all turn events are attention no-ops — they must not write
   session-wide `agent` metadata, because record-wide source priority would
   otherwise suppress Peon prompt inference — and Peon plus terminal fallback
   remain available. Only an accepted, allowlisted prompt notification report
   for the live session promotes authority; after activation, only the
   harness's mapped turn and clear events may update or clear attention, and
   they never activate authority again. Unknown, malformed, nonprompt, and
   unauthorized reports do not change state.
2. **Validation and identity wall.** Every Claude/Copilot native-ID
   registration and attention report must carry the event's native ID
   (`session_id` for Claude, `sessionId` for Copilot), the live session's
   `ORKWORKS_REPORT_TOKEN`, and the immutable per-live-session launch
   generation (`ORKWORKS_PROMPT_HOOK_GENERATION`) issued only when the
   integration is enabled and its owned prompt-notification hook is available.
   Reporters register the native ID through
   `POST /sessions/:id/harness-session` before every attention POST and stop
   if registration fails; registration alone never activates authority. The
   sidecar owns the conversation epoch: the first successful registration
   binds one native ID, and rebinding is permitted only for an exact reset
   command declared in the persisted harness definition — Claude
   `SessionStart(source=clear)` or Copilot `sessionStart(source=new)` — under
   a reservation opened before PTY dispatch, with an early matching lifecycle
   report held until successful delivery acknowledgement binds it once.
   Ordinary registrations, startup, resume, fork, and unrecorded resets cannot
   rebind. The lifecycle registration's own event claim is not accepted
   report evidence; its event name and `source` travel in
   `sessionStartEvent`/`sessionStartSource` and must match the recorded reset.
3. **Promotion/demotion scope.** Authority covers exactly the four-field
   prompt tuple (`observed_status`/`attention`, `needsUserInput`,
   `detectedQuestion`, `suggestedOptions`). Metadata source is record-wide, so
   on promotion or demotion the sidecar clears the whole tuple when the
   record-wide source is not `user` and preserves the whole tuple when it is
   `user`; it never selectively retains fields by assumed per-field
   ownership. Peon continues summary, phase, diagnostics, and workflow
   evidence independently. Explicit disable/uninstall or reconciliation that
   detects the owned notification hook missing or drifted revokes the
   generation before demotion; silent inactivity does not. Reports from a
   revoked generation are rejected and cannot reactivate authority; re-enable
   does not alter an already-running harness's environment. Orphan recovery
   after a sidecar restart makes the old session non-live, so authority does
   not revive across restarts.
4. **Claude Code mapping.** `Notification` types `permission_prompt`,
   `elicitation_dialog`, and `elicitation_url_dialog` open Needs You.
   `PreToolUse` attention writes are removed. After activation,
   `UserPromptSubmit` clears an earlier wait and marks work;
   `idle_prompt` may set delayed idle but cannot clear an elicitation;
   `elicitation_response` and `elicitation_complete` clear only an
   unambiguous active elicitation and never clear permission waits; generic
   successful `PostToolUse` and `PermissionRequest` alone do not clear
   permission waits (their identifiers cannot be matched); and `Stop` is not a
   completion signal because another configured Stop hook can block stopping,
   so it must not report `idle` or clear a wait. Keep `agent_needs_input` a
   no-op until [#643](https://github.com/Rambolarsen/orkworks/issues/643)
   verifies its root-session correlation and a reviewed spec update changes
   the allowlist. Claude notifications carry no event timestamp or turn ID:
   process them in receipt order and preserve the documented late-notification
   stale-reopen risk instead of claiming stale-event rejection.
5. **Copilot CLI mapping.** `notification` types `permission_prompt` and
   `elicitation_dialog` open Needs You; `agent_idle`, `agent_completed`, and
   shell-completion notifications describe background work and are no-ops.
   After activation, `userPromptSubmitted` clears an earlier wait and marks
   work; generic successful `postToolUse` cannot clear a permission wait (no
   invocation ID to correlate); and `agentStop` never reports idle or clears a
   wait because another configured hook can block completion and force
   continuation. Denied permissions may remain waiting; the documented contract
   has no rejection event. Timestamped reports are ordered against accepted
   reports and committed terminal input: reports at or older than the
   committed-input boundary are rejected, raw uncommitted typing never
   advances that boundary, and equal timestamps use a deterministic receive
   sequence.
6. **Aider stays on Peon fallback.** Remove Aider's launch-time static
   `Attention` hook flag. Its completion callback never activates prompt
   authority and never writes attention state, including `idle` or
   `waiting_for_input`: either agent-priority status could suppress Peon's
   ability to recognize a concrete question. Accepted terminal input and the
   sidecar's committed-input transitions continue to clear existing waits
   through their existing ownership.
7. **Prompt kind is preserved.** Deterministic single-key `CommittedWorking`
   may clear a permission wait; raw typing or a single unsubmitted key must
   not clear free-form elicitation. Accepted submitted input, a mapped clear
   event, or session end clears elicitation.

Product behavior detail lives in the MVP spec and harness integration
contract concept; this ADR is the architecture record, not a behavioral
superset.

## Consequences

- Exact allowlists avoid treating ordinary turn completion, background
  subagents, and authentication prompts as Needs You, but prompts can be
  missed: delayed Claude notifications may be resolved inside the delay, lost
  Copilot notifications may leave stale waits until the next recognized event
  or committed input, and Aider questions Peon does not recognize go unseen.
- Claude/Copilot idle is often unobservable: Claude `Stop` and Copilot
  `agentStop` are non-final, so `working` can persist after an ordinary
  completion until another authorized event, accepted input, or session exit;
  reliable root-idle reporting stays a separate signal-design gap.
- The token and generation bind reports to the live OrkWorks session and fence
  disabled or stale integrations, but they authenticate session capability,
  not process origin: any same-session process inheriting them can spoof a
  report, including a lifecycle claim during a reset reservation. ADR 0066's
  Codex/OpenCode decision and scope are unchanged; this record covers only the
  Claude, Copilot, and Aider boundary and follows [#652](https://github.com/Rambolarsen/orkworks/pull/652)'s
  review.
- Coverage remains limited until version-pinned fixtures and live prompt,
  clear, denial, and fallback checks are recorded under #643; the follow-up
  implementation issues keep their recorded evidence gates, and synthetic
  fixtures do not close those gates.
