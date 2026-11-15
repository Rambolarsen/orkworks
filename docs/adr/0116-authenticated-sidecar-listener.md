---
type: Architecture Decision
title: Authenticated sidecar listener with a renderer credential
description: Require a renderer, desktop or session credential on every sidecar route except health, and admit only the desktop renderer's browser origins.
tags: [orkworks, architecture, security, sessions]
status: stable
---

# Authenticated sidecar listener with a renderer credential

- Status: accepted
- Deciders: repository owner
- Date: 2026-11-15

## Context

[ADR 0003](0003-rust-backend-axum-localhost.md) put the sidecar on a dynamic
`127.0.0.1` port and treated loopback binding as the access boundary. The
router also answered every browser origin with permissive CORS, and the
terminal WebSocket checked neither a credential nor `Origin`.

A security review on 2026-11-15 showed the consequence against the built
sidecar. A request carrying `Origin: https://evil.example` and no credential
could set the workspace, create a `generic-shell` session, attach to its
terminal and run a command as the user. Any local process or other local
account could do the same without a browser. The desktop token
(`ORKWORKS_OPEN_PLAN_TOKEN`) protected only the handlers that checked it, and
[ADR 0077](0077-taskmaster-orchestrated-child-sessions.md) deliberately keeps
that token out of the renderer, so it could not become the renderer's
credential.

## Decision

Every route except `GET /health` passes one listener-wide access layer
(`crates/orkworksd/src/http/api_auth.rs`) before any handler runs.

1. **Origin.** A request with a browser `Origin` is rejected with `403` unless
   the origin is the packaged file renderer or loopback HTTP (development
   renderer). Chromium sends the packaged renderer's origin as `null` on HTTP
   requests and as `file://` on WebSocket handshakes; both are admitted. CORS grants the same origins and no others. This
   applies to `/health` as well.
2. **Renderer token.** Electron main creates a random 32-byte token once per
   app run and passes it to the sidecar as `ORKWORKS_RENDERER_TOKEN`. The
   renderer reads it over a synchronous IPC call answered only for the
   application window's top-level frame. HTTP requests send it as
   `x-orkworks-renderer-token`. WebSockets offer the subprotocols
   `orkworks.v1` and `orkworks.token.<token>`, because browsers cannot set
   handshake headers; the sidecar selects `orkworks.v1`. Ordinary
   main-process requests carry the same token.
3. **Desktop token.** `x-orkworks-open-plan-token` also passes the layer. It
   stays in Electron main, and privileged handlers keep their own check.
4. **Session token.** A live `ORKWORKS_REPORT_TOKEN` sent as
   `Authorization: Bearer` passes the layer only on the routes a coding tool
   reports to: `POST /sessions/:id/{attention,harness-session,plan-path,workflow-observations}`
   and `GET`/`PATCH /sessions/:id/metadata` for the token's own session, and
   the Taskmaster recommendation read, accept, complete and completion-packet
   routes for any live session. Reporter scripts and the
   `working-on-recommendation` skill send it on every call.
5. **Fail closed.** A sidecar started without `ORKWORKS_RENDERER_TOKEN`
   generates a token nobody knows, so renderer routes reject every request.
   An open listener can be built only by test fixtures.

The renderer token is filtered out of session and Codex probe environments,
like the desktop token. Passing the layer never replaces a handler's own
authorization.

## Consequences

- A web page or an unrelated local process can no longer create sessions,
  attach to terminals, change harness definitions or read session data.
- The renderer holds a credential for ordinary routes only. A compromised
  renderer still cannot call desktop-token routes, which preserves ADR 0077.
- Reporter hooks that predate this change and send no session token are
  rejected with `401` on plan-path, generic attention and OpenCode
  harness-session reports. They report again once the integration is repaired
  from Settings or the session is relaunched with the current scripts.
- Tools that probe sidecar routes other than `/health` without a credential
  now see `401`.
- This is not operating-system process isolation. A process running as the
  same user can read another process's environment on most platforms and so
  obtain a session or renderer token. Sidecar children other than sessions and
  the Codex probe still inherit the sidecar environment.
- Supersedes the access posture in ADR 0003 and the statement in ADR 0077 that
  generic `POST /sessions` is unauthenticated. The origin rule from
  [ADR 0092](0092-workspace-update-stream.md) now applies to every route.
