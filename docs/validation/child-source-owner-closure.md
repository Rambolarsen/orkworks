---
type: Validation Evidence
title: Child-source owner and transition closure audit
description: Bounded production-source inspection of the actual owners, entry points and fences the five retained child-source transitions need before complete preparation witnesses.
tags: [orkworks, hierarchy, child-source, sources, authority, qualification]
status: draft
---

# Child-source owner and transition closure audit

Tracking: [#1183](https://github.com/Rambolarsen/orkworks/issues/1183),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176),
[#1184](https://github.com/Rambolarsen/orkworks/issues/1184); initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `f17099a2` (origin/main; the [component packet](child-source-components.md)
and [transition/preparation effects](../superpowers/specs/2026-10-31-child-review-source-preparation-design.md#transition-ownership-and-preparation-effects--2026-11-02)
are accepted). This continues Gate B's
[third and fourth checkboxes](../superpowers/plans/2026-10-31-child-review-source-qualification.md#gate-b--bounded-component-codecs-and-source-ownership)
after the delivered byte packet. It supplies no source-valid transaction, new
schema, event publication, native ordering mechanism or runtime eligibility.

## Method and absence evidence

Bounded production searches over `crates/orkworksd/src` and
`apps/desktop/electron` at the inspection base returned **no matches** for
`child_source`, `child_plan_approvals`, `child_reservations`,
`child_allocations`, `child_attachments`, `ChildAssignmentSourceV1`,
`assignmentTaskVersion`, `child_review_input`, `ChildReviewInputSnapshotV1`,
`artifact_read_debits`, `PlanExecutionGrant`; no production worktree-creation
path (`.worktree(` appears only in test modules). This is a bounded finding, not
exhaustive absence proof; harness-side or external owners were not ruled out.

## Who creates ordinary sessions today

The only ordinary-session creation path is
`create_session_workflow` (`crates/orkworksd/src/session_application.rs:6414`,
entered by `SessionApplication::create_session:3970`). It mints a `uuid` id,
resolves the selected workspace/test cwd, runs `git::detect`, and writes
`SessionMetadata` (`crates/orkworksd/src/metadata.rs:379`), where
`worktree_binding: Option<WorktreeBinding>` records a DETECTED Git worktree
(`session_application.rs:62`, `git.rs:171::worktree_root`). Detection only — no
session creation ever creates a worktree, reservation or attachment.

## The five retained transitions against actual owners

| Source-owned transition | Nearest existing owner | Missing producer / control codec |
| --- | --- | --- |
| Immutable proposal publication (configuration, cited evidence objects, optional input snapshot, plan source + provenance/review-input edges + ledger charge) | `CoordinatorStore::put_proposed`/`activate/transition/resume/recover` (`crates/orkworksd/src/taskmaster/coordinator_store.rs:136/204/262/308/397`) — persistence-only for root `PlanRevision` (`coordinator.rs:359::from_proposal`); referenced only by `coordinator_tests.rs` | Child-source plan/configuration/input publication with #745 descriptors, the new exact Electron-main request/receipt (`orkworks.child-source-plan.approve.v1`), provenance edges and `child-source-retained-bytes` charges. No worktree hold or existing store transaction corresponds to the accepted atomic requirement. |
| Exact UI approval / reapproval | Electron-main generation-bound request authority exists as a pattern: `withReadyBackendGeneration` + `assertCurrentReadyBackendGeneration` (`apps/desktop/electron/main.ts:690`, generation bound against workspace lifecycle) with approved-CAS precedents in `taskmaster/inference_approval.rs` (expected revision/digest) | No request/receipt producer or store exists for the child-source approval shape; no grant surface. `CoordinatorPlanExecutionGrant` (#742) has no production implementation (`PlanExecutionGrant`: 0 matches). |
| Successful reservation (frozen `assignmentTaskVersion`, one attempt, launch version freeze) | No reservation producer. #742's task/task-version records are specification-only (no `taskVersion`/`assignmentTaskVersion` symbol in sidecar source). Existing revision CAS precedents: `session_store.rs:177` (`revision != patch.metadata_revision`) and `metadata/agent_patch.rs` rev counters | Full reservation codec + atomic version freeze with the #742 launching transition; launch/grant reconciliation; one-attempt fencing. |
| Verified allocation (native object/path/base verification, owning disposition) | `WorkspaceIdentity::resolve/revalidate` + `WorkspaceLease::acquire` (`crates/orkworksd/src/workspace_runtime.rs:22/52/171`, advisory lock on an open file) and `git.rs::worktree_root` detection; test-only worktree creation (`workspace_runtime.rs:364-383`, `metadata/agent_patch.rs:598`) | No allocation producer: no worktree creation call exists outside tests, no verify-newly-created-plan-owned-worktree path, no interrupted/success disposition record. `git2` (0.20, `Cargo.toml:26`) supplies the creation API but nothing production calls it for this purpose. |
| Unique attachment + full source preparation | Ordinary lineage records are the `SessionMetadata` writer above; attachment is extensions of that writer, not a replaced record | No reservation/allocation joined attach, no child `AssignmentIdentity` derivation from a reservation snapshot, no immutable `child_attachments` row, no initial zero-ledger/`summary_state` initialization. |

## Underlying mutable records, runtime state and index expansion

- Root-side source codecs exist dormant and validation-only:
  `validate_unverified_root_bootstrap_v2` (`crates/orkworksd/src/taskmaster/root_bootstrap.rs:293`)
  with the accepted assignment/provenance/executable/input/state/attachment
  component shapes in `taskmaster/root_bootstrap/source.rs` and run/approval
  shapes in `taskmaster/root_bootstrap.rs`. No I/O, no writer, root-only — they
  cannot be relabeled as child codecs (accepted boundary).
- The format-4 hierarchy storage plan (`#1176`) has no production twin: bounded
  searches found no `claim_proofs`-family, `artifact_read_debits`, evaluation
  artifact, quota-usage or object/dependency-store implementation in sidecar or
  Electron source. The accepted `artifact_read_debits` historical accounting
  consumer therefore has no registration owner to join yet; its membership name
  must route to the storage-implementation owner when that lands.
- Dependency/summary index expansion, protocol sidecar config, workspace lease
  interaction, and runtime capability issuance are all unimplemented as
  enforcement paths; they exist only as fixtures/checks in validation packet
  hypotheses.

## Native authority and workspace/lease fences

`WorkspaceLease::acquire` proves workspace ownership only; it does not provide
the child-source command's native/revocation commit cutoff (that gap is #1184).
Existing serialization precedents to reuse or fence explicitly: the coordinator
store's directory lock (`lock_file` + `sync_all`), the session-store admission
worker's bounded mutex pattern (`session_store/worker.rs` admission with
`spawn_blocking`), and Electron-main's generation-bound request gate. The
accepted design bars artifact-write claim-first handling, response prechecks and
core-lock-through-fsync for these actions; none of these mechanisms are present,
so the fence design for the child-source command must be brand-new, separately
reviewed work — not silently inherited from any of the precedents.

## Disposition and handoff

No complete source prestate is claimable. Gate B's remaining checkboxes need,
in dependency order:

1. **Storage-side owner** — implement (or constitute) the format-4 store
   owner accepting object/metadata/ledger tables with exactly-accepted shapes
   (#1176 acceptance pending scoped execution plan).
2. **Proposal/approval producers** — extend the dormant coordinator pattern with
   the child-source proposal publication and the Electron-main approval
   request/receipt + grant state (one atomic/recovery boundary each, reviewable
   against `#742`'s grant lifecycle).
3. **Reservation/allocation/attachment producers** — the ordinary-session
   lineage writer gains the joined reservation snapshot, a new worktree
   allocation owner built on `git2` + `WorkspaceIdentity::revalidate`, and
   unique attachment.
4. **Consumer/ledger closures** — the `child-source-provenance` purpose with
   both directions, protected peer ledgers, zero-row initialization ordering
   (per the accepted effects section), and the `artifact_read_debits`
   membership name routed to (1).

Each numbered item is separately scoped implementation work gated behind
written review; none is authorized by this audit. Original fixtures/checks and
accepted catalogs are unchanged; nothing here re-qualified a profile.
