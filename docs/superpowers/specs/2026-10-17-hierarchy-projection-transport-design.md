---
type: Design Proposal
title: Hierarchy snapshot and navigation transport
description: Proposed snapshot ordering, exact navigation targets, and lossless display encodings for hierarchy consumers.
tags: [orkworks, hierarchy, transport, review]
status: draft
---

# Hierarchy snapshot and navigation transport

Tracking: [#746](https://github.com/Rambolarsen/orkworks/issues/746), with producer
contracts [#742](2026-10-04-orchestrator-preparation-design.md) and
[#744](2026-10-04-assignment-evaluation-design.md).
Status: proposed written contract for owner and producer review. This does not
accept upstream contracts, select HTTP routes, or authorize runtime work.
#740 capability evidence remains deferred until November; profiles remain no-go.

## Boundary and alternatives

The [projection mapping](2026-10-15-hierarchy-projection-mapping.md) defines source
joins and consumer behavior. This proposal supplies transport metadata around
those joins. Canonical configuration/result/evaluation records and their digest
bytes remain unchanged. No renderer-held authority, root skill telemetry,
new command, cross-workspace aggregation, or automatic retry is added.

Use a complete, run-scoped read envelope and exact navigation descriptors.
Applying event deltas would require an additional replay/reconciliation protocol;
reusing source result revisions would miss coordination, session and attention
changes. A separate snapshot counter makes those concerns explicit. It is
volatile and orders reads; #744's durable summary revision still identifies
material summary changes across restarts.

The initial read is for one exact run in the selected workspace. It cannot
establish that the workspace has no other runs/actions. Workspace run discovery
and its completeness contract remain a separate planning boundary. Multiple
run envelopes are never presented as one atomic workspace snapshot.

## Snapshot envelope and ordering

All envelope fields are required; reject duplicate keys, unknown properties,
unknown tags and invalid values before accepting a response. IDs and digests
use #741's definitions; generation encodings use #742. Version 1 has exactly:

| Field | Type / meaning |
| --- | --- |
| `schemaVersion` | JSON integer `1` |
| `workspaceId` | Exact selected workspace ID |
| `runId` | Exact requested run ID; no path/label inference |
| `sidecarGeneration` | Current opaque 64-character lowercase hex lifetime |
| `snapshotVersion` | JSON integer `1..2147483647` for a tracked admitted run; `null` only for the stateless unavailable responses below |
| `state` | One of the two tagged records below |

`state` is exactly either:

- `{kind: "complete", projection: <validated run projection>}`.
- `{kind: "unavailable", reason: <reason>}` with reason from
  `not-found`, `source-unavailable`, `invalid-structure`, `limit-exceeded`,
  `reconciling`, or `version-exhausted`.

These are record shapes, not executable JSON fixtures. The run projection
contains the mapped hierarchy, source-bound details and navigation descriptors;
its complete field schema remains a producer/consumer review gate. Missing
records that the mapping explicitly allows, such as an absent ordinary session
for a valid retained assignment, are represented visibly inside a complete
projection. An invalid ownership join is unavailable for the run. Unavailable
never includes a partial projection or means an empty tree/No actions.

Allocate counters only after verifying an existing admitted run under #742's
admission fence, never merely for a requested ID. An unknown/nonexistent run
returns stateless `not-found` with `snapshotVersion: null`; it creates no
counter, negative-cache entry or tombstone. There are at most 1,024 tracked run
slots per workspace/lifetime, each at most 512 serialized metadata bytes, with
512 KiB aggregate metadata. A new admitted run when slots are exhausted returns
stateless `limit-exceeded` with `snapshotVersion: null`; existing tracked runs
remain readable. Projection quota exhaustion does not block #742 admission or
source corrections. These are proposed read-resource bounds, not a run-history
retention limit.

A tracked slot starts at 1 and is retained for the entire sidecar lifetime,
even after source deletion, so late responses cannot join a recreated counter.
For a tracked run deletion advances to a versioned `not-found`; it never becomes
a stateless response or frees the slot. No eviction, reset or ID reuse within
that lifetime is allowed. A genuine new lifetime may reconstruct slots only
from admitted retained runs; it cannot evade #742's retired-run admission fence.
Stateless responses are unavailable observations, not ordered snapshots: accept
them only for the latest still-current local request context, disable decisions,
and compare no counter. A later fresh read revalidates admission and may obtain
a tracked snapshot; no stateless response establishes durable run retirement.

The sidecar owns each tracked counter for its lifetime.
Every committed change that can alter visible projection content or navigation
validity invalidates the prior complete version and advances once with checked
arithmetic. This includes canonical session lifecycle/attention changes,
coordination, allocation, report/evaluation freshness, summary changes and
source deletion; it excludes polling, client view state and timestamps added
only for transport diagnostics. Equal versions identify identical validated
content within the same scope. A correction round trip advances twice.

Source mutation and invalidation serialize before serving a complete response.
Build a coherent read under that boundary, or capture source revision tokens and
revalidate every dependency before publication. A change during assembly causes
a bounded retry or `reconciling`, never a mixed-generation/currentness response.
The runtime plan must prove the chosen boundary across ordinary session data and
#742/#744 transactions; an event notification alone is insufficient.

Unavailable responses also carry the current counter. Availability/reason
changes advance it. At exhaustion fence the run projection and serve
`version-exhausted` at the maximum version, never reuse it for a new complete
state. The terminal `version-exhausted` response dominates **every** state at
the maximum version, complete or unavailable, in either arrival order. Once
accepted, reject all subsequent non-exhausted states in that scope, including
late `reconciling`, `not-found` or `limit-exceeded`; do not classify these as an
equal-version content conflict or loop on refresh. The exhaustion fence persists
until the consumer leaves that lifetime, independently of request failure state. Do not block authorized source corrections or restart a sidecar merely
to evade exhaustion. A genuine new lifetime starts its own counter; it cannot
restore reporting or approval authority.

Electron main obtains the current lifetime from its owned sidecar lifecycle,
not from whichever response arrives first. Renderer selection/request tokens
add local fencing. Before accepting a response validate selected workspace,
requested run, current lifetime and the still-current local request context.
Within that scope reject lower numeric versions; accept equal versions only as
identical data retries, except the terminal exhaustion fence above. A successful
complete response to a new latest current-context refresh clears a transient
local refresh-failure fence even at an unchanged equal version; this changes
request health/decision availability, not projection data, viewed summaries or
usage highlights. An old request or unavailable response cannot clear the fence,
and no success clears an exhaustion or unresolved equal-version-conflict fence. Conflicting equal-version
content makes the projection unavailable and triggers authoritative refresh.
A failed latest refresh disables displayed workflow decisions even if a retained
snapshot remains visible as stale. Later successful refresh may restore them;
no automatic action follows. An older request cannot clear that failure fence.

Events only request a new complete read. Missing/duplicate events and reconnect
never update source state by inference. Clearing scope on workspace/run switch
or sidecar replacement rejects old in-flight responses; counters from different
scopes are never compared. A successful fresh read may expose revalidated retained
review without reviving its old write grants or resetting summary lineage.

## Navigation targets and authority

A descriptor opens the exact decision's inspection surface. It does not submit
a decision or encode approval, readiness, quiescence or permission. Version 1
has exactly `kind`, `target`, and `label`; `label` is bounded display text of
1..512 UTF-8 bytes and has no lookup authority. Each `target` contains exactly
`workspaceId`, `runId`, `sidecarGeneration`, `expectedRunVersion`, and `subject`.
The run version is #742's JSON integer `1..2147483647` from the coherent snapshot.
No bearer, UI token, launch grant or arbitrary filesystem path is exposed.

| `kind` | Exact tagged `subject` properties beyond `kind` |
| --- | --- |
| `inspect-plan` | `kind: "plan"`, `planId`, `planRevision`, `planDefinitionDigest`, `preparationRevision`, `inputDigest` |
| `answer-clarification` | `kind: "clarification"`, `questionId`, `requestVersion`, `preparationRevision` |
| `inspect-task-review` or `inspect-quiescence` | `kind: "task"`, complete #744 child `assignmentIdentity`, `launchGeneration` |
| `inspect-run-recovery`, `inspect-stage-continue`, or `inspect-research-finish` | `kind: "run"`, `parentSessionId`, `parentRuntime` |

Only the listed kind/subject pairs are valid. Target IDs, versions and digests
come from #742's current records, with #741/#744 assignment joins; no synthetic
upstream decision ID is presumed. `requestVersion` is the clarification request
version under #742, not a new counter. The task identity includes its exact task
version/reservation/session/configuration, and generation means the current
child launch. `planDefinitionDigest` denotes the approved/proposed immutable
plan's definition digest; its upstream field mapping needs producer agreement.
Unavailable required fields disable the descriptor rather than supplying defaults.
A task without a bound assignment has no task descriptor. `parentRuntime` is exactly `{kind: "current", generation: <launch generation>}`
or `{kind: "absent"}`, validated against authoritative run/session state.
A known absent runtime may have an inspect-run-recovery descriptor; absence is
not unknown source data and does not authorize replacement/resume. Applicability for every run kind remains owned by #742; navigation adds no
new current-runtime requirement to an otherwise permitted UI decision.
Missing/corrupt runtime state disables the descriptor rather than being
treated as absent. Starting/ending a runtime changes
the run version and binding; stale absent/current targets cannot be retargeted.

The sidecar emits descriptors only for currently applicable decisions. Descriptor
identity is the complete kind/target tuple, excluding label; no timestamp,
list index, summary token or mutable label substitutes for it. Source decision
changes must advance run/task/request versions or alter the exact bound digest/
generation under their owning contracts. Upstream mutation coverage must be
reviewed before these targets can support runtime actions.

On user activation, Electron main validates kind/shape, current selected workspace,
owned lifetime and current run version, then retrieves the exact subject through
its authenticated sidecar path. The sidecar validates every subject member and
current applicability under #742's serialized authority boundary. A stale target
opens no actionable confirmation: refresh and require another explicit user
selection. Do not silently retarget to the current plan, child, question or run.
Ordinary node inspection may retain stale details with an explicit label.

The resulting inspection surface may offer existing explicit UI decisions only
through separately reviewed command contracts. Those commands must revalidate
at mutation time, including exact plan/input digests, task/clarification versions,
runtime generations, capability evidence and current authority as applicable.
Passing navigation validation is not a grant and does not close the read-to-write
race. No command routes or approval schemas are finalized by this proposal.

## Lossless display encoding

Use explicit display DTOs instead of exposing canonical JSON through ordinary
JavaScript parsing. Decode canonical integer tokens losslessly in Rust; construct
display values before serializing to Electron/preload/renderer. Never parse a
canonical u64 through JavaScript Number and then convert it to a string.

| Projection value | Version-1 encoding / validation |
| --- | --- |
| #742 bounded counters, `snapshotVersion`, result/evaluation revisions | Plain decimal JSON integers in their existing ranges; no exponent/fraction/coercion |
| `runSequence`, admission high-water | Preserve #742 canonical decimal strings and maximum `9223372036854775807`; zero only where source permits |
| #744 `subjectRevision`, artifact `version`, `sizeBytes` | Canonical decimal strings `"0".."18446744073709551615"`; artifact versions exclude zero; preserve any narrower source limit |
| `summaryRevision` | Preserve #744 string `"1".."2147483647"`; equality only within complete assignment identity |
| Runtime generations, IDs and digests | Exact source strings; no integer conversion or normalization |

Decimal strings contain ASCII digits only, no sign, whitespace, leading zero
except `"0"`, fraction or exponent. Validate length/value losslessly; preserve
strings in IPC and state. Decimal-length comparison then lexical comparison for
equal lengths, or checked BigInt parsing, may order permitted counters within
the same scope. Never lexically order arbitrary-length strings or compare
summary revisions across assignments. JSON schema validation must reject numeric
u64 display fields even for values currently below 2^53.

The display conversion is field-specific, not a recursive stringification of
all numbers. Source canonical objects retain #744 integer tokens and original
digest bytes. Do not recanonicalize a display DTO to verify a source digest or
submit it as a canonical candidate/evaluation. Any future canonical artifact
retrieval needs a separate exact-byte/lossless codec contract. Unlisted source
counters, including #743 workspace sequences, need an explicit producer bound
and encoding before inclusion; do not guess a range from example payloads.

## Required contract cases and remaining gates

| Case | Required disposition |
| --- | --- |
| Response 8 arrives after 9 in the same lifetime | Discard 8; duplicate 9 changes no projection/viewed/highlight state; fresh successful 9 may restore request health |
| Complete 9, failed refresh, then a fresh successful unchanged 9 | Clear transient failure fence and restore current decision availability; preserve projection/viewed/highlight state |
| Arbitrarily many unknown run IDs | Stateless not-found/null; allocate no counters or negative cache |
| Slot 1,024 allocated, then another admitted run is read | Stateless limit-exceeded/null for new run; retain all existing slots and source authority |
| Tracked run is deleted and late pre-deletion response arrives | Versioned not-found in retained slot; reject lower version, never recreate at 1 |
| Equal version has different source content | Projection unavailable; no decisions until authoritative reconciliation |
| Old lifetime version 50 arrives after fresh lifetime version 1 | Reject old lifetime; preserve only freshly revalidated retained review |
| Attention changes while hierarchy bytes are assembled | Retry coherently or unavailable; no stale No actions result |
| Latest refresh fails while older request later succeeds | Keep failure fence; require a new successful current-context refresh |
| Plan revision/input changes after descriptor selection | Navigation/mutation reject stale target; no implicit approval of replacement |
| Child resumes with same session ID and new launch generation | Reject old task descriptor, even if summary remains unchanged |
| Recovery target says absent but a parent runtime has since appeared | Reject stale run version/runtime binding; never start a replacement |
| Clarification answered or research run finished concurrently | Serialized current-version/applicability checks reject stale descriptor |
| Counter maximum followed by source correction | Serve terminal version-exhausted, accept authorized correction independently; late complete or unavailable non-exhausted maximum cannot revive UI in either arrival order |
| u64 `9007199254740993` and `18446744073709551615` | Preserve distinct exact strings through display transport; reject number, overflow, leading zero and exponent forms |
| Source bytes versus display DTO | Original canonical digest verifies only original bytes; source integer and display string are deliberately distinct |
| One complete run envelope among unavailable/undiscovered runs | No workspace-wide No actions or completeness claim |

This draft is capability-independent specification work. Review #741/#742
source mappings, counter invalidation coverage, exact target applicability and
#744 display/source separation with the component owners. The envelope payload
schema, bounded collection/response limits, read authorization, concrete routes,
preload interfaces and serialization/recovery implementation remain executable-
planning gates. #744 material bytes/storage/recovery and #743 sequence encoding
remain separate unresolved producer decisions. None is silently settled here.

After written acceptance, reconcile the mapping and owning proposed contracts;
then prepare separately approved runtime plans only when their capability and
#610 gates permit. Keep #746 open. Documentation validation and synthetic cases
do not establish an eligible profile, native usability/accessibility evidence,
or runtime delivery.
