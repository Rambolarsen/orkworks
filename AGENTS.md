# OrkWorks — Agent Guide

## Identity

Local-first observability + recommendation layer for AI coding sessions ("Mission Control for AI Agents"). Observes and recommends before it controls — does not replace Claude Code, Codex, OpenCode, Gemini CLI, or Aider.

## State of the repo

APM project bootstrapped — agent skills, hooks, and plugins are installed via [APM](https://github.com/anthropics/apm) in `orkworks/`. M1 (Electron app shell + Rust sidecar scaffold) is implemented. Subsequent milestones are tracked as GitHub issues.

## Package manager

Use **pnpm** for all Node.js package management. Do not use npm or yarn for project package management tasks.

```bash
# Install pnpm if missing
corepack enable
corepack prepare pnpm@latest --activate

# Install deps
cd apps/desktop && pnpm install

# Run dev
pnpm dev
```

## Issue board

All implementation work is tracked as GitHub issues: [https://github.com/Rambolarsen/orkworks/issues](https://github.com/Rambolarsen/orkworks/issues)

- **Pick new work** from the issue board. Start with the lowest incomplete milestone and work through milestones in order.
- **Add future work** as new issues. Break down into scoped, deliverable-sized issues with checkbox acceptance criteria.
- **Keep issues in sync** with the codebase — close when done, update when scope changes.
- If the issue board is inaccessible, do not guess at priorities. Stop and inform the user that issue board access is required before picking or closing work.
- Specs remain authoritative for product scope; issues track implementation progress.
- If an issue describes work not covered by the specs, do not implement it. Add a comment on the issue noting the gap and ask for a spec update.
- If the specs describe work with no corresponding issue, create one before implementing.

## Authoritative specs

- `specs/orkworks-mvp.md` — full product scope, architecture, milestones, non-goals
- `specs/native-harness-voice-support.md` — voice support design

Read both before starting any implementation work.

If either spec file is missing or unreadable, stop and notify the user before proceeding. Do not infer scope from context alone.

## Development workflow

Agents doing development work in this repo must use the installed Superpowers skills as workflow guardrails, not just mention them as available tools. Before acting, check whether a relevant skill applies and load/follow it through the harness skill mechanism.

- Start each task by checking for applicable skills; if one might apply, invoke it before responding or editing.
- Use `brainstorming` before creating features, building components, adding functionality, or modifying behavior.
- Use `writing-plans` for multi-step implementation work after scope is understood.
- Use `test-driven-development` for feature and bugfix implementation unless the change is docs-only, config-only, or the user explicitly opts out.
- Use `systematic-debugging` before fixing bugs, test failures, or unexpected behavior.
- Use `receiving-code-review` when responding to review feedback.
- Use `verification-before-completion` before claiming work is complete, committing, pushing, or opening a PR.
- Use `requesting-code-review` for substantial implementation work before merge/PR handoff.

These workflow requirements constrain how agents work in this repository. They do not expand OrkWorks product scope or override the MVP non-goals.

### OpenCode requirement

OpenCode must load the project-level `opencode.json` at the repo root. Start OpenCode with the repo root as the project, for example:

```bash
opencode /Users/froomiebot/workspace/orkworks
```

Do not use `--pure` for development work in this repo; it disables external plugins. The root `opencode.json` loads the APM-managed Superpowers and Ponytail plugins and exposes both `orkworks/.agents/skills` and committed repo skills from `skills/`.

Before OpenCode implementation work, verify that the skill tool lists Superpowers skills such as `superpowers/using-superpowers` and `superpowers/brainstorming`. If they are missing, stop, run `cd orkworks && apm install`, restart OpenCode from the repo root, and verify again before editing code.

## Decision tracking

Architecture decisions are captured as ADRs in `docs/adr/`. Each significant architectural, stack, protocol, or boundary decision gets a numbered markdown file with context, decision, and consequences.

- **Template**: `docs/adr/template.md`
- **Index**: `docs/adr/README.md`
- **Create an ADR** before writing any implementation code for a decision that shapes the architecture, stack, or protocol. If the decision only becomes clear during implementation, pause, write the ADR, and continue.
- A decision is reversed or replaced when: (a) a new ADR explicitly contradicts a prior ADR, or (b) implementation diverges from what an existing ADR records.
- **Supersede** old ADRs (don't delete) when a decision is reversed or replaced. In case (b), write the new ADR first, then update the old ADR status to `superseded` and reference the new ADR number.
- **Keep the index updated** — add each new ADR to the `docs/adr/README.md` table.

ADRs are complementary to specs: specs define what we're building; ADRs record why we chose to build it that way.

## Key naming

| Term | Meaning |
| ---- | ------- |
| OrkWorks | Product |
| `orkworksd` | Rust backend sidecar |
| Peon | Low-cost metadata observer |
| `.orkworks/` | Per-repo protocol directory (sessions/, events/, capacity/, skills/) |

## Planned architecture

```text
orkworks/
├─ apps/desktop/          # Electron + React/TypeScript + xterm.js
├─ crates/orkworksd/      # Rust sidecar (Axum HTTP/WS, PTY via portable-pty)
├─ docs/
│  └─ adr/                # Architecture Decision Records
├─ skills/                # Repo-level agent skills
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
- If asked to implement something listed as a non-goal in the specs, decline and explain which non-goal applies. Do not implement it even partially.
- Harness voice is pass-through only — OrkWorks never captures/proxies/stores audio for native voice
- Start every session metadata source and confidence where possible
- Capacity states: healthy, degraded, capped, unknown, disabled
- Cost tiers: local, low, medium, high, premium

## APM

Dependencies are managed by [APM](https://github.com/anthropics/apm) in the `orkworks/` directory. The `apm.yml` defines targets (claude, codex, copilot, opencode) and dependencies. Running `apm install` populates generated agent assets, commonly including:

| Path | Contents |
| ---- | -------- |
| `orkworks/apm_modules/` | Cloned dependency sources (gitignored) |
| `orkworks/apm.lock.yaml` | Resolved lock file (gitignored) |
| `orkworks/.agents/skills/` | Skills for all targets |
| `orkworks/.claude/` | Claude Code hooks + skills |
| `orkworks/.codex/` | Codex hooks |
| `orkworks/.github/hooks/` | Copilot hooks, when generated for the configured target |
| `orkworks/.opencode/` | OpenCode target |

## Agent plugins / skills

These are harness-level tools — OrkWorks hosts the terminal session; plugins run inside the agent in that session.

## Repo-level skills

The `skills/` directory contains repo-level agent skills committed with the project. These follow the Agent Skills standard: each skill is a directory with a `SKILL.md` file using YAML frontmatter and a markdown body.

| Skill | Description |
| ----- | ----------- |
| `writing-skills` | TDD-based skill creation following the Agent Skills standard |
| `clean-ddd-hexagonal` | Clean Architecture + DDD + Hexagonal patterns, language-agnostic |

### Anthropic Agent Skills (standard)

[anthropics/skills](https://github.com/anthropics/skills) — reference implementation of the Agent Skills standard. Defines the `SKILL.md` format (YAML frontmatter with `name` + `description`, markdown body). Contains `spec/`, `skills/` (examples), `template/`. OpenCode has native built-in skill discovery from `.opencode/skills/`, `~/.config/opencode/skills/`, and Claude-compatible paths.

### Superpowers

[obra/superpowers](https://github.com/obra/superpowers) — agentic skills framework & software development methodology. Installed per-harness (Claude Code, Codex, OpenCode, etc.). OpenCode is configured through the repo-root `opencode.json`:

```json
{
  "plugin": ["orkworks/apm_modules/obra/superpowers/.opencode/plugins/superpowers.js"]
}
```

### Ponytail

[DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail) — minimalist ruleset that enforces YAGNI: check necessity, stdlib, platform feature, dependency, one-liner before writing code. OpenCode is configured through the repo-root `opencode.json`:

```json
{
  "plugin": ["orkworks/apm_modules/DietrichGebert/ponytail/.opencode/plugins/ponytail.mjs"]
}
```

Ponytail also ships its own `AGENTS.md` — if cross-referenced from this repo's clone, OpenCode loads both automatically.

### Claude Mem

[thedotmack/claude-mem](https://github.com/thedotmack/claude-mem) — persistent memory for Claude using simple YAML files.

## Maintaining AGENTS.md and README.md

Keep both files current as the project evolves. Update AGENTS.md and README.md whenever any of the following occur: a new runtime dependency is added or removed, a directory in the planned architecture changes, a new agent target is added to `apm.yml`, a convention or workflow listed in this file changes, or a new ADR is created. Treat stale docs as a bug — if you notice something out of date while working, fix it.
