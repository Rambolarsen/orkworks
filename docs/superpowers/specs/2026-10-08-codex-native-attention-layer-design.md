---
type: "Design"
title: "Codex Native Attention With Sidecar-Mediated Metadata Writes"
description: "Design history: Codex Native Attention With Sidecar-Mediated Metadata Writes."
tags: ["orkworks", "design"]
---

# Codex Native Attention With Sidecar-Mediated Metadata Writes

- Status: owner-approved written design; producer-protocol implementation and
  production verification remain gated
- Deciders: repository owner, Codex
- Date: 2026-10-08
- Issue: [#761](https://github.com/Rambolarsen/orkworks/issues/761)
- Related: [#690](https://github.com/Rambolarsen/orkworks/issues/690),
  [#788](https://github.com/Rambolarsen/orkworks/issues/788),
  [#789](https://github.com/Rambolarsen/orkworks/issues/789), and
  [ADR 0076](../../adr/0076-codex-owned-native-approval-observer.md)

## Context

ADR 0076 requires native approval clearing to revoke its authority whenever a
competing writer changes the attention tuple or its source, including an
identical-value write. A sidecar revision and a final file-identity check do
not make a clear atomic against an agent that directly replaces
`sessions/<session-id>.json`.

The MVP currently permits direct agent reads and writes of that JSON record.
The sidecar cannot serialize a clear with a writer that does not participate
in its write boundary. The selected direction is therefore to route writes
for native-enabled sessions through the sidecar. This changes the current
direct-write contract for those sessions. The repository owner approved this
written contract on 2026-10-08. That approval resolves the specification
prerequisite only; it does not authorize runtime or producer-protocol changes
under #761. Native clearing remains disabled until the implementation and
independent verification gates are complete.

## Producers and write paths

| Producer | Proposed path for a native-enabled session | Authority |
| --- | --- | --- |
| User/manual override | Existing sidecar action and `MetadataStore` write | `user`; highest priority |
| Direct agent metadata writer | Versioned sidecar metadata write using the live session report token; direct JSON writes are unsupported while the session is active | `agent`; sidecar assigns source and confidence |
| Harness hook/API | Existing authenticated sidecar routes and Codex report relay | `agent`; `attentionOrigin` identifies the harness hook |
| Peon | Existing sidecar inference and `MetadataStore` write | `peon`; existing strict staleness rule |
| Backend inference and process lifecycle | Existing sidecar `MetadataStore` write paths | Existing source priority |
| Codex native observer | Sidecar application path after a validated hook candidate and bounded exact-root native observation | `agent`; `attentionOrigin=native_codex`; may clear only its owned wait |
| Debug injection | Existing debug-only sidecar path | `debug`; existing restrictions |

Native attention transitions include prompt submission, permission candidates,
native pending and resolved observations, stop, accepted committed input,
identity reset/revocation, and runtime termination. Each transition uses the
same per-session serialized writer boundary as agent, user, hook, Peon, and
lifecycle writes. The sidecar remains the only process that persists the
session JSON for an active native-enabled session.

The race guarantee applies to supported producers that use this boundary. A
versioned API patch may still publish by replacing the complete session JSON,
but it holds the same per-session transaction boundary across revision and
source checks, mutation, and replacement that native clear holds across its
final ownership check and replacement. An accepted competing attention write,
including an identical-value write, advances the attention ownership revision.
The implementation tests both commit orders at the final-check/replacement
boundary. A direct filesystem replacement that bypasses the API is unsupported
for a native-enabled session and receives no serialization guarantee; file
identity checks alone do not make that write atomic. If any supported
native-enabled producer still writes the JSON file directly, that session is
ineligible for native clearing.

## Decision

Keep the complete session record in `sessions/<session-id>.json`; do not add a
second attention file or merge two independently persisted records. All writes
for an active native-enabled session go through a versioned sidecar operation.
Direct JSON reads remain available. A direct JSON replacement is not a
supported write path for an active native-enabled session because it bypasses
the serialization and ownership checks. Sessions that are not native-enabled
retain their existing behavior. Setting the protocol marker after producer
migration and the static launch-eligibility checks activates the API-only write
contract immediately. The child-context handshake is a separate gate for
native-clear eligibility; while it is pending or if it fails, agent metadata
writes fail closed and direct JSON writes remain unsupported. Native clearing
also remains disabled until the independent production gates pass.

The implementation should expose authenticated `GET /sessions/:id/metadata`
and `PATCH /sessions/:id/metadata` operations for direct agents. Only a
launch eligible for the proposed native Codex runtime under its explicit
configuration and exact version/platform/protocol compatibility gates may set
`ORKWORKS_SESSION_METADATA_API_VERSION=1`; the resolved built-in harness
capability alone is insufficient. Unsupported or non-native launches keep
the existing direct JSON contract. Eligible launches set the marker alongside
the existing session ID, sidecar port, and report token. When that marker is
present, agents
must use the API for all metadata mutations and must fail closed if it is
unavailable; they must not fall back to writing the JSON file. Without the
marker, the existing direct JSON contract remains in effect for sessions
where native clear is disabled.

The read response contains the current record and an opaque
`metadataRevision`. A write carries that revision and a field-scoped patch
whose only top-level fields are:

- `attentionState`: the complete attention input with required keys
  `observedStatus`, `needsUserInput`, `detectedQuestion`, and
  `suggestedOptions`. Every key must be present; nullable values use explicit
  `null`. `false` is a value for `needsUserInput`, and an empty array is a
  value for `suggestedOptions`. The string and enum values use the existing
  validators. The caller does not submit `attention`; the sidecar derives it
  from `observedStatus` using the existing canonical mapping (`waiting_for_input`
  to `needs_you`, `stale`/`done` to `idle`, and `working`/`idle`/`blocked`/
  `failed`/`capped` to the same value; `null` remains `null`). This replaces
  the complete attention tuple as one unit.
- `agentMetadata`: a non-empty object containing any of three independent
  scopes: `workFields`, `summary`, and `planPath`. `workFields`, when present,
  is a complete snapshot containing `task`, `nextAction`, `workPhase`,
  `blockerDescription`, `failedCommand`, and `failedTest`; every key is required
  and nullable fields use explicit `null`. This complete snapshot is necessary
  because general work metadata has one record-wide source and freshness
  scope. `summary` and `planPath` are independently patchable and use their
  atomic rules below. Unknown fields and an empty `agentMetadata` object are
  rejected.

After field-level validation, the sidecar validates prompt fields against
`observedStatus` as one tuple. `waiting_for_input` requires
`needsUserInput=true`, but the question and options may be absent. `blocked`,
`failed`, and `capped` may also have `needsUserInput=true`; `working`, `idle`,
`stale`, and `done` may not. A non-empty `detectedQuestion` requires
`needsUserInput=true`; non-empty `suggestedOptions` require a non-empty
question. When `needsUserInput` is false or null, the question must be null
and options must be null or empty. A null `observedStatus` also requires an
empty prompt tuple. Whitespace-only questions are invalid. These rules keep
valid `blocked`-plus-input tuples and reject contradictory combinations
atomically without changing metadata or provenance.

Either object may be omitted to preserve that group, but a write must include
at least one non-empty group. An empty `agentMetadata` object or a patch with
both groups omitted is rejected without changing fields, provenance, ages, or
revisions. The sidecar rejects
unknown and protected fields, including session identity, lifecycle/process
state, `metadataSource`, `metadataConfidence`, revisions, attention
provenance, summary provenance, plan provenance, and timestamps. The sidecar
owns `lastActivity` and updates it according to existing activity semantics.
This whole-tuple operation prevents a direct writer from accidentally
combining fields from two attention producers.

The proposed API follows the MVP's target current-summary snapshot contract.
When a non-empty,
non-whitespace summary is supplied, the sidecar updates `summary`,
`summarySource=agent`, `summaryConfidence=1.0`, and `summaryObservedAt` together;
the caller cannot submit the provenance fields. Explicit `null` clears all four
fields together, omission preserves them, and an empty or whitespace-only
string is rejected. This keeps agent summaries usable as Taskmaster handoff
evidence without retaining stale snapshot provenance.

`planPath` accepts a validated relative path, not a caller-built
`PlanReference`; the sidecar resolves its worktree root and assigns the
proposed `agent_reported` source. This adds `agent_reported` to the
`PlanReference` source vocabulary; it is assigned by the sidecar and never
accepted from a request. A patch that sets or clears `planPath` while the
current reference has `source=user_selected` returns a conflict without
writing any part of the `agentMetadata` group. Only the explicit user plan
selection/clear operation may replace or clear that reference. Callers cannot
submit a plan source or worktree root.

The protocol contract is:

1. The read returns the current record and an opaque `metadataRevision`.
2. The write carries the revision it read and the allowlisted patch. The
   sidecar verifies the live session report token, session ID, and expected
   revision, then reads the latest record, applies the patch, performs the
   source-priority and ownership decisions, and atomically writes it under a
   per-session transaction boundary shared with native clear. The boundary
   covers the read/check/modify/replace sequence; locking only the final rename
   is insufficient.
3. A revision mismatch returns a conflict without writing. The agent rereads,
   reapplies its intended change, and retries; it must not blindly replace the
   current record. A patch that loses the existing source-priority check
   returns a conflict without changing the record.
4. A patch containing `attentionState` is an attention write. The sidecar
   checks the attention source priority, validates `observedStatus`, derives
   `attention` from its canonical mapping, and advances the attention
   ownership revision even when the submitted input is identical. A
   non-empty tuple receives `attentionSource=agent`,
   `attentionConfidence=1.0`, and `attentionOrigin=direct_agent`; the request
   cannot supply those authority fields. A complete explicit clear
   (`observedStatus=null`, no true input flag, no question, and no options)
   instead writes an empty tuple with `attentionSource=unknown`, zero
   confidence, and `attentionOrigin=direct_agent`, so Peon may populate a new
   observation immediately. A null status with residual prompt fields is
   invalid. The clear still advances the attention ownership revision.
   This group does not change `metadataSource`, `metadataConfidence`, or
   `workMetadataUpdatedAt`. Within `agentMetadata`, only a complete
   `workFields` snapshot checks the work-metadata source priority, assigns
   `metadataSource=agent` and confidence, and advances `workMetadataUpdatedAt`.
   A summary-only or plan-only patch changes only its own provenance scope and
   cannot promote or refresh retained work fields. A user-selected plan cannot
   be changed by the agent patch. If a request
   contains both groups, both source checks and updates run under the same
   transaction and commit atomically. If either scope loses its source check,
   the whole request returns a conflict without writing.
5. Every accepted patch advances the metadata revision used for optimistic
   concurrency. Every accepted attention/source write from any producer also
   updates the durable attention-specific update time; work-metadata writes do
   not refresh it. Every accepted work-metadata write updates
   `workMetadataUpdatedAt`; attention-only writes do not refresh it.

Every persisted session-record mutation advances `metadataRevision` under the
same per-session transaction boundary, including user, hook, Peon, backend,
process, lifecycle, native, and API writes. The revision is persisted as part
of session metadata, survives all writes within the live session, and is never
reused. Attention ownership revision is narrower: it advances on every
accepted attention write, even when the tuple and source are unchanged,
so a competing identical write revokes an in-flight native clear. Unrelated
work-metadata writes do not revoke a native clear. Runtime ownership tokens
stay process-local and are never persisted. No bearer, native connection
secret, or clear token is stored in session metadata.

For a legacy record with no persisted revision, the versioned read returns the
stable per-record initial revision `"0"` without rewriting the record. Every
accepted persisted mutation, including the first, advances that revision under
the transaction boundary and persists the result in the session record
(`"1"` for the first mutation).
The counter is per session record, is exposed as an opaque value, and must use a
checked increment; exhaustion fails closed without writing rather than
wrapping or reusing a revision. Concurrent readers therefore receive the same
initial revision, and only one writer using it can commit. This protocol
bookkeeping does not migrate the other fields in a legacy record.

Native clear performs its final ownership and source checks and its atomic
session-record replacement while holding the same per-session transaction
boundary. It may clear only while `attentionSource` is still in the `agent`
tier, `attentionOrigin` is `native_codex`, and the runtime token owns the
latest attention revision. Every accepted competing attention/source write
advances that revision, even when its values are identical. Therefore, if an
agent write is accepted first, the clear fails its ownership check; if the
clear commits first, the later agent write is ordered after it and becomes
the current record. This guarantee applies to compliant in-product producers
using the sidecar contract. Arbitrary external file replacement remains
outside the supported protocol and cannot be made atomic by this design.

The authenticated agent write must be scoped to the live session represented
by the report token and must not trust caller-selected source labels. The
existing report token is a process-local bearer capability inherited by the
session's child processes; it does not prove operating-system process
identity. The endpoint must enforce the existing request-size and field
validation bounds and must not log the token. If the token or active session
is unavailable, the write fails without falling back to direct JSON mutation.

## Producer migration gate

The protocol marker is a capability declaration, not proof that an arbitrary
process will obey it. Add a closed `sessionMetadataWriteProtocol` capability
to the resolved built-in harness definition. Only the source-controlled
Codex definition may advertise `sidecar-v1`, and only after its agent
instructions and reporter helpers have migrated; user overrides cannot add
this capability. The launch adapter derives
`ORKWORKS_SESSION_METADATA_API_VERSION=1` only when both that resolved
capability and the launch's native configuration plus exact version, platform,
and protocol compatibility gates are satisfied. Unsupported or non-native
launches omit the marker and retain direct JSON behavior. On an eligible
launch, the marker requires API-only writes and fail-closed behavior; it does
not itself enable native clear. Native-clear eligibility starts disabled and
activates only after a bootstrap handshake succeeds from the actual agent
execution context under the effective sandbox profile. The handshake performs an
authenticated metadata GET and a sidecar-defined validation-only PATCH. The
PATCH checks the route, method, token, revision, and schema without mutating
session metadata; a sidecar-only loopback probe is insufficient. Agent
instructions and reporter helpers must perform this handshake before any
metadata mutation. If it fails, the session remains API-only and native clear
stays disabled; the agent must not fall back to direct JSON writes. The
existing Codex report mailbox remains identity-only
and is not a metadata-write transport. A future mailbox-based metadata
transport needs its own protocol design and approval before it can enable
this capability.
Legacy integrations continue to use the direct JSON contract and cannot enter
a native-clear-eligible configuration. Migration tests must exercise every
in-product writer path, verify the generated instruction bundle and reporter
helpers use the API, and prove reachability from the same sandboxed context.
Out-of-band edits by a user or process that deliberately bypass the declared
protocol remain outside the supported producer contract; this design does not
claim to make arbitrary filesystem writes atomic with the sidecar.

## Attention ownership, age, and projection

Keep `metadataSource` and `metadataConfidence` for general work metadata.
Persist `workMetadataUpdatedAt` for that scope, and add `attentionSource`,
`attentionConfidence`, `attentionOrigin`, and `attentionUpdatedAt` for the
attention tuple. Expose these fields in the session projection.
`attentionOrigin` distinguishes `direct_agent`, `harness_hook`, `native_codex`,
`user`, `peon`, `backend_inference`, `process`, `debug`, and `legacy_unknown`.
Legacy records without the new fields derive attention source/confidence and
work-metadata source/confidence from the legacy metadata fields, use the JSON
file modification time as the initial age for both scopes, and project
`attentionOrigin=legacy_unknown`; they are never inferred to be owned by the
native observer. In particular, legacy `metadataSource=codex_hook` normalizes
to `attentionSource=agent` with its stored confidence and `legacy_unknown`
origin. Other legacy source values retain their existing priority tier.
Normalization may occur in memory; the next accepted write persists the new
fields. If legacy file age cannot be read, Peon must preserve the corresponding
current scope until a valid write to that scope establishes its timestamp.
Session projections keep their current one-record semantics; they do not
compose attention from another record.

The attention SourceBadge in session details must use `attentionSource` and
`attentionConfidence`; `metadataSource` remains the provenance for general
work metadata. This keeps the visible attribution aligned with the tuple after
the sources split.

Persist `attentionUpdatedAt` with the attention tuple and source. Peon's
attention `agent` staleness check uses it, not the session file's modification
time. Persist `workMetadataUpdatedAt` with general work metadata and use it for
the independent work-metadata source check. Any accepted write to that scope
refreshes only its timestamp; an attention-only write refreshes only
`attentionUpdatedAt`. The attention boundary remains strict: age 15 seconds
blocks Peon; age greater than 15 seconds permits it. Legacy records without
either timestamp use file modification time as the initial age for both
scopes; if it cannot be read, Peon preserves the affected scope until a valid
write to that scope establishes its timestamp.

The attention source order remains `user > agent > peon > backend_inference >
process > unknown > debug`. Direct agent, harness-hook, and native Codex
attention all use the `agent` tier; they are peers, not separate priority
levels. Every accepted attention write from one of those producers revokes
the prior native ownership token, including identical-value writes. Only
`user` has strictly higher attention priority. A live native-owned
`waiting_for_input` tuple is exempt from age-based Peon overwrite while its
runtime ownership token remains current, regardless of elapsed time. It
becomes eligible for normal source arbitration only after its owner resolves
or revokes it. Ordinary agent and hook attention retain the strict boundary:
age 15 seconds blocks Peon; age greater than 15 seconds permits it. A live
validated Codex `PermissionRequest` hook-owned wait is also exempt from
age-based Peon overwrite while its matching runtime hook authority remains
current, including when native observation is unavailable or ambiguous.
Hook revocation, an accepted committed-input transition, a later accepted
hook resolution, or session lifecycle end revokes that protection. Other
harness-hook attention keeps the strict age boundary, and Peon may continue
updating non-attention metadata while either Codex wait is protected.

Accepted committed terminal input is a trusted runtime transition. When it
supersedes either a native-owned approval wait or a validated Codex
hook-owned PermissionRequest wait, it atomically writes the existing
`process`-tier `working` tuple and revokes that wait's ownership token despite
the ordinary source ladder. The hook exception requires the live Codex
permission authority to still own the wait; it does not extend to other
harness-hook waits or free-form elicitation. This exception applies only to
the accepted live input transition and does not bypass a `user` override.
Other process transitions keep the ordinary source check. The versioned read
response exposes the metadata revision needed for writes; `attentionUpdatedAt`
is persisted and projected for arbitration and diagnosis.

The authenticated child-context bootstrap handshake verifies both metadata
read and write reachability under the effective sandbox profile. It performs
an authenticated GET and a sidecar-defined validation-only PATCH through the
same metadata endpoint, session token, and child execution context. The
validation-only PATCH uses the normal request envelope with
`metadataRevision` and a schema-valid non-empty patch, sent to
`PATCH /sessions/:id/metadata?validateOnly=true`. It authenticates the same
session token, checks session identity, current revision, allowlisted fields,
and field validators, then returns `200` with
`{"validated":true,"metadataRevision":"<current revision>"}`. It skips
source-priority mutation and does not persist fields or advance metadata,
attention, or work-metadata revisions/timestamps. A follow-up GET must return
the same revision. If either operation is blocked or cannot be verified, the
API marker remains present, API writes remain required, and native clear
remains disabled.

When lifecycle code snapshots observed attention into
`endingObservedStatusSnapshot` or `finalObservedStatusSnapshot`, the snapshot
uses `attentionSource` and `attentionConfidence`, not the record-wide work
metadata provenance, and persists `attentionOrigin` as an optional snapshot
field. Older snapshots without that field remain readable and project
`legacy_unknown`; a legacy snapshot `source=codex_hook` projects the attention
source as `agent` while retaining `legacy_unknown` origin. Ending and final
snapshot creation, serialization, and recovery are part of the migration and
require lifecycle coverage.

The existing work-metadata source ladder continues to protect descriptive
fields such as task, next action, and blockers. Peon applies attention,
work-metadata, summary, and plan provenance checks in their respective scopes,
so an agent attention or summary write cannot keep stale descriptive work
fields fresh, and an agent metadata patch cannot refresh or change attention
ownership.

Session-list and detail projections read one canonical session record. A
failed or malformed metadata read is an explicit projection error; it must
not be treated as an absent record, a lower-priority source, or permission to
show a stale cached tuple as current. A caller may retain a prior view only if
the response marks it stale and prevents it from authorizing a native clear.
Clear failures likewise leave persisted attention unchanged and do not publish
a resolved state.

## Lifecycle and compatibility

- Native-disabled and unsupported Codex sessions keep the existing JSON
  behavior and do not gain native clear authority.
- Native-enabled sessions require the versioned sidecar write path for all
  active metadata mutations, including agent writes. Read-only direct JSON
  access remains compatible; active direct JSON writes are unsupported. The
  atomicity guarantee covers compliant producers, not arbitrary external
  modification of the metadata directory.
- There is no automatic migration of old records. The versioned read path
  must define how it presents a revision and attention timestamp for legacy
  records before native clear can be enabled.
- Session end revokes all live clear tokens. Restart reconciliation follows
  the existing lifecycle contract and never restores native clear authority
  from persisted data.
- Forget, delete, and retention continue to operate on the single session
  record; there is no companion attention file to orphan.
- #763's listener ownership gate, #690's signal/configuration/platform gates,
  and ADR 0076's native verification requirements remain independent.

## Alternatives considered

### Keep direct JSON writes and store native attention separately

A separate layer avoids overwriting the base record, but it cannot make
session-list projections coherent with arbitrary direct replacements. A
direct writer can read stale attention, then replace the whole JSON record
while a native wait is active. The layer also needs separate provenance and
staleness composition rules. This option is not selected.

### Keep direct JSON writes and leave native clearing disabled

This remains the rollout behavior until the owner approves a mediated write
contract and its implementation evidence. It is the safe fallback if the
versioned write path cannot be delivered without losing supported agent
updates.

## Required implementation evidence

The implementation follow-up must include behavioral tests proving:

- An API read followed by a versioned patch updates only requested fields and
  receives a new metadata revision; an identical accepted tuple write also
  advances the attention ownership revision.
- `attentionState` rejects partial tuples, replaces all tuple fields together,
  derives canonical `attention` from `observedStatus`, and distinguishes
  omitted groups from explicit-null clears.
- Cross-field validation covers every accepted `observedStatus`: waiting
  requires input; blocked, failed, and capped may carry input; working, idle,
  stale, and done may not. Questions require input, options require a
  non-empty question, and null status requires an empty prompt tuple. Tests
  preserve `blocked` plus input and status-only waiting, and reject working
  plus input, question with input false, and options without a question.
- Empty patches and empty `agentMetadata` groups are rejected without changing
  data, provenance, timestamps, or revisions. A complete agent attention clear
  relinquishes the `agent` source tier, revokes native ownership, and permits
  immediate Peon inference; a null status with residual prompt fields is
  rejected.
- A non-empty agent summary updates the four current-summary fields together
  with sidecar-owned provenance; explicit null clears all four, and omission
  preserves them.
- Agent `planPath` writes cannot replace or clear `user_selected` references,
  cannot provide their own source/worktree root, and receive the sidecar-owned
  `agent_reported` source.
- A stale agent revision returns conflict and cannot replace newer metadata.
- A write from any sidecar producer advances `metadataRevision`, so an agent
  patch read before a native, Peon, hook, user, or lifecycle write conflicts.
- A competing agent write and native clear are serialized: whichever commits
  first is reflected by the next read, and an earlier agent write prevents
  the old native owner from clearing.
- Deterministic lock-order tests exercise both sides of the final-check/commit
  boundary. In the clear-first case, pause native clear after its final
  ownership check while it still holds the per-session writer lock; start an
  authenticated versioned `attentionState` patch that has already read the
  prior `metadataRevision` and would publish by replacing the whole session
  record.
  Verify the patch cannot commit during the pause, native clear commits first,
  and the patch then receives a revision conflict without overwriting the
  cleared record. After rereading and retrying, the attention patch commits
  after clear and is visible as the current tuple. In the writer-first case,
  commit the attention patch before native clear acquires the lock; verify the
  clear then fails its owner-revision check and leaves the agent tuple current.
  These interleavings prove the defined ordering for compliant sidecar
  writers; direct file replacement remains unsupported for an eligible session
  and is not counted as a passing serialization test.
- Native clear rechecks ownership after any staged I/O and immediately before
  the atomic replacement while the per-session writer lock remains held.
- Every in-product active-session writer uses the versioned sidecar contract;
  active direct JSON replacement remains unsupported by the eligible runtime
  contract.
- The persisted attention update time changes on attention/source writes,
  remains unchanged for unrelated writes, and preserves the strict Peon
  boundary at 15 seconds versus greater than 15 seconds for ordinary agent and
  hook attention. Live native-owned and validated Codex hook-owned
  PermissionRequest waits remain protected from Peon past that boundary while
  their respective runtime authority remains current; Peon can still update
  non-attention metadata. Hook/native resolution, accepted committed input,
  authority revocation, and session end remove that protection. Other
  harness-hook waits retain ordinary age arbitration.
- `workMetadataUpdatedAt` changes on work-metadata writes, remains unchanged
  for attention-only writes, and independently controls the work-metadata
  source staleness check.
- The API version marker is set only when the migrated built-in capability and
  the launch's native configuration, exact version, platform, and protocol
  compatibility gates pass. Unsupported and non-native launches omit it and
  retain direct JSON behavior. Native-clear eligibility remains
  disabled until the agent-context authenticated GET and validation-only PATCH
  succeed under the effective sandbox profile. The handshake makes no record
  mutation and preserves revisions/timestamps; failure leaves API-only writes
  required and native clear disabled. The identity-only report mailbox does
  not satisfy this transport requirement.
- The accepted committed-input transition clears both native-owned Codex
  approval waits and validated Codex hook-owned PermissionRequest waits while
  preserving `user` overrides. It does not clear unrelated harness-hook waits
  or free-form elicitation; other process transitions retain ordinary source
  arbitration.
- Summary-only and plan-only writes leave `metadataSource`,
  `metadataConfidence`, and `workMetadataUpdatedAt` unchanged. Any descriptive
  work-field update supplies the complete `workFields` snapshot; partial
  snapshots are rejected atomically, so retained Peon fields are never
  promoted to agent provenance by omission.
- Agent work-metadata patches update metadata source priority without changing
  attention source, provenance, timestamp, or native clear ownership; Peon
  applies the two source-priority checks independently.
- Failed or malformed session reads and failed writes do not appear as
  missing/lower-priority attention and do not publish a resolved state.
- User and agent source priority, tuple integrity, and projected attention
  provenance remain correct across agent, hook, Peon, native, and lifecycle
  writes; legacy records receive `legacy_unknown` provenance.
- Legacy `metadataSource=codex_hook` maps to the `agent` attention tier without
  being misidentified as a current native-owned tuple.
- Session API projection and the details SourceBadge use the attention-specific
  source and confidence while general metadata retains its own source fields.
- Ending/final observed-status snapshots persist attention source, confidence,
  and origin; older snapshots remain readable as `legacy_unknown`. Lifecycle
  ending, finalization, restart recovery, and a native-owned status ending
  session test cover the split from work-metadata provenance.
- End, restart, forget, delete, and retention do not restore native clear
  authority from stale state.

The production native-clear gate can be removed only after the written design
and implementation handoff are approved, the implementation evidence above
passes on every platform claimed by the producer migration, and required CI
and review pass on the exact PR head. In addition, each exact #690
version/platform/configuration entry must have the following installed-path
evidence; synthetic protocol and cooperating-writer tests do not substitute
for these checks:

- The #690 compatibility entry links a durable verification record for that
  exact entry. The record identifies the installed OrkWorks build (version and
  commit or artifact digest), OS version and architecture, effective launch
  configuration, run date, and the outcome of each applicable scenario below.
  It links the supporting logs, traces, or test artifacts and records any
  ineligible or ambiguous result. An entry without this record is not verified
  and cannot satisfy the production gate.

- The effective model, approval policy, sandbox, and configuration match the
  direct-launch path, including ordered shared options and unchanged
  selected-model metadata behavior. Unsupported configurations continue to use
  the unchanged direct launch.
- A fresh manual approval remains Needs You when held beyond the two-second
  grace. One user approval during a long-running tool produces the validated
  working transition before the tool finishes, while an observer disconnect
  leaves the native prompt usable.
- A separately configured automatic review is independently correlated to its
  actual Pre/Permission/Post invocation, runs beyond two seconds without a user
  approval click, and completes a long-running tool without a false Needs You
  state. If its native pending flag cannot be distinguished safely from a
  manual prompt, revise the signal design before enabling clearing.
- Root/subagent identity and overlapping-prompt cases show that no independent
  pending prompt is cleared. Any unverified or ambiguous case remains
  conservative and ineligible for assisted clearing.
- Owned-process startup, failure, cancellation, descendant cleanup, exact
  resume, detached-terminal behavior, workspace shutdown, and simultaneous
  session isolation pass on each platform included in the compatibility entry.

The separate #763 owned-listener gate must also be satisfied before bearer
delivery; it is independent of this metadata-writer contract. No compatibility
entry may ship while any applicable #690 or #763 gate remains open. Reducer
tests, cooperating-writer tests, or revision checks alone are insufficient.

## Handoff

This design changes no runtime code and does not itself enable native clearing.
[#788](https://github.com/Rambolarsen/orkworks/issues/788) owns the versioned
metadata API, single-record serialized writes, lifecycle handling, staleness
timestamps, projection error behavior, and behavioral writer/clear race tests.
Dependent [#789](https://github.com/Rambolarsen/orkworks/issues/789) owns the
Codex producer migration and child-context handshake. Keep #690's production
gate closed until both issues and the separate native verification gates are
complete. The implementation work requires its own authorization; #761 does
not authorize runtime changes. The approved disposition is linked from #690.
