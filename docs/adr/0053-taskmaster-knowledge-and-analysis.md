# Taskmaster knowledge distribution and independent analysis

- Status: accepted
- Deciders: repository owner
- Date: 2026-09-09

## Context

Deterministic workflow observations identify repeated friction but offer generic
fixes. The owner's brain contains reusable knowledge updated daily, which should
reach installations without requiring daily binary releases. Taskmaster analysis
needs a different model selection and workload budget from frequent Peon
observation. Historical knowledge must not masquerade as current workspace facts.

## Decision

Follow [the accepted knowledge specification](../../specs/taskmaster-knowledge.md).
Distribute allowlisted, signed, versioned reference data separately from app
binaries, with a bundled fallback. Electron main owns verified download and
activation; the sidecar owns bounded context collection, independent Taskmaster
inference, evidence validation, and recommendation lifecycle. The renderer owns
presentation through narrow settings/status contracts. No cross-imports between
Electron and renderer are introduced.

Use an explicit Taskmaster provider/model selection, never temporarily changing
Peon's applied selection. Preserve deterministic recommendations. Track proactive
repository facts and knowledge provenance separately from session observations.
Enforce context limits, durable app-wide call reservations, cache invalidation,
and user-approved handoff. Knowledge is untrusted reference content, not a source
of execution authority. Local decisions and completion records stay local.

## Consequences

Knowledge can evolve daily and old apps keep compatible snapshots. Publication
requires an explicit export boundary and signing secret. Downloads cannot run
code. Additional analysis is optional and bounded, but its monetary cost varies
with the selected model. A completion record is not proof of improved outcomes.
The existing observation evaluator and its dismissal rules remain independently
usable. Prior ADR 0042's observation separation and ADR 0048's explicit handoff
remain in force; this decision extends their evidence model.
