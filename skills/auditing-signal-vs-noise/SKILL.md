---
name: auditing-signal-vs-noise
description: Use when asked to audit UI truthfulness or generate situational-awareness improvement tasks — when displayed session state may be stale, flickering, overconfident, or missing relative to the metadata that feeds it.
---

# Auditing Signal vs Noise

## Overview

**Core question: on this UI surface, what does the user see that isn't true — and what's true that they can't see?**

OrkWorks' product principles make the sessions list and detail panel the situational-awareness surfaces: attention state, activity, and agent progress live there, not in parallel terminal views. That only works if the displayed state is *truthful*. This audit traces each displayed field back through the data flow to its source and files issues where the surface lies (stale, flickering, overconfident) or withholds (state exists in metadata but never reaches the user).

**A suspicion you did not trace to the source is not a finding.** The finding names the field, its source (`file:line` on both sides of the API), and the specific sequence that makes the display diverge from reality.

## Process

1. **Bound the area** — one surface per run: sessions list, session detail panel, capacity panel, or recommendations panel. Start bounded, follow the data flow (see `skills/surfacing-blind-spots/`) — every finding here crosses the frontend/backend boundary by nature.
2. **Inventory the surface** — list every piece of state it renders (status dots, labels, timestamps, badges, spinners).
3. **Trace each field to its source** and interrogate:

| Failure mode | Question to ask |
| ------------ | --------------- |
| Stale | What clears this value? Is there any state sequence where it's never cleared? |
| Flicker | Can this transition fire transiently during startup/shutdown/switching? (e.g. a 1-tick "idle" flash) |
| Overconfident | Metadata carries `source` and `confidence` — does the UI render an inference as if it were fact? |
| Priority inversion | Does a lower-priority source (peon, process) visibly overwrite a higher one (user, agent) contrary to the priority order in `AGENTS.md`? |
| Withheld | What does the backend know (blockers, questions, capacity hints) that this surface never shows? |
| Clock lies | Do "last activity" style timestamps update on things that aren't activity? |

4. **Verify with a sequence** — for each candidate, write down the concrete event sequence (create → output → silence → kill …) and check it against the code on both sides. Where cheap, reproduce against a running sidecar.
5. **Filter and file** — apply the guardrail filter and issue format from `skills/surfacing-blind-spots/` (Phases 3–4). Mind the non-goals hard: findings must improve *truthfulness of the existing surfaces*, never propose parallel terminal visibility (ADR 0013) or autonomous actions.

## Red flags — stop and restart

- A finding that is really a feature request for a new panel — this audit fixes lies and omissions, not layout
- "Withheld" findings that would surface raw noise (streaming inference guesses as fact) — surfacing must respect confidence
- Filing a flicker without the event sequence that produces it
- Proposing to show more terminals — that is context degradation by spec, drop it and cite ADR 0013

## Common mistakes

| Mistake | Fix |
| ------- | --- |
| Auditing the React components only | The lie usually originates in the backend state machine; trace both sides |
| Treating debounce/latency as a bug | Seconds-late is fine; *never-corrected* is the bug |
| Flagging confidence display everywhere | Only where acting on a wrong inference costs the user something |
