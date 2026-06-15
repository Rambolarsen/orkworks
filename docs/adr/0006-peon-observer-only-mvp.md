# Peon: observer-only inference in MVP

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-06-15

## Context

Not all agents will reliably write session metadata. OrkWorks needs a fallback mechanism to infer session status, phase, and blockers from terminal output. However, an AI observer that can type into terminals creates safety risks (injected commands, approval bypass).

## Decision

Peon will be observer-only for the MVP. It reads terminal output, calls a cheap model with compact context, requires strict JSON responses validated against a schema, and writes inferred metadata to `.orkworks/`. Peon must never type into terminals, approve commands, modify source code, or override user decisions.

## Consequences

- Sessions get metadata even when agents don't report it themselves
- Safety boundary is clear: read-only observer, no terminal input
- Strict JSON schema validation prevents malformed metadata
- Peon is a cost driver (cheap model calls per session); must be configurable
- Later versions may add gated terminal input suggestions with explicit user approval
