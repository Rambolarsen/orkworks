# Terminal Input Working Status Design

## Goal

Prevent ordinary terminal keystrokes from marking a session as `working`, while preserving the existing behavior that an accepted Enter submission may do so.

## Scope

The sidecar currently treats accepted bare input as enough evidence to transition a session to `working` for harnesses without an active-work hook. Remove that exception. Every accepted input frame will still update input bookkeeping and idle-timer boundaries; completed lines retain the existing label and last-user-input updates. Only a frame containing an Enter line terminator outside bracketed paste may perform the process-sourced working transition.

An empty Enter remains a submission: it can accept a harness prompt's default response and therefore may mark the session as `working`.

## Alternatives considered

1. Keep the bare-keystroke exception for harnesses without hooks. Rejected because it is the reported bug: normal typing changes session state.
2. Infer working from terminal output only. Rejected because it would remove the intentional, immediate status change after submitted input and make prompt acceptance lag behind output inference.
3. Require an Enter line terminator for the input-driven transition. Chosen because it precisely distinguishes composing a command from submitting one while retaining the existing explicit submission behavior.

## Data flow and boundaries

`record_terminal_input_impl` will continue to parse each delivered input frame and determine whether it completes a line. It will always record the accepted-input generation and idle baseline. It will request `ProcessTransition::CommittedWorking` only when `line_completed` is true. Harness active-work hooks and Peon output inference remain independent status sources and are unchanged.

## Error handling

Rejected or dropped input continues to have no metadata effects. Bracketed-paste newlines remain non-submissions. The change does not alter persistence failure handling or source-priority rules.

## Testing

Add a regression test proving a bare keystroke leaves an idle, no-active-hook session idle in both the live handle and persisted metadata while advancing `input_generation`, `accepted_input_at`, `min_peon_output_revision`, and the Peon idle baseline. Retain or add coverage that an accepted Enter transition still changes the same session to `working`, including an empty Enter.

Replace legacy assertions that a bare keystroke enters `working` with the new unchanged-status assertion. In `terminal_runtime.rs`, revise the single-key prompt-transition test and make the first input in the already-working bookkeeping test newline-terminated. In `session_runtime.rs`, update the direct terminal-input and hook-sourced needs-you tests to submit a newline-terminated input and rename them to reflect submission rather than a single key.
