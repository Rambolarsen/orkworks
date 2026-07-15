---
type: decision
status: accepted
title: "Rust backend sidecar with Axum over localhost HTTP/WebSocket"
---

# Rust backend sidecar with Axum over localhost HTTP/WebSocket

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-06-15

## Context

The desktop shell needs a backend process that manages PTY sessions, streams terminal output, and maintains session state. The backend must be performant, handle concurrent sessions, and communicate with the Electron frontend. It must also run as a subprocess of the Electron app on a dynamic localhost port.

## Decision

We will write the backend sidecar (`orkworksd`) in Rust using the Axum web framework. The frontend will communicate with it over HTTP and WebSocket on a dynamically assigned localhost port. Endpoints will include health checks, session management, PTY I/O streaming, terminal resize, and session kill/archive.

## Consequences

- Rust provides memory safety, strong concurrency, and small binary size
- Axum is well-suited for async HTTP/WS with a clean API and strong ecosystem
- Localhost-only binding avoids network exposure by default
- Dynamic port avoids conflicts with other local services
- The Rust-Electron FFI boundary is clean: no native Node addons, just HTTP/WS
