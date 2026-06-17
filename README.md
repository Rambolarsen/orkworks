# OrkWorks

Local-first observability + recommendation layer for AI coding sessions ("Mission Control for AI Agents"). Observes and recommends before it controls — does not replace Claude Code, Codex, OpenCode, Gemini CLI, or Aider.

## State

APM project bootstrapped — agent skills, hooks, and plugins are installed via [APM](https://github.com/anthropics/apm) in `orkworks/`. M1 (Electron app shell + Rust sidecar scaffold) is implemented. Subsequent milestones are tracked as GitHub issues.

## Architecture

```text
orkworks/
├─ apps/desktop/          # Electron + React/TypeScript + Dockview + xterm.js desktop UI
├─ crates/orkworksd/      # Rust sidecar (Axum HTTP/WS, PTY via portable-pty)
├─ docs/
│  └─ adr/                # Architecture Decision Records
├─ skills/                # Repo-level agent skills
└─ examples/
```

- Electron launches Rust sidecar; UI talks to it over localhost HTTP/WebSocket
- `nodeIntegration: false`, `contextIsolation: true`
- Desktop UI uses Dockview draggable panels for sessions, detail, terminal, capacity, and recommendations
- Peon writes observer metadata such as `observedStatus` without replacing runtime lifecycle `status`
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

Development agents should follow `AGENTS.md`, including the requirement to invoke and follow relevant Superpowers skills before implementation, debugging, review, verification, commit, push, or PR work.

OpenCode must be started from the repo root, or with the repo root as the project path, so it loads the project `opencode.json`:

```bash
opencode /Users/froomiebot/workspace/orkworks
```

| Plugin | Description |
| ------ | ----------- |
| [obra/superpowers](https://github.com/obra/superpowers) | Agentic skills framework & methodology |
| [DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail) | YAGNI-minimalist ruleset |
| [thedotmack/claude-mem](https://github.com/thedotmack/claude-mem) | Persistent memory for Claude |

## Repo skills

The `skills/` directory contains repo-level agent skills that are committed with the project. These follow the [Agent Skills standard](https://agentskills.io/specification) — each skill is a directory with a `SKILL.md` file (YAML frontmatter + markdown body).

| Skill | Description |
| ----- | ----------- |
| [writing-skills](skills/writing-skills/SKILL.md) | TDD-based skill creation following the Agent Skills standard |
| [clean-ddd-hexagonal](skills/clean-ddd-hexagonal/SKILL.md) | Clean Architecture + DDD + Hexagonal patterns, language-agnostic |

## Issue board

[https://github.com/Rambolarsen/orkworks/issues](https://github.com/Rambolarsen/orkworks/issues)

## Key naming

| Term | Meaning |
| ---- | ------- |
| OrkWorks | Product |
| `orkworksd` | Rust backend sidecar |
| Peon | Low-cost metadata observer |
| `.orkworks/` | Per-repo protocol directory |

## Specs

- `specs/orkworks-mvp.md` — full product scope, architecture, milestones, non-goals
- `specs/native-harness-voice-support.md` — voice support design
