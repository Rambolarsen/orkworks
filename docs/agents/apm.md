---
type: Agent Guide
title: APM and agent plugins
description: APM-managed agent dependencies, generated configuration, and repository skill operations.
tags: [apm, agents, plugins, skills, configuration]
status: stable
---

# APM and Agent Plugins

Dependencies are managed by [APM](https://github.com/anthropics/apm) at the repo root. The root `apm.yml` defines targets (claude, codex, copilot, opencode) and dependencies. Running `apm install` from the repo root populates generated agent assets:

| Path | Contents |
| ---- | -------- |
| `apm_modules/` | Cloned dependency sources (gitignored) |
| `apm.lock.yaml` | Resolved lock file (gitignored) |
| `.agents/skills/` | Skills for all targets (APM external skills plus synced repo-owned skills) |
| `.claude/` | Claude Code hooks + skills (APM external skills plus synced repo-owned skills) |
| `.codex/` | Generated/local Codex hook configuration and adapters (gitignored) |
| `.github/hooks/` | Copilot hooks |
| `.github/skills/` | Copilot workspace skills synced from `skills/` (gitignored) |
| `.opencode/` | OpenCode target |
| `.mcp.json` | Claude Code MCP server config |
| `.vscode/mcp.json` | VS Code MCP server config |
| `.codex/config.toml` | Codex MCP server config (gitignored, local-only) |

The committed `scripts/doc-check.sh` is the harness-neutral doc-diff detector. The committed `scripts/codex-doc-check-stop.sh` adapter wraps that output into valid Stop-hook JSON so Codex can surface the message without rejecting the hook output; the Codex hook configuration and its local adapter entry remain gitignored because OrkWorks installs machine-specific hook entries.

The APM lifecycle in `apm.yml` removes the incompatible Superpowers `SessionStart` registration from generated Codex hooks and runs `node scripts/sync-repo-skills.mjs` to materialize committed repo-owned skills from `skills/` after installs and updates. Lifecycle commands are trust-gated; on a new checkout, run `apm lifecycle trust` once before `apm install` or `apm update` if you want those post-install steps to run automatically. Both helpers are also available directly as `bash scripts/repair-codex-session-start-hooks.sh` and `node scripts/sync-repo-skills.mjs` (or `node scripts/sync-repo-skills.mjs --check` to verify sync state without writing).

## Product skill management

The [workspace and personal skills proposal](../../specs/workspace-skills.md)
defines the approved user-facing inventory and qualified APM additions.
See [qualification evidence](../validation/skills-management.md) for the initial
local-only profile and discovery gaps. The rest of this guide describes this
repository's development setup, not a universal installer configuration.

The desktop Skills page includes a collapsed APM configuration section below the
inventory: resolved executable,
CLI version, compatibility, Check again, and an optional absolute executable-path
override stored in desktop settings. Clearing the path restores desktop PATH
discovery; an unavailable override never falls back silently. Saving checks the
selected executable as the current user. Changing selection invalidates pending
installation reviews, including during native confirmation. Local discovery is
available without APM. Reviewed installation remains limited to local standalone
skills targeting Codex when APM exposes the required install options; OrkWorks does not install or upgrade APM.
Status and preview share capability discovery; version is informational. Missing
install options or probe failures disable reviewed adds with a specific reason.
The daily/manual [APM compatibility workflow](../../.github/workflows/apm-compatibility.yml)
and relevant PRs exercise the latest stable source CLI with disposable fixtures.
See [ADR 0105](../adr/0105-apm-capability-compatibility.md) for the accepted
tradeoff: help advertises capabilities; it cannot prove future CLI semantics.

## Dependency freshness report

The [dependency freshness workflow](../../.github/workflows/dependency-freshness.yml)
implements the [reviewed #973 contract](https://github.com/Rambolarsen/orkworks/issues/973#issuecomment-6948481712).
It runs Mondays at 07:41 UTC or manually against `main`, independently of
Taskmaster and the daily APM compatibility qualification. It reports APM ref
changes, desktop advisory findings, and explicit dismissal follow-up conditions.
It does not update dependencies, execute coding agents, or mutate alerts.

The workflow pins APM 0.35.0 to source commit
`17a2fc4dfff238c7d93e55cce6ef42ffa1db928e`. Before collection, a fresh-checkout
qualification trusts harmless lifecycle markers, then proves `APM_NO_SCRIPTS=1`
prevents their execution during dry-run and leaves files unchanged. Raw APM
0.35.0 dry-run can execute trusted lifecycle scripts; the environment guard is
required, not an optional optimization. PRs changing the sweep or APM
manifest run the fixtures and this qualification without publishing a live audit.
The collector runs `apm update --dry-run --verbose` with a disposable manifest
and isolated APM home, with `APM_NO_SCRIPTS=1` enforced. The report distinguishes proposed ref changes from
retained pins; a missing eligible upstream tag is not evidence of freshness.

Desktop collection runs `pnpm with 12.18.0 audit --json` against the committed
lockfile. The [explicit version invocation](https://pnpm.io/cli/with) bypasses
package-manager bootstrapping that can otherwise rewrite the lockfile; no dependency install or native build is required. Resolved pnpm
overrides annotate findings but never suppress them. An audit finding is a
triage item, not a claim that production code is exploitable. Source failures,
malformed output and unexpected file changes fail collection and are reported
separately from advisories. The Actions summary and raw evidence are retained
as artifacts for 30 days. The workflow has read-only repository permissions,
requires no new secrets, and creates no issues or PRs.

The versioned [watch list](../../.github/dependency-review.json) starts empty;
withdrawn alert #79 is historical context, not an active dismissal. Add entries
through a reviewed PR with these fields:

```json
{
  "advisory": "GHSA-aaaa-bbbb-cccc",
  "package": "example",
  "manifest": "apps/desktop/pnpm-lock.yaml",
  "rationale": "Evidence supporting the reviewed disposition",
  "evidence": "https://example.com/review",
  "reviewed_at": "2026-11-17",
  "condition": { "kind": "patched" }
}
```

Use `{"kind":"date","date":"2026-12-01"}` for a dated human review instead.
Patch conditions query public GitHub advisory metadata and distinguish an
available patch, no reported patch, withdrawal and inability to verify.
Watch entries record review reminders and explicit dispositions, not current
GitHub alert status. Matching advisories remain visible with the disposition;
conditions that become due require human review. This initial sweep covers
APM and desktop dependencies; docs and Rust audits remain outside its scope.

Run deterministic fixtures with `python3 scripts/dependency-freshness.test.py`.
Set `ORKWORKS_FRESHNESS_APM` to the absolute pinned APM executable to include
live fresh-checkout qualification. On a clean disposable checkout, run
`python3 scripts/dependency-freshness.py --output /tmp/dependency-freshness`.
The collector exits nonzero for incomplete collection or checkout changes;
advisory findings alone do not fail it.

## Superpowers

[obra/superpowers](https://github.com/obra/superpowers) — agentic skills framework & software development methodology. Installed per-harness. OpenCode loads its skills through the repo-root `opencode.json` `skills.paths` entry `.agents/skills` — the plugin file itself is **not wired into OpenCode 2**:

OpenCode 2 intentionally broke the V1 plugin API ("V1 plugin implementations do not run in V2") and requires `plugins` config entries to be plugin directories, while the vendored `superpowers.js` is a single V1-style file. Loading it as a config entry logs `configured plugin path must be a directory` and never runs its hooks. Until upstream ships an OpenCode 2 plugin (or the repo maintains a V2 adapter), OpenCode gets the Superpowers skills but not the plugin's bootstrap message injection; Claude Code and other targets are unaffected.

## Ponytail

[DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail) — minimalist ruleset that enforces YAGNI: check necessity, stdlib, platform feature, dependency, one-liner before writing code.

Ponytail is intentionally deployed for the Copilot and OpenCode targets in
`apm.yml`; other APM targets do not receive this plugin.

Same OpenCode 2 blocker as Superpowers: the vendored `ponytail.mjs` is a single V1-style plugin file, and OpenCode 2 only loads plugin directories through config with the V2 plugin API. It is **not wired into OpenCode 2** until upstream ships a V2 plugin (or the repo maintains a V2 adapter); the Copilot target keeps working.

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

The `skills/` directory is the canonical committed source for repo-level agent skills. These follow the Agent Skills standard: each skill is a directory with a `SKILL.md` file using YAML frontmatter and a markdown body.

Because `.claude/skills/`, `.agents/skills/`, and `.github/skills/` are gitignored, run `node scripts/sync-repo-skills.mjs` (automatically triggered by `apm.yml` `post-install`/`post-update` when lifecycle commands are trusted, and fast/offline in freshly created sibling worktrees) to copy every repo-owned skill from `skills/` into those three discovery directories as real files (not symlinks, preserving Windows portability and Codex symlink-denial boundaries). Managed entries are tracked per target in `.orkworks-repo-skills.json` so unrelated APM skills stay untouched, removed repo skills are pruned, legacy symlinks pointing directly to `skills/<name>` are migrated to real copies, and unmanaged name collisions fail closed before modifying any target. Use `node scripts/sync-repo-skills.mjs --check` for read-only drift verification.

Skill creation guidance (`writing-skills`) is supplied by the APM-pinned `obra/superpowers` dependency rather than duplicated under `skills/`.

| Skill | Description |
| ----- | ----------- |
| `starting-work` | Branch/worktree setup and per-checkout workflow for new code changes |
| `cutting-release` | Version bump, tag push, CI monitoring, and release verification workflow |
| `adding-harness` | Checklist for adding or changing a harness adapter |
| `working-on-recommendation` | Resolves a Taskmaster recommendation, scopes the work, and reports verified completion |
| `reviewing-plans` | Delivery complexity assessment and scope, simplicity, clarity, and verification checks before plan handoff |
| `troubleshooting-github-connectivity` | Separates stale RTK output, network failures, and GitHub authentication errors |
| `clean-ddd-hexagonal` | Clean Architecture + DDD + Hexagonal patterns, language-agnostic |
| `surfacing-blind-spots` | Planning checkpoint, end-of-session self-critique, and quality-improvement audit for investigated quality risks |
| `babysitting-pull-requests` | Repo-owned post-PR workflow for complete comment, CI, and review-state inventories |
| `auditing-test-honesty` | Finds passing tests that would survive the bug they're named after |
| `walking-failure-paths` | Traces external failures (files, processes, ports) through the code to the user-visible outcome |
| `grooming-the-board` | Board/code/spec consistency sweep: duplicates, done-but-open issues, stranded branches, doc drift |
| `auditing-signal-vs-noise` | UI truthfulness audit of the situational-awareness surfaces against their metadata sources |
| `consulting-the-brain` | Routes agent-readiness analysis/verification/improvement work through the owner's external "brain" knowledge repo (`Rambolarsen/brain`) instead of re-deriving it |
| `simplifying-repo` | Validates current repository evidence and investigates behavior-preserving reductions using local SonarQube |
| `orchestrating-task-graphs` | Multi-agent task-graph orchestration: fake-edge test before fan-out, separate diverse verifiers, one owned merge |

The five audit skills (`surfacing-blind-spots` plus the four above) share the guardrail filter and issue format defined in `skills/surfacing-blind-spots/` and rotate weekly via `.github/workflows/quality-audit.yml`.

### Repository workflow overrides

Root `AGENTS.md` governs preparation size and overrides heavier defaults in
installed skills. Load skills for concrete task/phase triggers and reuse
already-loaded context. Routine work proceeds after a brief approach within
the user's authorization. Written plans follow the
[local plan policy](development-workflow.md#plan-complexity-and-quality),
including concise artifacts, one review before execution/handoff, and no
mandatory full implementation code. Use the repo-owned
[reviewing-plans](https://github.com/Rambolarsen/orkworks/blob/main/skills/reviewing-plans/SKILL.md) for that review.
Approved scope does not need repeated approval; specific product gates remain
binding.

Keep these overrides in committed repository instructions and owned skills.
Do not edit external/generated `.agents/skills/` copies; `apm install` replaces
them. Instruction delivery follows the
[harness entry points](harness-instruction-coverage.md); this is policy, not
proof that a harness complied at runtime.

### Anthropic Agent Skills (standard)

[anthropics/skills](https://github.com/anthropics/skills) — reference implementation of the Agent Skills standard. Defines the `SKILL.md` format (YAML frontmatter with `name` + `description`, markdown body). OpenCode has native built-in skill discovery from `.opencode/skills/`, `~/.config/opencode/skills/`, and Claude-compatible paths.

## Update triggers

Update this file when:

- `apm.yml` changes (new targets, new plugins, removed plugins)
- `opencode.json` changes (new plugin paths or configuration)
- A new APM plugin is added or removed
- Generated path layout changes after `apm install`
- A new agent target is added (codex, copilot, gemini, etc.)

## OpenCode requirement

OpenCode must load the project-level `opencode.json` at the repo root. Start OpenCode with the repo root as the project, for example:

```bash
opencode /Users/froomiebot/workspace/orkworks
```

Do not use `--pure` for development work in this repo; it disables external plugins. The root `opencode.json` exposes both `.agents/skills` and committed repo skills from `skills/`. The APM-managed Superpowers and Ponytail plugin files are not wired into OpenCode 2 — see the blockers under each section above.

Before OpenCode implementation work, verify that the skill tool lists Superpowers skills such as `superpowers/using-superpowers` and `superpowers/brainstorming`. If they are missing, stop, run `apm install` from the repo root, restart OpenCode from the repo root, and verify again before editing code.

## Task-specific skill routing

Use `adding-harness` before harness adapter changes; preserve its launch,
resume, identity, voice, capacity, generated-command fixture, and readiness
checks. Use `working-on-recommendation` when work starts from a Taskmaster
recommendation. Use `orchestrating-task-graphs` before planning or dispatching
multi-agent work, and `babysitting-pull-requests` after opening/adopting a PR.
Use `surfacing-blind-spots` for high-risk planning, relevant close-out risks,
or a requested quality audit; routine work needs no separate planning ritual.
Use `consulting-the-brain` when the owner requests analysis through the brain
knowledge repo. Use `simplifying-repo` for requested repository validation and
behavior-preserving complexity reductions; see [local analysis](local-sonarqube.md).
Before GitHub work, use `troubleshooting-github-connectivity` to establish the
authorized target access path.
