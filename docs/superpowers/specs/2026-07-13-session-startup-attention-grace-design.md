# Session startup attention grace design

## Problem

Every live session currently becomes `working` when its PTY emits any visible
output. Harnesses normally print banners, prompts, or other startup text, so a
newly launched session is briefly reported as Working even when it has not yet
begun work.

## Decision

Add a two-second, per-session startup grace period in the sidecar runtime.
During that period, generic terminal output remains fully persisted, replayed,
and visible, but does not update inferred attention from Idle to Working.
Once the deadline passes, visible terminal output follows the existing
process-inference behavior.

## Scope and behavior

- The grace period begins when a new session runtime is launched.
- It applies to all harnesses because it protects the shared terminal-output
  inference path rather than recognising individual banners.
- Existing explicit signals, including agent/harness-reported attention such as
  `waiting_for_input` and `blocked`, are not delayed or suppressed.
- Idle timing still records startup output normally, so a quiet session becomes
  idle through the existing timeout path.
- The deadline is runtime-only state; it is not persisted and does not apply to
  restored historical sessions.

## Alternatives considered

1. Suppress only the first output chunk. Rejected because startup banners can
   arrive in multiple chunks.
2. Filter known harness banners. Rejected because it is fragile and requires
   harness-specific maintenance.
3. Apply a two-second shared startup grace period. Chosen because it is small,
   harness-neutral, and preserves genuine output handling immediately after
   launch.

## Verification

Add runtime tests proving that visible output during the grace period does not
mark a new live session Working, while output after the deadline does. Run the
targeted Rust tests and the full Rust test suite.
