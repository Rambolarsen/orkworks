# Codex Native Attention Layer for Session Metadata

- Status: proposed; implementation approval and verification remain gated
- Deciders: repository owner, Codex
- Date: 2026-10-08
- Issue: [#761](https://github.com/Rambolarsen/orkworks/issues/761)
- Related: [#690](https://github.com/Rambolarsen/orkworks/issues/690),
  [ADR 0076](../../adr/0076-codex-owned-native-approval-observer.md)

## Context

ADR 0076 requires native approval clearing to revoke its authority whenever a
competing writer changes the attention tuple or its record-wide source. The
sidecar can serialize cooperating `MetadataStore` writes and issue runtime-only
write tokens. It cannot make a final check and rename atomic against a supported
agent that directly replaces `sessions/<session-id>.json`.

The MVP explicitly supports reading and writing that session JSON, watching it
for changes, and trusting explicit agent-written records. The direct writer
does not participate in a sidecar lock. A second check of file identity narrows
the race but cannot close the interval between the check and rename.

## Supported attention and metadata producers

| Producer | Current input and authority | Proposed storage |
| --- | --- | --- |
| User/manual override | Explicit user action; `user` is the highest source tier | Existing session JSON |
| Direct agent JSON | Agent reads or replaces `sessions/<id>.json`; the MVP treats explicit agent metadata as authoritative | Existing session JSON, unchanged |
| Harness hook/API | Validated harness events arrive through authenticated sidecar routes or the Codex report relay; sidecar currently persists their attention in session JSON | Codex live attention moves to the native layer only for an eligible native runtime; other harnesses and fallback Codex launches keep existing behavior |
| Peon | Terminal observation and inference, source `peon` | Existing session JSON |
| Backend inference and process lifecycle | Deterministic sidecar inference and process/runtime state | Existing session JSON |
| Codex native observer | A validated hook candidate plus bounded exact-root native observation; it may resolve only a wait it owns | Codex native attention layer |
| Debug injection | Debug-only temporary state injection, below normal runtime sources | Existing session JSON |

Session lifecycle, accepted terminal input, reset, and hook authority changes
also update attention. In a native-enabled Codex runtime, they update the same
Codex layer instead of clearing or replacing another producer's session JSON.

## Decision

For feature-eligible native Codex sessions, store the complete Codex live
attention tuple in a separate, sidecar-owned record. Keep
`sessions/<session-id>.json` as the existing source record. Setting or clearing
the Codex layer never writes, deletes, or renames the session JSON file.

The proposed directory is `native-attention/<session-id>.json` beneath the
workspace metadata root. It is separate from `sessions/`, so the session-file
reader and watcher cannot mistake it for a `SessionMetadata` record. The new
record contains a schema version, session ID, monotonic layer revision, the
full Codex attention tuple, and a `live` or `final` lifecycle state. It contains
no report bearer, native connection secret, or runtime ownership token.

All validated Codex attention transitions for an eligible native session use
this layer: prompt submission, permission candidates, native pending and
resolved observations, stop, accepted committed input, reset/revocation, and
runtime termination. It stores both waiting and non-waiting Codex states. This
prevents removal of a native wait from revealing an old `codex_hook` value left
in the base record.

The layer is written atomically before its state is published to live session
views. Its writes and conditional clears serialize through the sidecar's
per-session ownership boundary while the existing single-writer workspace
lease is held. The ownership token is scoped to this persisted layer: every
accepted layer write, including an identical-value write, revokes the prior
token and increments the layer's monotonic revision. A native clear persists a
revisioned cleared state rather than unlinking the layer while the session
exists. A failed write does not publish a state transition and cannot authorize
a native clear. Tokens remain process-local and are never persisted.

### Read and write boundaries

`MetadataStore::read_session` remains a raw read for read-modify-write paths.
It must not return merged native fields, because a subsequent ordinary write
could persist those fields into the agent-owned record. Session-list and
session-detail projections use a distinct merged read path that composes the
raw record with the Codex layer. The public session/API shape stays unchanged.

Only the sidecar owns `native-attention/`. Supported direct agents continue to
read and write `sessions/<id>.json`; they need no lock, API, migration, or new
file knowledge. This isolates the clear operation from those writes. It is not
a security boundary against an actor that deliberately edits arbitrary files
under the workspace metadata root.

### Projection and precedence

Projection uses the existing whole attention tuple:
`observed_status`/`attention`, `needs_user_input`, `detected_question`, and
`suggested_options`. It never combines individual fields from competing
producers.

1. A `user` tuple in the base record wins over the native layer.
2. Explicit `agent` JSON in the base record wins over the native layer. This
   preserves the existing MVP authority of direct agent-written JSON even when
   its tuple has the same values as the native tuple.
3. Otherwise, an active Codex layer is projected as `codex_hook` attention and
   follows the existing `codex_hook` source-priority and Peon staleness rules.
   It takes precedence over backend inference and process-only state.
4. If no active Codex layer applies, the existing session record and source
   priority rules apply unchanged.

When a higher-priority user or agent tuple masks the Codex layer, the native
observer may still retire its own candidate. The clear changes only the native
record, so the user or agent tuple remains available to the next projection.
This deliberately narrows ADR 0076's ownership token from the composed
session-wide tuple to the native layer itself: a base-record write is not a
write to that layer and does not revoke its token. The safety property is that
base authority always wins projection and native clear never mutates it.

### Consistent projection

The base JSON and native layer are separate files, so a merged read is not a
cross-file transaction. Each attempt reads, in order: base identity and exact
bytes plus parsed record; native-layer identity, revision, and exact bytes
plus parsed record; base identity and exact bytes again; native-layer identity,
revision, and exact bytes again. The attempt succeeds only when the base
identity/byte pair matches and the layer identity/byte pair and monotonic
revision match. Layer revisions never repeat while a session exists, including
for identical-value writes and clears. Thus a native clear committed during
projection changes the second layer observation and forces a retry. The stable
layer revision and second base observation define the projection point; a
subsequent native write or direct base replacement is later and appears on the
next read or file-watcher refresh. Native-layer writes use atomic replacement.

On either pair changing or either read failing, the adapter retries the full
attempt up to three times. If it cannot stabilize the base and layer, it
returns the latest valid base record without the native overlay (or no session
if no valid base record can be read). It never guesses that a failed or
unstable read means the base source is lower priority. The adapter must
distinguish an absent layer from an I/O or parse failure; only a confirmed
initial absence can participate as revision zero.

The layer revision check orders the projection against native commits, while
the base identity and byte check detects direct replacement or in-place
content changes during composition. The design does not promise a transaction
spanning files or coordination with a direct writer that changes the base
after the final base observation. It does promise that native clear never
overwrites the base record and that a clear committed before the layer
recheck cannot be hidden by a successful stale projection.

### Persistence and lifecycle

The layer is durable across ordinary writes and reads while its session runtime
is live, but it is not authority that survives a sidecar restart. On orderly
session end, the layer becomes `final`: it can supply only the existing
`final_observed_status_snapshot` projection for the ended session, and all live
clear authority is revoked. Restart reconciliation ends sessions under the
existing lifecycle contract and converts orphaned `live` layers to `final`
snapshots; a persisted `live` marker never recreates a runtime owner or clear
token. Final snapshots do not project into current attention. Forget, delete,
and retention paths remove the companion record with the same session
identity as the base record. No report credential or ownership token is
restored from disk.

Native-disabled and unsupported Codex configurations keep the existing direct
launch and session-JSON behavior. There is no bulk migration of existing
records. An eligible native runtime must initialize its layer for its own
launch generation before projecting native attention; legacy base records stay
readable and are never rewritten merely to create the layer.

## Alternatives considered

### Coordinate all writers with a shared lock

This would require every direct agent writer to adopt a new lock protocol or
replace direct JSON writes with a sidecar API. An advisory lock only works for
cooperating writers; imposing it would change the currently supported MVP
protocol and could silently lose writes from existing agents. It is not the
selected approach.

### Keep the record-wide tuple and leave clearing disabled

This is safe and remains the rollout behavior until implementation evidence is
complete, but it does not resolve the writer gap in #690. It is the fallback
if the layer cannot meet the acceptance evidence below.

## Compatibility and limits

- The `sessions/<id>.json` schema and direct agent read/write contract do not
  change.
- Clients continue to receive one session view with the existing attention
  fields; the sidecar owns composition.
- A direct `user` or `agent` tuple can mask native attention by design, in
  accordance with metadata authority. The native observer never erases it.
- Direct base writes do not revoke the native-layer token. ADR 0076 is amended
  to use layer-scoped ownership; source precedence and bounded identity/content
  rechecks protect the independently stored base record.
- The layer serializes sidecar writers. It does not protect against deliberate
  external modification of the new sidecar-owned path.
- Restart ends live sessions under the existing contract. Persistence does
  not resume native ownership, pending approvals, or clear authority.
- #763's listener ownership gate, #690's signal/configuration/platform gates,
  and ADR 0076's native verification requirements remain independent.

## Required implementation evidence

The implementation follow-up must include behavioral tests proving:

- An identical direct agent tuple written while a native candidate is active
  remains the projected tuple after native clear.
- A direct replacement of `sessions/<id>.json` after the native clear's final
  authority check but before the native-layer commit is not overwritten or
  hidden after the clear.
- A direct replacement or in-place content change to either file during view
  composition causes a retry; if the pair cannot be stabilized, projection
  fails closed by omitting native attention.
- A native clear that commits between the first layer read and the layer
  recheck cannot return pre-clear attention as a successful stable projection.
- The tests use an independent direct file replacement (including atomic
  rename), not only a cooperating `MetadataStore` writer.
- Native set/clear failure, duplicate native writes, and unrelated base
  read-modify-write paths do not publish or persist merged fields incorrectly.
- Projection preserves the entire user/agent tuple and uses the Codex layer
  only over lower-priority base sources.
- End, forget, delete, retention, and restart reconciliation do not revive
  authority from stale layer records.

The production native-clear gate can be removed only after the written design
and implementation handoff are approved, these tests pass on every supported
platform, required CI and review pass on the exact PR head, and the separate
#690 and #763 verification gates are satisfied. Reducer tests, cooperating
writer tests, or file-identity checks alone are insufficient.

## Handoff

This design changes no runtime code and does not itself enable native clearing.
Create an implementation issue for the isolated store, attention transition
routing, merged projections, lifecycle cleanup, and the behavioral tests above.
Keep #690's production gate closed until that work and its separate native
verification gates are complete.
