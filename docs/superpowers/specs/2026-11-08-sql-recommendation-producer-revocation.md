---
type: Design
title: SQL recommendation producer revocation
description: Accepted fail-closed legacy operation revocation before exact-job SQL recommendation cleanup.
tags: [orkworks, sqlite, recommendations, deletion, authority]
status: stable
workflow_status: accepted
---

# SQL recommendation producer revocation

Parent: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Inspection base: `02431980407875d6d55f7cd5844f84adeaf8510a` (#1524).
The owner accepted this concrete policy after #1528 by answering “go ahead”
to the explicit implementation request on 2026-11-08. Source implementation
remains dormant; no production or restart qualification is implied.
The accepted [storage design](2026-11-01-sqlite-session-storage-design.md#external-payloads-events-backup-and-deletion)
and [recovery contract](2026-11-07-sql-recommendation-journal-recovery.md#restart-and-progress)
require stale producer fencing; they do not select this revocation mechanism.

## Outcome and bounded first step

While the original shared owner remains alive, a legacy evaluator, assessment
recovery, redaction or rollback must not recreate a recommendation after an
admitted SQL cleanup releases its operation permit. This first step does not
close the complete stale-producer qualification gate.
Choose a fail-closed first step: once the original workspace owner admits SQL
recommendation cleanup, revoke its legacy recommendation operations. Exact-job
SQL cleanup may retry through its own retained permit. No legacy caller receives
a bypass merely because it holds the same directory, clone, or workspace lease.

This step intentionally makes legacy recommendation operations unavailable for
that SQL-owned workspace instance. It does not deliver a replacement SQL-aware
producer or action flow. It remains private and source-only; production JSON
routing is unchanged, and no database existence check enables SQL mode.
Qualified producer publication and startup routing must precede activation.

## Verified source paths

- [Operation owner](../../../crates/orkworksd/src/taskmaster/store/operation.rs)
  contains a nonblocking busy flag and release notification, shared by canonical
  root through weak registry entries. It has no producer freshness or SQL mode.
- [Store operations](../../../crates/orkworksd/src/taskmaster/store.rs) acquire
  permits per call. `put_current` can create a missing record. Graph transactions
  validate expected file hashes, not current SQL session creations. Reads and
  constructors can mutate files through legacy recovery or filename migration.
- [Evaluator](../../../crates/orkworksd/src/taskmaster/evaluator.rs) can publish
  retained model results with `put` or rollup graph reconciliation.
  [SessionApplication](../../../crates/orkworksd/src/session_application.rs)
  also owns deterministic refresh, audit/packet changes and rollback writes.
- [Assessment recovery](../../../crates/orkworksd/src/taskmaster/assessment.rs)
  can upsert a retained pending recommendation when its file is absent.
  [Assessment access](../../../crates/orkworksd/src/taskmaster/assessment_access.rs)
  independently opens a store and rewrites redacted copies.
- [Delivery settlement](../../../crates/orkworksd/src/session_application/recommendation_delivery.rs)
  retries Busy while retaining its in-flight guard, with no repeated PTY input.
  A permanent revocation during that settlement must not be treated as Busy.
- [Captured cleanup](../../../crates/orkworksd/src/session_application/sql_writes/deletion.rs)
  already rereads exact SQL identity under the original session lane and retains
  application reservation/lease through file work and progress. Its store
  composite currently claims the recommendation permit after app admission.

The writer inventory for this first step is the complete public store interface,
not a hard-coded list of evaluator call sites. Every public operation that may
read, recover, publish, migrate or delete recommendation files participates.
Memory-only clone/owner comparison/capture and release notification stay usable.

## Shared owner state and failure semantics

Extend the existing canonical-root owner with one monotonic mode:
`Legacy` or `SqlCleanup`. Mode and busy admission share one short memory-only
critical section, so a legacy claim cannot race past revocation. No I/O or await
occurs under that section. Keep the existing nonblocking Busy behavior for an
operation already in progress; after its release, an otherwise eligible legacy
claim returns a distinct typed authority error in SqlCleanup mode.

There is no reset, expiry, per-session blacklist, generation history, retry timer,
strong global owner registry or new persistent marker. All independent opens
and existing clones see the same owner while it is retained. Legacy constructors
may resolve the canonical existing root before admission, but must check the
mode before mkdir, filename migration, journal recovery or graph validation.
Refactor their current create-before-claim ordering as needed; a new root in
unchanged legacy mode keeps its current initialization behavior.

Because even legacy reads can replay journals, revoke legacy reads as well as
mutations. Preserve existing files and journal evidence on authority refusal.
Return unavailable/authority failure, never an empty recommendation list or a
successful no-op. Do not silently switch a failed SQL operation back to legacy.

## Original application admission and permit handoff

The transition happens only for the Recommendations stage, after the captured
SQL identity matches the fresh exact tombstone/header and the original runtime,
workspace, reader, recommendation owner and lease checks pass. No transition at
capture, database initialization, discovery, failed pre-admission checks, expiry
or cancellation.

Under the existing short application admission guards:

1. Check that the original workspace has no recommendation delivery in flight.
   If any delivery is pending, return a visible conflict before mode changes.
   This workspace-wide rule includes unrelated target sessions; do not wait for
   a PTY acknowledgement while holding any application guard or store permit.
2. Reserve the session operation and obtain one nonblocking recommendation
   operation grant from the original bound owner. Busy changes no mode.
3. Atomically admit the existing retained worker and switch the owner to
   SqlCleanup while retaining that exact grant. An admission failure releases
   the grant without revocation. Delivery admission/guard registration uses the
   same application workspace exclusion, preventing a new delivery between the
   in-flight check and the transition.
4. Move the grant into blocking file work. Exact-job recovery, fresh inspection,
   publication, receipt, definitive SQL progress and detached-result checks must
   reuse it without release/reacquire or recursive claims.

Separate a private SQL operation grant from legacy admission. It is minted only
by this checked admission path; copying a SQL identity or calling a legacy store
method cannot mint it. Store-layer tests may construct the trusted fixture grant
inside test utilities. Do not expose a general caller-controlled mode switch.

If later file work fails, including foreign/invalid journal refusal, the mode
stays SqlCleanup. The original exact-job cleanup may retry after full current
admission and obtain a new SQL grant; legacy operations remain refused. Preserve
Busy/file/progress error distinctions and earlier cleanup bits. Caller loss after
admission retains the same grant/lease/reservation through progress. No job is
finalized by this transition.

The in-flight check is necessary because blanket revocation would otherwise
strand previously acknowledged delivery settlement. Existing legacy settlement
continues before revocation, retains its guard through Busy, and sends input at
most once. Never clear the delivery registry or relabel a permanent authority
error as Busy to make the transition pass.

## Lifetime, reopen and recovery boundary

The active workspace recommendation store and retained SQL cleanup grants keep
the shared owner alive. Independent same-root legacy opens are denied even after
the SQL receipt drops. Revocation is not cross-process or durable authority.
Once every owner reference is gone, a new registry owner can be Legacy again;
this proposal cannot prevent that reopen on its own. Concrete counterexample:
file cleanup succeeds, SQL progress fails, the workspace closes and every owner
reference drops. A fresh legacy open can then publish before the pending job is
retried. Source inspection establishes this limitation; no executed reproduction
is claimed by this docs-only proposal.

Keep this first step restricted to private fixtures/source-only calls with the
original owner retained. It must not enable live SQL cleanup or be classified as
complete producer qualification. Before any live SQL cleanup is enabled, qualified
startup/reopen routing must preserve or refuse SQL ownership for pending jobs,
including the all-file-effects-complete/progress-failed case above. A durable
fence would be a separate owner-approved design, not an implicit addition here.

Qualified SQL startup must retain its original lease, choose SQL-owned admission
before any legacy recommendation constructor, and refuse unqualified operation
routes. That remains a separate pre-activation gate. Old binaries and arbitrary
external file writers are unsupported after migration under the accepted storage
contract. No on-disk claim is made that this process-local barrier survives a
restart, backup restore, owner loss or a foreign process.

## Later SQL-aware publication qualification

A later producer path must capture exact source creation identities from SQL
before retaining work and revalidate them under the same recommendation permit
that publishes file effects. Reused IDs, tombstones, missing records, changed
store epochs/workspaces and unavailable SQL must deny publication. The complete qualification must prevent a stale publication after cleanup,
including workspace teardown/reopen and pending-job recovery; the process-local
first step alone does not establish that property.
Retain existing graph/reference semantics and original lease/runtime authority.
Do not hold terminal-critical guards while awaiting SQL or file I/O.

Pending assessment publications need original source provenance across restart;
current retained ID-only recommendations must not be assigned current creations
and called validated. That representation and recovery policy require a separate
concrete proposal. This first step adds no recommendation/assessment wire fields,
SQL schema, public capability or allowlisted writer bypass. Its deliberate cost
is unavailable legacy Taskmaster operations in the dormant SQL instance until
those producers/actions and startup are qualified.

## Verification and decision

Required tests before implementation can merge: unchanged legacy behavior;
retained stale put/graph/packet/rollback denial; independent same-root open denial
before effects; read recovery/migration refusal with byte preservation; Busy and
admission cancellation without accidental revocation; file/SQL failure and caller
loss after revocation; repeated exact-job SQL cleanup; in-flight acknowledged and
failed delivery blocking transition until settlement without resend; free input
and application guards while blocking file work stalls. Test distinct workspaces
and the explicitly non-durable owner-loss boundary, including progress failure
followed by owner loss/reopen as an unqualified counterexample. Preserve existing caps and
platform support; run full verification and current manual/CI reviews.

Accepted owner decision — 2026-11-08: use this monotonic process-local legacy-operation
revocation and checked permit handoff as a private prerequisite. This accepts
neither live SQL selection nor a new SQL producer, finalizer, startup recovery,
backup/migration or activation. Production continues using JSON.


## Follow-up representation proposal — 2026-11-08

The [assessment source-provenance proposal](2026-11-08-sql-assessment-source-provenance.md)
traces input dependencies even when the derived card has empty session arrays.
It proposes format4 and a bounded original-creation envelope with fail-closed
legacy handling. Owner decision is pending; this link changes no accepted
producer/publication permission or source-origin qualification.
