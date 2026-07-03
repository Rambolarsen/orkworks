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
- the final observed state is frozen into a dedicated `finalObservedStatus`

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

Meaning: the current process state or terminal outcome.

### `observedStatus`

Live Peon or agent-observed attention state. This is only meaningful while `lifecyclePhase = active`.

### `finalObservedStatus`

Frozen historical observed state captured during the `ending` phase. This is displayed for historical context after the session reaches `ended`.

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

- preserve the intended terminal `status`
- transition `lifecyclePhase` to `ending`

The runtime must always pass through `ending`. This rule is consistent across all exit paths.

### Final scan completion

During `ending`, the runtime attempts one final Peon scan against the last buffered output snapshot.

If the scan succeeds:

- write `finalObservedStatus` from the final inference if present, otherwise preserve the last known observed state
- clear live `observedStatus`
- transition `lifecyclePhase` to `ended`
- set final `status` to `ended`, `killed`, or `error`

If the scan fails or times out:

- log the failure
- preserve the last known observed state as `finalObservedStatus`
- clear live `observedStatus`
- transition `lifecyclePhase` to `ended`
- set final `status` to `ended`, `killed`, or `error`

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
- `begin_ending(pending_terminal_status)`
- `complete_ending(final_terminal_status, final_observed_status)`

The implementation should use these operation names unless a compile-time conflict forces a near-identical spelling. The domain API must make the state machine explicit and prevent skipping `ending`.

## Peon Behavior

### Active phase

While `lifecyclePhase = active`:

- normal Peon inference is allowed
- `observedStatus` remains the live observer field

### Ending phase

While `lifecyclePhase = ending`:

- one final Peon inference attempt is allowed
- runtime orchestration, not generic metadata merge logic, owns the timeout and completion rule

### Ended phase

While `lifecyclePhase = ended`:

- Peon inference is disabled
- `observedStatus` is no longer considered live state
- `finalObservedStatus` is the only observer state shown for historical context

## Persistence Model

### Metadata

Persist:

- `workPhase`
- `lifecyclePhase`
- `status`
- `observedStatus`
- `finalObservedStatus`

Rules:

- `observedStatus` is for active sessions only
- `finalObservedStatus` is populated when the session completes ending
- metadata persistence must reflect domain/runtime decisions, not invent lifecycle behavior on write

### Events

Append timestamped lifecycle transition events covering:

- `creating -> active`
- `active -> ending`
- `ending -> ended`

Final-scan timeout or failure should produce diagnostic logging and may emit a dedicated event, but it must not prevent the `ending -> ended` transition.

## API and Frontend

Expose to the frontend:

- `workPhase`
- `lifecyclePhase`
- `status`
- `observedStatus`
- `finalObservedStatus`

Frontend behavior:

- attention and sorting use `observedStatus` only when `lifecyclePhase = active`
- non-active sessions must never regain live attention from historical observer state
- detail views display `finalObservedStatus` as historical context after the session has ended

## Non-goals

- Do not change the set of observed attention-state values
- Do not redesign the session list or detail panel layout
- Do not change the meaning of `status`
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

### Metadata/API tests

Cover:

- `phase` is replaced by `workPhase`
- `lifecyclePhase` serializes correctly
- `finalObservedStatus` serializes when present
- active sessions do not expose stale final-state fields as live attention

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
- `crates/orkworksd/src/http/session_handlers.rs`
- `crates/orkworksd/src/session_types.rs`
- `apps/desktop/src/api.ts`
- `apps/desktop/src/sessionSort.ts`
- `apps/desktop/src/components/SessionDetailPanel.tsx`
- `docs/agents/domain-entities.md`
- `AGENTS.md` if terminology references need alignment

## Migration Notes

This change renames a persisted field from `phase` to `workPhase`. Implementation should support backward-compatible reads for existing metadata files during rollout, while all new writes use `workPhase` consistently.

## Acceptance Summary

Issue `#26` is complete when:

- lifecycle is domain-owned and explicit
- the old work-classification `phase` name is retired in favor of `workPhase`
- all exit paths flow through `ending`
- final observer state is frozen into `finalObservedStatus`
- timeout/failure of the final scan cannot block completion
- frontend behavior keys off `lifecyclePhase` rather than ad hoc lifecycle inference
