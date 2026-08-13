# Session label reset commands design

## Context

ADR 0029 made the session label a stable, one-shot topic. That prevents titles
from changing on every input, but a harness command that starts a fresh
conversation leaves the old topic in place. Issue #240 addresses that gap.

## Decision

The resolved harness definition declares an optional, explicit list of
`labelResetCommands`. A terminal input is a reset only when its trimmed text is
an exact member of the active session harness's list. The initial built-ins are:

| Coding tool | Reset commands |
| --- | --- |
| Claude Code | `/clear`, `/reset`, `/new` |
| OpenCode | `/clear`, `/new` |
| Codex, Antigravity CLI, Aider, Copilot, Shell | none |

On a reset command OrkWorks clears that session's pending label inference and
re-arms its one-shot label lifecycle. The reset command itself never becomes a
label. The next descriptive user input uses the existing synchronous fallback
and one-shot `InputLabel` Peon refinement. Subsequent descriptive inputs remain
frozen until another declared reset command.

`labelResetCommands` applies only to the active session's resolved harness
definition. Custom harnesses may supply their own documented commands; an
unknown or undeclared slash command has no label effect.

## Non-goals

- Inferring a fresh conversation from terminal output, compacting, or arbitrary
  topic drift.
- Treating similarly named commands as resets, or inventing commands for tools
  without verified documentation.
- Adding a manual label rename flow.

## Verification

Rust regression tests cover exact matching, harness scoping, clearing stale
Peon label work, re-seeding after the next descriptive input, and preservation
of the existing frozen behavior for normal input.
