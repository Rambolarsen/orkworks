# Copilot label reset commands design

## Context

Issue #326 adds verified Copilot CLI session commands to the existing session
label-reset capability. Copilot documents `/clear` as abandoning the current
session and starting fresh, and `/new` as starting a new conversation. The
runtime already resets labels for exact commands declared by the active
harness definition; Copilot is the remaining supported harness missing these
declarations.

## Decision

Add `labelResetCommands: ["/clear", "/new"]` to the Copilot builtin entry in
`crates/orkworksd/resources/harnesses-v2.json`. Extend the existing embedded
harness-definition regression coverage to assert that Copilot resolves those
two commands.

The implementation will not change runtime matching, label lifecycle logic,
custom override behavior, UI, or other harness definitions. Matching remains
the existing exact, trimmed command match described by the session label-reset
design.

## Verification

Run the focused harness-definition test, then the complete Rust test suite and
Rust build. Confirm the JSON remains valid and the repository doc-drift and
worktree checks are clean.

## Non-goals

- Adding Copilot model, voice, or capacity support.
- Inferring resets from terminal output.
- Treating `/rename`, `/fork`, or similar commands as label resets.
- Refactoring the shared harness definition or runtime code.
