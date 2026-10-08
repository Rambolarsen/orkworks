# Codex Native Attention With Sidecar-Mediated Metadata Writes

- Status: proposed; implementation approval and verification remain gated
- Deciders: repository owner, Codex
- Date: 2026-10-08
- Issue: [#761](https://github.com/Rambolarsen/orkworks/issues/761)
- Related: [#690](https://github.com/Rambolarsen/orkworks/issues/690),
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
direct-write contract for those sessions and requires an explicit owner
decision before implementation. It does not change the rollout gate: native
clearing remains disabled until the design, implementation, and independent
verification gates are complete.

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

## Decision

Keep the complete session record in `sessions/<session-id>.json`; do not add a
second attention file or merge two independently persisted records. All writes
for an active native-enabled session go through a versioned sidecar operation.
Direct JSON reads remain available. A direct JSON replacement is not a
supported write path for an active native-enabled session because it bypasses
the serialization and ownership checks. Sessions that are not native-enabled
retain their existing behavior until a separately approved migration changes
that contract.

The implementation should expose authenticated `GET /sessions/:id/metadata`
and `PATCH /sessions/:id/metadata` operations for direct agents. Native-enabled
launches set `ORKWORKS_SESSION_METADATA_API_VERSION=1` alongside the existing
session ID, sidecar port, and report token. When that marker is present, agents
must use the API for all metadata mutations and must fail closed if it is
unavailable; they must not fall back to writing the JSON file. Without the
marker, the existing direct JSON contract remains in effect for sessions
where native clear is disabled.

The read response contains the current record and an opaque
`metadataRevision`. A write carries that revision and a field-scoped patch
whose only top-level fields are:

- `attentionState`: the complete attention tuple with required keys
  `observedStatus`, `attention`, `needsUserInput`, `detectedQuestion`, and
  `suggestedOptions`. Every key must be present; nullable values use explicit
  `null`. `false` is a value for `needsUserInput`, and an empty array is a
  value for `suggestedOptions`. The string and enum values use the existing
  attention/status validators. This replaces the tuple as one unit.
- `agentMetadata`: an object containing any subset of `task`, `summary`,
  `nextAction`, `workPhase`, `planPath`, `blockerDescription`,
  `failedCommand`, and `failedTest`. Omitted fields are preserved; explicit
  `null` clears nullable fields. `task` is a string; `workPhase` is one of
  `ideation`, `implementation`, `review`, `debugging`, or `unknown`; `planPath`
  uses the existing validated `PlanReference` shape. Other fields retain
  their current serialized types and bounds.

Either object may be omitted to preserve that group. The sidecar rejects
unknown and protected fields, including session identity, lifecycle/process
state, `metadataSource`, `metadataConfidence`, revisions, attention
provenance, and timestamps. The sidecar owns `lastActivity` and updates it
according to existing activity semantics. This whole-tuple operation prevents
a direct writer from accidentally combining fields from two attention
producers.

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
   checks the attention source priority, assigns `attentionSource=agent`,
   `attentionConfidence=1.0`, and `attentionOrigin=direct_agent` rather than
   accepting those authority fields from the request, and advances the
   attention ownership revision even when the submitted tuple is identical.
   A patch containing only `agentMetadata` checks the existing metadata
   source priority, assigns `metadataSource=agent` and confidence, and leaves
   the attention tuple, attention source/confidence/origin, update time, and
   ownership revision unchanged. If the corresponding source is `user`, that
   scope's patch returns a conflict without writing.
5. Every accepted patch advances the metadata revision used for optimistic
   concurrency. Every accepted attention/source write from any producer also
   updates the durable attention-specific update time; work-metadata writes do
   not refresh it.

Every persisted session-record mutation advances `metadataRevision` under the
same per-session transaction boundary, including user, hook, Peon, backend,
process, lifecycle, native, and API writes. The revision may be persisted as
part of session metadata or maintained by an equivalent sidecar-owned version
protocol, but it must survive all writes within the live session and must not
be reused. Attention ownership revision is narrower: it advances only when
the attention tuple or its source changes, so unrelated work-metadata writes
do not revoke a native clear. Runtime ownership tokens stay process-local and
are never persisted. No bearer, native connection secret, or clear token is
stored in session metadata.

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
`ORKWORKS_SESSION_METADATA_API_VERSION=1` from that resolved capability, and
native-clear eligibility requires it. Legacy integrations continue to use
the direct JSON contract and cannot enter a native-clear-eligible
configuration. Migration tests must exercise every in-product writer path
and verify that the generated instruction bundle and reporter helpers use
the API. Out-of-band edits by a user or process that deliberately bypass the
declared protocol remain outside the supported producer contract; this
design does not claim to make arbitrary filesystem writes atomic with the
sidecar.

## Attention ownership, age, and projection

Keep `metadataSource` and `metadataConfidence` for general work metadata.
Add `attentionSource`, `attentionConfidence`, `attentionOrigin`, and
`attentionUpdatedAt` for the attention tuple, and expose these attention-
specific fields in the session projection. `attentionOrigin` distinguishes
`direct_agent`, `harness_hook`, `native_codex`, `user`, `peon`,
`backend_inference`, `process`, and `debug`. Legacy records without the new
fields derive `attentionSource` and confidence from the legacy metadata source
and confidence, use the JSON file modification time as the initial age, and
project `attentionOrigin=legacy_unknown`; they are never inferred to be owned
by the native observer. In particular, legacy `metadataSource=codex_hook`
normalizes to `attentionSource=agent` with its stored confidence and
`legacy_unknown` origin. Other legacy source values retain their existing
priority tier. Normalization may occur in memory; the next accepted write
persists the new fields. If legacy file age cannot be read, Peon must preserve
the current tuple until a valid attention write establishes a timestamp.
Session projections keep their current one-record semantics; they do not
compose attention from another record.

The attention SourceBadge in session details must use `attentionSource` and
`attentionConfidence`; `metadataSource` remains the provenance for general
work metadata. This keeps the visible attribution aligned with the tuple after
the sources split.

Persist an attention-specific update time with the attention tuple and source.
Peon's `agent` staleness check uses that time, not the session
file's modification time. The current boundary remains strict: age 15 seconds
blocks Peon; age greater than 15 seconds permits it. A write to unrelated
metadata must not refresh this timestamp. Legacy records without the field
must follow a documented compatibility rule that does not make an active
native wait immediately stale; the implementation handoff must settle this
rule before coding.

The attention source order remains `user > agent > peon > backend_inference >
process > unknown > debug`. Direct agent, harness-hook, and native Codex
attention all use the `agent` tier; they are peers, not separate priority
levels. Every accepted attention write from one of those producers revokes
the prior native ownership token, including identical-value writes. Only
`user` has strictly higher attention priority. The versioned read response
exposes the metadata revision needed for writes; `attentionUpdatedAt` is
persisted and projected for arbitration and diagnosis.

The existing work-metadata source ladder continues to protect descriptive
fields such as task, summary, and blockers. Peon applies attention and
work-metadata source checks independently, so an agent metadata patch can
protect its summary without refreshing or changing attention ownership.

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
  and distinguishes omitted groups from explicit-null clears.
- A stale agent revision returns conflict and cannot replace newer metadata.
- A write from any sidecar producer advances `metadataRevision`, so an agent
  patch read before a native, Peon, hook, user, or lifecycle write conflicts.
- A competing agent write and native clear are serialized: whichever commits
  first is reflected by the next read, and an earlier agent write prevents
  the old native owner from clearing.
- Native clear rechecks ownership after any staged I/O and before the atomic
  replacement while the per-session writer lock remains held.
- Every in-product active-session writer uses the versioned sidecar contract;
  active direct JSON replacement remains unsupported by the eligible runtime
  contract.
- The persisted attention update time changes on attention/source writes,
  remains unchanged for unrelated writes, and preserves the strict Peon
  boundary at 15 seconds versus greater than 15 seconds.
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
- End, restart, forget, delete, and retention do not restore native clear
  authority from stale state.

The production native-clear gate can be removed only after the written design
and implementation handoff are approved, these tests pass on every supported
platform, required CI and review pass on the exact PR head, and the separate
#690 and #763 verification gates are satisfied. Reducer tests, cooperating
writer tests, or revision checks alone are insufficient.

## Handoff

This design changes no runtime code and does not itself enable native clearing.
After owner approval, create an implementation issue for the versioned
metadata API, producer migration, single-record serialized writes, lifecycle
handling, staleness timestamp, projection error behavior, and the behavioral
tests above. Keep #690's production gate closed until that work and its
separate native verification gates are complete.
