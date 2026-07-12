# OrkWorks

Local-first mission control for AI coding sessions. Peons observe individual sessions; Taskmaster recommends what should happen next across harnesses, models, reviews, capacity, and Git context. OrkWorks observes and recommends before it controls — it does not replace Claude Code, Codex, OpenCode, Gemini CLI, or Aider.

**Documentation:** https://rambolarsen.github.io/orkworks/

## State

APM project bootstrapped — agent skills, hooks, and plugins are installed via [APM](https://github.com/anthropics/apm) at the repo root. M1 (Electron app shell + Rust sidecar scaffold) is implemented, and the alpha release pipeline now packages desktop artifacts through GitHub Actions + electron-builder. Subsequent milestones are tracked as GitHub issues.

## Architecture

```text
orkworks/
├─ apps/desktop/          # Electron + React/TypeScript + Dockview + xterm.js desktop UI
├─ crates/orkworksd/      # Rust sidecar (Axum HTTP/WS, PTY via portable-pty)
├─ docs/
│  ├─ adr/                # Architecture Decision Records
│  └─ agents/             # Agent-facing docs (architecture, domain entities, APM)
├─ skills/                # Repo-level agent skills
└─ specs/                 # Authoritative product specs
```

- Electron launches Rust sidecar; UI talks to it over localhost HTTP/WebSocket
- `nodeIntegration: false`, `contextIsolation: true`
- Desktop UI uses Dockview draggable panels for sessions, detail, terminal, and recommendations; Capacity is a non-Providers stub surface
- New agent sessions can be launched with a selected coding tool, optional model override, and optional initial prompt; the dialog remembers the last coding tool/model choice, and harness definitions are loaded from the sidecar's built-ins plus `~/.orkworks/harnesses.json`
- The app remembers the last workspace and repo-local active session for relaunch restore
- The Electron main process owns app-level settings in `userData`, including canonical default hotkeys and persisted hotkeys that drive native menu accelerators
- Session details show read-only `Coding tool`, `Model provider`, `Model`, and `Provider state` for the selected session. The backend fallback system (Peon skips disabled/capped model providers) remains in place behind the scenes.
- Runtime lifecycle is explicit as `creating → alive → stopping → dead`; live attention exists only while a session is alive, and dead sessions retain a frozen final observer snapshot for history and recovery (see [ADR 0023](docs/adr/0023-simplified-session-lifecycle.md))
- PTY lifetime is owned by the Rust sidecar session runtime rather than by a renderer WebSocket; active work survives terminal detach while `orkworksd` stays alive (see [ADR 0022](docs/adr/0022-session-runtime-owned-pty-lifetime.md))
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
- `~/.orkworks/hook-scripts/` — stable copies of harness reporter scripts, so installed hooks survive app updates and packaging path changes
- Priority: user > agent > peon > backend_inference > process > unknown (see [ADR 0005](docs/adr/0005-metadata-source-priority.md))
- Session records include explicit work phase, lifecycle, alive-only attention, pending terminal outcome, and final observed-state snapshots; legacy records are normalized on read
- Peon reads terminal output, writes inferred metadata, never types into terminals
- Harnesses can write deterministic attention signals at `agent` priority via `POST /sessions/:id/attention`; installation is explicit and user-confirmed only ([ADR 0019](docs/adr/0019-attention-signal-endpoint-opt-in-hook-install.md))
- Taskmaster proposes cross-session transitions; every v1 transition requires explicit user approval

## Setup

```bash
# from the repo root
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

Normal pull requests use `.github/workflows/pr-ci.yml`. That workflow routes by changed surface:

- `apps/desktop/**` runs desktop type-check, tests, and build
- `crates/orkworksd/**` runs Rust tests
- PRs that touch neither surface get a lightweight passing no-op check for status clarity
- Agent `/code-review` defaults to lightweight effort; escalate to medium effort or higher only for bigger/riskier changes such as architecture/runtime, concurrency/lifecycle, protocol/schema/migration, security-sensitive work, or unusually large diffs

A third workflow, `.github/workflows/quality-audit.yml`, runs weekly on a schedule: it rotates through the audit skills in `skills/` (blind spots, test honesty, failure paths, board grooming, UI signal integrity) and files scoped quality issues. It authenticates with a Claude Pro/Max subscription via the `CLAUDE_CODE_OAUTH_TOKEN` repo secret (generate with `claude setup-token`; API-key alternative documented in the workflow header) and can be run manually from the Actions tab with a specific skill.

## Containerized dev environment (optional)

A Podman/OCI toolchain container lets you build, type-check, and test OrkWorks without installing Node, Rust, or the Electron toolchain on the host. It's an **alternative** to the native pnpm flow above, not a replacement — GUI runs still use the native flow (see [issue #80](https://github.com/Rambolarsen/orkworks/issues/80) Tier 2). Toolchain versions are pinned in `rust-toolchain.toml`, `.nvmrc`, and `packageManager` so the container and host agree.

Requires only Podman (or Docker) — no host Node/Rust install. Substitute `docker compose` for `podman compose` if you use Docker.

```bash
# Build the toolchain image
podman compose build

# Install deps, type-check, and run the frontend test suite
podman compose run --rm dev bash -lc "cd apps/desktop && pnpm install"
podman compose run --rm dev bash -lc "cd apps/desktop && npx tsc --noEmit"
podman compose run --rm dev bash -lc "cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs"

# Build, lint, and test the Rust sidecar
podman compose run --rm dev cargo build   --manifest-path crates/orkworksd/Cargo.toml
podman compose run --rm dev cargo clippy  --manifest-path crates/orkworksd/Cargo.toml
podman compose run --rm dev cargo test    --manifest-path crates/orkworksd/Cargo.toml
```

`apps/desktop/node_modules` and `crates/orkworksd/target` live in **named volumes**, never bind-mounted from the host — Electron and native deps are platform-specific, so host and container caches must stay separate. Removing the volumes (`podman compose down -v`) forces a clean reinstall/rebuild.

**Windows:** Podman runs inside a `podman machine` (WSL2) VM, so bind-mounting the source tree from an NTFS path incurs a filesystem-perf penalty; keeping the repo on the Linux/WSL2 side is faster. Set `git config core.autocrlf input` (or use a `.gitattributes` `* text=auto`) so CRLF line endings from Windows checkouts don't break shell scripts inside the Linux container.

## Peon configuration

Peon runs in the Rust sidecar as a background task. After a session's terminal goes quiet, Peon asks a model provider to classify the recent output and writes the result to `~/.orkworks/workspaces/<hash>/sessions/<id>.json`. User input into the terminal also resets this debounce window — typing counts as activity. While an inference is in flight for a session, a second one is not launched for the same session. Sessions quiet past `PEON_IDLE_TIMEOUT` are marked idle by timer, without an LLM call.

Which tool performs the inference is no longer chosen by environment variable: Peon routes through the model-provider fallback system (`providers.rs`), which skips disabled/capped providers in fallback order. The per-provider Peon model is configured in the app's Settings.

The observation loop itself is tuned via environment variables on `orkworksd`:

| Variable | Default | Purpose |
| -------- | ------- | ------- |
| `PEON_ENABLED` | `true` | Set to `false`/`0` to disable Peon entirely |
| `PEON_INTERVAL` | `5` | Seconds between Peon scan cycles |
| `PEON_IDLE_TIMEOUT` | `15` | Seconds of terminal silence before a session is marked idle by timer |
| `PEON_MAX_LINES` | `200` | Ring-buffer size of terminal lines fed to inference |

(`PEON_HARNESS`, `PEON_HARNESS_ARGS_JSON`, `PEON_MODEL`, and `PEON_TIMEOUT` are legacy — still parsed, but session inference no longer uses them.)

## Agent plugins

Managed via APM in `apm.yml` at the repo root. Running `apm install` from the repo root populates skills and hooks for all configured targets (claude, codex, copilot, opencode).

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
| [leonardomso/rust-skills](https://github.com/leonardomso/rust-skills) | Rust coding rules for work under `crates/` |

## Repo skills

The `skills/` directory contains repo-level agent skills that are committed with the project. These follow the [Agent Skills standard](https://agentskills.io/specification) — each skill is a directory with a `SKILL.md` file (YAML frontmatter + markdown body).

| Skill | Description |
| ----- | ----------- |
| [starting-work](skills/starting-work/SKILL.md) | Branch/worktree setup and per-checkout workflow for new code changes |
| [cutting-release](skills/cutting-release/SKILL.md) | Version bump, tag push, CI monitoring, and release verification workflow |
| [adding-harness](skills/adding-harness/SKILL.md) | Checklist for adding or changing a harness adapter (launch, resume, session ID capture, voice, capacity) |
| [writing-skills](skills/writing-skills/SKILL.md) | TDD-based skill creation following the Agent Skills standard |
| [clean-ddd-hexagonal](skills/clean-ddd-hexagonal/SKILL.md) | Clean Architecture + DDD + Hexagonal patterns, language-agnostic |

## Issue board

[https://github.com/Rambolarsen/orkworks/issues](https://github.com/Rambolarsen/orkworks/issues)

- Prefer issues that restore or stabilize current functionality before starting new milestone feature work.
- Treat user-visible bugs, regressions, failing tests, and correctness or data-integrity bugs as stabilization work.
- When no meaningful stabilization work is open, pick from the lowest incomplete milestone and work forward in milestone order.
- If both a bugfix and a feature slice are plausible, break ties in favor of current usability and data correctness.

## Key naming

| Term | Meaning |
| ---- | ------- |
| OrkWorks | Product |
| `orkworksd` | Rust backend sidecar |
| Peon | Low-cost session/repo metadata observer |
| Taskmaster | Workspace-level next-step coordinator |
| `.orkworks/` | Global metadata directory under `~/.orkworks/` |

User-facing UI says `Coding tool` for CLI coding applications. Internal code and metadata continue to use `harness` for that integration abstraction. `Model provider` is reserved for inference services and local inference runtimes.

Session metadata and session API payloads now accept canonical `harnessId`, `modelProviderId`, and `modelId` fields while remaining compatible with legacy `harness`, `providerId`, and `model` records during the migration window.

## Specs

- `specs/orkworks-mvp.md` — full product scope, architecture, milestones, non-goals
- `specs/native-harness-voice-support.md` — voice support design
- `specs/release-pipeline.md` — alpha desktop packaging and GitHub Releases workflow
- `specs/review-queue.md` — repo-local review inbox for plan/spec artifacts
- `specs/taskmaster.md` — cross-session coordination and next-step recommendations
