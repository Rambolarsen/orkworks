---
type: "Architecture Decision"
title: "Session-runtime-owned PTY lifetime"
description: "Architecture decision record: Session-runtime-owned PTY lifetime."
tags: ["orkworks", "architecture"]
---

# Session-runtime-owned PTY lifetime

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-07-07

## Context

OrkWorks currently couples PTY/process lifetime to the terminal WebSocket that
feeds `xterm.js`. That makes terminal detach destructive: switching sessions,
disposing an inactive terminal, or losing the renderer-side socket can kill
still-running work. It also prevents the renderer from safely reducing terminal
render load because hidden terminals must remain attached to keep the PTY alive.

ADR 0010 established the `portable-pty` + `xterm.js` stack, and ADR 0013
established the single-active-context constraint for terminal visibility. ADR
0021 made lifecycle ownership explicit via `lifecyclePhase`, but terminal
attachment was still effectively the owner of runtime lifetime.

## Decision

- PTY/process lifetime is owned by the sidecar session runtime, not by a
  renderer WebSocket.
- Terminal attachment is a separate runtime concern with `detached`/`attached`
  state.
- One interactive terminal attachment per session remains the rule.
- Detached session runtimes continue draining PTY output, persisting terminal
  history, and feeding Peon / metadata inference inputs while `orkworksd`
  remains alive.
- Detach does not change `lifecyclePhase`; only process exit, kill, or runtime
  error may drive `ending`/`ended`.
- App-restart PTY persistence is out of scope for the initial implementation.
  After a sidecar restart, persisted sessions reconcile through the existing
  metadata/lifecycle rules rather than being treated as live detached PTYs.

## Consequences

- The frontend can safely dispose inactive terminals and keep only the active
  session attached, reducing renderer cost without killing work.
- The Rust runtime must explicitly own PTY creation, child wait/reap,
  persistence fanout, replay buffering, and attachment ownership tokens.
- WebSocket control messages need typed attach/replay/end semantics rather than
  relying on socket close to imply process termination.
- ADR 0010 remains valid for stack choice, but its operational ownership model
  changes: WebSocket is now transport for attach/detach and PTY output, not the
  lifetime owner of the PTY.
- ADR 0013 remains unchanged: supporting detach/reattach does not permit
  multiple visible or interactive terminals for one session.
- ADR 0021 remains unchanged in lifecycle meaning: `active` vs `ending` vs
  `ended` tracks process/runtime state, not attachment presence.

## Amendment — 2026-10-08: central surfaces preserve one terminal

[ADR 0078](0078-fixed-desktop-shell-and-central-navigation.md) supersedes ADR
0013's session-only definition of visible context while preserving the one
selected-terminal invariant. Terminal may be hidden while Review or an eligible
Workflow surface is central; detaching its renderer attachment continues to
leave PTY lifetime and output draining with the sidecar.
