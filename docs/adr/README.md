# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for OrkWorks.

See [ADR 0001](./0001-record-architecture-decisions.md) for the rationale.

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](./0001-record-architecture-decisions.md) | Record architecture decisions | accepted |
| [0002](./0002-electron-react-typescript-desktop.md) | Electron + React + TypeScript desktop shell | accepted |
| [0003](./0003-rust-backend-axum-localhost.md) | Rust backend sidecar with Axum over localhost HTTP/WS | accepted |
| [0004](./0004-orkworks-metadata-protocol.md) | `.orkworks/` metadata protocol directory structure | superseded by [0018](./0018-global-metadata-store.md) |
| [0018](./0018-global-metadata-store.md) | Move metadata store from workspace directory to global config directory | accepted |
| [0005](./0005-metadata-source-priority.md) | Metadata source priority | accepted |
| [0006](./0006-peon-observer-only-mvp.md) | Peon: observer-only inference in MVP | accepted |
| [0007](./0007-product-boundary-observe-recommend.md) | Product boundary: observe and recommend before controlling | accepted |
| [0008](./0008-git-context-detection-not-control.md) | Git context detection first, not workflow control | accepted |
| [0009](./0009-electron-security-posture.md) | Electron security posture | accepted |
| [0010](./0010-pty-portable-pty-xtermjs.md) | PTY management via `portable-pty`, terminal via `xterm.js` | accepted |
| [0011](./0011-dockview-panel-layout.md) | Replace `react-resizable-panels` with `dockview` for draggable panel layout | accepted |
| [0012](./0012-peon-repo-scope.md) | Peon scope expands to per-repo | accepted |
| [0013](./0013-single-active-context-primitive.md) | Single-active-context primitive: session = context, switching = context-switch | accepted |
| [0014](./0014-main-process-owned-app-settings.md) | Main-process-owned app settings and menu accelerators | accepted |
| [0015](./0015-provider-ops-peon-fallback.md) | Provider ops panel and app-wide Peon fallback | superseded by 0016 |
| [0016](./0016-session-details-provider-context.md) | Session details provider context | superseded by 0017 (Settings surface) |
| [0017](./0017-provider-context-session-scoped.md) | Provider context is session-scoped, not app-wide | superseded (peon model picker restores per-provider model selection in Settings) |
| [0019](./0019-attention-signal-endpoint-opt-in-hook-install.md) | Attention signal via unauthenticated localhost endpoint, opt-in hook install only | accepted |
| [0020](./0020-phosphor-visual-refresh-token-layer.md) | Phosphor visual refresh: cool-graphite + lime token layer | accepted |
