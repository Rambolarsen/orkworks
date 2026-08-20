# Harness-declared session-label reset commands

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-08-13

## Context

ADR 0029 defines `label` as a stable, one-shot topic. A long-lived terminal
session can nevertheless contain multiple independent conversations when the
coding tool accepts a command such as `/clear` or `/new`. Without an explicit
reset, the title stays tied to the first conversation.

The tools do not share a universal reset command, and matching arbitrary slash
commands would make the title lifecycle depend on guessed semantics.

## Decision

Resolved harness definitions may declare `labelResetCommands`. The terminal
runtime treats only an exact, trimmed, successfully delivered non-sensitive
match for the active session's persisted harness ID as a topic reset. It
atomically replaces both live and persisted labels with the ADR 0029 placeholder
and clears queued label work, so the placeholder is visible until the next
descriptive input seeds a new topic.

Each queued or in-flight `InputLabel` refinement carries a per-session label
epoch. A reset increments that epoch; the refinement must still match it before
writing either live or persisted state. Thus an old conversation's late Peon
result cannot restore its title after a reset.

At this ADR's original decision, the initial built-ins declared only documented
fresh-conversation commands: Claude Code used `/clear`, `/reset`, and `/new`;
OpenCode used `/clear` and `/new`; Copilot was undeclared. Copilot currently
declares `/clear` and `/new`. `/reset` is intentionally deferred because the
existing
`minVersion` gate controls integration-status probing only, not terminal
label-reset matching, so it cannot safely protect a `/reset` declaration.
Harnesses without verified reset semantics declare none. Copilot's commands
are sourced from the [GitHub Copilot CLI command reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference).
Only the bare command forms are declared; optional prompt-bearing forms are
not exact matches and remain outside this decision. The definition field
defaults for existing custom documents; custom definitions and sparse built-in
overrides may replace it or clear it with `null`.

## Consequences

- Titles follow deliberate in-session conversation resets without resuming the
  old per-keystroke churn.
- Reset semantics stay auditable in the harness registry and are naturally
  scoped to the tool that owns the command.
- New documented commands require a small registry update and a regression
  test rather than a terminal-text heuristic.
- Label inference has an explicit generation check, avoiding a stale async
  write at the cost of carrying one small per-session counter.
