---
type: Design
title: SQL recommendation ownership on reopen
description: Accepted cutover-evidence refusal and private SQL-owned recommendation opening across owner loss.
tags: [orkworks, sqlite, recommendations, startup, recovery]
status: stable
workflow_status: accepted
---

# SQL recommendation ownership on reopen

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243), with
production startup/migration owned by [#1244](https://github.com/Rambolarsen/orkworks/issues/1244).
Inspection base: `4e5215db38a3bb7085dc4fa3ce4ca2f1ed4604fe` (#1534).
Owner accepted this concrete design on 2026-11-08 by answering “keep going”
to the explicit implementation approval request after #1537. This accepts the
private prerequisite, not production SQL selection, migration or live recovery.

## Outcome

After all recommendation owners disappear, retained `format.json` or
`session-store` evidence must prevent a fresh legacy recommendation constructor from recovering or publishing
files. A private, explicitly selected SQL workspace must establish revoked
legacy recommendation authority before exposing its recommendation owner, even
with zero pending deletion jobs. Pending jobs remain discoverable and retryable
through existing exact-job admission; opening alone never grants file cleanup.

Reuse the accepted [single-authority activation rule](2026-11-01-sqlite-session-storage-design.md#migration-and-single-authority-activation)
and [owner-lifetime revocation](2026-11-08-sql-recommendation-producer-revocation.md#lifetime-reopen-and-recovery-boundary).
Do not add another marker, schema version, strong owner registry, session
blacklist or implicit database discovery that enables SQL operation.

## Inspected owners and missing boundary

- [Recommendation open](../../../crates/orkworksd/src/taskmaster/store.rs)
  resolves the root and claims the process-local owner before mkdir, migration
  and recovery. It does not inspect storage-format evidence.
- [Operation registry](../../../crates/orkworksd/src/taskmaster/store/operation.rs)
  retains weak canonical-root entries. A fresh owner starts Legacy after the
  last reference drops; extending its lifetime is not restart protection.
- [Recovery binding](../../../crates/orkworksd/src/taskmaster/store/cleanup/removal/journal.rs)
  can bind an existing root and lease without constructor effects. Binding is
  identification, not permission to revoke or recover an arbitrary job.
- [SQL open](../../../crates/orkworksd/src/session_store/worker.rs) opens
  `session-store/v1.sqlite` without creation. Database open validates the schema,
  workspace, epoch and integrity; initialization is a separate staging path.
- [Discovery](../../../crates/orkworksd/src/session_store/deletion_recovery.rs)
  provides bounded keyset pages (1–100 jobs, at most 256 KiB), and exact creation/
  epoch/workspace-bound records. Neither is cleanup authority.
- [Workspace opening](../../../crates/orkworksd/src/session_application.rs)
  currently performs legacy migration, normalization, recommendation open and
  orphan scrubbing. Production SQL routing cannot pass through that legacy
  preparation and then switch storage owners afterward.

The [#1534 negative test](../../../crates/orkworksd/src/session_application/sql_writes/delete_recommendation_tests/revocation.rs)
`sql_cleanup_progress_failure_then_last_owner_loss_is_explicitly_not_fenced`
establishes file success, failed SQL progress, recommendation-owner loss and
successful fresh Legacy publication while the SQL worker remains available. This design changes the desired
result only for roots retaining the SQL evidence below. It does not claim that
removing every durable authority artifact can be detected.

## Legacy constructor refusal

Before mkdir, recommendation filename migration, journal recovery, graph reads
or any recommendation write, legacy `RecommendationStore::open` must inspect the
resolved metadata root for either `format.json` or `session-store`. Use
non-following entry inspection: any entry type, including empty directories,
regular files, malformed data and dangling symlinks, counts as evidence. Only
proven NotFound for both names permits this check to succeed. Permission or
other inspection errors are unavailable failures, never absence.

Evidence refuses legacy opening regardless of marker validity, job count or
whether the database can be opened. Return a typed SQL-authority/recovery
failure and preserve all bytes; do not parse a marker to grant a legacy bypass,
create a database, remove staging, replay a journal or report an empty store.
Busy precedence remains: claim the existing nonblocking owner first, then
inspect under that operation before effects. No filesystem I/O belongs inside
the short owner-registry lock or atomic transition.

This is a recommendation-layer refusal floor for the two already specified
names. The eventual workspace authority resolver must also account for every
owned staging/cutover name before production startup is qualified; this slice
does not invent those unpublished representations. Independent assessment
access uses the same legacy constructor and receives the same refusal.

Supported writers hold the workspace lease across format publication and
workspace use. Concurrent arbitrary external artifact replacement and old
unmodified binaries remain unsupported. This check does not promise a general
symlink-race defense or physical power-loss qualification.

## Private SQL-owned opening

Provide one private application-owned opening path for copied-fixture/source
qualification, separate from production `open_workspace`. It opens an existing
SQL store, retains the original workspace lease and root identity, validates the
SQL header through its worker, and binds the original recommendation directory
without invoking the legacy constructor. The binding must validate the lease
against the captured root; absent roots or unsupported binding platforms fail
without mkdir. Revalidate retained root/lease/store identity before adoption.

After those checks, obtain the same nonblocking recommendation operation and
monotonically revoke legacy I/O before publishing the returned owner. This
startup admission is a distinct opaque one-shot proof, privately minted by the
checked opening owner. It is not a callback, public boolean or deserializable
capability, and cannot mint the exact-job cleanup admission proof from #1534.
Reuse the original mode/operation machinery; do not release/reacquire between
claim and revocation. Existing Busy or mismatched ownership rejects adoption.
An unsuccessful check before revocation has no recommendation effects. Once
revoked, later startup failure does not reset a retained owner to Legacy.

All SQL-owned opens revoke, including zero jobs or jobs with every progress bit
set. An empty discovery page is never permission to activate legacy producers.
No pending-job scan is needed to choose the mode. Retain the owner and lease
through the adopted private workspace lifetime and every admitted retry.
Caller cancellation must not orphan an admitted opening task: a retained task
owns any in-progress worker/binding until its result is settled and safely
released. It must not publish a replacement runtime after a workspace switch.

Opening cannot recover legacy journals, normalize recommendation graphs,
scrub orphans from JSON membership, create recommendations or mark cleanup bits.
Use the existing no-effects recovery binding where possible; do not enlarge it
into a general legacy bypass. Legacy Taskmaster operations stay visibly
unavailable until SQL-aware producers/actions are independently qualified.

This private opening path does not validate a production format-2 activation
record or grant maintenance authority. It must have no production/HTTP caller.
#1244 must supply the qualified marker parser, complete evidence resolver and
pre-legacy routing before adopting it. Database existence alone never selects
this path. Production JSON behavior on evidence-free roots remains unchanged.

## Pending jobs and restart settlement

After private SQL ownership is established, bounded discovery may list pending
jobs without file effects. Each selected retry rereads the exact job/header and
retains its original SQL store, lease, application workspace/runtime/reservation
and recommendation owner through existing cleanup admission, v3 recovery,
fresh inspection, receipt and definitive progress. A discovery page, copied ID
or journal directory cannot grant a retry. Reused creations, foreign epochs,
missing jobs or unavailable SQL fail closed.

Keep stage progress semantics: only successful file completion ORs its existing
bit; neither opening nor discovery sets bits or finalizes a row/job. The recovery
case with no remaining recommendation transaction directory must still deny
Legacy and allow a freshly admitted exact pending job to re-inspect the graph.
An all-file-effects-complete job with failed progress is not inferred complete.
Unknown/foreign/corrupt journals remain preserved and visibly unavailable.

The private opener may return ownership before retry completion: legacy
operations are already revoked, and each retry has its own bounded admission.
No automatic background retry loop, eager unbounded scan, new runtime adoption,
finalizer, deletion API or event/replay/image/settings recovery policy is added.
Complete startup recovery of those owners remains with #1244.

## Qualification and limits

Required copied-fixture tests: unchanged evidence-free legacy open; each
artifact alone, malformed/unreadable evidence and dangling entries refusing
before effects; existing Busy precedence; aliases and independent assessment
opens; original admitted owner loss after file success/progress failure, then
fresh legacy refusal and private SQL reopen/exact-job retry; zero jobs and
all-bits-set jobs remaining SQL-owned; foreign epoch/root/lease rejection;
cancellation, worker failure and detached opening; free terminal-critical guards
while worker/binding is stalled. Preserve existing journal caps and supported
platform refusals. Use raw-file evidence for intentionally unavailable legacy
reads, not a test-only production bypass.

Implementation evidence is recorded in the linked plan and implementation PR. This
closes only the recommendation constructor/owner-loss prerequisite for retained
known SQL artifacts and the private opening route. Activation still requires
complete workspace evidence resolution, pre-legacy routing, SQL-aware producers/
assessment provenance, safe finalization, all external recovery owners, complete
backup/migration, platform durability and existing native/hierarchy gates.

## Accepted decision — 2026-11-08

Accept the two-name legacy refusal floor and separate checked SQL-owned private
opener for source-only implementation/qualification. Keep production startup and
activation blocked until #1244 consumes the complete accepted format contract.
The accepted cost is deliberate unavailability of legacy recommendation tools
beside SQL or ambiguous cutover evidence; no automatic repair or fallback.
