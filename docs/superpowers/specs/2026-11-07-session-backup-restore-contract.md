---
type: Contract Proposal
title: Complete session backup and same-version restore
description: Conditional scope, validation and restore ownership for quiescent complete workspace recovery sets.
tags: [orkworks, sessions, backup, recovery, ownership]
status: draft
workflow_status: backup-restore-contract-proposed
---

# Complete session backup and same-version restore

Tracking: [#1517](https://github.com/Rambolarsen/orkworks/issues/1517),
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250),
[#1244](https://github.com/Rambolarsen/orkworks/issues/1244).
Consumes the accepted [storage requirements](../../../specs/session-storage.md),
[storage design](2026-11-01-sqlite-session-storage-design.md#external-payloads-events-backup-and-deletion),
[additive adoption policy](2026-11-07-legacy-source-adoption-amendment.md) and
[publication/recovery contract](2026-11-07-legacy-adoption-publication-recovery.md).
This is a written proposal; no producer, platform, complete recovery fixture,
resource fit, runtime activation or live-user operation is qualified here.

## Existing decisions and proposed refinements

Reuse accepted quiescent maintenance, complete database-plus-external-file
capture, isolated validation, destination preservation, same-version recovery
and the existing format-2 marker authority. This does not add online snapshots,
repository/native-tool backup, retention changes or process adoption.

Proposed refinements for owner review:

1. **Closed-store initial profile.** All relevant writers stop and drain; the SQL
   owner seals/closes every connection and accounts for its complete auxiliary
   set before copying. An online/open-connection backup is outside this initial
   profile. Failure to close or prove a stable complete image blocks capture.
2. **Versioned private directory bundle.** Store an immutable full manifest,
   owner validation bodies and ordinary copied files, with exclusive final
   publication and a completion seal written last. No compression, encryption,
   deduplication, hard-link sharing, remote transfer or cross-version conversion
   is supplied by this profile. Existing confidentiality rules still apply.
3. **Distinct damaged-state preservation.** Before replacement, preserve current
   destination state even if it is corrupt or mid-cutover. Such a capture is
   labelled preservation-only and cannot satisfy a complete-backup receipt.
   If current bytes cannot be preserved completely, replacement is blocked.
4. **Durable restore fence before replacement.** A separate qualified restore
   protocol must block every supported opener across crash/restart before any
   destination replacement, including while an old valid format-2 marker remains.
   This is a maintenance fence; it does not replace marker authority.

The exact serialized codecs, supported native profiles and numerical resource
bounds are later qualification inputs, not builder defaults. Acceptance of
these logical refinements would not qualify those inputs or authorize code.

## Ownership and source evidence

Inspected source: `8e01b409e41d761341fb181f7dc27082615645b5`, 2026-11-07.
Source was read only; no SQL, native, product test or live capture was executed.
The earlier [evidence-owner audit](../../validation/legacy-adoption-evidence-owners.md)
is retained as historical evidence, not relabelled as current execution.

| Owner/source | Available primitive; remaining required guarantee |
| --- | --- |
| [WorkspaceLease](../../../crates/orkworksd/src/workspace_runtime.rs), lines 162–218 | Advisory lease and Unix lock-inode validation. Needs complete writer admission/drain and supported root/identity/durability profiles; a lock filename or no managed runtime is insufficient. |
| [Private SQL adapter](../../../crates/orkworksd/src/session_store.rs), lines 94–150; [worker commands](../../../crates/orkworksd/src/session_store/worker.rs) | Exclusive staging create; existing open validates app/schema/workspace/epoch/generation and quick_check. No complete backup, closed-image seal, full related-file inventory or maintenance-drain command is established by those operations. Opening with this adapter is not a read-only backup validator. |
| [MetadataStore](../../../crates/orkworksd/src/metadata.rs), root/sessions/events/workspace accessors | Legacy metadata, replay/event and workspace settings owners. Needs lossless complete inventory and same-creation associations, requiredness and no-secret capture rules. A session list is not a full directory inventory. |
| [Image storage](../../../crates/orkworksd/src/image_context/storage.rs) | Private captured directories/bundles and cleanup. Needs non-mutating backup inventory/validation; cleanup or recover calls cannot be used as read-only backup validation. |
| [Recommendation store](../../../crates/orkworksd/src/taskmaster/store.rs) and [workflow observations](../../../crates/orkworksd/src/workflow_observations.rs) | Separate graph/transaction and observation/sequence owners. Needs complete stable bodies, dependency closure and pending-work classification; opening a recovering store is not an inert validation step. |
| [Workspace memory edit](../../../crates/orkworksd/src/metadata/workspace_memory.rs) | One owner serializes workspace.json edits. This is neither all-writer maintenance nor a complete backup capability. |

#1250 owns backup/restore orchestration and set validation. Existing owners retain
file semantics and protection; #1244 owns cutover/startup routing and consumption
of recovery results. #1242 producers, #1243 SQL store and #1248 history semantics
remain dependencies. No cleanup handle or stored audit record grants restore
permission. Native/control, application and resource owners must supply actual
qualified operations before effects.

## Scope and consistency point

A set identifies one exact logical workspace, source native root and captured
storage format, plus its application/schema/profile versions. Initial restore
is to the same logical workspace using the same qualified application version;
cloning into a new workspace identity, relabelling SQL headers, changing epoch
or converting format is outside this profile.

The maintenance owner rejects starts, lifecycle/metadata/organization mutations,
provider result application, hooks/agent writers, event/replay/image writes,
Taskmaster/observation writes, retention/cleanup and workspace-setting changes.
It pauses scheduling, drains admitted operations through physical settlement,
and obtains each owning module's closure acknowledgement while retaining the
workspace lease. Unknown external descendants after an unclean stop require
explicit owner quiescence resolution; silence, old metadata or lease acquisition
cannot replace it. No silent session shutdown is authorized.

After the last drain/SQL close and before inventory, establish consistency point
Q: exact root identity, format/marker evidence, SQL epoch/schema/actual generation
when applicable, all module revisions/protection and the maintenance instance.
Hold the same live claim through capture, final complete revalidation and seal
publication. Re-enumerate/revalidate the complete tree and owner state at the end;
any change, missing owner or ambiguous result invalidates the attempt. Releasing
maintenance before sealing yields no complete set. A stored Q cannot recreate a
live claim after restart. Arbitrary unsupported same-user writers are not confined.

| Scope class | Capture and restore rule |
| --- | --- |
| Format-1 baseline | Full legacy sessions and organization state, including orphan/protected evidence, plus workspace-owned related files below. No format-2/cutover evidence may be present for a recoverable format-1 set. Preserve source bytes/extensions; do not normalize the backup. |
| Format-2 baseline | Exact marker and closed SQL image, complete SQL auxiliary classification, metadata/organization/deletion-job identities and related files. Retained legacy import evidence is included as inactive evidence; never restored as a live JSON mirror beside SQL. |
| External content | Complete events/replay/terminal-size/checkpoint stores and session-images, including owner-classified unreferenced/orphan content. Reference-only copying is insufficient for related/protected evidence. No pruning or image-adoption permission follows. |
| Related workspace stores | Recommendations, full graph/rollup state, workflow observations/tombstones/sequence, workspace settings, branch-document associations and other registered durable workspace-owned data. Capture existing authorities without migrating them into SQL. |
| Recovery/cleanup state | All durable owner-classified deletion jobs, journals, staging and protection evidence are inventoried. Unknown or unsettled work blocks a recoverable set; qualified settled/resumable state may be included only with an exact owner validation/restoration contract. Never delete it to make capture pass. |
| Explicit exclusions | Locks, process handles, runtime/report credentials, transient native relay/mailboxes and their capabilities; global tool/provider/application settings and native conversations; repository/worktree content. Record each excluded namespace and owning reason; never follow a metadata path into those stores. |

Every enumerated entry receives exactly one classification: included durable
file/directory, qualified excluded namespace, or blocking unknown. A filename
allowlist, filename suffix, UI page or reference list cannot silently omit an
unknown workspace entry. Owners must identify credential-bearing data before
copy; do not copy first and redact later. If a durable record mixes required
state with credentials and no lossless qualified split exists, block this profile.

Symlinks, reparse entries, unsupported native names/permissions, special files,
root aliases, overlapping roots and paths escaping the captured namespace block
unless the exact native profile has separately qualified their representation.
The initial profile follows no links and uses independent byte copies. Preserve
required mode/access metadata through the native owner; hashes alone do not prove
restorable identity or access. Copied native handles/lock files are never live.

## Bundle model and validation

Every bundle has a closed purpose class, bound by its manifest/seal and every
consuming owner result:

- `recoverable-workspace`: a semantically validated complete format-1 or format-2
  workspace at Q, eligible for same-version isolated restore after new admission.
- `retained-evidence`: a complete, readable capture of retained local source
  evidence, eligible only for the accepted per-candidate adoption review. Preserve
  raw bytes/identities and classify malformed records or known missing content;
  do not claim the old source is a semantically recoverable workspace.
- `preservation-only`: raw current-destination state retained before replacement,
  including damaged or unresolved cutover evidence. It grants neither workspace
  restore eligibility nor adoption-input validity.

No class can be relabelled or substituted by the caller. Full physical capture,
identity, enumeration, exclusions and seal validation apply to all three; semantic
validation and consuming rights are class-specific. Missing optional/required
content is reported under its actual owner classification, never fabricated.

Logical bundle version is `orkworks.workspace-backup.v1`. It is a private directory
under a qualified destination parent disjoint from source, restore destination
and other bundle roots. The owner exclusively creates an attempt directory; it
never overwrites another bundle. Names/layout/codec and native publication must
be fixed in the subsequent representation qualification before implementation.
No archive extractor, arbitrary caller path or serialized callback is accepted.

The immutable manifest retains these complete fields (no summary substitutes):

- Version and required application/schema/native/owner profile versions; backup
  attempt identity, exact workspace identity, source root binding and Q.
- Bundle purpose class, source format and all authority evidence. A recoverable
  format-1 set records proven absence of cutover evidence; a recoverable format-2
  set binds valid marker to exact SQL workspace/epoch/schema/import identity and captured actual generation, never a made-up zero.
- Exhaustive relative-entry inventory: exact supported name bytes, object kind,
  owner, source native identity, independent bundle entry identity, content byte
  count/hash, required access metadata and owner revision/association/protection.
  Native identities describe each occurrence; they are not transferred on restore.
- Requiredness/reference inventory with exact session creation identity and reason
  for each required or optional relationship, including orphan durable evidence.
  Known optional absence is explicit; a failed read/enumeration is not absence.
- Complete exclusions, owner validation bodies, pending-work disposition, aggregate
  entry/content/physical-accounting totals and the full bounded validation result.

All manifest and referenced body bytes are retained, canonicalized under a closed
versioned codec, hashed and bound by the final completion seal. Seal publication
requires actual read-back validation of the copied entries and complete scope,
not the source hash alone. Reject duplicate paths/IDs, alias/case collisions,
unknown versions/tags, missing bodies, conflicting owner IDs, overflowing counters,
truncation, unlisted entries and unsupported limits. Before code, pin the codec,
ordering, digest domain separation, seal envelope and every owner body; until
then no production parser/receipt can claim this logical schema is executable.

Validation occurs in an inert private context: no normal application/store open
that repairs files, resumes journal effects, schedules work, renews credentials
or launches tools. For recoverable-workspace sets, verify database integrity and
complete record/organization/revision/reference semantics through the qualified
SQL validator, beyond file hashes/quick_check. Include real generation and
tombstones; no deleted row may be resurrected by restore. Required external/
protection evidence must exist and validate. A missing optional payload is allowed only if its owner proves
absence at Q, its optional status and the retained historical fact/availability
result; unreadable, ambiguous or corrupt required content blocks recovery-set
success. The absence of an optional payload does not permit copying corrupt
present content as valid.

For retained-evidence sets, readable malformed independent local metadata remains
exact raw evidence with a malformed classification, never an invented Session.
Known missing required content remains explicitly missing; its source-only
metadata must be excluded from the adoption batch. The full inventory and raw
bytes of every extant supported file must still validate. Unreadable bytes,
incomplete enumeration, uncertain identity or unqualified exclusion blocks the
capture. Adoption's existing history/association/protection/normalization owners
must validate every selected record and payload; a retained-evidence seal cannot
bless malformed selected content or bypass current-global corruption/protection.
These distinctions preserve v1's accepted missing-required/excluded and
malformed-local/excluded oracles without granting whole-workspace restore.

A recoverable set returns a complete result only after owner validation, copied
byte read-back, durable file/directory publication and completion seal settlement.
A partial directory, digest or progress record is not success. Separate truthful
outcomes are complete, stopped, failed, or unknown; failure carries exact phase,
owned partial effects and reason without exposing secrets. Stored complete bytes
prove only captured data: consumption revalidates full integrity, supported
profiles, complete scope and the intended restore/capture subject.

For retained-source adoption, produce two separately validated sets: a complete
`retained-evidence` capture of the selected local source and a
`recoverable-workspace` format-1 backup of the current global root. Bind both
classes and complete scope to the exact v1 Snapshot inventories/roots and current
publication Intent/Plan through the actual backup owner. Neither covers the other
by implication. Later backup-current evidence must revalidate both actual sets,
the source's complete evidence-capture validity and the global restore-point
validity, not merely re-emit a retained receipt. The local set can restore captured
source evidence bytes only through its separately qualified evidence owner; it
cannot be installed as a workspace. Do not modify accepted v1 bytes or supply a
dummy known receipt. Unsupported source scope blocks adoption.

## Capture phases and interruption

Sequence: admitted maintenance/resources → Q/inventory → private copied files →
full validation → durable bundle/seal publication → complete result. No global
cutover file or source record is changed by capture. All phases require exact
owned paths and live maintenance/resource/native owners. Cancellation prevents
new effect admission; an admitted copy/flush/publication settles truthfully before
owners/resources release. A completion seal may exist after response loss or a
cancellation race; validate it before reporting complete, never promise deletion.

After owner loss, a new operation may inertly validate an existing sealed bundle
as data; it cannot reconstruct the old maintenance permission or continue an
unsealed attempt. Unknown/partial attempts remain unavailable and preserved.
Cleanup needs separately authorized, exact current ownership and evidence; no
path/hash match alone authorizes removal. There is no automatic retry loop.

## Same-version restore and preservation

Workspace restore accepts only a `recoverable-workspace` set. Neither a
retained-evidence nor preservation-only seal satisfies that input. Restore
requires a separate explicit local-owner decision binding the selected complete
set, exact current destination/workspace, intended captured point and visible
age/data-loss implications. Backup creation approval grants no restore
permission. Agent/report capabilities and retained confirmation bytes cannot
instantiate this live claim. Validation failure invalidates it; restart requires
new confirmation and actual maintenance admission.

1. Validate the selected bundle inertly, without touching the destination. Admit
   destination maintenance, block and drain every writer, capture its exact state
   and obtain current native/control/resource guarantees. Reject workspace/version
   mismatch, unsupported schema or corrupt/incomplete selected input.
2. Capture and durably retain **all current destination durable bytes and unresolved
   evidence** before replacement. If current state is valid, use a new complete
   backup. If damaged/mid-cutover, use a preservation-only capture with full raw
   inventory, errors/unknowns and no claim of a valid database or recovery point.
   Unreadable bytes, unknown exclusions or inability to preserve the full current
   state blocks replacement. Never recursively require the damaged current state
   to pass complete-backup semantic validation. Preservation is not lossless
   recovery of writes already absent from the selected backup.
3. Restore independent copies into an exclusively owned private staging namespace.
   Validate content, access metadata, all owner semantics and complete references;
   retain historical/native reference bytes but grant no runtime/process/native
   resume or old approval authority. Destination-specific capabilities are newly
   issued only by their actual owners. Do not use normal mutating recovery on
   staged files as a substitute for inert validation.
4. Installation consumes a qualified #1250 restore claim through #1244's maintenance
   and startup-routing seam. **Before the first destination replacement**, a durable
   restore-in-progress guard and exact effect intent must be discoverable by every
   supported opener, including after a crash with an old valid marker present.
   The guard is a fail-closed maintenance fence, not a new storage format or an
   alternative activation point. Its exact native representation/discovery and
   effect journal must be qualified before installation code; unavailable means
   no replacement. Do not infer a guard from a missing filename or process handle.
5. Under that fence, publish the complete validated state through the actual owners,
   with exact per-effect intent/outcome and source/destination revalidation. No
   cross-file atomicity or pathname-based overwrite permission is implied. Retain
   existing data/preservation and truthful partial effects on any error. For
   format 2, publish/validate the restored SQL/external set before the matching
   existing format-2 marker; the old marker cannot authorize mixed content while
   the restore fence is present. Format-1 restore is a separately explicit recovery
   of that captured point, never automatic fallback or lossless SQL downgrade.
6. Only after the entire restored set and normal-open authority rules validate,
   settle installation and durably release the restore fence through its qualified
   owner. No start is admitted earlier. Retain both selected backup and preserved
   destination until separate explicit owner cleanup; ordinary retention cannot
   remove them. Installation completion permits normal validation/open, not resume.

A crash/cancel after installation admission does not promise rollback. Any pending
or unknown effect keeps the fence and blocks open; restarted code reconstructs
facts, never old approval. An explicit recovery operation must validate the whole
actual state and exact owned effects before choosing a new authorized attempt.
The restore fence and effect journal belong to a separate qualified restore
operation. They cannot resume adoption, reconstruct its lost approval/publication
claim, synthesize adoption progress or roll forward an interrupted pre-activation
attempt. The accepted adoption restore-first rule stands: resolve the old cutover
through qualified same-version recovery, then capture and review a new v1 attempt.
This design does not supply a generic recursive rollback or lost-owner roll-forward
algorithm. If fence release or its acknowledgement is ambiguous, qualified startup
must prove completion or remain blocked. Stored progress cannot override a valid
format-2 marker once the qualified restore fence has definitively settled.

## Failure and rejection oracles

These are conditional requirements, not executed crash/platform results.

| Condition/boundary | Required outcome |
| --- | --- |
| Any writer/descendant, root or resource ownership unknown | No Q, backup success or replacement; stop/unavailable. |
| Close fails or SQL auxiliary set is unresolved | No closed-image capture; do not copy only v1.sqlite and call it complete. |
| Source changes during copy/final validation | Invalidate complete set; retain partial owned attempt, do not reseal old Q. |
| Known optional absence versus unreadable/required missing file | Recoverable-workspace requires explicit optional-absence proof; required missing blocks. Retained-evidence reports known missing content and requires exclusion of affected source-only metadata. Unreadable/ambiguous capture blocks every class. |
| Readable malformed independent local record, all raw bytes/identities captured | Retained-evidence may preserve/report it for explicit exclude-only review; never eligible selected metadata or whole-workspace restore. Global corruption still blocks the recoverable global backup. |
| Retained-evidence or preservation-only bundle passed to workspace restore | Reject purpose mismatch before destination effects; no class substitution or automatic conversion. |
| Unknown entry, secret-bearing durable record, unsupported link/name/profile | Block before unsafe copying; never silently omit, traverse or sanitize required bytes. |
| Disk-full/cancel/crash during capture/validation/seal | No claimed complete result unless an actual valid durable seal/set can be established; no automatic partial cleanup. |
| Seal succeeds but response is lost | New inert validation can identify captured data, not revive old permission. |
| Copied corruption, unlisted extra file, marker/SQL mismatch, malformed owner body | Reject set; no fallback, repair or partial success. |
| Current destination corrupt or mid-cutover | Preserve raw current state separately; it is not a known complete-backup receipt. Unpreservable bytes block replacement. |
| Destination changes after review/preservation | Revoke restore admission; no stale overwrite, fresh capture/confirmation required. |
| Restore staging incomplete or validation fails | Destination remains unchanged; retain exact private partial state. |
| Crash before durable/discoverable restore fence | No destination replacement may have been admitted. If this cannot be proved, stop; no ordinary open assumption. |
| Crash/cancel during any installed file, SQL or marker effect | Fence continues blocking; retain complete selected/preserved sets and actual effect evidence. No mixed open or implicit rollback. |
| Restored set complete, fence settlement/response lost | Qualified startup proves complete installation/fence settlement or remains unavailable; never guesses from matching hashes. |
| Post-cutover writes newer than selected set | Owner sees point-in-time loss implications; current state is preserved. Old backup cannot claim those writes recovered or lossless downgrade. |
| Repeat restore/restart, reused ID/path, unsupported/older binary | New confirmation/owners and exact identity checks; no reconstructed permit. Unsupported binaries remain unsupported. |

## Qualification and handoff gates

Before implementation: close the exact bundle/seal and owner-body codecs;
complete per-platform tree/exclusive-copy/flush/fence discovery and replacement
contracts; maintenance/application capture/restore confirmation; inert validators;
full current-state preservation; SQL auxiliary/closure; and finite physical
resource accounting. These are missing guarantees, not mocks to make a test pass.

Account for source/current trees, selected and preservation sets, both adoption
backups, staged/restored copies, full inventories and owner bodies, SQL auxiliary
files, progress/partial attempts, directory/access metadata and callback/recovery
scratch, charging every simultaneous occurrence before allocation. Checked finite
counts/bytes and retained admission are required; free space, page caps or another
journal's budget do not establish fit. Numerical limits and execution budgets
must be separately reviewed; this design assigns none.

Copied/synthetic qualification must exercise every matrix row and install effect
boundary, invalid/unsupported versions, owner loss, full disk, concurrent mutation,
all source/global/current-root bindings, optional absence and post-cutover writes.
Run real owner validators, scoped/full repo verification and current-diff reviews
when code is later authorized. Native execution needs an explicit finite request.
#1250 remains open for implementation/qualification; #1244/#1247 and producer,
history, native/hierarchy/rollout gates remain. No product backup UI/API, live
restore, dependency bypass or new model/native probe is delivered here.

## Review disposition

A fresh builder completeness review found no important unanswered planning
questions beyond explicitly gated codecs/native profiles/resource bounds. A
fresh authority/source-fidelity review found an ambiguity between the separate
restore protocol and adoption recovery. Clarified that restore guard/journal
cannot resume adoption or reconstruct a lost approval/publication claim; the
accepted restore-first boundary remains. Inspected source claims matched the
pinned baseline. Neither review qualifies a producer or physical operation.

Codex current-head review of the initial PR found that requiring the retained
local source to be a semantically recoverable workspace rejected accepted v1
exclude-only inputs. Corrected with purpose-bound retained-evidence capture,
separate global recoverable backup and damaged-state preservation, class-specific
validation/consumption and explicit rejection oracles. All classes still require
complete readable physical evidence; none can bypass accepted candidate decisions.
