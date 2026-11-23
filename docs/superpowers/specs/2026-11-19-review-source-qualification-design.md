---
type: Design
title: Review artifact revision and Start admission sources
description: Proposed source bindings and fail-closed durable admission contract required by the accepted review preview design.
tags: [orkworks, review, source-contracts, admission]
status: stable
---

# Review artifact revision and Start admission sources

**Status: source choices and offline plan accepted 2026-11-19; no runtime implementation or migration.**
The owner authorized this qualification follow-up after accepting
[#1913](https://github.com/Rambolarsen/orkworks/pull/1913). Existing
[production binding approval](2026-11-19-review-handoff-binding-design.md)
remains valid. The owner accepted the missing source mechanics, durable-record choices and
bounded offline plan with “keep going”. Acceptance does not qualify production
integration or physical durability.
Tracking: [#1899](https://github.com/Rambolarsen/orkworks/issues/1899) and
[#68](https://github.com/Rambolarsen/orkworks/issues/68).

## Source audit and decisions

Inspected at main `1cdb460c6c7b0082afa8cfcb8355c922a5d01270`:

| Source | Established behavior | Consequence |
| --- | --- | --- |
| `branch_documents_store.rs::save` | Checked AtomicU64 generation increments before publication, including failed writes; report/reconcile/try_publish/use_latest reach save under the branch selection lock. | A conservative store-wide epoch already exists. Do not invent a durable per-document revision merely to bind a 30-second preview. |
| `branch_documents_store/recovery.rs::reset_invalid` | The same checked generation advances before explicit recovery replacement under the branch lock. | Recovery also invalidates an old epoch even if a file later looks identical. |
| `branch_documents.rs::Document::selection_id` | Hashes branch, anchor, path and normalized content; omits pin and history. | Bind explicit pin state and epoch as well as fresh selection identity. |
| `session_application.rs::create_session_workflow` | Allocates a UUID internally, inserts a runtime handle, persists creating metadata and schedules a detached startup task. | There is no durable caller attempt lookup or reserved-ID input for the new recommended Start consumer. A creating response is not proof of successful spawn. |
| `resolve_session_launch` and creation workspace reconciliation | Ordinary creation can resolve generic-shell/default-model fallback and reconcile a changed current workspace. | The recommended consumer cannot reuse this path unchanged: its inspected tool/model/workspace must be exact. Preserve manual creation behavior separately. |
| `metadata.rs::write_atomic` | Flushes staging and publication; a post-publication failure can leave new bytes visible. | A write error is an uncertain publication, not proof no record exists. |

Paths above are under `crates/orkworksd/src/`. These are source observations,
not executed runtime/physical durability qualification.

## Artifact owner binding: reuse a volatile epoch

Proposed initial binding is internal to the owning sidecar:

`{ exact Store instance, generation, branch identity, document kind, anchored reference, pinned, selectionId }`

Store identity is an owned/weak reference compared by identity, never an address
serialized to clients. Workspace replacement constructs another owner; sidecar
replacement loses all preview entries. Do not expose or repurpose the private
recovery nonce. The existing preview UUID is the only renderer-visible handle.
Generation remains the existing checked u64; it is not a new persisted field.

Capture selected record, explicit pin and current generation under the existing
branch selection fence. Perform bounded fresh repository/content validation
through the artifact owner, then compare the same owner/generation/material
selection before returning a binding. A mismatch returns changed, not repeated
retry. Existing fresh-read limits and optimistic revalidation still apply.

At status/Start validation, compare all these components and freshly validate
the selected content. An observed mismatch permanently invalidates that preview.
Hold the subject's existing selection fence through the consuming admission's
short final check; the executable plan must prove its lock ordering without
blocking terminal input on repository scans or queued I/O.

This is deliberately conservative: publication of another kind or branch,
identical repeated reports, or a failed save may advance the store-wide epoch
and require refresh even if this plan's bytes did not change. Copy must say
**Document context changed**, not assert that the plan content changed. There
is no claim that unrelated branch changes are serialized by this branch's lock;
the generation comparison is a conservative observation at the validation point.
Only actual subject mutations share its branch fence. A later unrelated epoch
advance need not cancel an already admitted attempt.

A status read observing a different content selection invalidates its preview
even if it performs no durable publication. Other non-publishing reads do not
create a global filesystem history. Existing scans/publications contribute their
actual observed changes; unobserved external A→B→A remains outside the accepted
history guarantee. No watcher, file-content cache, TTL extension or read-triggered
source association/pin write is introduced.

At generation exhaustion, deny new artifact bindings until a new owner is
constructed; never wrap/reset the same owner's epoch. Detached or legacy
session-local documents without this qualified shared owner remain unavailable
for the first production consumer. The fixture remains usable.

## Ordinary Start attempt owner

Propose a dedicated workspace-owned **ordinary review admission** registry,
separate from preview cache, ordinary lifecycle records and #610 plan grants.
This is a new metadata contract requiring acceptance before implementation.
It does not activate the proposed SQLite session store or reuse hierarchy rows.

The first registry representation is one versioned JSON document under the
selected workspace metadata root, proposed name `ordinary-review-admissions.json`.
Use the existing workspace lease plus one registry writer lane; update the whole
bounded document through the checked staged/flush/replace/sync writer. A single
record/index update is therefore one publication, not independent file writes.
Malformed, unsupported or unreadable existing state blocks recommended Start;
never create an empty replacement. Ordinary manual creation is unaffected.

Proposed closed envelope: `schemaVersion:1`, `workspaceId` (64 lowercase hex
characters from the canonical workspace owner), positive canonical u64 decimal
`revision`, and `attempts` in insertion order. Each attempt has exactly:

- `attemptId`, `previewId`, `allocatedSessionId`: canonical lowercase UUIDv4;
- `recommendationId`, `sourceSessionId`: validated opaque ASCII identifiers
  `[A-Za-z0-9_-]{1,128}`; unsupported existing IDs are unavailable, not rewritten;
- `proposalRevision`: positive canonical u64 decimal string;
- `preparedInputDigest`: 64 lowercase hex characters from the qualified proposal
  owner's complete frozen launch-input recipe, not a caller assertion;
- `state`: one of the closed states below;
- `recordRevision`: positive canonical u64 decimal string;
- `reason`: null, or one of `stale`, `expired`, `source_unavailable`,
  `configuration_unavailable`, `storage_uncertain`, `creation_uncertain`,
  `creation_failed`, `source_owner_lost`.

The stored encoding is compact UTF-8 JSON with the envelope keys and attempt
keys in the exact order listed above, no BOM, whitespace or trailing newline.
All accepted values here are ASCII. Decode with fatal UTF-8 handling, parse,
validate the bounded closed shape, re-encode through the fixed-order writer and
require byte equality. This standard-library round trip rejects duplicate keys,
escaped alternate keys, reordered/noncanonical numbers and extra syntax without
inventing another general JSON parser. Never stringify an unvalidated arbitrary
tree. The same canonical encoder defines complete encoded byte charges.

Reason/state combinations are closed: reserved, dispatchCommitted, linked and
settled require null; rejectedBeforeCreate permits stale, expired,
source_unavailable, configuration_unavailable or creation_failed only when
creator non-entry is proven; unknown permits storage_uncertain,
creation_uncertain or source_owner_lost. A known failed creator call is not a
pre-creation rejection. Its actual session outcome remains with the ordinary
runtime owner rather than being inferred from this registry.

The encoded workspaceId must match the registry's owning metadata root and
authenticated admission scope. A copied registry from another workspace is
unavailable, not adopted. The caller cannot choose an alternate expected root
or supply a workspace identity as authority. No prompt, document bytes,
credential, provider output, raw exception or arbitrary path is retained here.
The exact frozen-input digest recipe still needs the typed proposal/configuration
owner codec under #68/#66; accepting this record does not qualify an opaque hash
or a reconstructed subset as proof of those inputs.

Proposed caps: 256 retained attempts, 4 KiB per complete encoded attempt and
2 MiB for the complete encoded registry, whichever saturates first. Reject
unknown/duplicate keys, invalid IDs/states/counters and counter overflow. No
automatic eviction, revision reset or time-based forgetting. At saturation,
new recommended Starts are unavailable; exact retained retries remain readable.
Retention/export/reset policy must be reviewed separately before any space can
be reclaimed. These are new limits for owner acceptance, not inherited preview
cache limits.

## State and duplicate contract

| State | Meaning and replay |
| --- | --- |
| reserved | Attempt and session ID durably allocated; no creator invocation is permitted yet. Replay returns pending, never creates again. |
| dispatchCommitted | Durable permission for this owner to attempt creation once has been published; a crash after this point is conservatively uncertain. Replay never invokes the creator. |
| linked | Creator published the exact allocated creating-session identity. This is admission, not proof of running, completion or acceptance. Replay returns that session identity with current availability. |
| rejectedBeforeCreate | This live owner proved the creator was never entered and durably recorded the reason. Replay returns the original rejection. |
| unknown | Publication/creation outcome or owner identity cannot be established. Replay reports unavailable/pending recovery; never respawns. |
| settled | The exact owning runtime's terminal/cleanup disposition is qualified and retained. Replay returns historical admission/availability; never resumes or creates. |

All transitions increase the attempt and envelope revisions; no state moves
back to reserved. The allowed forward transitions are reserved→dispatchCommitted
or rejectedBeforeCreate; dispatchCommitted→linked, rejectedBeforeCreate or
unknown; linked→settled or unknown; reserved→unknown on uncertain publication or
owner loss. unknown may become linked/settled only with qualified exact owned
creation/runtime evidence, never a matching title/path or inferred process exit.
rejectedBeforeCreate and settled are terminal. Reads do not perform transitions.

The stable conflict slot is **one outstanding ordinary plan-review attempt per
source session in this workspace**, across recommendation/proposal revisions.
It is derived from retained records, not a second independently written index.
Reserved, dispatchCommitted, linked and unknown occupy the slot. Only qualified
rejectedBeforeCreate/settled release it. Another recommendation ID or preview
cannot bypass a prior unknown attempt. Normal Taskmaster deduplication and its
review-chain policy still apply after slot release.

An exact retry must match its original attempt ID, recommendation ID and
preview ID. Any different tuple is conflict, even if its new instruction would
look similar. Lookup happens under authenticated current-workspace ownership
before checking preview expiry/loss; it cannot leak another workspace's result.
A new attempt requires a fresh inspected preview, eligible source and a free
slot. There is no automatic retry or new-attempt generation by the UI.

## Admission ordering and crash boundaries

1. Under the registry writer, resolve exact retry/conflict and reserve a fresh
   session ID for the fresh attempt. Any uncertain registry write returns
   unavailable and fences new admission until exact reconciliation; a successful
   later read alone is not a durability proof after a failed flush.
2. Publish `dispatchCommitted` durably **before** entering any session-creation
   code. This is not the final source validation. A crash before the call may
   leave an unknown attempt even when no process started; that conservative
   cost is explicit, not grounds for replaying it.
3. Revalidate the unexpired preview and controlled proposal/source/configuration/
   artifact bindings at the existing short admission fences, after all slow
   preparatory work. Staleness here is a known rejection only if this live owner
   proves the creator was never entered and can persist that disposition.
4. A private prepared-creation seam consumes the allocated session ID, exact
   workspace and resolved tool/model/initial instruction once. It must reject
   generic-shell/default-model fallback, workspace reconciliation/retargeting
   and ID collisions for this consumer. It must publish the creating record
   under the same accepted admission boundary before deferred spawn. The
   existing public manual-create request does not gain an arbitrary session ID
   or a review authority flag.
5. On known creating-record publication, retain linked; on uncertain creation or
   result publication, retain/report unknown. Losing the HTTP reply after linked
   resolves the retained identity on exact retry. A creating response does not
   declare startup success, release the conflict slot or complete the recommendation.

This ordering requires a reviewed prepared-creation interface and actual
side-effect boundary tests. The current generic creator is not qualified for
this contract merely because it accepts an initial prompt. No disk flush or
repository scan is added while holding a hot terminal-input fence by this design;
implementation must prove that all final owner validation and first creation
mutation share the required ordering using the existing short admission pattern.
If that cannot be established, the consumer remains unavailable rather than
relaxing the stale-input guarantee.

On reopen, reserved/dispatchCommitted records have lost their live invocation
owner; recovery treats them as unknown, not as permission to finish the launch.
A matching ordinary metadata file is not live process ownership or proof of the
original initial prompt. Preserve the assigned identity and error state without
adopting a native process. Exact live/runtime and independent-instance recovery
gates remain unchanged. No preview cache restoration or provider/native probe
is authorized to resolve uncertainty in this task.

## Retention, compatibility and qualification gate

The proposed registry is a protected source. Workspace GC/retention/forgetting,
backup/restore and future JSON→SQLite cutover must account for every outstanding
and retained attempt before this registry may be written in production. Losing
it must never reset duplicate admission; absent-after-known-initialization is
unavailable, not first use. The installation/format marker and complete export/
restore proof belong to the storage integration plan; this document does not
claim those missing mechanisms exist. Old binaries do not read or enforce this
new registry, so activation/downgrade compatibility must be resolved before use.

| Qualification witness | Pass condition |
| --- | --- |
| Pin/unpin/recovery A→B→A with same final selection | Old Store epoch/pin binding rejects; new owner cannot reuse it |
| Identical/other-kind report or failed save | Conservative invalidation surfaced as context change, not fabricated content history |
| Fresh content mismatch, detached/missing/invalid source | No usable artifact binding; no durable association write from status read |
| Two Starts/same attempt, same source/different attempt | One reservation/invocation; exact replay resolves; competing slot denied |
| Crash/fault before/after every durable publication and creator call | No replay invokes creation; uncertain outcomes stay fenced |
| Source changes during slow work / final admission | Both race orders exercised at actual controlled-source boundary |
| Wrong session ID/tool/model/workspace or ordinary fallback | Prepared consumer denies before creation, manual creation contract unchanged |
| Linked response lost, then cache/sidecar loss | Attempt resolves assigned identity without new creation or native adoption |
| Counter/byte/count saturation and malformed state | Finite denial with retained exact retry; no eviction/empty fallback |
| Forget/GC/backup/cutover/downgrade | Duplicate protection survives, or admission remains visibly unavailable |

Disposition: **Source choices accepted; production admission remains Investigate.**
The accepted offline scope qualifies the artifact binding
and registry codec/state machine with finite offline oracles and denial/fault
vectors under a concise implementation plan. Those fixtures may prove logical
transitions/encoding only; source integration, physical durability, complete
storage/retention compatibility, producer-input digest and runtime admission
need their own evidence. No source or fixture implementation is authorized by
this proposal's existence or merge.

## Coupled offline qualification plan

The [finite component plan](../plans/2026-11-19-review-source-components.md) was
accepted in the same owner decision as this source contract. That acceptance
authorizes the logical-only codec/state fixtures under the unchanged design. It does not authorize application metadata writes, native
processes or production integration. Independent correctness and builder review
found no material issue in the source proposal; the plan was self-reviewed once
under the local plan policy. The [offline evidence](../../validation/ordinary-review-source-components.md)
records finite logical checks and their explicit source/physical limits.

## Launch-input follow-up

The [typed launch-input contract](2026-11-19-review-launch-input-design.md)
defines candidate logical proposal/configuration values and the exact prepared
digest composition. The owner accepted its narrower initial profile, source/byte choices and
offline qualification plan on 2026-11-19. Logical component evidence does not
qualify actual producers, effective leaf inputs or runtime delivery.
