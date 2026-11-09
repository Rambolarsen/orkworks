---
type: Architecture Decision
title: Ordinary sessions use SQLite with bounded history reads
description: Accepted single-authority ordinary-session storage and quiescent migration, separate from hierarchy storage.
tags: [orkworks, architecture, sessions, sqlite, migration]
status: stable
---

# Ordinary sessions use SQLite with bounded history reads

- Status: accepted; linked-profile worker separation superseded by [ADR 0106](0106-joint-child-preparation.md); implementation and qualification pending
- Deciders: repository owner; approved the concrete design on 2026-11-01
- Date: 2026-11-01
- Tracking: [#1241](https://github.com/Rambolarsen/orkworks/issues/1241)
- Product contract: [session storage](../../specs/session-storage.md)
- Design: [SQLite session storage and bounded history](../superpowers/specs/2026-11-01-sqlite-session-storage-design.md)

## Context

The workspace retains hundreds of ordinary session records and roughly a GiB
of event/replay files. The complete session projection feeds routine list and
push refresh; indexed metadata queries and bounded pages are useful independently
of the live-capacity scan fix in ADR 0102. The sidecar already bundles rusqlite.
Existing agents can still write JSON directly, and organization state is stored
separately; a database is not authoritative until those writers are migrated.

## Decision

Store ordinary metadata, revisions and organization in one workspace-local
SQLite database behind a private worker with typed operations. Retain metadata
JSON inside a row with transactionally maintained query columns/search indexes.
Use one connection and rollback DELETE/EXTRA journaling for short captured reads;
verify the linked accepted limits and effective database settings.

Separate live reads/updates from bounded indexed history and exact detail reads.
Preserve complete internal-domain queries, historical facts, existing authority
checks and native eligibility gates. Keep large replay/images and existing event
history external, with identity-bound recoverable cleanup and complete backups.

Perform a maintenance-only, verified import and single activation with preserved
source backups, explicit incompatible-writer/downgrade behavior and no automatic
JSON fallback. Keep current retention settings and archive protection.

This supersedes ADR 0018 only for ordinary session-file authority and watching
after qualified format-2 activation; global workspace-scoped placement remains.
ADR 0004 was already superseded by ADR 0018 and is retained as history. This also
supersedes ADR 0088's organization-file persistence mechanism and ADR 0092's
complete-all-history REST/stream projection for format-2 workspaces, preserving
their state/retention and authoritative synchronization guarantees. Existing
format-1 source behavior remains until implementation and activation. ADR 0090's
separately gated hierarchy store is not implemented or superseded.

## Consequences

Indexed history becomes naturally bounded and metadata/organization updates can
share transactions. The implementation must migrate direct producers and existing
synchronous lock-held writers, negotiate transport versions, qualify crash and
durability behavior, and handle external payload recovery explicitly.

Implementation follows the [reviewed delivery plan](../superpowers/plans/2026-11-01-sqlite-session-storage.md).
Acceptance supplies no runtime code, live migration, destructive retention, native
enablement or platform qualification.


## Linked preparation decision — 2026-11-05

[ADR 0106](0106-joint-child-preparation.md) records the owner-approved linked
transaction broker, versioned lineage and retained-tombstone direction for
further qualification. Its partial supersession changes only that linked-profile
connection/worker arrangement; this record's other authority, recovery, retention
and activation decisions remain. No implementation or source permission follows.

## Amendment — 2026-11-07: retained legacy source adoption

The owner approved the concrete [retained-source adoption policy](../superpowers/specs/2026-11-07-legacy-source-adoption-amendment.md)
after PR #1495, for ADR and qualification-plan updates. Before first format-2
activation, the existing maintenance/import owner may prepare explicitly
approved additive recovery from the selected retained workspace-local source.
Current global metadata and every existing payload take precedence and remain
unchanged. Source-only sessions and missing same-creation payloads require
explicit adopt/exclude decisions bound to inventories, verified backups and
current identity/protection; absence never proves a missed import or deletion
intent. Conflicting or unknown identity cannot be approved around.

This operationalizes preserved sources and explicit recovery under the existing
single-authority activation decision. It adds neither startup replay nor a
standalone repair engine, overwrite authority, post-activation adoption, native
resume or new retention behavior. External publication is non-atomic and must
retain exact progress and block ordinary opening through unresolved cutover.

The [qualification plan](../superpowers/plans/2026-11-07-legacy-source-adoption-qualification.md)
closes inventory/approval, publication/ownership and restart/accounting contracts
before dependent implementation. #1244/#1250 and existing producer, historical,
linked-store, native/hierarchy and rollout gates remain. No runtime implementation,
live repair, new probe budget or platform guarantee follows from policy acceptance.

### Representation acceptance — 2026-11-07

The owner subsequently approved the unchanged [v1 inventory/report/confirmation
representation](../superpowers/specs/2026-11-07-legacy-adoption-inventory-approval.md)
and its one-use local-owner confirmation ordering after #1504, for further
qualification. This supplies no native/backup/application profile, trusted
transport implementation, publication/recovery evidence or probe budget. The
[evidence-owner audit](../validation/legacy-adoption-evidence-owners.md) separates
existing private primitives from those still-required guarantees. The policy,
single activation point, current-data precedence and runtime gates above stand.

### Publication/recovery acceptance — 2026-11-07

After #1512, the owner answered “keep going” to the explicit approval request for
three [publication/recovery choices](../superpowers/specs/2026-11-07-legacy-adoption-publication-recovery.md):
restore-first after owner loss or ambiguous effects; existing final-payload
parents only; and permit-first individual-effect settlement. Cancellation can
prevent the next admission but cannot undo an already admitted effect; if that
effect publishes a valid format-2 marker, SQL is authoritative. The unchanged
conditional representation is accepted for further qualification in #1514.

This does not reconstruct lost approval, authorize rollback by pathname, qualify
physical owners or permit runtime implementation. Actual native/control,
backup/restore, SQL/marker, maintenance/application and resource guarantees,
complete physical accounting and #1242/#1243/#1248/#1250 remain prerequisites.
The accepted additive policy, v1 bytes and existing marker activation rule stand;
no new probe budget, live recovery or rollout permission follows.

## Amendment — 2026-11-08: complete backup contract acceptance

After #1518, the owner answered “keep going” to the explicit request to approve
four [backup/restore refinements](../superpowers/specs/2026-11-07-session-backup-restore-contract.md)
for further qualification: closed-store initial capture; versioned private
directory bundles with purpose-bound recoverable-workspace, retained-evidence
and preservation-only classes; complete damaged-state preservation before
replacement; and a durable opener-discoverable restore fence before replacement.
#1522 records acceptance of the unchanged conditional contract.

Only recoverable-workspace sets are eligible for workspace restore. Retained
local evidence preserves readable malformed/excluded inputs without asserting
whole-workspace recoverability; current global recovery semantics remain required.
The separate restore protocol cannot revive an interrupted adoption claim. The
existing format-2 marker remains the authority switch after qualified restoration;
the restore fence blocks mixed/incomplete opening and supplies no alternate format.

Reuse the accepted quiescent model, additive policy, v1 and publication/recovery
choices. Exact bundle/seal and owner-body codecs, supported native publication/
fence discovery, all-writer maintenance/application admission, SQL closure and
complete physical accounting remain prerequisites. This acceptance supplies no
runtime implementation, executed recovery, native/model probe budget or rollout
permission. #1250/#1244 and their dependency/qualification gates remain open.

### Backup representation acceptance — 2026-11-08

After #1527, the owner answered “go ahead” to the explicit approval request for
[bundle/evidence v1](../superpowers/specs/2026-11-08-backup-bundle-evidence-v1.md)
for further qualification. #1531 records acceptance of the unchanged private
layout, canonical stream/owner-record bytes, purpose bindings, complete owner
coverage and acyclic Intent/Content/Manifest/Seal representation. The three
written byte/hash fragments remain ineligible components, not valid backups.

This does not register a producer/native profile, validate opaque access proofs,
implement an encoder/parser, establish resource fit or grant restore authority.
All-writer maintenance, inert validation, SQL closure/auxiliary handling, actual
native/access/fence operations and complete physical accounting remain gates.
The next source qualification audits actual writer admission/drain ownership;
no new native/model execution budget or live recovery permission follows.

### Maintenance admission acceptance — 2026-11-08

After #1540, the owner answered “go ahead” to the explicit request to approve
four [maintenance choices](../superpowers/specs/2026-11-08-backup-maintenance-admission-design.md)
for further qualification: original-root work tracking including detached jobs;
quiescent runtimes without automatic stopping; outer physical settlement before
SQL close/Q; and explicit validated release/readiness, including blocked/unverified
startup rather than treating a lost volatile ledger as empty. #1541 records
acceptance of the unchanged conditional design and its
[qualification plan](../superpowers/plans/2026-11-08-backup-maintenance-qualification.md).

Atomic Q retirement, original-job continuations and serialized cancellation remain
as reviewed. The existing marker, bundle purposes and restore-first policy stand.
Actual module interfaces/registration, physical completion, inert validators,
SQL closure/reopen, native/root/external quiescence, durable restore routing and
finite resource/time enforcement remain qualification gates. No controller code,
live session stop/capture/restore, new probe budget or rollout permission follows.

### Maintenance registration and settlement acceptance — 2026-11-08

After #1549, the owner answered “go ahead” to the explicit four-choice request.
[#1552](https://github.com/Rambolarsen/orkworks/issues/1552) records acceptance of
the unchanged [registration/settlement design](../superpowers/specs/2026-11-08-maintenance-registration-settlement-design.md)
for further qualification: a fixed private producer catalog with root-specific
registrations; completion obligations retained independently of caller/execution
capability; finite original-job continuations; and generation-bound coordinated
closure/readiness. Prepared/reopened handles remain owned and gated after partial
readiness failure until actual safe teardown or qualified resolution.

Reuse this approval and existing maintenance/backup/adoption/v1 authority. Actual
owner integration, complete producer coverage, native/external/physical/resource
evidence, inert validators and restore-fence/startup readiness remain unqualified
under the [qualification plan](../superpowers/plans/2026-11-08-backup-maintenance-qualification.md).
No runtime implementation, live stop/capture/restore, native/SQL/model execution,
new numeric allowance or rollout/dependency bypass follows.

### Bounded binding and delivery settlement acceptance — 2026-11-08

After #1560, the owner answered “go ahead” to the explicit three-choice request.
[#1561](https://github.com/Rambolarsen/orkworks/issues/1561) records acceptance of
the unchanged [bounded-settlement design](../superpowers/specs/2026-11-08-maintenance-settlement-bounds-design.md)
for further qualification: complete retained resource/result ownership through
physical settlement; one absolute attempt deadline without renewal; and one
initial store claim, one availability wait and one final claim, with no input
resend. Unresolved work stays blocked for exact owner resolution. Late binding
results remain owned through safe disposal; timeout is not physical completion.

Existing accepted maintenance/registration/backup/adoption/v1 rules stand. Actual
numeric profiles/enforcement, delivery-outcome evidence, compatible owner
operations and complete physical/native/external/restore readiness remain gates
under the [qualification plan](../superpowers/plans/2026-11-08-backup-maintenance-qualification.md).
No runtime implementation, live operation, native/SQL/model execution, numerical
execution allowance or rollout/dependency bypass follows.

### Private terminal input outcome acceptance — 2026-11-09

After #1571, the owner answered “go ahead” to the explicit three-choice request.
[#1576](https://github.com/Rambolarsen/orkworks/issues/1576) records acceptance of
the unchanged [private input-outcome interface](../superpowers/specs/2026-11-08-terminal-input-outcome-interface-design.md)
for further qualification: one original submission record independent of callers;
distinct input-effect truth, physical completion and application settlement; and
rich recommendation consumption preserving partial/unknown protection without
resend. Known full byte write is not command execution, and known no input effect
is not absence of workspace effects. Panic/owner loss can drop local handles;
unknown completion/ownership remains blocked for actual resolution.

Reuse this approval with accepted maintenance/registration/bounded-settlement/
backup/adoption/v1 rules. Actual owner operations, complete numeric reservation/
deadline enforcement and physical/native/external/restore readiness remain gates
under the [qualification plan](../superpowers/plans/2026-11-08-backup-maintenance-qualification.md).
No runtime implementation, live operation, native/SQL/model execution, numerical
execution allowance or rollout/dependency bypass follows.

### Single source-only submission owner — 2026-11-09

After #1587, the owner approved a reviewed implementation plan followed by one
unwired submission-owner module with offline tests. [#1589](https://github.com/Rambolarsen/orkworks/issues/1589)
records the [accepted narrow sequencing exception](../superpowers/specs/2026-11-09-submission-owner-qualification-sequencing.md)
and its [bounded plan](../superpowers/plans/2026-11-09-input-submission-owner-candidate.md).
Candidate handles/counters establish only local record/accounting behavior. They
carry no native RootKey, input or maintenance authority; synthetic fixture limits
are not production values or measured physical fit. Actual producer wiring and
complete native/resource/backup/restore qualification remain gated. No terminal,
SQL/file/model effect, live operation or production activation follows.

## Clarification — 2026-11-07: SQL-bound recommendation recovery

After reviewing the complete [recovery protocol](../superpowers/specs/2026-11-07-sql-recommendation-journal-recovery.md)
in PR #1507, the owner answered “keep going” to the explicit implementation
request. The accepted representation binds a reserved transaction basename to
the existing database epoch/workspace/session creation before backups, carries
the full identity in a strict v3 deletion manifest, and refuses reserved journals
in legacy recovery before effects. Exact-job recovery retains the existing
operation/lane/lease through file completion and SQL progress. No SQL schema bump
or public API is selected.

This operationalizes identity-bound external deletion under the existing decision.
The [delivery plan](../superpowers/plans/2026-11-07-sql-recommendation-journal-recovery.md)
keeps producer revocation, final job completion, qualified startup routing,
backup/migration and activation gates. Approval permits source-only implementation;
it claims no executed recovery, live change or performance qualification.

## Clarification — 2026-11-08: recommendation producer revocation

After #1528, the owner approved the concrete [revocation design](../superpowers/specs/2026-11-08-sql-recommendation-producer-revocation.md)
and [plan](../superpowers/plans/2026-11-08-sql-recommendation-producer-revocation.md)
with “go ahead” in response to the explicit implementation request. The private
first step revokes legacy recommendation I/O monotonically for the original
shared owner lifetime when SQL cleanup is admitted, refuses transition while
workspace delivery is in flight, and hands the same checked permit through
file work and progress. It adds no SQL schema, marker, producer bypass or reset.

This operationalizes stale-producer fencing within the existing storage decision.
Owner loss/reopen is explicitly not fenced: successful file work followed by
failed progress and final-owner drop can admit a new Legacy owner. Live SQL
cleanup remains blocked until qualified startup/reopen handles pending-job
ownership. Complete SQL producers, durable source provenance, safe finalizer,
backup/migration and activation remain separate gates. No live change or executed
qualification is established by this acceptance.


## Clarification — 2026-11-08: recommendation ownership on reopen

The owner accepted the [reopen design](../superpowers/specs/2026-11-08-sql-recommendation-reopen.md)
and [qualification plan](../superpowers/plans/2026-11-08-sql-recommendation-reopen.md)
after #1537, replying “keep going” to the explicit implementation request.
Legacy recommendation constructors refuse retained `format.json` or
`session-store` entries before effects. A distinct checked private SQL opener
retains original root/lease/store identity and revokes legacy recommendation
I/O before adoption regardless of pending-job count. Existing exact-job cleanup
admission remains separate. No new marker/schema, public/HTTP opener or live
activation is accepted. #1244 still owns complete evidence resolution, qualified
format-2 validation and pre-legacy production startup routing; SQL-aware
producers, all external recovery owners, finalization and backup/migration gates
remain. Production stays JSON.

## Clarification — 2026-11-08: retained SQL assessment provenance

The owner accepted the [source-provenance contract](../superpowers/specs/2026-11-08-sql-assessment-source-provenance.md)
and [qualification plan](../superpowers/plans/2026-11-08-sql-assessment-source-provenance.md)
after #1546, replying “keep going” to the explicit implementation request.
Format4 binds original epoch/workspace, assessment/input identity and sorted
source creations, with 128-pair/16 KiB provenance and existing 64 KiB record limits.
Legacy/unproven sources cannot be backfilled; top-level and nested SQL fields
are rejected on legacy formats, including null. The first accepted runtime
slice is strict data, SQL-use refusal and typed worker/copied-fixture validation.
No production origin-claim constructor, observation wire upgrade, recommendation
I/O bypass, SQL publication/recovery route or activation is accepted. Actual
origin producers and publication/deletion/cleanup/action integration, qualified
startup, backup/migration and rollout remain separate gates. Production stays JSON.


## Clarification — 2026-11-08: original SQL observation capture

After #1554 the owner answered “go ahead” to the explicit request to implement
the [observation-origin contract](../superpowers/specs/2026-11-08-sql-observation-origin-capture.md)
and [qualification plan](../superpowers/plans/2026-11-08-sql-observation-origin-capture.md).
This accepts private tagged observation/tombstone records, original scope before
inference, strict bounded byte-preserving reads, canonical SQL admission/legacy
refusal and exact-job Workflow deletion ordering. Reuse the existing assessment
format4 acceptance. Execute the codec/reader prerequisite first, without a
production origin constructor, producer routing or legacy opener changes; then
qualify the captured producer/deletion owner separately. Recommendation/readiness
origins, assessment publication, complete startup, backup/migration, native/
hierarchy and activation remain gated. Production stays JSON.


### Source qualification progress — 2026-11-08

After the strict data prerequisite in #1565, the next private slice retains the
worker's original checked opening identity and implements the captured final-scan
source/read admission plus canonical shared observation mode and legacy refusal.
A distinct checked existing-SQL read opener supports owner-loss qualification
without legacy repair or AppState adoption. This follows the accepted decision;
it does not add a production format route, persist new tagged observations,
compose Workflow deletion or construct assessment inputs. Those integrations and
physical/rollout gates remain unchanged.
