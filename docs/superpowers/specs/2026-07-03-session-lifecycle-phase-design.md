# Session Lifecycle Phase Design

> **Date:** 2026-07-03
> **Scope:** Domain-owned session lifecycle phases with final observed-state freezing

## Goal

Implement issue `#26` by introducing an explicit runtime lifecycle state machine owned by the Rust session domain, while also renaming the existing inferred work-classification field to avoid semantic collision.

## Why This Change

Today the codebase mixes three different concerns:

- process/outcome state in `status`
- inferred work classification in `phase`
- live observer state in `observedStatus`

That leaves lifecycle behavior spread across runtime code, metadata merge rules, and frontend guards. The current narrower fix only prevents non-running sessions from surfacing stale live attention. It does not remove the race window between process exit and Peon inference, and it does not make lifecycle ownership explicit.

This design makes lifecycle a first-class domain concern:

- the existing `phase` field is renamed to `workPhase`
- a new `lifecyclePhase` field tracks runtime lifecycle explicitly
- the final observed state is frozen into a dedicated `finalObservedStatusSnapshot`, with `finalObservedStatus` exposed only as a derived API/frontend projection

## Terminology

### `workPhase`

The existing inferred task-type field, renamed from `phase`.

Values:

- `ideation`
- `implementation`
- `review`
- `debugging`
- `unknown`

Meaning: what kind of work the session appears to be doing.

`workPhase` remains enum-constrained at the persistence and API boundary. Legacy or free-form Peon `phase` strings that do not match the allowed values must map to `unknown`. Backward-compatible reads should continue accepting legacy `phase` as an input alias during rollout.

### `lifecyclePhase`

The new runtime lifecycle field.

Values:

- `creating`
- `active`
- `ending`
- `ended`

Meaning: where the session is in its runtime lifecycle.

### `status`

The existing process/outcome detail field remains.

Values:

- `creating`
- `running`
- `ended`
- `killed`
- `error`

Meaning: the current lifecycle-visible process state or terminal outcome.

`running` means the session has not yet completed lifecycle finalization. During `lifecyclePhase = ending`, the backing process may already have exited while `status` remains `running` until `complete_ending(...)` commits the terminal outcome.

### `observedStatus`

Live Peon or agent-observed attention state. This is only meaningful while `lifecyclePhase = active`.

### `ObservedStatusSnapshot`

Canonical structured observer snapshot type used for ending-state capture, final frozen state, recovery, and finalization.

Type:

- `value: ObservedStatus | null`
- `source: MetadataSource | "recovery" | "unknown"`
- `confidence: number | null`
- `observedAt: ISO-8601 timestamp | null`

Rules:

- the snapshot object itself may be present even when `value = null`
- `source = "recovery"` is allowed for synthesized recovery-time snapshots
- `source = "unknown"` is allowed when provenance is unavailable
- `confidence = null` is allowed when the snapshot was synthesized or provenance is unavailable
- `observedAt = null` is allowed when the original observation time is unavailable
- canonical synthesized null snapshot is `{ value: null, source: "recovery", confidence: null, observedAt: null }`
- canonical legacy backfill snapshot uses preserved source, confidence, and observedAt when available; otherwise `{ value: <legacy value>, source: "unknown", confidence: null, observedAt: null }`

### `finalObservedStatusSnapshot`

Frozen historical `ObservedStatusSnapshot` captured during the `ending` phase.

Persisted JSON shape:

```json
{
  "value": "blocked",
  "source": "peon",
  "confidence": 0.82,
  "observedAt": "2026-07-03T12:34:56Z"
}
```

Rules:

- the object itself is nullable before lifecycle finalization completes
- when present, it obeys the `ObservedStatusSnapshot` type above
- this is the backend source of truth for historical observed state after `ended`
- normalized or finalized `ended` sessions must have a populated snapshot object, using the canonical synthesized null snapshot when no observed value exists

### `finalObservedStatus`

Frontend/API convenience field derived from `finalObservedStatusSnapshot.value`.

Type: nullable string using the same observed-status vocabulary as `observedStatus`.

### `endingObservedStatusSnapshot`

Backend-internal `ObservedStatusSnapshot` captured atomically at `begin_ending(...)`.

Persisted JSON shape:

```json
{
  "value": "working",
  "source": "agent",
  "confidence": 1.0,
  "observedAt": "2026-07-03T12:30:00Z"
}
```

Rules:

- the object is required while `lifecyclePhase = ending`
- when present, it obeys the `ObservedStatusSnapshot` type above
- object presence is distinct from object absence

It exists only to guarantee deterministic fallback finalization and crash recovery. It is persisted for backend recovery and never exposed in frontend DTOs.

## Recommended Approach

Use a domain-first refactor.

The Rust session domain should own lifecycle transitions explicitly. Runtime code detects process events and triggers domain transitions. Metadata persists the resulting state. Peon participates as an observer during `active` and a single final scan during `ending`, but it does not own lifecycle decisions.

This is preferred over a metadata-first patch because issue `#26` is fundamentally about removing lifecycle races and clarifying ownership boundaries, not only about changing serialized fields.

## Domain Model

### Session aggregate

The `Session` aggregate should own:

- `work_phase`
- `lifecycle_phase`
- `status`
- `pending_terminal_status`
- `ending_observed_status_snapshot`
- `final_observed_status_snapshot`

The aggregate remains the source of truth for whether a session is live, ending, or terminal.

### New enum

Add a domain `LifecyclePhase` enum with:

- `Creating`
- `Active`
- `Ending`
- `Ended`

### Renames

Rename the existing domain `Phase` enum and related fields to `WorkPhase`.

The rename applies across:

- Rust domain model
- metadata serialization
- HTTP/API DTOs
- frontend types
- docs

## Transition Rules

### Creation

When a session is created:

- `status = creating`
- `lifecyclePhase = creating`

### Activation

When the runtime marks the session as live:

- `status = running`
- `lifecyclePhase = active`

### Exit detection

When the process exits for any reason, including normal exit, kill, or error:

- preserve the intended terminal `status` in a dedicated pending terminal field
- transition `lifecyclePhase` to `ending`

The runtime must always pass through `ending`. This rule is consistent across all exit paths.

During `ending`:

- `status` remains `running`
- `pendingTerminalStatus` holds one of `ended`, `killed`, or `error`
- `endingObservedStatusSnapshot` captures the last accepted live observed state atomically at `begin_ending`

This avoids reporting a session as already terminal before finalization completes while still preserving the exit cause.

### Final scan completion

During `ending`, the runtime attempts one final Peon scan against the last buffered output snapshot.

If the scan succeeds:

- if the final scan returns a valid snapshot object, use it as authoritative
- a final inference snapshot with `value = null` is authoritative and must not fall back to `endingObservedStatusSnapshot`
- if no final inference snapshot object is returned, preserve `endingObservedStatusSnapshot`
- clear live `observedStatus`
- transition `lifecyclePhase` to `ended`
- set final `status` from `pendingTerminalStatus`
- clear `pendingTerminalStatus`

If the scan fails or times out:

- log the failure
- preserve `endingObservedStatusSnapshot` as `finalObservedStatusSnapshot`
- clear live `observedStatus`
- transition `lifecyclePhase` to `ended`
- set final `status` from `pendingTerminalStatus`
- clear `pendingTerminalStatus`

## Final Scan Timeout

The final scan timeout should be configurable and default to `2` seconds.

Requirements:

- configuration is explicit in backend Peon/runtime config
- default is `2` seconds when not configured
- timeout expiration must not block transition to `ended`

## Domain Ownership

Lifecycle transitions should be represented explicitly in the Rust domain model and service layer rather than being encoded only as metadata side effects.

Required domain operations:

- `mark_active()`
- `begin_ending(pending_terminal_status, ending_observed_status_snapshot)`
- `complete_ending(final_observed_status_snapshot)`

The implementation should use these operation names unless a compile-time conflict forces a near-identical spelling. The domain API must make the state machine explicit and prevent skipping `ending`.

`complete_ending(...)` must be idempotent so duplicate runtime notifications or overlapping final-scan completion paths cannot freeze the session twice.
Timeout completion and final-scan completion race through the same idempotent operation; whichever commits first is authoritative.
The first successful `complete_ending(...)` transition wins. Later completion attempts are no-ops.

## Lifecycle Invariants

| lifecyclePhase | status | pendingTerminalStatus | observedStatus | finalObservedStatusSnapshot | Peon inference | Attention hook writes |
|---|---|---|---|---|---|---|
| `creating` | `creating` | `null` | allowed but ignored for attention | `null` | no | no |
| `active` | `running` | `null` | live | `null` | yes | yes |
| `ending` | `running` | `ended` or `killed` or `error` | frozen input only, no new live writes | `null` until completion | one final attempt only | no |
| `ended` | `ended` or `killed` or `error` | `null` | `null` | historical snapshot | no | no |

Rules:

- `status` must never be terminal while `lifecyclePhase = active`.
- `pendingTerminalStatus` is required while `lifecyclePhase = ending`.
- `endingObservedStatusSnapshot` is required while `lifecyclePhase = ending`, even when its value is `null`.
- `finalObservedStatusSnapshot` must not be used for attention or sorting.
- `observedStatus` must be cleared before or at `ending -> ended`.

## Peon Behavior

### Active phase

While `lifecyclePhase = active`:

- normal Peon inference is allowed
- `observedStatus` remains the live observer field

### Ending phase

While `lifecyclePhase = ending`:

- one final Peon inference attempt is allowed
- runtime orchestration, not generic metadata merge logic, owns the timeout and completion rule
- normal in-flight Peon results must be ignored once `begin_ending(...)` has been recorded
- if Peon is disabled, the provider is unavailable, or the buffered snapshot is empty or useless, finalization skips inference and immediately freezes `endingObservedStatusSnapshot`
- agent attention writes such as `POST /sessions/:id/attention` must be rejected or ignored unless `lifecyclePhase = active`
- final-scan completion must freeze observer state exactly once
- finalization may use only either the final-scan result or `endingObservedStatusSnapshot` captured at `begin_ending`

### Ended phase

While `lifecyclePhase = ended`:

- Peon inference is disabled
- `observedStatus` is no longer considered live state
- `finalObservedStatusSnapshot` is the persisted historical observer state
- `finalObservedStatus` is a derived frontend/API convenience from `finalObservedStatusSnapshot.value`

## Persistence Model

### Metadata

Persist:

- `workPhase`
- `lifecyclePhase`
- `status`
- `pendingTerminalStatus`
- `endingObservedStatusSnapshot`
- `observedStatus`
- `finalObservedStatusSnapshot`

Rules:

- `observedStatus` is for active sessions only
- `pendingTerminalStatus` is only populated while `lifecyclePhase = ending`
- `endingObservedStatusSnapshot` is written atomically during `begin_ending(...)` and is only populated while `lifecyclePhase = ending`
- `finalObservedStatusSnapshot` is populated when the session completes ending
- metadata persistence must reflect domain/runtime decisions, not invent lifecycle behavior on write

### Normalization and recovery

Persisted records must be normalized on read/startup.

Rules:

- if a persisted session is found in `lifecyclePhase = ending` with a valid `pendingTerminalStatus`, recover it to `lifecyclePhase = ended`
- recovery finalization uses `pendingTerminalStatus` as the final `status`
- recovery finalization uses `finalObservedStatusSnapshot` if already present, otherwise `endingObservedStatusSnapshot` if present, otherwise a synthesized snapshot from legacy or stale `observedStatus` if present, otherwise the canonical synthesized null snapshot `{ value: null, source: "recovery", confidence: null, observedAt: null }`
- recovery finalization clears `pendingTerminalStatus` and live `observedStatus`
- recovery may skip final inference because the original runtime snapshot owner is gone after restart
- invalid combinations such as non-null `pendingTerminalStatus` outside `ending` must normalize to `pendingTerminalStatus = null`
- invalid `ending` records without `pendingTerminalStatus` must normalize directly to `lifecyclePhase = ended`, `status = error`, and preserve any available historical observed state

### Events

Append timestamped lifecycle transition events covering:

- `creating -> active`
- `active -> ending`
- `ending -> ended`

Final-scan timeout or failure should produce diagnostic logging and may emit a dedicated event, but it must not prevent the `ending -> ended` transition.

## Runtime Ownership Constraint

This refactor is not complete unless runtime lifecycle writes stop bypassing the domain path.

Implementation constraint:

- process-start, process-exit, kill, and error transitions must flow through domain/application lifecycle operations
- runtime helpers that currently write status or metadata directly must be refactored to call those operations instead
- metadata persistence becomes a projection of domain/runtime decisions, not an alternate authority

This specifically includes exit/status handling in `crates/orkworksd/src/runtime/terminal_runtime.rs`.

## API and Frontend

Expose to the frontend:

- `workPhase`
- `lifecyclePhase`
- `status`
- `observedStatus`
- `finalObservedStatus`

Do not expose `pendingTerminalStatus` to the frontend. It is an internal runtime/domain field used to preserve exit cause during `ending`.
Do not expose `endingObservedStatusSnapshot` to the frontend. It is a backend-internal lifecycle recovery field.

Frontend behavior:

- attention and sorting use `observedStatus` only when `lifecyclePhase = active`
- non-active sessions must never regain live attention from historical observer state
- detail views display `finalObservedStatus` as historical context after the session has ended

MVP API contract:

- expose `finalObservedStatus` now
- frontend DTOs always include `finalObservedStatus`, set to `finalObservedStatusSnapshot.value` when a snapshot exists, otherwise `null`
- do not expose `finalObservedStatusSnapshot` in MVP frontend DTOs
- a future advanced/debug API may expose `finalObservedStatusSnapshot` provenance explicitly, but that is not part of this MVP contract

## Non-goals

- Do not change the set of observed attention-state values
- Do not redesign the session list or detail panel layout
- Do not change the `status` value set or terminal outcome meanings; `running` remains the only non-terminal active/ending value, but it is not a guaranteed process-liveness flag once `lifecyclePhase = ending`
- Do not add automatic terminal control or Peon autonomy beyond observer behavior

## Testing Requirements

### Domain tests

Cover:

- `creating -> active`
- `active -> ending`
- `ending -> ended`
- terminal `status` survives the `ending` phase correctly
- invalid transition shortcuts are rejected or impossible

### Runtime tests

Cover:

- every process exit path enters `ending`
- final Peon scan is attempted once
- timeout defaults to `2s`
- configured timeout overrides the default
- timeout or inference failure still transitions to `ended`
- no further Peon inference runs after `ended`
- restart recovery finalizes persisted `ending` sessions deterministically

### Metadata/API tests

Cover:

- `phase` is replaced by `workPhase`
- `lifecyclePhase` serializes correctly
- `finalObservedStatusSnapshot` serializes correctly
- `finalObservedStatus` always serializes in frontend DTOs as `ObservedStatus | null`
- active sessions do not expose stale final-state fields as live attention
- `pendingTerminalStatus` normalizes to `null` outside `ending`
- backend persistence may store recovery-only fields that frontend DTOs exclude

### Frontend tests

Cover:

- attention ignores `finalObservedStatus` for non-active sessions
- sorting respects `lifecyclePhase`
- detail panel shows `finalObservedStatus` as historical context

## Files Likely Affected

- `crates/orkworksd/src/domain/session/value_objects.rs`
- `crates/orkworksd/src/domain/session/entity.rs`
- `crates/orkworksd/src/domain/session/services.rs`
- `crates/orkworksd/src/infrastructure/session_repository.rs`
- `crates/orkworksd/src/metadata.rs`
- `crates/orkworksd/src/runtime/peon_runtime.rs`
- `crates/orkworksd/src/runtime/terminal_runtime.rs`
- `crates/orkworksd/src/http/session_handlers.rs`
- `crates/orkworksd/src/session_types.rs`
- `apps/desktop/src/api.ts`
- `apps/desktop/src/sessionSort.ts`
- `apps/desktop/src/components/SessionDetailPanel.tsx`
- `docs/agents/domain-entities.md`
- `AGENTS.md` if terminology references need alignment

## Migration Notes

This change renames a persisted field from `phase` to `workPhase` and introduces persisted `lifecyclePhase`, `pendingTerminalStatus`, `endingObservedStatusSnapshot`, `finalObservedStatusSnapshot`, plus projected or API-level `finalObservedStatus`.

Backward-compatible read rules during rollout:

- if `workPhase` is missing, read legacy `phase`
- if the legacy `phase` value is free-form or unknown, coerce to `workPhase = unknown`
- if `lifecyclePhase` is missing and `status` is `creating`, derive `lifecyclePhase = creating`
- if `lifecyclePhase` is missing and `status` is `running`, derive `lifecyclePhase = active`
- if `lifecyclePhase` is missing and `status` is `ended`, `killed`, or `error`, derive `lifecyclePhase = ended`
- if `pendingTerminalStatus` is missing on a derived `creating`, `active`, or `ended` record, normalize it to `null`
- if `pendingTerminalStatus` is non-null outside `ending`, normalize it to `null`
- if `endingObservedStatusSnapshot` is missing outside `ending`, normalize it to `null`
- if `endingObservedStatusSnapshot` is present outside `ending`, preserve it only long enough to finish normalization and then clear it
- if `finalObservedStatusSnapshot` is missing on a derived terminal session and legacy `observedStatus` is present, synthesize a snapshot object from that legacy value with best-effort provenance
- if `finalObservedStatusSnapshot` is missing on a derived terminal session and no legacy `observedStatus` exists, populate the canonical synthesized null snapshot
- if `finalObservedStatus` is missing on a derived terminal session and `finalObservedStatusSnapshot` is present, expose `finalObservedStatusSnapshot.value`
- if a session is derived terminal from legacy metadata, do not expose legacy `observedStatus` as live state

Write rules:

- all new writes use `workPhase`
- all new writes populate `lifecyclePhase`
- active sessions write `observedStatus` only
- ending sessions write `pendingTerminalStatus` and `endingObservedStatusSnapshot`
- ended sessions write `finalObservedStatusSnapshot`; API projection may emit `finalObservedStatus`

## Acceptance Summary

Issue `#26` is complete when:

- lifecycle is domain-owned and explicit
- the old work-classification `phase` name is retired in favor of `workPhase`
- all exit paths flow through `ending`
- final observer state is frozen into `finalObservedStatusSnapshot`, with `finalObservedStatus` derived for API or frontend convenience
- timeout/failure of the final scan cannot block completion
- frontend behavior keys off `lifecyclePhase` rather than ad hoc lifecycle inference
