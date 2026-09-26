# Hook-owned prompt attention for Codex and OpenCode

- Status: accepted
- Deciders: Rambolarsen, Codex
- Date: 2026-09-26

## Context

A Peon reading terminal output can mistake an old conversational question for
a current prompt. This left a session showing **Needs You** after the coding
tool had moved on. Codex and OpenCode have direct attention hooks, but an
installed hook file or a registry capability does not prove that a hook ran
for a particular session. A hook may be absent, unapproved, or broken.
OpenCode also needs explicit question lifecycle events: turn completion alone
cannot distinguish an unanswered question from ordinary idle time.

## Decision

For Codex and OpenCode, Peon inference from conversational text cannot
establish a prompt. It must not set `waiting_for_input`, `needsUserInput`,
`detectedQuestion`, or `suggestedOptions` from an inferred question. Before a
session accepts a direct attention event, Peon may still supply nonprompt
status and descriptive metadata. The next attention reconciliation clears an
existing Peon-sourced prompt status and fields to unknown unless a newer
direct event or user override has arrived; it does not turn the old inference
into `idle`.

Only an accepted, session-scoped hook report activates hook authority for a
live Codex or OpenCode session. Installation and capability remain distinct
from activation. Codex retains its owned bundle fingerprint, harness, and
event validation. OpenCode starts with `active_work_hook` false; its reporter
tags a report with `source: "opencode_hook"`, the originating event, and the
session's `ORKWORKS_REPORT_TOKEN`. The sidecar validates the bearer token,
live session, OpenCode harness, and event/status pair before accepting and
activating the report. `session.created` may report initial `idle`;
`session.status` with `busy` may report `working`; `session.idle` may report
`idle`; permission or question asks may report `waiting_for_input`; and
matching replies or rejections may report the remaining effective state.
Generic `agent` posts, debug injection, plugin files, and registry flags do
not activate hook authority. Possession of the token proves the session
capability, not the identity of code within the OpenCode process.

After activation, Peon may update summary, phase, diagnostics, and workflow
evidence, but cannot replace the hook-owned attention status or prompt fields
after the ordinary staleness window. The first accepted hook report clears
older Peon-sourced prompt fields. Later accepted hook events, user overrides,
accepted terminal input, and session lifecycle transitions retain their
existing authority; an untagged OpenCode `agent` post cannot inherit hook
authority or replace its attention. Authority belongs to one session and is
retained across an accepted user-input process transition. User-sourced
status keeps its higher priority.

`metadata.rs` remains the owner of status merging and source priority. A
validated OpenCode hook report persists at agent priority with the existing
`agent` metadata source; the live session's `active_work_hook` records its
validated authority. The Peon-preserving merge retains each harness's actual
provenance rather than recording OpenCode reports as `codex_hook`.

## Consequences

- **Needs You** for Codex and OpenCode requires a validated direct prompt
  signal. A real prompt may be missed when hooks do not execute; Peon does
  not replace missing direct evidence with a guess.
- OpenCode's pending permission and question IDs are process-local. A lost
  reply or rejection can leave **Needs You** stale until a later recognized
  event, accepted terminal input, or session death changes attention. Plugin
  reload cannot reconstruct an already pending request from
  `session.created` alone.
- Codex's current hook bundle does not report queued-question opening or
  resolution; [#632](https://github.com/Rambolarsen/orkworks/issues/632)
  tracks that direct-signal gap.
- Claude Code, Copilot CLI, Aider, and hookless tools keep their existing Peon
  policy. [#643](https://github.com/Rambolarsen/orkworks/issues/643) tracks
  separate review of their event coverage.
