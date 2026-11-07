---
type: Source Qualification Review
title: Legacy adoption evidence-owner audit
description: Pinned private source primitives and missing actual-owner guarantees behind accepted legacy inventory v1.
tags: [orkworks, sessions, migration, ownership, qualification]
status: stable
---

# Legacy adoption evidence-owner audit

Inspection: 2026-11-07, source
`418cacba3b7b3f9b9ce90b1ab6d59d11c1fecf42`.
Tracking: [#1506](https://github.com/Rambolarsen/orkworks/issues/1506),
[#1244](https://github.com/Rambolarsen/orkworks/issues/1244).
Contract: [accepted logical v1](../superpowers/specs/2026-11-07-legacy-adoption-inventory-approval.md).

The owner approved v1's unchanged byte representation and confirmation ordering
for further qualification after #1504. This audit uses repository source only:
no live metadata, backup/restore, native/model probe, product build or runtime test was
executed. Source observations are not platform execution evidence. No production
`known` v1 profile/receipt or actual approval witness is qualified by this audit.

## Actual owners and remaining inputs

Line references below refer to the pinned source. Local links identify the
canonical source file; do not assume a later version has identical line numbers.

| v1 input | Available inspected source | What it does not establish |
| --- | --- | --- |
| Native root/lease identity | [`WorkspaceLease`](../../crates/orkworksd/src/workspace_runtime.rs), lines 162–218: advisory exclusive lock; Unix `validate_directory` compares the captured lock inode with the directory's no-follow lock entry. | No complete two-root tree inventory, profile registry, Windows identity body or proof all external writers stopped. The lock file's presence is not ownership. |
| Tree/exclusive I/O building blocks | [`journal/protocol/files.rs`](../../crates/orkworksd/src/taskmaster/store/cleanup/removal/journal/protocol/files.rs), lines 1–121: validated basenames, anchored leaf reads/rechecks, O_EXCL/no-follow creation, sync and anchored removal. [`protocol.rs`](../../crates/orkworksd/src/taskmaster/store/cleanup/removal/journal/protocol.rs), lines 23–43, revalidates captured root identity/canonical path. | These are private recommendation-removal operations, not an adoption inventory/publisher. Check-before-operation is not a general atomic identity comparison; native races, complete enumeration and receipt generation remain to qualify. |
| Platform scope and local budgets | [`journal.rs`](../../crates/orkworksd/src/taskmaster/store/cleanup/removal/journal.rs), lines 1–81: source-only journal selected on macOS/Linux, unsupported elsewhere. The protocol declares LIMIT=1 MiB and TOTAL=32 MiB (protocol.rs lines 15–18). | No portable `Native` profile, Windows adoption support or adoption manifest/resource fit. Domain-specific journal limits cannot be borrowed as adoption limits. |
| Normalization/creation identity | [`session_store.rs`](../../crates/orkworksd/src/session_store.rs), lines 400–434: checks complete serialized metadata size, ID grammar and nonempty ID/created_at up to 256 bytes, runs existing normalization and merges known fields into the original document. | No v1 normalization/history receipt body or profile registry, no native-source ownership or original payload association. A source field that fails these checks cannot be labelled valid by an adapter profile. |
| Historical/organization identity | [`session_archive.rs`](../../crates/orkworksd/src/session_archive.rs), lines 140–180: validates the ID and checks record ID/version/revision/state, matching exact created_at. The accepted historical-fact task is #1248. | No qualified legacy-adoption history result, permission to invent terminal outcomes, or proof that a source-only session was never deleted. |
| Payload association and absence | [`MetadataStore`](../../crates/orkworksd/src/metadata.rs), lines 1351–1377, exposes sessions/events under its root. [`replay_cleanup.rs`](../../crates/orkworksd/src/metadata/replay_cleanup.rs), lines 10–106, captures an ID and binds Unix root/events handles to a lease; non-Unix binding is unsupported. | An ID-derived pathname is not v1's same-creation association/requiredness receipt. Cleanup binding is not an adoption absence/tree proof or exclusive-create publication permission. |
| Protection/current cleanup state | [`CapturedDeletion`](../../crates/orkworksd/src/session_application/sql_writes/deletion.rs), lines 147–267: captures a closed/runtime-creation context, requires an exact SQL deletion record, retains file-budget/lease owners and binds cleanup handles outside application guards. | A deletion-specific claim cannot authorize additions or mean “no protection” for every other session. Global organization, deletion jobs and protected recommendation/evidence relationships still need a complete current profile. |
| Quiescence/maintenance | [`open_workspace_inner`](../../crates/orkworksd/src/session_application.rs), lines 3525–3601: holds projection ownership, acquires/adopts a lease, creates directories and takes the workspace-memory edit guard before legacy copying. | This inspected open flow supplies no complete paused/drained writer set or explicit unclean-stop resolution for both old and global sources. It is not #1250 maintenance admission. |
| Complete backup and capture scope | [Accepted storage backup contract](../superpowers/specs/2026-11-01-sqlite-session-storage-design.md#external-payloads-events-backup-and-deletion) and open #1250 require complete, quiescent, validated workspace/external-file sets. [`settingsMemory.ts`](../../apps/desktop/electron/settingsMemory.ts), lines 499–529 and 570–578, has local settings-file backup/recovery. | A single settings backup, copied DB, partial import or static scope document is not a verified source/global backup receipt or complete restore capability. No such v1 producer was established by the inspected flows. |
| Trusted application confirmation | [`main.ts`](../../apps/desktop/electron/main.ts), lines 789–803, constructs a main-process inference trust context with sidecar-generation checks and a local dialog. [`inferenceTrust.ts`](../../apps/desktop/electron/inferenceTrust.ts), lines 18–65, validates/rechecks the exact adapter revision around confirmation. | This is executable-inference approval, not three-phase inventory presentation, a v1 review/approval handle or adoption authority. Its route/credential and dialog result cannot be repurposed as a `known` application receipt. |

## Keep cleanup authority separate from adoption

The captured SQL recommendation-cleanup stage delivered in #1505 composes
existing deletion owners; it does not turn their handles into a source importer.
Its [fixture tests](../../crates/orkworksd/src/session_application/sql_writes/delete_recommendation_tests.rs)
name the expected full-graph cleanup, reopened progress and unchanged legacy
metadata behavior. Those tests were **read, not run** in this audit. Their presence
is not a new execution or portability result.

Likewise, the private [image cleanup binding](../../crates/orkworksd/src/image_context/storage/sql_cleanup.rs),
lines 63–124, captures root/bundle handles for generated UUID image owners under
a lease. It provides no local-image adoption rule, and the accepted policy still
excludes local-image import. Replay cleanup's durable deletion and directory
absence checks cannot become generic file creation or backup authority.

The source-only journal's sync/progress and exact captured-leaf checks are useful
inputs to a publication design. Do not infer atomic SQL-plus-filesystem commit,
a portable conditional unlink, complete rollback or same-user confinement from
these individual operations. The actual adoption profile must close its own
pre-operation identity, publication and restart conditions before code relies on
it. No existing ordinary or linked-pair budget/probe allowance is transferred.

## Next contract gate

Use [qualification step 2](../superpowers/plans/2026-11-07-legacy-source-adoption-qualification.md)
for conditional written publication/recovery work against these actual owners:

1. Obtain #1250's explicit source/global scope, complete backup validation and
   restore-owner interfaces; keep unavailable inputs Unknown while pending.
   Include both roots and required external evidence, with credentials/transient
   material excluded by the actual owner rather than an ad hoc file filter.
2. Map the native tree/absence, normalization/history/association, protection,
   quiescence and trusted application profiles to concrete producing owners and
   full receipt bodies. v1's carrier shape and digest are not those guarantees.
   Preserve exact creation identity, original/current data and the accepted
   local-owner presentation/confirmation rules without re-opening their approval.
3. Define the exact durable progress/effect states around each exclusive payload,
   staged SQL and unchanged format-2 marker. Require an evidence-backed recovery
   or stop/complete-backup restoration outcome for every interruption, including
   foreign replacement and source/protection changes. No pathname/digest guess
   may authorize removal or overwrite.

Static contract drafting may proceed conditionally; dependent runtime code cannot.
Qualified backup, complete physical accounting, platform identity/durability and
actual trusted approval transport remain missing evidence. Any required native
execution needs its own explicit finite request; this audit supplies no budget
or permission for live recovery. Existing #1242/#1243/#1248/#1250 and native/
hierarchy/rollout gates remain. Reuse accepted v1 and additive-policy approval.

## Review disposition

A fresh read-only source/acceptance-fidelity review on 2026-11-07 found no material
issues in the pinned owner claims or scoped v1 acceptance. Existing native,
backup, application and publication/recovery qualification remains Unknown;
no product runtime or physical-fit result was promoted from source inspection.
Docs metadata/index checks, all 1,202 tests and the site/link build passed.
