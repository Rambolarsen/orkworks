---
type: Implementation Plan
title: Hierarchy storage readiness and first source-codec unit
description: Reconciled storage-kernel prerequisites and one dormant Rust child-source byte validator; no format-4 store creation or producer publication is authorized.
tags: [plans, hierarchy, storage, child-sources, validation]
status: draft
workflow_status: source-codec-unit-implemented
---

# Hierarchy storage readiness and first source-codec unit

Outcome: move the accepted selected child-source byte validation into a pure,
dormant Rust module, so a later storage owner can reuse strict source types and
joins without copying documentation JavaScript into production. This first unit
creates no store, journal command, producer route, grant or eligible adapter.
Pass condition: the Rust interface accepts the independent selected worker/
reviewer golden bytes, rejects their malformed/rehashed semantic variants and
returns only unverified-source-bytes-only; existing root validation is unchanged.

Tracking: first coding unit [#1345](https://github.com/Rambolarsen/orkworks/issues/1345),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176),
[#1183](https://github.com/Rambolarsen/orkworks/issues/1183), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Reviewed base: `5e8d54339e03dd670eb29f85b9f83d6feb879b55`.
This replaces the executable instructions from
[#1337](https://github.com/Rambolarsen/orkworks/pull/1337); its original reviewed
plan is preserved in Git at `70d8e2724c74831a5555c16ec2d3734f241f7e2c`.
**SQL kernel: Investigate, not ready for execution. First pure codec unit: scoped
owner execution approval recorded on 2026-11-03 below.** An existing worktree
or merged plan document is not evidence of approval or ownership of another task.

## Review findings and corrected disposition — 2026-11-03

| Finding in the #1337 plan | Evidence and disposition |
| --- | --- |
| “Complete” format-4 catalog listed only the eleven base tables plus four child source tables | The accepted root, compact-claim, artifact/debit and nine control families are also mandatory, including empty tables. A writable subset cannot define another format-4 reader/profile. Retire that DDL/admission instruction; close the complete catalog before store creation |
| Generic single-object/row/edge journal commands were described as non-source-valid, but still wrote format-4 manifests | The accepted manifest has a closed command registry and complete source-owned effects. Relabeling a partial write does not select an admissible commandKind, source policy or fresh preparation state. Remove this publisher interface; no standalone journal command is proposed here |
| Recovery required “chain continuity” on every open | Accepted retention is latest plus directly named origin/expected/checkpoint/receipt proofs, not every predecessor forever. Sparse unreferenced sequence gaps are not automatically corruption. Future reload must validate the required direct joins and header without electing a maximum or reconstructing rows |
| CoordinatorStore was cited as a mutex around a private connection | `crates/orkworksd/src/taskmaster/coordinator_store.rs:76-113` owns a directory, IDs and Mutex<()>; it is JSON/file-backed, not a SQLite connection/worker precedent. Do not derive SQL worker/cancellation or native durability from it |
| Source quota reservation “deactivation”, outdated source/control status and fixture name | Source/control retained pools keep reserved="0"; end/resume does not release immutable history. Controls and membership are now separately accepted. Use the real child-source-vectors.json path; do not invent a domain bump or new acceptance for unchanged codecs |

These are plan corrections, not a finding that an implementation exists or is
incorrect. This task inspects the merged source and plan only; it does not adopt,
stop, edit or authorize another session's storage-kernel worktree. It makes no
claim about instructions supplied in another session or uninspected working code.

## Accepted basis

Reuse these decisions without asking for them again:

- [Selected child source codecs](../specs/2026-10-31-child-review-source-preparation-design.md#owner-acceptance-and-bounded-component-handoff--2026-11-02):
  worker/reviewer v3 configurations and framing, input/report snapshots, exact
  plan/approval/reservation/allocation/attachment/source bytes and frozen identity.
- [Transition/preparation effects](../specs/2026-10-31-child-review-source-preparation-design.md#transition-ownership-and-preparation-effects--2026-11-02):
  inseparable owner effects and no historical zero-row repair, not permission to
  publish their pieces independently.
- [Selected controls](../specs/2026-11-02-child-launch-control-design.md#scoped-owner-acceptance-and-component-handoff--2026-11-02)
  and [consumer discovery](../specs/2026-11-03-child-consumer-membership-design.md#scoped-owner-acceptance-and-component-handoff--2026-11-03):
  accepted logical codecs/lookup ownership, not qualified current/native owners.
- [Independent child source bytes](../../validation/child-source-components.md),
  [control bytes](../../validation/child-control-components.md) and
  [membership relations](../../validation/child-membership-components.md):
  synthetic component evidence, not one source-valid store or original manifest.
- Existing dormant root validator in
  [root_bootstrap.rs](../../../crates/orkworksd/src/taskmaster/root_bootstrap.rs)
  is a suitable pure-byte implementation precedent: no I/O or eligibility.
  Common byte helpers may be reused; root semantics cannot be relabeled as child.

SQLite/private-worker DELETE/EXTRA remains an accepted architecture direction,
and rusqlite 0.32 with bundled/hooks is already in Cargo.toml. This unit does not
use, replace or qualify that driver. Accepted format-4 row/change domains stay
unchanged; no alternate schema, command tag, migration or physical projection is
selected by this revision.

## First executable unit

One pure crate-private module, under `crates/orkworksd/src/taskmaster/child_sources/`
with its entry in `child_sources.rs`, and one registration in taskmaster/mod.rs.
No production caller. It validates only the already selected retained-data worker/
reviewer source pair from the existing component packet; it is not a generalized
all-role configuration engine or store/source admission owner.

### Interface and inputs

Expose one crate-private validation entry:

```text
validate_unverified_child_sources_v1(&ChildSourceValidationInput)
    -> Result<ValidatedUnverifiedChildSources, ChildSourceError>
```

ChildSourceValidationInput is a Rust-only borrowed input container, not a new
serialized product/source schema. It supplies the complete canonical source bytes
already named by the selected packet:

- Exact retained run and revision-one parent InitialInputV1 context.
- Worker and reviewer bundles, each containing complete configuration, cited
  evidence-object bytes, plan, approval request/source/receipt, reservation,
  allocation, attachment and child-assignment-source bytes.
- Exact retained reviewer input snapshot, including its target result and report
  descriptor bytes, plus the packet's declared report validation context and
  repository binding digest. That context is supplied synthetic/source-validation
  data, not proof of currentness, native identity or authenticated custody.

The module accepts bytes and validates them; it never fetches files, executes a
harness, searches a repository or reads a current process/catalog. The result
exposes validated original canonical configuration/source bytes and unchanged
digests through read-only accessors, plus classification exactly
unverified-source-bytes-only. It contains no eligibility/admitted/approved flag,
active grant, commit witness, store handle or public transport envelope.

### Ownership and delivery

| Responsibility | Files and deliverable |
| --- | --- |
| Validation interface and orchestration | child_sources.rs: one pure entry, private validated result and bounded source/context joins; no storage dependencies |
| Selected child configuration/framing | child_sources/configuration.rs: worker/reviewer role, permissions, evidence/settings/model, child frames/startup and approved input/output joins |
| Original plan and event-source codecs | child_sources/records.rs: exact plan/request/approval/receipt/reservation/allocation/attachment/source reconstruction and frozen assignmentTaskVersion |
| Strict bytes and report context | Private helper code inside the module, reusing existing coordinator/root byte/hash machinery where appropriate; existing root entrypoint/errors/goldens remain unchanged. Any small shared-helper extraction stays byte-only and crate-private |
| Tests and independent fixtures | child_sources_tests.rs or a private tests module; test-only adapter reads include_str! fixture catalogs and supplies original canonical vector bytes. Fixture loading/JSON wrapper extraction must not ship as a new product parser |

Root is the sole writer in an owned worktree during implementation. No concurrent
writer may alter the same helper/root modules. The owning agent implements
directly unless the owner explicitly chooses delegated writers; code review
remains independent. Do not adopt a foreign storage-kernel branch to execute this unit.

### Coupled implementation sequence and pass checks

- [x] **Strict bytes/context and selected configuration.** Write failing tests
  first for the original worker/reviewer configuration, evidence/settings/model,
  input/output and exact child rendering/startup joins. Implement the pure
  configuration path. Accept original canonical bytes/digests; reject changed
  source context or unsupported live-access/root-role substitutions.
- [x] **Exact plan and original event-source chain.** Add the plan/request/
  approval/receipt/reservation/allocation/attachment/source validators with the
  same entry and context. Keep resulting assignmentTaskVersion frozen separately
  from mutable versions. Resolve original full refs and exact report/target/
  criteria/rubric joins; a recomputed outer hash cannot hide a changed identity.
- [x] **Boundary and dormancy close-out.** Finish the meaningful denial classes
  below, run existing root and new child tests, then all repository checks and
  current-code review. Record the production reference inventory and exact code
  head. Stop after this unit's PR terminal state; do not begin SQL as a follow-on.

All steps serve one deliverable; no separate wire-helper or ledger-only PR is
required. Tests cross the same pure entry used by future callers. Use the existing
28 Python-generated vectors and unchanged independent expected source digests,
not expected values computed by the Rust implementation under test. Standalone
records within that packet still require their complete supplied source context;
a matching digest alone is not a valid source chain.

Mandatory denial coverage, with an observed failing regression before its fix:

- Unknown/duplicate fields (including escaped duplicate keys), noncanonical order/
  spacing, invalid UTF-8/Unicode, floats/exponents/negative zero and rounded JSON
  integers. Keep storage CommitRef positive u64 TEXT separate from runSequence,
  JSON task/version and source-size encodings; test exact limits/successors.
- Child/root/profile/role replacement, live permission/delegation tools, missing/
  changed evidence bytes, settings/model/adapter mismatch and startup order drift.
- Wrong configuration/input/plan/request/approval or original reservation version;
  source/allocated worktree substitution and shared worker/reviewer allocation
  descriptors; wrong unique child session/attachment identity.
- Wrong target result/criteria/rubric/report identity, selected evidence or content
  bytes; reviewer rubric cannot override the target's required evaluator role.
- Inclusive accepted UTF-8/body/frame/startup/complete-plan/metadata wrapper bounds
  and denial when individually valid fields exceed their complete envelope.
  Preserve per-occurrence accounting; maximum source fields need not fit together.

The existing JavaScript checker has 129 child-source tests; those contracts and
its negative classes are the parity checklist, not a requirement to clone its
internal functions. No Rust control/membership/summary/receipt publisher, actual
report owner or full proof validator is included in this first unit.

## Verification for the coding PR

Follow [Rust instructions](../../../crates/orkworksd/AGENTS.md) and TDD. Before
Rust/full repository tests, use the repository Node version and frozen desktop
pnpm dependencies as those instructions require. Use host Node native ABI for
those transport tests; no premature Electron rebuild.

Run focused child/root tests first, then:

```bash
rtk proxy cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
rtk proxy cargo build --manifest-path crates/orkworksd/Cargo.toml
rtk proxy cargo test --manifest-path crates/orkworksd/Cargo.toml
rtk proxy bash scripts/verify-repo.sh
rtk proxy bash scripts/doc-check.sh
rtk proxy bash .claude/hooks/worktree-check.sh
```

Record failures or missing prerequisites explicitly. Rust changes require the
current-code `/code-review low` gate before merge, escalating only under the
repository's documented size/risk conditions. Automatic Codex/Copilot review does
not replace it. The dormancy inventory allows module declarations/definitions/
private helpers/tests only; there must be no HTTP/session/workspace/producer caller.
Do not test dormancy by running a native coding tool or creating a database.

## SQL kernel prerequisites left open

The first pure unit does not satisfy these gates. A later SQL execution plan must
name complete codecs, exact source-owned commands and physical/resource/recovery
mechanisms before advertising or creating an admitted format-4 store.

1. **Whole catalog.** At least 33 already named required table families must be
   reconciled: the eleven base storage tables; claim_proofs/mutation_lookup;
   root_drafts/root_preparations/run_states/parent_attachments/preparation_artifacts;
   the four child source tables; the nine selected child control tables;
   evaluation_artifact_streams; artifact_read_debits. This list is not a claim that
   all upstream codecs/receipts/consumers/DDL are closed. Include the attached
   parent variant and every required object/purpose/resource even for empty tables.
   No unknown/unsupported variant or historical store is normalized/adopted.
2. **Complete producers and fresh preparation.** Replace arbitrary row journals
   with exact approved source-owned commands and complete Q,S,D,P,K effects.
   Explicitly prepare fresh root/control/source/assignment owner rows and preserve
   all existing shared ledgers. Attachment is inseparable from its lineage,
   summary branch, five fresh assignment ledgers, history and disposition rules.
   The debit command keeps its exact six-row effect; no new primitive commandKind
   or namespace/source acceptance is supplied by a helper label.
3. **Complete membership and content separation.** Implement qualified forward/
   reverse physical lookups under one header, with accepted 4,096-observation /
   16 MiB cumulative discovery limits and independent command/row/source/control/
   artifact pools. Do not materialize evaluation content before the proven debit.
   Locators and the synthetic relation packet are not actual absence/index proofs.
4. **Private worker and native/resource ownership.** Select and test the real
   single worker/retained connection/lease/cancellation/disposition mechanism.
   Honor the accepted queue, command, SQL-work/deadline/busy, main-file/journal/
   scratch and stop/revocation constraints; PRAGMA defaults do not prove a hard
   journal/scratch ceiling or native durability. Existing session-store patterns
   may inform design but qualify no hierarchy owner automatically. No lock through
   SQL/fsync, replacement worker or silent VFS construction is approved here.
5. **Direct proof recovery and accounting.** Validate latest and all directly
   required origin/receipt/checkpoint/expected/predecessor joins with the accepted
   format-4 domains; do not demand a full recursively retained sequence chain.
   Missing required proof/body/index/zero row is unavailable, never repaired.
   Source/control retained pools keep reserved="0" and preserve immutable history;
   exact mutable control-byte deltas are distinct from lifetime debit/receipt work.
   Ordinary end, resume and namespace change restore no allowance or authority.

No selected source/control/membership acceptance needs repeating. Closing these
remaining mechanisms may require new scoped architecture/compatibility/native
choices; they cannot be hidden inside this pure validation unit. #1184 current
read-work ordering, actual launch/source participation, full old/new transaction
witnesses and #740's separately qualified role/adapter eligibility stay distinct.
#1183/#1176 remain open; #740 research stays closed.

## Plan review and handoff

Scope and clarity: the deletion of generic SQL journal/DDL instructions resolves
the catalog/command ambiguity without inventing another format-4 profile. The
first unit has one pure interface, accepted byte oracles, explicit denials and
no persisted state. It is useful independently as the next storage-owner codec
prerequisite. This is a material reduction of #1337's proposed first deliverable,
not an approval or claim of implementation.

First-unit risk: dependencies 3 (shared byte helpers and accepted source/report
contracts), blast radius 2 (crate-private dormant module; root behavior preserved),
state changes 1 (no I/O/persisted state), reversibility 1 (code-only local revert),
uncertainty 3 (Rust/independent-byte parity still unimplemented); sum 10. SQL kernel
risk remains dependencies 4, blast radius 4, state changes 3, reversibility Unknown,
uncertainty 4; sum incomplete. Scores are not approval thresholds.

Disposition: **Revise addressed for the first pure unit; Ready for its scoped
owner decision. SQL kernel remains Investigate.** Approval of this revised first
unit authorizes only its dormant Rust byte implementation and verification, not
SQLite/store creation, DDL, generic journal commands, migration, production wiring,
native/model probes or new configuration/authority eligibility.

One bounded read-only correctness/coverage review of this changed plan precedes
handoff: root owns edits, two Luna reviewers maximum, three active including root.
Review unchanged accepted schemas once; do not reopen them. Implementation uses
an isolated owned branch and a PR, and must not reuse another owner's worktree.


### Independent review disposition

One bounded pair of read-only Luna reviewers checked the reconciled draft against
the accepted source/control/membership/storage contracts and original #1337 plan.
The correctness reviewer reported no material findings, including the 33-family
arithmetic and command/recovery/quota caveats. The builder/coverage reviewer found
no important interface/scope/test gap for the first pure unit. These reviews are
not execution approval and supply no actual source/store/native evidence. Root
remained the sole writer; no second round or foreign-worktree change occurred.
The concrete coding handoff is #1345; its execution gate remains pending.


### Scoped first-unit execution approval — 2026-11-03

After #1346, reviewed head c1971690de9d871f2a96feb5744dee221c6057ed,
the owner answered “go ahead” to the concrete approval request for #1345's pure
Rust byte validator with no I/O, SQLite writes or production caller. This approves
the first unit's direct implementation and verification in an owned worktree.
It approves no SQL kernel, generic journal/producer transition, runtime wiring,
native/model probe or configuration/adapter eligibility. Existing source/control/
membership choices remain unchanged. Other sessions' worktrees stay separate.


### First-unit implementation and checks — 2026-11-03

The pure Rust source codec is implemented on owned branch
`feat/rust-child-sources`; [canonical evidence](../../validation/child-source-components.md#dormant-rust-source-codec--2026-11-03)
records its retained-byte boundary. Initial positive entry test failed before
implementation; the final 12 focused tests pass, including all 28 independent
vectors through full supplied context and 117 complete-packet verdicts (115 from
the unchanged 129-case assay plus two exact metadata-envelope boundaries).
Full repository verification passed: Rust formatting/build/tests, OpenCode
reporter tests, desktop type-check/native ABI/tests/build, docs build, diff,
documentation currency and worktree checks. Rust unit tests: 2,726 passed,
17 intentionally ignored; existing root validation tests pass unchanged.

One bounded pair of read-only Luna reviewers completed `/code-review medium`;
the coupled codec port exceeds the repository's 500-line threshold. Correctness
review approved; coverage feedback led to test-only vector/byte/accounting
regressions, and both reviewers cleared the final diff. The PR records the exact
reviewed code head. Root remained the sole writer. Reference inventory finds
only definitions/private orchestration and test calls for the new entry/types;
there is no production caller. An authenticated Taskmaster read returned no
active recommendation to tie off.

Documentation's user-facing-change advisory needs no onboarding edit because
this is dormant source validation. The merged `harness-workspace-signals`
worktree advisory names another owner's branch, which remains untouched.
The full SQL kernel and every native/runtime qualification gate above remain
open; this unit authorizes no next implementation task.

## Post-codec storage prerequisite review — 2026-11-03

Review base: `b9af416b3d72a397833d248700dfff771820f9b6`, after
[#1349](https://github.com/Rambolarsen/orkworks/pull/1349) completed #1345.
This review resolves the previously unexpanded family inventory against the
accepted owner contracts. It changes no source codec, quota, command kind,
compatibility policy or execution approval. The first unit is complete; its
pure-byte result supplies neither an admitted store nor an actual event owner.
Tracking remains #1183/#1176 under #738; neither parent issue closes here.

### Required family inventory

The selected contracts already name **33 distinct mandatory logical table families**:
11 base + 2 compact-proof additions + 5 root-preparation families + 4 child-source
families + 9 child-control families + 1 artifact stream + 1 read debit.
This expands the previously listed 33 families; it does not prove that unclosed
upstream owners need no further family or variant. Any required addition needs
explicit catalog/version compatibility review, not an implicit format-4 table
addition. Executable DDL and the entire catalog remain unqualified. Each reader must support the whole reconciled catalog even when a family
is empty. Physical indexes are not additional logical table families/manifest
changes, and these names do not authorize a generic row writer.

The following keys are logical identities; use the owning codec's exact canonical
representation. In particular, mutation_lookup's physical key is 64 ASCII bytes,
while its manifest key is the digest JSON string. Storage Counter remains checked
u64 TEXT; source JSON revisions/sizes retain their own encodings. Apply accepted
format-4 header/manifest/row domains and the later row variants, rather than
copying the historical base header/manifest schemaVersion 1 or converting
format-3 history. Source and owning row-wrapper schemaVersion values remain
their accepted values; format 4 does not rewrite them.

Base rows originate in the
[storage representation](../specs/2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation),
with later [compact proof/slot/lookup](../specs/2026-10-25-hierarchy-compact-bindings-design.md#three-closed-claimed-mutation-metadata-rows),
[reserved/finalization slots](../specs/2026-10-25-hierarchy-slot-finalization-design.md)
and [candidate command](../specs/2026-10-25-hierarchy-candidate-command-manifests-design.md#exact-state-lookup-and-index-contract)
variants. The accepted
[membership recipes](../specs/2026-11-03-child-consumer-membership-design.md#existing-subject-proof-and-control-families)
resolve their retained source/proof references without changing those codecs.

| Family | Exact logical key | Owning row or variant |
| --- | --- | --- |
| store_header | singleton 1 | Format-4 workspace/epoch/latest control; validate latest's exact proof separately |
| objects | ObjectKey | Closed source kind/identity, complete canonical body, source digest and lastCommit |
| payloads | AssignmentIdentity, resultRevision, declarationId | Original result/declaration-bound bytes; raw size/digest, separate occurrence accounting |
| heads | HeadKey | Exact report, result or evaluation stream; retained target ObjectRef, no maximum election |
| dependencies | consumer ObjectRef, target ObjectRef, purpose | Exact typed edge and owning proof; complete lookup in both directions |
| summary_state | AssignmentIdentity | Accepted row/checkpoint/admission variants; no initialization from missing history |
| pending_material | AssignmentIdentity, assignedRevision | Both complete before/after materials and direct origin proof |
| receipt_slots | AssignmentIdentity, slotId | Distinct reserved, consumed claimed-mutation and internal-finalization codecs; no fabricated claim |
| candidate_work | candidate ObjectRef | Conservative rehash reservation, consumed passes, terminal state and outcome slot |
| quota_usage | exact owner, resource | Closed assignment/run/workspace resource scope; checked used/reserved and owning proof |
| commit_proof | positive commitSequence | Full format-4 manifest/digest; header/new-proof control equations remain separate |
| claim_proofs | ClaimIdentity | ClaimProofV2, its proof digest and original commit; original source/action joins |
| mutation_lookup | RetryKeyDigestV1 | Compact claim reference and subject slot; reconstruct full original retry key |

The five root families are owned by
[root admission](../specs/2026-10-25-root-preparation-producer-design.md#storage-membership-and-atomic-effects)
and [initial effects](../specs/2026-10-28-root-preparation-effects-design.md#closed-initial-registry-additions).
The selected control package adds the explicit
[attached parent variant](../specs/2026-11-02-child-launch-control-design.md#shared-types-and-bounds)
to parent_attachments; this is not another table and does not mutate the immutable
initial run-state seed into a current run control.

| Family | Exact logical key | Owning row or variant |
| --- | --- | --- |
| root_drafts | workspaceId | One pending RootPreparationDraftV1 and lastCommit; consumed by exact admission |
| root_preparations | workspaceId, runId | Original bootstrap/run/approval/receipt source and direct owning links |
| run_states | workspaceId, runId | Immutable InitialInputV1 and initial assessing/null-runtime state |
| parent_attachments | parentSessionId | Reserved or selected attached ParentAttachmentV1; attached generation resolves immutable runtime history |
| preparation_artifacts | workspaceId, runId, referenceId | Exact cited Base64/raw-size/digest occurrence; equal bytes under distinct references stay separately charged |

The four child source families use the accepted
[source registry](../specs/2026-10-31-child-review-source-preparation-design.md#closed-format-4-source-registry-and-commitment).
Each complete metadata key/value wrapper is at most 16 KiB, independently of
nested source/command limits. Frozen assignmentTaskVersion and original approval/
reservation/allocation/attachment identities remain distinct from live controls.

| Family | Exact logical key | Owning row or variant |
| --- | --- | --- |
| child_plan_approvals | plan: PlanRef, approvalId | Original approval source, digest, receipt and lastCommit |
| child_reservations | TaskKey | Immutable ChildReservationSourceV1 and lastCommit |
| child_allocations | ReservationRef | Immutable ChildAllocationSourceV1 and lastCommit |
| child_attachments | childSessionId | Immutable ChildAttachmentSourceV1 and lastCommit; exact unique reserved identity |

The nine control families use the accepted
[control catalog](../specs/2026-11-02-child-launch-control-design.md#closed-storage-catalog-and-finite-accounting).
Its complete wrapper ceilings and immutable/mutable distinctions apply unchanged;
all nine contribute to the separate run/workspace control-retained resource.

| Family | Exact logical key | Owning row or variant |
| --- | --- | --- |
| child_control_inputs | PreparedInputRef without inputDigest | PreparedChildInputV1, original producer RuntimeRef and lastCommit; 64 KiB wrapper |
| child_runtime_attachments | RuntimeKey | Immutable parent/child RuntimeAttachmentV1, digest and lastCommit; 4 KiB wrapper |
| child_run_controls | workspaceId, runId | Mutable ChildRunControlV1 and lastCommit; 1 MiB wrapper |
| child_plan_controls | PlanRef | Mutable ChildPlanControlV1 and lastCommit; 16 KiB wrapper |
| child_task_controls | TaskKey | Mutable ChildTaskControlV1 and lastCommit; 16 KiB wrapper |
| child_launch_controls | TaskKey | Mutable ChildLaunchControlV1 and lastCommit; 32 KiB wrapper |
| child_session_states | TaskKey | Mutable ChildSessionStateV1 and lastCommit; 16 KiB wrapper |
| child_coordination_receipts | TaskKey | Original request/requestDigest/CoordinationReceiptV1 and lastCommit; 16 KiB wrapper |
| child_recovery_decisions | task: TaskKey, recoveryId | Immutable RecoveryDecisionV1, digest, receipt and lastCommit; 2 MiB wrapper |

The final two families are owned by the accepted artifact producer and debit
contracts. Their source/custody/authority joins remain mandatory even though no
new evaluation-content BLOB is introduced.

| Family | Exact logical key | Owning row or variant |
| --- | --- | --- |
| evaluation_artifact_streams | producerAssignment, declarationId | [Immutable stream mapping](../specs/2026-10-30-evaluation-artifact-producer-design.md#immutable-producer-stream-mapping): artifactId, target ResultRef and lastCommit; 8 KiB complete wrapper |
| artifact_read_debits | DebitIdentityV1 | [Exact retained debit](../specs/2026-10-31-artifact-read-debit-proof-design.md#one-retained-row-and-exact-six-row-effect): body, digest and lastCommit; 16 KiB complete wrapper |

### What the inventory does and does not close

The selected object dependency registry names twelve source kinds: the six base
candidate/result/evaluation/disposition/evidence-source/report kinds, five child
plan/configuration/input/evidence/assignment-source kinds, and the evaluation
artifact kind. Resolve their exact identities/bodies through the
[closed object recipes](../specs/2026-11-03-child-consumer-membership-design.md#closed-object-dependency-registry)
and owning source definitions. Root bootstrap/run bytes are retained by the root
metadata source, not invented additional ObjectKey kinds. No generic proposal,
learning object or arbitrary event is selected by this inventory.

The six retained dependency purposes are the base review-input,
evaluation-evidence and proposal-input, the two
[artifact custody purposes](../specs/2026-10-30-evaluation-artifact-producer-design.md#custody-edges-and-registry-versioning),
and [child-source-provenance](../specs/2026-10-31-child-review-source-preparation-design.md#consumer-membership-and-retention).
The existing report/source mapping reuses evaluation-evidence. A registered
purpose does not make an unsupported producer codec eligible; in particular,
proposal-input supplies no generic proposal schema. Direct metadata/source-group
membership is additional typed retention, not another dependency purpose/table.

| Remaining prerequisite | Concrete evidence still needed before SQL execution |
| --- | --- |
| Complete catalog and command selection | Reconcile all supported row/object/purpose/resource variants into one fresh format-4 definition. Map each selected child source/control producer to an exact reviewed command and complete effects; a source record/action name alone is no ChangeManifest commandKind |
| Fresh preparation and atomic publication | Exact original-to-final source/control/summary/lineage/receipt/claim/lookup/quota changes for every selected command, including all required fresh zero ledgers. Preserve the accepted debit's four listed changes plus header/new-proof controls; no extra logical index row |
| Metadata-only physical membership | Reviewed forward/reverse physical layout under one header, complete incoming/outgoing/source-group membership and accepted cumulative discovery bounds. Prove artifact/evaluation metadata resolution does not fetch/hash/copy evaluation content before debit; a generic eager objects loader is insufficient |
| One private worker and hard resources | Concrete retained connection/OS lease/queue/latch/cancellation/disposition design and enforcement mechanism for every accepted main-file/journal/scratch/work bound. Existing rusqlite availability and PRAGMA settings do not qualify these mechanisms |
| Direct recovery and compatibility | Exact sparse directly required proof/receipt/checkpoint/expected/origin joins and complete old/new disposition. No recursive all-history retention, missing-row repair, old-store import or second format-4 subset |

The family inventory is now explicit. The whole-catalog and execution gates remain
open because logical family coverage is not source-owned command closure, a
physical index, native authority or bounded recovery evidence. Existing selected
codec/authority approvals remain valid; only genuinely new choices need a scoped
owner decision. This review does not re-review or reopen those choices.

### Next bounded task and pass condition

Next: close the selected child source/control **durable command and preparation
mapping** under #1183. Reuse the accepted
[transition owners](../specs/2026-10-31-child-review-source-preparation-design.md#transition-ownership-and-preparation-effects--2026-11-02),
[control transitions](../specs/2026-11-02-child-launch-control-design.md#transition-and-recovery-rules)
and [initial root recipes](../specs/2026-10-28-root-preparation-effects-design.md#exact-command-effects).
For each supported producer, name its command codec/tag, complete read/CAS set,
original-to-final logical effects, all fresh preparation/accounting effects and
its direct recovery witness. Explicitly list unsupported transitions rather than
admitting them through a generic writer. New command/catalog choices require
scoped acceptance before dependent complete-state fixtures or implementation.

Pass condition: every selected publication has one exact effect recipe and an
accounted command/manifest/row envelope, with any new choices accepted. A named
unresolved owner decision keeps that publication Investigate and blocks its
dependent fixtures/implementation; it is not a completed recipe. There is no
unnamed generic SQL mutation path. Keep physical metadata-only
lookups and worker/resource qualification separately visible. Do not create a
store, DDL, migration, runtime route, native/model probe or another dormant codec
as part of this mapping task.

Review disposition: **Ready as an inventory of accepted contracts; Investigate
for SQL execution.** No new architecture/protocol decision is made here. The
existing SQL risk assessment remains incomplete because reversibility and native
resource/disposition evidence remain unresolved. Verification for this docs-only
review is source-to-inventory key/variant reconciliation, 33-family uniqueness,
documentation checks/build and diff/worktree checks; it makes no runtime claim.

### Durable command mapping follow-up — 2026-11-03

[#1355](https://github.com/Rambolarsen/orkworks/issues/1355) derives the five already
accepted source command kinds into explicit producer/read/CAS/atomic effects in
[the canonical source contract](../specs/2026-10-31-child-review-source-preparation-design.md#durable-command-effect-map--2026-11-03).
It spells out five fresh assignment inserts versus ten preserved peer keys,
allocation intent/outcome phases, complete attachment/lineage preparation and
unsupported standalone control publisher boundaries. #1183 is now closed on the
live board and is not reopened; remaining qualification routes through #1176.

The current baseline includes merged dormant #1351 kernel code. Its inspected
17-family catalog, single-row hierarchy-storage-journal methods and predecessor
loop do not satisfy the 33-family inventory or the source-owned publication/
sparse direct-proof contracts. This is evidence from those merged seams, not an
assessment of another session's former authorization or whole implementation.
Neither merge nor dormancy makes those helpers complete source commands.

Mapping disposition: accepted effects are explicit; executable command-body,
actual source/ordinary-lineage authority, complete physical catalog/index and
native resource/recovery qualification remain Investigate. No dependent code,
fixture, database or new source/authority policy is selected by this update.

The next bounded stabilization item is
[#1356](https://github.com/Rambolarsen/orkworks/issues/1356): fence the incomplete
dormant profile from accepted format-4 admission/publication. It is not a request
to fill the entire catalog or activate producers. No code for that item is
implemented by this command-map documentation task.
