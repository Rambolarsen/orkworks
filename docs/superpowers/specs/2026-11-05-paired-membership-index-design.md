---
type: Design Proposal
title: Format-6 physical paired membership and index contract
description: Proposed exact metadata-only forward/reverse/direct lookup layout, full-key uniqueness, complete bounded absence and sparse direct-origin proof recovery at one H/G; scoped review only.
tags: [orkworks, hierarchy, storage, membership, sqlite, qualification]
status: draft
workflow_status: owner-review-pending
---

# Format-6 physical paired membership and index contract

Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176),
initiative [#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `c0c8a974b4d9981ceea053ea5514ebeffbc4b0ac` (merged catalog
reconciliation, [audit](../../validation/child-preparation-catalog-reconciliation.md)).

## Outcome and scope

The [catalog audit](../../validation/child-preparation-catalog-reconciliation.md)
closed the 35-family logical inventory and named one remaining design
deliverable: the *physical paired membership/index contract*. This document
supplies exactly that. It proposes the concrete physical representation of
metadata-only forward, reverse and direct lookup for the selected profile;
full-key permit, ordinary-lineage and child/session/attachment uniqueness;
atomic owning maintenance; complete bounded absence at one captured header
pair; and sparse direct-origin proof recovery.

These are **new physical persistence/compatibility choices for one scoped owner
review**. Everything else is reused below without reapproval: every accepted
logical recipe, codec, domain, quota and proof rule named in
[Accepted basis](#accepted-basis). No logical table family, manifest field,
ObjectRef kind/purpose, quota resource, proof domain, wire field or authority
bearer is added. No SQL/DDL, database, migration, runtime route, native/model
probe, fixture factory or executable plan is delivered; the production
hierarchy opener stays fenced and
[#1242](https://github.com/Rambolarsen/orkworks/issues/1242) producer
qualification stays separate.

## Accepted basis

Reuse these decisions verbatim; this document does not select for them again:

| Accepted contract | Reused substance |
| --- | --- |
| [Membership registry](2026-11-03-child-consumer-membership-design.md#scoped-owner-acceptance-and-component-handoff--2026-11-03) | Closed typed ConsumerLocationV1/ReferenceTargetV1, exhaustive per-family recipes, no new logical graph, coherent header-bound discovery with no read-time repair, and the 4,096-observation / 16 MiB cumulative discovery profile |
| [Joint preparation architecture](2026-11-05-joint-child-preparation-design.md#scoped-architecture-acceptance--2026-11-05) and [ADR 0106](../../adr/0106-joint-child-preparation.md) | Ordinary-worker attached rollback transaction broker; reserved ordinary lineage member with the exact canonical full-assignment-key projection/index and ordinary schema-2 direction; new-store-only hierarchy profile with the lineage commit-link exclusion; retained linked tombstones until qualified joint purge |
| [Authorization evidence representation](2026-11-05-child-preparation-authorization-design.md#scoped-representation-acceptance--2026-11-05) and [ADR 0107](../../adr/0107-original-child-preparation-permit.md#amendment-authorization-evidence-accepted--2026-11-05) | One immutable P-keyed child_preparation_authorizations row; full canonical PreparationPermitIdentityV1 uniqueness; four typed control snapshots with their origin links; fresh format-6 header/manifest/row domains and link exclusions; compare-and-consume permit cutoff |
| [Catalog audit](../../validation/child-preparation-catalog-reconciliation.md) | 35 logical families and exact keys; original-attach effect map; physical membership indexes are not additional logical families or extra manifest changes |
| [Storage representation](2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation) and [commit/domain rules](2026-10-24-hierarchy-storage-contract-design.md#commit-commitments-without-a-digest-cycle) | Canonical-key row envelopes with excluded storage-link paths; ChangeManifest changes/equations; no trigger/function-generated domain state or arbitrary JSON extension column; row/manifest digests; exact authenticated retries change nothing |
| [Source preparation](2026-10-31-child-review-source-preparation-design.md#fresh-attachment-and-preparation-closure) and [durable command map](2026-10-31-child-review-source-preparation-design.md#durable-command-effect-map--2026-11-03) | Original attach effects; five fresh zero assignment ledgers; ten reconciled peers; frozen assignmentTaskVersion; no post-commit index registration |
| [Control catalog and snapshots](2026-11-02-child-launch-control-design.md#closed-storage-catalog-and-finite-accounting) | Nine control families, wrapper ceilings, immutable versus mutable rows, snapshot capture rules |
| [Artifact/debit retention](2026-10-31-artifact-read-debit-proof-design.md#one-retained-row-and-exact-six-row-effect) and [custody](2026-10-30-evaluation-artifact-producer-design.md#custody-edges-and-registry-versioning) | Debit six-row effect; historical debit retention; artifact precharge/content boundary; metadata-only resolution without fetching/hashing evaluation content before a proven debit |
| [Source-publisher boundary](2026-11-05-source-publisher-execution-design.md) | Physical membership obligation list; RegistryRoot remains a separate owner-pending live-registry proposal |
| [Recovery behavior](2026-10-20-hierarchy-read-recovery-design.md) with the [storage contract's](2026-10-24-hierarchy-storage-contract-design.md#commit-commitments-without-a-digest-cycle) election rules | Unavailable-not-repaired recovery; "never elect a header from maximum sequence or recreate it from remaining rows" and the sparse proof-retention wording; no recursive all-history replay (as recorded also in the [authorization design](2026-11-05-child-preparation-authorization-design.md#atomic-commitments-and-compatibility)) |

## Decision boundary

- The three physical structures below are stored objects inside the single
  closed format-6 hierarchy catalog. They are **not logical families**: no
  ChangeManifest change entry, no new command kind, no caller-visible row, no
  consumer-registration command and no durable graph authority ever covers
  them. Every occurrence is maintained by the owning row write in the same
  transaction and is validated only by reload against retained rows.
- The [proposed RegistryRoot](2026-11-05-source-publisher-execution-design.md#proposed-registry-ordering)
  is an admission-layer live structure with its own race ordering and remains
  separately owner-pending. Nothing here adopts, extends or assumes it.
  Durable membership at captured H/G and live registry discovery are disjoint
  read sets; neither substitutes for the other.
- The test-only 17-family mechanics (`hierarchy_storage/mechanics/`) prove no
  part of this contract. In particular its reload predecessor-continuity loop
  is not the accepted sparse direct-proof retention rule; the reload contract
  below supersedes that interpretation inside the proposed profile.
- Native ordering, engine/file durability, row content visibility, and actual
  build/VFS/filesystem behavior remain unqualified; this contract defines
  mechanisms and verification recipes, not platform evidence.

Rejected alternatives: scanning all consumer families per reverse query
(observation exhaustion at store scale and not complete under the closed
recipes); expression/function indexes or JSON extension columns (forbidden);
a second catalog or an alternate format selected by row presence (catalog
closure forbids); persisting consumer groups as logical rows (the accepted
graph-table rejection); deriving absence from current-head scans, store
emptiness or decoded-row heuristics (explicitly rejected); and treating a
missing index as repairable state.

## Captured-view model (H/G and lookup authority)

One storage owner performs every lookup against one captured workspace/storage
epoch and its `store_header.latest` **H**; joint events additionally capture
the paired ordinary database epoch/history generation **G** and revalidate
both at the transaction boundary under the accepted broker. Lookup authority
comes from this captured pair plus the verified store, never from a caller
flag, timeout, claimed error or absence report.

Two preconditions make every absence/uniqueness result below authoritative:

1. **Verified correspondence.** The declared indexes carry each lookup's
   completeness. At store open, a private bounded reload proves complete
   row/index correspondence ([reload](#direct-origin-proof-recovery-and-reload)).
   Against a verified captured header, a probe is authoritative; no checksum,
   scan or emptiness observation can substitute this precondition.
2. **Closed recipes.** Only the exhaustive accepted registry produces
   occurrences. Unknown/unsupported consumers or targets, an inconsistent
   probe or budget exhaustion deny the operation; no partial result set is
   ever returned as complete ([coherent
   discovery](2026-11-03-child-consumer-membership-design.md#coherent-discovery-and-publication)).

A logical lookup occurrence remains exactly
`{consumer:ConsumerLocationV1,target:ReferenceTargetV1}` as accepted. Field
originality is inherited: object locations include their exact body-domain
digest; metadata identity is the existing full table/key resolved at the
captured header, never by latest-row guessing; objects and dependencies are
not admitted as metadata locators. `C(x)` below is the owning contracts'
canonical strict UTF-8 JSON encoding
([authorization design](2026-11-05-child-preparation-authorization-design.md#canonical-identity-and-evidence-body)).

## Physical representation and closed dispatch

### The three lookup directions

| Direction | Basis | Representation |
| --- | --- | --- |
| Forward | by complete ConsumerLocationV1 | Exact-key probe on the owning family's canonical primary key decodes the retained row once; occurrences are derived in memory from the closed recipes. Every derived element that itself needs another record probes that record's exact key under the same budget |
| Reverse | by complete ReferenceTargetV1 | Closed dispatch (below) onto the declared reverse structures; a complete-target probe returns every retaining consumer; absence and completeness follow from verified correspondence plus the closed recipes |
| Direct | a row's owning origin/proof | The row's own `last_commit` link plus the exact-key probes defined in [Direct-origin proof recovery](#direct-origin-proof-recovery-and-reload); no chain continuity is required |

A returned occurrence's owning row or verified metadata projection is read
and validated per the accepted budget units; a returned row alone proves only
that occurrence.

### Closed target dispatch

Reverse lookups dispatch by complete target kind, exactly. A consumer's
reference instance that already has an accepted dedicated storage relation
(its own `dependencies` row for one of the six accepted dependency purposes)
is served by that relation and is **not** duplicated in the reverse
projection; every other closed occurrence of a metadata/object consumer is
projected:

| ReferenceTargetV1 kind | Physical route | Justification |
| --- | --- | --- |
| `{kind:"object"}` | The exact `dependencies` covering target index below for the six accepted dependency purposes, plus the uniform reverse projection for every other closed occurrence naming the object (debit intent sources/artifacts/evaluation attachments, authorization attachment/producerSource, heads stream targets, payloads' retained-result resolution, candidate object references, direct source-group object bindings) | `dependencies` purposes are a closed set; recipes beyond them cannot become new dependency purposes, so their occurrences project |
| `{kind:"metadata"}` | Declared uniform reverse projection for every remaining closed occurrence | Not derivable from other families' key shapes; embedded list entries require per-occurrence storage |
| `{kind:"control-origin"}` | Declared uniform reverse projection, including rowDigest/origin in the target bytes | History-safe: historical snapshot consumers survive later mutable control changes |
| `{kind:"commit-proof"}` | **Unavailable in this contract.** No closed recipe of the selected paired-attach profile performs reverse-by-proof — the [audit effect map](../../validation/child-preparation-catalog-reconciliation.md#complete-paired-effect-obligations) and the [closed registry](2026-11-03-child-consumer-membership-design.md#closed-object-dependency-registry) contain no incoming-by-proof lookup, and no purge path is selected here. A future purge-class producer that must prove a proof unreferenced owns that lookup in its own scoped command contract | The candidate-command manifest design's default empty proof-removal set remains *as proposed*; this contract selects no removal path and invents none |

A reverse-by-object result returns one entry per exact distinct occurrence:
one per `dependencies` row (each with its stored purpose) and one per
projection row. No de-duplication across the two structures occurs; forward
and reverse lookups return the same derived membership set per the accepted
coherence rule. Unknown kinds, metadata locators naming object-family rows,
invented kinds, and missing/extra/duplicated inconsistent probe results deny
the operation before any effect.

### Uniform reverse projection (new scoped choice)

One declared physical table stores every projected occurrence and nothing
else:

```text
CREATE TABLE membership_reverse (
  target BLOB NOT NULL,
  consumer BLOB NOT NULL,
  occurrence INTEGER NOT NULL,
  PRIMARY KEY (target, consumer, occurrence)
) WITHOUT ROWID;
```

- `target` is the exact canonical `C(ReferenceTargetV1)` bytes for the target
  kinds the dispatch assigns to the projection (metadata, control-origin, and
  non-dependency object occurrences; the kind tag travels inside the encoding,
  and the control-origin closure includes the snapshot rowDigest and origin
  columns). `consumer` is the exact canonical `C(ConsumerLocationV1)` bytes.
  Byte equality only; no collation, hash-only identity, delimiter-joined
  surrogate or field-path query is supported.
- `occurrence` is the 1-based rank of that exact `{target, consumer}` pair
  among the consumer's derived occurrences under one canonical enumeration
  order, defined as: (1) the consumer family's accepted recipe entry order in
  the [membership registry](2026-11-03-child-consumer-membership-design.md)
  (a fixed textual order, fixed by that document's lists); (2) within one
  recipe component, the owning row's canonical field/array order (arrays keep
  their accepted meaningful order; unique sets are the codec's canonical sort);
  (3) ascending canonical target bytes within one enumerated source position.
  Every derived occurrence holds a distinct enumeration position, so the rank
  is a deterministic function of the owning row bytes alone and per-pair
  duplicate rows receive distinct ascending ranks. Nothing collapses: as
  accepted, duplicate tuples never discount charges, and every original
  field, copy and physical-index occurrence is retained and charged.
- The table is created empty with each fresh store; it holds no row for an
  unretained consumer or an unsupported target. Row absence for a complete
  consumer proves a lookup result only together with verified correspondence,
  never by itself.

### Declared unique and secondary structures (new scoped choice)

The declared physical object set of a fresh format-6 store is exactly:

- the 35 logical family tables in their captured shapes (uniform row shape,
  the `store_header` and `commit_proof` singletons, and the per-family
  specializations below — `dependencies` with its three-part primary key);
- the declared indexes: `dependency_target`, the two declared UNIQUE indexes
  below (each including its engine-internal backing structure), and no other
  index, view, trigger or table object;
- the physical `membership_reverse` table; and no other object.

```text
CREATE TABLE dependencies (
  consumer BLOB NOT NULL, target BLOB NOT NULL, purpose TEXT NOT NULL,
  last_commit BLOB NOT NULL, sequence INTEGER NOT NULL,
  PRIMARY KEY (consumer, target, purpose)
) WITHOUT ROWID;
CREATE INDEX dependency_target ON dependencies (target, consumer, purpose);

ALTER TABLE child_preparation_authorizations ADD COLUMN permit BLOB NOT NULL;
CREATE UNIQUE INDEX preparation_permit
  ON child_preparation_authorizations (permit);

ALTER TABLE child_attachments ADD COLUMN attachment BLOB NOT NULL;
CREATE UNIQUE INDEX child_attachment_identity
  ON child_attachments (attachment);
```

- Row tables not listed otherwise are exactly the inherited uniform shape: a
  WITHOUT ROWID table with `key BLOB PRIMARY KEY, value BLOB NOT NULL,
  last_commit BLOB NOT NULL, sequence INTEGER NOT NULL`. `key` is the family's
  complete canonical logical key with byte equality (notably `mutation_lookup`
  keeps its inherited 64-byte ASCII RetryKeyDigest form while its manifest key
  remains the digest JSON string, per the accepted distinction). `value`
  carries the family's canonical wrapper; `last_commit` stores the committed
  CommitRef; `sequence` stores the denormalized commit sequence — the existing
  uniform-layout pattern — which also serves direct proof resolution below.
- `child_preparation_authorizations`: `value` carries the full canonical
  `{body: OriginalPreparationAuthorizationV1, digest: Digest}` wrapper;
  `permit` stores `C(PreparationPermitIdentityV1)` exactly as embedded in the
  committed body — never recomputed from value bytes, never independently
  editable. Every write and reload re-derives the column from the retained
  body fields; a changed or foreign column is fenced, so the physical column
  cannot diverge from its body without being unavailable.
- `child_attachments`: `value` carries the accepted full wrapper including
  `attachmentId`; the `attachment` column stores the canonical
  `C({childSessionId, attachmentId})` pair and is UNIQUE across all retained
  rows. Its added value is the canonical pair's exact-key probe route and the
  frozen-binding mirror; equality with the reservation's frozen binding stays
  a validated recipe join, not something the column uniqueness alone
  enforces.
- Ordinary schema-2 keeps the joint architecture's accepted row: one canonical
  full-assignment-key projection with uniqueness **including tombstoned
  rows**, derived transactionally from the reserved lineage member, never a
  digest-only index. Its concrete DDL remains a separately approved
  maintenance/compatibility item; what this contract fixes is the
  paired-capture correspondence rule only.

All declared indexes exist only for lookups the closed recipes name; nothing
invents a route. Within one engine transaction, SQLite's own index/table
consistency makes a partial index write impossible — an engine property that
must still be qualified by the native/storage qualification plan before any
production admission; no part of this document claims that qualification.

## Family projection recipes

`membership_reverse` rows are exactly the occurrences the exhaustive accepted
registry derives — not a separate decision layer. For every retained consumer
row, the closed recipes' non-link reference instances that are not already
stored as `dependencies` rows each produce one projection row for that
consumer, including:

- every object consumer's direct source-group resolution named by the
  registry (root-preparation/runtime/plan/task/approval/reservation/
  allocation/attachment reference bindings of child-source-plan,
  child-source-configuration, child-review-input and child-assignment-source
  objects);
- every metadata family's complete non-link reference recipe expanded per
  entry: each InputRecordV1 entry, each ordered planReference, each nonnull
  blocker TaskKey, each embedded control snapshot's typed references, each
  receipt binding's claim/result/candidate/slot join, each claim_proofs local
  identity/precondition expansion, each debit intent join (its
  producerSource/artifact/evaluation/object references), each quota owner
  mapping, and each authorization's attachment/producerSource/group joins;
- every historical consumer a retained row still protects — never just
  current heads or the selected plan.

Controlling rules:

- A family marked read/no-insertion in the
  [audit](../../validation/child-preparation-catalog-reconciliation.md) still
  derives its recipe occurrences whenever its rows exist, and still
  participates in every reverse lookup; "no insertion" never means "assume
  globally absent".
- Direct-origin resolution (a row's own lastCommit, checkpoint.applied,
  pending origin, receipt.commit and the like) is recipe-defined probe work,
  not projection state: the projection never stores pure link targets, and
  the dispatch's reverse routes never need them.
- A row's projection set is derived only from that row's own accepted codec
  and the closed recipes; no caller-supplied affected list and no field-path
  query exists. An unknown consumer type blocks the affected operation instead
  of being silently dropped.
- The map below is the per-family closure. It adds lookup mechanics only; it
  does not restate the audit's effect obligations.

| Consumer family | Projection occurrences | Direct-resolution-only references (no projection row) |
| --- | --- | --- |
| Non-projecting group: store_header, commit_proof, dependencies rows, root_drafts | none. store_header and commit_proof are control/adjacency structures, not retained-child consumers; dependencies rows are the object-target adjacency themselves; root_drafts is explicitly outside the retained locators per the accepted membership contract | store_header's `latest` reference and commit_proof's manifest and nonnull claim references resolve by direct recipe probes |
| Projecting metadata/source-group group (14): objects, payloads, heads, summary_state, pending_material, receipt_slots, candidate_work, quota_usage, claim_proofs, mutation_lookup, root_preparations, run_states, parent_attachments, preparation_artifacts | every non-link accepted recipe reference instance, with embedded entity joins as separate rows — for objects this is each registry-named direct source-group metadata occurrence; dependency-purpose object edges stay only in `dependencies` | each reference's owning metadata family probes by exact full key; origin/checkpoint/expected links resolve by their own accepted structures |
| Source/control/debit group (15): child_plan_approvals, child_reservations, child_allocations, child_attachments, child_control_inputs, child_runtime_attachments, child_run_controls, child_plan_controls, child_task_controls, child_launch_controls, child_session_states, child_coordination_receipts, child_recovery_decisions, evaluation_artifact_streams, artifact_read_debits | each recipe join as above, including each embedded snapshot's typed control-origin references and each debit's object/evaluation joins | same |
| ordinary_child_lineage | body reference instances resolve exactly as its accepted recipe says; its preparationCommit target is an excluded commit link (direct-resolution only) | its own full-assignment-key PK probe; cross-store pairing via the ordinary schema-2 projection rule |
| child_preparation_authorizations | every embedded snapshot's typed joins; original request/target/group references retained by the permit body's declared recipe; origin-CommitRef-bound control-origin references | its own PK probe and the permit unique index |

Object-consumer recipes resolve endpoints through each accepted source
command's exact identity/body join before comparing; the projection stores the
occurrence, the owning `dependencies` row keeps its own closed-purpose
storage, and both dispatch routes keep the accepted both-directions rule.
The arithmetic is 14 metadata/source-group + 15 source/control/debit +
2 ordinary-lineage/authorization + 4 non-projecting (store_header,
commit_proof, dependencies, root_drafts) = 35 families, matching the
[audit](../../validation/child-preparation-catalog-reconciliation.md) exactly.
Subject expansions (SubjectSource(A), MaterialRefs(M)) follow the accepted
membership recipes as usual and add no new target kinds. The projection is
physical lookup support only; it carries no logical authority semantics of
its own.

## Uniqueness and paired identity at one H/G

All canonical bytes are the stored wrappers themselves; hashes never stand in
for them. Full-key uniqueness is enforced at physical primary-key level;
the two declared UNIQUE indexes add named secondary properties on top.

| Identity | Enforcing mechanism | Read scope |
| --- | --- | --- |
| child_preparation_authorizations full canonical P | family primary key | one retained row per exact identity; no overwrite or collision chaining |
| child_preparation_authorizations permit C(PreparationPermitIdentityV1) | declared UNIQUE permit column/index | one permit per retained row; a permit can never name two P values. Key/permit/body disagreement, a changed same-key bytes or a foreign permit entry fences the store |
| ordinary_child_lineage full canonical P (hierarchy) | family primary key | one retained lineage row per identity |
| Ordinary schema-2 lineage projection for P | ordinary UNIQUE canonical full-assignment-key projection, including tombstones | one projection entry per P across the ordinary store, including deleted rows retained by the [accepted tombstone rule](2026-11-05-joint-child-preparation-design.md#permission-ordinary-lifecycle-and-visibility) |
| child_attachments childSessionId | family primary key | unique across all runs and tombstones |
| child_attachments (childSessionId, attachmentId) | declared UNIQUE column | the canonical pair's exact-key route; the pair uniqueness it adds on top of the row's PK is the unused-pair denial, and equality with the reservation's frozen binding stays a validated recipe join, not something the column uniqueness alone enforces |
| child_reservations TaskKey | family primary key | one immutable reservation per task key; no second reservation |
| child_plan_approvals (plan, approvalId) | family primary key | one retained approval instance per approval identity |
| child_control_inputs PreparedInputRef | family primary key | input row uniqueness per producer ref |
| child_run_controls, child_plan_controls, child_task_controls, child_launch_controls, child_session_states, child_coordination_receipts, child_recovery_decisions | family primary keys over the accepted keys | one exact control/recovery row per accepted key |
| evaluation_artifact_streams (producerAssignment, declarationId) | family primary key | one immutable stream mapping row per exact producer/declaration pair |
| receipt_slots, candidate_work, quota_usage, objects, payloads | family primary keys per owning contracts | exact per-family uniqueness |

**Paired ordinary/hierarchy identity rule.** At one captured pair (H, G), and
only under the qualified joint broker mechanism, the hierarchy
`ordinary_child_lineage` row for P and the ordinary schema-2 projection entry
for P must reduce to one identity:

- both hold the exact full canonical P bytes (byte equality, not digest or
  surrogate equality);
- the ordinary member's `preparationCommit` is exactly the hierarchy row's
  excluded commit link C; the ordinary creating identity (id = P.childSessionId,
  exact created_at, epoch/G capture) matches the retained body's committed
  ordinary shape;
- the member's `assignmentIdentity`, `attachmentId` and `assignmentSourceDigest`
  equal the hierarchy row's committed body and the attachment/source rows'
  retained digests;
- tombstoned hidden ordinary rows participate unchanged: an unreconciled,
  missing, extra tombstoned or semantically divergent entry is unavailable.

Two stores each agreeing only with their own side does not satisfy the
paired-identity rule; a paired capture confirms both sides jointly at one
(H, G). Supporting hierarchy-only facts — the lineage row's preparationCommit,
identity bytes, ordinary binding digests and retained source/attachment proofs —
do not substitute cross-store verification, and no lookup recipe here
weakens that boundary.

## Complete bounded absence and probe accounting

The accepted read-work budget governs all probes here, with unchanged units
(returned occurrence/owning record/per-page/miss observation classes and the
16 MiB cumulative cap). Absence never derives from a content scan, store
emptiness or a digest; every absence recipe resolves its owning structure
exactly. The set below is the proposed physical absence contract for the
closed paired-attach profile.

The original preparation's absence/uniqueness read set at captured (H, G):

| Probe | Expected | Observation cost |
| --- | --- | --- |
| hierarchy child_preparation_authorizations by PK P | absent | 1 (miss) |
| hierarchy ordinary_child_lineage by PK P | absent | 1 (miss) |
| hierarchy child_attachments by PK childSessionId | absent | 1 (miss) |
| hierarchy child_attachments identity via the declared UNIQUE column (childSessionId, attachmentId) | absent | 1 (miss) |
| hierarchy child_reservations by PK TaskKey | present exactly once (revalidated with the frozen assignmentTaskVersion/read-set as accepted) | 1 |
| hierarchy child_launch_controls by PK TaskKey | present, allocated phase, attachment/spawn null | 1 |
| hierarchy child_task_controls by PK TaskKey | present, launching, accepted live-version reads | 1 |
| ordinary schema-2 lineage projection by exact canonical P | absent | 1 (miss) |
| ordinary creating-record session identity by exact childSessionId, across retained and tombstoned ordinary records | absent | 1 (miss) |
| "no previous preparation" claims from store emptiness, digest checks, or a head scan | not permitted to stand in for any probe above | n/a |

Six of the nine are absence/miss probes; the three hierarchy prestate probes
(5–7) revalidate presence with their owning rows. The two ordinary probes are
the two exactly pinned key shapes at correspondence-rule level: the schema-2
full-assignment-key projection (its concrete DDL remains a separately
approved maintenance item) and the ordinary creating-record session identity,
whose uniqueness the accepted ordinary schema already guarantees per record.
Both absence claims concern today's un-purged retained state: after a future
qualified joint purge, an absent probe is no longer evidence against removed
preparations — the accepted purge then owns that history.

The exact-key miss probes are *complete absence proofs* because identity
constructions cannot collide: each accepted identity reconstructs its exact
key from no other identity, so an absent probe on the exact canonical full
key denies precisely when that exact key could not already name a previous
preparation. This property additionally holds because the indexes themselves
were verified at captured H — no completeness shortcut exists.

Two boundary facts follow:

- Observations exhaust together with the other accepted discovery expansions:
  running the paired probes only when the accepted budget can still prove
  completion; reaching the limit without proving completion denies.
- Cross-family reverse membership for metadata/control-origin targets goes
  through the declared projection with per-unit accounting; it never
  degrades into a digest-only, heuristic or current-head check.

Arithmetic over representative encoding shapes appears in
[Verification](#verification). As there: bounds describe canonical encodings,
not admitted source states, and never assert that all independently maximal
input shapes fit together.

## Atomic owning maintenance

All three declared structures are maintained by the owning logical row write
inside that command's single engine transaction under the accepted broker.
No other sender exists, and no reader ever writes.

| Owning event | Obligated maintenance |
| --- | --- |
| Fresh store initialization | `membership_reverse` created empty; all declared indexes exist from creation; no family row produces projection rows before it is written by an owning command |
| Insert of a logical row | insert exactly that row's recipe-derived projection occurrences in the same transaction; declared unique column values inserted with the row |
| Replacement of a mutable row (control/value transfer) | rederive the row's full derived occurrence set under the closed recipes; compare against the replacement's *old* value's derived set; update only actual differences in the same transaction |
| Row replacement with an unchanged derived set | a real logical row change (accepted manifest entry), with zero projection-row differences; identical derived sets never rewrite projection rows, and nothing treats an unchanged derived set as a reason to skip the accepted logical row change |
| Deletion of a logical row | delete that row's projection occurrences in the same transaction. This path exists only for accepted purge-class producers under their own scoped, qualified commands; no deletion route is selected by this contract |
| Exact authenticated retry | the prebuilt immutable plan is compared against retained bytes; identical derived sets produce no new projection rows and no extra charges ([accepted retry rule](2026-10-24-hierarchy-storage-contract-design.md#commit-commitments-without-a-digest-cycle)) |
| Linked joint event | hierarchy-side maintenance happens in the same attached engine transaction as the ordinary schema-2 projection update; rollback/unknown outcomes leave no half-paired projection state and no post-commit registration |
| Retained tombstoned consumer | The hierarchy side has no tombstone state of its own: every retained hierarchy family row keeps its projection occurrences and continues to participate in reverse lookups until a row leaves the store only through an accepted purge-class producer command (separately gated). Ordinary tombstoned rows retain their schema-2 projection entries and union into the paired rules unchanged |
| Rollback or unknown outcome | engine-managed inside the transaction: no partial projection/owning-row difference can survive as committed state; unsupported or failed dispositions keep the store's own captured state (unavailable-not-repaired rule) |

Any change of the declared structures outside the sealed owning transaction,
or any maintenance attempted via read paths or generic journal/API machinery,
is unavailable. Maintenance never creates manifest entries; it rides along
with accepted logical row changes and is validated by reload correspondence.

## Direct-origin proof recovery and reload

The retained profile validates every retained row's qualifying direct proof
links; it never demands a recursively complete chain. The per-row recipe is:

1. A row's `last_commit` resolves its `commit_proof` row by exact integer
   primary-key probe on the denormalized `sequence` column; epoch and digest
   fields must equal the row's link, and the proof's manifest hash validates
   under the accepted domain.
2. Locate the exact `(table, key)` entry inside that manifest (a bounded
   decode of the already-read proof body) and require `afterDigest` equality
   against the retained row's stored wrapper digest under the accepted row
   domain — compare with the row's *own* accepted projection, never with
   today's mutable versions of other rows.
3. An embedded snapshot's origin `CommitRef` (e.g. the four accepted control
   snapshots) resolves through the same rule and pins the historical snapshot
   bytes to their original manifest `afterDigest`; the current mutable row may
   have changed or been removed and is never compared against the snapshot.
4. Receipt/claim/checkpoint/receipt-slot expansions and the debit recipe's
   expected/owning proofs resolve through their accepted per-family recipes,
   each as exact-key probes; no recipe invents an alias or a fabricated proof.
5. A command's predecessor must equal the pre-transaction `header.latest`
   exactly and precede it by one; only that admission-time predecessor proof
   is additionally required by the accepting command. At reload, a retained
   row demands a predecessor proof only where its own accepted recipe names a
   required expected/predecessor proof (and for the current `header.latest`);
   a legitimately removed, no-longer-required predecessor does not imply a
   corrupt sequence gap. Intermediate unrelated predecessors are not demanded,
   consistent with the accepted sparse direct-proof retention.
6. A missing proof/body/row/link is unavailable, never repaired or
   zero-initialized; a changed/deleted historical row is not required to
   survive merely because a retained manifest once changed it.

Reload on open (private, bounded, no caller, no writer):

1. Enumerate the physical object set and require exact equality with the
   declared format-6 shape: same tables, same column names/types/keys, and the
   declared index set; any other or missing object fences the store outright.
2. Validate the stored header singleton: schemaVersion 6, exact workspace/epoch
   agreement with every row/ref, `latest` resolving one exact proof row; no
   retained proof sequence greater than header.latest.
3. For every retained family row: validate canonical bytes, key
   reconstruction, wrapper/digest and specialization column invariants, then
   resolve its direct proofs per the recipe above; then rederive its expected
   `membership_reverse` rows and compare against the stored values. Any
   extra, missing, mismatched or inconsistent entry fences the store. No
   read/recovery repair, missing-zero refill, row insertion or authority
   restoration is permitted.
4. The paired ordinary/hierarchy reload/crash-recovery correspondence is part
   of the accepted broker's linked qualification work; until that mechanism is
   qualified, linked admission stays unavailable and hierarchy-side reload
   validates only hierarchy-resolved links. Cross-store correlation remains a
   distinct open mechanism, not silently reduced to this contract.

Work budget: reload is not a discovery admission and does not consume the
admission budget. It is instead bounded by a **new, separate proposed
recovery profile** requiring its own scoped owner acceptance: at most one
observation per retained row decode, one per derived occurrence compare,
one per required direct proof probe, and their not-to-exceed byte counts —
proportional to the store's retained population, not to a fixed admission
constant. The observation/byte unit definitions are unchanged; the bound is
distinct from the 4,096-observation / 16 MiB admission profile and differs
from it deliberately: a legitimately retained store may hold more
membership-adjacent work than one admission may consume, and the two rules
must not be conflated. Exhaustion without a proved complete reload fences
the store, never a partial correspondence claim. Actual wall-clock/native
reload latency remains a native plan requirement, not evidence supplied
here.

## Accounting and no new allowance

| Cost | Owner |
| --- | --- |
| Each derived projection entry's bytes | the owning command's complete command accounting through the existing named terms (`headAndIndexBytes`, `receiptAndLookupBytes`, `dependencyBytes`, others); no source-payload credit and no omission |
| Declared unique/secondary structure storage | hierarchy main-file occupancy and reload workload under the existing store/file ceilings; no discount for engine-managed internal pages |
| Probe/byte observations | the accepted 4,096-observation / 16 MiB cumulative discovery profile, unchanged |
| Projection-table occupancy | bounded by the retained rows' own extent and their accepted identity/body widths; no independent allowance is created |
| Reload workload | the bounded private reload recipe above under the separate proposed recovery profile; a wall-clock/native bound is separately qualified |

No new numerical allowance, quota resource, discount or free read appears.
Where an arithmetic check bounds a representation below, that bound describes
*encoding scales* (canonical identity widths), not admitted source states,
and never asserts that all independently maximal shapes fit together — that
maximum-profile qualification stays its own gate.

## Compatibility

- The format-6 catalog from the [authorization design](2026-11-05-child-preparation-authorization-design.md#atomic-commitments-and-compatibility)
  is unchanged; this document declares only physical objects within one fresh
  format-6 store.
- Format-5 and earlier readers reject format 6 before any row interpretation;
  no migration, import, dual-profile read or backward copy path is created.
- No existing profile, format-4 store, ordinary schema-1 file, test mechanics
  fixture or old fixture bytes change. The physical shapes apply only to
  explicitly qualified new stores.
- A format-6 reader that sees any shape outside the declared set (an extra
  column, an undeclared index, a different collation or surrogate path) is
  fenced: no forward compatibility, no tolerated added column, no silently
  accepted variant.

## Denial, race and recovery oracles

| Case | Required outcome |
| --- | --- |
| Duplicate permit attempt under the same P (second fresh attempt after first is unknown or in-flight) | denied via the exact P PK and the UNIQUE permit column; no second durable row, no partial success, no silent overwrite |
| Two different P values attempting the same permit identity | one wins its compare-and-consume cutoff; the other denied entirely; the loser's prebuilt candidate grants nothing |
| Same P with conflicting producerSource/attachment bytes | denied conforming to accepted source-binding checks; no retry with patched bytes |
| Declared structure missing/extra/inconsistent for a retained row (simulated corruption) | reload correspondence check fences the store; no partial read and no auto-fill |
| Projection row for an unsupported consumer kind or unknown target | denied; the unsupported consumer removal is never valid |
| Reverse probe with malformed/unsubstitutable target encoding (digest-fallback or surrogate key) | denied before any effect |
| Historical snapshot consumed after the owning mutable control row changed later | the snapshot's origin link still resolves; the historical occurrence is not invalidated or rewritten |
| Later row body change with unchanged derived reference set | owning logical row change per accepted rules; zero projection differences; no authorship/implicit rewrite of the projection |
| Paired ordinary G drift between capture and transaction boundary | paired probes deny before effects (accepted race rule); no re-read/late-write path silently retries with a second G glance |
| Stale/advanced hierarchy H between capture and the transaction boundary | the captured header comparison denies before any effect; no patching of the captured read set, no second capture riding the same attempt |
| Reverse probe exhaustion (large fan-out on a hot target, shared admission budget) | denial at the accepted 4,096-observation / 16 MiB limit without proved completion; no partial consumer set, no pagination shortcut |
| One-sided lookup, hierarchy only (or ordinary only) | unavailable for the paired identity rules; no accepted lookup supports cross-store substitution with one-side only facts |
| Missing commit_proof for a retained row's link / deleted proof a retained consumer still requires | unavailable; missing proof cannot be regenerated from remaining rows |
| A projection row colliding with another command's outcome or a foreign command's receipts | denied; each command's recipe-derived occurrences must fit its own accepted edits; no cross-command aliasing |
| An unknown family, unsupported row, or unknown consumer | fences the affected probe/read; no "assert unknown means empty" shortcut |
| An attempt to treat indexed metadata as evaluation-content access (e.g. reusing receipt or approval bytes to substitute the precharge/content rule) | metadata-only boundary maintained; content access follows accepted debit/evaluation rules |

## Retention and boundary facts

- The declared structures never release capacity (no auto-shrink on any
  command); at resource limits the owning workload's accepted budget decides,
  never a partial entry loss.
- Workspace rotation or sidecar-generation replacement does not reset stored
  membership; the accepted purge/joint-purge requirement stays a separate
  scoped, qualified command.
- Every debit/subject/receipt protected-retention and consumption rule of its
  owning contract remains in force; this document adds none and removes none.
- The live admission-layer RegistryRoot proposal and any future reuse of
  registered admission tickets never replace this durable contract; their
  race ordering remains separately gated.

## Verification

Scoped docs-only verification; no SQL, DDL, database or Rust execution:

```text
$ pnpm --dir docs install --frozen-lockfile
$ pnpm --dir docs docs:check
$ node --test docs/.vitepress/okf.test.mjs
$ pnpm --dir docs docs:build
$ bash scripts/doc-check.sh
$ bash .claude/hooks/worktree-check.sh
```

One arithmetic evidence rule follows. Consistent with prior component work,
encoding bounds use representative canonical identity widths (IDs at their
inherited 128-byte ASCII maximum, digests at 64 lowercase hex characters):
representative shapes are not admitted source states and not a max-profile
claim.

```python
import hashlib, json

def canon(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"),
                      ensure_ascii=False).encode("utf-8")

def short(payload):
    return hashlib.sha256(payload).hexdigest()[:12]

# Representative maximal child AssignmentIdentity P: every ID field at its
# inherited 128-byte ASCII maximum, digest at full width, counters at bounds.
p_obj = {
    "childSessionId": "I" * 128,
    "configurationDigest": "d" * 64,
    "configurationId": "I" * 128,
    "kind": "child",
    "parentSessionId": "I" * 128,
    "planId": "I" * 128,
    "planRevision": 64,
    "reservationId": "I" * 128,
    "runId": "I" * 128,
    "taskId": "I" * 128,
    "taskVersion": 2147483647,
    "workspaceId": "I" * 128,
}
p_repr = canon(p_obj)

task_key = canon({
    "planId": "I" * 128,
    "planRevision": 64,
    "runId": "I" * 128,
    "taskId": "I" * 128,
    "workspaceId": "I" * 128,
})

# The paired absence probe set from this contract (observation costs):
# six absence/miss probes plus three revalidated prestate probes.
probe_keys = [
    p_repr,                                                     # 1 authorization PK (miss)
    p_repr,                                                     # 2 lineage PK (miss)
    b"I" * 128,                                                 # 3 attachments PK (miss)
    canon({"attachmentId": "I" * 128, "childSessionId": "I" * 128}),  # 4 UNIQUE pair (miss)
    task_key,                                                   # 5 reservations PK (prestate)
    task_key,                                                   # 6 launch controls PK (prestate)
    task_key,                                                   # 7 task controls PK (prestate)
    p_repr,                                                     # 8 ordinary lineage projection (miss)
    b"I" * 128,                                                 # 9 ordinary session identity (miss)
]
probe_bytes = sum(len(k) for k in probe_keys)
observations = len(probe_keys)
assert observations <= 4096 and probe_bytes <= 16 * 1024 * 1024

print(json.dumps({
    "absence_probe_bytes": probe_bytes,
    "absence_probe_observations": observations,
    "p_len": len(p_repr),
    "task_key_len": len(task_key),
    "p_sha256_prefix": short(p_repr),
    "evidence_sha256_prefix": short(str(observations).encode() + b":" + str(probe_bytes).encode()),
}, sort_keys=True))
```

Executed result at the inspection base on this tree, with an independent
repository-Node (v22) run reproducing the same canonical bytes and totals:

```json
{"absence_probe_bytes": 6215, "absence_probe_observations": 9, "evidence_sha256_prefix": "c4e2436fc667", "p_len": 1305, "p_sha256_prefix": "2de7dea067a8", "task_key_len": 583}
```

Observations (9) and cumulative probe-key bytes (6,215, counting each probe's
own canonical key bytes) sit far inside the accepted 4,096/16 MiB units; the
budget is shared with all other accepted discovery expansions in the same
admission — owning-record reads, join validations and any reverse lookups
still charge their own units, and exhaustion without proved completion still
denies. This is a canonical-encoding check only. Cross-language parity was
checked byte-for-byte with Python and Node (canonical JSON requires
identical UTF-8 output, and identical digests confirm it); it establishes no
native, filesystem, durability or database result and does not assert that
all simultaneously maximal source profiles fit together.

## Handoff and scoped decision

The single scoped decision this document requests: **accept the proposed
format-6 physical representation for further qualification**:

1. the declared physical structures — `membership_reverse`, the
   `dependencies` covering target index, the unique
   `child_preparation_authorizations.permit` and `child_attachments.attachment`
   columns, and the closed declared-object-set rule;
2. the closed target dispatch (including object-kind projection for
   non-dependency occurrences), per-family projection recipes and the paired
   ordinary/hierarchy identity rule as the physical lookup contract;
3. the exact-key absence recipes as the complete bounded absence mechanism,
   and the sparse direct-origin reload procedure as the recovery contract;
   this supersedes the chain-continuity interpretation *interpretively,
   inside the proposed profile* — it authorizes no change to any existing
   test-mechanics code;
4. the separate bounded reload/recovery probe profile (proportional to the
   retained population) as one scoped owned-work bound, distinct from the
   admission discovery budget;
5. the maintenance/accounting/compatibility rules above, with no new logical
   family, quota, allowance, command kind or manifest field.

It grants no eligible producer, no ordinary schema activation, no source
registration, no executable/native implementation plan and no runtime
capability. Actual ordinary/source/revocation owners
([#1242](https://github.com/Rambolarsen/orkworks/issues/1242) and the pending
cutoff/native qualification),
[RegistryRoot](2026-11-05-source-publisher-execution-design.md#proposed-registry-ordering),
complete transaction fixtures and any executable plan retain their existing
separate gates. #1176 stays open; #740 stays closed research.


## Bounded review disposition

One bounded read-only lower-cost review round used two independent reviewers
against the design plus its cited accepted contracts. Neither wrote any file
or ran a build; root resolved all findings.

Correctness review findings and dispositions:

- **Material (1):** object-target occurrences of metadata-consumer recipes
  (debit intent/authorization/heads/payloads/candidate sources) had no
  declared route because the projection excluded object-kind targets while
  `dependencies` purposes are a closed set. **Applied:** the dispatch and
  projection now cover every non-dependency occurrence of any consumer,
  including object-kind targets; `dependencies` keeps its accepted purposes,
  with an explicit no-dedup union rule for reverse-by-object results.
- **Material (2):** the reload work bound reused the admission discovery
  budget, which can fence legitimately larger stores. **Applied:** reload is
  no longer bound by the admission profile; a separate proportional
  recovery-probe profile is proposed as its own scoped choice, distinct from
  the admission budget, with the same unit definitions.
- **Minor:** the blanket reload-time predecessor demand was narrowed to its
  accepted conditional rule (requirement only where the owning recipe names a
  predecessor proof, plus the admission-time rule); the mis-cited recovery
  basis was corrected to the storage-contract/authorization wording; the
  candidate-command empty proof-removal default is now labeled "as proposed";
  the "19 families" label was replaced with the exact 14+15+4+2 = 35
  decomposition; root_drafts moved to the non-projecting group per the
  accepted outside-locators boundary; the occurrence rank received a
  deterministic three-level canonical enumeration rule making byte-identical
  duplicates distinguishable; the child_attachments UNIQUE column's role was
  reworded (exact-key route; recipe joins stay recipe validations).
- Builder-review findings were the same four blocking gaps (occurrence/order
  determinism; the family-closure contradictions for objects/store_header/
  commit_proof; the conditional ordinary absence probes; the unstated
  tombstone/projection rule), now closed by the applied edits above. Minor
  dispositions applied: reverse-by-object granularity named; mechanics
  supersession clarified as interpretive-only with no code change; a `C(x)`
  pointer added; the declared object set's engine-internal wording tightened
  to declared UNIQUE backing structures only; an H-drift oracle row and a
  fan-out exhaustion oracle row added; `workflow_status` frontmatter added.
- Both reviewers verified the recorded arithmetic independently and
  byte-for-byte; the arithmetic, budget accounting, gate separation,
  uniqueness/paired-identity rules and sparse direct-origin recipe otherwise
  produced no findings.

Applied corrections are author-applied clarifications from one round; no
second independent post-edit review is claimed. They do not constitute
acceptance, qualification, or an owner decision on any proposed choice; the
recovery probe profile and every other choice here remain gated exactly as
the handoff states.

