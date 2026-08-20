# Copilot label reset commands design

## Context

Issue #326 adds verified Copilot CLI session commands to the existing session
label-reset capability. GitHub's current Copilot CLI command reference lists
`/clear [PROMPT]`, `/new [PROMPT]`, and `/reset [PROMPT]` as starting a new
conversation. The runtime already resets labels for exact commands declared
by the active harness definition; Copilot is currently undeclared despite
having verified reset commands.

## Decision

Add `labelResetCommands: ["/clear", "/new", "/reset"]` to the Copilot
builtin entry in `crates/orkworksd/resources/harnesses-v2.json`. Extend the
existing embedded harness-definition regression coverage to assert that
Copilot resolves all three commands, and extend the existing runtime test
matrix to exercise Copilot's declared commands with a persisted Copilot
session.

The implementation will not change runtime matching, label lifecycle logic,
custom override behavior, UI, or other harness definitions. Matching remains
the existing exact, trimmed command match described by the session label-reset
design. Only the bare commands are declared: Copilot's optional prompt-bearing
forms, such as `/new fix auth`, remain ordinary input because they are not
exact matches. Supporting prompt-bearing forms is outside this change.

The command evidence is the [GitHub Copilot CLI command
reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference),
which currently lists all three commands under its interactive slash commands.

## Verification

Run `embedded_builtins_are_complete_and_valid` and the existing declared-reset
runtime test after adding Copilot cases, then the complete Rust test suite and
Rust build. Confirm the JSON remains valid and the repository doc-drift and
worktree checks are clean.

## Non-goals

- Adding Copilot model, voice, or capacity support.
- Inferring resets from terminal output.
- Treating `/rename`, `/fork`, or similar commands as label resets.
- Refactoring the shared harness definition or runtime code.
