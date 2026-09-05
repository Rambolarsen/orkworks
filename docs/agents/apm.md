# APM and Agent Plugins

Dependencies are managed by [APM](https://github.com/anthropics/apm) at the repo root. The root `apm.yml` defines targets (claude, codex, copilot, opencode) and dependencies. Running `apm install` from the repo root populates generated agent assets:

| Path | Contents |
| ---- | -------- |
| `apm_modules/` | Cloned dependency sources (gitignored) |
| `apm.lock.yaml` | Resolved lock file (gitignored) |
| `.agents/skills/` | Skills for all targets |
| `.claude/` | Claude Code hooks + skills |
| `.codex/` | Generated/local Codex hook configuration and adapters (gitignored) |
| `.github/hooks/` | Copilot hooks |
| `.opencode/` | OpenCode target |
| `.mcp.json` | Claude Code MCP server config |
| `.vscode/mcp.json` | VS Code MCP server config |
| `.codex/config.toml` | Codex MCP server config (gitignored, local-only) |

The committed `scripts/doc-check.sh` is the harness-neutral doc-diff detector. The committed `scripts/codex-doc-check-stop.sh` adapter wraps that output into valid Stop-hook JSON so Codex can surface the message without rejecting the hook output; the Codex hook configuration and its local adapter entry remain gitignored because OrkWorks installs machine-specific hook entries.

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

## mattpocock/skills (pinned subset)

[mattpocock/skills](https://github.com/mattpocock/skills) — a multi-skill bundle repo; only three skills are pinned via `apm.yml`'s `skills:` list rather than the whole ~17-skill bundle:

- `improve-codebase-architecture` — scans for deepening opportunities, presents them as a visual HTML report, then hands off to `grilling` for the one picked
- `codebase-design` — shared deep-module vocabulary (module, interface, seam, adapter, leverage, locality) the above skill designs against
- `grilling` — one-question-at-a-time decision-tree interview, used to walk a chosen deepening candidate to a shared understanding before implementation

## addyosmani/agent-skills (pinned subset)

[addyosmani/agent-skills](https://github.com/addyosmani/agent-skills) — a multi-skill bundle repo; only two skills are pinned via `apm.yml`'s `skills:` list rather than the whole bundle:

- `context-engineering` — optimizes agent context setup: rules files, project context, and session/task-switch hygiene
- `doubt-driven-development` — subjects non-trivial decisions to a fresh-context adversarial review before they stand

## MCP servers

MCP servers are declared in `apm.yml` under `dependencies.mcp` and materialized per-client by `apm install` — not by hand-editing `.mcp.json`, `.vscode/mcp.json`, `.codex/config.toml` (gitignored, local-only), or `opencode.json`'s `mcp` key directly. None are currently declared.

## Repo-level skills

The `skills/` directory contains repo-level agent skills committed with the project. These follow the Agent Skills standard: each skill is a directory with a `SKILL.md` file using YAML frontmatter and a markdown body.

| Skill | Description |
| ----- | ----------- |
| `starting-work` | Branch/worktree setup and per-checkout workflow for new code changes |
| `cutting-release` | Version bump, tag push, CI monitoring, and release verification workflow |
| `adding-harness` | Checklist for adding or changing a harness adapter |
| `writing-skills` | TDD-based skill creation following the Agent Skills standard |
| `clean-ddd-hexagonal` | Clean Architecture + DDD + Hexagonal patterns, language-agnostic |
| `surfacing-blind-spots` | End-of-session self-critique and codebase audit that files scoped quality issues |
| `auditing-test-honesty` | Finds passing tests that would survive the bug they're named after |
| `walking-failure-paths` | Traces external failures (files, processes, ports) through the code to the user-visible outcome |
| `grooming-the-board` | Board/code/spec consistency sweep: duplicates, done-but-open issues, stranded branches, doc drift |
| `auditing-signal-vs-noise` | UI truthfulness audit of the situational-awareness surfaces against their metadata sources |
| `consulting-the-brain` | Routes agent-readiness analysis/verification/improvement work through the owner's external "brain" knowledge repo (`Rambolarsen/brain`) instead of re-deriving it |
| `orchestrating-task-graphs` | Multi-agent task-graph orchestration: fake-edge test before fan-out, separate diverse verifiers, one owned merge |

The five audit skills (`surfacing-blind-spots` plus the four above) share the guardrail filter and issue format defined in `skills/surfacing-blind-spots/` and rotate weekly via `.github/workflows/quality-audit.yml`.

### Anthropic Agent Skills (standard)

[anthropics/skills](https://github.com/anthropics/skills) — reference implementation of the Agent Skills standard. Defines the `SKILL.md` format (YAML frontmatter with `name` + `description`, markdown body). OpenCode has native built-in skill discovery from `.opencode/skills/`, `~/.config/opencode/skills/`, and Claude-compatible paths.

## Update triggers

Update this file when:

- `apm.yml` changes (new targets, new plugins, removed plugins)
- `opencode.json` changes (new plugin paths or configuration)
- A new APM plugin is added or removed
- Generated path layout changes after `apm install`
- A new agent target is added (codex, copilot, gemini, etc.)
