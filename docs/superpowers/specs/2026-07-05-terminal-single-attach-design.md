# Terminal Single-Attach Design

> **Date:** 2026-07-05
> **Scope:** Prevent a second concurrent terminal WebSocket from spawning a second PTY for one session

## Goal

Implement issue `#118` by making terminal ownership explicit in the Rust runtime so one session can have at most one live terminal attachment at a time.

## Problem

Today `handle_session_terminal(...)` accepts any non-terminal session state and unconditionally opens a new PTY and child process. The frontend's `ensureTerminal(...)` map avoids duplicate sockets in the happy path, but the backend does not enforce single ownership.

That leaves a real race:

- a second WebSocket can connect for the same session while the first is still active
- the runtime spawns a second PTY and child under the same session id
- terminal output and metadata updates now come from two writers
- when either socket exits, it can drive session finalization for a session that still has another live child

This is a backend ownership bug, not a frontend coordination issue.

## Recommended Approach

Reject the second concurrent terminal attachment.

The session's in-memory runtime handle should carry an explicit `terminal_attached` boolean. A terminal connection must claim that flag before it opens a PTY or spawns a child process. If another connection arrives while the flag is already set, the backend should reject it immediately and leave the existing PTY, child, and session status untouched.

The claim/release contract should be represented as an ownership-scoped helper, not open-coded boolean mutation spread across early-return branches. Concretely, the runtime should expose a narrow helper such as `try_claim_terminal_attachment(...) -> Option<TerminalAttachGuard>`, where only a successful claimant receives the guard and only that guard releases the attachment on drop.

This is preferred over "supersede the old socket" because supersession would require splitting PTY ownership from the current WebSocket task, moving lifetime management to a broader session runtime abstraction, and carefully preserving input/output routing across socket handoff. That is unnecessary for issue `#118`.

## Runtime Design

### Session handle

Add a `terminal_attached: bool` field to the Rust `SessionHandle`.

Meaning:

- `false`: no terminal runtime currently owns the session
- `true`: one terminal runtime has claimed the session and is responsible for the PTY, child process, and terminal I/O loop

This flag is in-memory runtime state only. It is not persisted to metadata.

### Claim rules

When `handle_session_terminal(...)` starts:

1. under a single `state.sessions` mutex lock, confirm the session exists, is not already terminal (`killed`, `ended`, `error`), and has `terminal_attached == false`
2. if those checks pass, set `terminal_attached = true` in that same critical section and return an ownership guard/token to the caller
3. if the flag is already `true`, reject the new socket and return without spawning a PTY
4. if the claim succeeds, continue with normal terminal startup while holding the ownership guard

The claim must happen before PTY creation so the guard actually prevents the duplicate-child bug. The read/check/set sequence must be atomic under one lock; a split “read under one lock, then set under another” implementation is not acceptable because it can still admit two owners.

The lifecycle gate must reject more than terminal outcome states. A session in `lifecycle_phase = ending` must also reject new terminal attaches even if its visible `status` is still `running`, because terminal exit paths currently transition through `ending` before final terminal status is committed. Claim logic should therefore reject when either:

- `status` is already terminal (`killed`, `ended`, `error`), or
- `lifecycle_phase` is `ending` or `ended`, or
- equivalent pending-finalization state exists in the in-memory session handle

### Release rules

The terminal runtime must always release `terminal_attached` before returning, regardless of how the connection ends:

- session missing after initial checks
- pre-start kill path
- terminal-state rejection
- PTY open failure
- child spawn failure
- WebSocket close before or during the main loop
- normal child exit
- kill signal path
- runtime error path

The implementation should centralize release so early returns do not leak the flag into the `true` state.

Release must be owner-scoped:

- only a connection that successfully claimed the session may release `terminal_attached`
- a duplicate attach that was rejected because the flag was already `true` must not run shared cleanup that resets the flag
- the preferred shape is for the successful claimant to hold a guard whose `Drop` performs the release

### Rejection behavior

When a second attach is rejected:

- log a warning with the session id
- send a short text reason if practical
- close the WebSocket
- do not change session status
- do not kill the active child
- do not write terminal output
- do not finalize the session

This preserves the existing session as the sole owner of terminal execution.

## Handle Reuse Rules

`SessionHandle` is reused by some resume paths instead of always being replaced. The ownership flag needs explicit reset semantics for that shape.

Rules:

- new `SessionHandle` instances initialize `terminal_attached = false`
- resume/recreate paths that mutate an existing handle in place must also leave `terminal_attached = false` before any new terminal attach begins
- those paths may only reset the flag when the previous terminal owner is definitively gone
- resume/recreate logic must not clear a live owner's claim as part of generic session-field reinitialization
- resume/recreate logic must check `terminal_attached` under the same `state.sessions` lock before mutating an existing handle
- if a live owner is still attached, resume/recreate must return a conflict or otherwise defer instead of rewriting `info`, `kill_tx`, buffers, or command fields underneath the active terminal runtime

This keeps the ownership flag aligned with the actual runtime owner instead of treating it as just another field to overwrite during session setup.

## Frontend Impact

No renderer architecture change is required.

The existing `terminalStore.ts` behavior is sufficient for the rejected-duplicate case:

- the duplicate socket closes quickly
- input is disabled for that handle on `onclose`
- if no live data was received, the renderer falls back to persisted terminal output

This is acceptable for issue `#118` because the main requirement is to prevent the backend from spawning a second PTY. If the UX around rejected duplicate attaches is later considered confusing, that can be handled as a separate follow-up.

## Testing

Add a Rust test that proves the ownership guard.

Required coverage:

- a first attach can claim the session
- a second concurrent attach is rejected while the first claim is still held
- rejecting the second attach does not mutate session status
- rejecting the second attach does not release or clear the first claim
- releasing the first claim allows a later attach attempt
- only one claimant can win when claim attempts race
- successful owners release through the centralized guard path

The test should target the ownership guard directly if possible, rather than requiring an end-to-end PTY/WebSocket integration test. The bug is about claim semantics, and the narrow unit under test should reflect that.

## Non-Goals

- supporting seamless WebSocket handoff between clients
- preserving a PTY across terminal-owner replacement
- changing terminal persistence format
- changing session lifecycle finalization rules beyond preventing the duplicate-owner race

## Acceptance Criteria

- a second concurrent terminal WebSocket for one session cannot spawn a second PTY
- the active terminal connection remains unaffected when the duplicate attach is rejected
- a rejected duplicate attach cannot clear the active connection's ownership claim
- claim/check/set happens atomically under one `sessions` lock
- sessions already in `ending` or `ended` lifecycle cannot be re-attached
- the claim flag is released on every exit path so reconnects still work after disconnect or failure
- Rust tests cover the claim/reject/release behavior
