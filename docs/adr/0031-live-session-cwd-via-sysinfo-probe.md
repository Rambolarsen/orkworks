# Live session cwd via cross-platform sysinfo probe

- Status: accepted
- Date: 2026-07-29
- Deciders: OrkWorks maintainers

## Context

Issue #241: a session's displayed `repo_root`/`branch` reflects the PTY
child's cwd at spawn time (`crates/orkworksd/src/runtime/session_runtime.rs`,
`cmd.cwd(&command.cwd)`), never updated afterward.
`enrich_sessions_with_git_context`
(`crates/orkworksd/src/http/session_handlers.rs`) re-derives git context on
every `GET /sessions` poll, but keyed off that same frozen cwd.

This is a routine failure mode for this project specifically: `AGENTS.md`'s
own `starting-work` skill tells agents to `git worktree add` mid-session. The
moment an agent does that, the sessions panel keeps showing the original
checkout's branch forever, even though the agent has moved to an isolated
worktree — undermining the panel's ability to confirm a session is working in
its own clean workspace.

## Decision

Capture the PTY child's OS pid at spawn
(`portable_pty::Child::process_id()`, already available in the pinned
`portable-pty` dependency, previously unused) and probe its live cwd on each
poll via the `sysinfo` crate (new dependency), which supports Linux, macOS,
and Windows behind one `Process::cwd()` call — no per-OS `unsafe` FFI, no
`cfg` branches in our own code.

1. Store the pid in a new `session_pids: Mutex<HashMap<String, u32>>` side
   table on `AppState`, mirroring the existing `PeonState` side-table
   pattern rather than widening `SessionHandle`/`SessionInfo`.
2. Add `crates/orkworksd/src/procfs.rs` exposing
   `pub fn live_cwd(pid: u32) -> Option<String>`, backed by `sysinfo`.
   Any failure (process gone, permission denied, unsupported platform)
   collapses to `None`.
3. `enrich_sessions_with_git_context` resolves an effective cwd per session
   (live cwd if the probe succeeds, else the launch-time `info.cwd`) before
   its existing per-cwd git-detection dedup, so the recommendation/dedup
   logic operates on the *live* location.
4. Polling stays purely on-demand inside `GET /sessions` (already polled by
   the frontend every ~2s) — no new background task.
5. Persisted `SessionMetadata.cwd`/`repo_root`/`branch` keep meaning
   "launch-time/last-known," not a continuously-live mirror; no schema
   change.

## Rejected alternatives

- **Hand-rolled per-OS `unsafe` FFI** (`/proc/<pid>/cwd` on Linux,
  `proc_pidinfo` on macOS, an `NtQuerySystemInformation`-family call on
  Windows): three separate platform-specific paths, `unsafe` on two of
  them, and no existing Windows process-introspection precedent in this
  crate to build on. `sysinfo` already solves this correctly and is
  widely used; only looked worth avoiding when Windows was going to be
  out of scope.
- **OSC 7 shell escape-sequence tracking** (parse the PTY output stream for
  the `file://host/path` sequence shells emit on prompt draw, as VS
  Code/iTerm2/Windows Terminal do): no new dependency, but requires
  injecting a prompt-hook snippet per shell (bash/zsh/fish/PowerShell) at
  spawn and silently fails to track cwd for any shell without a wired
  hook — more moving parts than one library call.
- **Shelling out to `lsof`/platform equivalents per lookup**: subprocess
  spawn every ~2s poll cycle, depends on external binaries being present,
  not meaningfully simpler than a library call.
- **Windows unsupported**: considered and rejected — the user explicitly
  wants Windows covered, and `sysinfo` makes full three-platform support
  no harder than single-platform support.

## Consequences

- New runtime state (`session_pids`) must be kept in sync with session
  spawn/deletion.
- One new dependency (`sysinfo`), no new `unsafe` code.
- The git chip reflects the agent's real working directory on all three
  supported desktop platforms without any frontend change.
- Windows behavior relies on `sysinfo`'s own correctness; not manually
  verified in this repo's primarily macOS/Linux dev environment.
