---
type: Architecture Decision
title: Recover gaps in live terminal output
description: Recover retained PTY chunks in order and make unrecoverable gaps visible in the terminal panel.
tags: [orkworks, architecture, terminal, replay]
status: stable
---

# Recover gaps in live terminal output

- Status: accepted
- Deciders: project owner
- Date: 2026-11-27

## Context

[Issue #2104](https://github.com/Rambolarsen/orkworks/issues/2104) tracks silent loss when a live terminal attachment falls behind its broadcast buffer. The sidecar assigns a consecutive cursor to each PTY chunk, saves the chunk in its bounded live replay buffer, then broadcasts it under the session lock. The WebSocket currently skips the lag signal and discards the cursors on live sends.

The [session lifetime contract](../../specs/orkworks-mvp.md#session-lifetime) gives the sidecar ownership of the PTY and allows one interactive attachment. [The live replay implementation](../agents/architecture.md) already retains the latest 256 chunks and a bounded interaction-mode, parser, and UTF-8 checkpoint at the start of that window. The attachment write deadline and cleanup from [PR #2137](https://github.com/Rambolarsen/orkworks/pull/2137) give a stalled attachment a way to release the claim while the PTY continues.

## Decision

### Recover on the same attachment

- Seed the attachment’s next expected cursor from the successful initial `replay-end` boundary. Advance after each successful socket write of a PTY chunk.
- On a broadcast lag signal or a forward cursor jump, capture a contiguous, owned snapshot from the next expected cursor through the current replay high-water mark. Capture under the session lock for the original runtime identity and latest issued attachment generation. An ending runtime remains eligible while that identity and generation still match. Release the lock before any socket write.
- Send the retained chunks in order, discard overlapping live output with cursors below the next expected cursor, and keep the original broadcast receiver for the terminal’s final `Ended` or `Error` event. Recovery must finish before that event is sent. Determine loss by comparing the next expected cursor with the start of the retained replay window.
- Share the existing five-second write budget across the notice and all chunks in each recovery batch. Use the existing attachment cleanup on failed writes, timeout, cancellation, or a stale runtime. Use the existing PTY, input admission, authentication, and persistence contracts.

### Report a gap beyond the replay window

- Before sending the available suffix, send one additive `terminal-output-gap` JSON control message with `fromCursor` and `toCursor` for the half-open missing range, plus `checkpoint` as a byte array. The checkpoint comes from the same locked snapshot and uses the existing replay prefix at its oldest retained cursor. The renderer accepts finite nonnegative safe integer cursors with `fromCursor < toCursor`, and a checkpoint of at most 512 byte values.
- The renderer displays a fixed, accessible notice outside xterm that says **Some terminal output could not be recovered.** The notice belongs to that live attachment and stays visible while its terminal is displayed, without taking keyboard focus. The renderer queues a local cancel and full reset, then the checkpoint and recovered bytes, in xterm’s write order. The reset clears the current screen and in-memory scrollback after an unrecoverable gap. Retained output rebuilds what it can. Saved history remains available through historical replay. A fully recovered gap keeps the display and scrollback intact.
- Keep the current handshake, initial replay markers, and raw binary frames. The recovery control message carries the checkpoint so older renderers can ignore it and continue receiving raw terminal bytes. A newer renderer accepts the original replay and raw binary frames from an older sidecar. The notice is informational.

## Consequences

A healthy attachment can recover a backlog without a disconnect or duplicate terminal writes. When retention has removed output, the user sees an explicit warning and the retained suffix starts from the existing mode and parser checkpoint. Each catch-up batch remains bounded by the replay window and socket write deadline. The checkpoint restores supported modes and parser state. The original screen contents and scrollback before the gap cannot be reconstructed.

## Implementation and checks

1. **Sidecar recovery.** Update the existing replay and socket owners in `runtime/session_runtime.rs` and `runtime/terminal_runtime.rs`. Add deterministic socket tests for full recovery, eviction with the exact missing range, a second lag during recovery, and live replay overlap. Verify that stale attachments cannot read a replacement runtime, terminal end/error follows the last recovered chunk, and a stalled recovery releases the attachment while leaving the PTY running.
2. **Renderer notice and resynchronization.** Extend `terminalProtocol.ts` for the bounded control message and `terminalStore.ts` and its terminal presentation for the fixed notice and ordered local writes. Cover a recoverable gap with no notice or reset, one visible notice on a missing range, malformed controls with no renderer side effects, and a real xterm check that an open control string or partial UTF-8 sequence cannot swallow the reset or the first recovered bytes. Verify attachment switching, disposal, and keyboard focus.
3. **Compatibility checks.** Keep the existing attach/replay and socket deadline tests passing. Exercise both mixed-version directions with the existing raw frame fixtures. Run the repository verifier and the current-head review before merge. Update the runtime concept and the session lifetime spec with the accepted behavior.

The project owner approved the control message and the live screen reset after an unrecoverable gap on 2026-11-27.
