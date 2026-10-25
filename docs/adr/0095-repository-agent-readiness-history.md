---
type: Architecture Decision
title: Repository agent-readiness scores and local history
description: Proposed independent manual readiness scoring with a complete signed rubric and workspace-local historical snapshots.
tags: [orkworks, architecture, taskmaster, brain, readiness]
status: draft
workflow_status: proposed
---

# Repository agent-readiness scores and local history

- Status: proposed; product intent approved, concrete design awaits owner approval
- Deciders: repository owner
- Date: 2026-10-25
- Tracking: [#1063](https://github.com/Rambolarsen/orkworks/issues/1063)

## Context

The owner wants a single current agent-readiness number, individual ratings and
repeat assessments after fixes to see repository evolution. The Brain rates
fourteen concepts from zero to four without a combined score. OrkWorks' existing
manual workflow assessment deliberately finds one improvement and retains one
latest report. Its strict record schema and current-input fences cannot serve
as historical score storage without changing its lifecycle contract.

The packaged `brain-1` starter contains maturity models for three concepts and
only the workflow-assessment capability. Full readiness scoring requires a
reviewed complete rubric rather than invented definitions for missing areas.
The authoritative [proposed specification](../../specs/repository-agent-readiness.md)
defines the complete behavior and bounds.

## Decision

Propose a separate manual readiness action, strict two-call scoring protocol and
workspace-local history. Reuse the existing evidence collector, configured
Taskmaster transport and shared analysis lease. Keep existing workflow assessment
and recommendation publication independent. No readiness run executes commands
or generates recommendation mutations.

The sidecar computes an equal-weight 0–100 summary from all fourteen validated
0–4 ratings. Insufficient evidence produces an unassessed component, partial
coverage and no overall score. Confidence remains separate. Complete signed
rubric capability and descriptor are a publisher/client compatibility dependency;
private Brain content and credentials never enter runtime distribution.

Store up to 100 bounded immutable completed snapshots in a separate atomic
history file and keep transient run state separately. Historical comparisons
preserve commit, dirty state, evidence, rubric and evaluation conditions. Changed
conditions suppress numerical improvement deltas. Access narrowing redacts
source-derived scores as well as text across all affected workspace histories.
Crash recovery never replays inference and reconciles completion by run ID.

## Consequences

Users can inspect change over time without losing the existing single-improvement
assessment. Missing evidence cannot inflate a total and model/rubric changes are
visible rather than attributed to repository fixes.

This adds a publisher dependency, strict inference/API contracts, local metadata
and redaction work. Implementation requires approval of this concrete proposal,
review of the rubric publication and the plan's verification gates. Missing
readiness files are backward-compatible absence; once retained metadata exists,
forward fixes preserve redaction fences and binary downgrade is unsupported.
ADRs 0085–0087 remain accepted for the existing workflow assessment.
