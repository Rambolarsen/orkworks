---
type: Policy
title: Subagent model policy
description: Default delegated-work model tier and conditions for escalation.
tags: [subagents, models, delegation, policy]
status: stable
---

# Subagent Model Policy

This project defaults to `gpt-6-luna` for delegated implementation and review
work. Resolve model IDs against the current session's supplied tool inventory
before dispatch; do not assume an older alias is still available. If the named
Luna model is unavailable, use an available Luna model and state the substitution.
If no Luna model is available, report that limitation before selecting a stronger
model under the escalation rule below.

The 2026-11-12 Codex tool inventory exposes these current choices:

| Purpose | Model ID |
| --- | --- |
| Default delegated implementation and review | `gpt-6-luna` |
| Authorized stronger general-purpose work | `gpt-6.1-sol` |
| Authorized stronger coding work | `gpt-6.1-terra` |
| Authorized demanding reasoning work | `gpt-6-astra` |

These are dispatch IDs verified in this session, not a pricing comparison or a
claim that every harness/account supports them. An explicit model override must
follow the current tool's context-inheritance requirements. In particular, the
current Codex collaboration tool requires `fork_turns: "none"` when overriding
the model; provide the task context explicitly.

Use Luna for:

- mechanical implementation tasks;
- focused task reviews;
- test-writing and verification tasks; and
- documentation or plan maintenance.

Use a stronger model only when the user explicitly authorizes it or when a
task is blocked after a Luna attempt and the additional reasoning is necessary
to make progress. Any deviation should be visible in the task commentary.

## Revisit note

Re-evaluate this policy whenever a new model family or Luna-tier release is
available, and before starting the next major implementation milestone. Compare
task quality, review findings, turnaround time, and cost, then update the
default model name and exception rules here if the evidence supports a change.

Last reviewed: 2026-11-12. Model IDs were checked against the supplied Codex
collaboration tool inventory. Two focused design reviews completed on
`gpt-6-luna`; no cross-model cost or quality benchmark was performed.
