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

The MVP does not own Git workflow, worktree management, merging, or arbitrary
task decomposition. Taskmaster may recommend session transitions, but v1 never
starts a session without explicit user approval. If a requested change is a
specified non-goal, decline it and identify the applicable non-goal; do not
implement it partially.

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

## Single-active-context UX invariant

A session is the unit of context, and switching sessions is the context-switch
operation. The sessions list is the multi-view across sessions; the active
terminal is deliberately singular. Do not propose, plan, or build tiled,
split, stacked, or picture-in-picture terminal views.

Parallel terminal rendering degrades context rather than improving visibility:
it divides attention and consumes space without improving situational
awareness. Put cross-session awareness in the sessions list (legibility,
attention state, last activity, and agent-action summary) and the focused
session's detail panel. The same one-active-context rule applies to future
context-bearing surfaces such as editors and agent transcripts.

When improving situational awareness or throughput, favor fast context
switching—keyboard navigation, MRU ordering, and jump-to-session search—not
parallel visibility. See [ADR 0013](../adr/0013-single-active-context-primitive.md)
for the decision and consequences.

## Related context

- [Project context](project-context.md) covers the product's repository state
  and development environment.
- [Architecture](architecture.md) describes the runtime and metadata design.
- [Development workflow](development-workflow.md) covers repository rules for
  implementing scoped work.
