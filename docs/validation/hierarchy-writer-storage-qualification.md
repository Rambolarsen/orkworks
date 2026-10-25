---
type: Qualification Register
title: Hierarchy writer and storage qualification
description: Source-inspected persistence seams, missing hierarchy writers and required transaction and crash evidence.
tags: [orkworks, hierarchy, storage, recovery, validation]
status: draft
---

# Hierarchy writer and storage qualification

Tracking: [#746](https://github.com/Rambolarsen/orkworks/issues/746), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection baseline: `907decdf9769b6c0551d192ac96d897c9a481c4d` (PR #987).
Evidence method: static source and contract inspection on 2026-10-23. No runtime
or filesystem crash qualification was executed. Every hierarchy storage/platform
combination remains **unqualified**.

The [accepted recovery choices and proposed producer mappings](../superpowers/specs/2026-10-20-hierarchy-read-recovery-design.md)
define the requirements. This register identifies existing seams and missing
proof; it selects no storage engine, changes no accepted behavior, and authorizes
no implementation. Producer agreement, shared-learning custody, architecture
review and separately approved runtime plans remain gates. #740 capability work
stays deferred until November; role profiles remain unverified/no-go.

## Existing seams and limits

Paths below refer to the inspection baseline. Function names are search anchors;
these are concrete integration candidates, not a claim of complete writer coverage.

| Surface | Inspected seam | Observed scope and qualification gap |
| --- | --- | --- |
| Data-only plan records | [`CoordinatorStore`](../../crates/orkworksd/src/taskmaster/coordinator_store.rs): `put_proposed`, `activate`, `transition`, `resume`, `recover`, `publish` | Serializes cooperating handles with an in-process mutex and retained advisory file lock. Stages and syncs one record, rechecks predecessors/approval, atomically publishes and syncs directories. Recovery validates records and removes abandoned temporary files. No joint report/evaluation/dependency/summary transaction or ordered summary history is present |
| Existing recoverable multi-record publication | [`RecommendationStore`](../../crates/orkworksd/src/taskmaster/store.rs): `apply_recommendation_graph_transaction`, `recover_transactions`, `recover_transaction` | Recommendation-specific staged replacements, backups and committed manifests; reads invoke recovery. Useful precedent for recovery fixtures, not a generic hierarchy transaction API or proof that report receipts and summary lineage can share its commit. Preserve existing recommendation behavior |
| Canonical session metadata | [`MetadataStore`](../../crates/orkworksd/src/metadata.rs): `try_write_session_owned`, `try_mutate_session_owned`; [`session_transactions`](../../crates/orkworksd/src/metadata/session_transactions.rs): `update_session_locked`, `commit_session_record_with_revision_before_replace` | Retained store supplies a per-session lane and revision checks. Inspected replacement paths write a temporary file and rename it without file/directory sync barriers. They do not prove durable multi-source hierarchy publication. Separate reader stores do not share those write locks |
| Session-list projection | [`SessionProjection`](../../crates/orkworksd/src/session_projection.rs): `snapshot`, `list`; [`AppState`](../../crates/orkworksd/src/main.rs): `projection_lock` | List projection has an outer lock, but snapshots catalog/live/workspace and disk metadata through separate acquisitions. It supplies neither the proposed hierarchy source-token validation nor proof that every writer invalidates every affected tracked run |
| Session removal and retention | [`SessionApplication`](../../crates/orkworksd/src/session_application.rs): `delete_session`, `forget_session`; [`retention`](../../crates/orkworksd/src/runtime/retention.rs): `retention_cleanup_once`, `delete_session_evidence` | Close and forget are distinct. Evidence deletion orders several stores and may fail partway through; existing reservations/protections must survive integration. It is not #744's atomic subject/dependency/summary purge |
| Agent metadata delivery | [Metadata protocol](../agents/architecture.md#metadata-protocol), [`session-metadata.py`](https://github.com/Rambolarsen/orkworks/blob/907decdf9769b6c0551d192ac96d897c9a481c4d/crates/orkworksd/scripts/session-metadata.py) | The API-only producer is prepared, while #789 migration/eligibility remains pending. Inventory direct-file producers and unsupported paths before claiming all hierarchy-visible metadata participates; a sidecar mutex cannot serialize an external file writer |

The data-only coordinator is not the proposed ordinary-child runtime. Existing
workflow observations are not #743 skill evidence, and current session-summary
checkpoints are not #744 summary revisions. Reusing similarly named stores or
counters cannot establish their identity, authority, replay or retention contracts.
See the [authoritative scope](../../specs/taskmaster.md#proposed-ordinary-child-orchestration-extension)
and [current architecture](../agents/architecture.md#proposed-ordinary-child-orchestration).

## Writer coverage required before implementation handoff

No concrete production owner exists yet for the proposed hierarchy stores below.
The implementation plan must replace each missing entry with exact module,
mutation/read entry points, lock ownership/order, durable commit/recovery hooks,
and a test that bypassing that writer prevents complete projection. Include
background, startup, retention and read-triggered mutations, not just HTTP handlers.

| Producer/dependency | Required writer coverage | Current disposition |
| --- | --- | --- |
| #741 configuration and display catalog | Approved immutable snapshot admission, retirement/catalog label changes, exact configuration/model joins | Immutable hierarchy configuration owner missing; catalog reads exist in `SessionProjection::snapshot`. Catalog mutation inventory remains required |
| #742 run/plan coordination | Run creation, proposal/approval/revocation, task membership, reservations/allocation recovery, ownership transfer, runtime/capacity changes, cancellation/resume, blockers and clarification | `CoordinatorStore` is a data-only precedent; run/grant/admission publication owner missing |
| #742 report stream | Initial report, correction, exact retry, report head, receipt, dependent preparation/freshness changes and source degradation | Proposed only; no hierarchy report-head/receipt store qualified |
| #743 evidence | Event/replay/alias receipts, high-water and aggregates, coverage correction, clear/trim, generation changes and detected corruption | Proposed only; no hierarchy evidence store qualified. Telemetry cannot change #744 quality/completeness |
| #744 result/evaluation | Candidate admission/finalization, result head, reviewer/user stream replacement, dispositions/invalidation, eligibility/currentness and incoming dependency membership | Proposed only; no result/evaluation aggregate or authoritative dependency index qualified |
| #744 durable summary | Initial admission, every committed material transition, unavailable/exhausted fences, quota accounting, replay/checkpoint/rotation and subject purge | Proposed only; no durable summary owner qualified |
| Ordinary runtime and display | Session start/resume/end/forget, attention, copied labels/model/time values, catalog updates, workspace/lease replacement and source-health detection | `SessionApplication`, `MetadataStore`, `SessionProjection`, retention and runtime callbacks are inspection roots; complete caller inventory and publication integration remain required |
| Projection/read owner | Tracked run membership, counter lifetime/exhaustion, capture/final validation, exact detail ownership/freshness and cancellation | Proposed only; no hierarchy overview/detail publication owner qualified |

A complete inventory must trace callers and direct writes, not merely add a
wrapper around a store. Document the order of the publication boundary relative
to workspace, session, metadata-lane, catalog and storage locks. No waiting for
PTY/model/network activity or unbounded I/O belongs inside the proposed short
boundary. Preserve the ordinary-session persistence contract; in-memory attention
changes gain coherent capture/invalidation, not new durable summary obligations.

## Transaction coverage checklist

These are requirements to demonstrate against the existing producer contracts,
not a new commit format or ordering decision:

- A #742 correction commits its exact source descriptor/head and retry receipt
  with the affected dependency/preparation changes and summary transitions or
  durable unavailable fences. Preserve rate accounting before receipt lookup and
  exact retry before head CAS. Old receipts never replay an old correction.
- #743 commits evidence and its replay/high-water/aggregate effects together or
  recovers them before complete reads. Its alias/generation/coverage semantics
  remain independent of #742 receipts. Only visible changes invalidate snapshots;
  telemetry alone never changes summary material.
- #744 commits result/evaluation/disposition and dependency changes with all
  affected summary transitions or unavailable fences. Prove the complete affected
  set; incomplete discovery requires conservative reconciling. Optional-only
  changes preserve material revision, while pass → unassessed → pass counts twice.
- Summary-capacity exhaustion preserves rows/history. An authorized correction
  may proceed only with durable unavailable protection; failure to persist that
  protection rejects publication. Source-quota failures still reject admission.
  Initial admission without summary capacity cannot be repaired by inventing
  revision 1 on a later read. Counter exhaustion never wraps or resets.
- Purge rechecks exact subject revision and incoming retained dependencies under
  the same publication ordering and removes the source/summary coherently.
  Retention, corruption/quarantine and workspace deletion must not expose an
  old current pass. Cross-workspace custody is a separate prerequisite.
- Snapshot invalidation happens before releasing the boundary after publication.
  Events are hints. Exact detail reads revalidate ownership, health, dependencies
  and the selected snapshot, preserving canonical bytes and lossless integers.
  They cannot repair missing lineage or restore approval/report authority.

The durable unit must be identified explicitly. A set of successful independent
renames, a mutex, or notifications after writes does not demonstrate it. The
runtime design must choose how interrupted publication is detected before reads,
and how receipts, heads, indexes and summary lineage recover together. Preserve
source-owner semantics even if multiple owners share storage.

## Storage and crash evidence packet

For each claimed OS/filesystem/storage adapter, retain exact source revision,
OS/filesystem and mount details, storage/library versions and settings, executable
fixture commands, fault locations, observed disk states, recovery results and
limitations. A platform API branch or passing mock is not native durability proof.
No macOS, Windows or Linux hierarchy slice is qualified by this inspection.

The packet must specify file/parent-directory flush and replacement semantics
where applicable, checksummed transaction identity and integrity checks, replay
idempotence, finite transaction/log/checkpoint byte and work limits, and bounded
rotation. Summary row/workspace caps do not bound journals or staging by themselves.
Qualify insufficient space, permission/I/O/flush failures and unsupported storage
without silently weakening durability. Process-kill tests establish less than
power-loss durability; state which failure model the native evidence covers.

| Required interruption or race | Observable acceptance oracle |
| --- | --- |
| Staging or validation fails before commit | Complete old state or reconciling; no accepted new head/receipt or tentative summary |
| Durable commit succeeds before some heads/rows/indexes are installed | Reads remain unavailable until the committed source, receipts, dependency fences and ordered lineage recover coherently |
| Installation succeeds before acknowledgement/event | Exact retry resolves the original receipt; repeated recovery adds no summary transition or same-lifetime snapshot increment |
| Two required corrections commit while summary is unavailable | A→B→A replays as two material increments; final-byte equality cannot collapse history |
| Journal/row/checkpoint is truncated, missing or has wrong identity/digest | Recover only with complete ordered proof; otherwise unavailable, never reconstruct a current pass from latest objects |
| Summary full, log/staging full, flush denied, or revision exhausted | Preserve the accepted quota/exhaustion behavior; reject a source change if its required durable fence cannot commit |
| Rotation/checkpoint crashes before, during or after old-log removal | Retain sufficient ordered proof at every acknowledged boundary; repeated restart cannot reset a revision |
| Report correction races dependent evaluation, purge or dependency insertion | Serialize/revalidate complete membership and authority; no dangling current pass or unprotected deletion |
| Ordinary attention/catalog/runtime change races overview or detail read | Capture validation retries/fails within existing budgets; no mixed snapshot or implicit detail retarget |
| Replacement sidecar starts before recovery finishes | New volatile snapshot scope, fenced reads and recovered durable revision; no restored launch/report bearer |

Existing [`coordinator_tests`](../../crates/orkworksd/src/taskmaster/coordinator_tests.rs)
include `activation_retry_reflushes_record_after_post_publication_sync_failure`
and `coordinator_store_recovers_abandoned_temp_without_publishing_it`.
[`RecommendationStore` tests](../../crates/orkworksd/src/taskmaster/store.rs)
include `fault_points_recover_to_a_complete_old_or_new_graph`. These are inspected
fixture precedents only: they were not run for this register and cannot qualify
hierarchy cross-owner transactions, crash lineage, native power loss or every
platform. Extend or reuse their methods only through an approved runtime plan.

## Handoff disposition

This register delivers the baseline seam inventory and concrete qualification
oracles. SQLite/private-worker DELETE/EXTRA is now the accepted direction for
further design/qualification only. Complete writer coverage, detailed contracts
and numerical limits, native crash evidence and producer acceptance remain open.
The next bounded deliverable is the caller/direct-write inventory and strict
schema/publication-budget specification needed to propose a qualification plan.
Detailed design/default acceptance is still required before dependent runtime
code. Qualification execution needs its own scoped approved
plan; these tables are not executed tests or that plan. Keep #744/#746 open.

A [candidate transaction/storage design](../superpowers/specs/2026-10-23-hierarchy-transaction-storage-design.md)
records the accepted direction, proposed writer routes and finite limits. See its
[acceptance boundary](../superpowers/specs/2026-10-23-hierarchy-transaction-storage-design.md#scoped-architecture-direction-acceptance--2026-10-24);
no qualification gap is closed by that acceptance.

## SQLite design review follow-up — 2026-10-24

The [architecture review](../superpowers/specs/2026-10-23-hierarchy-transaction-storage-design.md#architecture-owner-review-handoff--2026-10-24)
remains static evidence, with every platform unqualified. The pinned dependency
and available PRAGMAs do not qualify hard active-journal, aggregate scratch or
logical-row work limits. Include auxiliary file classes, effective wrapper/API
features, cooperative SQL-deadline interval/overshoot and ordinary SQLite-user
compatibility in the future native packet.

Before code, also fix the exact authority/commit ordering across revocation and
SQL/fsync without blocking ordinary controls. Client wait cancellation cannot
stand in for that ordering or prove rollback. Required synthetic additions are:

| Case | Required observable outcome |
| --- | --- |
| Queued caller disconnects before worker start | Waiter detaches; source authority and exact receipt/predecessor rules still decide the command, with no invented acceptance |
| Active caller cancels before/after durable commit | Keep the worker/fence until proven disposition; a committed receipt survives response loss; no replacement worker or blind retry |
| Runtime revocation races a listed #744 claim/commit | Revoke-first denies the claim; claim-first may finish only that exact allowed attempted transaction, with no surviving bearer. Verify the accepted amendment's binding/response/fence rules; no queue exemption |
| Lease loss or excluded-action revocation races commit | No claim-based exemption; retain its own cutoff/ownership no-go until qualified. An interrupt/precheck cannot prove denial |
| Ordinary generation overflows | Terminal unavailable lifetime, no token reuse; ordinary controls remain usable and fresh scope requires recapture/fencing |
| Statement journal or other auxiliary spill is attempted | Qualified adapter enforces the allowed class/budget or rejects; `temp_store` alone is not evidence |
| Interrupt deadline is reached during prepare/step or near commit | Observe qualified cancellation interval and reconcile actual commit/rollback; no hard response-time claim from the interrupt request |

These are future qualification requirements, not executed tests or an approved
probe/implementation plan. The architecture direction is now accepted only for
further design/qualification; detailed default acceptance, artifact/payload
placement, producer agreement, #740 eligibility and all existing custody/runtime
gates remain independent.


## Caller and strict-row preparation — 2026-10-24

The [detailed storage preparation](../superpowers/specs/2026-10-24-hierarchy-storage-contract-design.md)
traces ordinary callers/direct writes at `0ed5de99`, proposes strict storage
metadata and nonrecursive commit commitments, supplies two synthetic byte/hash
vectors and calculates complete-command budget exclusions. It explicitly does
not certify complete external-producer coverage, all domain row schemas or an
eligible maximum profile. Its authority/commit counterexample rejects a volatile
precheck as proof of revocation-before-persistence denial.

Retain that race in the future ordering packet alongside source-health, quota,
proof-retention and missing-registry fixtures. A hidden committed write followed
by a fence or compensating deletion does not establish the former cutoff.
For the later accepted listed #744 class, verify the exact claim-first/revoke-first
oracle instead; all excluded actions retain their own cutoff and open gates.
No SQL/native fixture ran; every platform remains unqualified. Complete domain
schemas, exact profile budgets, qualified ordering and enforcement evidence plus
a separately approved execution plan remain necessary before runtime work.


## Authority-ordering proposal — 2026-10-24

The [single-operation claim amendment](../superpowers/specs/2026-10-24-hierarchy-authority-ordering-design.md#owner-acceptance-and-handoff--2026-10-24)
is owner-accepted for further design/qualification, including queued denial,
claim-first completion for only its listed #744 publications, namespace/finish
conflicts, one retired worker/lease, actual-ownership-loss no-go and no authority
restoration after crash. This explicitly changes the scoped policy; it does not
reinterpret a precheck as physical-commit denial. Research reports and every
excluded action retain their own cutoff and gates. Every platform remains
unqualified; no runtime/native/model probe or executable plan is delivered.
Proof-schema, action-owner, writer, lease-lifetime and native durability/resource
qualification requirements remain open. Next bounded design work is strict
nonsecret claim-proof metadata and versioned storage commitment mapping.


## Claim-proof schema preparation — 2026-10-25

The [proposed claim-proof and format-2 contract](../superpowers/specs/2026-10-25-hierarchy-claim-proof-design.md)
adds strict receipt/source/claim/commit validation, hash-cycle exclusions, slot
charging, immutable retention and ASCII encoding vectors. Its fixtures are
written expectations and local hash calculations, not SQL or native evidence.
A proof is not independent evidence that the runtime latch preceded revocation,
and cannot restore a claim/grant. Keep native ordering/lease/resource proof and
all excluded-action gates open. The schema/version decisions, complete non-ASCII/
maximum variants and exact combined slot/command budgets need separate review;
no migration, eligible profile, probe or executable plan is supplied.
