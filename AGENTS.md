# OrkWorks — Agent Guide

## Identity

Local-first observability + recommendation layer for AI coding sessions ("Mission Control for AI Agents"). Observes and recommends before it controls — does not replace Claude Code, Codex, OpenCode, Gemini CLI, or Aider.

## State of the repo

APM project bootstrapped — agent skills, hooks, and plugins are installed via [APM](https://github.com/anthropics/apm) in `orkworks/`. The Electron app, Rust sidecar, and metadata protocol are not yet implemented.

## Issue board

All implementation work is tracked as GitHub issues: https://github.com/Rambolarsen/orkworks/issues

- **Pick new work** from the issue board. Start with M1 issues and work through milestones in order.
- **Add future work** as new issues. Break down into scoped, deliverable-sized issues with checkbox acceptance criteria.
- **Keep issues in sync** with the codebase — close when done, update when scope changes.
- Specs remain authoritative for product scope; issues track implementation progress.

## Authoritative specs

- `specs/orkworks-mvp.md` — full product scope, architecture, milestones, non-goals
- `specs/native-harness-voice-support.md` — voice support design

Read both before starting any implementation work.

## Decision tracking

Architecture decisions are captured as ADRs in `docs/adr/`. Each significant architectural, stack, protocol, or boundary decision gets a numbered markdown file with context, decision, and consequences.

- **Template**: `docs/adr/template.md`
- **Index**: `docs/adr/README.md`
- **Create an ADR** when making a decision that shapes the architecture, stack, or protocol — before or alongside the implementation.
- **Supersede** old ADRs (don't delete) when a decision is reversed or replaced. Mark status `superseded` and reference the new ADR.
- **Keep the index updated** — add each new ADR to the `docs/adr/README.md` table.

ADRs are complementary to specs: specs define what we're building; ADRs record why we chose to build it that way.

## Key naming

| Term | Meaning |
|------|---------|
| OrkWorks | Product |
| `orkworksd` | Rust backend sidecar |
| Peon | Low-cost metadata observer |
| `.orkworks/` | Per-repo protocol directory (sessions/, events/, capacity/, skills/) |

## Planned architecture

```
orkworks/
├─ apps/desktop/          # Electron + React/TypeScript + xterm.js
├─ crates/orkworksd/      # Rust sidecar (Axum HTTP/WS, PTY via portable-pty)
├─ docs/
│  └─ adr/                # Architecture Decision Records
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

## Key conventions from specs

- Do **not** expand fantasy naming beyond "Peon" — use normal engineering terms
- MVP does not own git workflow, worktree management, merging, or task decomposition
- Harness voice is pass-through only — OrkWorks never captures/proxies/stores audio for native voice
- Start every session metadata source and confidence where possible
- Capacity states: healthy, degraded, capped, unknown, disabled
- Cost tiers: local, low, medium, high, premium

## APM

Dependencies are managed by [APM](https://github.com/anthropics/apm) in the `orkworks/` directory. The `apm.yml` defines targets (claude, codex, copilot, opencode) and dependencies. Running `apm install` populates:

| Path | Contents |
|------|----------|
| `orkworks/apm_modules/` | Cloned dependency sources (gitignored) |
| `orkworks/apm.lock.yaml` | Resolved lock file (gitignored) |
| `orkworks/.agents/skills/` | Skills for all targets |
| `orkworks/.claude/` | Claude Code hooks + skills |
| `orkworks/.codex/` | Codex hooks |
| `orkworks/.github/hooks/` | Copilot hooks |
| `orkworks/.opencode/` | OpenCode target |

## Agent plugins / skills

These are harness-level tools — OrkWorks hosts the terminal session; plugins run inside the agent in that session.

### Anthropic Agent Skills (standard)

[anthropics/skills](https://github.com/anthropics/skills) — reference implementation of the Agent Skills standard. Defines the `SKILL.md` format (YAML frontmatter with `name` + `description`, markdown body). Contains `spec/`, `skills/` (examples), `template/`. OpenCode has native built-in skill discovery from `.opencode/skills/`, `~/.config/opencode/skills/`, and Claude-compatible paths.

### Superpowers

[obra/superpowers](https://github.com/obra/superpowers) — agentic skills framework & software development methodology. Installed per-harness (Claude Code, Codex, OpenCode, etc.). OpenCode install:

```json
{
  "plugin": [".opencode/plugins/superpowers.mjs"]
}
```

### Ponytail

[DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail) — minimalist ruleset that enforces YAGNI: check necessity, stdlib, platform feature, dependency, one-liner before writing code. OpenCode install:

```json
{
  "plugin": [".opencode/plugins/ponytail.mjs"]
}
```

Ponytail also ships its own `AGENTS.md` — if cross-referenced from this repo's clone, OpenCode loads both automatically.

### Claude Mem

[thedotmack/claude-mem](https://github.com/thedotmack/claude-mem) — persistent memory for Claude using simple YAML files.

## Maintaining AGENTS.md and README.md

Keep both files current as the project evolves. After any significant change (new dependencies, new architecture, new conventions, changed workflows), update these docs to match reality. Treat stale docs as a bug — if you notice something out of date while working, fix it.
