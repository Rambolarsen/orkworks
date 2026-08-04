# Codex hook installation uses a portable, home-relative reporter path

- Status: accepted
- Deciders: Lars-Erik, Claude Sonnet 5
- Date: 2026-08-04

## Context

ADR 0035 added a real Codex `SessionStart` hook integration targeting
project-level `.codex/hooks.json`, following the same
`require_local_or_ignored_untracked` safety rule every other JSON-hook
integration (Claude, Gemini, Copilot) uses. That rule assumes the target
file is local-only by convention — true for Claude's `settings.local.json`,
which has a genuine local/shared split (`settings.json` vs
`settings.local.json`). Codex has no such split: `.codex/hooks.json` is its
only hooks file. In any APM-managed repo, including this one, it's
deliberately git-tracked so APM's `ponytail` skill can install real
team-shared hooks there. The safety rule correctly refuses to write to a
tracked target, which left Codex's integration a permanent no-op in exactly
the repos it was built for — tracked as
[issue #276](https://github.com/Rambolarsen/orkworks/issues/276).

The actual hazard the safety rule guards against isn't sharing the file —
it's that every JSON-hook integration's reporter invocation bakes in the
resolved absolute path to `~/.orkworks/hook-scripts/report-harness-event.sh`,
which is per-machine. Writing that into a file every teammate shares means
whoever installs first commits their own home directory into version
control, and the next teammate's OrkWorks reads the fragment as `Drifted`,
not `Installed`. This repo's own committed `.codex/hooks.json` (installed by
APM) already demonstrates the alternative: every command APM writes there
resolves paths at shell-run time (`$ROOT=$(git rev-parse --show-toplevel)`,
repo-relative script references) rather than baking in whoever ran
`apm install`'s absolute path.

## Decision

- Codex's `merge`/`probe` (`crates/orkworksd/src/harness/integrations/codex.rs`)
  now build their hook command via a new `portable_reporter_invocation`
  (`crates/orkworksd/src/harness/integrations/mod.rs`), which rewrites the
  resolved reporter-script path as a `$HOME`-relative shell expression
  (e.g. `"$HOME/.orkworks/hook-scripts/report-harness-event.sh"`) instead of
  an absolute one. The committed command text is now byte-identical
  regardless of whose machine generated it.
- The existing `shell_quote`/`powershell_quote` helpers always single-quote,
  and single-quoted strings don't expand `$HOME` in POSIX shells.
  `portable_reporter_invocation` double-quotes the `$HOME`-relative path
  segment instead — safe because that segment is always a fixed,
  OrkWorks-authored suffix, never user input — while the marker argument
  keeps the existing single-quote escaping.
- `ValidatedWorkspaceTarget::require_local_or_ignored_untracked`
  (`crates/orkworksd/src/harness/integration.rs`) is split into a shared
  `require_confined_git_target(allow_tracked: bool)` helper. A new
  `require_tracked_or_ignored_untracked()` (Codex only) calls it with
  `allow_tracked: true`: a tracked target is now accepted, but an
  untracked-and-unignored one is still refused exactly as before — only the
  tracked case widens.
- `JsonHookHandler::load()` branches on `harness_id == "codex"` (matching
  the existing `is_attention_signal` special-case precedent from ADR 0035)
  to call the relaxed check instead of the standard one.
- This is POSIX-only. Whether Codex's `command` field is parsed by a shell
  that expands `$HOME` on Windows is unverified — `cmd.exe` doesn't, and an
  outer `powershell.exe`'s `-File` argument is not expression-evaluated the
  way inline script text is. Codex on Windows keeps writing an absolute
  path and is still refused on a tracked target: a known, pre-existing
  limitation, not a regression from this change.

## Consequences

- Codex's integration now actually activates in APM-managed repos (this one
  included), closing the gap ADR 0035 left open and closing issue #276.
- A pre-fix, absolute-path Codex fragment installed by an older OrkWorks
  version reads as `Drifted`, not silently `Installed`, once this version
  runs `probe` against it — the next install/reconcile replaces it with the
  portable form.
- Windows Codex support for the tracked-file case remains unresolved,
  tracked as follow-up work rather than solved speculatively here — the
  blocker is verifying which shell actually parses Codex's `command` field
  on Windows and whether `$HOME`/`$env:USERPROFILE` expansion reaches it,
  not a design decision this ADR can make from this repo alone.
- The portable/absolute split means Codex's reporter-invocation code path
  now differs from Claude/Gemini/Copilot's, which was intentional — those
  three have a real local-only file by convention (a tracked instance is a
  misconfiguration, correctly still refused) and don't need this.
