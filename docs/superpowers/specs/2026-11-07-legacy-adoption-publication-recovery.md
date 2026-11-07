---
type: Contract
title: Legacy adoption publication and interrupted recovery
description: Conditional effect ordering, ownership and restore-first recovery for accepted additive legacy adoption.
tags: [orkworks, sessions, migration, publication, recovery]
status: stable
workflow_status: accepted-for-qualification
---

# Legacy adoption publication and interrupted recovery

Tracking: [#1510](https://github.com/Rambolarsen/orkworks/issues/1510),
[#1244](https://github.com/Rambolarsen/orkworks/issues/1244).
Consumes the accepted [additive policy](2026-11-07-legacy-source-adoption-amendment.md),
[logical inventory/confirmation v1](2026-11-07-legacy-adoption-inventory-approval.md)
and [pinned evidence-owner audit](../../validation/legacy-adoption-evidence-owners.md).
This is step 2's **owner-accepted conditional contract for further qualification**,
not a qualified producer, implementation plan, probe budget or live repair
permission. No accepted v1 bytes, current-global precedence or format-2 activation
rule is changed.

## Accepted scope choices

1. **Restore-first after owner loss or ambiguity.** The initial publication
   profile does not reconstruct a lost approval/publication claim or roll forward
   an interrupted pre-activation attempt. Qualified same-version backup recovery
   must resolve the old cutover before a new v1 attempt is reviewed. Within an
   uninterrupted live claim, only fully evidenced ordered steps may continue.
2. **Existing final-payload parents.** All destination payload parents must be
   captured, qualified existing directories in the approved baseline. This
   version creates no new external payload directories. A missing parent blocks
   adoption; any separately authorized preparation changes the baseline and
   requires fresh inventory and confirmation.
3. **Permit-first individual effects.** Cancellation ordered before an effect's
   admission prevents it. Cancellation after admission allows that single effect
   to settle and be reported accurately, but prevents the next effect. If the
   admitted effect publishes a valid format-2 marker, SQL becomes authoritative
   even if cancellation arrives meanwhile. There is no rollback promise.

After #1512, the owner answered “keep going” to the explicit request to approve
these three choices for further qualification; [#1514](https://github.com/Rambolarsen/orkworks/issues/1514)
records that acceptance on 2026-11-07. The unchanged conditional representation
is accepted within this scope. Reuse the policy/v1 and publication/recovery
approvals; actual producer, physical-accounting and dependency qualification
remain required before dependent code.

## Required producing owners

All required evidence must be issued and revalidated by its actual closed owner
profile. It must bind the attempt and immutable intent/plan as appropriate,
native keys, inputs and outcome;
retained bytes/digests are data, never proof that the operation was authorized.
The audit established private building blocks, **not** the profiles below.

| Owner | Required guarantee before any corresponding effect |
| --- | --- |
| Maintenance/application | Qualified two-root writer quiescence, lease/context ownership, one consumed v1 approval and current cancellation/admission ordering. Restart loses this claim; no stored handle may recreate it. |
| Backup/restore (#1250) | Verified complete source/global backup sets and exclusions, current validity, a separately authorized restore operation that preserves current data before replacement, and verified recovery completion. A pre-cutover backup cannot preserve later SQL writes. |
| Native/private-control | Exact captured roots/parents, exclusive private namespace creation, complete enumeration, immutable ordered progress publication and qualified file/directory flush semantics. A partial/missing control record is not proof of no effects. |
| Payload owner | Approved same-creation association, captured staged bytes, exact destination absence, exclusive creation/publication and durable outcome with native identity. No check-then-copy overwrite or pathname/hash ownership guess. |
| SQL/format owner | Qualified sealed import-only database identity, schema/epoch/import binding, actual change counter, complete record/reference validation, exclusive database publication, and the existing atomic marker publication/directory-flush contract. |
| Resource owner | Finite complete retained/transient/callback bounds and reservations before allocation/copy/publication, held until physical settlement. The report-page cap, free-space observation and another owner's allowance are not admission. |

An unavailable, expired, unsupported or ambiguous owner result is `Unknown` and
blocks that effect. No `known` profile, generic restore primitive, portable atomic
identity operation, physical fit or native execution result is supplied here.
#1242/#1243/#1248/#1250 and native/hierarchy/rollout gates remain binding.

## Immutable intent and durable progress

Use v1's exact `C`, `H`, `Attempt`, `U`, `D`, native/profile/reference types and
retained full manifest bodies. New `las.publication-*` tags are a separate
publication representation; they do not extend v1's closed Receipt-kind enum.

The target set is derived **only** from v1 payload candidates whose decision is
`adopt`, in candidate ordinal order. Source-only metadata is added to staged SQL;
no global format-1 metadata is replaced. `Targets = ["las.publication-targets.v1",
H(Batch),[candidateOrdinalU,...]]` may be encoded/hashed as a stream. Its count N
must equal the selected payload count and every full candidate/source/association/
absence binding must validate. A target list from a caller or summary is invalid.

`Join = H(["las.publication-import.v1",baseImportDigestD,H(Batch),H(Snapshot),H(Targets)])`
binds the ordinary complete global import and selected additions without a hash
cycle. The qualified existing SQL/format owner binds this import digest in its
existing database/marker fields; this contract adds no format.json field or
activation point. No raw database hash substitutes for the owner's import-only
identity/change-counter validation.

Before cue creation, derive
`Intent = H(["las.publication-intent.v1",Attempt,H(Batch),H(Snapshot),baseImportDigestD,H(Targets),sqlIntent])`.
Control/resource admission binds this intent and planned roles. The control
constructor additionally binds the freshly created cue's native identity.
These pre-Plan bodies must **not** bind H(Plan), which would create a digest
cycle. Later effect evidence binds the actual H(Plan) and exact phase target.

After the cutover cue is created and bound, retain exactly:

```text
Plan = ["las.publication-plan.v1",Attempt,H(Batch),H(Snapshot),
        baseImportDigestD,H(Targets),cueNative,sqlIntent,controlBinding,resourceBinding]
sqlIntent = ["las.publication-sql.v1",workspaceScopeD,databaseEpochX,schemaU,Join]
Record = ["las.publication-progress.v1",H(Plan),sequenceU,previousRecordDOrNull,
          phase,payloadCandidateOrdinalOrNull,evidenceSetOrNull,reasonOrNull]
```

SQL publication destination is the existing `session-store/v1.sqlite`; the marker
is the existing root `format.json`. Private staging/control names and native
ownership must be supplied by the qualified control profile beneath the captured
`session-store/` cue; no arbitrary source/caller path is accepted. `sqlIntent`
values come from the SQL owner, not client counters or made-up zero defaults.

`controlBinding`, `resourceBinding` and effect evidence are full-body references
of the form `["las.publication-evidence.v1",kind,Profile,idX,bodyDigestD]` or v1
`Unknown`; kind is `control`, `resource`, `sql`, `payload`, `marker`,
`backup-current`, `maintenance-current` or `restore`.
`evidenceSetOrNull` is null or a sorted, duplicate-free array of these full-body
references, ordered by kind, Profile, then idX byte order. Conflicting bodies
under the same owner/profile/id reject the record. These distinct current-input
bindings are mandatory, not aliases for control/restore evidence:

- `backup-current` is issued by the backup owner. Its closed body binds the Intent
  (before Plan) or H(Plan), current effect/phase, **both exact original v1 backup
  Receipts and their roots/inventories**, validation generation and actual current
  complete-set/restore-point validity. Original snapshot receipt presence alone
  is not current validation; Unknown on either set blocks admission. This grants
  no restore operation authority.
- `maintenance-current` is issued by the qualified maintenance/application owner.
  Its closed body binds the subject Intent/Plan, owner instance/Attempt, consumed
  v1 batch, current quiescence/application Receipts, the complete relevant current
  protection/revision evidence and operation/admission serial. It distinguishes
  `new-effect-admitted` from `settlement-only` and stopped/invalid state. The
  producer must validate the actual live claim and serialized cancellation gate;
  stored audit bytes, receipt IDs or agent capabilities cannot instantiate it.
  Settlement-only evidence can record an already admitted effect after cancel;
  it cannot admit another payload, SQL or marker effect.

Prepared/sealed and every effect admission require fresh backup-current and
maintenance-current validation, in addition to their separate resource/control/
staged or publication evidence. Completed-effect records retain the exact
admission binding and current settlement result; no new-effect permission is
inferred from a done record. Initial cue admission uses Intent-bound versions
because Plan does not yet exist; later record bodies use H(Plan), avoiding the
pre-Plan digest cycle. `restore` belongs only to #1250's separately authorized
recovery operation and cannot substitute for either current-input binding. Each phase requires all its
separate producing owners' bindings, not one SQL receipt claiming payload or
application authority. The closed owner bodies must prove the table above and
the exact phase facts below. Their actual codecs/native guarantees are **required
unavailable profile inputs** until qualified; these carriers alone are not known
receipts. Retain and validate every
referenced body and native constructor/operation claim, not just the digest.

Records start at sequence `"0"` with null predecessor and `prepared` phase. Later
sequences are contiguous checked U with exact predecessor digest; unknown tags,
missing/duplicate/forked records, missing bodies, counter overflow or malformed/
partial publication stop the attempt. Each phase transition is written once.
No automatic retry of a failed effect/control write, skipped target or rebuilt
journal is allowed. A malformed/unknown tail cannot receive a synthetic stopped
successor; report unavailable externally and preserve it for qualified recovery. Retain all successfully committed records and any partial
objects for qualified recovery; never truncate a tail to fabricate success.

The progress record describes an effect; it never grants authority to repeat it.
Only the live maintenance/import owner with the consumed current v1 approval can
admit subsequent effects. A durable plan/nonce/native ID is not a restored permit.

## Bootstrap and revalidation

Before creating the cue: complete and verify both backups and captured manifests,
review all three v1 report phases, consume approval, and reserve all required
resources. The baseline must have no existing format-2/cutover evidence and every
final payload parent must be qualified. If any condition is Unknown, do not create
staging, a payload or a database.

Creating the fixed `session-store/` cue is the first global cutover effect. Use
exclusive creation under the captured root and durably establish the cue/parent
before any final publication. Ordinary opening must block on that evidence even
if the first Plan/Record write never completes. A crash leaving an empty or
partially described cue is **unknown cutover state**, not permission to remove it
or reopen legacy JSON. Native flush/discovery guarantees must be qualified before
this sequence is implemented.

Validate/write/flush Plan, retained full v1 bodies and record zero before building
staged SQL or publishing final payloads. Full control/capture files must have
qualified ownership; their bytes are included in accounting even when not user
payload. This contract does not claim a filesystem-atomic directory-plus-manifest
bootstrap. Before record zero, any partial creation takes the stop path.

At every later admission, revalidate source and global baselines, current backup/
quiescence/protection, the full v1 decisions and the live claim. Expected global
state is **unchanged baseline + exactly this attempt's evidenced private objects
and published additions**. Compare actual full entries/identities, not only
hashes. No blanket exclusion for `session-store/`, temporary names or matching
content is allowed. Unknown or foreign additions/replacements invalidate the
attempt. Control records do not hash their own envelope/identity: each record
hash is external and later records link predecessors; the current owner's
qualified live bindings validate its just-created control objects.

## Effect order and state obligations

All effect admissions and cancellation use the owning operation's serialized
ordering. Admission retains maintenance/resources through physical completion and
truthful progress settlement, even if the caller cancels. No claim of OS atomicity
between cancellation and filesystem effects is made.

| Phase/transition | Preconditions and effect | Required durable evidence before next admission |
| --- | --- | --- |
| `prepared` | Qualified cue/Plan/full inputs and record zero exist; no final target changed. | Exact Plan, bootstrap ownership, backups and current live approval claim. |
| `sealed` | Build the complete ordinary import plus approved metadata additions and private payload staging; validate all records/organization/references, bytes and counts. Flush/close the staged database and payloads. | SQL owner seal with exact epoch/schema/Join and actual import-only change counter; every selected payload's staged native identity/bytes and full private-object inventory. |
| `payload-intent(i)` | i is the next selected payload ordinal; revalidate association, staged bytes, qualified parent/absence and live resources. Commit/flush intent **before** final namespace effect. | Exact target/source/stage/parent binding. No target publication can precede it. |
| `payload-done(i)` | Admit one exclusive publication. Only an owner-proven durable publication of exactly those bytes/identity can be recorded done. | Native outcome and required file/parent flush evidence. Present/changed destination, uncertain durability or failed acknowledgement stops; never infer “already ours” by path/hash. |
| `sql-intent` | Every selected payload has exact durable done evidence; no unresolved effect. Recheck baselines plus additions and SQL seal; record intent. | Complete ordered payload prefix, sealed unchanged import-only database and verified current owners. |
| `sql-done` | Qualified exclusive publication of `session-store/v1.sqlite`; do not replace an existing database. | Exact published SQL identity/epoch/schema/Join/change counter and required directory flush. This is **not activation**; normal open still blocks without a valid marker. |
| `marker-intent` | All prior evidence is complete/current; prepare the existing marker binding the published SQL import. Commit/flush intent, then order one marker-effect admission against cancellation. | Current SQL/metadata/payload/reference integrity, current owners and exact marker image; no data still pending. |
| `marker-seen` | The admitted existing marker publication and directory flush have a qualified successful outcome. | Valid format-2 marker and matching database. **That existing marker is the activation point**, regardless of whether this final progress acknowledgement is later lost. |
| `stopped` | An error, cancellation, lost claim/ownership/quiescence, stale input or Unknown result prevents the next effect. Record truth if the control owner can safely do so. | No rollback assertion. If stop-record publication fails, unresolved evidence still blocks normal open. |

`sealed`, `sql-*`, `marker-*` and `stopped` use null payload ordinal. Payload phases
use the exact selected candidate ordinal; each intent/done pair must be adjacent
and increasing in the target set. An evidence set containing every required producing-owner reference is mandatory
for prepared/sealed and all intent/done/seen phases. Required staged/current
owner evidence accompanies intents; completed effects require actual durable
outcome evidence. Stopped may use null evidence only to report inability to
prove an outcome, never to prove absence or authorize cleanup. Reason is null
except stopped, whose closed outcome is `cancelled`, `stale`, `owner-lost`,
`effect-unknown`, `effect-failed`, `control-failed`, `resource-unavailable` or
`invalid-state`. Unknown/failed evidence cannot be written as a successful phase.

After cancellation wins before an admission, no next effect begins. If a payload
admission won first, settle that one effect then stop; do not begin SQL/marker.
If marker admission won first, settle/check the marker result. A valid marker
selects SQL; report activated rather than cancelled-with-rollback. A missing or
invalid marker remains stopped/unavailable. Never respond durable success before
the qualified marker outcome and required data validation. A valid matching
format-2 marker takes authority precedence over an incomplete or stopped progress
record; such a record cannot switch authority back to JSON.

## Interruption and recovery matrix

This matrix gives **logical oracles under qualified owners**, not executed crash
or platform evidence. Current missing profiles therefore yield stop/unavailable.
An owner-loss restart never reconstructs the old v1 handle or publication claim.

| Observed boundary/state | Required authority/result and next action |
| --- | --- |
| Before cue; incomplete backup/capture or confirmation | No adoption effects; no success. Existing format-1 opening follows its existing rules only if qualified inspection establishes no cutover evidence. A partial backup is not a valid restore set. |
| Cue created, Plan/record zero missing, malformed or not durably known | Block normal open. Unknown state; no assumed no-effects cleanup or replay. Qualified restore/repair owner must resolve it. |
| Prepared/sealed, no final payload yet; caller cancels or record fails | Stop; retain private evidence. No source/global metadata rollback is needed, but cue removal still requires qualified current ownership and separate recovery authority, never a pathname guess. |
| Payload intent committed; crash/error before a durable done record | Outcome unknown, whether final entry is absent, matches bytes or appears present. Do not repeat create, adopt a foreign entry or synthesize done. Stop for qualified recovery. |
| Some payloads have done records; later payload fails | Keep baseline/current data and truthful partial-effect evidence; no next payload, SQL or marker admission. SQL is not activated; normal open remains blocked by cutover evidence. |
| All payloads done, SQL not yet published | Live uninterrupted claim can follow sql-intent only after full revalidation. After cancellation/owner loss, stop; prior done records do not restore a claim. |
| SQL publication admitted; no durable SQL-done acknowledgement | Stop/unknown even if the filename exists. No second SQL publication, overwrite or JSON fallback. |
| Published import-only SQL, no valid marker | Not active. Live uninterrupted claim may admit marker only through the ordered gate; lost/cancelled/unknown claim uses restore-first recovery. |
| Marker admission wins cancellation; marker outcome unsettled | Retain owners/resources through settlement. Report pending/unavailable until qualified outcome; do not say the cutover rolled back. |
| Valid matching marker survives, marker-seen/response is lost | Existing qualified format-2 startup validates/selects SQL; do not replay adoption or require an old claim for already-established authority. Retained progress is evidence only. |
| Marker absent/corrupt/unknown with any final/staging/cutover evidence | Stop normal open; never fall back to JSON, auto-create an empty SQL store or infer that no SQL writes occurred. |
| Valid marker but missing/corrupt/mismatched database | SQL recovery condition; stop. No fallback or old import retry. |
| Post-cutover SQL writes or their presence is unknown | Current SQL authority is retained. A pre-cutover backup/old change counter cannot prove lossless recovery; require qualified current-state/same-version recovery under #1250 or remain unavailable. |
| Changed source/root/backup/protection, foreign target or unrelated new data | Stop the attempt. Preserve current/foreign evidence; no automatic removal/overwrite or use of stale backup authority. |
| Lost maintenance/application process before a valid activation | Revoke live authority. Restore-first: qualified fresh recovery resolves old evidence, then a clean new capture/Attempt and all v1 presentation/confirmation phases are required before a new adoption. |
| Repeated recovery request or old nonce/record replay | Data cannot mint approval or effect authority. No duplicate publication; the separate recovery owner validates its current operation idempotently. |

Restore is an explicitly authorized #1250 maintenance operation, **not** a right
inherited from the old adoption decision. Before replacing a destination, that
owner must capture/preserve current data and unresolved/foreign additions as
required by its actual restore contract. If it cannot preserve/validate the set,
stop; do not wipe the root to make old backups fit. After verified pre-activation
restoration, a clean eligible format-1 baseline can start a new attempt with fresh
owner confirmation. Post-cutover recovery must respect later writes and the
existing marker/authority rules; no lossless old-backup claim is made.

This restore-first profile intentionally does not implement lost-owner roll-forward
or automatic discard of staged/final additions, even when old hashes match. A
future proposal for stronger recovery needs actual ownership/approval evidence
and separate review; it cannot quietly broaden this contract. Deliberate erasure
of all format/cutover evidence by the same OS user is outside the accepted marker's
protection model; this is not same-user confinement.

## Complete accounting and implementation gates

For N selected payloads, the successful linear path has exactly `6 + 2*N` records:
prepared, sealed, N intent/done pairs, SQL intent/done and marker intent/seen.
At most one stopped record can be attempted instead of/after the failed boundary;
no automatic effect/record retries or unbounded recovery log are permitted.
These are logical counts, not physical-byte or allocation guarantees.

Before implementation, close finite reserved storage/transient bounds for all of:
original/current baseline, both verified backups, every captured full v1/receipt
body and Plan, all retained progress records/indices, private SQL and its complete
auxiliary set, staged payloads, published copies, control/staging directories,
failed partial/temporary objects and owner callback/flush/recovery scratch. Charge
all simultaneous occurrences; no hard-link/dedup/reference discount without the
qualified resource owner's explicit physical accounting. Counter overflow,
missing bounds or failed admission prevents the operation before its allocations.
No 256 KiB report cap, journal LIMIT/TOTAL, free-space probe or linked-pair budget
supplies those bounds. This contract proves no resource fit.

Conditional written design may proceed using the audited owners; runtime code
cannot while native/control, backup/restore, SQL/marker, application or resource
inputs remain unqualified. Owner acceptance supplies no missing qualification.
Before implementation, close those actual producer contracts and evidence,
then add bounded tasks to #1244's plan within #1242/#1243/#1248/#1250 and
native/hierarchy/rollout gates.
No live restore, SQL/native/model probe or extra budget is authorized here.

## Review disposition

Fresh authority-fidelity review found no material issues with the explicit new
scope choices or preservation of accepted policy/v1/marker authority. Fresh
state/effect review verified the successful-path count and found a missing
explicit current-backup/maintenance evidence route. The correction adds separate
backup-current and maintenance-current bindings, exact pre-Plan/Plan scopes and
admission-versus-settlement rights; control or restore receipts cannot stand in
for them. No physical producer, implementation or executable crash result was
promoted by these reviews.

Docs validation passed metadata/index checks, all 1,202 tests and the site/link
build. The state/effect matrix remains a conditional oracle, not crash/native
execution evidence. Actual producer and physical gates are unresolved.
