# Session State Model

This document describes the current Rust session state model in `crates/orkworksd/src/`.

There is no separate `domain/`/`application/` DDD layer in this crate — an earlier scaffold along those lines existed but was never wired into production and has been removed. Session state is modeled directly as data: `SessionMetadata` (the on-disk/API source of truth) plus the in-memory `SessionHandle`/`SessionInfo` types used by the HTTP and runtime layers.

## Scope

Today the state model is centered on **agent sessions**.

The code uses:

- `SessionMetadata` (`metadata.rs`) as the persisted record for one session
- `SessionInfo` (`main.rs`) as the in-memory/API DTO built from metadata plus live runtime state
- `Harness` for the internal coding-tool integration abstraction
- `provider_id` for the inference-service identity when it is known

The user-facing UI may call the selected CLI application a **Coding tool**, but the internal data still stores that concept as harness-related fields.

## SessionMetadata

File: `crates/orkworksd/src/metadata.rs`

`SessionMetadata` is the persisted record read from and written to `~/.orkworks/workspaces/<hash>/sessions/<id>.json`. It is a flat, serde-mapped struct — not an aggregate with behavior. Notable fields:

- `id`, `label`, `workspace`, `task`, `cwd`
- `harness: String` (serialized `harnessId`, aliased from legacy `harness`)
- `model: String` (serialized `modelId`, aliased from legacy `model`)
- `status: String` — process/terminal state (see vocabulary below)
- `work_phase: String` (serialized `workPhase`) — `ideation` | `implementation` | `review` | `debugging` | `unknown`, normalized on read via `normalize_work_phase`
- `lifecycle_phase: String`, `lifecycle: String` — see vocabulary below
- `attention: Option<String>`, `connectivity: String`
- `terminal_outcome: Option<String>`, `pending_terminal_status: Option<String>`
- `observed_status: Option<String>` plus `ending_observed_status_snapshot` / `final_observed_status_snapshot` (`ObservedStatusSnapshotMetadata { value, source, confidence, observed_at }`)
- `summary`, `next_action`, `needs_user_input`, `detected_question`, `suggested_options`, `blocker_description`, `failed_command`, `failed_test`, `capacity_hints`, `peon_last_inference` — Peon-inferred fields
- `provider_id`, `provider_label`, `provider_model`, `provider_state`
- `created_at`, `last_activity`, `metadata_source`, `metadata_confidence`
- `repo_root`, `branch`, `dirty`, `changed_files`, `is_worktree` — Git context
- `resume`, `resume_options`, `resumed_from`, `harness_session_id_source/confidence/captured_at`
- `last_user_input`

`normalize_session_metadata` runs on every read to backfill defaults and reconcile `lifecycle`/`lifecycle_phase` drift between old and new records.

## Status vocabulary

Unlike a typed domain enum, `status`/`lifecycle`/`lifecycle_phase`/`attention`/`connectivity` are plain strings, and the vocabulary has grown organically across the HTTP and runtime layers rather than being centrally enumerated. Known values in current use:

- `status`: `creating`, `running`, `killed`, `ended`, `error`
- `lifecycle`: `creating`, `alive`, `stopping`, `dead` (ADR 0023 canonical form); `ending` also appears as an intermediate value in places
- `lifecycle_phase`: `creating`, `active`, `ending`, `ended` (`default_lifecycle_phase_for_status` derives this from `status` when absent)
- `attention` (raw observed values, before `canonical_attention` collapses them): `working`, `idle`, `blocked`, `failed`, `capped`, `waiting_for_input`, `stale`, `done`; canonicalized to `needs_you` / `idle` / passthrough for the UI-facing attention model
- `connectivity`: `online` (default) and other values set via `connectivity_for_status`
- `terminal_outcome`: `ended`, `killed`, `error`

Because these are untyped strings threaded through many call sites rather than a single enum, adding a new status value does not get compiler-checked exhaustiveness — grep for the field name across `http/` and `runtime/` before assuming a closed set.

## Terminology boundary

Current intended interpretation:

- `harness`: internal coding-tool integration identity
- `provider_id`: inference-service identity when known
- `model`: selected model when known
- a session is one running or remembered agent session, identified by `id`

This means:

- a harness is not a model provider
- a model provider is optional
- a model is optional
- a session may legitimately know only the harness and not the provider/model

Where provider/model identity cannot be determined, code should preserve `None`/empty rather than invent values.

## Mapping to the API/UI layer

```text
SessionMetadata (on disk)
  -> SessionHandle / SessionInfo (main.rs, in-memory)
  -> HTTP JSON DTO (http/session_handlers.rs)
  -> apps/desktop/src/api.ts SessionInfo
  -> renderer components
```

## Prior DDD scaffold (removed)

An earlier `domain/session/` + `application/session/` + `infrastructure/session_*` layer existed as a typed alternative to the above (a `Session` aggregate with a 5-variant `SessionStatus` enum, `MemoryState`, `AttentionState`, `WorkPhase`, `LifecyclePhase`, domain events, a `SessionRepository` port, and command handlers). It was never wired into any production code path — `SessionModule` was constructed only to populate an unread `AppState` field, and its PTY adapters were unimplemented stubs. It has been deleted. Two gaps would need to be closed before a typed state machine like this could work: it modeled only the 5-variant `SessionStatus` enum where production status vocabulary is the larger untyped set documented above, and it had no representation of PTY runtime state (`kill_tx`, output buffers, `SessionRuntime`) at all. See [issue #181](https://github.com/Rambolarsen/orkworks/issues/181) for the idea captured for future work.

## Related files

- `crates/orkworksd/src/metadata.rs`
- `crates/orkworksd/src/main.rs` (`SessionHandle`, `SessionInfo`, `AppState`)
- `crates/orkworksd/src/http/session_handlers.rs`
- `docs/agents/architecture.md`
