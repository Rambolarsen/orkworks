# Claude `PostToolUse` plan-path transport

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-08-05

## Context

ADR 0037 landed the sidecar's `POST /sessions/:id/plan-path` route and
explicitly left the Claude Code `PostToolUse` transport — Task 2 of its
implementation plan — as in-flight work. Since then Claude sessions have still
been relying on the terminal-output fallback in `plan_handoff::printed_plan_path`,
which two real bugs keep surfacing in production:

1. `printed_plan_path` returns the **first** matching "wrote" line it sees and
   `session_runtime` only ever sets `meta.plan_path` when the field is already
   empty (`is_none() && !plan_path_is_explicitly_cleared`). A session that
   writes an earlier unrelated plan and then a real one keeps the wrong
   association until the workspace reloads the session.
2. The terminal fallback's write-signal vocabulary is a hand-curated list that
   can misattribute an incidental mention into a stale "Plan available" card.

ADR 0037 also stated it "deliberately does not generalize `ToolHookContract`;
that broader concern remains issue #271." Implementing the Claude transport
forced the framework to express *"this integration can report plan paths"* so
the hook installer and the rendered integration status share one source of
truth.

## Decision

Claude Code's owned integration installs an additional synchronous
`PostToolUse` hook with matcher `Write|Edit`. Its reporter invocation passes
`--report-plan-path`, which switches the shared reporter script
(`report-harness-event.sh` / `report-harness-event.ps1`) into plan-path mode:
the script extracts `tool_input.file_path` from the stdin payload, skips the
generic attention and harness-session POSTs entirely, applies a cheap lexical
whitelist filter ("the path ends in `.md` and contains a recognised plan/spec
root segment"), and forwards the raw path to `POST /sessions/:id/plan-path`.
The sidecar's `report_session_plan_path` already canonicalizes and rejects
non-Markdown, workspace-escaping, symlink-pivoting, or non-existent files, so
the reporter's lexical filter exists only to avoid a wasted round-trip on
clearly-unrelated writes — it is never the authority.

`ToolHookContract` gains a `reports_plan_path: bool` field. Claude sets it
`true`; every other built-in handler keeps its existing coverage and the field
defaults to `false`. `base_status` enriches the integration coverage summary
(`"Limited harness notifications"`) with `" + plan/spec reporting"` when this
flag is set, so the user can read which integrations actually own a plan/spec
association from the integration UI rather than from code.

The reporter's flag is marker-agnostic: any `JsonHookHandler` may install a
hook entry that passes `--report-plan-path` and get the same forwarding shape.
Codex remains on the terminal fallback regardless, because its `apply_patch`
hook payload carries patch text rather than a canonical file path, just as
ADR 0037 already records.

This retracts ADR 0037's "deliberately does not generalize `ToolHookContract`"
sentence: that decision was always partial to that PR's scope, not a
project-wide stance, and #271.2's declared-attention-semantics concern stays
open as the larger unification this ADR does not attempt.

## Consequences

- Claude sessions stop relying on `printed_plan_path` for any write that the
  PostToolUse hook observes. The terminal fallback remains as the hookless
  backstop for harnesses that cannot report paths (e.g. Codex, Aider).
- The fallback's first-match-locked-forever behavior in
  `session_runtime.rs:898` no longer misleads any Claude session, because the
  hook report lands later and `report_session_plan_path` overwrites
  `meta.plan_path` unconditionally (it is not gated on `is_none()`). The
  fallback's old `is_none()` guard stays in place as the only protection
  against a later hook report clobbering a path that a real hook has already
  established — that's intentional and correct, since the terminal fallback
  is lower-confidence than the hook.
- The reporter scripts grow a new flag and a Claude-shaped payload extraction
  (`tool_input.file_path`). The marker-suffix dispatch for cwd/session_id
  capture is unchanged.
- splitting the hook contract by event (Notification / PreToolUse /
  PostToolUse / SessionStart) — which #271's full unification would also
  have to address — is still open. Every handler's `merge`/`probe`/`remove`
  keeps hand-coding its event list; this ADR adds one event to Claude's list
  rather than re-pivoting the framework around an events table.