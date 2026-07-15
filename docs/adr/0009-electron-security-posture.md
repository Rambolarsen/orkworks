---
type: decision
status: accepted
title: "Electron security posture"
---

# Electron security posture

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-06-15

## Context

Electron apps run with full Node.js privileges by default, which is a security risk for an app that embeds terminal sessions and communicates with a local backend. The renderer process should not have direct access to Node.js APIs or the filesystem.

## Decision

We will configure Electron with `nodeIntegration: false` and `contextIsolation: true`. The frontend will communicate with the Rust backend over HTTP/WebSocket through a secure preload bridge. No direct Node.js access from the renderer.

## Consequences

- Renderer is sandboxed from Node.js, reducing attack surface
- All backend communication goes through the preload bridge over HTTP/WS
- Follows Electron security best practices
- Preload bridge must expose a minimal, well-defined API surface
- PTY I/O, terminal rendering, and session management stay on the backend side
