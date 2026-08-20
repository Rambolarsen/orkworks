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
| Copilot | `/clear`, `/new`, `/reset` |
| Codex, Antigravity CLI, Aider, Shell | none |

Copilot's documented commands are sourced from the [GitHub Copilot CLI
command reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference).
Only the bare forms are declared. The runtime's exact-match rule intentionally
does not treat optional prompt-bearing forms such as `/new fix auth` as label
resets; prompt-bearing reset semantics are outside this design.

On a successfully delivered, non-sensitive reset command, OrkWorks atomically
replaces the live and persisted label with `Session <id-prefix>`, clears that
session's pending hint/inference request, and increments its label epoch. The
placeholder is intentionally visible until the next descriptive input arrives.
The reset command itself never becomes a label. The next descriptive user input
uses the existing synchronous fallback and one-shot `InputLabel` Peon
refinement. Subsequent descriptive inputs remain frozen until another declared
reset command.

Each queued or in-flight `InputLabel` refinement carries the epoch captured
with its hint. It may update the live or persisted label only if that epoch is
still current. A reset therefore cannot be overwritten by an old conversation's
late Peon result.

`labelResetCommands` applies only to the active session's resolved harness
definition, using the persisted session harness ID; missing or unknown IDs do
nothing. Existing custom definitions remain compatible because the field has a
Serde default. Custom definitions and sparse built-in overrides may replace the
list with their own documented commands (or clear it with `null`); an unknown,
undeclared, or merely similar slash command has no label effect.

## Non-goals

- Inferring a fresh conversation from terminal output, compacting, or arbitrary
  topic drift.
- Treating similarly named commands as resets, or inventing commands for tools
  without verified documentation.
- Adding a manual label rename flow.

## Verification

Rust regression tests cover exact matching (`/new` only, not `/new extra`,
`new`, or `/NEW`), harness scoping, missing/unknown harness IDs, sensitive
input, clearing stale Peon label work, rejection of a delayed old-epoch result,
re-seeding after the next descriptive input, override parsing, and preservation
of the existing frozen behavior for normal input.
