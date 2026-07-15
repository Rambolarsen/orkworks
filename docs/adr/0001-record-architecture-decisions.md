---
type: decision
status: accepted
title: "Record architecture decisions"
---

# Record architecture decisions

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-06-15

## Context

OrkWorks is an early-stage project with an evolving architecture. Decisions made during development (stack choices, protocol design, boundaries, security posture) need to be captured so future contributors understand why things are the way they are.

## Decision

We will use Architecture Decision Records (ADRs) as described by Michael Nygard. Each ADR is a short markdown file in `docs/adr/` with a sequential number, title, status, context, decision, and consequences.

## Consequences

- Architectural intent is preserved and discoverable
- Contributors can understand past decisions without digging through git history or specs
- ADRs that are superseded are kept for historical context (marked `superseded`)
- Adds a small documentation burden per significant decision
