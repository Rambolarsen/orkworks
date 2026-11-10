---
type: Design Proposal
title: Terminal correction UI scope and authority metadata protocol
description: Proposed bounded main-sidecar scope control, exact operation admission and versioned report-receipt authority metadata.
tags: [orkworks, hierarchy, reports, authority, protocol]
status: draft
workflow_status: proposed-for-owner-review
---

# Terminal correction UI scope and authority metadata protocol

Tracking: [#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#742](https://github.com/Rambolarsen/orkworks/issues/742), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `7af090ed684051900b6872955922d64c705a9766`.
**New protocol/codec choices proposed for scoped owner review.** The
[cutoff policy is already accepted](2026-11-10-terminal-correction-authority-design.md#scoped-owner-acceptance-and-handoff--2026-11-10);
this document does not ask to accept it again. New message/state/receipt choices
below are not accepted by drafting or merging this proposal. No fixture, route,
adapter, migration, lifecycle topology, native/model probe or runtime plan is
implemented or authorized here.

## Outcome, alternatives and ownership

Close the next written seam in the [qualification handoff](../plans/2026-10-29-terminal-report-publication-qualification.md):
which owner closes UI authority, how reordered messages cannot reopen it, and
which nonsecret metadata accompanies one durable correction. Reuse existing
Electron-only UI authentication, immutable report/request domains and the
[terminal/evaluation-only profile](2026-10-29-report-publication-profile-design.md).

Choose a memory-only per-lifetime scope high-water mark, rather than retaining an
unbounded set of closed UUIDs or treating fetch abortion as remote closure.
Choose a tagged version-2 report receipt storage value, rather than a separate
authority table or #744 claim/protected receipt slot. These are the author's new
technical choices. No quota is increased to make them fit.

| Owner | Responsibility / required seam |
| --- | --- |
| Electron main | Original application window/main-frame identity, selected workspace and sidecar lifetime, scope counter, fixed authenticated routes, local closure and final response checks. Renderer never chooses these transport identities or obtains a credential |
| Private sidecar UI authority owner | One original workspace/store/lease binding and memory-only scope/decision state; all matching closure, admission, retirement and failure transitions share its short latch |
| Existing single hierarchy worker | Immutable request preparation, source/consumer/epoch/health/quota validation, SQL transaction/disposition, durable receipt, snapshot invalidation and retirement. No replacement worker or second queue |
| Source and lifecycle owners | Real terminal creation, initialized receipt/usage history, complete consumers, exclusive lease and store disposition. This proposal cannot manufacture those Gate 2 proofs |

Source precedents and missing stages are traced in the
[accepted policy's source inventory](2026-11-10-terminal-correction-authority-design.md#source-trace-implemented-precedents-and-missing-stages).
In particular, `withReadyBackendGeneration` checks main's generation around a
request; `onCloseAdmission` cancels requests and advances it; `closeCurrentRuntime`
currently awaits sidecar stop before opening another workspace. None implements
this scope owner or independent hierarchy draining. The proposed owner must retain
the original store rather than resolve whichever workspace is current later.

## Closed types and identities

Use #741 canonical JSON and existing bounded ID/digest encodings. Counter is the
[storage contract's canonical u64 decimal string](2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation),
never a JavaScript Number. PositiveCounter excludes zero. Generation is the
existing #742 opaque 64-character lowercase hex sidecar lifetime. All fields are
required; unknown/duplicate members, coercions, leading zeros, overflow and
unrecognized variants reject. No token, filesystem path, timestamp or PID is an
identity substitute.

```text
UiScopeV1 = {workspaceId:Id,sidecarGeneration:Generation,
             scopeSequence:PositiveCounter}
```

The lifetime must equal the original authenticated readiness/store owner, not a
value learned from this request. ScopeSequence is main-owned, starts at 1 for
that lifetime, advances by one after closure, and never wraps or reuses a value.
Main binds the scope to its original window/main-frame and selected workspace.
Frame replacement closes its old scope; an intact main counter can allocate the
next scope for the new frame only after acknowledged closure and old-decision
retirement. Loss of the counter/owner binding or main-process restart instead
requires separately qualified owner retirement/new lifetime before further
admission; it cannot reattach an old scope. This is not an automatic sidecar
restart or recovery permission. Changing a scope never changes the source
run epoch, resets a per-run rate/version budget or revives child/grant authority.

This protocol supports one original workspace binding per sidecar lifetime. It
must not replace that owner with a new empty high-water state using the same
workspace/lifetime pair. Same-lifetime rebinding to another workspace is outside
this first protocol; a future lifecycle integration must use a genuinely new
qualified lifetime and preserve the one active-or-retiring hierarchy owner cap.
This selects no sidecar process topology and gives no permission to replace a
stalled owner. Namespace rotation closes the current scope and settles the old
decision before activation, retaining scope/admission high-water counters in that
same lifetime; it cannot reset them or reopen the old scope. Subsequent scope
opening is a fresh explicit action under the new epoch's source checks.

## Proposed fixed main-sidecar transport

These are proposed route names, not existing APIs. Use the existing private HTTP
server and `x-orkworks-open-plan-token` authorization. Authenticate before source
lookup or scope disclosure. Main checks the owned application window, main-frame
sender and app origin before deriving scope. Neither a renderer argument nor a
parent/child reporting bearer may select a control operation or authorize it.
Keep independently declared Electron/renderer contract types; no cross-boundary
imports. Scope control remains private to main, with no generic renderer HTTP bridge.
Open and correction require the currently selected workspace and live original
frame. Close and inspect may use only main's already retained original owner/scope,
including after its frame/workspace closes; they cannot resolve an arbitrary old
workspace from renderer input. Internal main closure callbacks do not require the
now-destroyed frame to send another IPC message. Sidecar validates the original
captured owner/lifetime rather than the newly selected workspace.

### Scope control

`POST /taskmaster/terminal-report-ui-scope` accepts exactly:

```text
{schemaVersion:1,operation:open|close|inspect,scope:UiScopeV1}
```

For every successful control response return exactly:

```text
{schemaVersion:1,scope:UiScopeV1,closedThrough:Counter,
 openScopeSequence:PositiveCounter|null,
 unsettledScopeSequence:PositiveCounter|null}
```

Scope echoes the validated request. The other fields are one memory snapshot of
that same original owner/lifetime. UnsettledScopeSequence names its one admitted
operation's scope while disposition/retirement remains incomplete, not a current
head or a promise to finish. A close acknowledgement proves only that the echoed
scope is closed: closedThrough is at least scopeSequence and it is not open.
It does not prove rollback, durable acceptance or lease release. Inspect grants
no new scope/operation permission and cannot be used as a stored bearer.

Requests must fit 4 KiB and responses 1 KiB, including complete JSON scope and
snapshot. These are proposed control ceilings, not an expansion of the existing
32 KiB correction-body or detail-response limits. Main permits one outstanding
open/inspect control and one close for the same owner; repeated same-target closes
coalesce locally. The sidecar admits at most one bounded open/inspect handler and
one reserved close handler outside the SQL queue; excess work returns bounded
unavailable, never an unbounded waiter list. Close has priority over new opens.

Open/inspect share the existing workspace UI read-attempt budget (60/minute and
two in-flight reads); the new handler bound is an additional smaller constraint,
not another allowance. Close does not wait for a read-rate token or SQL slot, but
uses only its one reserved handler and remains authenticated and size-bounded.
Saturation/failed transport is not a close acknowledgement. These proposed
control-slot/rate mappings need owner acceptance and native resource qualification;
they do not establish a wall-clock closure deadline.

### Correction submission

`POST /taskmaster/terminal-report-corrections` uses the existing #742 correction
JSON body/normalization and 32 KiB body ceiling. Main adds exactly these two new
transport headers alongside existing UI authorization:

- `x-orkworks-hierarchy-lifetime`: the captured Generation;
- `x-orkworks-terminal-scope`: the captured PositiveCounter.

The stream's workspace must equal the scope owner's workspace. Lifetime and
sequence must equal the currently open scope. A port, matching source digest or
valid UI token alone is insufficient. Header values are internal main-derived
metadata, bounded by their fixed encodings, never renderer capabilities. They are
excluded from the existing normalized request fingerprint as transport metadata;
source report/request/response bytes and hashes are unchanged. Sidecar still
sets user provenance and resolves the exact immutable predecessor under #742.

Main allows one pending terminal correction request per scope, including a
retained unknown disposition; a second local submission returns busy rather than
creating another queue. This is a proposed UI admission bound, not a new worker
or a change to the shared eight-waiter storage limit. It does not serialize other
ordinary session activity. A later exact retry requires an explicit current UI
action, fresh authorization/rate admission and the original normalized bytes;
there is no automatic resend or new-request retry.

## Memory-only authority state and transition table

Each original sidecar workspace owner has exactly:

- `closedThrough: Counter`, initially 0;
- `openScopeSequence: PositiveCounter|null`, initially null;
- `lastAdmissionSequence: Counter`, initially 0;
- one immutable active decision or null, and existing closing/health/lease fences.

An active decision retains UiScopeV1, its newly allocated admissionSequence,
complete canonical normalized request bytes and immutable proposed descriptor,
existing receipt key/expected predecessor/run bindings, and the original worker,
store/lease and source eligibility references. It is not serialized into a
bearer. All owned copies count against existing worker/queue/scratch limits.
Hashing, copying and physical lease checks occur outside the latch; latch admission
compares already prepared immutable identities and participating memory fences.
Physical-ownership/health races still need their native source-owner proof.

| Message / transition under the short authority latch | Exact result |
| --- | --- |
| Open n with n = closedThrough+1, no open scope and no unsettled old decision | Install n and return its snapshot, after current main/store/lease/health scope checks |
| Duplicate open of the same currently open n | Return the current snapshot without counters, storage or permission renewal |
| Open n at or below closedThrough | Deny closed scope; it cannot be reopened by message arrival |
| Open skips the next sequence or conflicts with another open scope | Conflict, no state change; do not raise the counter to accommodate the caller |
| Open while a closed-scope decision remains unsettled | Unavailable; retain the old owner/worker/lease rather than replace it |
| Close n = closedThrough+1, including before open n arrives | Advance closedThrough to n and clear open n if present; retain any active decision. This closes delayed-open races without a per-scope history set |
| Duplicate/delayed close n <= closedThrough | Return the snapshot, preserving any newer open scope and active decision |
| Close skips a sequence | Conflict, no state change; main must reconcile the exact original owner rather than guess skipped scopes |
| Inspect | Return a memory snapshot only; no counter advance, source read, receipt lookup or permission |
| Admit new correction for open n | Require exact lifetime/workspace, n = closedThrough+1, no closing/uncertain fence and no other active decision; checked-increment lastAdmissionSequence and atomically install its one exact decision |
| Close after admission | Deny queued/later admissions immediately; keep only that exact admitted attempt permitted through disposition. Never clear a later fence using it |
| Proven rollback or committed publication/retirement | Retire the exact active decision after all source/resource/publication obligations; never decrement or reuse lastAdmissionSequence |
| Counter exhaustion, poisoned state or unknown owner | Fence new admission; existing decision stays retained until proven disposition. No wrap, reset, inferred empty owner or automatic replacement |

Install/close/retire use the existing proposed lock order and perform no SQL,
filesystem, network I/O, waiting or callbacks under the latch. Main closes local
admission and invalidates pending response contexts before dispatching close; it
does not wait for remote closure to update the UI. A delayed correction may still
win sidecar admission during transit, as already accepted. If a frame closes before
its open arrives, main sends the allocated sequence's close; later open is denied.
An unknown open/close outcome keeps main closed/unknown. An explicit inspect may
reconcile the original counter state but cannot silently reopen/resubmit. Any
observed ahead/conflicting state is unavailable, not repaired by counter guessing.
A correction timeout/cancellation also closes its local scope and dispatches its
exact close; an open scope with no active decision is not proof that a delayed
request cannot still arrive. Main may release an old unknown pending slot only
on an authenticated original-owner snapshot with closedThrough at least that
scope and unsettledScopeSequence null. This proves no future admission through
that closed scope, not absence of a prior committed receipt. Any subsequent
explicit retry first opens the next scope and uses the original receipt lookup;
it never resends under the closed identity. Delayed old acknowledgements cannot
clear a newer local scope's state.

A valid control request for a foreign workspace/lifetime is unauthorized or stale
before source disclosure. Malformed control data is bad request; ordering conflict
is conflict; saturation/uncertain owner is unavailable. These bounded failures
never include secrets, raw source content or an accepted report receipt.

## Receipt lookup, source validation and response ordering

Preserve #742's order: validate current UI/owner scope and per-run request rate,
normalize against the immutable expected predecessor, then compare exact stored
request/receipt before mutable run/head CAS. A current authorized exact retry
returns the original source receipt without acquiring a write decision or adding
metadata, source effects, quota changes or snapshot invalidations. Final response
commitment still checks the original open scope; main independently checks its
original sender/frame/local scope before delivery. Closure-first denies that
response even though the durable historical receipt exists.

For a new request, prepare immutable intent and bounded complete changed-set
inputs outside the latch. Atomically admit the exact decision only after current
scope/owner checks, then release the latch before final source validation and SQL.
Admission grants no new source read, freezes no mutable head/dependency/health
state and cannot retarget a request. Final source/epoch/quota/consumer validation
may still reject it. Keep one worker and original lease through commit, rollback
or uncertain disposition. Ordinary runtime stop closes its own authority without
waiting for this worker. Run completion/cancellation alone still does not revoke
an otherwise eligible explicit terminal UI correction.

After durable commit, invalidate every affected original-store run snapshot before
releasing the publication position or serving a current read. No completion is
published into another workspace, clears its failure fence or marks a summary
viewed. Response commitment is separate from permission to persist; no stale
success reaches the renderer, and caller cancellation/timeout proves no rollback.
Sidecar exit destroys volatile scopes/decisions; recovery reads complete durable
state/receipts without restoring authority or re-executing a lost operation.

## Proposed report receipt storage variant

Keep the existing `report_receipts` key `{stream:S,reportVersion}`. For each new
correctResearchReport write under this protocol, use exactly this new value:

```text
{schemaVersion:2,operation:correctResearchReport,expectedTaskVersion:null,
 expectedRunVersion,expectedReportHead,requestDigest,
 authorization:{policy:terminal-ui-correction-v1,sidecarGeneration:Generation,
                scopeSequence:PositiveCounter,admissionSequence:PositiveCounter},
 response:ResearchReportReceiptV1,lastCommit:CommitRef}
```

Every unchanged field/receipt/normalization rule comes from the
[existing report registry](2026-10-29-report-publication-profile-design.md#report-head-and-receipt-registry).
Policy is the literal string above. Authorization values are copied only from
the original admitted decision. Its sidecarGeneration is the admitting UI owner
lifetime; it need not equal the replacement report's copied predecessor runtime
lineage. Validate those independent generations against their own owners, never
rewrite source provenance to make them equal. The key, reconstructed complete request, exact
canonical request bytes and requestDigest must agree; digest equality alone does
not suffice. The key's run binds the current admission epoch, and lifetime/scope/
admission counters must equal the original retained memory decision **when writing
this new row**. Historical retry/recovery validation does not compare them with a
current decision, lifetime or scope: validate the closed stored shape, its original
row/manifest commitment, same receipt/source/request joins and original command
provenance under the owning recovery contract. Fresh current authorization remains
a separate prerequisite to serving that historical receipt. Do not rewrite its
original authorization tuple or recreate a live decision from it. A missing or
unqualified historical proof is unavailable, not an excuse to invent provenance.
Sequence gaps from rejected/rolled-back decisions are legal; numbers alone prove
no prior authority or completed command.

Write this value atomically with the existing complete source/head/run/summary/
quota/control effects. Its lastCommit joins that command's original manifest.
The format-3 row commitment includes the entire new value except outer lastCommit;
there is no excluded authorization field or post-commit metadata write. Existing
row/manifest domains and report/source/request domains are unchanged. Registry
recognition must explicitly allow this closed schemaVersion 2 correct variant;
unknown variants remain unavailable. Do not reuse #744 ClaimProofV2, mutation_lookup
or protected receipt credits. The success response remains the identical source
ResearchReportReceiptV1 bytes and existing 201/new versus 200/exact-retry statuses.

Retain original version-1 historical rows and their original commit proofs without
inventing authorization metadata. Their producer eligibility remains Gate 2's
independent proof, not inferred from missing authorization or catalog presence.
Version-2 rows missing/mismatching authorization are corrupt; recovery never
synthesizes the field or treats them as version 1. Old readers that do not recognize
the variant must refuse it. No live store, migration, downgrade or automatic schema
upgrade is authorized. A future compatibility plan must fence unsupported readers
and bind historical producer proofs before any adapter/runtime selection.

These are nonsecret decision annotations, not cryptographic proof that a live
scope won the race. A self-consistent hash or conditional fixture cannot replace
original producer and native ordering evidence. No token, original header value
containing a credential, frame URL, transcript or user content is added to them.

## Packing and resource consequences

All existing limits remain: 2 KiB complete receipt key+value, 32 KiB request/report,
32 MiB run preparation/report aggregate, 256 MiB workspace report aggregate and
the shared worker, command/changed-row/query/scratch/physical limits. Count the
new authorization bytes in receipt metadata and existing report-retained-bytes
usage for both run and workspace, and in complete command/row commitments. This
is not another receipt allocation or retention pool. Retain them with the receipt;
no independent truncation, eviction or authorization-only cleanup is allowed.

The existing [maximum-width receipt witness](../../validation/report-publication-components.md)
is 1,925 bytes including its key and outer link. The new authorization member,
including its preceding comma, adds at most **224 bytes** with maximum u64 values
and a 64-byte lifetime; schemaVersion 1→2 adds no byte. Applied to that witness,
the result is **2,149 bytes**, **101 over** the unchanged 2,048-byte cap. It must
reject. This is width arithmetic, not a source-valid/admitted version-2 fixture.
Existing historical witness bytes are unchanged. Smaller encodings may fit but
must be measured with their complete key/value, manifest and resource effects;
adding 224 to an old command total alone is not a whole-command proof.

This proposed representation therefore reduces packing headroom without raising
source limits. It promises no maximal eligible profile or full admission of all
locally valid field combinations. If practical source-valid profile qualification
cannot fit, revise this proposal explicitly; never omit provenance, borrow #744
slots, split atomic effects or raise limits silently.

## Failure oracles and next gate

Before any executable handoff, accepted documentation fixtures must cover:

- close n before open n; delayed/duplicate close n after n+1 opens; skipped counters,
  old lifetimes, counter maximum and main counter loss;
- admission-first versus close-first, queued denial, wrong intent/descriptor,
  source/head/epoch/dependency/health drift and exhausted original rate budgets;
- lost open/close acknowledgement, explicit inspect without permission renewal,
  stale local context and a committed write whose response cannot be delivered;
  no-active-but-still-open must not clear an unknown pending request;
- exact historical retry before mutable CAS, no metadata/byte/effect rewrite,
  version-2 authorization/commit substitution and original version-1 provenance gaps;
- exact complete receipt packing at/beyond 2 KiB and unchanged aggregate/command
  accounting; the existing maximum-width witness must fail under this new variant;
- stalled storage with independent ordinary stop, original lease retention, actual
  ownership loss, sidecar exit and complete old/new recovery without re-execution.

The first five groups are future logical/component fixtures, not permission to run
them before these codecs/defaults are accepted. Native closure delivery/handler
availability, complete writer coverage, actual terminal producer and source-owner
consumer proof, lifecycle integration and measured resource enforcement remain
separate gates. The current stop-before-open sidecar code is unchanged; this
protocol cannot claim independently draining old/new processes or bypass the
single active-or-retiring owner cap. An affected ADR and separately approved
implementation/native qualification plans are required before architecture code.

**Owner decision requested after review:** accept or revise the exact bounded scope
state/control transport and handler/rate limits; the main-only correction headers
and one-pending-request bound; and the tagged version-2 receipt metadata/compatibility/
accounting representation, including its maximum-width rejection cost. The accepted
cutoff and its distributed closure/lease tradeoffs are not reopened. Gate 2 terminal
producer, receipt-history initialization and complete consumer/index proof remain
open; #1089/#742/#744/#746 stay open. No new live/model/native probe or retry budget.

## Bounded review record — 2026-11-10

Two fresh read-only reviews independently found the same actionable ambiguity:
historical receipt validation appeared to require the original live memory
decision after that decision had retired. The correction above limits memory
equality to new writes and separately defines original commit/request/provenance
validation for retries/recovery under fresh current authorization. This preserves
existing idempotency and does not renew old authority. No second independent
review of that correction is claimed here.

The author also clarified retained-owner close/inspect after frame/workspace
closure, and the closed-scope/no-unsettled snapshot required before releasing an
unknown local request slot. These prevent stale local state from being confused
with denied remote admission. Native delivery, complete owner integration and
source-valid historical producer evidence remain unqualified. The author also
made independent UI/source-lineage generations and retained scope counters across
namespace rotation explicit; same-lifetime workspace rebinding is outside this
first protocol and cannot reset the old owner. These clarifications have no
separate fresh review claim. This review is
written-contract review only, not a runtime/fixture execution or owner acceptance.
