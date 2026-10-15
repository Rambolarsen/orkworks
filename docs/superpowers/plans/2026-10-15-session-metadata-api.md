---
type: Implementation Plan
title: Coordinated session metadata API and provenance
description: Implement the remaining sidecar contract for Codex native attention coordination.
tags: [codex, metadata, api, concurrency]
status: draft
---

# Coordinated session metadata API and provenance

## Outcome

Complete [#788](https://github.com/Rambolarsen/orkworks/issues/788) against the
[owner-approved metadata design](../specs/2026-10-08-codex-native-attention-layer-design.md),
building on [#883](https://github.com/Rambolarsen/orkworks/pull/883).
Authenticated live-session GET/PATCH operations update exactly the permitted
scopes under the retained store's transaction boundary. Every sidecar writer
participates; attention ownership and freshness stay independent of descriptive
work, summary, and plan metadata. Native clearing and launch remain disabled;
#789 owns producer instructions/capability/handshake activation.

## Contract and boundaries

- `SessionMetadata` and `SessionInfo` gain one flattened `provenance` member of
  type `MetadataProvenance`, with optional snake-case Rust fields serialized as
  `attentionSource`, `attentionConfidence`, `attentionOrigin`,
  `attentionUpdatedAt`, `workMetadataUpdatedAt`, `summarySource`,
  `summaryConfidence`, and `summaryObservedAt`. Existing record-wide source
  fields remain the work-metadata authority. No companion file is introduced.
- `ObservedStatusSnapshotMetadata` gains optional `attention_origin`. Legacy
  snapshots normalize codex_hook to agent with legacy_unknown origin.
- Storage owns legacy hydration from the file modification time without a read
  rewrite. Absent/unreadable legacy age remains unknown and preserves authority.
  Explicit unknown-source attention clears must not rehydrate as old authority.
- `WriteScope { attention: bool, work: bool }` identifies accepted scopes;
  attention writes, including identical tuples, invalidate durable ownership;
  work/summary/plan-only patches do not. A versioned scoped mutation preserves
  current metadata revision checking and atomic replacement.
- The API envelope is `{metadataRevision, patch:{attentionState?,agentMetadata?}}`.
  GET returns the canonical record with metadataRevision. PATCH accepts only
  the approved complete tuples/scopes, returns the updated record/revision,
  and maps validation to 400, revision/source/user-plan conflicts to 409,
  capability failures to 401, and storage/projection failures to explicit 500.
  `validateOnly=true` checks auth/revision/schema without a write or authority
  mutation and returns the approved validated/revision response.
- Use existing validators/path resolution and the live report token. No token,
  caller-selected provenance, arbitrary PlanReference, lifecycle, or identity
  fields are accepted. Keep the route body bound explicit.
- A retained-store scoped transaction is the only new mutation boundary.
  All internal read/check/mutate/replace paths migrate; nested metadata calls
  must be moved outside a locked transaction or use its non-reentrant context.
  Atomic publication failure cannot update the in-memory session mirror.
- Attention and work source/age checks are independent. Live native-owned and
  validated Codex permission waits retain runtime protection, while Peon may
  update descriptive metadata. Accepted input can supersede the matching Codex
  permission/native wait, never a user override or unrelated elicitation.
- Projection errors must be explicit; no malformed record becomes missing or
  permission to return cached attention as current. The detail SourceBadge uses
  attention-specific provenance. Legacy and unsupported launches gain no native
  clearing authority or protocol marker.

## Deliverables and routing

1. Root establishes the shared additive provenance/snapshot/WriteScope types and
   mechanical constructor defaults, with failing-then-passing serialization and
   ownership tests. Commit this common interface before worker branches fork.
2. Runtime worker owns `metadata.rs`, `metadata/provenance.rs`,
   `metadata/session_transactions.rs`, `runtime/observed_status.rs`,
   `runtime/codex_approval_application.rs`, `runtime/terminal_runtime.rs`, and
   `runtime/peon_runtime.rs`. Implement scoped transaction/ownership and producer
   arbitration, split freshness, legacy hydration, native final checks, and input
   protection. Tests must cover 15 seconds vs greater, work-only writes, identical
   attention writes, native/hook protected waits, and I/O failures.
3. API worker owns new `metadata/agent_patch.rs` and
   `http/session_metadata_handlers.rs`, with tests in those modules. It consumes
   fixed types/interfaces in this plan and implements strict parsing, auth,
   validation-only, provenance assignment, source conflicts, plan selection
   protection, and versioned writes. Root wires routes/modules after integration.
4. Projection worker owns `session_types.rs`, `session_view.rs`,
   `session_projection.rs`, `http/session_handlers.rs`, desktop renderer API types,
   SessionDetailPanel and relevant tests. Implement scoped provenance projection,
   atomic attention tuple replacement including explicit clears, explicit read
   errors, legacy/final snapshot projection, and attention SourceBadge.
5. Root owns `session_application.rs`, `main.rs`, route wiring, lifecycle/caller
   migration not allocated above, docs/spec status synchronization and integration.
   Root integrates worker commits and validates all in-product writer paths.

One production round, at most three lower-cost worker agents plus the root.
Workers use isolated owned sibling worktrees and do not edit outside their
allocated files or revert another writer. Each branch starts from the committed
shared interface so worker deliverables do not depend on uncommitted outputs.
If a required interface is missing, report it to root before inventing a rival
contract. Root alone reconciles and integrates. After integration, use separate
read-only correctness and completeness review contexts on the actual merged diff;
record exact current code SHA and dispositions. No other coding harness is run.

## Verification and recovery

- Red/green tests for complete/null/omitted tuples and all status/input rules;
  unknown/protected fields, spoofed source, empty groups, user plan conflict,
  wrong/dead/foreign token, stale revision, validation-only no-write behavior,
  attention clear releasing Peon, independent summary/work timestamps, and API
  writer/native-clear ordering with real storage and deterministic channels.
- Lifecycle ending/finalization/restart tests preserve attention source/origin;
  user overrides survive input. Projection read corruption returns explicit error.
- Scoped unit/route/UI tests followed by `bash scripts/verify-repo.sh`, Node 22
  and locked pnpm dependencies, docs build/check, currency hooks and current-head
  CI. Record warnings against the actual diff.
- Additive fields remain readable by old readers. No production native gate,
  capability marker, compatibility entry, transport, or producer behavior is
  enabled. Rollback reverts code; no destructive record migration is performed.

## Investigated risks and plan review

The remaining uncertainty is the breadth of existing read/modify/write paths:
metadata merges call identity merging recursively, application rollback retains
whole snapshots, and projections construct independent read-only stores. Explicit
transaction contexts and retained workspace stores must replace mutation access;
independent readers stay read-only. Runtime protection is process-local and must
never be restored by persisted provenance. Failure paths must not publish mirror
updates. The approved design already addresses external direct replacements;
those remain unsupported and cannot qualify native clearing.

Dependencies 3 (three fixed module owners), blast radius 5 (shared state and API),
state change 3 (additive schema and split authority), reversibility 3 (known code
revert with legacy reads), uncertainty 3 (bounded migration across inspected paths).
Total 17. Disposition: Ready under existing design approval and current user
instruction. Review covers #788, with #789 and #690/#763 excluded from activation.
