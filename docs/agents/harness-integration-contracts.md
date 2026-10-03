---
type: Integration Reference
title: Harness integration contracts
description: Evidence requirements and local signal contracts for coding-tool integrations.
tags: [harnesses, integrations, hooks, signals, evidence]
status: stable
---

# Harness integration contracts

This evidence register is the implementation gate for compiled harness signal
and integration bindings. It was retrieved on 2026-07-23 (Codex row
re-verified 2026-08-02, superseding that review's negative finding — Codex
now documents a stable `hooks.json` schema; OpenCode row re-verified
2026-08-18 against the published `@opencode-ai/plugin`/`@opencode-ai/sdk`
type declarations, superseding that review's negative finding — OpenCode's
plugin export shape and session ID field are now pinned; OpenCode's
attention-event mapping re-verified 2026-09-02 against the published event
list, superseding that row's "session ID only" scope — issue #104). A row marked
**limited** or **unsupported** must not be promoted until its missing exact
fixture and version/feature evidence are added beside the binding.

| Harness | Primary evidence | Local configuration / event contract | Status and no-op rule |
| --- | --- | --- | --- |
| Claude Code 2.1.274 (installed) | [Hooks reference](https://code.claude.com/docs/en/hooks) | `.claude/settings.local.json` is local/non-shareable. The documented `Notification` payload includes `session_id` and `notification_type`; explicit permission and elicitation notifications are distinguishable from `idle_prompt` and other notifications. `UserPromptSubmit`, `PreToolUse`, and `Stop` have separate lifecycle meanings. Current OrkWorks installation reports every `Notification` as `waiting_for_input` without reading `notification_type`, and does not install `UserPromptSubmit` or `Stop`. | Limited: the upstream reference is not version-pinned and no live event fixture has been captured for 2.1.274. Install only owned local entries; unrecognized payloads must not establish authority. Claude also installs a synchronous `PostToolUse` `Write\|Edit` hook (ADR 0038) whose reporter invocation passes `--report-plan-path`, forwarding `tool_input.file_path` to `/sessions/:id/plan-path` and skipping the generic attention + harness-session POSTs — the deterministic replacement for the terminal-fallback plan-path association (closes the Claude portion of #278). |
| Codex | [Hooks](https://learn.chatgpt.com/docs/hooks) | Generated/local `.codex/hooks.json` nests every event under a top-level `hooks` object — `{"hooks": {"SessionStart": [...], "UserPromptSubmit": [...], "PreToolUse": [...], "PermissionRequest": [...], "PostToolUse": [...], "Stop": [...]}}`. OrkWorks installs one owned command group for each event. Any owned Codex hook may provide the first native `session_id`; Codex documents this as a common field and says subagent hooks carry the parent session ID. Internal Codex subagents remain within their parent OrkWorks session and do not receive separate OrkWorks session IDs. `SessionStart` alone forwards its lifecycle `source`; `UserPromptSubmit` reports `working`; `PermissionRequest` reports `waiting_for_input`; and `Stop` reports `idle` because it marks the end of a turn rather than an explicit user request. `PreToolUse` and `PostToolUse` are capture-only for the #690 verification gate: the local `~/.orkworks/hook-scripts/report-harness-event-diagnostic.json` retains a bounded ordered sequence of at most 16 `PreToolUse`, `PermissionRequest`, and `PostToolUse` records, preserving event order without session IDs. Each record has only the top-level key names `hook_event_name`, `model`, `permission_mode`, `turn_id`, `tool_name`, `tool_use_id`, and `tool_response`, plus scalar values `hook_event_name`, `permission_mode`, `turn_id`, `tool_name`, and `tool_use_id` (the latter limited to strings of at most 128 characters). It omits `session_id`, `transcript_path`, `cwd`, `tool_input`, token-related keys, and all other field values. This does not change attention behavior. The reporter includes the event name and bundle fingerprint in attention reports and forwards `ORKWORKS_REPORT_TOKEN` on the identity report when available. A capability-authenticated identity report opportunistically looks up the exact native ID in `$CODEX_HOME/state_5.sqlite` (or `~/.codex/state_5.sqlite`) and uses `threads.name`, then `threads.title`, as the session label. | Feature-probed. Codex requires a one-time `/hooks` approval inside the tool before an installed hook actually executes (hash-pinned trust). Adding the capture-only event changes the SHA-256 bundle fingerprint, so users must re-approve the hook bundle once through `/hooks`. OrkWorks reports `active` only after observing a matching execution and promotes only that live session to hook-authoritative attention. Until then the session retains terminal/Peon fallback. Existing Codex identity is retained across differing hook reports; only an authenticated root `SessionStart` with `source=clear`, paired with OrkWorks' recorded reset for that ID, may replace it. The report token authenticates the OrkWorks session but is inherited by child processes, so this protocol does not prove the sender's operating-system process. Resume requires the exact ID to exist in the supported local `state_5.sqlite` and its rollout file to exist; no latest-session fallback is declared. The native label lookup is read-only, bounded, schema-sensitive, and falls back silently; it does not parse prompts or rollout JSONL. Install only owned entries (ownership recognized by the `orkworks:harness-integration:` marker embedded, as a whole `--marker '<value>'`/`-Marker '<value>'` flag argument, in the `command` string, since this schema has no dedicated marker field). |
| OpenCode | [Plugins](https://dev.opencode.ai/docs/plugins/) + `@opencode-ai/plugin@1.18.18` and `@opencode-ai/sdk@1.18.18` type declarations (npm tarballs, re-verified 2026-08-18; the docs page's own examples omit the exact `session.created` payload shape, so the published `.d.ts` was the deciding source). Attention events re-verified 2026-09-02 against the published plugin event list and community event-reference docs: `session.idle` (turn boundary, `{ sessionID }`), `session.status` (`{ sessionID, status: Info }` with `Info.type` of `busy`/`idle`/...), and `permission.asked`/`permission.replied` (the real permission events; `permission.updated` is typed in the SDK union but never emitted at runtime). The 1.18.18 and 1.18.32 schemas also define `question.asked`, `question.replied`, and `question.rejected` with request IDs, as recorded in the [prompt attention design](../superpowers/specs/2026-09-26-opencode-prompt-attention-design.md). | `.opencode/plugins/orkworks-session-reporter.js` is a project-local, gitignore-eligible target (issue #110). A plugin file exports a named async factory (`export const Name = async (input) => Hooks`, not `export default {...}`); `Hooks.event?: (input: { event: Event }) => Promise<void>` is the only entry point for session lifecycle events — there is no individual `"session.created"` hook key. `EventSessionCreated = { type: "session.created", properties: { info: Session } }` and `Session.id: string` carries the native OpenCode session ID (confirmed by extracting the real npm packages, not the docs prose, which does not state the payload shape). `ORKWORKS_PORT`/`ORKWORKS_SESSION_ID` reach the plugin via `process.env`, standard Node/Bun runtime behavior rather than an OpenCode-specific grant. Attention mapping (issue #104): `session.created` establishes initial `idle`; `session.status` type `busy` and `session.idle` update the underlying turn state to `working` and `idle`. `permission.asked` and `question.asked` track type-qualified `id` values and report `waiting_for_input`. `permission.replied`, `question.replied`, and `question.rejected` remove a matching `requestID` and report the effective state: still waiting while another request is pending, otherwise the current working or idle turn state. Attention POSTs are filtered to the captured session ID so extra sessions in one TUI cannot steer attention. | Feature-probed. The installed plugin's `event` hook is verified against the real `EventSessionCreated` type and exercised end-to-end (real ESM import, synthetic event, real HTTP POST to `/sessions/:id/harness-session`) before landing; the attention events are verified against the published event list and reference docs only — not yet exercised end-to-end inside a live OpenCode process — so coverage stays **limited** until those fixtures exist. Activation still reads `unknown` until the coding tool is detected as compatible. Install only writes the OrkWorks-owned file; a foreign, un-marked file at the same path is left untouched (`ownership_ambiguous`). |
| Antigravity CLI | No compiled signal or integration binding | OrkWorks launches `agy`, resumes an exact conversation with `agy --conversation={harnessSessionId}`, and resumes the latest conversation in the current folder with `agy --continue`. | Unsupported for integration installation and deterministic session signals until a stable, documented contract is added. |
| Gemini CLI (retired) | [Hooks reference](https://geminicli.com/docs/hooks/reference/) | Legacy `gemini` settings and historical sessions remain readable; new sessions never select or launch this retired client. | Existing owned settings are preserved rather than migrated. |
| GitHub Copilot CLI 1.0.83 (installed) | [Hooks reference](https://docs.github.com/en/copilot/reference/hooks-reference) | `.github/copilot/settings.local.json` supports inline `hooks`; command hooks use version 1 JSON configuration. Payloads include `sessionId`, `cwd`, and numeric `timestamp`. The `notification` event distinguishes `permission_prompt`, `elicitation_dialog`, background `agent_idle`/`agent_completed`, and shell-completion events. Separate `userPromptSubmitted` and `agentStop` events fire on prompt submission and when the agent is about to finish a turn; `agentStop` can block completion and force continuation, so it does not prove the turn ended. Current OrkWorks installation reports every `notification` as `waiting_for_input` without reading its type, and installs neither turn event. `sessionId` is captured via the shared reporter and feeds `ResumeStrategy::Exact` (`copilot --resume {harnessSessionId}`); `--continue` was verified empirically (not documented) to recover the most recent session machine-wide regardless of cwd, so no `latestCwd`/`latestRepo` fallback is declared for it. | Limited: the upstream reference is not version-pinned and no live event fixture has been captured for 1.0.83. Install only owned local entries; unsupported event/payload variants are a no-op until exact fixtures and version evidence pass. |
| Aider (not installed) | [Notifications](https://aider.chat/docs/usage/notifications.html) | `--notifications-command` runs when the LLM finishes a response and Aider is waiting for the next input. The callback has no native Aider session ID or structured event payload and does not distinguish an ordinary completed response from a response containing a question. OrkWorks correlates its reporter to the launched session through its inherited session environment. Current OrkWorks launch augmentation reports it as `waiting_for_input`. | Limited: the official contract is unversioned and no live fixture is available in this environment. The callback's session correlation does not make completion a prompt signal; it should not write attention state, so Peon remains the attention fallback even when notification integration is enabled. No repository Aider config is edited. |
| Generic shell | No deterministic extension point | None. | Unsupported; all integration mutation requests are no-ops with a conflict response. |

### Prompt attention authority

The #690 serial capture gate has passed; current behavior below is unchanged.
[Proposed ADR 0076](../adr/0076-codex-owned-native-approval-observer.md) and its
[written design](../superpowers/specs/2026-10-03-codex-native-approval-status-design.md)
define the proposed next step: owned native observation, a two-second grace,
and conservative resolution. They are proposed, not shipped capability.

Codex's `PermissionRequest` maps to `waiting_for_input`, with one known
exception: it also fires when `approvals_reviewer = "auto_review"` resolves the
request with no human prompt. `PreToolUse` and `PostToolUse` are capture-only
for the #690 verification gate; their redacted payloads may help establish
whether the invocation can be matched safely, but they do not resolve a wait.
The local capture omits `tool_input` and other free text. A working session can
remain Needs You until `Stop` ([#690](https://github.com/Rambolarsen/orkworks/issues/690); amendment
in [ADR 0051](../adr/0051-codex-deterministic-attention-hooks.md)). `Stop`
marks an idle turn, and conversational questions in terminal output do not
establish a prompt. The local redacted diagnostic verifies the payload fields
for #690; it stores only the key names `hook_event_name`, `model`,
`permission_mode`, `turn_id`, `tool_name`, `tool_use_id`, and `tool_response`,
plus the five allowlisted scalar values `hook_event_name`, `permission_mode`,
`turn_id`, `tool_name`, and bounded `tool_use_id`. It does not resolve
attention or alter the current
mapping. Any turn-scoped resolution stays gated on real captured payload
evidence. Peon can provide a nonprompt status, summary, phase, and diagnostics
before a validated hook executes; after activation, its descriptive fields
cannot replace hook-owned attention. The current Codex bundle has no direct
event for a queued question opening or resolving, so such prompts can be
missed; [#632](https://github.com/Rambolarsen/orkworks/issues/632) tracks that
signal gap.

OpenCode's supported question and permission events carry `id` and
`sessionID` when asked, then `requestID` and `sessionID` when replied to or
rejected. The reporter tracks pending question and permission IDs separately
for the captured session. `question.asked` and `permission.asked` report
`waiting_for_input`; matching `question.replied`, `question.rejected`, and
`permission.replied` report the remaining effective state. Busy and idle
events update turn state without clearing an outstanding request. The
reporter includes its event name, `source: "opencode_hook"`, and the
per-session `ORKWORKS_REPORT_TOKEN`; the sidecar validates the bearer token,
live session, harness, and event/status pair before activating that session's
hook authority. An installed plugin alone does not activate it. Existing
installations must reconcile the OpenCode integration through Settings to
receive the new reporter bytes.

Before OpenCode activation, Peon may supply nonprompt status and descriptive
fields. For Codex and OpenCode, a Peon-inferred chat question cannot create
**Needs You** or prompt fields; the next Peon reconciliation also clears an
older Peon-sourced wait to unknown or a newer nonprompt status. After
activation, Peon still supplies summary and diagnostics while direct reports
own attention. OpenCode attention coverage remains **limited**: versioned
schemas and reporter sequence tests establish the mapping, but live OpenCode
attention delivery has not been confirmed end to end. A lost reply or reject
may leave **Needs You** until a later recognized event, accepted input, or
session death; plugin reload cannot reconstruct requests already pending.
Before Claude Code or GitHub Copilot prompt authority activates, turn events
(Boundary: [ADR 0073](../adr/0073-other-harness-prompt-attention-authority.md))
are attention no-ops. They must not write session-wide `agent` metadata, which
would temporarily prevent Peon from writing prompt fields under the current
record-wide source-priority rule. The sidecar's existing committed-terminal-
input transition continues to clear waits independently. Peon prompt inference
remains available until an accepted, session-correlated prompt notification from
Claude's `Notification` hook or Copilot's `notification` hook arrives. Only
recognized prompt types activate authority. Before activation, recognized
nonprompt notifications do not write attention or readiness-only state. After
activation, mapped lifecycle notifications below may update or clear attention
without activating authority again. Each Claude/Copilot report must carry the
event's native ID (`session_id` or `sessionId`), the live session's report
token, and the immutable generation inherited from that session's launch
environment.
Before sending attention, the reporter
must register that ID through `POST /sessions/:id/harness-session` using the
same token and generation, with `Authorization: Bearer <ORKWORKS_REPORT_TOKEN>`
on both requests, and must stop if registration fails.
This includes the first prompt notification, so it is registered before
attention validation runs. Identity registration alone does not activate prompt
authority. The owned hook set also reports Claude `SessionStart` and Copilot
`sessionStart` solely to bind a replacement native ID after an explicit
recorded reset; those lifecycle events never write attention or activate
prompt authority by themselves. Their registration request carries the event
name and `source` in `sessionStartEvent` and `sessionStartSource`, and the
sidecar accepts only the harness-specific pair matching the recorded reset.
The first successful registration binds the current conversation epoch to one
native ID. A different ID can rebind only for an exact reset command declared
in the persisted harness definition: Claude `SessionStart(source=clear)` or
Copilot `sessionStart(source=new)`. Before dispatching that command to the
PTY, OrkWorks opens a single-use reservation for the current epoch. A matching
lifecycle registration for a replacement ID that arrives before the PTY write
acknowledgement is held as a candidate, along with any following reports for
that ID; it does not change the binding, epoch, prompt tuple, or attention
yet. The lifecycle registration is not reported as accepted until delivery
resolves. On successful delivery acknowledgement, OrkWorks commits the reset,
advances the epoch, retires the old ID, clears the prior prompt tuple under
the record-wide source rule, and preserves the launch generation. A queued
candidate binds the replacement ID and queued reports are checked afterward;
if there is no candidate, the committed reservation stays available for one
matching lifecycle registration. Until a replacement binding is accepted,
reports and exact-resume lookups using the old ID are rejected; the sidecar
does not fall back to the prior conversation. Failed or canceled delivery closes the
reservation, discards its candidate reports, and leaves the existing binding,
epoch, and tuple unchanged. Only one reset delivery may be pending per
session; a later exact reset supersedes an unmatched committed reservation.
Ordinary registration and attention reports cannot rebind; startup, resume,
fork, and unrecorded reset events cannot rebind either. Copilot's event
timestamp must be later than reservation creation, and successful delivery
acknowledgement is still required. Claude's lifecycle event has no documented
timestamp, so receipt order cannot tie a delayed report to the reset that
produced it. Copilot currently declares only bare `/clear` and `/new`;
prompt-bearing forms and `/reset` remain outside the accepted label-reset
scope in [ADR 0040](../adr/0040-harness-declared-session-label-resets.md) and
[issue #326](https://github.com/Rambolarsen/orkworks/issues/326), even though [the Copilot CLI command reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference) lists them as
starting new conversations. They do not authorize rebind
under this proposal; expanding support requires separate spec and
implementation review. The event/source claim does not prove root-process
origin: any same-session process with the inherited report token can spoof it
during a reservation, so this contract retains a same-session
identity-replacement risk.
Attention must match the registered native ID, token, and
launch generation against the sidecar's current conversation-epoch binding.
Missing, invalid, stale,
unregistered, or mismatched values are rejected. A recognized prompt report
makes
the live session hook-authoritative for `observed_status`/`attention`,
`needsUserInput`, `detectedQuestion`, and `suggestedOptions`. Metadata source
is record-wide, so the sidecar cannot identify provenance for each prompt
field separately; treat the four fields as one tuple. On activation or
demotion, clear the whole tuple when the record-wide source is not `user`, and
preserve the whole tuple when it is `user`. This intentionally clears any
non-user tuple, including a newer accepted direct report, because the record
does not retain per-field provenance. Source priority governs later writes; it
cannot preserve individual non-user fields during this group clear. Do not
selectively retain fields based on assumed per-field ownership.
Peon continues summaries, phase, diagnostics, and workflow evidence.
When integration reconciliation detects that an owned notification hook is
missing or drifted, it ends prompt authority, clears the prompt tuple as a unit
under the record-wide source rule above, and restores Peon/terminal fallback.
Silence alone does not prove hook loss. For Claude, the recognized prompt notifications are
`permission_prompt`, `elicitation_dialog`, and `elicitation_url_dialog`. After
authority activates, `UserPromptSubmit` marks work and clears older waits, and
Generic successful `PostToolUse` must not clear a permission wait: its
`tool_use_id` cannot be matched to `PermissionRequest`, which has no such ID.
Claude `Stop` is not a completion signal: a different configured Stop hook can
block stopping, so OrkWorks must not report `idle` or clear a wait from that
event. `idle_prompt`
may report `idle` after its documented delay but does not resolve an
outstanding elicitation. After activation, `elicitation_response` and
`elicitation_complete` clear only an unambiguous outstanding elicitation; the
notification carries no elicitation request ID, so it must not clear a
permission wait or an ambiguous parallel elicitation. These events do not
establish authority by themselves. Before authority activates, these turn
events do not write attention. `PreToolUse` fires before the tool executes and
before the permission decision, so it must not write attention or clear a
pending prompt.
Claude's `PermissionRequest` runs before
the permission flow and does not prove that a visible prompt was shown. Its
permission notification may be delayed about six seconds and has no event
timestamp or turn ID; a late notification may reopen stale attention and
cannot be reliably rejected as stale from documented fields alone. For
Copilot, `permission_prompt` and `elicitation_dialog` are prompt events; after
authority activates, `userPromptSubmitted` marks work. Generic successful
`postToolUse` must not clear a permission wait because its payload has no
invocation ID to correlate with the prompt. `agentStop` fires before the agent
is necessarily done because a configured hook can block it and force
continuation, so it does not write idle or clear a wait. Before activation,
`userPromptSubmitted` does not write attention.
Background `agent_idle`, `agent_completed`, and shell-completion notifications
do not mean the root session needs the user. The first two describe background
subagents, and `agentStop` is not final when another hook forces continuation;
the documented hooks provide no safe root-turn idle signal. After
`userPromptSubmitted`, `working` may persist until another authorized status
event or session end. Reliable root-idle reporting remains a separate
signal-design gap. Copilot event timestamps order
hook reports against one another and committed terminal input; older reports
must be rejected, but raw uncommitted typing must not advance the input time
used by this stale-report check. In both harnesses, a missing or inactive
integration before the first prompt-notification report preserves Peon prompt
fallback; explicit disable, uninstall, or reconciliation-detected
notification-hook drift after
activation returns future inference to fallback. Accepted terminal input means
sidecar-committed work, not raw typing or unsent input.

For the proposed authority, the Claude/Copilot native-ID registration and
attention routes both validate the session report token and unrevoked launch
generation. The token authenticates the report to the OrkWorks session, not the
harness process, because child processes inherit it. A process with access to
that session environment can submit matching values, so the protocol does not
prevent same-session process spoofing or prove that the harness emitted an
event. Event acceptance also checks the allowlisted harness/event/status
mapping and exact equality between the event's native session ID and the ID
accepted through the authenticated, generation-bound harness-session route.
The sidecar issues an immutable, per-live-session generation at launch only
when the integration is enabled and its owned prompt-notification hook is
available; otherwise the variable is absent and reports are rejected. It
passes the value as `ORKWORKS_PROMPT_HOOK_GENERATION`. Hook reporters inherit
it and forward that exact value for native-ID registration and each
attention report; they capture it at invocation start and never fetch the
session's current generation while submitting. Explicit disable, uninstall,
or reconciliation-detected notification-hook drift revokes that generation
before clearing the prompt tuple under the record-wide source rule above.
Reports from the revoked generation are rejected and cannot reactivate
authority. Re-enabling does not change the environment of an
already-running harness: that session stays on Peon/terminal fallback until it
is relaunched under a new OrkWorks live session with a fresh generation. Reports
without a generation are rejected. The generation fences lifecycle races; it
does not authenticate the sending process.

Aider's notification means that a response ended and the tool is ready for
another input. The launch reporter can correlate that callback to the owning
OrkWorks session, but the callback does not distinguish an ordinary response
from an explicit question and has no matching prompt-resolution lifecycle.
Therefore it must not report either `waiting_for_input` or `idle`: an
agent-priority idle report could suppress Peon's ability to recognize a
concrete question. Remove Aider's launch-time static hook flag and retain Peon
plus terminal fallback even when its callback is enabled. This can miss a
question Peon fails to recognize, but avoids turning completion into Needs
You or suppressing Peon with an idle report. The three integrations' current
implementations do not yet meet these rules; see [#643](https://github.com/Rambolarsen/orkworks/issues/643)
for signal validation and coverage work.

Decision rule: primary schema + reproducible fixture + version/tag evidence is
verified; primary schema + fixture without version evidence is feature-probed;
a documented event without stable payload schema is limited with unknown
activation; and a target that is not local-only or already ignored/untracked is
unsupported for installation.
