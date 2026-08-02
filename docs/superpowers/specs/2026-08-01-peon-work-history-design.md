# Peon work-history design

## Goal

Task history records concrete session work, not guesses about terminal rendering.

## Design

This applies only to Peon's terminal-derived history. Existing explicit agent
and user checkpoints remain unchanged.

The runtime classifies the exact observed output before it merges Peon's
inference. It supplies the metadata store an optional eligible history summary;
the store uses that value, rather than the model's raw `summary`, for both the
live summary and its durable checkpoint.

Eligible entries are limited to:

- A descriptive `[User input]:` task instruction, summarized by Peon.
- A recognizable `cargo`, `pnpm`, or `npm` test or build command with
  a corresponding success or failure result. These use a fixed summary such as
  `Tests passed` or `Build failed`.

Words such as `error`, `loading`, terminal redraws, ANSI sequences, and
spinners are never evidence. An ineligible model `summary` must neither
replace `SessionMetadata.summary` nor create a history entry; status
observation continues normally.

Existing event shape, provenance, and UI remain unchanged. Previously
persisted history is not rewritten; this prevents future noise only.

## Errors and tests

An omitted or rejected summary still permits status observation; it simply adds
no task-history entry. Tests cover a user-task summary and command outcome
being persisted, plus a vague terminal-state summary being excluded.
