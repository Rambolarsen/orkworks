---
type: Design
title: Production review preview binding
description: Proposed authenticated immutable preview and consuming Start contract for ordinary plan-review recommendations.
tags: [orkworks, taskmaster, review, contracts]
status: draft
---

# Production review preview binding

**Status: proposed for owner review.** The owner authorized preparing this
contract after [#1906](https://github.com/Rambolarsen/orkworks/pull/1906), not
runtime implementation. The accepted [interaction design](2026-11-18-review-handoff-preview-design.md)
and delivered isolated preview remain unchanged. Track this proposal under
[#1899](https://github.com/Rambolarsen/orkworks/issues/1899), with the ordinary
launch consumer owned by [#68](https://github.com/Rambolarsen/orkworks/issues/68).

## Outcome and first supported subject

One authenticated read prepares an immutable view of the exact ordinary
plan-review handoff. The existing New session composer may edit the proposed
coding tool, model and user instruction, but each edit requires another preview.
Explicit Start consumes the inspected view after fresh validation. Reading,
refreshing, closing or changing fields never accepts a recommendation or starts
work. Preserve the existing selected-session fixed review prompt unchanged.

Proposed first production scope: one new ordinary reviewer, one exact source
session/runtime and one selected plan/spec. No existing-reviewer retargeting,
code-only review without a plan, images, recursive delegation, child launch,
new selection engine or cross-workspace operation. Those are not silently
represented as optional fields. A missing plan is inspectable but cannot start.

The initial review directory must equal the current workspace's validated
canonical root and the source's effective worktree. If they differ, report the
proposed review location as unavailable; do not substitute either root. Current
`CreateSessionCommand` carries an expected workspace path, not an independently
selected working-directory argument. Broader directory editing needs #68's
separate creation/containment contract. Ordinary shared-directory review remains
read-only by instruction, not filesystem confinement. #610's independent child
chains retain their separate plan-owned worktrees and exact-plan authority.

## Verified source seams and ownership

Inspected at main `b3d0e87e`:

| Owner | Existing seam | Required responsibility |
| --- | --- | --- |
| Taskmaster proposal owner | `crates/orkworksd/src/taskmaster/mod.rs::RecommendationType/Recommendation` currently supports improve_workflow and cleanup | Future typed plan-review intent, immutable proposal revision and source/change/evidence binding. Suggested strings alone are insufficient. |
| Session/location owner | `session_application.rs::CreateSessionCommand` and effective worktree validation | Resolve exact source/runtime and canonical location; validate ordinary creation eligibility without interpreting branch names as identity. |
| Artifact owner | `branch_documents.rs::Document::selection_id`, `branch_documents_application.rs::read_document_content/request_document_review` | Preserve anchored worktree, selected kind/path/content identity and fresh read checks; selection hashing normalizes CRLF. |
| Desktop boundary | `electron/main.ts::withReadyBackendGeneration`, `electron/planOpener.ts` | Existing authenticated generation-scoped transport; independent preload/renderer types and narrow exact-subject inputs. |
| Ordinary Start owner | #68; existing session creation path | Durable duplicate/pending/recovery admission and resulting recommendation/session linkage before claiming at-most-once launch. |

The source API currently returns document content separately from session and
recommendation fields. Neither that response nor a completion packet supplies
the combined binding defined here. This proposal does not add a current review
recommendation type, qualify a role adapter, activate hierarchy storage or alter
passive Fix with AI / cleanup behavior.

## Proposed immutable preview record

Use a bounded volatile sidecar-owned inspection cache rather than a new durable
approval ledger or renderer-computed digest. A preview ID is a lowercase UUIDv4,
not authentication or a grant. Only the existing authenticated desktop boundary
may prepare/read/consume its entries; reporting capabilities cannot do so.

Each entry freezes these owner-validated inputs and their display projection:

- sidecar/workspace incarnation and canonical workspace identity;
- typed ordinary plan-review recommendation ID, immutable proposal revision,
  current eligible status, source/change/evidence binding;
- exact source session/runtime identity and lifecycle, effective worktree and
  Git common-directory/branch-or-detached identity;
- resolved proposed reviewer tool/model and configuration/capability revision,
  instruction-only access description and validated review directory;
- selected document kind, anchored reference, artifact-owner selection ID,
  explicit pin state and a source-owned material artifact revision;
- complete proposed user instruction as UTF-8 bytes, with the plan identified
  as a validated reference, not embedded document content;
- the exact editable draft values that produced that instruction.

Human titles/status strings are display metadata, not identity. Their changes
alone need not invalidate the binding; lifecycle/eligibility changes do.
The proposal owner must define a revision for every material source/change or
instruction/configuration change; an ABA return to the same visible text cannot
restore old validity. No updatedAt timestamp or branch label substitutes for it.

The entry's validated internal owner references need not be exposed as absolute
document paths. Return source/reviewer worktree display labels through the
review-context projection, and the plan's repository-relative path plus anchored
worktree display identity and opaque selection ID. This is a proposed narrow
display projection, not an extension of content-only access into a filesystem
read API. No credentials, role secrets or provider-internal prompts are included.

Proposed finite defaults, requiring owner acceptance: at most 8 entries per
workspace incarnation; 30 seconds from preparation with no sliding renewal;
at most 64 KiB encoded entry/response, 24 KiB encoded request and 16 KiB user
instruction. Count complete UTF-8 bytes before storing/returning; bound the raw
request before JSON decoding. Reject oversized input rather than truncate.
Reject duplicate/unknown JSON keys and non-string or invalid identifiers.
Existing stricter artifact/configuration/creation limits still win. No document
content is cached; existing document read/discovery budgets remain unchanged.

Artifact-owner revision/pin binding is a prerequisite, not an existing property
of `selection_id`: that ID omits pin state and mutation history. The artifact
owner must qualify a revision that changes for every observed material selection,
pin, anchor or content publication. Returning to the same selection bytes after
such a publication cannot restore a preview. Unobserved external filesystem
A→B→A changes between validations are outside this history guarantee.

Opening/refreshing may add only an ephemeral inspection entry. It cannot change
recommendation lifecycle, source association, pins, current selection, unread,
attention, configuration or approval. Expired entries may be discarded. When
8 unexpired entries exist, return unavailable; do not evict another active view
or invalidate it merely because a second window opens the same recommendation.
Workspace/sidecar replacement destroys the cache. No preview is restored from
disk, logged with its instruction, or exported through session metadata.

## Proposed prepare/read interface

Subject to #68's typed proposal source, add one authenticated operation:

`POST /taskmaster/recommendations/:id/review-preview`

Closed request shape: `schemaVersion:1`, `expectedProposalRevision` (canonical
positive u64 decimal string), and `draft` (either null for the owner's proposed
values, or exactly `codingToolId`, `modelId`, `userInstruction`, all strings).
A changed proposal revision returns changed; it is never silently prepared as
the old subject. Empty/unsupported choices are unavailable, not default picks.
The reviewer configuration owner resolves effective role/skill settings; callers
cannot supply source IDs, worktree paths, capability claims or configuration
receipts. Directory is fixed by the initial-scope rule above.

Response is a closed version-1 union:

| State | Fields / behavior |
| --- | --- |
| available | `previewId`, `remainingMs`, complete `view`; future consuming Start may be offered only when its own gates are qualified |
| unavailable | stable `reasonCode` and complete bounded `view` where safe; no consumable preview ID |
| changed | `reasonCode`; no replacement instruction or binding installed automatically |

`view` has the accepted design's source identity/tool/runtime, proposed
reviewer role/tool/model, separate work locations, artifact available/missing/
unavailable state, full instruction and launch-gate explanation. Unsupported
subject, absent/changed source, invalid location/artifact, unqualified reviewer,
limit exhaustion and generation change must be distinguishable reason codes.
Reason codes are closed: `unsupported_subject`, `proposal_changed`,
`source_unavailable`, `location_unavailable`, `artifact_missing`,
`artifact_unavailable`, `configuration_unavailable`, `cache_full`,
`generation_changed`, `oversized`, `expired` and `busy`. Proposal/generation
mismatch is changed; other preparation failures are unavailable. Unknown version
or malformed shape is a request error, not a new reason code.
Only available entries are cached. Do not return partial old/new fields or a
usable ID with a missing required plan. HTTP authentication failures expose no
view; malformed requests are rejected before preparation.

Electron binds request completion to its current backend/workspace generation.
The renderer binds it to the current recommendation, draft revision and local
request sequence. Any superseding edit, close or scope replacement drops late
responses. A changed/unavailable response preserves the old inspected context
labelled stale until explicit refresh; it does not edit the draft or retry.
A local expiry timer is advisory UI state only; the sidecar clock decides expiry.

## Visible currentness while open

Propose one authenticated status read, with no draft or replacement view:

`GET /taskmaster/recommendations/:id/review-preview/:previewId/status`

It revalidates the same owner-controlled revision set and anchored artifact as
preparation. Return `current` with remaining lifetime and a server check time,
`changed` with a nonempty unique set of changed fields, or `unavailable` with one
of the closed reason codes above. Changed fields are exactly `proposal`,
`sourceRuntime`, `sourceEligibility`, `sourceLocation`, `reviewerConfiguration`,
`reviewLocation` and `artifact`. Expired/unknown entries return unavailable
without revealing another workspace's data. A changed entry is permanently
invalidated in this cache; a later equality cannot reactivate it.

The renderer polls only while the detail is open and visible, once every
5 seconds (a proposed default), with at most one request in flight. The sidecar
admits at most one validation per entry at a time and returns busy rather than
queueing more work. Existing owner-projected changes invalidate the displayed
binding immediately when their exact identity is available; no existing event
is assumed to carry an unqualified runtime/artifact revision. Initial preparation
already supplies a fresh check. On visibility regain, require a new status check
before offering Start; on close, replacement or any local draft edit, cancel
and discard old completions. Expiry requires explicit refresh, not automatic
preparation. A status read never renews the 30-second entry lifetime.

Display the last validation age instead of claiming continuous filesystem
observation. A changed response identifies the changed fields and labels the
old context stale; it never replaces the instruction. Source-runtime replacement
invalidates pending approval even when names/branches match. Existing workspace/
backend replacement clears the detail. Busy/unavailable responses disable the
consuming action; the next scheduled status read is not a launch retry. Local
request sequencing rejects late status responses after a new preview or edit.
Start always revalidates independently; no status result authorizes it.

## Coherent read and consuming Start

Prepare reads one stable proposal/source/configuration revision set, obtains
its selected artifact through the artifact owner, then revalidates the same
set and anchored document before publishing the entry. A changing or unavailable
set returns changed/unavailable, never an unbounded retry loop. A cache hit
is not currentness evidence: every new preparation performs owner validation.

Future #68 ordinary Start consumes only the recommendation ID, `previewId` and
its own canonical idempotency/attempt identity through the authenticated desktop
path. No replacement prompt/tool/directory fields are accepted at that boundary.
The launcher reads the frozen values, not the renderer's display strings.

Admission must serialize owner-controlled source/proposal/configuration checks
with its durable attempt reservation before any creation effect. The scoped
implementation plan must prove the lock/transaction ordering and existing
creation-path integration; this document grants no new command lane. A newer
proposal, source runtime/lifecycle, location, configuration, artifact selection,
workspace incarnation or expired entry denies admission before creation. Material
draft edits require another preview and explicit Start. No fallback tool,
implicit current-session injection or automatic retry.

Artifact content remains a reference: re-read and compare the anchored selection
and source-owned artifact revision/pin binding
at the final validation boundary, including same-path content changes, pins and
branch/worktree replacement. External filesystem/Git writers do not participate
in sidecar locks. This guarantees validation at that boundary, not immutable
files while the reviewer runs, native confinement, or transactional locking of
external Git. If a consumer needs immutable artifact custody or a stronger
physical guarantee, it remains unavailable until that owner's contract qualifies
it; never infer such a guarantee from the preview cache.

#68's durable attempt owner is authoritative for duplicate Start. An exact retry
of an already admitted attempt resolves its retained result/pending status before
consulting an expired or lost preview. Conflicting payload/preview identity for
that attempt is rejected. An unknown outcome never spawns again; sidecar restart
loses preview entries but cannot reset launch deduplication. A fresh Start needs
a new inspected preview and is still subject to the retained prior-attempt gate.
The reservation/receipt schema, retention and crash-recovery integration must be
accepted under #68 before this consuming path is implementable. This proposal
is not an alternative ledger and does not change recommendation completion or
user result acceptance.

## Approval, compatibility and handoff

Owner choices requested by this proposal: ordinary plan-review-only/current-root
initial support; authenticated volatile inspection cache and its numerical caps;
reference-only artifact transport; 5-second visible status validation;
artifact-owner revision/pin prerequisite; immutable exact draft at Start; the explicitly
bounded filesystem guarantee. Accepting these choices does not close #68's
producer/admission storage or #610/#746's capability and projection gates.

No current wire enum, stored recommendation, route or IPC type changes in this
design task. Old clients keep passive recommendation behavior. Future negotiated
support must return unsupported/unavailable for absent review-preview capability;
never deserialize an unknown type as improve_workflow. Runtime implementation
requires a scoped ADR under the normal lifecycle, accepted source/attempt codecs
and a reviewed executable plan. Reverting this document has no data effect;
a future cache can be removed by restart, while admitted attempts remain under
#68's durable owner rather than being erased with volatile inspection state.

| Qualification case | Required evidence before production connection |
| --- | --- |
| Shared branch, different sessions | Exact source/runtime retained; source and reviewer locations displayed separately |
| Observed artifact A→B→A publication or changed pin/anchor | Artifact-owner revision/pin binding rejects the inspected entry even when selection ID returns to its earlier value |
| Unobserved external filesystem A→B→A between checks | No history-detection claim; validate current content at the stated boundary |
| Edit model/tool/instruction after preview | No old preview consumption; new complete view and explicit Start |
| Ordinary versus #610 child request | Child consumer rejected by this route; exact-plan authority unchanged |
| Concurrent open, expiry and full cache | No silent active-entry eviction; fixed TTL and bounded unavailable response |
| Read or Start races with controlled mutations | Both serialization orders exercised at actual admission boundary |
| External file change after validation | Document the stated boundary; do not claim native exclusion or immutable input |
| Duplicate Start, lost response or restart | Retained attempt lookup prevents another spawn despite lost preview |
| Open view, material owner change or visibility regain | Status validation identifies changed fields; no replacement view, TTL renewal or stale Start enablement |
| Close, workspace/backend/source-runtime replacement | Old responses and preview IDs cannot restore or launch context |
| Invalid/oversized input, auth failure, unsupported version | Bounded rejection, no source mutation, prompt submission or creation effect |

The next work after scoped acceptance is source-owner contract closure and its
finite codec/admission fixtures, followed by an executable implementation plan.
No model call, native probe, production adapter or runtime test is authorized
by this design's existence or merge.

## Bounded design review

One independent correctness pass and one fresh builder check identified two
material omissions, corrected here: open-view status revalidation, and the
selection ID's inability to represent pin/history changes. The proposed status
operation, polling bound and artifact-owner revision prerequisite resolve the
consumer contract while retaining the explicit external-filesystem limitation.
Source codecs, durable attempt admission and runtime qualification remain gates;
this review is not evidence that those prerequisites exist.
