---
type: Product Reference
title: Product boundaries and terminology
description: Product scope, naming rules, non-goals, and load-bearing UX constraints.
tags: [orkworks, product-scope, terminology, ux]
status: stable
---

# Product boundaries and terminology

## Product scope

OrkWorks is local-first mission control for AI coding sessions. Peons observe
individual sessions; Taskmaster recommends what should happen next across
harnesses, models, reviews, capacity, and Git context. OrkWorks observes and
recommends before it controls: it does not replace Claude Code, Codex,
OpenCode, Gemini CLI, or Aider.

The MVP does not own Git workflow, worktree management, merging or arbitrary
task decomposition. The proposed [ordinary-child orchestration extension](../../specs/taskmaster.md#proposed-ordinary-child-orchestration-extension)
is a narrow exception: a UI-created run coordinates exactly approved plans
through ordinary child sessions, with separate worktrees for independent
chains. Research completion preserves planning authority, never execution
approval. Sequential reuse within a plan requires a terminal predecessor and
explicit user quiescence acknowledgement. Edits remain for manual integration.
The initial slice has no automated cleanup; any later separately reviewed
removal requires a clean, quiescent, plan-owned worktree. Orchestration never
commits, merges, transfers edits or deletes branches.

Scope is accepted; detailed contracts, coding-tool capability evidence and
scoped implementation approval remain open under [#610](https://github.com/Rambolarsen/orkworks/issues/610).
Ordinary Taskmaster recommendations retain per-action approval. If a request
is a specified non-goal, decline it and identify that non-goal; do not implement
it partially. The proposed exception provides no native confinement or new
independent-instance cleanup/replacement authority.

Harness voice support is pass-through only. OrkWorks never captures, proxies,
or stores native-voice audio. Preserve metadata source and confidence wherever
possible. Capacity states are `healthy`, `degraded`, `capped`, `unknown`, and
`disabled`; cost tiers are `local`, `low`, `medium`, `high`, and `premium`.

See the authoritative [MVP specification](../../specs/orkworks-mvp.md), the
[native harness voice-support design](../../specs/native-harness-voice-support.md),
and the [Taskmaster specification](../../specs/taskmaster.md) for product
scope.

## Naming

| Term | Meaning |
| ---- | ------- |
| OrkWorks | Product |
| `orkworksd` | Rust backend sidecar |
| Peon | Low-cost session/repo metadata observer |
| Taskmaster | Workspace-level next-step coordinator |
| `.orkworks/` | Global metadata directory under `~/.orkworks/` (`workspaces/<hash>/`, `harnesses.json`, `hook-scripts/`) |

In user-facing UI, call CLI coding applications a **Coding tool**. Internal
code and metadata use **harness** for that integration abstraction. Reserve
**Model provider** for inference services and local inference runtimes.

Use normal engineering terminology for every other concept. Peon and
Taskmaster are the only intentional product-specific worker names; do not
expand the fantasy naming without an explicit spec update.

## Single-central-context UX invariant

The desktop shell shows one central context at a time: Terminal, Review, or an
eligible Workflow overview. There remains one selected terminal, and selecting
an actual session explicitly switches to that session's Terminal. The compact
Sessions list carries cross-session awareness; inspecting a workflow node,
restoring a surface, or opening an inspector does not select or acknowledge a
session.

Do not propose, plan, or build tiled, split, stacked, or picture-in-picture
terminal views. Do not add app-panel drag-and-drop, floating, docking, tab
reordering, or tab-based shell navigation. Fixed region resizing remains
available. Navigation restore is presentation state only and never resumes a
session, run, or action.

When improving situational awareness or throughput, favor fast context
switching—keyboard navigation, MRU ordering, and jump-to-session search—not
parallel visibility. See [ADR 0078](../adr/0078-fixed-desktop-shell-and-central-navigation.md)
for the accepted shell and navigation contract.

## Related context

- [Project context](project-context.md) covers the product's repository state
  and development environment.
- [Architecture](architecture.md) describes the runtime and metadata design.
- [Development workflow](development-workflow.md) covers repository rules for
  implementing scoped work.
