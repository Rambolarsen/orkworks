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
| Claude Code | [Hooks reference](https://code.claude.com/docs/en/hooks) | `.claude/settings.local.json` is local/non-shareable. The documented `hooks` object contains event matcher groups and command hooks; common payload includes string `session_id`, `cwd`, and `hook_event_name`. | Feature-probed. Install only owned local entries; unknown event fields are ignored. Coverage remains limited until version-pinned fixtures cover selected events. Claude also installs a synchronous `PostToolUse` `Write\|Edit` hook (ADR 0038) whose reporter invocation passes `--report-plan-path`, forwarding `tool_input.file_path` to `/sessions/:id/plan-path` and skipping the generic attention + harness-session POSTs — the deterministic replacement for the terminal-fallback plan-path association (closes the Claude portion of #278). |
| Codex | [Hooks](https://learn.chatgpt.com/docs/hooks) | Generated/local `.codex/hooks.json` nests every event under a top-level `hooks` object — `{"hooks": {"SessionStart": [...], "UserPromptSubmit": [...], "PermissionRequest": [...], "Stop": [...]}}`. OrkWorks installs one owned command group for each event. Only the root `SessionStart` reports the native `session_id`; internal Codex subagents remain within their parent OrkWorks session and do not receive a separate OrkWorks session ID. `SessionStart` forwards its `source`; `UserPromptSubmit` reports `working`; `PermissionRequest` reports `waiting_for_input`; and `Stop` reports `idle` because it marks the end of a turn rather than an explicit user request. The reporter includes the event name and bundle fingerprint in attention reports and forwards `ORKWORKS_REPORT_TOKEN` on the identity report when available. A capability-authenticated identity report opportunistically looks up the exact native ID in `$CODEX_HOME/state_5.sqlite` (or `~/.codex/state_5.sqlite`) and uses `threads.name`, then `threads.title`, as the session label. | Feature-probed. Codex requires a one-time `/hooks` approval inside the tool before an installed hook actually executes (hash-pinned trust). The installed bundle carries one SHA-256 fingerprint; OrkWorks reports `active` only after observing a matching execution and promotes only that live session to hook-authoritative attention. Until then the session retains terminal/Peon fallback. Existing Codex identity is retained across differing hook reports; only an authenticated root `SessionStart` with `source=clear`, paired with OrkWorks' recorded reset for that ID, may replace it. The report token authenticates the OrkWorks session but is inherited by child processes, so this protocol does not prove the sender's operating-system process. Resume requires the exact ID to exist in the supported local `state_5.sqlite` and its rollout file to exist; no latest-session fallback is declared. The native label lookup is read-only, bounded, schema-sensitive, and falls back silently; it does not parse prompts or rollout JSONL. Install only owned entries (ownership recognized by the `orkworks:harness-integration:` marker embedded, as a whole `--marker '<value>'`/`-Marker '<value>'` flag argument, in the `command` string, since this schema has no dedicated marker field). |
| OpenCode | [Plugins](https://dev.opencode.ai/docs/plugins/) + `@opencode-ai/plugin@1.18.18` and `@opencode-ai/sdk@1.18.18` type declarations (npm tarballs, re-verified 2026-08-18; the docs page's own examples omit the exact `session.created` payload shape, so the published `.d.ts` was the deciding source). Attention events re-verified 2026-09-02 against the published plugin event list and community event-reference docs: `session.idle` (turn boundary, `{ sessionID }`), `session.status` (`{ sessionID, status: Info }` with `Info.type` of `busy`/`idle`/...), and `permission.asked`/`permission.replied` (the real permission events; `permission.updated` is typed in the SDK union but never emitted at runtime). The 1.18.18 and 1.18.32 schemas also define `question.asked`, `question.replied`, and `question.rejected` with request IDs, as recorded in the [prompt attention design](../superpowers/specs/2026-09-26-opencode-prompt-attention-design.md). | `.opencode/plugins/orkworks-session-reporter.js` is a project-local, gitignore-eligible target (issue #110). A plugin file exports a named async factory (`export const Name = async (input) => Hooks`, not `export default {...}`); `Hooks.event?: (input: { event: Event }) => Promise<void>` is the only entry point for session lifecycle events — there is no individual `"session.created"` hook key. `EventSessionCreated = { type: "session.created", properties: { info: Session } }` and `Session.id: string` carries the native OpenCode session ID (confirmed by extracting the real npm packages, not the docs prose, which does not state the payload shape). `ORKWORKS_PORT`/`ORKWORKS_SESSION_ID` reach the plugin via `process.env`, standard Node/Bun runtime behavior rather than an OpenCode-specific grant. Attention mapping (issue #104): `session.created` establishes initial `idle`; `session.status` type `busy` and `session.idle` update the underlying turn state to `working` and `idle`. `permission.asked` and `question.asked` track type-qualified `id` values and report `waiting_for_input`. `permission.replied`, `question.replied`, and `question.rejected` remove a matching `requestID` and report the effective state: still waiting while another request is pending, otherwise the current working or idle turn state. Attention POSTs are filtered to the captured session ID so extra sessions in one TUI cannot steer attention. | Feature-probed. The installed plugin's `event` hook is verified against the real `EventSessionCreated` type and exercised end-to-end (real ESM import, synthetic event, real HTTP POST to `/sessions/:id/harness-session`) before landing; the attention events are verified against the published event list and reference docs only — not yet exercised end-to-end inside a live OpenCode process — so coverage stays **limited** until those fixtures exist. Activation still reads `unknown` until the coding tool is detected as compatible. Install only writes the OrkWorks-owned file; a foreign, un-marked file at the same path is left untouched (`ownership_ambiguous`). |
| Antigravity CLI | No compiled signal or integration binding | OrkWorks launches `agy`, resumes an exact conversation with `agy --conversation={harnessSessionId}`, and resumes the latest conversation in the current folder with `agy --continue`. | Unsupported for integration installation and deterministic session signals until a stable, documented contract is added. |
| Gemini CLI (retired) | [Hooks reference](https://geminicli.com/docs/hooks/reference/) | Legacy `gemini` settings and historical sessions remain readable; new sessions never select or launch this retired client. | Existing owned settings are preserved rather than migrated. |
| GitHub Copilot CLI | [Hooks reference](https://docs.github.com/en/copilot/reference/hooks-reference) | `.github/copilot/settings.local.json` supports inline `hooks`; command hooks use versioned JSON configuration. Documented payload includes string `sessionId` (camelCase, unlike Claude/Codex's `session_id`), string `cwd`, and numeric `timestamp`; `notification` reports `agent_idle` and `permission_prompt`. `sessionId` is captured via the same reporter script (`report-harness-event.sh`/`.ps1`) as Claude/Codex and feeds `ResumeStrategy::Exact` (`copilot --resume {harnessSessionId}`); `--continue` was verified empirically (not documented) to recover the most recent session machine-wide regardless of cwd, so no `latestCwd`/`latestRepo` fallback is declared for it. | Feature-probed. Install only owned local entries. Unsupported event/payload variants are a no-op until exact fixtures and version evidence pass. |
| Aider | [Notifications](https://aider.chat/docs/usage/notifications.html) | `--notifications-command` runs a configured command when Aider is waiting for input; it provides no native session ID or lifecycle schema. | Limited. The workspace-owned enablement flag may augment launch with the stable reporter; no repository Aider config is edited. |
| Generic shell | No deterministic extension point | None. | Unsupported; all integration mutation requests are no-ops with a conflict response. |

### Prompt attention authority

Codex's `PermissionRequest` is a direct prompt signal. `Stop` marks an idle
turn, and conversational questions in terminal output do not establish a
prompt. Peon can provide a nonprompt status, summary, phase, and diagnostics
before a validated hook executes; after activation, its descriptive fields
cannot replace hook-owned attention. The current Codex bundle has no direct
event for a queued question opening or resolving, so such prompts can be
missed; [#632](https://github.com/Rambolarsen/orkworks/issues/632) tracks
that signal gap.

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
Other coding tools retain their current Peon policy pending the separate
event-coverage review in [#643](https://github.com/Rambolarsen/orkworks/issues/643).

Decision rule: primary schema + reproducible fixture + version/tag evidence is
verified; primary schema + fixture without version evidence is feature-probed;
a documented event without stable payload schema is limited with unknown
activation; and a target that is not local-only or already ignored/untracked is
unsupported for installation.
