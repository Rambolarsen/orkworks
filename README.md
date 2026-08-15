# OrkWorks

Local-first mission control for AI coding sessions. Peons observe individual sessions; Taskmaster recommends what should happen next across harnesses, models, reviews, capacity, and Git context. OrkWorks observes and recommends before it controls — it does not replace Claude Code, Codex, OpenCode, Antigravity CLI, or Aider.

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
- New agent sessions can be launched with a selected coding tool, optional model override, and optional initial prompt; harness definitions resolve from embedded built-ins plus sparse versioned overrides in `~/.orkworks/harnesses.json`
- Antigravity CLI is the supported Google coding tool (`agy`); retired Gemini CLI records and settings remain readable for compatibility but cannot start new sessions
- Session labels are stable topics, re-seeded only after a harness-declared fresh-conversation command; delayed old-topic inference cannot overwrite the reset placeholder (ADR 0040)
- The app remembers the last workspace and repo-local active session for relaunch restore
- The Electron main process owns app-level settings in `userData`, including canonical default hotkeys and persisted hotkeys that drive native menu accelerators
- Session details show read-only `Coding tool`, `Model provider`, `Model`, and `Provider state` for the selected session. The backend fallback system (Peon skips disabled/capped model providers) remains in place behind the scenes.
- ADR 0023 defines the target runtime lifecycle as `creating → alive → stopping → dead`, with live attention only while a session is alive. The current implementation retains the earlier lifecycle vocabulary until that migration lands (see [ADR 0023](docs/adr/0023-simplified-session-lifecycle.md))
- Lifecycle transitions remain metadata-driven; the previously unwired domain aggregate was removed, with a future typed state-machine tracked in [issue #181](https://github.com/Rambolarsen/orkworks/issues/181) (see [ADR 0021](docs/adr/0021-session-lifecycle-phases.md)).
- PTY lifetime is owned by the Rust sidecar session runtime rather than by a renderer WebSocket; active work survives terminal detach while `orkworksd` stays alive (see [ADR 0022](docs/adr/0022-session-runtime-owned-pty-lifetime.md))
- Raw terminal replay is bounded to the newest 1,000 lines and 1 MiB; dead sessions display that saved output read-only, while accepted session summaries are retained as durable checkpoints (see [ADR 0024](docs/adr/0024-bounded-terminal-replay-durable-summary-checkpoints.md))
- Session plans/specs appear in a reusable Review tab; the renderer receives availability and document content, never a filesystem path. The sole terminal-input exception is a user-clicked, fixed review prompt (see [ADR 0025](docs/adr/0025-authenticated-session-plan-handoff.md), [ADR 0034](docs/adr/0034-user-approved-session-review-prompt.md))
- Harness capabilities and workspace integration status resolve from one immutable registry; mutations require Electron-main confirmation and never expose mutation authority to the renderer or child processes (see [ADR 0026](docs/adr/0026-resolved-harness-capability-registry.md))
- Harness version-probe results use bounded TTL caching with generation-aware invalidation; integration actions still revalidate identity after a probe (see [ADR 0028](docs/adr/0028-generation-aware-harness-version-probe-cache.md))
- Codex sessions capture their native session ID via a `SessionStart` hook (`.codex/hooks.json`), enabling exact resume; because that event isn't a "needs input" signal like every other integrated harness's hooked event, it does not post the generic attention update (see [ADR 0035](docs/adr/0035-codex-session-start-hook-not-attention-signal.md))
- Codex's hook installer writes a `$HOME`-relative, machine-independent reporter command (POSIX only) rather than an absolute path, so it can safely install into a git-tracked `.codex/hooks.json` — the shape every APM-managed repo actually has, unlike Claude/Gemini/Copilot's local-only config files (see [ADR 0036](docs/adr/0036-codex-hooks-portable-reporter-path.md))
- Plan/spec path reporting uses a dedicated path-only sidecar route (`POST /sessions/:id/plan-path`) that canonicalizes the file and stores its workspace-relative form without changing session attention, superseding terminal-text inference when a harness reports a canonical file path; Codex remains on the conservative terminal fallback because its hook payload provides patch text (see [ADR 0037](docs/adr/0037-hook-reported-plan-paths.md))
- Claude Code's owned integration installs a synchronous `PostToolUse` `Write|Edit` hook whose shared reporter passes `--report-plan-path`, forwarding the hook payload's `tool_input.file_path` to `/sessions/:id/plan-path` and skipping the generic attention + harness-session POSTs. `ToolHookContract::reports_plan_path` declares the opt-in so the framework stays open to non-Claude harnesses (see [ADR 0038](docs/adr/0038-claude-plan-path-post-tool-use-hook.md))
- Session `label` (title) is a one-shot Peon-authored topic, decoupled from the turn-by-turn summary/checkpoint log (see [ADR 0029](docs/adr/0029-session-label-topic-vs-activity-summary.md))
- Session git-context fields (`repo_root`/`branch`/`dirty`/etc.) reflect each session's live cwd, prioritizing the harness's own self-reported cwd (Claude Code, via its hook payload) over a cross-platform pid probe (`sysinfo`), over the frozen launch-time cwd — a session that `cd`s or `git worktree add`s mid-session no longer shows a stale location (see [ADR 0031](docs/adr/0031-live-session-cwd-via-sysinfo-probe.md), [ADR 0032](docs/adr/0032-harness-reported-cwd-via-hook-payload.md))
- Taskmaster consumes Peon reports and workspace context to propose the next session or user action
- PTY handles only text I/O; voice (native harness) bypasses PTY entirely

## Metadata protocol

All metadata lives under `~/.orkworks/` (see [ADR 0018](docs/adr/0018-global-metadata-store.md)). Per-workspace data is keyed by a hash of the workspace path:

- `~/.orkworks/workspaces/<hash>/sessions/<id>.json` — session state
- `~/.orkworks/workspaces/<hash>/events/<id>.ndjson` — append-only event log with durable, exact consecutive-deduplicated summary checkpoints and accepted provenance
- `~/.orkworks/workspaces/<hash>/events/<id>.terminal` — recent raw terminal replay, bounded on append to the newest 1,000 lines and 1 MiB; existing oversized dormant files remain unchanged until their next append
- `~/.orkworks/workspaces/<hash>/events/<id>.terminal-size` — the PTY's `cols`x`rows` at the moment a session reaches a terminal status (`killed`/`ended`/`error`), written once; used to render dead-session terminal replay at its recorded size instead of the current panel width. Absent for sessions that ended before this file existed, and for sessions whose in-memory runtime handle was already gone at the terminal-status transition — both cases fall back to fit-to-container replay. See [ADR 0033](docs/adr/0033-recorded-terminal-replay-size-sidecar.md).
- `~/.orkworks/workspaces/<hash>/capacity/<id>.json` — capacity per model/harness
- `~/.orkworks/workspaces/<hash>/recommendations/<id>.json` — Taskmaster recommendation state and history
- `~/.orkworks/workspaces/<hash>/workspace.json` — workspace memory, including the last active session
- `~/.orkworks/workspaces/<hash>/integrations/aider.json` — versioned OrkWorks-owned Aider notification-command preference
- `~/.orkworks/harnesses.json` — global harness definitions
- `~/.orkworks/hook-scripts/` — stable copies of harness reporter scripts, so installed hooks survive app updates and packaging path changes
- Priority: user > agent > peon > backend_inference > process > unknown > debug (see [ADR 0005](docs/adr/0005-metadata-source-priority.md))
- Current session records expose the canonical `creating → alive → stopping → dead` lifecycle. Only alive sessions have attention: `working`, `idle`, `needs_you`, `blocked`, `failed`, or `capped`.
- Peon reads terminal output and writes inferred metadata; the only terminal-write exception is the explicit user-approved session-plan review prompt ([ADR 0034](docs/adr/0034-user-approved-session-review-prompt.md))
- Harnesses can write deterministic attention signals at `agent` priority via `POST /sessions/:id/attention`; generic workspace integration installation is explicit and user-confirmed only ([ADR 0026](docs/adr/0026-resolved-harness-capability-registry.md))
- `GET /sessions/:id/summary-log` returns checkpoints in append order as `{entries: [{timestamp, summary, source, confidence}]}`; `confidence` is nullable and missing data returns `{entries: []}`. Rendered in the session detail panel as "Task history" — distinct from the session's `label` (title), which is a stable, one-shot Peon-authored topic rather than this turn-by-turn activity log (see [ADR 0029](docs/adr/0029-session-label-topic-vs-activity-summary.md))
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

The Rust sidecar has one Windows-only dependency feature (`windows-sys` / `Win32_Storage_FileSystem`) so durable configuration writes use `ReplaceFileW` for an expected existing file and non-replacing `MoveFileExW` for an expected new file. This narrows external-edit races but is not portable compare-and-swap; Unix builds do not include the dependency.

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

Development agents should follow `AGENTS.md`, including the requirement to invoke and follow relevant Superpowers skills before implementation, debugging, review, verification, commit, push, or PR work. The root file routes subsystem work: read [`apps/desktop/AGENTS.md`](apps/desktop/AGENTS.md) before desktop changes, [`crates/orkworksd/AGENTS.md`](crates/orkworksd/AGENTS.md) before sidecar changes, and both for cross-component work.

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

## MCP servers

This repo manages project-scoped MCP server configuration through `apm.yml`, not by hand-editing per-client config files.

Current MCP server:

- `oraios/serena`

To materialize the client-specific config, run:

```bash
# from the repo root
apm install
```

`serena` runs through `uvx`, so `uv` must be installed locally.

## Repo skills

The `skills/` directory contains repo-level agent skills that are committed with the project. These follow the [Agent Skills standard](https://agentskills.io/specification) — each skill is a directory with a `SKILL.md` file (YAML frontmatter + markdown body).

| Skill | Description |
| ----- | ----------- |
| [starting-work](skills/starting-work/SKILL.md) | Branch/worktree setup and per-checkout workflow for new code changes |
| [cutting-release](skills/cutting-release/SKILL.md) | Version bump, tag push, CI monitoring, and release verification workflow |
| [adding-harness](skills/adding-harness/SKILL.md) | Checklist for adding or changing a harness adapter (launch, resume, session ID capture, voice, capacity) |
| [writing-skills](skills/writing-skills/SKILL.md) | TDD-based skill creation following the Agent Skills standard |
| [clean-ddd-hexagonal](skills/clean-ddd-hexagonal/SKILL.md) | Clean Architecture + DDD + Hexagonal patterns, language-agnostic |
| [surfacing-blind-spots](skills/surfacing-blind-spots/SKILL.md) | Turns investigated uncertainties and project blind spots into scoped issues |
| [auditing-test-honesty](skills/auditing-test-honesty/SKILL.md) | Audits whether tests actually pin the behavior their names claim |
| [walking-failure-paths](skills/walking-failure-paths/SKILL.md) | Traces external failures (files, processes, ports) through code to the user-visible outcome |
| [grooming-the-board](skills/grooming-the-board/SKILL.md) | Sweeps for board/code/spec drift — duplicates, done-but-open issues, stranded branches, doc drift |
| [auditing-signal-vs-noise](skills/auditing-signal-vs-noise/SKILL.md) | Audits UI truthfulness of situational-awareness surfaces against their metadata sources |
| [consulting-the-brain](skills/consulting-the-brain/SKILL.md) | Routes agent-readiness analysis/improvement work through the owner's external "brain" knowledge repo |

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
- `specs/review-queue.md` — superseded repo-local review inbox proposal
- `specs/session-plan-review.md` — selected-session plan/spec review and explicit review prompt handoff
- `specs/taskmaster.md` — cross-session coordination and next-step recommendations
