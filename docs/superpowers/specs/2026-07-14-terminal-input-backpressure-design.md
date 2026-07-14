# Terminal Input Backpressure Design

> **Date:** 2026-07-14
> **Scope:** Bound memory growth on the renderer-to-PTY input path when the terminal transport or PTY writer is unavailable/stalled

## Goal

Prevent unbounded memory growth on the input path (renderer → sidecar → PTY) when the terminal WebSocket is disconnected or the PTY writer is behind, without silently dropping normal interactive keystrokes.

## Problem

Two related gaps on the input path, both flagged by issue #159:

1. **Renderer (`apps/desktop/src/terminalStore.ts`)**: `term.onData` appends every keystroke/paste to `handle.pendingInput` whenever the WebSocket is not `OPEN` (`handle.pendingInput += data`). There is no cap — a large paste, or automated input, repeated while reconnecting grows this string without bound, then gets flushed as one message on reconnect.
2. **Sidecar (`crates/orkworksd/src/runtime/session_runtime.rs`)**: `SessionRuntime::live()`/`::detached()` construct the `control_tx`/`control_rx` pair via `mpsc::unbounded_channel()`. `send_runtime_command` and `update_runtime_size` push `RuntimeCommand::Input`/`Resize`/`Kill` onto it with a synchronous, always-succeeding `send`. If the PTY writer falls behind (e.g. a hung child process not draining its PTY, or a burst of input/resize events), the queue can grow without bound.

This mirrors the already-fixed output-side problem (`2026-07-07-pty-output-backpressure-design.md`), just in the opposite direction.

## Options Considered

### 1. Bounded channel with real backpressure (blocking send) — sidecar; capped buffer with visible drop — renderer

Sidecar: bound the control channel and make the two send functions `async fn` using `Sender::send(..).await`, so a full queue makes the affected session's websocket read loop wait for room rather than growing the queue. This is the same shape of fix already applied to the output path.

Renderer: cap `pendingInput` at a fixed byte budget; once full, drop further keystrokes while disconnected and surface one visible warning line in the terminal pane (not a silent drop), matching the AC's "documented finite limit or ... clear user feedback" language.

Pros:

- No data loss on the sidecar side — a stalled PTY writer pauses the reader loop for that one session rather than accumulating memory or dropping keystrokes.
- Small, mechanical change: 3 call sites become `.await`, already inside an async loop.
- Matches the precedent set by the output-backpressure fix (bounded queues + real backpressure over drop policies).
- Renderer cap only matters in the disconnected/reconnecting case, which is inherently bounded by the time it takes to reconnect; a fixed byte budget with a visible warning satisfies the AC without inventing new UI infrastructure (reuses the existing `term.writeln(...)` pattern already used for `ended`/`error`/`terminal-unavailable`).

Cons:

- Renderer drop-with-warning does lose data in the disconnected case (accepted trade-off — an unbounded buffer during an indefinite disconnect is worse).

Recommended.

### 2. Sidecar: bounded channel with drop-on-full (`try_send`) instead of blocking

Pros:

- Never blocks the websocket read loop.

Cons:

- Silently loses keystrokes exactly in the stall scenario the issue is about, unless paired with a new control-message protocol addition to surface it — more moving parts than the recommended option for no clear benefit, since blocking briefly on one session's own read loop has no cross-session cost.

Rejected.

### 3. Renderer: no cap, rely on eventual reconnect

Pros:

- No behavior change, simplest.

Cons:

- Doesn't address the issue; a long disconnect with continued input (e.g. an automated agent typing) still grows `pendingInput` without bound.

Rejected.

## Decision

Implement Option 1: bounded + blocking-send backpressure on the sidecar control channel, and a capped + warn-once buffer on the renderer's `pendingInput`.

## Design

### Sidecar control channel (`crates/orkworksd/src/runtime/session_runtime.rs`)

- Add `const CONTROL_CHANNEL_CAPACITY: usize = 64;` near the other runtime constants.
- `SessionRuntime::live()` and `SessionRuntime::detached()`: replace `mpsc::unbounded_channel()` with `mpsc::channel(CONTROL_CHANNEL_CAPACITY)`.
- `send_runtime_command` and `update_runtime_size` become `async fn`, using `tx.send(command).await` (bounded `Sender::send` is async and resolves once there is room, or errors if the receiver was dropped — same error semantics as today via `.map_err(|_| ())`).
- Call sites (`terminal_runtime.rs`, `TerminalAction::Input` / `Resize` / `Kill` in the websocket message loop) add `.await`. All three are already inside an `async fn`/`select!` loop, so this is mechanical — no new spawn or thread needed.
- `RuntimeCommand::Kill` shares the same bounded channel as Input/Resize, consistent with the existing output-backpressure design's finding that the *authoritative* kill path is the separate `kill_tx`/`kill_rx` watch channel (used directly by the HTTP kill-session endpoint) — this in-terminal `Kill` action is a secondary path and does not need its own priority channel.
- Effect: if the PTY writer is stalled, the affected session's own `ws.recv()`-driven select loop pauses on the full send — it does not affect other sessions, and it does not grow memory.

### Renderer pending-input cap (`apps/desktop/src/terminalStore.ts` + `terminalProtocol.ts`)

- Add a pure helper to `terminalProtocol.ts` (matching its existing home for small testable protocol/logic helpers like `parseTerminalControlMessage`):

  ```ts
  export function appendPendingInput(
    current: string,
    incoming: string,
    maxLength: number,
  ): { next: string; dropped: boolean };
  ```

  `maxLength` is measured in JS string `.length` (UTF-16 code units), not encoded bytes — terminal input is overwhelmingly ASCII, so this is a fast, sufficiently-accurate proxy for a byte budget without needing a `TextEncoder` pass on every keystroke. If `current.length + incoming.length` would exceed `maxLength`, return `{ next: current, dropped: true }` (drop the whole incoming chunk, keep what's already buffered); otherwise `{ next: current + incoming, dropped: false }`.

- `TerminalHandle` gains a `pendingInputOverflowed: boolean` field (reset to `false` alongside `pendingInput = ""` on successful flush in `ws.onopen`).
- In `term.onData`, when the socket is not `OPEN`, call `appendPendingInput(handle.pendingInput, data, MAX_PENDING_INPUT_LENGTH)` with `MAX_PENDING_INPUT_LENGTH = 64 * 1024`. If `dropped` is true and `pendingInputOverflowed` was previously `false`, write one warning line via `term.writeln(...)` and set `pendingInputOverflowed = true` (so repeated drops during the same disconnect don't spam the pane).

### Out of scope

- Browser-native `WebSocket.send()` buffering once the socket is `OPEN` (no public API to bound it; not mentioned by the issue).
- Coalescing resize commands (rare enough relative to input volume that the same bounded+blocking path is sufficient).
- Making the channel capacity or byte cap user-configurable.

## Testing

- Rust: a focused test on the bounded control channel proving capacity is enforced (a send blocks until a receive drains a slot), plus re-verifying existing `session_runtime.rs`/`terminal_runtime.rs` tests pass with the now-async call sites (regression coverage for Input/Resize/Kill still functioning).
- Frontend: unit tests for `appendPendingInput` in `terminalProtocol.test.ts` — under cap appends normally, at/over cap drops and reports it, repeated overflow keeps returning `dropped: true` without growing `next`.
- Full `cargo test` (Rust) and `node --experimental-strip-types --test` (frontend) suites green.
