---
type: Design Specification
title: Explicit adoption of retained legacy session evidence
description: Accepted-for-qualification additive, owner-approved recovery of selected workspace-local session records and missing payloads during storage maintenance.
tags: [orkworks, sessions, migration, recovery, ownership]
status: stable
workflow_status: accepted-for-qualification
---

# Explicit adoption of retained legacy session evidence

Status: accepted by the owner on 2026-11-07 for ADR and qualification-plan updates;
implementation and live recovery remain gated.
Drafting: [#1494](https://github.com/Rambolarsen/orkworks/issues/1494).
Acceptance record: [#1498](https://github.com/Rambolarsen/orkworks/issues/1498).
Delivery owner: [#1244](https://github.com/Rambolarsen/orkworks/issues/1244),
consuming verified backup/restore from [#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
The dated amendment in [ADR 0103](../../adr/0103-sqlite-session-storage.md)
records this additive policy without changing the existing activation point.

## Problem and scope

The [source review](../../validation/legacy-migration-recovery-review.md) found
that legacy workspace-local copying has no completion/deletion manifest. A
source-only session might be a failed import or intentionally deleted history.
Replaying the source can overwrite newer data or resurrect deleted sessions.

Extend the [ordinary-session storage design](2026-11-01-sqlite-session-storage-design.md#migration-and-single-authority-activation)
with explicit selection of retained local evidence **before the first format-2
activation**, under the existing maintenance/import owner. Keep global format-1
metadata authoritative until the existing activation point. No startup merge,
standalone format-1 repair engine, post-activation supplemental import or live
recovery is included. An urgent repair before the storage prerequisites are
ready needs its own scope; this acceptance does not make #1244 ready to execute.

The owner approved the concrete policy after reviewing PR #1495 at
`a1fd8b7834a44b3b7342bf84ae7b9c55d7e9bdd2`, replying “keep going” to the
explicit request to approve it for ADR and plan updates. Existing producer,
backup, historical-fact, linked-store, native/hierarchy and rollout gates remain
binding. This approval supplies no runtime implementation, model/native probe
budget or live-data access.

## Authority and preservation

The owner explicitly selects the canonical workspace and its retained
`<workspace>/.orkworks/` source, then reviews exact candidate decisions. Do not
scan unrelated repositories or infer authority from a directory's presence.
Require source/destination identity checks, unchanged roots and qualified
quiescence for all current and legacy writers. Reject symlinks, junctions,
reparse redirects or replaced roots that escape the selected captured roots;
implementation must qualify those checks on each supported platform.

Use #1250's maintenance owner to verify complete backups of current global data
and the selected local evidence before **any** adoption writes. The repository
and coding tools' native stores remain excluded. Backups exclude credentials,
locks and transient runtime/relay artifacts. Preserve original roots and all
current records on rejection or interruption; a backup failure blocks adoption.
After an unclean stop, require explicit owner quiescence resolution. A lease,
old PID, metadata status or matching hash alone does not prove writers stopped.

Selection and approval are application-owner maintenance actions. An agent
report token, task recommendation, previous migration approval or source file
cannot authorize adoption. The application presents the captured report and
requires explicit local-owner confirmation of the exact decision manifest.
The maintenance owner revalidates its identity/digests before acting; generic
session metadata/report routes cannot supply this approval. A new or changed
manifest needs a new confirmation.

## Inventory and supported inputs

Capture and stream complete source/global inventories with root identities,
relative paths, file kind/identity, size, content digest and supported schema/
creation identity. Preserve effective provenance and unknown supported metadata
members under the accepted import contract. Never infer precedence from mtime;
mtime remains evidence only where existing legacy provenance requires it.
A read/enumeration/parse/identity error is reported and blocks a claim of a
complete inventory. Do not follow escaping links or treat unavailable data as
an empty source/destination.

Initial adoptable inputs are supported ordinary-session metadata and its known
external `events/<id>.ndjson`, `<id>.terminal` and `<id>.terminal-size` files.
Report unsupported versions, filenames, file kinds and relationships explicitly.
Existing global images and other related files remain under their current
owners; this initial scope does not import a local image bundle without a
separately qualified association rule. The owner must explicitly exclude an
unsupported candidate; if that candidate is required for a selected record's
validity, the record cannot be selected. Never advertise complete recovered
history when optional payloads are missing or intentionally excluded.

Local `workspace.json`, capacity, recommendations, observations, settings,
organization and arbitrary extra files are evidence only. Back up/report their
presence and exclusion; do not replay them or infer session-adoption authority
from them. Current global organization and external-evidence protections retain
their existing owners. This is not restoration of an entire old workspace.

Inventory/report processing is streamed and separate from PTY/runtime work.
Report pages use at most 100 entries and 256 KiB including the envelope;
continue through all pages before confirmation. Do not truncate entries or
silently omit oversized input: stop with a diagnostic when a single entry cannot
fit. These report limits do not change existing history/SQL limits.
Existing import record limits still apply, including the 1 MiB canonical metadata
record bound. Full manifest storage/accounting and platform path representation
must be qualified before implementation; no all-history in-memory inventory is
permitted.

## Decision rules

Global metadata and every existing global payload stay unchanged. There is no
replace, field merge, event concatenation, timestamp winner or overwrite option.
Byte equality can identify already-present evidence, but never establish session
identity or owner permission by itself.

| Candidate | Permitted decision and result |
| --- | --- |
| Global-only record/payload | Keep global; do not remove it because the local source lacks it. |
| Same exact session/creation identity, equal metadata | Keep global metadata; report equality, do not copy over it. |
| Same identity, differing metadata | Keep global metadata and report the difference. Source metadata cannot replace it in this amendment. |
| Same session ID, different or unprovable creation identity | Conflict; no metadata or payload from that source candidate may be adopted. Explicitly exclude it or stop for separately designed repair. |
| Supported source-only session with provable creation identity | Default unresolved. Owner explicitly adopts the exact record plus individually selected valid payloads, or excludes it. Absence never proves a missed migration or deletion intent. |
| Source payload absent globally for an existing same-creation session | Default unresolved. Owner can explicitly adopt that exact payload after association/content validation, or exclude it; absence is not sufficient. |
| Existing global payload, equal or different source bytes | Keep global payload. Report equality/difference; no concatenation or replacement. |
| Malformed, unsupported or unassociated local candidate with readable, inventoried bytes | Report; explicitly exclude when independent of selected input, otherwise stop. No silent omission or identity invention. |
| Unreadable input or incomplete enumeration/backup | Block the batch until resolved; exclusion cannot turn an incomplete backup/inventory into success. |
| Current tombstone/deletion work, corrupt/unknown global record or protection conflict | Never classify as source-only/absent. Block adoption of that identity until its existing owner resolves it; approval cannot override protection. |

An intentionally deleted source-only session can be restored only through the
same explicit adopt decision, with the report warning that it may restore
previously deleted history. No inferred deletion ledger is introduced. Source
selection alone is not this per-candidate approval. Exclusion preserves the
original evidence; it never authorizes deleting source or current data.

Imported source-only sessions are historical records, never managed runtimes.
Qualify their state using #1248's historical-fact policy and accepted import
normalization. Do not invent a terminal outcome, imply a live process, resume a
native conversation, select/acknowledge a session or add runtime authority.
Unresolvable lifecycle or organization identity blocks that candidate. Existing
global lifecycle/organization decisions cannot be replaced by local evidence.

## Report, approval and stale decisions

The review report identifies workspace/source/global roots, baseline inventories,
backup identities, all candidate classifications, selected metadata/payloads,
kept-global/excluded items, unresolved items and missing optional content.
Each adopt/exclude decision binds the exact source path/file identity/digest and
classification. Adoption additionally requires a proved target session/creation
identity; an excluded malformed candidate may retain an explicit unknown identity
and can never authorize adoption. Payload decisions also bind the unchanged global
metadata identity and destination-absence observation. The complete ordered
manifest has a versioned canonical encoding and digest bound into the existing
import manifest. It contains no credentials and is retained as recovery evidence.

Confirmation is disabled while any candidate lacks a disposition or a selected
candidate has unresolved required content, invalid identity or protection.
The application shows counts and bytes for adoption, exclusions, conflicts and
missing payloads, and the possible-deleted-history warning before confirmation.
The owner confirms one exact reviewed batch, not a rule for future source files.

Before publication, revalidate both complete inventories, captured root/file
identities, backup validity, owner decisions and quiescence. Any changed source,
new destination, changed metadata/protection, invalid backup or incomplete
inventory invalidates the batch. Re-inventory and obtain confirmation for the
new digest; never turn stale decisions into implicit approval or partial success.
No candidate may be added because it appeared after confirmation.

## Staging, activation and recovery

Use #1244's existing maintenance admission and private import staging; normal
opening, writers, retention and cleanup remain blocked throughout preparation
and recovery. The ordinary accepted global import is preserved; selected local
records add to it without rewriting the global source tree.

Validate metadata, relationships, selected external payloads and complete report
accounting before publishing. Stage adopted payload bytes privately. Keep their
final placement under existing external-file owners with **exclusive create**
semantics: a destination that becomes present is a conflict, never overwritten.
Do not create a missing payload for an unapproved/reused session identity.

External-file publication and SQLite/marker publication are not one atomic
transaction. Retain an adoption manifest/progress record before publishing any
final payload; it binds the exact approved files and current import generation.
Only after all selected payloads, staged SQL, backups and source rechecks validate
may the existing format-2 activation marker publish. Ordinary opening remains
blocked by staging/cutover evidence even if a crash leaves only some new external
files. No new activation point or alternate source authority is introduced.

On interruption before activation, use the accepted verified import-only recovery
path only when the exact manifests, progress, source/global inventories and
newly published payload identities are demonstrably consistent. Rechecking the
baseline accounts separately for additions recorded by this exact progress
manifest; arbitrary extra changes invalidate recovery. Otherwise stop for
#1250 same-version complete-backup recovery under maintenance. Never remove a
pathname merely because its bytes match a staged file, guess ownership, recopy
old evidence, or enable normal legacy opening beside unresolved evidence.

After activation, SQL remains authoritative and adopted payloads follow existing
owners. Marker loss, post-cutover writes or ambiguous publication use the accepted
recovery rules; re-running local adoption is forbidden. Source backups and
manifests survive until explicit owner cleanup outside ordinary retention.
Partial filesystem effects must be reported accurately; failure cannot claim a
rolled-back external file or complete successful import without verification.

## Qualification and delivery gates

The policy approval and dated ADR amendment permit the
[qualification plan](../plans/2026-11-07-legacy-source-adoption-qualification.md),
within #1244/#1250's existing dependencies. Settle and review the canonical
approval/manifest codec, complete on-disk inventory/progress accounting,
application-owner approval transport, external-owner identity/exclusive
publication and interrupted-recovery contracts before code depends on them.
Those are qualification deliverables, not runtime implementation approval; no
finite storage/native fit or platform guarantee is asserted here.
The [v1 inventory/report/confirmation contract](2026-11-07-legacy-adoption-inventory-approval.md)
is the proposed step-1 representation, not a qualified approval mechanism.

Copied/synthetic regression cases must exercise:

- A copied session with a missing event, mixed copied/missing sessions and the
  empty-global case after deliberate deletion, with no automatic adoption.
- Source-only records explicitly adopted versus excluded, possible-deleted-history
  warnings, differing global metadata and payloads, creation-ID reuse, corrupt
  global records and deletion/protection work that approval cannot bypass.
- Missing payloads explicitly adopted for the same creation identity, mismatched
  or unsupported payloads, unsupported local images, and partial optional history
  reported honestly; current global bytes remain unchanged.
- Every input/report page and complete decision accounting; oversized entries,
  unreadable/enumeration failures and source changes never yield incomplete success.
- Changed roots/files, late destination creation, lost quiescence, changed backup/
  protection/decision digest and unknown state reject the confirmed batch.
- Failure before/after each backup, progress, payload, SQL and marker boundary;
  partial payload publication, invalid/missing marker, repeat recovery and later
  SQL writes preserve single authority and never overwrite/delete by pathname guess.
- Historical-fact/organization/native identity preservation, no runtime resume,
  no changes to current retention/evidence owners, and no startup merge path.

Native filesystem, durability, ownership and producer gates require separately
bounded execution evidence on supported platforms. No live-user repair or new
probe budget is supplied by this acceptance.

## Acceptance and delivery status

The additive policy, explicit per-candidate decisions and maintenance-only
ownership are accepted for further qualification. Reuse that approval within
scope rather than reopening the same policy. The linked plan begins with
written inventory/approval contract closure; it does not execute adoption.
Runtime implementation, live migration and probe budgets remain outside this
acceptance until their stated contracts, dependencies and plan gates pass.
