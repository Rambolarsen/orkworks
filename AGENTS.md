# OrkWorks — Agent Guide

Local-first mission control for AI coding sessions. Peons observe sessions;
Taskmaster recommends next steps. OrkWorks observes and recommends before it
controls; it does not replace the coding tools it integrates with.

## Working approach

Read this guide once, then load only the instructions, spec sections, and source
files relevant to the task. Follow links when their trigger applies; the
[knowledge index](docs/agents/index.md) is a map, not a reading checklist.
Use **pnpm** for all Node.js package management, including desktop and docs.

Size preparation to the actual decisions and risks:

| Work | Preparation before implementation |
| --- | --- |
| Routine: clear bugfix, docs/config edit, or bounded adjustment to existing behavior | Inspect the affected flow, state a brief approach and relevant checks, then proceed within the user's authorization. No separate design, written plan, complexity rubric, or repeat approval. |
| Feature with meaningful design choices | Resolve those choices and outline a short plan in chat. Save a plan only when needed for coordination, handoff, or the user's request. |
| Architecture, protocol/schema migration, security-sensitive change, or explicitly gated work | Use a written design and concise implementation plan; reuse approved artifacts. Resolve compatibility, recovery, evidence, and required approvals before dependent implementation. |

Multiple steps or files alone do not require the heavier path. Investigate a
specific uncertainty before escalating; reduce ceremony when evidence resolves
it. Ask when missing information changes scope, behavior, or authorization.
An existing approval remains valid within its scope. Get user approval of a
new or materially changed architecture, migration, or security design before
dependent implementation; one approval of the concrete design is sufficient.
Product-specific approval and capability gates remain binding.

This sizing policy **overrides conflicting skill defaults**, including blanket
design approval, speculative skill loading, mandatory plan files, full code in
plans, and fresh review of unchanged artifacts. Load a skill when its concrete
task/phase trigger applies; reuse already-loaded instructions. Use
`brainstorming` for unresolved design choices, `systematic-debugging` for bugs,
and `test-driven-development` for feature/bugfix code unless the user opts out.
Docs/config-only changes do not require TDD. Use `receiving-code-review` when
responding to feedback and `requesting-code-review` for substantial code work
before PR handoff.

For written plans, use `writing-plans` under the
[local plan policy](docs/agents/development-workflow.md#plan-complexity-and-quality)
and [reviewing-plans](skills/reviewing-plans/SKILL.md). Review once before
execution or handoff; review revisions only where scope, risks, or acceptance
criteria changed. Plans contain decisions, boundaries, deliverables, and checks;
use links and small contract examples instead of prewriting implementation.

## Task-specific context

Before changing a subsystem, read its canonical instructions:

| Scope | Read |
| --- | --- |
| Desktop | [apps/desktop/AGENTS.md](apps/desktop/AGENTS.md); preserve the `electron/` / `src/` boundary |
| Rust sidecar | [crates/orkworksd/AGENTS.md](crates/orkworksd/AGENTS.md) and `rust-skills` |
| Cross-component | Both scoped guides |
| Runtime, metadata, or integration behavior | Relevant [runtime contracts](docs/agents/runtime-contracts.md), [architecture](docs/agents/architecture.md), and [harness contracts](docs/agents/harness-integration-contracts.md) sections |
| UI behavior or terminology | [Product boundaries](docs/agents/product-boundaries.md) |
| Public docs | [Site maintenance](docs/agents/site-maintenance.md); claims must distinguish source behavior from published installers |
| Agent setup, APM, MCP, or OpenCode | [Agent plugins](docs/agents/apm.md); OpenCode must load root `opencode.json`, use plugins, and verify Superpowers discovery before implementation |

Read the applicable authoritative spec sections **before implementation**.
Follow referenced contracts where they constrain the change; do not read every
spec for every task. Stop if a required authoritative source is missing or
unreadable. Specs define product scope; issues track progress.

| Subject | Authoritative spec |
| --- | --- |
| Product scope, architecture, milestones, non-goals | [MVP](specs/orkworks-mvp.md) |
| Concurrent workspaces | [Multi-workspace](specs/multi-workspace.md); proposed, written-spec review required |
| Native coding-tool voice | [Voice support](specs/native-harness-voice-support.md) |
| Packaging and releases | [Release pipeline](specs/release-pipeline.md) |
| Selected-session plan/spec review | [Session plan review](specs/session-plan-review.md); supersedes [review queue](specs/review-queue.md) |
| Coordination and recommendations | [Taskmaster](specs/taskmaster.md) |
| Signed knowledge and independent analysis | [Taskmaster knowledge](specs/taskmaster-knowledge.md) |

## Scope and authority

Validate assumptions that change scope, target, permissions, or expected
behavior against the task and relevant authoritative sources. Investigate
before deciding. Do not resume, reopen, or modify another session based on
stale metadata or inferred status. Resolve a named or implied PR to a concrete
number/URL before acting using the
[PR reference procedure](docs/agents/development-workflow.md#resolving-a-pr-reference).

Track implementation on the [issue board](https://github.com/Rambolarsen/orkworks/issues).
Before picking work, read the [current priority and scope rules](docs/agents/development-workflow.md#issue-prioritization-and-scope).
If the board is inaccessible, stop rather than guess priorities or close work.
For work outside spec coverage, comment on the issue and request a spec update;
create an issue before implementing specified work that has none.

Before GitHub work, use
[troubleshooting-github-connectivity](skills/troubleshooting-github-connectivity/SKILL.md)
to confirm an authorized path to the exact target. A sandbox block is not an
authentication failure; read access does not establish write permission.

If the verified diff implements an active Taskmaster recommendation, complete
it through the authenticated sidecar API **before the PR reaches a terminal
state**, following [recommendation tie-off](docs/agents/development-workflow.md#taskmaster-recommendation-tie-off).
Use `working-on-recommendation` when starting from one; never edit
`~/.orkworks/` recommendation files directly.

## Branch and PR workflow

Every change, including docs, uses a branch and PR. Invoke
[starting-work](skills/starting-work/SKILL.md) before editing. Default to an owned
sibling worktree; the primary-checkout exception requires the skill's residue
check and explicit confirmation that it stays unshared. Do not commit on
another owner's branch without authorization. Only the primary checkout may
hold `main`; never detach or force another worktree to recover it.

One session owns one logical task through its PR's terminal state or a bounded,
explicit handoff. Do not launch another coding harness. For delegation, apply
`orchestrating-task-graphs` and the current lower-cost model tier first; use
explicit task ownership and isolated worktrees for concurrent writers.
See [branch procedures](docs/agents/development-workflow.md#branch-and-pr-workflow)
for session budgets, review evidence, recovery, and cleanup.

PRs touching `apps/desktop/` or `crates/orkworksd/` require a completed
`/code-review low` of the current code diff before merge. Escalate only for
documented size/risk conditions; automated reviews do not replace this gate.
Docs-only PRs are exempt. The single-maintainer admin merge path requires
passing required checks and the applicable review gate; no direct-to-main
pushes. Squash by default. Use `babysitting-pull-requests` after opening/adopting
a PR, with the bounded budget and complete comment inventory. Clean up owned
merged worktrees using `bash scripts/finish-pr.sh <PR_NUMBER_OR_URL>`.

GitHub rejects approval from the PR author; do not retry `gh pr review --approve`.
Non-maintainers stop at the external approval gate. As maintainer, wait for every required status check to pass
and complete the applicable code review, then use `gh pr merge <PR_NUMBER> --squash --admin`.
This per-PR exception is checked by `scripts/branch-protection-policy-check.sh`.

## Verification and documentation

All Markdown under `docs/` follows the [OKF 0.2 authoring profile](docs/agents/documentation-format.md).
Add new concepts to their directory index; `pnpm --dir docs docs:check` and
the docs build enforce metadata and index coverage, including historical plans.

Use `verification-before-completion` before completion claims, commits, pushes,
and PRs. Run checks appropriate to the changed surface; after code
implementation use `bash scripts/verify-repo.sh`. Required CI routing lives in
[project context](docs/agents/project-context.md#ci-routing).
Run `bash scripts/doc-check.sh` and `bash .claude/hooks/worktree-check.sh` at
close-out. Review warnings against the actual diff; update changed claims or
record why a warning needs no edit. Only clean up branches/worktrees you own.

Keep durable detail in its canonical concept or scoped guide. Change this root
guide when a universal rule or routing entry changes; dependency bumps and new
ADRs alone do not need new root prose. Follow the
[ADR lifecycle](docs/agents/development-workflow.md#architecture-decision-records)
before architecture/stack/protocol implementation. Update domain entities when
session metadata or lifecycle vocabulary changes. Never hand-edit generated
APM assets to establish repository policy.

Troubleshooting is loaded on demand: [Codex API](docs/agents/codex-api-troubleshooting.md),
[Peon timeouts](docs/agents/peon-timeout-troubleshooting.md),
[Peon/model-detection or repeated recommendation noise](docs/agents/peon-model-detection-troubleshooting.md),
and [Serena startup](docs/agents/serena-mcp-startup-troubleshooting.md).
