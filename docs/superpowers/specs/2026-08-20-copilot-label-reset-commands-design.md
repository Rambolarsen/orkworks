# Copilot label reset commands design

## Context

Issue #326 adds verified Copilot CLI session commands to the existing session
label-reset capability. GitHub's current Copilot CLI command reference lists
`/clear [PROMPT]`, `/new [PROMPT]`, and `/reset [PROMPT]` as starting a new
conversation. Before this change, Copilot was undeclared despite having
verified reset commands. After this change, it is declared through the existing
minimum-version detection gate with a Copilot CLI 1.0.33 floor, so `/reset` is
not claimed for older accepted versions.

## Decision

Add `labelResetCommands: ["/clear", "/new", "/reset"]` to the Copilot
builtin entry in `crates/orkworksd/resources/harnesses-v2.json`, alongside
`minVersion: { "min": [1, 0, 33] }`. Extend the existing embedded
harness-definition regression coverage to assert both Copilot's minimum
version and all three commands, and extend the existing runtime test matrix to
exercise Copilot's declared commands with a persisted Copilot session.

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

Run the focused minimum-version definition regression and the existing
declared-reset runtime test after adding Copilot's 1.0.33 floor, then the
complete Rust test suite and Rust build. Confirm the JSON remains valid and
the repository doc-drift and worktree checks are clean.

## Non-goals

- Adding Copilot model, voice, or capacity support.
- Inferring resets from terminal output.
- Treating `/rename`, `/fork`, or similar commands as label resets.
- Refactoring the shared harness definition or runtime code.
