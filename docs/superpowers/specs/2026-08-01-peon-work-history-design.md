# Peon work-history design

## Goal

Task history records concrete session work, not guesses about terminal rendering.

## Design

Peon may emit a summary only when the observed output contains either a
`[User input]:` task instruction or a recognizable command outcome. It omits a
summary when it can only infer terminal state from redraws, ANSI sequences, or
other non-semantic output.

The metadata store accepts a Peon summary as a durable checkpoint only when it
passes that same concrete-work rule. Existing event shape, provenance, and UI
remain unchanged.

## Errors and tests

An omitted or rejected summary still permits status observation; it simply adds
no task-history entry. Tests cover a concrete work summary being persisted and
a vague terminal-state summary being excluded.
