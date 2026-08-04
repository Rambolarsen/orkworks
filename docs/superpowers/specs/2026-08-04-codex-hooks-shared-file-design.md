# Codex Hooks Shared-File Ownership Design

## Goal

Resolve [issue #276](https://github.com/Rambolarsen/orkworks/issues/276): Codex's `SessionStart` hook integration (ADR 0035) targets project-level `.codex/hooks.json`, but that file has no local-only counterpart the way Claude's `settings.local.json` does. In any APM-managed repo (including this one), it's git-tracked and shared, so the generic `require_local_or_ignored_untracked` safety rule correctly refuses to write to it — leaving the integration a no-op in exactly the repos it targets.

## Design

Scope is Codex-only. Claude, Gemini, and Copilot each have a genuinely local-only config file by convention; a tracked instance of one of those is a misconfiguration, and refusing remains correct for them.

The blocker isn't sharing the file — it's that the reporter invocation OrkWorks writes bakes in the resolved absolute path to `~/.orkworks/hook-scripts/report-harness-event.sh`, which is per-machine. Writing that into a file every teammate shares means whoever installs first commits their own home directory into version control, and the next teammate's OrkWorks reads it as `Drifted`, not `Installed`.

The fix: for Codex only, express the reporter path as a `$HOME`-relative shell expression instead of a resolved absolute path, so the committed command text is identical no matter who generated it. Codex's `command` field is shell-interpreted (unlike Claude's `program`/`args` argv-style fields), so this is safe and specific to Codex's schema.

- `integrations/mod.rs` gains `portable_reporter_path(reporter: &Path) -> Result<PathBuf, IntegrationError>` (strips `dirs::home_dir()` as a prefix; errors if home is unresolvable or the reporter isn't under it) and `portable_reporter_invocation(reporter: &Path, marker: &str) -> Result<ReporterInvocation, IntegrationError>`.
- `codex.rs`'s `merge()` and `marker_state()`'s exact-match check use `portable_reporter_invocation` instead of the shared `reporter_invocation`.
- `JsonHookHandler::load()`/`install()` branch on `self.contract.harness_id == "codex"` (same precedent as the existing `is_attention_signal` check) to call the relaxed workspace-safety check.
- `integration.rs`: `ValidatedWorkspaceTarget::require_local_or_ignored_untracked()` is split — the shared prefix (revalidate identity, discover the repo, confirm workdir matches) moves into a private helper; the existing method keeps the tracked/ignored gate on top (unchanged for Claude/Gemini/Copilot); a new `require_confined_to_git_workspace()` stops before that gate (Codex only — still requires a real Git workspace, just not an untracked-and-ignored target).

### Quoting detail

`shell_quote()`/`powershell_quote()` wrap values in single quotes, and single-quoted strings don't expand `$HOME` in either POSIX shells or PowerShell. `portable_reporter_invocation` double-quotes the `$HOME`-relative path segment instead (safe: it's a fixed, OrkWorks-authored suffix, never user input) so the shell actually expands the variable, while the marker argument keeps the existing single-quote escaping untouched.

## Error handling

If `dirs::home_dir()` fails or the resolved reporter path isn't under it, Codex's install fails with a diagnostic. No fallback to writing an absolute path into a shared file.

## Testing

- Unit tests for `portable_reporter_path`/`portable_reporter_invocation`: prefix-stripping, error cases, POSIX and PowerShell quoting.
- A real-shell execution test (extending the existing `sh -n -c` syntax-check pattern to actually run the generated command with `HOME` pointed at a tempdir holding a fake reporter script) proving `$HOME` expands and the script is invoked, not just that the syntax parses.
- An integration test with a real `git2` repo and a committed (tracked) `.codex/hooks.json`: `install()` succeeds, and two different simulated home directories produce byte-identical persisted JSON.
- A fixture seeded with this repo's actual `.codex/hooks.json` shape (multiple `_apm_source: "ponytail"` groups alongside OrkWorks' own), per the issue's explicit "not just a fresh tempdir fixture" acceptance criterion — proving activation works when unrelated tracked hook groups are already present, without running install against the live repo file.
- Existing Claude/Gemini/Copilot tests must pass unchanged.

## Docs

New ADR 0036, superseding ADR 0035's open Consequences item on this question. Update `docs/agents/harness-integration-contracts.md`'s Codex row. Close issue #276.

## Non-goals

- No global `~/.codex/hooks.json` target — project-level stays authoritative, now for tracked and untracked repos alike.
- No cross-platform (POSIX/PowerShell) command harmonization for a mixed-OS team installing from different shells — pre-existing, unrelated to this fix.
- No change to Claude/Gemini/Copilot's invocation format or safety check.
