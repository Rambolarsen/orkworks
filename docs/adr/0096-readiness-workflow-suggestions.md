---
type: Architecture Decision
title: Readiness assessment followed by a grounded workflow suggestion
description: Proposed combined action preserving independent score history and workflow recommendation publication.
tags: [orkworks, architecture, taskmaster, readiness]
status: draft
workflow_status: awaiting-owner-design-approval
---

# Readiness assessment followed by a grounded workflow suggestion

- Status: proposed, 2026-10-26; dependent runtime work is not approved
- Tracking: [#1091](https://github.com/Rambolarsen/orkworks/issues/1091)
- Concrete design/checks: [readiness suggestions plan](../superpowers/plans/2026-10-26-readiness-suggestions.md)

## Context

A saved 46/100 assessment did not lead to an actionable recommendation.
ADR 0095 intentionally separates scores from workflow recommendations. Preserve
those evidence/storage boundaries while connecting the user-facing workflow.
Low scores can reflect missing evidence or model limitations rather than defects.

## Proposed decision

Add explicit **Assess and suggest**. A sidecar operation saves readiness history,
then seeks one suggestion through the existing workflow assessment under the same
shared analysis lease. The combined four-call ceiling comprises existing two-call
stages; standalone scoring is unchanged. A bounded source-free operation record
supplies child linkage, visible outcomes and recovery without inference replay.

The second stage uses fresh repository evidence. A low score is not a citation and
does not waive deduplication, dismissal, active-card, privacy, current-input or
explicit Fix with AI gates. Keep both existing report formats and recommendation
provenance unchanged. Display evidence limits and model-profile qualification
without silently changing the selected model.

## Consequences and approval

Users can request a score and next step together while retaining independent
history and the established recommendation lifecycle. The combined action may
cost four provider calls and can honestly yield no proposal. It adds a bounded
progress record and lease-transfer seam.

On acceptance this supersedes ADR 0095 only for the absence of a combined action;
retain its score storage, signed rubric, history and privacy decisions by
reference. Link/mark that supersession and update the accepted spec when the owner
approves this concrete design. This proposed record changes no runtime behavior
and establishes neither model quality nor installer availability.
