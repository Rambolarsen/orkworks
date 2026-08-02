# Terminal Input Working Status Design

## Goal

Prevent ordinary terminal keystrokes from marking a session as `working`, while preserving the existing behavior that an accepted Enter submission may do so.

## Scope

The sidecar currently treats accepted bare input as enough evidence to transition a session to `working` for harnesses without an active-work hook. Remove that exception. Every accepted input frame will still update input bookkeeping and idle-timer boundaries; completed lines retain the existing label and last-user-input updates. Only a frame containing an Enter line terminator outside bracketed paste may perform the process-sourced working transition.

An empty Enter remains a submission: it can accept a harness prompt's default response and therefore may mark the session as `working`.

## Alternatives considered

1. Keep the broad bare-keystroke exception for hookless harnesses. Rejected because it is the reported bug: shell-session typing changes session state. (This was the `!active_work_hook && metadata_source != "peon"` gate that PR #183 introduced and commit `31f9b4e` reverted.)
2. Infer working from terminal output only. Rejected because it would remove the intentional, immediate status change after submitted input and make prompt acceptance lag behind output inference.
3. Require an Enter line terminator for the input-driven transition. Chosen because it precisely distinguishes composing a command from submitting one while retaining the existing explicit submission behavior.

## Narrowed exception (recorded by issue #273)

The rejection in alternative 1 above only covers the **broad** bare-keystroke
gate. Issue #273 reintroduces a narrowly-scoped arming exception (not a direct
`working` commit) for the case `31f9b4e` inadvertently broke: Claude Code's
hook POSTs `needs_you` with `metadata_source = "agent"`, and its prompts take
single keystrokes (`y`/`n`/`1`/`2`/`3`/`Esc`) with no Enter. PR #183's broad
gate admitted ordinary shell sessions, which echo each keystroke, so commit
`31f9b4e` correctly reverted it. The narrowed gate is the conjunction:

- `attention == "needs_you"`
- `metadata_source == "agent"` (so shell sessions — whose source is `process`
  or `None` — are excluded by construction)
- `!active_work_hook` (so harnesses whose own hook is the source of truth for
  work start are excluded)
- this frame actually grew `input_buf` (a printable char was pushed, not a
  pure ANSI control sequence such as an arrow key)

In that exact state, the bare keystroke arms `pending_work_signal` (the same
10-second output-gated fallback an Enter-terminated submission arms in the
parent design) but does **not** directly commit `working`. The next visible
PTY chunk then promotes through the existing `consume_pending_work_signal`
path. The arming-only shape means this exception does not reintroduce the
premature-working bug that motivated `31f9b4e`: shell sessions still never enter
this gate, and even for hook-sourced `needs_you`, attention only advances on
later model output, not on the keystroke itself.

See `docs/superpowers/specs/2026-07-17-single-key-work-signal-design.md` for
the full gate, edge cases, and echo-prefix treatment. The 2026-07-14 parent
spec (`docs/superpowers/specs/2026-07-14-harness-work-state-design.md`,
"Fallback for unsupported harnesses") records the same exception in its
fallback section and points at the 2026-07-17 doc for the details. This
2026-07-28 spec's "Goal" still holds: bare keystrokes never directly commit
`working`. The narrowed exception only arms the output-gated fallback for the
exact hook-sourced case; without later model output the session stays
`needs_you`, exactly as `31f9b4e` intended for the no-Enter case.

## Data flow and boundaries

`record_terminal_input_impl` will continue to parse each delivered input frame and determine whether it completes a line. It will always record the accepted-input generation and idle baseline. It will request `ProcessTransition::CommittedWorking` only when `line_completed` is true. Harness active-work hooks and Peon output inference remain independent status sources and are unchanged.

## Error handling

Rejected or dropped input continues to have no metadata effects. Bracketed-paste newlines remain non-submissions. The change does not alter persistence failure handling or source-priority rules.

## Testing

Add a regression test proving a bare keystroke leaves an idle, no-active-hook session idle in both the live handle and persisted metadata while advancing `input_generation`, `accepted_input_at`, `min_peon_output_revision`, and the Peon idle baseline. Retain or add coverage that an accepted Enter transition still changes the same session to `working`, including an empty Enter.

Replace legacy assertions that a bare keystroke enters `working` with the new unchanged-status assertion. In `terminal_runtime.rs`, revise the single-key prompt-transition test and make the first input in the already-working bookkeeping test newline-terminated. In `session_runtime.rs`, update the direct terminal-input and hook-sourced needs-you tests to submit a newline-terminated input and rename them to reflect submission rather than a single key.
