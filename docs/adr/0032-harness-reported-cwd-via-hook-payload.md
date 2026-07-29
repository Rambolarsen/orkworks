# Harness-reported cwd via existing hook payload

- Status: accepted
- Date: 2026-07-29
- Deciders: OrkWorks maintainers

## Context

ADR 0031 implemented live session cwd tracking by probing the PTY child
process's own OS-level cwd via `sysinfo`. Code review on PR #254
(`chatgpt-codex-connector`, P1) identified a fundamental gap: for
command-template harness sessions — Claude Code, Codex, OpenCode, Aider —
the PTY child is the harness binary itself
(`crates/orkworksd/src/harness/registry.rs`, `ResolvedHarness::build_launch`,
`LaunchCapability::CommandTemplate` sets `program` directly, no shell
wrapper). Coding-agent CLIs track "current directory" as internal state and
pass it explicitly as `cwd` to each subprocess they spawn for tool calls —
they don't call `chdir()` on their own process. So ADR 0031's pid-probe only
actually tracks the secondary case (`LaunchCapability::PlatformShell`, a bare
shell session) — not the primary scenario issue #241 was written about: an
agent inside a coding-tool session running `git worktree add` and continuing
work there.

## Decision

Have the harness report its own logical cwd through the hook mechanism this
project already has, used **alongside** (not replacing) ADR 0031's
pid-probe.

`crates/orkworksd/scripts/report-harness-event.sh` is already invoked by
Claude Code's Notification hook and already parses the hook's JSON stdin
payload to extract `session_id` (used for
`POST /sessions/:id/harness-session`). Per Claude Code's own hooks
documentation, that same JSON payload includes a `cwd` field (current
working directory) on every hook event, alongside `session_id`/
`transcript_path`/`hook_event_name` — sitting right next to the field
already being parsed.

1. Extract `cwd` in the script's existing `claude-code` marker branch and
   fold it into the existing attention-report POST body (no new HTTP call).
2. `AttentionReportRequest` gains an optional `cwd` field.
3. `report_attention` stores it directly on `SessionHandle.reported_cwd` —
   not a new side-table (learned from ADR 0031's own review: `session_pids`
   as a separate `AppState` map required its own insert/remove lifecycle
   management at every session-teardown site; `report_attention` already
   holds the `sessions` lock at the point it would set this field, so it
   costs zero new locks and needs zero separate cleanup).
4. `resolve_effective_cwds` priority order becomes: reported cwd (if set) >
   pid-probed live cwd (if resolved) > frozen launch cwd — canonicalized the
   same way in all three cases per ADR 0031's symlink-normalization fix.

**Scope**: only Claude Code and Aider have a working hook-install mechanism
today (`crates/orkworksd/src/harness/integrations/{claude,aider}.rs`). Codex
and OpenCode's integration handlers unconditionally return `unsupported` —
there is no hook channel to extend for them at all yet (tracked separately:
issues #103, #104). Aider's script dispatch has no per-marker payload
parsing today and no confirmed cwd-equivalent field, so this decision
implements Claude Code specifically. Sessions on unsupported harnesses keep
today's pid-probe/launch-cwd behavior — an honest, non-regressive partial
fix, not a full solution for every harness.

**Known uncertainty**: whether Claude Code's hook `cwd` field updates to
reflect the agent's *current* tool-call directory as it navigates
mid-session, versus staying fixed at the harness's launch directory, was not
confirmed by live end-to-end observation before this change shipped — only
by the documented schema. If real-world observation later shows it doesn't
update dynamically, this mechanism degrades gracefully to "no worse than
before" (same fallback chain as ADR 0031), but the primary problem would
remain unsolved for harness sessions and this ADR's premise should be
revisited.

## Consequences

- Claude Code sessions (this project's primary harness) get
  harness-authoritative cwd tracking, actually solving issue #241 for the
  scenario it was reported for.
- Aider, Codex, and OpenCode sessions remain on ADR 0031's pid-probe/launch-
  cwd fallback until their own hook support lands.
- One more field threaded through `SessionHandle`, `AttentionReportRequest`,
  and `resolve_effective_cwds`'s priority chain; no new side-table.
- `report-harness-event.sh`/`.ps1` carry one more piece of parsing logic in
  an already-established pattern (mirrors the existing `session_id`
  extraction).
