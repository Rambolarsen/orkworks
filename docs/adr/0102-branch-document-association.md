---
type: Architecture Decision
title: Shared branch design and implementation documents
description: Branch-scoped two-slot associations with durable selections and bounded Git discovery.
tags: [orkworks, architecture, plans, git]
status: stable
---

# Shared branch design and implementation documents

- Status: accepted; owner approved design and implementation on 2026-11-01
- Deciders: OrkWorks maintainer
- Date: 2026-11-01
- Issue: [#1230](https://github.com/Rambolarsen/orkworks/issues/1230)

## Context

Single session plan references rely on reports or conservative terminal wording.
Plans can therefore be committed yet never appear. Multiple sessions working on
one branch should respect the same design and implementation plan. A branch may
also be reused; its latest documents are the useful default without requiring
OrkWorks to infer work-item boundaries or enforce the coding workflow.

## Decision

Use two ordered branch-scoped slots, design/specification followed by
implementation plan, with independent missing states and explicit user pins.
Use canonical Git common-directory identity plus the full local branch ref
within the existing workspace owner. Persist additive versioned branch records;
preserve session metadata and bodyless APIs for compatibility and rollback.
Git branch changes and bounded uncommitted-file inspection complement existing
reports. Automatic defaults follow document recency; only user selections pin.
The complete identity, ranking, bounds, protocol and recovery contract is the
[session plan review extension](../../specs/session-plan-review.md#branch-document-extension).

Keep Rust as the sole resolver and validator, Electron as the authenticated
desktop boundary, and one reusable central Review surface. Add only a closed
document-kind selector, opaque content-bound selection ID and slot-specific pin
reset. Reject stale displayed selections and capture the target runtime before
inspection, revalidating it and the document at input admission.
Neither discovery nor display changes attention, approves work, types terminal
input, performs Git mutations or starts another session. Existing explicit
independent-review requests retain their fixed prompt and user-click authority.

This extends [ADR 0039](0039-terminal-plan-link-selection.md)'s explicit
selection and [ADR 0037](0037-hook-reported-plan-paths.md)'s reporting pipeline.
Its two-slot/latest default supersedes ADR 0037 only for the old
single-session empty-association rule; hook transport and path validation stay.
ADR 0037 remains the transport/path-validation record, with this scoped
association replacement noted there.

## Consequences

Missed reports are recoverable from durable Git evidence, and sessions on one
branch agree. The UI remains two steps rather than an inferred task hierarchy.
An old plan can temporarily accompany a new spec when a branch is reused; this
is accepted and made visible through titles. Missing/partial Git history is
honestly unavailable rather than a license to attach inherited documents.

Branch records add an atomic persistence seam and cache invalidation work.
Legacy references without branch provenance cannot safely become shared
selections; keep them session-local until fresh evidence establishes ownership.
Independent app instances keep their current isolated workspace writers.
The owner approved this concrete additive storage/protocol design after its
explanation and explicitly authorized implementation. Acceptance alone does not
establish which published installers contain the runtime.
