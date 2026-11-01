---
type: Implementation Plan
title: Shared branch design and implementation documents
description: Deliver durable Git-assisted discovery and two ordered document steps for sessions sharing a task branch.
tags: [orkworks, plans, git, desktop]
status: stable
workflow_status: in_progress
---

# Shared branch design and implementation documents

## Outcome and authority

Sessions on the same repository branch display the latest design/specification
and implementation plan as two ordered steps. Commit and restart do not lose
the documents. Explicit user selections stay pinned. OrkWorks displays these
documents; approval, completion and gating remain the coding workflow's concern.

Issue: [#1230](https://github.com/Rambolarsen/orkworks/issues/1230).
Contract: [Session plan review](../../../specs/session-plan-review.md#branch-document-extension).
Decision: [Branch document association](../../adr/0102-branch-document-association.md).

The owner approved the behavior, interactive UI and concrete additive
storage/protocol design on 2026-11-01, then authorized implementation. Execute
inline in the owned `branch-plan-documents` worktree.

## Decisions and boundaries

- Rust owns branch identity, discovery, selection persistence and path validation.
  Add focused `branch_documents.rs` and `branch_documents_store.rs` modules rather
  than putting discovery into the large session application module.
- Use the canonical Git common directory plus full local branch ref as the shared
  key within the current workspace. Resolve the validated explicit task-worktree
  binding first; unavailable bindings never fall back to another root.
- Keep a versioned additive branch record with independent design/implementation
  selections, provenance, observation time and explicit-selection pin state.
  Preserve existing `plan_path` metadata and bodyless legacy endpoints.
- Discover branch-specific documents from Git objects and bounded uncommitted
  files; no fetch, shell, Git mutation, external filters or repo-wide inbox.
- Electron owns the desktop credential; renderer requests contain only session
  ID, the closed `design | implementation` discriminator and an optional opaque
  selection ID to reject actions for stale displayed content.
- Reuse the single central Review surface and existing return navigation.
  Keep document-slot selection in transient renderer state; reopening after
  restoration may default to the implementation slot, then design.

## Delivery

### Shared identity, selection and discovery

Files: new Rust branch-document modules; `session_location.rs`, `git_changes.rs`,
`plan_handoff.rs`, `metadata.rs` and focused module tests where necessary.

- [x] Add failing repository fixtures for committed design and plan discovery,
  dirty/untracked documents, inherited documents, multiple work items, two
  sessions sharing a branch, distinct repositories with equal branch names,
  branch switches and detached/unborn/unavailable repositories.
- [x] Implement the scoped ranking, classification and validation rules in the
  contract. Share the existing default-base selection policy with Changes.
  Bound scans; incomplete discovery never replaces an established selection.
- [x] Add atomic, versioned branch selection persistence using the existing
  workspace single-writer boundary. Test restart, pin precedence, corrupt
  records, missing documents and rollback with legacy session metadata intact.
- [x] Route accepted hook, metadata, terminal and explicit selections into the
  appropriate slot without changing attention or native identity. An existing
  legacy association remains session-local unless fresh evidence establishes
  branch ownership; never copy a historical reference to today's checkout.

### Authenticated projection and document access

Files: `session_application.rs`, `session_projection.rs`, `session_types.rs`,
`http/session_handlers.rs`, `main.rs`; Electron `planOpener.ts`, `main.ts`,
`preload.ts`; independent renderer contract types.

- [x] Add failing route/projection tests for both slots, shared associations,
  unavailable bound roots, deleted/symlinked documents, missing desktop token,
  legacy requests, slot validation and workspace-generation races.
- [x] Project two sanitized document summaries plus branch sharing context.
  Retain `hasOpenablePlan` as the compatibility aggregate. Discover outside
  runtime/workspace mutexes and reuse bounded per-worktree results for polling.
- [x] Add optional closed document-kind selection to content and fixed-review
  endpoints and the narrow preload bridge. Bodyless calls preserve legacy
  behavior. A read or handoff revalidates branch and path before use.
- [x] Ensure document changes refresh session rows/update-stream snapshots;
  no polling, projection or discovery operation types into a terminal.

### Two ordered document steps

Files: renderer `api.ts`, `orkworksWindow.d.ts`, `useSessionReview.ts`,
`components/SessionDetailPanel.tsx`, `components/ReviewPanel.tsx`,
`components/ApplicationShell.tsx`, `App.tsx`, `App.css`; existing shell fixtures.

- [x] Add failing UI tests for two available steps, either missing slot,
  opening each document, shared-branch sessions, switching branches, unavailable
  refresh and stale reads crossing document/session/workspace boundaries.
- [x] Replace the single plan card with Design/specification then Implementation
  plan, filenames/titles, available/missing state and expandable provenance.
  Add the slot-specific Use latest action for explicitly pinned selections.
  Selecting a step opens it in the central reader; retain return-to-Terminal and
  Workflow behavior. No approval, completed-step checkmarks or gates.
- [x] Preserve explicit independent review for the displayed document and a live
  session. Ensure exactly one handoff and no arbitrary path/text input.
- [x] Visually verify desktop/narrow layouts and shared-session behavior in an
  isolated app fixture matching the approved concept.

## Verification and handoff

- [ ] Run focused Rust and desktop regression tests, then required Rust build,
  full tests and fmt check; desktop type checks and relevant app-flow tests.
- [ ] Run `bash scripts/verify-repo.sh`, `bash scripts/doc-check.sh`,
  `pnpm --dir docs docs:check`, docs build and the worktree hook as applicable.
- [ ] Complete current-code-diff `/code-review low`, record findings/dispositions,
  open the PR and follow the repository's bounded PR lifecycle and cleanup.

## Risk and recovery review

Dependencies 3: sidecar discovery/storage and duplicated Electron/renderer
contracts ship together. Blast radius 3: shared document projection and content
routes, without lifecycle changes. State changes 3: additive versioned branch
records, preserving session metadata. Reversibility 1: older code ignores new
records; legacy associations remain available. Uncertainty 3: bounded discovery,
ranking and races require the fixture coverage above. Total: 13/25.

Disposition: **Approved; implementation and verification in progress**. Checked
coverage against #1230 and the proposed spec extension. The main risks are stale
cross-branch associations, inherited documents, incomplete scans choosing a
false latest document, unsafe worktree access and stale renderer requests; each
has an owning delivery step and regression check. No independent orchestration,
agent-workflow gating, native-hook research or automatic Git control is included.

Investigation evidence: `report_plan_path` normalizes against the workspace
root while terminal fallback resolves against launch CWD; neither currently
uses task-worktree/branch identity for a shared pair. `read_plan_content` reads
one persisted reference, and projection has only the `hasOpenablePlan` flag.
`session_location::WorktreeBinding::validate` already establishes canonical
common-directory identity. `git_changes::bases` provides the exact local-base
policy. These existing seams support the proposal without another repository
registry. The docs drift warning for root README needs no edit: this proposed
contract introduced no shipped behavior or changed onboarding claim. The
implementation updates the public sessions guide with source-only behavior.
