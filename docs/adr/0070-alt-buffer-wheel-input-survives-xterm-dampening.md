# Alternate-buffer wheel input survives xterm dampening

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-09-29

## Context

OrkWorks uses xterm.js 6.0.0 to render live harness terminals. Full-screen
terminal applications commonly switch to the alternate buffer, which has no
xterm scrollback, so wheel input must be forwarded to the application. xterm
calls `consumeWheelEvent()` in both its mouse-report path and its passive
alternate-buffer wheel path. The return value can be zero for a nonzero
gesture: small pixel deltas are dampened and rounded down, and shift-wheel is
ignored by the accumulator. Each path then drops the event even though it only
needs the direction to send one wheel report or one up/down key sequence.

This affects any full-screen TUI hosted in OrkWorks, including Codex and
OpenCode. Upstream issue [xterm.js #6105](https://github.com/xtermjs/xterm.js/issues/6105)
remains open, and [PR #6118](https://github.com/xtermjs/xterm.js/pull/6118)
has stalled. Its proposed change removes the gate from the passive path but
does not remove the corresponding gate from the mouse-report path used when a
TUI enables mouse tracking.

## Decision

Keep xterm's wheel accumulator call in both paths so its partial-scroll state
stays synchronized. In the mouse-report path, preserve the accumulator gate
while the active buffer has scrollback and bypass it only when that buffer has
no scrollback. In the passive path, which only runs when the active buffer has
no scrollback, send one direction sequence for every nonzero vertical event.
Preserve the explicit `deltaY === 0` guard and the application-owned wheel
handler behavior. Carry this minimal change as a pnpm patch against the exact
xterm version; verify small and large input with and without TUI mouse tracking,
and verify normal-buffer mouse dampening before removing the patch after an
upstream release provides equivalent behavior.

## Consequences

- Trackpad and other small wheel deltas reach full-screen terminal apps in
  either mouse-tracking mode.
- The app sends one direction event per nonzero wheel event in alternate-buffer
  mode; the magnitude remains unused, matching xterm's existing behavior.
- Mouse-report events in a normal buffer with scrollback continue to honor
  xterm's accumulated line threshold.
- OrkWorks owns a small version-specific patch and must revalidate it when
  upgrading xterm.js.
- Zero-delta events keep their existing behavior.
