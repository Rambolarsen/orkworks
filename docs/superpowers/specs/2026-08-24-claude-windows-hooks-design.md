# Claude Windows Hooks Design

## Goal

Make OrkWorks' Claude Code hooks use the hook schema Claude Code actually
supports on Windows, while preserving the existing POSIX behavior.

## Decision

Use Claude Code's single `command` field and its supported `shell` field.
Remove the unsupported `commandWindows` configuration. Hooks that execute
PowerShell scripts will set `shell` to `powershell`; hooks that execute the
repository's POSIX scripts will explicitly set `shell` to `bash`.

The Rust Claude integration already selects the platform-specific reporter
asset (`report-harness-event.ps1` on Windows) and emits a PowerShell command.
Add regression coverage for the generated Windows hook shape so future
changes cannot reintroduce unsupported `commandWindows` fields or POSIX
reporter paths on Windows.

## Scope

- Update `.claude/settings.json`'s project hooks to use supported fields.
- Keep the untracked `.claude/settings.local.json` machine-local and do not
  overwrite it as part of this change.
- Add Rust tests for the Windows Claude integration command shape.
- Do not convert repository shell scripts to PowerShell; Claude's supported
  `shell: "bash"` path remains the contract for those scripts.

## Verification

- Parse `.claude/settings.json` and assert no hook uses `commandWindows`.
- Run the focused Rust harness integration tests.
- Run the repository doc and worktree currency checks.
