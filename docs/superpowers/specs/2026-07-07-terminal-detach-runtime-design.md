# Terminal Detach Runtime Design

> **Date:** 2026-07-07
> **Scope:** Decouple PTY/process lifetime from terminal WebSocket attachment so live sessions survive UI detach and can be reattached later

## Goal

Fix terminal lag at the architecture level by making the Rust sidecar own session runtime lifetime while the frontend terminal socket becomes an attach/detach transport.

## Relationship To Existing Design

This design extends [2026-07-05 terminal single-attach](2026-07-05-terminal-single-attach-design.md).

That design correctly established that one session must have at most one live interactive terminal attachment at a time. This design keeps that rule, but changes what the attachment owns:

- before: the attachment owned PTY creation, process lifetime, and terminal I/O
- after: the sidecar-owned session runtime owns PTY/process lifetime, and the attachment only owns one interactive view into that runtime

Single-attach remains correct. WebSocket-owned PTY lifetime does not.

## Problem

Today `handle_session_terminal(...)` both:

- claims terminal ownership for a session, and
- creates and owns the PTY and child process for that session

That coupling creates two problems:

1. Terminal visibility is tied to process lifetime. If the frontend needs to dispose or detach an inactive terminal to reduce renderer load, the backend interprets the closed socket as session termination and kills the child.
2. The runtime cannot support safe detach/reattach. A reconnect is not "show me the same running session again"; it is "start terminal execution again for this session id."

This is the wrong ownership model for OrkWorks. The product is supposed to observe and switch between sessions, not make browser attachment the thing that keeps work alive.

## Options Considered

### 1. Keep the current WebSocket-owned PTY lifetime

Pros:

- smallest implementation
- preserves current control flow

Cons:

- hidden-terminal optimizations remain unsafe
- UI disconnects can kill real work
- session visibility stays coupled to session lifetime

Rejected.

### 2. Treat detach as intentional session termination

Pros:

- simpler than full decoupling
- fewer runtime states

Cons:

- still user-hostile for long-running work
- keeps terminal optimization and session continuity in conflict
- makes transient UI disconnects destructive

Rejected.

### 3. Make the PTY sidecar-owned and attachments reattachable

Pros:

- fixes the ownership bug at the right boundary
- allows renderer load reductions without killing work
- preserves session continuity across panel switches and socket drops
- better matches OrkWorks' "observe and reattach" product role

Cons:

- requires a broader runtime refactor
- introduces explicit attach/detach state and replay semantics

Recommended.

## Decision

The Rust sidecar will own PTY/process lifetime for active sessions. Terminal WebSockets will attach to and detach from an already-running session runtime. At most one interactive attachment may exist per session at a time, but loss of that attachment must not kill the PTY.

"PTY survives detach" means:

- the PTY survives frontend terminal disposal, session switching, and WebSocket disconnect while `orkworksd` is still running
- the PTY does **not** need to survive a full sidecar/app restart in the first implementation

App-restart persistence is explicitly out of scope for this fix.

## Runtime Model

Split the current single lifecycle into two related lifecycles.

### 1. Session runtime lifecycle

Owned by the sidecar:

- `creating`
- `active`
- `ending`
- `ended`

This lifecycle is driven by PTY creation, child process execution, kill/end/error transitions, and session resume behavior.

### 2. Terminal attachment lifecycle

Owned by the frontend WebSocket:

- `detached`
- `attached`

This lifecycle only controls whether an interactive viewer is connected. Transitioning from `attached` to `detached` must not change the session runtime lifecycle by itself.

## Recommended Architecture

### Backend ownership

The sidecar should create the PTY and spawn the child process at session creation/resume time, not inside `handle_session_terminal(...)`.

`SessionHandle` should evolve from "metadata plus spawn inputs" into "metadata plus live runtime owner". Concretely, the in-memory session runtime should own:

- PTY/process handles
- task handles for PTY read/write loops
- recent output buffer for replay on attach
- persisted terminal history integration
- attachment state for the current interactive client
- attachment token / generation for owner-scoped cleanup
- last known terminal size for detached startup and later resizes
- output cursor or equivalent replay marker

The frontend socket should attach to this runtime, receive recent buffered output, then continue with live stream delivery.

While detached, the session runtime must continue to:

- drain PTY output
- persist terminal history
- feed Peon / metadata inference inputs
- advance output cursors and replay buffers

WebSocket delivery is optional fanout layered on top of the runtime reader. It must not be the thing that keeps PTY draining alive.

### Attachment rules

- at most one interactive attachment per session
- first implementation policy: reject a new interactive attach when another live attachment still exists
- superseding a live attachment is out of scope for the first implementation and requires a separate design
- stale attachments should be explicitly cleaned up so a real reattach can succeed, but cleanup must be owner-scoped
- detach closes only the socket
- kill/end/error transitions kill the PTY and end any attachment

Stale-owner cleanup contract:

- the runtime owns an attachment token or generation for the currently attached client
- cleanup may only detach the matching token after a defined stale-owner check such as WebSocket close/error or heartbeat timeout
- cleanup must never clear attachment ownership by session id alone
- a rejected second attach must not clear or replace the current attachment token

### Frontend behavior

- switching sessions detaches from the old session and attaches to the new one
- hidden sessions do not need live xterm instances just to keep their work running
- on attach, the client receives recent buffered output first, then live stream continuation
- on first attach after detached startup, the client sends the current terminal size and the runtime resizes the existing PTY without restarting it

### Initial terminal sizing

Detached startup still needs a size contract. The first implementation should:

- use the last known terminal size when the runtime already has one for that session
- otherwise start the PTY at a fixed fallback size of `24x80`
- resize the PTY on first attach when the frontend reports its actual rows/cols

This avoids making durable terminal-size persistence a prerequisite for the refactor while still defining deterministic startup behavior.

## Output And Replay

Reattach needs more than persisted terminal history. The session runtime should maintain a sidecar-owned in-memory replay buffer for live sessions so the next attachment can render the recent terminal state immediately without waiting for new PTY output.

First implementation requirements:

- preserve the existing persisted terminal output path
- add replay semantics for the recent in-memory live buffer
- bound the replay buffer by bytes, lines, or both; truncation is acceptable as long as live continuation is reliable
- do not require perfect full-screen TUI reconstruction beyond the existing scan/buffer model

Correctness matters more than perfect visual fidelity in the first pass. Reattach must be reliable even if replay is only "recent output plus continued live stream."

Attach/replay handoff contract:

- runtime output must advance under a monotonic sequence number, byte offset, or equivalent cursor
- an attaching client subscribes at a cursor
- the runtime replays buffered output from that cursor
- live streaming continues from the same stream/cursor boundary without gaps or ownership races

The implementation may tolerate bounded duplicate replay better than dropped output, but it must define one explicit cursor model so detach/reattach does not depend on timing luck.

## Failure Handling

Required behavior:

- WebSocket disconnect alone must not kill the PTY
- PTY/process failure must still finalize the session
- a second interactive attach must never create a second PTY
- resume/kill/delete paths must operate on the sidecar-owned runtime, not on attachment existence
- persisted sessions from an earlier `orkworksd` process must still reconcile through existing metadata/lifecycle rules, not be treated as live detached PTYs

The implementation should prefer explicit cleanup over inference. If an attachment disappears unexpectedly, the runtime should mark the session as detached and remain active until the process actually ends or is explicitly killed.

## Testing Strategy

Minimum regression coverage should include:

- session runtime can start without any terminal attachment
- detaching an attachment does not kill the PTY or transition the session to ended
- reattaching to a detached active session resumes output delivery
- second interactive attach is rejected while a live attachment exists and never spawns a second PTY
- stale-owner cleanup only clears the matching attachment token / generation
- kill/end/error transitions still finalize the session and close any attachment
- resume paths still behave correctly with detached runtimes
- detached runtimes continue draining PTY output, persisting history, and feeding Peon/metadata
- detached startup uses the defined initial PTY size contract
- attach replay uses the cursor handoff contract without dropped output across replay/live transition

The first implementation should favor targeted Rust tests around runtime ownership and detach/reattach semantics. Frontend tests should focus on attach/detach orchestration rather than trying to emulate full PTY behavior.

## ADR Requirement

This is a load-bearing architecture decision. Before implementation code lands, add a new ADR describing:

- PTY lifetime is session-runtime-owned, not WebSocket-owned
- one interactive attachment per session remains the rule
- detach/reattach is supported while the sidecar remains alive
- app-restart runtime persistence is out of scope for the initial design
- detach does not change `lifecyclePhase`; only process exit, kill, or runtime error does

The ADR must explicitly align with:

- ADR 0010 for PTY/xterm architecture ownership
- ADR 0013 for the single-active-context constraint
- ADR 0021 for session lifecycle phase semantics and terminal finalization behavior

If ADR 0010, ADR 0021, or another existing ADR is materially contradicted by the new behavior, supersede it rather than editing history in place.

## Non-Goals

- supporting multiple simultaneous interactive viewers for one session
- persisting live PTY runtime across full app/backend restart
- redesigning the terminal persistence format beyond what replay needs
- adding workflow-control features outside terminal/session runtime ownership
- perfect TUI screen-state reconstruction in the first implementation

## Acceptance Criteria

- closing or detaching a terminal WebSocket does not kill a still-running session
- reattaching to a live detached session shows recent output and resumes live streaming
- one session cannot spawn multiple PTYs because of multiple terminal attaches
- session runtime lifetime is owned by the sidecar, not by the renderer socket
- frontend terminal disposal for inactive sessions becomes safe
- regression tests cover detach, reattach, single-attach enforcement, and terminal end/kill behavior
