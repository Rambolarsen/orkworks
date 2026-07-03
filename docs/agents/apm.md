# APM and Agent Plugins

Dependencies are managed by [APM](https://github.com/anthropics/apm) at the repo root. The root `apm.yml` defines targets (claude, codex, copilot, opencode) and dependencies. Running `apm install` from the repo root populates generated agent assets:

| Path | Contents |
| ---- | -------- |
| `apm_modules/` | Cloned dependency sources (gitignored) |
| `apm.lock.yaml` | Resolved lock file (gitignored) |
| `.agents/skills/` | Skills for all targets |
| `.claude/` | Claude Code hooks + skills |
| `.codex/` | Codex hooks |
| `.github/hooks/` | Copilot hooks |
| `.opencode/` | OpenCode target |

## Superpowers

[obra/superpowers](https://github.com/obra/superpowers) — agentic skills framework & software development methodology. Installed per-harness. OpenCode is configured through the repo-root `opencode.json`:

```json
{
  "plugin": ["apm_modules/obra/superpowers/.opencode/plugins/superpowers.js"]
}
```

## Ponytail

[DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail) — minimalist ruleset that enforces YAGNI: check necessity, stdlib, platform feature, dependency, one-liner before writing code. OpenCode is configured through the repo-root `opencode.json`:

```json
{
  "plugin": ["apm_modules/DietrichGebert/ponytail/.opencode/plugins/ponytail.mjs"]
}
```

Ponytail also ships its own `AGENTS.md` — if cross-referenced from this repo's clone, OpenCode loads both automatically.

## Claude Mem

[thedotmack/claude-mem](https://github.com/thedotmack/claude-mem) — persistent memory for Claude using simple YAML files.

## rust-skills

[leonardomso/rust-skills](https://github.com/leonardomso/rust-skills) — 265 Rust coding rules across 26 categories (ownership, error handling, async/tokio, unsafe, API design, memory, concurrency, serde, observability, performance, anti-patterns, and more). Current for Rust 1.96 / 2024 edition. Invoke with `/rust-skills` when writing, reviewing, or refactoring any code under `crates/`.

## Repo-level skills

The `skills/` directory contains repo-level agent skills committed with the project. These follow the Agent Skills standard: each skill is a directory with a `SKILL.md` file using YAML frontmatter and a markdown body.

| Skill | Description |
| ----- | ----------- |
| `starting-work` | Branch/worktree setup and per-checkout workflow for new code changes |
| `cutting-release` | Version bump, tag push, CI monitoring, and release verification workflow |
| `adding-harness` | Checklist for adding or changing a harness adapter |
| `writing-skills` | TDD-based skill creation following the Agent Skills standard |
| `clean-ddd-hexagonal` | Clean Architecture + DDD + Hexagonal patterns, language-agnostic |

### Anthropic Agent Skills (standard)

[anthropics/skills](https://github.com/anthropics/skills) — reference implementation of the Agent Skills standard. Defines the `SKILL.md` format (YAML frontmatter with `name` + `description`, markdown body). OpenCode has native built-in skill discovery from `.opencode/skills/`, `~/.config/opencode/skills/`, and Claude-compatible paths.

## Update triggers

Update this file when:

- `apm.yml` changes (new targets, new plugins, removed plugins)
- `opencode.json` changes (new plugin paths or configuration)
- A new APM plugin is added or removed
- Generated path layout changes after `apm install`
- A new agent target is added (codex, copilot, gemini, etc.)
