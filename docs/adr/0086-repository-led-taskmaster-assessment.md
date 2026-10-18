---
type: Architecture Decision
title: Repository-led Taskmaster assessment with bounded evidence selection
description: Approved two-stage manual assessment with confined follow-up reads and complete persisted input identity.
tags: [orkworks, architecture, taskmaster, assessment]
status: stable
---

# Repository-led Taskmaster assessment with bounded evidence selection

- Status: accepted; owner approved the concrete design and implementation plan on 2026-10-18 (#931)
- Deciders: repository owner
- Date: 2026-10-18
- Tracking: [#929](https://github.com/Rambolarsen/orkworks/issues/929)

## Context

The one-shot assessment can return no proposal because its supplied snippets
cannot resolve a concrete candidate. Source-code access expands permitted file
types but does not allow a follow-up. The owner approved this concrete repository-led design, including its protocol,
data format, evidence access rules and embedded implementation plan, on 2026-10-18.
Issue #931 records that approval and the runtime handoff.

## Decision

Extend the manual assessment from [ADR 0085](0085-taskmaster-assessment-and-analysis-outcomes.md)
with repository-first seed collection, one strict discovery response selecting
at most eight IDs from a permitted catalog, and at most one final assessment
call. The sidecar alone authorizes and performs reads. Preserve the total
32-file/64-KiB collection budget and 8-KiB per-file prefixes across both stages.
No commands, edits, new sessions or provider fallback are introduced.

The [concrete design and implementation plan](../superpowers/specs/2026-10-18-repository-led-brain-assessment.md)
defines catalog/seed bounds, strict stage contracts, selected-model execution,
format-2 manifests, revalidation, redaction, legacy gates, recovery and quality
acceptance. The owner approved those contracts at `432a3249ad9b8c6e5559a888b92a4944ec94d7fc`;
#931 records approval and authorizes runtime implementation.

Keep the existing final assessment envelope, signed knowledge eligibility,
recommendation family, completion path and crash-consistent pending transaction.
Store complete bounded input identities through an explicit format-2 record,
without full prompts or uncited file contents. Old binaries cannot enforce
the new semantics; use a forward fix with new admission disabled after rollout.

## Alternatives and consequences

A prompt-only adjustment cannot supply missing related files. An unrestricted
agent audit expands authority and usage beyond this need. Two stages add one
bounded read decision and at most one extra selected-provider call, while
preserving the advisory product boundary.

No-proposal remains valid. Test healthy repositories and refuted candidates as
well as genuine workflow gaps, and require real selected-model quality evidence
before claiming improved usefulness. This proposal does not change ordinary
Analyze now or background analysis and does not supersede ADR 0085's existing
lifecycle and authority decisions.
