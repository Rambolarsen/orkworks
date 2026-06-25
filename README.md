# OrkWorks

Local-first mission control for AI coding sessions. Peons observe individual sessions; Taskmaster recommends what should happen next across harnesses, models, reviews, capacity, and Git context. OrkWorks observes and recommends before it controls — it does not replace Claude Code, Codex, OpenCode, Gemini CLI, or Aider.

## State

APM project bootstrapped — agent skills, hooks, and plugins are installed via [APM](https://github.com/anthropics/apm) in `orkworks/`. M1 (Electron app shell + Rust sidecar scaffold) is implemented, and the alpha release pipeline now packages desktop artifacts through GitHub Actions + electron-builder. Subsequent milestones are tracked as GitHub issues.

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
- Desktop UI uses Dockview draggable panels for sessions, detail, terminal, and recommendations; Capacity is a non-Providers stub surface
- New agent sessions can be launched with a selected coding tool, optional model override, and optional initial prompt; harness definitions are loaded from the sidecar's built-ins plus `~/.orkworks/harnesses.json`
- The app remembers the last workspace and repo-local active session for relaunch restore
- The Electron main process owns app-level settings in `userData`, including canonical default hotkeys and persisted hotkeys that drive native menu accelerators
- Session details show read-only `Coding tool`, `Model provider`, and `Provider state` for the selected session, sourced from session metadata. The backend fallback system (Peon skips disabled/capped model providers) remains in place behind the scenes.
- Peon writes observer metadata such as `observedStatus` without replacing runtime lifecycle `status`
- Taskmaster consumes Peon reports and workspace context to propose the next session or user action
- PTY handles only text I/O; voice (native harness) bypasses PTY entirely

## Metadata protocol

All metadata lives under `~/.orkworks/` (see [ADR 0018](docs/adr/0018-global-metadata-store.md)). Per-workspace data is keyed by a hash of the workspace path:

- `~/.orkworks/workspaces/<hash>/sessions/<id>.json` — session state
- `~/.orkworks/workspaces/<hash>/events/<id>.ndjson` — append-only event log
- `~/.orkworks/workspaces/<hash>/capacity/<id>.json` — capacity per model/harness
- `~/.orkworks/workspaces/<hash>/recommendations/<id>.json` — Taskmaster recommendation state and history
- `~/.orkworks/workspaces/<hash>/workspace.json` — workspace memory, including the last active session
- `~/.orkworks/harnesses.json` — global harness definitions
- Priority: user > agent > peon > backend_inference > process > unknown
- Peon reads terminal output, writes inferred metadata, never types into terminals
- Taskmaster proposes cross-session transitions; every v1 transition requires explicit user approval

## Setup

```bash
cd orkworks
apm install
```

## Build and release

```bash
# frontend + Electron build
cd apps/desktop && pnpm build

# Rust sidecar
cd apps/desktop && pnpm build:rust

# package a host-arch desktop artifact locally
cd apps/desktop && pnpm package:release
```

GitHub Releases are tag-driven. Pushing `vX.Y.Z` runs `.github/workflows/release.yml`, which builds:

- macOS x64 on `macos-13`
- macOS arm64 on `macos-latest`
- Windows x64 on `windows-latest`
- Linux x64 on `ubuntu-latest`

## Peon configuration

Peon runs in the Rust sidecar as a background task. After a session's terminal goes quiet for `PEON_INTERVAL` seconds (default `5`), Peon shells out to a configurable harness, asks it to classify the recent output, and writes the result to `.orkworks/sessions/<id>.json`. User input into the terminal also resets this debounce window — typing counts as activity. While an inference is in flight for a session, a second one is not launched for the same session.

Tune via environment variables on `orkworksd`:

| Variable | Default | Purpose |
| -------- | ------- | ------- |
| `PEON_ENABLED` | `true` | Set to `false`/`0` to disable Peon entirely |
| `PEON_INTERVAL` | `5` | Seconds of terminal silence before inference fires |
| `PEON_HARNESS` | `opencode` | Binary Peon shells out to for classification |
| `PEON_HARNESS_ARGS_JSON` | `["run","--pure"]` | JSON array of args passed to the harness (falls back to space-split `PEON_HARNESS_ARGS`) |
| `PEON_MODEL` | unset | Reserved for harness model selection |
| `PEON_MAX_LINES` | `200` | Ring-buffer size of terminal lines fed to the harness |
| `PEON_TIMEOUT` | `30` | Seconds before a harness invocation is killed |

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
| [starting-work](skills/starting-work/SKILL.md) | Branch/worktree setup and per-checkout workflow for new code changes |
| [cutting-release](skills/cutting-release/SKILL.md) | Version bump, tag push, CI monitoring, and release verification workflow |
| [writing-skills](skills/writing-skills/SKILL.md) | TDD-based skill creation following the Agent Skills standard |
| [clean-ddd-hexagonal](skills/clean-ddd-hexagonal/SKILL.md) | Clean Architecture + DDD + Hexagonal patterns, language-agnostic |

## Issue board

[https://github.com/Rambolarsen/orkworks/issues](https://github.com/Rambolarsen/orkworks/issues)

## Key naming

| Term | Meaning |
| ---- | ------- |
| OrkWorks | Product |
| `orkworksd` | Rust backend sidecar |
| Peon | Low-cost session/repo metadata observer |
| Taskmaster | Workspace-level next-step coordinator |
| `.orkworks/` | Global metadata directory under `~/.orkworks/` |

User-facing UI says `Coding tool` for CLI coding applications. Internal code and metadata continue to use `harness` for that integration abstraction. `Model provider` is reserved for inference services and local inference runtimes.

## Specs

- `specs/orkworks-mvp.md` — full product scope, architecture, milestones, non-goals
- `specs/native-harness-voice-support.md` — voice support design
- `specs/release-pipeline.md` — alpha desktop packaging and GitHub Releases workflow
- `specs/review-queue.md` — repo-local review inbox for plan/spec artifacts
- `specs/taskmaster.md` — cross-session coordination and next-step recommendations
