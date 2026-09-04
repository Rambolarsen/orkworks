---
name: orchestrating-task-graphs
description: Use when coordinating multiple agents or subagents on one goal — deciding whether to fan out at all, drawing the plan as a graph of jobs, running parallel workers with separate verifiers, or merging multi-agent results into one deliverable. Triggers include "orchestrate", "fan out", "parallel agents", "multi-agent", or any task you are about to split across subagent dispatches.
---

# Orchestrating Task Graphs

## Overview

Design the topology agents work through, not the prompts. Nodes are jobs (each one something you would hand to a single assistant); draw an edge only when a job consumes another job's *result*. Most orchestration failures are topology failures: fanning out work that is secretly sequential, or verifying work in the same context that produced it.

## Before fanning out: the fake-edge test

Ask: **where does this work split into pieces that never read each other's results?** Split only those pieces. Everything where each step needs the full picture stays with one agent — multi-agent setups on sequential work degrade rather than help. Two corollaries:

- Splitting along component boundaries is not the test; independence of *results* is.
- If you cannot state a worker's inputs without referencing another worker's output, that edge is real — sequence it or move the connection to the merge node.

## The diamond

```
        ┌─ worker 1 ─┐
plan ───┼─ worker 2 ─┼─→ verify ─→ merge ─→ result
        └─ worker 3 ─┘
```

- **Plan (orchestrator):** write the routing down *before* dispatching — workers, their exact questions, dependencies, caps. The routing is fixed; agents fill jobs, not redesign the plan mid-flight.
- **Workers:** parallel, isolated contexts, read-only when the deliverable is a report. Require every claim to carry a `file:line` (or equivalent) citation and explicit `UNVERIFIED` flags instead of guesses.
- **Verify — separate contexts, different questions.** A model grading its own work in its own context misses most of its own mistakes; a fresh verifier that grades the *merged draft* catches what workers cannot see. Give each verifier a different question — *is every claim correct?* vs *what is missing?* minimum. The correctness verifier checks citations against the artifacts; the completeness verifier hunts for what nobody covered.
- **Merge — one owned node.** The orchestrator reconciles cross-worker tensions into a single draft *before* verification (verification of a paste-job verifies mush), then applies verifier verdicts. Exactly one owner; never merge by concatenation.

## Guardrails (non-negotiable caps)

1. Maximum **one round** without returning to the human — no silent rework loops.
2. **One writer per artifact** — no two jobs mutate the same file; prefer read-only workers plus an explicitly owned write step.
3. **Hard cap on agent count** — state it in the plan before dispatching.
4. **Judge results on evidence, not self-reports** — verifier verdicts against the artifact, never worker claims of success.

## The human gate

Route every irreversible step (commit, push, PR, publish, delete) through explicit approval — placed where a mistake is expensive to undo, not on every step. Reporting findings is not irreversible; landing them is.

## When NOT to fan out

- The work is sequential — each step needs the previous step's full result.
- The question is scoped to one module one agent can hold.
- You cannot articulate each worker's question without another worker's answer — sequence it instead.
- No subagent facility exists — do a single pass and self-verify by re-checking citations.

## Common mistakes

| Mistake | Fix |
| ------- | --- |
| Symmetric fan-out ("3 workers looks right") | Fan out on the fake-edge test's answer, not symmetry |
| One verifier, one generic "review it" question | Separate verifiers, different questions |
| Verifier reads worker summaries instead of artifacts | Verify against files/code, citations included |
| Merge = paste worker outputs together | Reconcile contradictions first; then verify the draft |
| Verification skipped because "workers were thorough" | Workers miss what fresh skeptics catch — always verify |

## Related skills

- `dispatching-parallel-agents` — the mechanics of spawning independent subagents safely
- `subagent-driven-development` — executing a written implementation plan one subagent per task
- `doubt-driven-development` — the adversarial-review instinct behind the verify node
