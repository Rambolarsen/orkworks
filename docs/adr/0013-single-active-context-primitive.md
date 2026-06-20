# Single-active-context primitive: session = context, switching = context-switch

- Status: accepted
- Deciders: user
- Date: 2026-06-19

## Context

OrkWorks' identity ("Mission Control for AI Agents") invites a natural-looking design move: tile, split, or stack multiple terminal panes so the user can see N sessions running in parallel. The first pass of the 2026-06-19 design audit (`DESIGN-IS-2026-06-19/`) recommended exactly that, framing the current single-active-terminal panel as a usefulness failure to fix.

The user rejected this framing: showing many terminals at once is *context degradation*, not visibility. It divides attention, consumes screen real estate, and conflates "I can see it" with "I am working on it". The correct primitive is single-active context with deliberate, fast switching — the sessions list is the multi-view across N sessions, the active terminal is the one context the user is actually engaged with.

This is not a limitation to engineer around; it is the intentional UX axis the product is built on. Without an ADR pinning it down, future agents (and future planning sessions, as the audit just demonstrated) will keep re-proposing parallel terminal views and need to be corrected each time.

The principle also generalises beyond terminals: any future context-bearing surface — editors, agent transcripts, full-page logs — follows the same rule.

## Decision

OrkWorks' desktop UI is built around a **single-active-context** primitive:

1. A **session is the unit of context.** Switching sessions is the context-switch operation; the user changes session to change what they are working on.
2. The **sessions list is the multi-view.** Situational awareness across N sessions is the list's job — attention state, last activity, agent action summary, source confidence. Adding signal to the list is the right way to improve "what's happening across my work".
3. The **active terminal is single by design.** No multi-terminal / tiled / split / stacked / picture-in-picture views. Not as a default, not as an opt-in, not as a feature toggle.
4. The **detail panel is the secondary context** for the currently-focused session — it never becomes a parallel-context surface for a different session.
5. The right axis to improve when situational awareness or task throughput matters is **fast context-switching**: keyboard navigation, MRU ordering, jump-to-session search, focus-handoff guarantees. The wrong axis is parallel visibility.
6. The same logic applies to any future context-bearing surface (editors, agent transcripts, etc.): one active, switch deliberately.

This principle is also recorded as a project-wide rule in `AGENTS.md` under **Product design principles** so every harness loads it on every session.

## Consequences

- **Easier**: Design decisions about layout, panel composition, and information density have a clear test — does this expose more *signal in the index*, or does it try to render more *parallel context*? The first is in-scope; the second is out-of-scope.
- **Easier**: Sessions-list work (attention legibility, status labels, last-activity timestamps, agent-action summaries) gains explicit budget and priority — it is the load-bearing surface for multi-session awareness, not a secondary index.
- **Easier**: Onboarding of new agents and contributors — the rule is committed, referenceable, and self-explanatory. "Why no split terminals?" → ADR 0013.
- **Harder**: Resists a visually-impressive demo pattern. Tiled-terminal dashboards make better screenshots than a focused single-pane view; the product trades demo appeal for working usefulness.
- **Harder**: Puts more pressure on the sessions list to carry information density well. If the list is unreadable, the whole single-active-context bet fails — so token-layer work, plain-language enum labels, and dashboard legibility (audit moves 1-3) are no longer optional polish.
- **Reverses if**: a future ADR explicitly supersedes this one with documented user research showing that parallel context views measurably improve multi-session work without degrading focus. Until then, any feature proposal involving multi-terminal rendering is declined by default.

## Related

- `AGENTS.md` — Product design principles section (project-wide enforcement)
- `DESIGN-IS-2026-06-19/03-verdict.md` — redesign verdict that respects this principle
- ADR 0011 — Dockview panel layout (single-active-context shapes which panels open by default)
