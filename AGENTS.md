# OrkWorks — Agent Guide

## Identity

Local-first mission control for AI coding sessions. Peons observe individual sessions; Taskmaster recommends what should happen next across harnesses, models, reviews, capacity, and Git context. OrkWorks observes and recommends before it controls — it does not replace Claude Code, Codex, OpenCode, Gemini CLI, or Aider.

## State of the repo

APM project bootstrapped — agent skills, hooks, and plugins are installed via [APM](https://github.com/anthropics/apm) at the repo root. M1 (Electron app shell + Rust sidecar scaffold) is implemented. Subsequent milestones are tracked as GitHub issues.

## Package manager

Use **pnpm** for all Node.js package-management tasks in this repository, including `apps/desktop/` and `docs/`. Do not use npm or yarn.

## CI routing

CI routing, required checks, and informational automated reviews are repository
obligations; follow the branch and PR workflow below. For workflow classes,
path routing, schedules, and review thresholds, read
[`docs/agents/project-context.md`](docs/agents/project-context.md#ci-routing).


GitHub's native Copilot code review is also enabled as a repository ruleset (Settings → Rules → Rulesets), scoped to PRs targeting `main` only — it is not a workflow file, so it has no corresponding entry under `.github/workflows/`. Like `pr-review.yml`, it is a second automated reviewer, not a required check, and does not replace the manual `/code-review` review gate below.


## Containerized dev environment (optional)

The optional Podman/OCI toolchain is an alternative to the native pnpm workflow,
never a replacement; GUI runs remain native. Use the documented named-volume
setup rather than host bind mounts for platform-specific dependencies. For
commands, provider setup, platform notes, and sidecar behavior, read
[`docs/agents/project-context.md`](docs/agents/project-context.md#containerized-development-environment-optional).



## Issue board

All implementation work is tracked as GitHub issues: [https://github.com/Rambolarsen/orkworks/issues](https://github.com/Rambolarsen/orkworks/issues)

- **Prioritize GitHub Copilot harness work first**, then stabilization work, then net-new work in lowest-incomplete-milestone order. Break ties by user impact.
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
- `specs/review-queue.md` — superseded repo-local review inbox proposal
- `specs/session-plan-review.md` — selected-session plan/spec review and explicit review prompt handoff
- `specs/taskmaster.md` — proposed cross-session coordination and next-step recommendation layer
- `specs/taskmaster-knowledge.md` — signed reference knowledge, independent Taskmaster analysis, and background context controls

Read these before starting any implementation work.

If any authoritative spec file is missing or unreadable, stop and notify the user before proceeding. Do not infer scope from context alone.

## Documentation

Durable project context is organized as an OKF v0.2-style knowledge bundle in
[`docs/agents/index.md`](docs/agents/index.md). Read the index and the relevant
concept before changing documentation, architecture, integrations, or other
work whose context is not fully captured in this root guide. The root guide
remains authoritative for repository-wide rules; the bundle provides focused
reference detail and progressive disclosure.

## Assumption discipline

Before acting, make every assumption that could change the scope, target, permissions, or expected behavior explicit. Treat missing context as unknown, not as permission to guess.

- Validate material assumptions against the authoritative source for the task: the user's request, the live OrkWorks recommendation/API when working from a recommendation, applicable specs, and scoped repository instructions.
- Distinguish facts, inferences, and open questions in the working update or plan. If authoritative evidence is missing or conflicts, stop and ask rather than silently choosing an interpretation.
- Do not treat a source session, stale metadata, or an inferred status as permission to resume, reopen, or modify that session; follow the task's explicit scope instead.

## Docs site

The public homepage prioritizes prospective users and onboarding. Keep its
claims tied to implemented behavior, and distinguish source documentation
from published installer capabilities. See [site maintenance](docs/agents/site-maintenance.md)
for generated coding-tool/release facts, daily publishing, PR builds, and the
weekly documentation-audit PR workflow.

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

When delegating work to subagents, prefer the harness's current lower-cost model tier at high reasoning effort over older or premium-tier models. Escalate to a more capable or expensive model only when the task's risk or complexity clearly warrants it.

These workflow requirements constrain how agents work in this repository. They do not expand OrkWorks product scope or override the MVP non-goals.

## Instruction scoping

`AGENTS.md` is the repository entry point and routing contract. Before changing a subsystem, read its canonical instructions:

| Scope | Required instructions |
| ---- | ---- |
| Desktop app (`apps/desktop/`) | [`apps/desktop/AGENTS.md`](apps/desktop/AGENTS.md) |
| Rust sidecar (`crates/orkworksd/`) | [`crates/orkworksd/AGENTS.md`](crates/orkworksd/AGENTS.md) |
| Cross-component work | Both scoped files |

Codex and Copilot load the nested `AGENTS.md` files. Claude loads path-scoped rules that import the same canonical files. OpenCode must follow this root router and read the applicable file on demand; do not use `opencode.json`'s session-wide `instructions` list for this split. See [Harness instruction coverage](docs/agents/harness-instruction-coverage.md) for the configured-target delivery record.

### electron/ and src/ are hard boundaries

The Electron-main and renderer boundary is a global architectural invariant. Its implementation rules live in [`apps/desktop/AGENTS.md`](apps/desktop/AGENTS.md); read them before changing desktop code.

### OpenCode requirement

OpenCode must load the project-level `opencode.json` at the repo root. Start OpenCode with the repo root as the project, for example:

```bash
opencode /Users/froomiebot/workspace/orkworks
```

Do not use `--pure` for development work in this repo; it disables external plugins. The root `opencode.json` loads the APM-managed Superpowers and Ponytail plugins and exposes both `.agents/skills` and committed repo skills from `skills/`.

Before OpenCode implementation work, verify that the skill tool lists Superpowers skills such as `superpowers/using-superpowers` and `superpowers/brainstorming`. If they are missing, stop, run `apm install` from the repo root, restart OpenCode from the repo root, and verify again before editing code.

## Branch and PR workflow

`main` is the trunk, not the workspace. All changes — code and docs alike — land through branches and PRs.

**Resolving a PR reference:** Never guess or assume the current branch's PR when the user names one without full detail. Resolve it to a concrete PR before acting; see [Resolving a PR reference](docs/agents/development-workflow.md#resolving-a-pr-reference).

**`main` checkout ownership:** The local `main` branch may be checked out only in the primary checkout. Linked worktrees must be attached to an explicitly agent-owned or owner-authorized feature or fix branch; they must never check out `main` or remain detached. The primary checkout may temporarily use an agent-owned or owner-authorized branch under the rules below.

When starting any task that will produce changes (code or docs), invoke the `starting-work` skill (in `skills/starting-work/`) before editing. It walks through the branch-vs-worktree decision, naming convention, and per-checkout setup that operationalize the rules in this section.

Each coding session owns one task through its PR reaching a terminal state (merged or closed), not just through opening it — a session may keep checking on its own PR (CI, comments, reviews) within a bounded check-in budget instead of handing off immediately. Do not launch another coding harness; hand off work beyond that budget, or any follow-up unrelated to the PR being watched, by stopping and having the user start a separate session from the appropriate repository root or sibling worktree. The detailed session-scope, self-babysit budget, and supported-handoff rule lives in `skills/starting-work/`.

**Don't stack commits on branches you don't own.** This rule exists to prevent two writers on one branch: an agent silently adding commits to a branch another agent or person is actively working on causes lost work, confusing history, and clobbered checkouts. It is not a ban on landing legitimate changes — if the branch owner explicitly asks you to push to their branch (e.g. applying review fixes to their PR), do so. Absent that permission: if the primary checkout is on a branch someone else created, do not add commits to it — open a worktree on your own branch instead. If you find yourself on a foreign branch in a worktree, stop and create a new one.

**Every change requires a branch + PR.** This includes docs-only changes (`docs/`, `specs/`, ADRs, `README.md`, `AGENTS.md`, `CLAUDE.md`, and other `*.md` outside `apps/`/`crates/`) and trivial code fixes under ~20 lines (typos, comment edits, single-line config tweaks). There are no direct-to-`main` pushes. Branch protection requires one approving review plus the required status checks for all PRs; GitHub blocks self-approval, and this repo has a single maintainer, so `enforce_admins` is disabled and the maintainer lands PRs via explicit admin override (`gh pr merge --admin`) — a deliberate, per-merge act, not a general exemption (policy decision, 2026-08-30). Required status checks still gate non-admin actors, and `main-ci.yml` re-validates `main` itself after every merge. The `/code-review` gate below still applies only to PRs touching code, not to docs-only PRs.

**Single-maintainer review and merge handoff:** GitHub rejects approval from the PR author; do not spend time retrying `gh pr review --approve` from the author account. If you are not the repository maintainer, request an approving review and stop at that external gate. If you are the maintainer of this single-maintainer repository, wait for every required status check to pass and complete the applicable `/code-review low` gate, then use the explicit, per-PR admin path:

```bash
gh pr merge <PR_NUMBER> --squash --admin
```

The admin override is the documented recovery for the impossible self-approval case; it is not permission to merge failing or unreviewed work, and it does not make OrkWorks or an agent the approver. The contract is checked by `scripts/branch-protection-policy-check.sh` in PR CI.

**One PR per logical unit of work.** A burst of 5–10 small commits in a few minutes that share a feature name is one PR, not ten commits on main. Squash or rebase locally before opening it.

**Review gate:** PRs that touch code under `apps/desktop/` or `crates/orkworksd/` must have a `/code-review` run before merge. Always invoke it with an explicit effort argument — **`/code-review low` is the default and the expected case**: a diff-scoped pass over the changed lines only, no repo-wide exploration, reporting only findings that would change the diff. A bare `/code-review` (harness default effort) does not satisfy this gate. Escalate to medium effort or higher only for bigger or riskier changes: cross-cutting architecture/runtime work, concurrency or lifecycle changes, protocol/schema/migration changes, security-sensitive work, or unusually large diffs (roughly more than 8 code files or 500 lines) — and if a PR grows into that territory, prefer splitting it over escalating the review. Address findings or note why each is intentional in the PR description. `pr-review.yml` (see "CI routing" above) posts an automated first-pass comment on the same escalation-sized PRs; treat it as a convenience heads-up, not a substitute for the manual run this gate requires. GitHub's native Copilot code review (also configured under "CI routing") may post a second, independent review comment on PRs into `main`; it is likewise a heads-up, not a substitute for this gate.

**Squash-merge by default.** Preserve multiple commits only when the history tells a story worth keeping (e.g. a refactor followed by a focused fix on top).

**Stranded branches:** branches that go >7 days without merging must either be rebased and progressed, or closed with a one-line reason in the PR. No long-lived dev branches. The same rule applies to stranded worktrees.

**Recovering `main`:** If the primary checkout is detached, do not check out `origin/main` or use a linked worktree as a substitute. Inspect `git worktree list --porcelain`. If another worktree holds `main`, ask its owner to restore its owner branch or remove it. Only when that specific worktree is clean and you are explicitly authorized may you perform that recovery yourself. Never detach the worktree or use force operations. If an active owner or uncommitted changes would be affected, stop and obtain direction.

**Parallel work:** when more than one branch is in flight at once (multiple agents running concurrently, a hotfix on top of an in-progress feature), use `git worktree` so each branch has its own filesystem checkout — branch-switching in the main checkout will collide with other agents' uncommitted edits and build output. Also use a worktree whenever the active branch in the primary checkout is one you did not create, even if no other agent is running. Invoke the `starting-work` skill before opening a worktree for the path convention, per-worktree setup, and cleanup steps.

**Clean up your worktrees when done.** Remove the worktree and prune it as soon as the branch merges (or the task is abandoned). Leaving stale worktrees behind wastes disk space and confuses subsequent `git worktree list` output. The `starting-work` skill includes the exact cleanup commands.

Because parallel agents each see only their own worktree, none of them individually notices the fleet-wide sprawl this creates — see the [worktree currency check](#worktree-currency-check) below, which runs at the end of every session and reports on all worktrees, not just the current one.

## Decision tracking

Architecture decisions are captured as ADRs in `docs/adr/`. Each significant architectural, stack, protocol, or boundary decision gets a numbered markdown file. See the [development workflow reference](docs/agents/development-workflow.md) for lifecycle and curation detail.

- **Template**: `docs/adr/template.md`
- **Index**: `docs/adr/README.md`
- **Create an ADR** before writing any implementation code for a decision that shapes the architecture, stack, or protocol. If the decision only becomes clear during implementation, pause, write the ADR, and continue.
- A decision is reversed or replaced when: (a) a new ADR explicitly contradicts a prior ADR, or (b) implementation diverges from what an existing ADR records.
- **Supersede** old ADRs (don't delete) when a decision is reversed or replaced. In case (b), write the new ADR first, then update the old ADR status to `superseded` and reference the new ADR number.
- Before changing an existing ADR, follow the [ADR change sequence](docs/agents/development-workflow.md#adr-change-sequence): amend it when the decision still stands and the change clarifies its consequences; supersede it only for a reversed decision or deliberate divergence.
- **Keep the index updated** — add each new ADR to the `docs/adr/README.md` table.
- **The `## Architecture` section's inline ADR bullets are curated, not comprehensive** — see the eligibility rule stated there. When you supersede an ADR that has an inline bullet, remove that bullet in the same change (the README table remains the historical record). When you add prose elsewhere (e.g. `docs/agents/architecture.md`) that covers an ADR already inlined here, collapse its bullet to a pointer in the same change.

## Key naming

| Term | Meaning |
| ---- | ------- |
| OrkWorks | Product |
| `orkworksd` | Rust backend sidecar |
| Peon | Low-cost session/repo metadata observer |
| Taskmaster | Workspace-level next-step coordinator |
| `.orkworks/` | Global metadata directory under `~/.orkworks/` (workspaces/<hash>/, harnesses.json, hook-scripts/) |

Use `Coding tool` in UI, `harness` internally for that integration abstraction,
and `Model provider` only for inference services and local inference runtimes.
Use normal engineering terminology for all other concepts; Peon and Taskmaster
are the only intentional product-specific worker names.
See [product boundaries and terminology](docs/agents/product-boundaries.md) for
the detailed naming rules.

## Architecture

Provider process cleanup uses Windows Job APIs through the existing
`windows-sys` dependency; preserve suspended-child assignment and bounded cleanup
when changing inference transports. See [provider process ownership](docs/agents/architecture.md)
and the [ADR 0055 amendment](docs/adr/0055-json-taskmaster-inference-adapters.md).

Electron + React/TypeScript frontend (`apps/desktop/`) communicates with a Rust sidecar (`crates/orkworksd/`) over a dynamic localhost HTTP/WebSocket port. The desktop UI uses Dockview draggable panels around xterm.js terminal sessions, and renders plan/spec content in the Review tab as Markdown via `react-markdown`/`remark-gfm`. The sidecar manages PTY sessions, Git context, the metadata protocol (under `~/.orkworks/workspaces/<hash>/`), Peon observation, and Taskmaster recommendation state. The desktop `pnpm dev` command builds the debug sidecar before launching Electron so development runs use the current Rust implementation.

An ADR earns a bullet below only while it is `accepted` (not superseded), constrains how agents should write code, and has no independent prose summary elsewhere — once another doc gains prose coverage of an already-inlined ADR, its bullet collapses to a pointer. See [`docs/adr/README.md`](docs/adr/README.md) for the full historical index, including superseded decisions, and [`docs/agents/architecture.md`](docs/agents/architecture.md) for the full inter-component breakdown (port discovery, preload bridge, API data flow, Rust modules, panel layout).

- ADR 0022: PTY lifetime is session-runtime-owned in the sidecar; renderer terminal attachment is detachable and does not own process lifetime.
- ADR 0028: Harness version-probe results are cached with bounded TTLs and generation-aware invalidation, preserving the integration action's post-probe identity revalidation.
- ADR 0051: Codex's generated/local four-event hook bundle captures `harness_session_id` at `agent`-tier confidence and reports deterministic turn attention through the existing `JsonHookHandler`/reporter-script framework; `SessionStart` remains identity-only while `UserPromptSubmit`, `PermissionRequest`, and `Stop` drive validated per-session authority.
- ADR 0037: Plan/spec paths can be reported through a dedicated path-only sidecar route (`POST /sessions/:id/plan-path`) that canonicalizes the file and stores its workspace-relative form without changing session attention, superseding terminal-text inference when a harness reports a canonical file path. Codex remains on the conservative terminal fallback because its hook payload provides patch text, not a canonical file path.
- ADR 0025, 0026, 0031, 0032, 0042: prose lives in [`docs/agents/architecture.md`](docs/agents/architecture.md).
- ADR 0038: prose lives in [`docs/agents/harness-integration-contracts.md`](docs/agents/harness-integration-contracts.md).
- ADR 0052: one `orkworksd` process owns a workspace's metadata at a time through an OS advisory lease; workspace open/switch returns conflict before orphan reconciliation when another sidecar holds it.

## Metadata protocol

The detailed paths, bounds, lifecycle, authentication, and ADR reference are in the [architecture concept](docs/agents/architecture.md#metadata-protocol).

- Metadata is workspace-scoped under `~/.orkworks/workspaces/<hash>/`; global harness definitions and stable hook reporters live under `~/.orkworks/`.
- Priority: user > agent > peon > backend_inference > process > unknown > debug.
- Peon reads terminal output and writes inferred metadata; it never types into terminals.
- Detached runtimes keep draining terminal output, persisting history, and feeding Peon while `orkworksd` remains alive; losing a renderer terminal attachment alone must not end a session.
- Taskmaster proposes cross-session transitions, but v1 requires explicit user approval for every action. `improve_workflow` may display without approval, but cannot focus a terminal, edit a file, or start a session; a user may dismiss it or accept it to send a scoped fix prompt to their active session.
The current metadata paths and behavior are also summarized below for quick
operational reference; the architecture concept remains authoritative for the
full protocol detail.

- `~/.orkworks/workspaces/<hash>/sessions/<id>.json` — session state. (design, not yet implemented — see issue #313) Gains a current-summary snapshot (`summary`, `summarySource`, `summaryConfidence`, `summaryObservedAt`, all four updated or cleared together — ADR 0042)
- `~/.orkworks/workspaces/<hash>/events/<id>.ndjson` — append-only event log with durable, exact consecutive-deduplicated summary checkpoints and accepted provenance
- `~/.orkworks/workspaces/<hash>/events/<id>.terminal` — recent raw terminal replay, bounded on append to the newest 1,000 lines and 1 MiB; existing oversized dormant files remain unchanged until their next append
- `~/.orkworks/workspaces/<hash>/events/<id>.terminal-size` — the PTY's `cols`x`rows`, used to render dead-session terminal replay at its recorded size instead of the current panel width. Written authoritatively at the moment a session reaches a terminal status (`killed`/`ended`/`error`), and best-effort on every live resize so a daemon restart mid-session still leaves a usable last-known size for orphan reconciliation (`metadata::reconcile_orphaned_session`), which has no in-memory runtime handle to read a size from and never reaches the terminal-status transition itself. Still absent for sessions that ended before this file existed and for sessions that never lived long enough to receive a resize before an untimely daemon restart — both cases fall back to fit-to-container replay, which can misrender recorded output that used absolute-column cursor addressing computed for a different width than the container happens to fit to.
- `~/.orkworks/workspaces/<hash>/workflow-observations/<session-id>.ndjson` and `~/.orkworks/workspaces/<hash>/workflow-observations/sequence` — bounded (1,000 records/2 MiB per session), sequenced, immutable `WorkflowObservation` evidence recorded through one shared module (`workflow_observations.rs`) from the authenticated `POST /sessions/:id/workflow-observations` agent-report route (`http/workflow_observation_handlers.rs`); durable improvement evidence for Taskmaster, deliberately separate from the current-summary snapshot above (ADR 0042). The route authenticates with a per-session `ORKWORKS_REPORT_TOKEN` bearer capability, generated from OS randomness (`getrandom`) at session start/resume and never persisted, logged, or serialized; session creation/resume fails closed if OS randomness is unavailable rather than spawning with a weak or empty token. Peon-inferred recording and Taskmaster's `improve_workflow` correlation are implemented.
- `~/.orkworks/workspaces/<hash>/capacity/<id>.json` — capacity per model/harness
- `~/.orkworks/workspaces/<hash>/recommendations/<id>.json` — Taskmaster recommendation state and history
- `~/.orkworks/workspaces/<hash>/workspace.json` — workspace memory, including the last active session
- `~/.orkworks/workspaces/<hash>/.sidecar.lock` — retained lock file whose OS advisory lock identifies the sidecar currently owning workspace metadata; lock ownership releases automatically when that sidecar exits (ADR 0052)
- `~/.orkworks/workspaces/<hash>/codex-hook-observation.json` — the last Codex hook fingerprint observed executing; Settings reports Codex activation only when it matches the currently installed hook definition
- `~/.orkworks/workspaces/<hash>/integrations/aider.json` — versioned OrkWorks-owned Aider notification-command preference
- `~/.orkworks/harnesses.json` — global harness definitions
- `~/.orkworks/hook-scripts/` — stable copies of harness reporter scripts (e.g. the Claude Code Notification hook), installed hook commands always point here rather than at the packaged/dev source, so they keep working across app updates and packaging schemes whose own paths aren't stable at runtime (Linux AppImage's per-launch mount point, in particular). The workspace-local harness hook configuration that invokes these reporters is gitignored and must not be committed.
- Priority: user > agent > peon > backend_inference > process > unknown > debug
- Peon reads terminal output, writes inferred metadata, never types into terminals
- Detached runtimes continue draining terminal output, persisting history, and feeding Peon while `orkworksd` stays alive; losing the renderer terminal attachment alone must not end the session
- `GET /sessions/:id/summary-log` exposes checkpoints in append order as timestamp, summary, source, and nullable confidence; missing data returns `{ "entries": [] }`. Rendered in the session detail panel as "Task history," distinct from the session's `label` (title), which is a stable, one-shot Peon-authored topic rather than this turn-by-turn activity log (ADR 0029).
- Taskmaster consumes normalized metadata and proposes cross-session transitions; v1 requires explicit user approval for every action. The implemented passive `improve_workflow` recommendation requires no approval to *display* — it still cannot focus a terminal or edit a file on its own, and it never starts a session. It can be dismissed, or explicitly accepted by the user to send a generated fix prompt into the user's currently active session, scoped to the recommended target surface (ADR 0042, ADR 0048).

## Key conventions from specs

- MVP does not own Git workflow, worktree management, merging, or arbitrary task decomposition
- Taskmaster may recommend session transitions but must not start sessions without explicit user approval in v1
- If asked to implement something listed as a non-goal in the specs, decline and explain which non-goal applies. Do not implement it even partially.
- Harness voice is pass-through only — OrkWorks never captures/proxies/stores audio for native voice
- Store metadata source and confidence where possible
- See [product boundaries and terminology](docs/agents/product-boundaries.md) for detailed scope, naming, and product conventions.

## Product design principles

These are load-bearing constraints on desktop UI work. A session is context;
switching sessions is the context-switch primitive. Keep one active terminal and
do not build multi-terminal, tiled, split, stacked, or picture-in-picture
views. Improve situational awareness through fast context switching, not
parallel visibility. See [ADR 0013](docs/adr/0013-single-active-context-primitive.md)
and [product boundaries and terminology](docs/agents/product-boundaries.md).

## APM and agent plugins

Agent dependencies (Superpowers, Ponytail, Claude Mem, rust-skills) are managed by [APM](https://github.com/anthropics/apm) at the repo root (`apm.yml`). Run `apm install` from the repo root to populate skills and hooks for all configured targets (claude, codex, copilot, opencode). APM lifecycle commands are trust-gated; run `apm lifecycle trust` once in a new checkout so the post-install/post-update repair can remove Superpowers' incompatible Codex `SessionStart` registration. The repair can also be run directly with `bash scripts/repair-codex-session-start-hooks.sh`.

See [`docs/agents/apm.md`](docs/agents/apm.md) for the full plugin list, generated path layout, and OpenCode configuration.

## Codex API failures

When Codex reports a generic API failure such as `Error in Codex API`, use the
read-only repository helper `bash scripts/codex-api-diagnostics.sh` and follow
[`docs/agents/codex-api-troubleshooting.md`](docs/agents/codex-api-troubleshooting.md).
Preserve the exact error, status/code, request ID, timestamp, and timezone
before retrying. Do not loop blind retries, print credentials or config
contents, or resume/reopen another session to work around the failure.

## Peon provider timeouts

When Peon reports a provider timeout, use the read-only repository helper
`bash scripts/peon-timeout-diagnostics.sh` and follow
[`docs/agents/peon-timeout-troubleshooting.md`](docs/agents/peon-timeout-troubleshooting.md).
`PEON_TIMEOUT` is legacy and does not control current session inference; check
the applied provider/model in Settings instead. Retry the current session once
after verification and do not resume or reopen another session as a workaround.

## MCP configuration

Project-scoped MCP servers are declared in `apm.yml` under `dependencies.mcp`.

- Do not hand-maintain parallel MCP definitions for Copilot, Claude, Codex, or OpenCode when APM can generate them.
- After changing `dependencies.mcp`, run `apm install` to refresh the project config files.
- Keep secrets out of committed files. No MCP servers are currently declared in `apm.yml`.

## Repo-level skills

The `skills/` directory contains committed repo skills (`starting-work`, `cutting-release`, `writing-skills`, `clean-ddd-hexagonal`, `adding-harness`, `working-on-recommendation`, `surfacing-blind-spots`, `auditing-test-honesty`, `walking-failure-paths`, `grooming-the-board`, `auditing-signal-vs-noise`, `consulting-the-brain`, `orchestrating-task-graphs`). `orchestrating-task-graphs` applies whenever multi-agent work is being planned or dispatched — it forces the fake-edge test before fanning out, separate verifiers with different questions, one owned merge, and the guardrail caps. Each is a directory with a `SKILL.md` following the [Agent Skills standard](https://agentskills.io/specification). Use `skills/adding-harness/` before adding or changing a harness adapter; it forces the launch/resume/session-ID/voice/capacity checklist for the harness. Use `skills/working-on-recommendation/` whenever a Taskmaster recommendation starts work; it resolves the recommendation, scopes edits, and reports verified completion. Use `skills/surfacing-blind-spots/` when closing out a session or when asked to generate quality-improvement tasks; it turns investigated uncertainties and project blind spots into scoped issues. Use `skills/consulting-the-brain/` when asked to analyze, verify, or improve OrkWorks' own agentic workflow using the owner's external "brain" knowledge repo (`Rambolarsen/brain`, viewer at rambolarsen.github.io/brain).

Five of these are **audit skills** that generate quality-improvement work: `surfacing-blind-spots` (uncertainties and blind spots), `auditing-test-honesty` (tests that don't pin what they claim), `walking-failure-paths` (behavior under external failure), `grooming-the-board` (board/code/spec drift), and `auditing-signal-vs-noise` (UI truthfulness). They share the guardrail filter and issue format defined in `skills/surfacing-blind-spots/`. The weekly `quality-audit.yml` workflow rotates through them; they can also be run ad hoc.

## Doc currency check

Before ending any session, run:

```bash
bash scripts/doc-check.sh
```

Address all flagged files before closing. See the [development workflow reference](docs/agents/development-workflow.md) for check behavior and CI/harness integration.

## Consolidated verification

After implementation changes, run the repository's one-shot verification
helper:

```bash
bash scripts/verify-repo.sh
```

It runs the required Rust, desktop, documentation, formatting, diff, and
worktree checks in a fixed order and stops at the first failure. Use the
individual commands from the scoped instructions when narrowing a failure;
`bash scripts/verify-repo.sh --dry-run` only displays the sequence and is not
verification.

## Worktree currency check

Before ending any session, also run:

```bash
bash .claude/hooks/worktree-check.sh
```

Only act on branches you own. See the [development workflow reference](docs/agents/development-workflow.md) for fleet-wide check behavior and follow-up.

## Maintaining AGENTS.md and README.md

Keep both files current as the project evolves. Update them when runtime
dependencies, planned architecture directories, `apm.yml` agent targets,
documented conventions or workflows, or ADRs change. Treat stale docs as a bug.
Also keep `docs/agents/domain-entities.md` current when `SessionMetadata`,
session/API vocabulary, or terminology boundaries change. See the
[development workflow reference](docs/agents/development-workflow.md) for
maintenance detail.
