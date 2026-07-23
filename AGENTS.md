# OrkWorks — Agent Guide

## Identity

Local-first mission control for AI coding sessions. Peons observe individual sessions; Taskmaster recommends what should happen next across harnesses, models, reviews, capacity, and Git context. OrkWorks observes and recommends before it controls — it does not replace Claude Code, Codex, OpenCode, Gemini CLI, or Aider.

## State of the repo

APM project bootstrapped — agent skills, hooks, and plugins are installed via [APM](https://github.com/anthropics/apm) at the repo root. M1 (Electron app shell + Rust sidecar scaffold) is implemented. Subsequent milestones are tracked as GitHub issues.

## Package manager

Use **pnpm** for all Node.js package management. Do not use npm or yarn for project package management tasks.

GitHub Actions now has four distinct workflow classes:

- `.github/workflows/release.yml` for tag-driven release packaging only
- `.github/workflows/pr-ci.yml` for pull-request validation on `main`
- `.github/workflows/docs.yml` builds the VitePress docs site and deploys it to GitHub Pages on doc-path pushes to `main`
- `.github/workflows/quality-audit.yml` for the weekly scheduled quality audit — rotates through the audit skills in `skills/` (one per week, so each fires roughly monthly) and files scoped issues per those skills' guardrails; requires the `CLAUDE_CODE_OAUTH_TOKEN` repo secret (subscription auth via `claude setup-token`; an `ANTHROPIC_API_KEY` swap is documented in the workflow header)

PR CI is path-routed: desktop changes run desktop validation, Rust changes run Rust tests, and non-code PRs receive a lightweight passing no-op check.

```bash
# Install pnpm if missing
corepack enable
corepack prepare pnpm@latest --activate

# Install deps
cd apps/desktop && pnpm install

# Run dev (Vite + Electron, auto-launches Rust sidecar)
cd apps/desktop && pnpm dev

# Build Electron app
cd apps/desktop && pnpm build

# Package a host-arch release artifact locally
cd apps/desktop && pnpm package:release

# Build Rust sidecar
cd apps/desktop && pnpm build:rust
# or directly:
cargo build --manifest-path crates/orkworksd/Cargo.toml

# TypeScript type-check
cd apps/desktop && npx tsc --noEmit

# Run frontend tests (Node built-in test runner)
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs

# Run a single test file
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts

# Run Rust tests
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

## Containerized dev environment (optional)

A Podman/OCI toolchain container (`Containerfile` + `compose.yaml` at the repo root) can build, type-check, lint, and test both `apps/desktop` and `crates/orkworksd` without a host Node/Rust/Electron install. It is an **alternative** to the native pnpm flow above, never a replacement — the native host workflow and the release pipeline (`.github/workflows/release.yml`) are unchanged. Toolchain versions are pinned in `rust-toolchain.toml`, `.nvmrc`, and the `packageManager` field so the container and host agree. GUI runs stay on the native flow (issue #80 Tier 2).

Substitute `docker compose` for `podman compose` if you use Docker.

```bash
# Build the toolchain image
podman compose build

# Non-GUI tasks (each runs in a throwaway container)
podman compose run --rm dev bash -lc "cd apps/desktop && pnpm install"
podman compose run --rm dev bash -lc "cd apps/desktop && npx tsc --noEmit"
podman compose run --rm dev bash -lc "cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs"
podman compose run --rm dev cargo build  --manifest-path crates/orkworksd/Cargo.toml
podman compose run --rm dev cargo clippy --manifest-path crates/orkworksd/Cargo.toml
podman compose run --rm dev cargo test   --manifest-path crates/orkworksd/Cargo.toml
```

`apps/desktop/node_modules`, `crates/orkworksd/target`, and the Cargo registry live in **named volumes** — never bind-mounted from the host, since Electron and native deps are platform-specific and the caches must not corrupt each other. On Windows, Podman runs in a WSL2 VM (bind-mount perf penalty on NTFS paths; set `git config core.autocrlf input` so CRLF endings don't break in-container shell scripts).

## Issue board

All implementation work is tracked as GitHub issues: [https://github.com/Rambolarsen/orkworks/issues](https://github.com/Rambolarsen/orkworks/issues)

- **Prioritize stabilization work first.** Prefer issues that restore or stabilize existing functionality before starting new milestone feature work. This includes user-visible bugs, regressions, failing tests, and correctness or data-integrity bugs.
- **Use milestone order for net-new work.** When no meaningful stabilization work is open, pick from the lowest incomplete milestone and work forward in milestone order.
- **Break ties by user impact.** If both a bugfix and a feature slice are plausible next steps, favor current usability and data correctness.
- **Add future work** as new issues. Break down into scoped, deliverable-sized issues with checkbox acceptance criteria.
- **Keep issues in sync** with the codebase — close when done, update when scope changes.
- If the issue board is inaccessible, do not guess at priorities. Stop and inform the user that issue board access is required before picking or closing work.
- Specs remain authoritative for product scope; issues track implementation progress.
- If an issue describes work not covered by the specs, do not implement it. Add a comment on the issue noting the gap and ask for a spec update.
- If the specs describe work with no corresponding issue, create one before implementing.

## Authoritative specs

- `specs/orkworks-mvp.md` — full product scope, architecture, milestones, non-goals
- `specs/native-harness-voice-support.md` — voice support design
- `specs/release-pipeline.md` — alpha desktop packaging and GitHub Releases workflow
- `specs/review-queue.md` — proposed repo-local review inbox for plan/spec artifacts
- `specs/taskmaster.md` — proposed cross-session coordination and next-step recommendation layer

Read these before starting any implementation work.

If any authoritative spec file is missing or unreadable, stop and notify the user before proceeding. Do not infer scope from context alone.

## Docs site

Repo markdown is rendered as a docs site at https://rambolarsen.github.io/orkworks/ (VitePress config in `docs/.vitepress/`, deployed by `.github/workflows/docs.yml`). The markdown files in the repo are the single source of truth — the site is a rendering layer only. User-facing documentation lives in `docs/user/`; agents read it like any other repo markdown. The build fails on dead links, so keep links valid when moving or renaming docs.

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

### electron/ and src/ are hard boundaries

`apps/desktop/electron/` (Electron main process) and `apps/desktop/src/` (renderer) must never import from each other. They are compiled by separate TypeScript configs with separate `rootDir` settings — a cross-boundary import either produces stray compiled artifacts or forces a `rootDir` change. Either symptom means the design is wrong, not the config.

IPC contract types shared across the boundary must be defined independently in both directories. Duplication is intentional: each side owns its copy. If you need to change a shared type, update both.

Do not change `rootDir` in `tsconfig.node.json` or `tsconfig.json` to accommodate a new import. A required `rootDir` change is a signal to reconsider the import, not to adjust the config.

### OpenCode requirement

OpenCode must load the project-level `opencode.json` at the repo root. Start OpenCode with the repo root as the project, for example:

```bash
opencode /Users/froomiebot/workspace/orkworks
```

Do not use `--pure` for development work in this repo; it disables external plugins. The root `opencode.json` loads the APM-managed Superpowers and Ponytail plugins and exposes both `.agents/skills` and committed repo skills from `skills/`.

Before OpenCode implementation work, verify that the skill tool lists Superpowers skills such as `superpowers/using-superpowers` and `superpowers/brainstorming`. If they are missing, stop, run `apm install` from the repo root, restart OpenCode from the repo root, and verify again before editing code.

## Branch and PR workflow

`main` is the trunk, not the workspace. Use branches and PRs for code; keep main fast for low-risk writing.

**`main` checkout ownership:** The local `main` branch may be checked out only in the primary checkout. Linked worktrees must be attached to an explicitly agent-owned or owner-authorized feature or fix branch; they must never check out `main` or remain detached. The primary checkout may temporarily use an agent-owned or owner-authorized branch under the rules below.

When starting any task that will produce code changes, invoke the `starting-work` skill (in `skills/starting-work/`) before editing. It walks through the branch-vs-worktree decision, naming convention, and per-checkout setup that operationalize the rules in this section.

**Don't stack commits on branches you don't own.** This rule exists to prevent two writers on one branch: an agent silently adding commits to a branch another agent or person is actively working on causes lost work, confusing history, and clobbered checkouts. It is not a ban on landing legitimate changes — if the branch owner explicitly asks you to push to their branch (e.g. applying review fixes to their PR), do so. Absent that permission: if the primary checkout is on a branch someone else created, do not add commits to it — open a worktree on your own branch instead. If you find yourself on a foreign branch in a worktree, stop and create a new one.

**Direct to `main` is allowed for:**
- Docs-only changes: `docs/`, `specs/`, ADRs, `README.md`, `AGENTS.md`, `CLAUDE.md`, and other `*.md` outside `apps/`/`crates/`.
- Trivial code fixes under ~20 lines (typos, comment edits, single-line config tweaks). When in doubt, branch.

**Everything else requires a branch + PR**, including any change to `apps/desktop/src/`, `apps/desktop/electron/`, `apps/desktop/tests/`, or `crates/orkworksd/`, regardless of commit-type prefix.

**One PR per logical unit of work.** A burst of 5–10 small commits in a few minutes that share a feature name is one PR, not ten commits on main. Squash or rebase locally before opening it.

**Review gate:** PRs that touch code under `apps/desktop/` or `crates/orkworksd/` must have a `/code-review` run before merge. Default to a lightweight review. Escalate to medium effort or higher only for bigger or riskier changes: cross-cutting architecture/runtime work, concurrency or lifecycle changes, protocol/schema/migration changes, security-sensitive work, or unusually large diffs (roughly more than 8 code files or 500 lines). Address findings or note why each is intentional in the PR description.

**Squash-merge by default.** Preserve multiple commits only when the history tells a story worth keeping (e.g. a refactor followed by a focused fix on top).

**Stranded branches:** branches that go >7 days without merging must either be rebased and progressed, or closed with a one-line reason in the PR. No long-lived dev branches. The same rule applies to stranded worktrees.

**Recovering `main`:** If the primary checkout is detached, do not check out `origin/main` or use a linked worktree as a substitute. Inspect `git worktree list --porcelain`. If another worktree holds `main`, ask its owner to restore its owner branch or remove it. Only when that specific worktree is clean and you are explicitly authorized may you perform that recovery yourself. Never detach the worktree or use force operations. If an active owner or uncommitted changes would be affected, stop and obtain direction.

**Parallel work:** when more than one branch is in flight at once (multiple agents running concurrently, a hotfix on top of an in-progress feature), use `git worktree` so each branch has its own filesystem checkout — branch-switching in the main checkout will collide with other agents' uncommitted edits and build output. Also use a worktree whenever the active branch in the primary checkout is one you did not create, even if no other agent is running. Invoke the `starting-work` skill before opening a worktree for the path convention, per-worktree setup, and cleanup steps.

**Clean up your worktrees when done.** Remove the worktree and prune it as soon as the branch merges (or the task is abandoned). Leaving stale worktrees behind wastes disk space and confuses subsequent `git worktree list` output. The `starting-work` skill includes the exact cleanup commands.

Because parallel agents each see only their own worktree, none of them individually notices the fleet-wide sprawl this creates — see the [worktree currency check](#worktree-currency-check) below, which runs at the end of every session and reports on all worktrees, not just the current one.

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
| Peon | Low-cost session/repo metadata observer |
| Taskmaster | Workspace-level next-step coordinator |
| `.orkworks/` | Global metadata directory under `~/.orkworks/` (workspaces/<hash>/, harnesses.json, hook-scripts/) |

User-facing UI says `Coding tool` for CLI coding applications. Internal code and metadata continue to use `harness` for that integration abstraction. `Model provider` is reserved for inference services and local inference runtimes.

Use normal engineering terminology for all other concepts. Peon and Taskmaster are the two intentional product-specific worker names; do not expand the fantasy naming further without an explicit spec update.

## Architecture

Electron + React/TypeScript frontend (`apps/desktop/`) communicates with a Rust sidecar (`crates/orkworksd/`) over a dynamic localhost HTTP/WebSocket port. The desktop UI uses Dockview draggable panels around xterm.js terminal sessions. The sidecar manages PTY sessions, Git context, the metadata protocol (under `~/.orkworks/workspaces/<hash>/`), Peon observation, and Taskmaster recommendation state.

- ADR 0017: Provider context is session-scoped (read-only in Details), not app-wide.
- ADR 0022: PTY lifetime is session-runtime-owned in the sidecar; renderer terminal attachment is detachable and does not own process lifetime.
- ADR 0024: Raw terminal replay is bounded to the newest 1,000 lines and 1 MiB; durable summary checkpoints live in the event log.
- ADR 0025: Session-plan handoff uses a sidecar-scoped secret and Electron main-process path revalidation; the renderer never receives filesystem paths.
- ADR 0026: Harness capabilities resolve from one immutable registry; integration mutations require Electron-main confirmation and sidecar-only authority.

**Rust module layout** (`crates/orkworksd/src/`):
- `metadata.rs` — `SessionMetadata` and the on-disk metadata store (source of truth for session state)
- `session_types.rs`, `session_view.rs` — session-facing types and view/projection helpers
- `harness.rs`, `harness_registry.rs` — harness adapter abstraction and built-in harness definitions
- `providers.rs` — model provider registry, fallback, and capacity state
- `peon.rs` — terminal-output observation and label/status inference
- `git.rs`, `watcher.rs`, `migration.rs`, `workspace_runtime.rs` — Git context detection, metadata file watching, on-disk migrations, workspace bootstrap
- `http/` — thin HTTP handler submodules (session, harness, provider, retention, attention hook) delegating to `AppState`
- `runtime/` — background tasks: terminal/PTY runtime (`SessionRuntime`, PTY lifecycle), Peon observation loop, retention cleanup
- `main.rs` — Axum router, `AppState`/`SessionHandle` struct definitions, startup

See [`docs/agents/architecture.md`](docs/agents/architecture.md) for the full inter-component breakdown (port discovery, preload bridge, API data flow, Rust modules, panel layout).
See [`docs/agents/domain-entities.md`](docs/agents/domain-entities.md) for the current session state model: `SessionMetadata` fields, session status vocabulary, and terminology boundaries.

## Metadata protocol

- `~/.orkworks/workspaces/<hash>/sessions/<id>.json` — session state
- `~/.orkworks/workspaces/<hash>/events/<id>.ndjson` — append-only event log with durable, exact consecutive-deduplicated summary checkpoints and accepted provenance
- `~/.orkworks/workspaces/<hash>/events/<id>.terminal` — recent raw terminal replay, bounded on append to the newest 1,000 lines and 1 MiB; existing oversized dormant files remain unchanged until their next append
- `~/.orkworks/workspaces/<hash>/capacity/<id>.json` — capacity per model/harness
- `~/.orkworks/workspaces/<hash>/recommendations/<id>.json` — Taskmaster recommendation state and history
- `~/.orkworks/workspaces/<hash>/workspace.json` — workspace memory, including the last active session
- `~/.orkworks/harnesses.json` — global harness definitions
- `~/.orkworks/hook-scripts/` — stable copies of harness reporter scripts (e.g. the Claude Code Notification hook), installed hook commands always point here rather than at the packaged/dev source, so they keep working across app updates and packaging schemes whose own paths aren't stable at runtime (Linux AppImage's per-launch mount point, in particular)
- Priority: user > agent > peon > backend_inference > process > unknown > debug
- Peon reads terminal output, writes inferred metadata, never types into terminals
- Detached runtimes continue draining terminal output, persisting history, and feeding Peon while `orkworksd` stays alive; losing the renderer terminal attachment alone must not end the session
- Backend-only `GET /sessions/:id/summary-log` exposes checkpoints in append order as timestamp, summary, source, and nullable confidence; missing data returns `{ "entries": [] }`. There is no renderer/preload consumer.
- Taskmaster consumes normalized metadata and proposes cross-session transitions; v1 requires explicit user approval for every action

## Key conventions from specs

- Do **not** expand fantasy naming beyond Peon and Taskmaster — use normal engineering terms everywhere else
- MVP does not own Git workflow, worktree management, merging, or arbitrary task decomposition
- Taskmaster may recommend session transitions but must not start sessions without explicit user approval in v1
- If asked to implement something listed as a non-goal in the specs, decline and explain which non-goal applies. Do not implement it even partially.
- Harness voice is pass-through only — OrkWorks never captures/proxies/stores audio for native voice
- Store metadata source and confidence where possible
- Capacity states: healthy, degraded, capped, unknown, disabled
- Cost tiers: local, low, medium, high, premium

## Product design principles

These are load-bearing UX decisions. Treat them as constraints on any feature, design, or plan that touches the desktop UI.

- **Session = context. Switching sessions is the context-switch primitive.** The sessions list is the multi-view across N sessions; the active terminal is single by design. Do not propose, plan, or build multi-terminal, tiled, split, stacked, or picture-in-picture terminal views. Showing many terminals at once is context degradation, not visibility — it divides attention and consumes screen real estate without adding situational awareness. Situational awareness belongs in the sessions list (legibility, attention state, last activity, agent action summary) and the detail panel — not in parallel terminal rendering. The same logic extends to any other context-bearing surface added later (editors, agent transcripts): one active, switch deliberately. See [ADR 0013](docs/adr/0013-single-active-context-primitive.md) for context and consequences.
- Fast context-switching (keyboard nav, MRU ordering, jump-to-session search) is the right axis to improve when situational awareness or task throughput is the goal. Parallel visibility is the wrong axis.

## APM and agent plugins

Agent dependencies (Superpowers, Ponytail, Claude Mem, rust-skills) are managed by [APM](https://github.com/anthropics/apm) at the repo root (`apm.yml`). Run `apm install` from the repo root to populate skills and hooks for all configured targets (claude, codex, copilot, opencode).

See [`docs/agents/apm.md`](docs/agents/apm.md) for the full plugin list, generated path layout, and OpenCode configuration.

## Repo-level skills

The `skills/` directory contains committed repo skills (`starting-work`, `cutting-release`, `writing-skills`, `clean-ddd-hexagonal`, `adding-harness`, `surfacing-blind-spots`, `auditing-test-honesty`, `walking-failure-paths`, `grooming-the-board`, `auditing-signal-vs-noise`). Each is a directory with a `SKILL.md` following the [Agent Skills standard](https://agentskills.io/specification). Use `skills/adding-harness/` before adding or changing a harness adapter; it forces the launch/resume/session-ID/voice/capacity checklist for the harness. Use `skills/surfacing-blind-spots/` when closing out a session or when asked to generate quality-improvement tasks; it turns investigated uncertainties and project blind spots into scoped issues.

Five of these are **audit skills** that generate quality-improvement work: `surfacing-blind-spots` (uncertainties and blind spots), `auditing-test-honesty` (tests that don't pin what they claim), `walking-failure-paths` (behavior under external failure), `grooming-the-board` (board/code/spec drift), and `auditing-signal-vs-noise` (UI truthfulness). They share the guardrail filter and issue format defined in `skills/surfacing-blind-spots/`. The weekly `quality-audit.yml` workflow rotates through them; they can also be run ad hoc.

## Doc currency check

Before ending any session, run:

```bash
bash .claude/hooks/doc-check.sh
```

This checks git diff against known triggers and lists any doc files that likely need updating. Address all flagged files before closing. Claude Code runs this automatically via a Stop hook; all other agents must run it manually as part of `verification-before-completion`.

## Worktree currency check

Before ending any session, also run:

```bash
bash .claude/hooks/worktree-check.sh
```

This lists every worktree/branch in the repo (not just the one this session used) and flags branches that are already merged (worktree can be removed), or stale >7 days with no open PR (needs a rebase-and-progress or close decision per the stranded-branches rule above). It exists because with multiple agents working in parallel worktrees, no single session sees the whole fleet — this surfaces it every time any session ends. Only act on branches you own; for others, note the flag for the human or the branch's owner rather than touching their worktree. Claude Code runs this automatically via a Stop hook; all other agents must run it manually as part of `verification-before-completion`.

## Maintaining AGENTS.md and README.md

Keep both files current as the project evolves. Update AGENTS.md and README.md whenever any of the following occur: a new runtime dependency is added or removed, a directory in the planned architecture changes, a new agent target is added to `apm.yml`, a convention or workflow listed in this file changes, or a new ADR is created. Treat stale docs as a bug — if you notice something out of date while working, fix it.

Also keep `docs/agents/domain-entities.md` current whenever `SessionMetadata` fields, status/lifecycle vocabulary, or terminology boundaries change in `crates/orkworksd/src/metadata.rs` or in closely related session/API mapping code.
