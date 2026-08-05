# Codex Hooks Shared-File Ownership Design

## Goal

Resolve [issue #276](https://github.com/Rambolarsen/orkworks/issues/276): Codex's `SessionStart` hook integration (ADR 0035) targets project-level `.codex/hooks.json`, but that file has no local-only counterpart the way Claude's `settings.local.json` does. In any APM-managed repo (including this one), it's git-tracked and shared, so the generic `require_local_or_ignored_untracked` safety rule correctly refuses to write to it — leaving the integration a no-op in exactly the repos it targets.

## Design

Scope is Codex-only. Claude, Gemini, and Copilot each have a genuinely local-only config file by convention; a tracked instance of one of those is a misconfiguration, and refusing remains correct for them.

The blocker isn't sharing the file — it's that the reporter invocation OrkWorks writes bakes in the resolved absolute path to `~/.orkworks/hook-scripts/report-harness-event.sh`, which is per-machine. Writing that into a file every teammate shares means whoever installs first commits their own home directory into version control, and the next teammate's OrkWorks reads it as `Drifted`, not `Installed`.

The fix: for Codex only, express the reporter path as a `$HOME`-relative shell expression instead of a resolved absolute path, so the committed command text is identical no matter who generated it. Codex's `command` field is shell-interpreted (unlike Claude's `program`/`args` argv-style fields), so this is safe and specific to Codex's schema.

- `integrations/mod.rs` gains `portable_reporter_path(reporter: &Path) -> Result<PathBuf, IntegrationError>` (strips `dirs::home_dir()` as a prefix; errors if home is unresolvable or the reporter isn't under it) and `portable_reporter_invocation(reporter: &Path, marker: &str) -> Result<ReporterInvocation, IntegrationError>`.
- `codex.rs`'s `merge()` propagates `portable_reporter_invocation`'s `Result` with `?` (its signature is already fallible, no change needed). `probe()`/`remove()` (also already `Result`-returning) compute the portable invocation once per call and pass the resolved value (not a raw path) down into the private, infallible `marker_state()` helper — so `marker_state`'s second parameter changes from `Option<&Path>` to `Option<&ReporterInvocation>` (or equivalent), a change contained entirely inside codex.rs. If the portable path can't be resolved, the error surfaces through `probe`/`merge`'s existing `Result` — same "hard stop, not fallback" behavior as the Error handling section already states, just applied uniformly.
- `JsonHookHandler::load()`/`install()` branch on `self.contract.harness_id == "codex"` (same precedent as the existing `is_attention_signal` check) to call the relaxed workspace-safety check.
- `integration.rs`: `ValidatedWorkspaceTarget::require_local_or_ignored_untracked()` is split — the shared prefix (revalidate identity, discover the repo, confirm workdir matches) moves into a private helper; the tracked-check becomes a parameter rather than being dropped wholesale. The existing public method keeps calling it with `allow_tracked: false` (unchanged behavior for Claude/Gemini/Copilot: tracked refused, untracked-and-unignored refused, untracked-and-ignored ok). A new `require_tracked_or_ignored_untracked()` (Codex only) calls it with `allow_tracked: true`: tracked is now accepted, but untracked-and-unignored is still refused exactly as before — only the tracked case widens, nothing else.

### Platform scope

The portable rewrite applies to POSIX only for this change. Whether Codex's `command` field is parsed by a shell that expands `$HOME` on Windows (`cmd.exe` doesn't; an outer `powershell.exe` might, but `-File`'s argument is not expression-evaluated the way inline script text is) is unverified and not something this repo can confirm without a real Windows Codex install. Rather than ship an untested assumption, `portable_reporter_invocation` is POSIX-only; on Windows, Codex keeps today's behavior unchanged (absolute path, still refused on a tracked target — a known, pre-existing limitation, not a regression). Verifying and extending to Windows is follow-up work, tracked as a note in the ADR's Consequences rather than solved speculatively here.

### Quoting detail

`shell_quote()`/`powershell_quote()` wrap values in single quotes, and single-quoted strings don't expand `$HOME` in either POSIX shells or PowerShell. `portable_reporter_invocation` double-quotes the `$HOME`-relative path segment instead (safe: it's a fixed, OrkWorks-authored suffix, never user input) so the shell actually expands the variable, while the marker argument keeps the existing single-quote escaping untouched.

## Error handling

If `dirs::home_dir()` fails or the resolved reporter path isn't under it, Codex's install fails with a diagnostic. No fallback to writing an absolute path into a shared file.

## Testing

- Unit tests for `portable_reporter_path`/`portable_reporter_invocation`: prefix-stripping, error cases, POSIX quoting (PowerShell quoting is out of scope per Platform scope above).
- A real-shell execution test (extending the existing `sh -n -c` syntax-check pattern to actually run the generated command with `HOME` pointed at a tempdir holding a fake reporter script) proving `$HOME` expands and the script is invoked, not just that the syntax parses.
- Any test that sets the `HOME` env var must use the existing `ENV_LOCK: Mutex<()>` guard pattern from `peon.rs` (around line 772) — `dirs::home_dir()` reads `$HOME` directly on unix, so unguarded mutation across parallel `cargo test` threads will flake.
- The existing shared `json_handler_conformance_matrix_preserves_unrelated_configuration` test (mod.rs, currently ~line 682) builds each handler's `ReporterAssetResolver.stable_dir` under an arbitrary tempdir unrelated to `dirs::home_dir()`. Since Codex's `load()` now requires the resolved reporter path to be under home, the Codex case in that shared loop needs its `stable_dir` reshaped to live under an `ENV_LOCK`-guarded temporary `HOME` (e.g. `stable_dir: home.path().join(".orkworks").join("hook-scripts")` with `HOME` set to `home.path()` for the duration of that iteration) instead of reusing the other handlers' arbitrary tempdir directly.
- An integration test with a real `git2` repo and a committed (tracked) `.codex/hooks.json`: `install()` succeeds, and two different simulated home directories (each its own `ENV_LOCK`-guarded `HOME`) produce byte-identical persisted JSON — the core claim the whole design rests on.
- A test asserting an untracked, non-gitignored `.codex/hooks.json` is still refused for Codex (confirms `require_tracked_or_ignored_untracked` only widened the tracked case, not that one).
- A fixture seeded with this repo's actual `.codex/hooks.json` shape (multiple `_apm_source: "ponytail"` groups alongside OrkWorks' own), per the issue's explicit "not just a fresh tempdir fixture" acceptance criterion — proving activation works when unrelated tracked hook groups are already present, without running install against the live repo file.
- Existing Claude/Gemini/Copilot tests must pass unchanged.

## Docs

New ADR 0036, superseding ADR 0035's open Consequences item on this question. Update `docs/agents/harness-integration-contracts.md`'s Codex row. Close issue #276.

## Non-goals

- No global `~/.codex/hooks.json` target — project-level stays authoritative, now for tracked and untracked repos alike.
- No Windows/PowerShell portable rewrite in this change (see Platform scope) — Windows Codex keeps today's behavior, not silently altered.
- No cross-platform (POSIX/PowerShell) command harmonization for a mixed-OS team installing from different shells — pre-existing, unrelated to this fix.
- No change to Claude/Gemini/Copilot's invocation format or safety check.
