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
