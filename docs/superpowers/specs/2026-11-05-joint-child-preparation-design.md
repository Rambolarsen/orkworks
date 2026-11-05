---
type: Design
title: Joint original child preparation publisher
description: Proposed one-worker atomic ordinary-session and hierarchy preparation, with explicit compatibility and availability tradeoffs.
tags: [taskmaster, hierarchy, sessions, storage, preparation]
status: draft
workflow_status: architecture-accepted-qualification-required
---

# Joint original child preparation publisher

## Decision and finite scope

[#1406](https://github.com/Rambolarsen/orkworks/issues/1406) continues
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176) and the
[owner-pending source-admission proposal](2026-11-05-evaluation-source-admission-design.md).
This document proposes the first actual publisher's execution architecture:
original child-source-attach, including ordinary session creation and the complete
hierarchy preparation outcome. It is not an implementation plan or an eligible
source profile. The prior source-admission choices remain proposed; “keep going”
on this design task is not recorded as their scoped implementation acceptance.

The author recommends one private per-workspace transaction broker, owned by the
ordinary SessionStore worker. A qualified linked profile attaches exactly its
one hierarchy database to that worker's ordinary database connection and commits
both stores in one rollback-journal transaction. Hierarchy producers supply sealed
typed plans, never SQL or database handles. There is no second concurrent hierarchy
connection or independent ordinary commit for this event.

This deliberately proposes amendments to the accepted no-ATTACH/separate-worker
arrangement, ordinary schema, hierarchy catalog/version and linked-child physical
purge behavior. Their exact consequences are below. No existing format or
approval is silently reinterpreted. The price is coupled SQL availability in a
linked workspace: a blocked hierarchy transaction can occupy its ordinary SQL
worker. PTY input/draining, immediate close/kill and core locks remain independent.
That availability tradeoff requires explicit owner review.

No production code, store opening, schema/migration, source registration, live
capability, native/model probe or new quota/bearer is delivered. The production
hierarchy opener remains fenced; #740 remains closed research. Physical source
authority at commit and the complete catalog/membership/resource/native profile
remain gates even if this atomicity architecture is accepted.

## Pinned evidence and alternatives

Inspection base: `08392ec987cf9e8752841b2dcad9c87722955c2d`.

| Evidence | Consequence |
| --- | --- |
| [Current create_session_workflow](../../../crates/orkworksd/src/session_application.rs) installs a SessionHandle and writes a JSON creating record before startup | Reusing that public workflow and later writing hierarchy rows cannot prove joint original preparation |
| [Private SessionStore](../../../crates/orkworksd/src/session_store.rs), Database::insert, and [worker](../../../crates/orkworksd/src/session_store/worker.rs) | The dormant ordinary adapter owns one connection and commits its create internally. The joint path must use private transaction-scoped persistence, not call create then another commit |
| [Dormant SQL lifecycle](../../../crates/orkworksd/src/session_application/sql_writes/lifecycle.rs) | Captures ordinary runtime/creation ownership; it is neither original hierarchy preparation nor an evaluation claim. Production lifecycle remains JSON-backed |
| [Ordinary storage contract](../../../specs/session-storage.md) and [accepted design](2026-11-01-sqlite-session-storage-design.md#storage-ownership-and-schema) | One workspace-owned SQL worker; rollback DELETE/EXTRA; 1 MiB record/intent and bounded queue. Activation/cutover and full producer qualification are still pending |
| [Hierarchy physical design](2026-10-23-hierarchy-transaction-storage-design.md#durable-unit-and-storage-ownership) and [test mechanics](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/connection.rs) | Accepted one private connection, no ATTACH; test mechanics also use DELETE/EXTRA but are not a qualified source publisher. This proposal needs a narrow explicit amendment |
| [Accepted attach effect map](2026-10-31-child-review-source-preparation-design.md#fresh-attachment-and-preparation-closure) | Ordinary lineage/session and complete source/control/summary/five-ledger preparation need one proven atomic/recovery disposition before spawn; missing historical rows cannot be backfilled |

SQLite documents multi-file atomic transactions for attached persistent databases
outside WAL, and describes the super-journal used by rollback-mode multi-file
commit. See [ATTACH](https://www.sqlite.org/lang_attach.html) and
[atomic commit, multi-file commit](https://www.sqlite.org/atomiccommit.html#multi_file_commit).
These are engine prerequisites, not qualification of OrkWorks' VFS, filesystem,
flush behavior, journal limits, leases or revocation cutoff. This design infers
that a qualified single-connection transaction is a candidate mechanism; it
claims no local crash/power-loss result.

| Alternative | Author disposition |
| --- | --- |
| Ordinary create then hierarchy commit, with later index/zero-row repair | Reject: a published ordinary child is not evidence of the complete original hierarchy event |
| Co-locate all ordinary history inside the hierarchy database | Reject for this slice: imports unrelated history/schema/retention into the bounded hierarchy file and changes both authorities more broadly |
| Two independent stores plus an application-level prepare/commit/recovery log | Do not select: adds participant states, durable decision representation, cleanup and unknown-outcome recovery before the original publisher exists |
| One broker, one attached rollback transaction | Recommend, subject to the explicit no-ATTACH/worker/format/retention amendments and native qualification. It supplies one engine commit rather than an application commit protocol |

## Owners and the permitted connection

Propose private `session_store::joint_preparation` transaction execution and
`taskmaster::child_source_publisher` domain planning. The source-admission module
owns only cutoff/freshness and the typed source transition; it cannot attest SQL
disposition by callback. SessionStore remains the ordinary metadata authority;
the hierarchy module remains the authority for its source/control/proof records.
The broker coordinates those typed owners, not a generic writable union.

In an eligible linked workspace:

- The existing ordinary worker owns the sole connection and all ordinary SQL.
  It opens its already-activated ordinary database as main and attaches exactly
  the verified existing hierarchy file under the fixed schema name hierarchy.
- The global hierarchy worker position/queue/resources remain unique. Hierarchy
  plans execute through this broker; the former standalone hierarchy connection
  is not opened concurrently. This changes execution ownership, not the number
  of admitted hierarchy workers or positions.
- All SQL, including header/pragma/index/proof queries, names its schema explicitly.
  The same table name in two databases cannot select the wrong owner implicitly.
- Attachment is an internal startup/maintenance operation on owner-resolved paths
  and exact workspace/database epochs, using existing-file-only access. No URI,
  VFS, schema name, path, SQL, ATTACH/DETACH or handle comes from a caller. Never
  create a missing peer as a side effect of attachment.
- Verify both files, their full catalogs, activation/format records, actual file
  ownership and effective settings. Require persistent databases, DELETE/EXTRA
  on both, the qualified common VFS/local-filesystem profile and actual owned
  workspace scope. WAL, memory/temp databases, unknown peers or a missing
  qualified auxiliary-file mechanism deny linked admission.

The default ordinary profile need not attach anything and remains independently
usable. Linked eligibility requires the qualified ordinary SQL cutover plus the
new hierarchy profile; it is unavailable on production JSON, old/unknown schemas
or the existing partial test mechanics. No opportunistic cross-store adoption.

## Original ordinary lineage and versioned commitments

The proposed ordinary canonical metadata reserves one internal member,
`orkworksHierarchyPreparation`, with exactly:

```text
{schemaVersion:1, assignmentIdentity:ChildAssignmentIdentity,
 attachmentId:ID, assignmentSourceDigest:Digest, preparationCommit:CommitRef}
```

Its childSessionId equals the ordinary session ID. The actual ordinary creation
identity retains the existing exact id/created_at contract; created_at is not an
authorization credential. This member is created only by the joint publisher,
then immutable through label/status/organization/resume updates. Ordinary agent,
renderer, hook and imported JSON writes cannot set or adopt it. An old/unknown
reserved member is retained as unverified legacy data under migration policy,
never recognized as qualified lineage or used to fill missing source history.

Propose ordinary database schema version 2 with a canonical full-assignment-key
projection/index for this member. The projection is derived transactionally from
the canonical metadata; it is not a digest-only index or another authority.
Verify key/value correspondence and uniqueness, including tombstoned rows, and
use it for exact forward/reverse lineage lookup without scanning session history.
This is an ordinary schema proposal, not an automatic migration or cutover.
Old readers reject the new schema; migration/backup/activation owners must define
a separately approved maintenance transition before existing ordinary stores
could participate. No existing schema-1 file is changed by this proposal.

The lineage is a new typed direct metadata/proof consumer. Do not hide it from
the complete manifest or keep it as an unregistered external reference. Propose
one logical family ordinary_child_lineage, physically projected from the ordinary
metadata/index, with exact key = the full ChildAssignmentIdentity and body:

```text
{createdAt:<exact ordinary created_at>,
 linkage:<the complete orkworksHierarchyPreparation member>}
```

The key, session ID, creation identity, source digest and attachment must join the
original hierarchy attachment/source and prepared ChildSessionState. It is a
fixed storage metadata projection, not a new ObjectRef kind, source artifact,
public upload or copy of the complete ordinary metadata body.

Because format 4 has one closed catalog and no alternate catalogs inferred from
row presence, propose a **distinct new-store-only hierarchy format 5**. Header
and manifest schemaVersion are 5; use orkworks.hierarchy-storage-change.v5 and
orkworks.hierarchy-storage-row.v5 plus LF. Retain all accepted source, receipt,
claim-proof and nested codec versions/domains unchanged. The new profile requires
the already named 33-family inventory to be reconciled and closed, plus this
34th logical projection. That catalog work is still open; the existing 17-family
test mechanics are not its implementation.

Use the existing closed manifest shape and normal row envelope, adding this
family's typed key/body. Its row commitment excludes only
ordinary_child_lineage.linkage.preparationCommit, preventing a digest cycle.
Every other key/body field is committed. The attached ordinary member receives
the resulting C after the complete manifest is hashed, within the same SQL
transaction; C also owns all original hierarchy preparation rows. Separately
validate that excluded link against the retained original proof and immutable
attachment/source, even after mutable bridge rows have later owning commits.
The original immutable attachment/source retain C; a changing latest header is
not a substitute for that original proof.

The projection is an additional real logical change and retained occurrence.
Charge its canonical key/body under the existing child-source-retained-bytes
run/workspace pools, complete logical-store/command/row and discovery budgets.
Do not charge only the bytes physically stored in the hierarchy file or claim
content-sharing discounts. Ordinary metadata/index bytes also keep their ordinary
owner accounting. No new resource pool or increased capacity is proposed.

A format-4 reader rejects format 5 before row interpretation. No v4/v5 mixed
commitment, migration, rewrite of old goldens or profile reset is allowed.
Complete registration of this projection and both lookup directions at the same
captured header pair is part of qualification, not a post-commit registration.

## Exact original preparation command

The source publisher owns child-source-attach's accepted semantics. It must
resolve original plan/configuration/input/approval/reservation/allocation and P,
actual complete membership and all source/control/summary/proof/quota rows.
Original reservation taskVersion remains frozen as assignmentTaskVersion; a
current live version cannot rewrite identity. Require full session/attachment
uniqueness across the retained source history and ordinary lineage index.

Capture the hierarchy header H and ordinary database epoch/history generation G
plus the exact ordinary creation/absence read set in the broker's coherent view.
Revalidate both and every applicable source read before effects. G is an existing
ordinary change token, not a new authority generation. An unrelated ordinary
change may conservatively invalidate a prepared capture; this grants no retry
or permission to omit the ordinary read set.

Private transaction-scoped ordinary and hierarchy functions accept only the
owning typed plan and borrowed transaction. They cannot commit independently.
Do not call SessionStore::create, journal_object, establish_resource or a generic
single-row journal and then attempt to join their already committed outcomes.

| Side | Inseparable effects under one C / engine transaction |
| --- | --- |
| Ordinary | Insert the complete normalized creating metadata at its original creation identity/revision, initial history/search projections and the immutable preparation member/index. Increment ordinary history generation once; no SessionHandle, startup future or PTY is published yet |
| Hierarchy | Insert original attachment and child-assignment-source, prepared ChildSessionState at producerRevision "1" / runtime null, prepared launch control, complete provenance/direct membership, original summary branch, five new assignment resource rows at used="0"/reserved="0", and every actual source/control/summary/index/proof effect |
| Joint linkage | Include ordinary_child_lineage in the manifest and reciprocal/direct membership; attach C only after hashes; publish both cache/reference sets and invalidate snapshots before releasing the position |

The ten corresponding run/workspace resource rows already exist and reconcile
against retained preparation history; do not initialize them on attachment or
stamp unchanged peers. The actual summary branch is either its exact initialized
Unassessed row/checkpoint or the accepted never-initialized marker. This event
creates no result, evaluation, receipt slot, artifact, read debit or live grant.
The minimum logical recipe is not a complete changed-set/packing witness.

After a proven joint commit and ordered reference publication, return one
owner-created PreparedChild result bound to C, both database epochs, P and the
ordinary creation identity. Its constructor is private to the qualified broker;
a struct decoded from caller bytes or a successful ordinary create is not that
result. Exact retry validates retained original C, the full original source
outcome and the ordinary lineage projection, returning the original identity
without another row, revision, ledger initialization, position or runtime grant.

A complete committed creating row may appear in ordinary history before the
application publishes its in-memory cache/reference result, including after a
proven joint recovery. This is intentional durable-history visibility, not
visibility of a live SessionHandle, a successful spawn or a new capability.
History reads cannot manufacture PreparedChild, release the pending source fence
or bypass the post-commit publication gate. Unknown recovery reports incomplete/
unavailable linkage rather than pretending history is empty or preparation is
accepted. No extra visibility transaction or pending-row repair is proposed.

Only then may the separately authorized runtime owner install its exact unissued
carrier and construct startup work. Actual native attachment, source-owned
runtime history/current bridge and final evaluation registration follow their
own source-admission checks. A committed preparation is neither a spawn permit
nor permission to launch after cancellation. Failure before first polling still
closes the unissued carrier.

## Permission, ordinary lifecycle and visibility

The joint commit solves a proposed storage atomicity problem, not source write
authority. Child-source-attach remains excluded from evaluation claims and debit
permits. Current source authority must hold at its own physical commit cutoff;
no new PreparationPermit, claim-first exception or permission to settle after
revocation is proposed. A pre-SQL check, before-commit callback, transaction
atomicity, SQLite hook or advisory lease does not prove that cutoff. A qualified
native mechanism remains a required executable gate; absent proof means no
eligible original publisher, even if the joint SQL primitive works.

During construction, only immutable bounded intent and application reservations
are captured under core guards. Queueing, SQL, fsync and proof validation run
without projection/workspace/sessions/prompt/publication locks. Begin the exact
source transition before effects and preserve newer fences on completion.
Publish only after the joint disposition is proven. A commit failure/unknown
response is not success and does not authorize reconstructing missing lineage.

Subsequent run/approval/source changes, spawn intent/outcome, resume/ending,
coordination and recovery need their accepted typed publishers and exact runtime
ownership. This document gives those events no new tag or generic write path.
The participating-writer inventory must cover public ordinary lifecycle actions,
actual runtime/ending owners, native/terminal failure, namespace/workspace/health
change, source/target/configuration/control mutation and supported recovery.
Pure label/attention/time display remains non-evidence and does not itself
publish quality or recreate original preparation.

Immediate ordinary close/kill/output handling does not wait for the broker.
If later lifecycle bridge publication cannot be proved, keep hierarchy unavailable
and capacity conservatively held. No stored ordinary status is converted into a
terminal source event or automatic grant. Complete ordinary/hierarchy lifecycle
coherence is a later coupled publisher qualification, not delivered by original
attachment atomicity.

Ordinary Delete may hide a linked session through its canonical tombstone and
stop its runtime independently. **Proposed additional physical-purge rule:** a
row carrying this lineage cannot be physically erased by ordinary retention or
external-cleanup completion while its hierarchy source/custody consumers survive.
Keep the hidden row/member/index as a typed retained consumer until a separately
qualified joint purge removes it with the source-owned complete closure. No
cascade, silent GC or new automatic cleanup. This can retain ordinary metadata
bytes after user-visible Delete; that cost is an explicit owner decision and
must be represented accurately in pending-cleanup/storage reporting. Ordinary
non-linked rows retain their existing deletion policy.

## Worker, storage, recovery and bounds

Ordinary scheduling remains 32 waiting mutations, eight reads, 32 MiB admitted
bytes and 1 MiB serialized intent/metadata limits. A hierarchy job also consumes
its existing global single active-or-retiring position, at most eight waiting
commands and the existing hierarchy byte allowance. Both admissions must succeed;
a reference to an out-of-band plan cannot evade either budget. The complete
joint job, its referenced source/effect bytes and any retained completion obey
the lower limits. In particular this broker profile may reject a source-valid
large effect set under the ordinary 1 MiB intent limit; no maximum-profile fit
or extra allowance is inferred. Queued expiration/FIFO/read fairness and native
SQL cancellation/busy behavior remain owned and bounded; no replacement worker.

The linked hierarchy file keeps its existing 4,096-byte pages / 1 GiB physical
ceiling and logical/source limits. The ordinary history file remains under its
ordinary owner rather than being counted as hierarchy history. The joint job's
actual logical projection bytes still count as described above. The accepted
2 GiB rollback/auxiliary-file ceiling must cover the joint command's ordinary
and hierarchy journals **and super-journal together**, including recovery and
statement-journal classes; it is not a fresh allowance per attached file.
The 8 MiB SQL scratch and all artifact/materializer/queue budgets remain separate.
Unknown native amplification or accounting denies qualification, not extra disk.

A blocked joint transaction can occupy both owners' durable service in this
workspace. Ordinary history/metadata requests may report bounded queue/busy/
unavailable results while input/draining/stop continue. This is the explicit
shared-worker availability amendment; it does not claim the old independent
SQL-service behavior. A retiring old workspace retains its exact resources and
cannot clear a newer scope or enable another hierarchy worker. Other workspaces'
ordinary authorities are not adopted by the old broker.

| Failure / recovery point | Required disposition |
| --- | --- |
| Admission fails, peer/schema/setting invalid, read-set drift or proven rollback | No new ordinary child, lineage or hierarchy effect; retain original reservation/allocation as the owning source policy requires; no new attempt or spawn |
| Process/power interruption during joint commit | Qualify SQLite recovery for both files under the same ownership/profile. Accept only complete old or complete new joint state. Do not independently recreate a missing peer, copy a live file or infer rollback from one missing row |
| Commit proven, response lost | Resolve the original complete C and its ordinary projection; return historical PreparedChild only under applicable current lookup authority. Do not auto-spawn, issue a grant or initialize again |
| Unknown native/storage outcome or one-sided/corrupt source | Keep both linked authorities unavailable for joint work and retain worker/leases until supported disposition. No partial repair/backfill, new connection to bypass retirement or successful empty history |
| Stop/cancel/workspace change races the operation | Immediate close remains responsive; no new grant/spawn. The excluded source-command cutoff must be independently proved; atomic SQL is no permission to commit after revocation |
| Ordinary Delete / retention after successful preparation | Hide/stop under ordinary rules, preserve the typed lineage consumer and original hierarchy proofs until qualified joint purge; no implicit individual custody purge |
| Backup/restore/migration | Quiesce both owners and include both databases, activation records and required related files as one consistent workspace set. Recover journals through SQLite, never omit a peer or hand-delete a hot journal; validate epochs/C/index closure before enabling linked work |

No orphan handling, same-version restore, old-store migration or empty profile
initialization grants runtime authority. The ordinary schema-2 transition and
new hierarchy format-5 creation require their own closed, owner-approved
maintenance/compatibility plan. Existing source domains and old store bytes stay
unchanged; no automatic upgrade or downgrade is selected.

## Reviewable choices and execution handoff

The concrete owner-pending choices are:

1. A private attached rollback transaction broker replacing the separate hierarchy
   connection for a linked workspace, with the disclosed SQL availability coupling.
2. A reserved immutable ordinary lineage member and exact indexed projection,
   ordinary schema version 2, and a new-store-only format-5 hierarchy catalog/row/
   manifest domain with the one explicit commit-link exclusion above.
3. Charge the new logical projection under existing source/command budgets and
   retain linked ordinary tombstones until qualified joint purge; no new quota.

These are proposals, not amendments accepted by drafting or merging this file.
In particular no-ATTACH, independent worker and format-4 rules still govern the
existing profile until the owner approves an exact replacement and its later
implementation/qualification plan. The source-admission design remains proposed;
unchanged accepted custody/control/claim/debit choices are reused.

After the scoped architecture decision, prepare one execution plan for the
coupled original publisher only when the complete new profile/catalog, paired
ordinary activation, metadata-only membership and actual source commit cutoff
are closed. Its tests must use full source-valid old/new ordinary and hierarchy
states, exact row/proof/index/ledger/summary effects, crash/unknown/retry/stop and
packing denial. Do not implement an eligible flag, partial catalog, generic
journal or fake PreparedChild factory as a substitute. A separate bounded native
plan must qualify the actual build/VFS/lease/auxiliary-file/durability behavior.
#1406 delivers this concrete design; #1176 remains open for the complete profile.


## Bounded review and dispositions

One bounded read-only lower-cost review round asked separate correctness and
builder/recovery questions. Neither found an architecture, commitment-cycle,
retention or resource contradiction in the proposed mechanism. The correctness
reviewer required wording that the 33-family inventory is still awaiting complete
reconciliation, rather than implying it is already closed; the author corrected
that statement. The builder requested a minor ordinary-history visibility rule;
the author made committed creating-row visibility explicit while preserving the
separate in-memory PreparedChild/publication and no-spawn gates.

Both reviewers left native crash/lease/VFS/auxiliary-file behavior, full new
catalog/proof/membership, ordinary schema activation, source commit cutoff and
later lifecycle integration unverified, as this proposal requires. No extra
review round, new numeric allowance, SQL fixture or native test was introduced.
The author clarifications are recorded here; merge does not accept the three
architecture choices or authorize their implementation.


## Scoped architecture acceptance — 2026-11-05

After reviewed [PR #1407](https://github.com/Rambolarsen/orkworks/pull/1407), head
`7b0cdae57d0489a12f057d7b65859dcc1df83a49`, merged
`34f47bd771da99683020d6194ccc7810544213b3`, the owner answered “keep going” to
the concrete question approving the shared SQL worker, versioned lineage profile
and retained child tombstones **for further qualification**. This accepts:

- The linked-workspace broker/one attached rollback transaction, with the
  disclosed coupling of ordinary SQL history/metadata availability; ordinary
  input/draining/immediate close remain independent.
- The exact reserved lineage member/index and ordinary schema-2 direction,
  new-store-only hierarchy format 5 / logical ordinary_child_lineage projection,
  commit-link exclusion and unchanged source/claim/nested domains and budgets.
- Retaining linked hidden ordinary tombstones as source consumers until qualified
  joint purge, including possible retained metadata after user-visible Delete.

Earlier owner-pending wording is superseded only for those three architecture
choices. [ADR 0106](../../adr/0106-joint-child-preparation.md) records their scoped
replacement of the old linked-profile worker separation. Do not request these
unchanged choices again. This is no executable-plan, SQL/schema/migration,
source registration, source-permission change, platform/native probe or runtime
activation approval. #1398's separate source-admission choices are not silently
accepted by this question. Complete catalog/membership/resources, ordinary
activation and physical source authority remain unqualified.

[#1410](https://github.com/Rambolarsen/orkworks/issues/1410) investigates the
[original preparation commit/revocation cutoff](../../validation/child-preparation-commit-authority.md).
Its possible one-command source-permission alternative is a separate proposal;
none of the accepted architecture choices widens the current source cutoff.


## Source-admission decision and execution routing — 2026-11-05

The owner separately accepted #1398's interface/ordering choices for further
design, as recorded in the [source-admission decision](2026-11-05-evaluation-source-admission-design.md#scoped-owner-acceptance-for-further-design--2026-11-05).
Earlier pending wording for those choices describes the original inspection
context. The joint architecture acceptance above independently settles its three
broker/schema/profile/retention choices for further qualification; neither scoped
decision supplies source permission or implementation/native qualification.

The [source-publisher execution design](2026-11-05-source-publisher-execution-design.md)
reuses this accepted direction and proposes retained live-registry ordering.
That new registry choice remains owner-pending. It preserves the separate
[preparation authority investigation](../../validation/child-preparation-commit-authority.md)
and its still-pending one-command alternative; no implementation follows from
routing these documents together.


## Separate preparation-permission acceptance — 2026-11-05

The [scoped one-original-preparation policy decision](../../validation/child-preparation-commit-authority.md#scoped-original-preparation-policy-acceptance--2026-11-05)
now permits an exact original preparation that wins its own short authority
cutoff first to finish saving after later volatile revocation. The original
permission section above describes the stronger pre-amendment rule; all other
source actions retain that rule. Engine atomicity remains distinct from this
permission, with source/H/G/membership/quota/health/ownership checks unchanged.

No runtime/startup/spawn follows after cancellation, and the complete unstarted
preparation can retain history, charges and capacity. [#1415](https://github.com/Rambolarsen/orkworks/issues/1415)
closes the separately proposed authorization-evidence/compatibility representation;
no schema/format or proof placement is selected here. Existing architecture
acceptance and all physical/producer/native/activation gates remain unchanged.
