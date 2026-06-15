# OrkWorks

Local-first observability + recommendation layer for AI coding sessions ("Mission Control for AI Agents"). Observes and recommends before it controls — does not replace Claude Code, Codex, OpenCode, Gemini CLI, or Aider.

## State

APM project bootstrapped — agent skills, hooks, and plugins are installed via [APM](https://github.com/anthropics/apm) in `orkworks/`. The Electron app, Rust sidecar, and metadata protocol are not yet implemented.

## Architecture

```
orkworks/
├─ apps/desktop/          # Electron + React/TypeScript + xterm.js
├─ crates/orkworksd/      # Rust sidecar (Axum HTTP/WS, PTY via portable-pty)
├─ docs/
└─ examples/
```

- Electron launches Rust sidecar; UI talks to it over localhost HTTP/WebSocket
- `nodeIntegration: false`, `contextIsolation: true`
- PTY handles only text I/O; voice (native harness) bypasses PTY entirely

## Metadata protocol

- `.orkworks/sessions/<id>.json` — agent-written session state
- `.orkworks/events/<id>.ndjson` — append-only event log
- `.orkworks/capacity/<id>.json` — capacity per model/harness
- Priority: user > agent > peon > backend_inference > process > unknown
- Peon reads terminal output, writes inferred metadata, never types into terminals

## Setup

```bash
cd orkworks
apm install
```

## Agent plugins

Managed via APM in `orkworks/apm.yml`. Running `apm install` populates skills and hooks for all configured targets (claude, codex, copilot, opencode).

| Plugin | Description |
|--------|-------------|
| [obra/superpowers](https://github.com/obra/superpowers) | Agentic skills framework & methodology |
| [DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail) | YAGNI-minimalist ruleset |
| [thedotmack/claude-mem](https://github.com/thedotmack/claude-mem) | Persistent memory for Claude |

## Issue board

https://github.com/Rambolarsen/orkworks/issues

## Key naming

| Term | Meaning |
|------|---------|
| OrkWorks | Product |
| `orkworksd` | Rust backend sidecar |
| Peon | Low-cost metadata observer |
| `.orkworks/` | Per-repo protocol directory |

## Specs

- `specs/orkworks-mvp.md` — full product scope, architecture, milestones, non-goals
- `specs/native-harness-voice-support.md` — voice support design
