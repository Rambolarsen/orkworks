---
name: surfacing-blind-spots
description: Use when closing out a work session before declaring it complete, when asked to audit OrkWorks or generate quality-improvement tasks, or when the backlog needs fresh issues covering risks, hidden assumptions, and missed alternatives.
---

# Surfacing Blind Spots

## Overview

Two complementary questions expose the two failure modes of an AI work session: what the **agent** doesn't know, and what the **project/user** hasn't considered. Answering them honestly — and *investigating* the answers instead of just listing them — routinely changes the direction of work or produces backlog items nobody asked for but everyone needed.

**Core principle: an uncertainty you listed but did not investigate is not a finding — it is a confession.** The output of this skill is investigated findings turned into scoped GitHub issues, never a raw list of doubts.

## Two modes

| Mode | Trigger | Scope of the questions |
| ---- | ------- | ---------------------- |
| **Session close-out** | Finishing any implementation or review task, before `verification-before-completion` | The work just done in this session |
| **Codebase audit** | Asked to "generate quality tasks", "audit X", or improve OrkWorks generally | A named area (e.g. `crates/orkworksd/src/runtime/`, the metadata protocol, PR CI) or the whole project |

For a codebase audit, pick a bounded area first. "The whole repo" produces shallow findings; one subsystem produces deep ones. Rotate areas across runs.

**Start bounded, follow the data flow.** The boundary sets where investigation *starts*, not where it must stop. When a finding's severity depends on code outside the area (the other end of a socket, the caller of an API, the consumer of a file format), follow it — the first live run's biggest finding required reading the frontend peer of the audited Rust module. What the boundary forbids is *drifting*: opening unrelated subsystems because something looked interesting. Follow edges out of the area only to verify or size a finding you already have.

## Phase 1 — Agent uncertainty

Ask yourself, in writing:

> **What am I least confident about right now?**

Enumerate concretely, in these categories:

- Things not investigated (files never opened, paths never executed)
- Assumptions made without evidence
- Edge cases considered but not verified
- Dependencies and integrations taken on faith (harness behavior, PTY/OS differences, Electron/packaging quirks)
- Tests that pass but don't actually pin the behavior they claim to

Then **investigate every item that could change the outcome** before moving on: read the code, run the test, check the spec, reproduce the edge case. Only items that survive investigation as *real, unresolved risks* become candidate findings. Items you resolve on the spot are just work you finished late — fix or note them, don't file them.

## Phase 2 — Project blind spots

Now invert the lens — critique the project's understanding, not your own:

> **What is the biggest thing this project is missing about the situation? What does it not realize?**

And the sharpening variants:

- If this breaks in three months, what is the most likely reason?
- What would a senior engineer question first, reading this cold?
- What assumption in the current design/spec would you challenge in review?
- What is the one industry-leading improvement nobody asked for?

Answer against the actual artifacts: the authoritative specs in `specs/`, the ADRs in `docs/adr/`, the open issue board, and the code. A blind spot that the specs already address (or explicitly reject) is not a blind spot.

## Phase 3 — Filter through repo guardrails

Every candidate finding passes this gate before becoming an issue. Findings that fail are dropped *with a one-line reason in your summary*, not silently.

1. **Non-goal check** — if the finding implies work listed under "Explicit MVP Non-Goals" in `specs/orkworks-mvp.md` (repo workflow manager, worktree manager, automatic merging, …) or barred by the constraints in `AGENTS.md` (multi-terminal views per ADR 0013, autonomous Taskmaster actions, voice capture/proxying), drop it and cite the source.
2. **Spec alignment** — if the specs cover it: candidate for an implementation issue. If the specs do **not** cover it: file a *spec-gap* issue proposing the spec update instead of an implementation issue. Never file implementation work outside spec coverage.
3. **Dedupe** — search open issues first. If an existing issue covers it, comment there if you have new evidence; do not file a duplicate.
4. **Scope** — one deliverable-sized unit per issue. A finding that needs a refactor *and* a test suite *and* a doc update is multiple issues or one issue with explicitly ordered checkboxes — never a vague mega-issue.

## Phase 4 — File the issues

Use the standard format:

```markdown
Title: <area>: <specific defect or gap>

## Context
What was found, how it was verified (file paths, commands run, spec sections).
Which blind-spot question surfaced it.

## Acceptance criteria
- [ ] Concrete, checkable outcome
- [ ] ...

Label/note: `quality-audit` provenance, plus spec section it traces to
(or "spec-gap" if it proposes a spec change).
```

Finish with a summary to the user: findings filed (with issue links), findings dropped and why, and anything fixed on the spot during Phase 1.

## Red flags — stop and restart the phase

- Filing an issue for an uncertainty you never investigated
- "Nothing much comes to mind" — the ritual answer; pick a category from Phase 1 and dig
- A finding phrased as a feature you'd enjoy building rather than a risk you found
- An issue with no checkbox acceptance criteria or no spec traceability
- More than ~5 issues from one run — you're padding; keep the sharpest findings

## Common mistakes

| Mistake | Fix |
| ------- | --- |
| Treating the question list as the deliverable | Investigate first; only survivors become issues |
| Auditing "everything" | Bound the area; rotate across runs |
| Blind spot duplicates an ADR's rejected alternative | Read `docs/adr/` before filing; supersede via new ADR if genuinely reopening |
| Filing implementation work with no spec coverage | File a spec-gap issue instead |
| Skipping the drop-list in the summary | Dropped findings with reasons are half the value — they show what was considered |
