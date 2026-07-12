# Simplified session lifecycle and alive-only attention

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-07-12

## Context

ADR 0021 made lifecycle finalization explicit, but its frontend vocabulary
overlaps process liveness (`running`), observer activity (`working`, `idle`,
`stale`, `done`), and terminal outcomes. This makes normal session state more
complicated than the user needs to interpret.

## Decision

- Canonical lifecycle is `creating -> alive -> stopping -> dead`.
- Canonical attention exists only while lifecycle is `alive` and is one of
  `working`, `idle`, `needs_you`, `blocked`, `failed`, or `capped`.
- `running` is not a canonical state; it duplicates `alive`. `done` is not a
  canonical attention state; session completion is represented by `dead` and
  terminal outcome. `stale` normalizes to `idle`.
- `stopping` retains ADR 0021's durable pending outcome, captured observer
  snapshot, bounded final scan, fallback snapshot, and startup recovery.
- Terminal outcome and final observer snapshot are history/debug data, not
  normal frontend state labels.

## Consequences

- The renderer consumes lifecycle and attention independently and never uses
  process status as an attention fallback.
- Compatibility fields (`status`, `lifecyclePhase`, `observedStatus`, and
  `connectivity`) remain temporarily derivable for older desktop builds. New
  writes and renderer behavior use the canonical model once the migration
  lands; a later protocol-removal decision removes those compatibility fields.
- ADR 0021 is superseded for its public vocabulary; its race-prevention and
  recovery invariants remain required.
