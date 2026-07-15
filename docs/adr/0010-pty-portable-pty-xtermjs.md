---
type: decision
status: accepted
title: "PTY management via `portable-pty`, terminal rendering via `xterm.js`"
---

# PTY management via `portable-pty`, terminal rendering via `xterm.js`

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-06-15

## Context

OrkWorks needs to spawn and manage real terminal sessions across platforms. The frontend needs a terminal emulator that can render PTY output and forward keyboard input. Both components must be cross-platform and well-maintained.

## Decision

The Rust backend will manage PTY sessions using the `portable-pty` crate, which provides a cross-platform API for pseudoterminal I/O. The frontend will use `xterm.js` for terminal rendering, keyboard input forwarding, and resize support. Session I/O is streamed to the frontend over WebSocket.

## Consequences

- `portable-pty` handles macOS/Linux/Windows PTY differences in a single API
- `xterm.js` is the standard web terminal emulator, used by VS Code and many others
- PTY output streaming over WebSocket keeps latency low
- Terminal resize is supported end-to-end
- Voice input (native harness) bypasses PTY entirely, going through a separate path
