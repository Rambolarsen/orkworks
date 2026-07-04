# Domain Entities

This document describes the current Rust domain model under `crates/orkworksd/src/domain/`.

It is intentionally narrower than the full product specs. The goal here is to document the domain entities and supporting types that exist in code today, how they relate to the metadata store and API layer, and where the current terminology boundary sits.

## Scope

Today the domain layer is centered on **agent sessions**.

The code uses:

- `Session` as the aggregate root in the Rust domain layer
- `Harness` for the internal coding-tool integration abstraction
- `provider_id` for the inference-service identity when it is known

The user-facing UI may call the selected CLI application a **Coding tool**, but the domain model still stores that internal concept as harness-related data.

This document reflects the current implementation. It does not introduce launch profiles.

## Session aggregate

File: `crates/orkworksd/src/domain/session/entity.rs`

`Session` is the primary domain entity and aggregate root.

Responsibilities:

- identify one running or remembered agent session
- capture lifecycle state
- capture attention and memory state
- hold the selected harness/model/provider references when known
- hold workspace and Git context
- hold resume metadata for harness-supported continuation

Current fields:

- `id: SessionId`
- `workspace_path: WorkspacePath`
- `status: SessionStatus`
- `memory_state: MemoryState`
- `attention_state: AttentionState`
- `work_phase: WorkPhase`
- `lifecycle_phase: LifecyclePhase`
- `pending_terminal_status: Option<TerminalOutcome>`
- `ending_observed_status_snapshot: Option<ObservedStatusSnapshot>`
- `final_observed_status_snapshot: Option<ObservedStatusSnapshot>`
- `created_at: String`
- `killed_at: Option<String>`
- `last_active_at: Option<String>`
- `harness_name: Option<String>`
- `provider_id: Option<String>`
- `task_description: Option<String>`
- `label: String`
- `cwd: String`
- `model: Option<String>`
- `repo_root: Option<String>`
- `branch: Option<String>`
- `dirty: Option<bool>`
- `changed_files: Option<usize>`
- `is_worktree: Option<bool>`
- `resume: Option<crate::harness::ResumeMemory>`
- `resumed_from: Option<String>`
- `resume_strategy: crate::harness::ResumeStrategy`

Behavior currently implemented on the entity:

- `is_live()`
- `is_killed()`
- `can_be_killed()`
- `kill(now)`
- `mark_running()`
- `mark_active()`
- `begin_ending(pending_terminal_status, ending_observed_status_snapshot)`
- `complete_ending(final_observed_status_snapshot)`

The lifecycle behavior is still orchestrated by runtime/application code, but the aggregate now owns the `creating -> active -> ending -> ended` transition rules and the structured final-observed-state snapshots.

## Value objects

File: `crates/orkworksd/src/domain/session/value_objects.rs`

The session domain currently uses these value objects and enums:

### `SessionId`

Stable identity for a session aggregate.

### `WorkspacePath`

Wrapper around the owning workspace path.

### `SessionStatus`

Lifecycle status:

- `Creating`
- `Running`
- `Killed`
- `Ended`
- `Error`

This captures the process state or terminal outcome. During `LifecyclePhase::Ending`, the session intentionally remains `SessionStatus::Running` until completion chooses the final terminal outcome.

### `MemoryState`

Persistence/restore classification:

- `Live`
- `Remembered`
- `Resumable`
- `Unsupported`

This is distinct from lifecycle status. A killed or ended session can still be remembered or resumable.

### `AttentionState`

Observed attention state:

- `WaitingForInput`
- `Blocked`
- `Failed`
- `Done`
- `Stale`
- `Working`
- `Idle`

This is the domain form of the attention model used for sorting and UI emphasis. `needs_attention()` is defined here.

### `WorkPhase`

High-level work phase:

- `Ideation`
- `Implementation`
- `Review`
- `Debugging`
- `Unknown`

This remains intentionally coarse.

`Phase` still exists in code today as a type alias to `WorkPhase` for compatibility, but the canonical domain term is now `WorkPhase`.

### `LifecyclePhase`

Explicit runtime lifecycle phase:

- `Creating`
- `Active`
- `Ending`
- `Ended`

This is distinct from `SessionStatus`. It models where the session is in the lifecycle state machine even when the process-facing `status` remains `Running` during `Ending`.

### `TerminalOutcome`

Pending/final terminal outcome:

- `Ended`
- `Killed`
- `Error`

This is used while a session is in `LifecyclePhase::Ending` so the aggregate can defer choosing the terminal `SessionStatus` until finalization completes.

### `ObservedStatusSnapshot`

Structured frozen observer state:

- `value: Option<AttentionState>`
- `source: String`
- `confidence: Option<f64>`
- `observed_at: Option<String>`

This is used for both:

- `ending_observed_status_snapshot` captured when the session begins ending
- `final_observed_status_snapshot` persisted when ending completes

## Domain events

File: `crates/orkworksd/src/domain/session/events.rs`

The domain emits a small set of session events:

- `SessionCreated`
- `SessionKilled`
- `SessionResumed`
- `SessionAttentionChanged`
- `SessionForgotten`

These events preserve the existing external naming convention through `event_type()`:

- `session.created`
- `session.killed`
- `session.resumed`
- `session.attention_changed`
- `session.forgotten`

This is important because infrastructure currently writes session/event data into the metadata store using those string conventions.

## Domain service

File: `crates/orkworksd/src/domain/session/services.rs`

`SessionLifecycle` is the current domain service.

Responsibilities:

- create a new `Session` aggregate plus `SessionCreated` event
- kill a session and emit `SessionKilled`
- resume a session and emit `SessionResumed`

Notable design choices:

- creation accepts optional Git context and normalizes it into the aggregate
- creation accepts optional harness/provider/model references
- resume support is still tied to `crate::harness::ResumeMemory` and `ResumeStrategy`
- creation initializes `work_phase = WorkPhase::Unknown`
- creation initializes `lifecycle_phase = LifecyclePhase::Creating`

The service is where aggregate construction rules currently live.

## Repository port

File: `crates/orkworksd/src/domain/session/repository.rs`

`SessionRepository` is the domain persistence port.

Methods:

- `save(session, events)`
- `load(id)`
- `delete(id)`
- `list_by_workspace(path)`
- `append_terminal_output(id, lines)`

This port makes two boundaries explicit:

1. session persistence and event persistence move together
2. terminal scrollback is treated as repository-owned session data even though it is not part of the aggregate itself

## Mapping to infrastructure

The domain model is not the same thing as the API DTOs or raw metadata files.

Current mapping path:

```text
Session aggregate
  -> infrastructure/session_repository.rs
  -> metadata::SessionMetadata
  -> main.rs SessionInfo JSON DTO
  -> apps/desktop/src/api.ts SessionInfo
  -> renderer components
```

Important consequence:

- the domain can stay `Harness`-oriented internally
- the UI can still present `Coding tool`
- compatibility aliases can exist at metadata/API boundaries without forcing a full domain rewrite

## Terminology boundary in the domain

Current intended interpretation:

- `harness_name`: internal coding-tool integration identity
- `provider_id`: inference-service identity when known
- `model`: selected model when known
- `Session`: one running or remembered agent session

This means:

- a harness is not a model provider
- a model provider is optional
- a model is optional
- the session aggregate may legitimately know only the harness and not the provider/model

Where provider/model identity cannot be determined, the domain should preserve `None` rather than invent values.

## Current limitations

This domain layer is still session-centric and intentionally small.

Not yet modeled as first-class domain entities:

- launch profiles
- coding-tool definitions as aggregates
- model providers as aggregates
- recommendation entities
- capacity entities

Those concepts may exist elsewhere in the codebase as infrastructure/config/runtime types, but they are not yet represented as domain aggregates in `crates/orkworksd/src/domain/`.

## Related files

- `crates/orkworksd/src/domain/session/entity.rs`
- `crates/orkworksd/src/domain/session/value_objects.rs`
- `crates/orkworksd/src/domain/session/events.rs`
- `crates/orkworksd/src/domain/session/services.rs`
- `crates/orkworksd/src/domain/session/repository.rs`
- `crates/orkworksd/src/infrastructure/session_repository.rs`
- `docs/agents/architecture.md`
