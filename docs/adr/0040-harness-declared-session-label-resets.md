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
runtime treats only an exact, trimmed match for the active session's harness as
a topic reset. It clears pending label inference, re-arms the one-shot label
lifecycle, and waits for the next descriptive user input to seed and refine a
new topic through ADR 0029's existing path.

The initial built-ins declare only documented fresh-conversation commands:
Claude Code uses `/clear`, `/reset`, and `/new`; OpenCode uses `/clear` and
`/new`. Harnesses without verified reset semantics declare none. Custom
harnesses may opt in with their own documented commands.

## Consequences

- Titles follow deliberate in-session conversation resets without resuming the
  old per-keystroke churn.
- Reset semantics stay auditable in the harness registry and are naturally
  scoped to the tool that owns the command.
- New documented commands require a small registry update and a regression
  test rather than a terminal-text heuristic.
