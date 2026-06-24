# Repo Learning Loop Design

- Date: 2026-06-23
- Status: proposed

## Summary

OrkWorks should add a repo-scoped setup-learning loop that turns repeated development patterns into draft recommendations for reusable repo artifacts.

The first slice should include:

- repo-local evidence collection from sessions, events, review artifacts, and existing agent setup files
- strict-by-default pattern detection for repeated workflow problems or repeated successful ad hoc patterns
- Taskmaster-authored draft recommendations for loops, skills, rules, hooks, docs, and related setup artifacts
- maintainer-gated review and promotion of approved drafts into committed repo assets
- rejection memory so OrkWorks stops resurfacing the same low-value idea after a maintainer declines it

The first slice should not include:

- autonomous mutation of repo setup
- silent runtime behavior changes for agents
- product-code patch drafting as part of the setup-learning flow
- cross-repo or user-global memory
- one-shot speculative loop creation without enough evidence unless repo policy explicitly lowers the threshold

## Problem

Repos repeatedly rediscover the same workflows, checklists, guardrails, and handoff patterns.

Today those patterns often stay trapped inside:

- terminal transcripts
- one-off prompts
- ad hoc review comments
- repeated explanations in docs and agent instructions

That wastes time and creates avoidable variance. A repo may already have the raw evidence that a reusable loop, skill, hook, rule, or doc update would help, but there is no structured path from "this keeps happening" to "this repo should capture it."

Memory and learning patterns are useful here, but OrkWorks must apply them within its own product boundary: observe, detect, recommend, and prepare reviewable artifacts without becoming an autonomous agent platform.

## Design Goals

- Keep learning strictly repo-scoped and explainable.
- Improve the repo's agent setup rather than silently changing agent behavior.
- Draft the smallest artifact that solves the repeated pattern.
- Reuse OrkWorks' existing Peon, Taskmaster, and review-surface direction instead of creating a parallel system.
- Default to conservative evidence thresholds and maintainer approval.
- Remember rejected ideas so the system becomes less noisy over time, not more.

## Proposed Design

### Product Boundary

OrkWorks gains a new capability: `setup-learning`.

Peons continue to observe sessions and repo artifacts. Taskmaster consumes that evidence and decides whether a repeated pattern is strong enough to justify a reusable repo-level improvement. When it is, Taskmaster drafts a recommendation package for review.

This capability is setup-facing by default. It may recommend:

- loop entries
- repo skills
- rules and guardrails
- hooks
- documentation and conventions
- related review or issue templates

It does not directly create or apply runtime behavior changes for active sessions. It does not automatically rewrite repo setup. Promotion into committed repo files requires explicit maintainer approval.

### Core Model

The learning loop has five stages:

1. `observe`
2. `detect`
3. `draft`
4. `review`
5. `promote`

`observe` gathers normalized repo-local evidence.

`detect` clusters repeated patterns such as:

- repeated manual workflow reconstruction
- repeated handoff friction
- repeated guardrail failures
- repeated doc or setup drift
- repeated prompt scaffolding that should become reusable

`draft` creates a concrete proposal for the smallest useful artifact set.

`review` surfaces the draft as a human-readable artifact with linked evidence and intended target surfaces.

`promote` happens only after maintainer approval. The first slice stops at draft creation plus reviewable promotion proposals, but the model leaves room for later "prepare apply-ready patch set after approval" behavior.

### Evidence And Data Model

The learning loop should store explicit, inspectable objects:

- `evidence bundle`
  - normalized references to sessions, events, review artifacts, test outcomes, and setup files that support the recommendation
- `learning candidate`
  - repeated pattern summary, cluster identity, confidence, threshold mode, and suggested artifact types
- `draft artifact`
  - the human-reviewable proposed loop, skill, rule, hook, doc, or mixed artifact package
- `decision record`
  - approved, rejected, deferred, superseded, with human rationale when available

Every recommendation must answer:

- what repeated pattern was detected
- which sessions or repo artifacts support it
- why a reusable repo artifact is the right response
- which repo surfaces would change

### Detection Logic

The default threshold mode is `strict`, with repo policy allowed to override it.

Strict mode requires at least:

- two completed occurrences across sessions
- outcome evidence that the occurrences are meaningfully similar
- a plausible repo-setup intervention that would have reduced repetition, risk, or friction

Similarity should consider more than text overlap. Relevant signals include:

- lifecycle phase
- blocker or workaround type
- repeated missing instruction or repeated ad hoc checklist
- repeated target repo surface
- repeated successful structure worth capturing

Repo policy may later allow:

- `balanced`
- `aggressive`

but the default is intentionally conservative.

### Artifact Selection Heuristic

Taskmaster should recommend the smallest artifact that solves the pattern.

- If the gap is a missing command or convention, draft docs.
- If the gap is a reusable sequence with checkpoints and stopping rules, draft a loop entry.
- If agents need operating instructions to use the sequence correctly, draft or update a skill.
- If enforcement matters more than guidance, draft a rule or hook.

The system should avoid turning every repeated idea into a heavyweight skill. Reuse and simplicity are more valuable than abstraction for its own sake.

### Repo Surfaces

Runtime learning state should live under `.orkworks/`.

Suggested runtime paths:

- `.orkworks/learning/candidates/*.json`
- `.orkworks/learning/drafts/*.md`
- `.orkworks/learning/decisions/*.json`

Approved artifacts should live in normal committed repo locations based on type, such as:

- `skills/`
- docs under `docs/agents/` or another repo-owned convention
- hook config or hook scripts in their existing repo locations
- a committed loop catalog such as `docs/agents/loops/` or another repo-defined path

This keeps draft state separate from promoted repo assets while still making approved learning part of the repo's versioned workflow.

### Repo Policy

Storage and promotion rules should be repo-owned, not global.

The first design assumes a committed repo policy file that defines:

- allowed artifact types
- threshold mode
- whether setup-learning can draft only or also prepare apply-ready patch sets after approval
- target promotion paths
- maintainer approval requirements

The default governance model is `maintainer gate`:

- Taskmaster may detect and draft
- maintainers decide whether a draft becomes committed repo setup

### Review Flow

Learning drafts should reuse the existing review-queue direction instead of inventing a second inbox.

Each draft review item should include:

- repeated-pattern summary
- evidence digest
- source sessions and artifacts
- suggested target surfaces
- expected benefit
- current confidence and threshold mode

Maintainer actions:

- `approve`
- `reject`
- `defer`

Approval authorizes promotion work. In the first slice that means preparing the repo-facing artifact set for confirmation, not silently applying it.

Rejection should be stored as a durable decision so the same low-value idea is not repeatedly resurfaced.

### Rejection Memory

Rejected ideas are first-class learning outcomes.

When a maintainer rejects a draft, OrkWorks should store:

- rejection reason when provided
- scope of the rejection
- the pattern cluster or similar-cluster identifiers it applies to
- whether the rejection is permanent, tentative, or time-bounded

Taskmaster should consult rejection memory before drafting or surfacing similar candidates. If a new candidate materially differs from the rejected one, it may still surface it, but it must explain why this case is meaningfully different.

This is a load-bearing part of the design. Without rejection memory, setup learning becomes repetitive process spam instead of useful curation.

## Components And Data Flow

1. Session Peons and repo Peons emit normalized session and repo signals.
2. Taskmaster reads those signals together with review artifacts, outcome history, and repo setup files.
3. Taskmaster clusters repeated patterns and checks the configured threshold mode.
4. If the threshold is met and no prior rejection blocks it, Taskmaster creates a learning candidate and a draft artifact.
5. The draft enters the review surface with a digest and linked evidence.
6. A maintainer approves, rejects, or defers the draft.
7. Approved drafts may be promoted into committed repo assets through a later confirmation step.
8. Decisions feed back into future recommendation behavior.

## Error Handling And UX

- If evidence is incomplete, Taskmaster should keep the candidate internal and avoid surfacing it until the threshold is met.
- If draft generation fails, the candidate may remain visible as "draft pending" rather than disappearing.
- If the review surface is unavailable, draft artifacts should still persist on disk for later inspection.
- If a repo lacks a learning policy file, OrkWorks should use conservative defaults and explain that the repo has not customized setup-learning governance.

The UX should feel like maintainers are reviewing proposed reusable workflow knowledge, not swatting away autonomous process chatter.

## Non-Goals

- Automatic promotion of setup-learning drafts into committed files
- Direct edits to active session prompts or running terminals
- Cross-repo memory synthesis
- User-global personality or preference modeling
- Broad code-change recommendation generation as part of this first slice
- Replacing issue tracking, PR review, or human architectural judgment

## Testing And Validation

Implementation should verify:

- repeated patterns are not surfaced before the strict threshold is met
- surfacing is blocked when a matching prior rejection applies
- materially different candidates can still surface after a rejection with an explanation
- the smallest-artifact heuristic prefers docs over skills when the gap is simple
- learning drafts appear in the review surface with linked evidence and digest data
- maintainer decisions persist across restart
- approved drafts remain separate from committed repo assets until the promotion step is explicitly confirmed

## Open Questions

None for this first slice.

Potential future work such as broader product-code recommendations or apply-ready patch generation should be handled as follow-up specs rather than left ambiguous in this design.
