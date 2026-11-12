---
type: Architecture Decision
title: Brain-guided manual improvement investigation
description: Proposed bounded progressive evidence collection with a shared assessment and preferred next improvement.
tags: [orkworks, architecture, taskmaster, brain, readiness]
status: draft
workflow_status: proposed
---

# Brain-guided manual improvement investigation

- Status: proposed; product direction approved on 2026-11-12, concrete contract pending.
- Decider: repository owner.
- Date: 2026-11-12.
- Tracking: [#1686](https://github.com/Rambolarsen/orkworks/issues/1686).
- Design: [Brain-guided next improvement](../superpowers/specs/2026-11-12-brain-guided-improvement-design.md).

## Context

The owner reports low readiness scores with no suggested improvement while a
direct Brain-guided agent finds useful next steps. Existing scoring and proposal
pipelines share neither final judgment nor an iterative investigation. Short
excerpts, one follow-up selection and strict response requirements are verified
constraints; their causal role in the exact reported 48-point run is unverified.

## Proposed decision

Replace only the primary manual combined action with a sidecar-owned investigation
following a reviewed signed adaptation of Brain's next-step playbook. Reuse
inference-only transports. The sidecar brokers permitted repository ranges and
signed guidance, with six calls maximum and a reserved final response. One final
response supplies ranked opportunities, one preferred next step and the assessment
from shared evidence. Observations are optional. Scores remain secondary and
retain existing calculation and unknown/coverage rules.

The design defines finite evidence/time/output limits, an additive workspace-local
report, freshness/privacy/publication recovery, compatibility and paired usefulness
qualification. No native model tools, shell, edits, new coding sessions, private
Brain reads or implicit implementation authority are introduced. Both implementation
and investigation handoffs require explicit user action.

If accepted, this replaces ADRs [0096](0096-readiness-workflow-suggestions.md),
[0099](0099-readiness-evidence-before-candidates.md) and
[0100](0100-score-directed-readiness-improvements.md) only for the primary manual
combined action's call sequence, evidence selection and score/proposal coupling.
Existing standalone actions/history, background analysis and recommendation lifecycle
remain. Those ADRs remain accepted until this proposed replacement is approved;
merging this document alone does not supersede them or authorize runtime changes.

## Consequences

The user gets a next-step explanation from the same investigation as its assessment,
rather than a score followed by a separately gated empty result. Investigation can
follow evidence without first proving a gap. A bounded implementation remains
less capable than an unrestricted direct agent and must disclose that limitation.

Calls, range-aware evidence identity, privacy reconciliation and output ownership
require new tests and a coordinated desktop/sidecar delivery. A signed playbook
capability is a separate publication dependency. Deterministic tests and transport
success are insufficient: paired usefulness qualification is required before
activation. Keep existing manual behavior until that evidence exists.
