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
   fold it into the existing attention-report POST body (no new HTTP call);
   both fields are pulled from a single parse of the hook payload.
2. `AttentionReportRequest` gains an optional `cwd` field, only accepted
   after the same staleness/ordering guard that already protects the
   attention status in the same handler (a delayed or superseded hook event
   carries an equally stale cwd, and since this is the top-priority tier
   below, a stale write here can't be self-corrected by the more-accurate
   live probe underneath it).
3. `report_attention` stores it in a new `PeonState.reported_cwd:
   StdRwLock<HashMap<String, String>>` side-table — **not** the
   `SessionHandle`-resident field originally planned. Storing it directly on
   `SessionHandle` was considered (and would have meant zero new locks and
   automatic cleanup on session removal, as ADR 0031's own review noted for
   `session_pids`), but `SessionHandle` has roughly 50 construction sites
   across the crate versus `PeonState`'s ~37, and `reported_cwd` is
   conceptually the same shape as `PeonState`'s existing per-session
   hook/inference-reported maps (`last_inference`, `label_hint`). This trades
   away the "zero separate cleanup" property: teardown sites
   (`clear_ended_session_tracking`, `forget_session`) must explicitly clear
   it, same as `session_pids`.
4. `resolve_effective_cwds` priority order becomes: reported cwd (if set) >
   pid-probed live cwd (if resolved) > frozen launch cwd. All three are
   canonicalized the same way (including stripping Windows'
   `std::fs::canonicalize` verbatim `\\?\` prefix, which external tools like
   libgit2 can mishandle) so they compare/group consistently regardless of
   source. The pid-probe batch only covers pids whose session lacks a
   reported-cwd override, so an all-Claude-Code workspace pays no
   process-table scan at all.

**Scope**: only Claude Code and Aider have a working hook-install mechanism
today (`crates/orkworksd/src/harness/integrations/{claude,aider}.rs`). Codex
and OpenCode's integration handlers unconditionally return `unsupported` —
there is no hook channel to extend for them at all yet (tracked separately:
issues #103, #104). Aider's script dispatch has no per-marker payload
parsing today and no confirmed cwd-equivalent field, so this decision
implements Claude Code specifically. Sessions on unsupported harnesses keep
today's pid-probe/launch-cwd behavior — an honest, non-regressive partial
fix, not a full solution for every harness.

**Known uncertainties** (not confirmed by live end-to-end observation before
this change shipped, only by the documented hook schema):
- Whether Claude Code's hook `cwd` field updates to reflect the agent's
  *current* tool-call directory as it navigates mid-session, versus staying
  fixed at the harness's launch directory. If it doesn't update dynamically,
  this mechanism degrades gracefully to "no worse than before" (same
  fallback chain as ADR 0031), but the primary problem remains unsolved for
  harness sessions and this ADR's premise should be revisited.
- Reporting cadence: cwd can only reach the sidecar bundled inside a
  Notification-hook-triggered attention POST, which fires on idle/
  permission-wait events — not on every tool call or directory change. An
  agent that moves into a worktree and keeps working productively without
  pausing for input won't have its new location reported until it next goes
  idle. This is a real gap versus a hook that fires on every tool use, but
  implementing that is a larger change than this ADR covers.

## Consequences

- Claude Code sessions (this project's primary harness) get
  harness-authoritative cwd tracking, actually solving issue #241 for the
  scenario it was reported for — subject to the reporting-cadence caveat
  above.
- Aider, Codex, and OpenCode sessions remain on ADR 0031's pid-probe/launch-
  cwd fallback until their own hook support lands.
- A second per-session side-table (`PeonState.reported_cwd`, alongside
  `AppState.session_pids`) now needs explicit cleanup at every
  session-teardown site, mirroring the maintenance cost ADR 0031 already
  accepted for `session_pids` — this ADR does not reduce that cost, only
  repeats the tradeoff for a second map. A future cleanup could unify both
  into one "cwd sources" concept if the maintenance burden grows.
- `report-harness-event.sh`/`.ps1` carry one more piece of parsing logic in
  an already-established pattern (mirrors the existing `session_id`
  extraction), sharing one payload parse rather than parsing twice.
