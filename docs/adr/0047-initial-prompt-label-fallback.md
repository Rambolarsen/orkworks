# Initial-prompt labels are replaceable bootstrap fallbacks

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-09-05

## Context

The New Session dialog can provide an `initialPrompt` that is shared across
several child sessions. That prompt is written directly to the PTY during
startup, so it bypasses the normal terminal-input path that seeds a session's
display label. Treating the initial prompt's synchronous fallback as the
permanent one-shot topic therefore gives related sessions the same title even
when their first actual terminal instructions differ.

Peon's input-label inference is asynchronous. A bootstrap inference can also
finish after a later terminal input, so the durable rule must survive both
daemon restarts and out-of-order inference completion.

## Decision

- Keep a descriptive initial prompt as the immediate synchronous label
  fallback, so a newly-created session has a useful title before Peon runs.
- Persist a `labelFromInitialPrompt` marker with that fallback. The first later
  descriptive, non-sensitive terminal input may replace the fallback once and
  clears the marker. If no initial-prompt fallback exists, the existing
  placeholder-to-first-descriptive-input behavior remains unchanged.
- Tag queued `InputLabel` work that came from the initial prompt. If that work
  finishes after the terminal input has replaced the fallback, discard its
  label instead of restoring the old startup topic. Terminal-derived label
  inference remains allowed for the selected topic.
- Legacy session records default the marker to false. A harness-declared topic
  reset also clears it, so the next descriptive input can seed the new topic.

This refines the stable one-shot label decision restated by [ADR
0042](./0042-workflow-observations-replace-summary-checkpoints.md); the label
remains outside the metadata source/confidence precedence system and is still
frozen after the bootstrap handoff.

## Consequences

- Child sessions can acquire distinct titles even when they share an initial
  startup prompt.
- The title remains stable after the first descriptive terminal topic rather
  than changing on every subsequent input.
- One additional backward-compatible field is stored in session metadata, and
  queued label work carries its source so stale bootstrap results cannot win.
