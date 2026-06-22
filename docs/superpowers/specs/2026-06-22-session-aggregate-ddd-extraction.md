# Session Aggregate DDD Extraction

**Date:** 2026-06-22
**Status:** draft

Extract the Session domain concept from the God-object `main.rs` (1420+ line HTTP handler monolith) and the flat `SessionInfo` TypeScript type into a proper Domain-Driven Design aggregate, with domain/application/infrastructure layering in Rust and a matching domain layer in TypeScript, using the existing OrkWorks ubiquitous language.

## Scope

- Rust: extract Session into `crates/orkworksd/src/domain/session/`, `crates/orkworksd/src/application/session/`, `crates/orkworksd/src/infrastructure/` — thin HTTP handlers remain in `crates/orkworksd/src/main.rs`
- TypeScript: add `apps/desktop/src/domain/session.ts` with the same value objects, enums, and sorting logic — wire format (`apps/desktop/src/api.ts`) unchanged
- Existing modules (`crates/orkworksd/src/metadata.rs`, `crates/orkworksd/src/git.rs`, `crates/orkworksd/src/peon.rs`, `crates/orkworksd/src/harness.rs`, `crates/orkworksd/src/providers.rs`, `crates/orkworksd/src/watcher.rs`) become backing implementations for adapters — no refactoring
- Peon, Taskmaster, capacity, recommendations, providers: untouched, follow-up work

## Rust

### Domain layer (`domain/session/`)

No I/O or platform dependencies. Domain-owned primitives only (e.g. timestamps). `GitContext` is a domain-defined value object — infrastructure maps `git2` types into it.

**Value objects** — equality by value, immutable:

```
SessionId(String)
SessionStatus(Creating | Running | Killed | Ended | Error)
MemoryState(Live | Remembered | Resumable | Unsupported)
AttentionState(WaitingForInput | Blocked | Failed | Done | Stale | Working | Idle)
Phase(Ideation | Implementation | Review | Debugging | Unknown)
WorkspacePath(PathBuf)
```

**Aggregate root** — identity by `SessionId`, consistency boundary:

```
Session {
  id: SessionId
  workspace_path: WorkspacePath
  status: SessionStatus
  memory_state: MemoryState
  attention_state: AttentionState
  phase: Phase
  created_at: DateTime
  killed_at: Option<DateTime>
  last_active_at: Option<DateTime>
  git_context: Option<GitContext>
  harness_name: Option<String>
  provider_id: Option<String>
  task_description: Option<String>
  resume_memory: Option<ResumeMemory>
  terminal_lines: Vec<String>  // in-memory buffer, persisted separately
}
```

**Domain events** — past-tense, immutable record of change:

```
SessionCreated { session_id, created_at, harness_name, workspace_path }
SessionKilled { session_id, killed_at, exit_code }
SessionResumed { session_id, resumed_at, previous_session_id }
SessionAttentionChanged { session_id, old_state, new_state }
SessionForgotten { session_id, deleted_at }
```

**Repository trait** (driven port) — one repository per aggregate:

```
SessionRepository {
  save(session: Session, events: Vec<DomainEvent>) -> Result<()>
  load(id: SessionId) -> Result<Option<Session>>
  delete(id: SessionId) -> Result<()>
  list_by_workspace(path: WorkspacePath) -> Result<Vec<Session>>
  append_terminal_output(id: SessionId, lines: Vec<String>) -> Result<()>
}
```

**Domain service** — stateless logic that spans entities or doesn't fit a single entity:

```
SessionLifecycle {
  create(command: CreateSessionCommand, git_context: Option<GitContext>) -> (Session, Vec<DomainEvent>)
  kill(session: Session) -> (Session, Vec<DomainEvent>)
  resume(session: Session, previous_session_id: SessionId) -> (Session, Vec<DomainEvent>)
}
```

`SessionLifecycle` enforces invariants that don't require I/O: cannot kill an already-killed session, status transitions are validated (e.g. Running → Killed is valid, Killed → Running is not). Existence checks (e.g. verifying a referenced `previous_session_id` exists) happen in the application handler before calling the domain service.

### Application layer (`application/session/`)

Orchestrates domain + infrastructure. Depends on domain and driven ports.

**Commands** — DTOs carrying external intent:

```
CreateSessionCommand { harness_name, cwd, resume_strategy, resume_session_id? }
KillSessionCommand { session_id }
ResumeSessionCommand { session_id }
ForgetSessionCommand { session_id }
RefreshAttentionCommand { session_id }
ListWorkspaceSessionsCommand { workspace_path }
```

**Driven ports** — interfaces the application needs, implemented in infrastructure:

```
PtySpawner { spawn(id: SessionId, cwd: PathBuf, command: String) -> Result<PtyHandle> }
PtyKiller { kill(handle: PtyHandle) -> Result<()> }
GitDetector { detect(path: PathBuf) -> Result<Option<GitContext>> }
```

`PtyHandle` is an opaque handle — the application layer doesn't know how PTYs work.

**Use case handlers:**

```
CreateSessionHandler {
  handle(repo: SessionRepository, pty: PtySpawner, git: GitDetector, lifecycle: SessionLifecycle, cmd: CreateSessionCommand)
    -> Result<Session>
}

KillSessionHandler {
  handle(repo: SessionRepository, pty: PtyKiller, lifecycle: SessionLifecycle, cmd: KillSessionCommand)
    -> Result<Session>
}

ResumeSessionHandler {
  handle(repo: SessionRepository, pty: PtySpawner, git: GitDetector, lifecycle: SessionLifecycle, cmd: ResumeSessionCommand)
    -> Result<Session>
}

ForgetSessionHandler {
  handle(repo: SessionRepository, cmd: ForgetSessionCommand) -> Result<()>
}

ListWorkspaceSessionsHandler {
  handle(repo: SessionRepository, cmd: ListWorkspaceSessionsCommand) -> Result<Vec<Session>>
}
```

### Infrastructure layer (`infrastructure/`)

Adapters implementing driven ports, plus composition root.

**Adapters:**

```
MetadataSessionRepository implements SessionRepository
  - delegates to existing MetadataStore (metadata.rs)
  - serializes Session to .orkworks/sessions/<id>.json
  - appends domain events to .orkworks/events/<id>.ndjson
  - maps domain types ↔ metadata JSON types

PtySessionSpawner implements PtySpawner
  - extracts PTY creation from current main.rs create_session handler
  - wraps portable-pty spawn logic

PtySessionKiller implements PtyKiller
  - extracts PTY cleanup from current main.rs kill_session handler

Git2Detector implements GitDetector
  - wraps existing git.rs GitContext detection
```

**Composition root:**

```
SessionModule {
  create_session_handler: CreateSessionHandler
  kill_session_handler: KillSessionHandler
  resume_session_handler: ResumeSessionHandler
  forget_session_handler: ForgetSessionHandler
  list_handler: ListWorkspaceSessionsHandler
}
```

Wired in `main.rs` at startup with concrete adapter implementations.

### main.rs changes

HTTP handlers collapse to ~5 lines each — deserialize request, delegate to `SessionModule`, serialize response:

```rust
async fn create_session(
    State(state): State<AppState>,
    Json(body): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let cmd = CreateSessionCommand::from(body);
    match state.session_module.create_session_handler.handle(cmd).await {
        Ok(session) => (StatusCode::CREATED, Json(session)),
        Err(e) => error_response(e),
    }
}
```

`AppState` drops only the sessions `Mutex<HashMap>` and inline session HTTP handlers. Keeps providers, harnesses, adapters, retention config, and Peon loop management. Peon still reads session state through `AppState.session_module` — the Peon loop is not refactored in this extraction.

Expected result: `main.rs` drops from ~1420 lines to ~150 lines.

## TypeScript

### Domain layer (`apps/desktop/src/domain/session.ts`)

Same value objects and enums, same names:

```typescript
declare const __sessionIdBrand: unique symbol;
type SessionId = string & { readonly [__sessionIdBrand]: true };

enum SessionStatus { Creating = "creating", Running = "running", Killed = "killed", Ended = "ended", Error = "error" }
enum MemoryState { Live = "live", Remembered = "remembered", Resumable = "resumable", Unsupported = "unsupported" }
enum AttentionState { WaitingForInput = "waiting_for_input", Blocked = "blocked", Failed = "failed", Done = "done", Stale = "stale", Working = "working", Idle = "idle" }
enum Phase { Ideation = "ideation", Implementation = "implementation", Review = "review", Debugging = "debugging", Unknown = "unknown" }
```

**Domain model** — identity + behavior:

```typescript
interface Session {
  id: SessionId;
  workspacePath: string;
  status: SessionStatus;
  memoryState: MemoryState;
  attentionState: AttentionState;
  phase: Phase;
  created: Date;
  killed?: Date;
  lastActive?: Date;
  gitContext?: GitContext;
  harnessName?: string;
  providerId?: string;
  taskDescription?: string;
  isDirty: boolean;
  branch?: string;
}
```

**Domain logic extracted from `sessionSort.ts`:**

```typescript
function needsAttention(session: Session): boolean
function sessionAttentionStatus(session: Session): AttentionState
function sortSessions(sessions: Session[]): Session[]
```

**Mappers** — convert between wire DTO and domain model:

```typescript
function fromApiDto(dto: SessionInfo): Session
function toApiDto(session: Session): Partial<SessionInfo>
```

### Existing files

| File | Change |
|------|--------|
| `apps/desktop/src/api.ts` | Unchanged — wire format stays |
| `apps/desktop/src/sessionSort.ts` | Gutted — delegates to `domain/session.ts` functions |
| `apps/desktop/src/App.tsx` | Calls domain layer for sort/display, not raw arrays and inline sorts |
| Components | Unchanged — same props, same rendering |

## Ubiquitous language — both sides aligned

| Rust | TypeScript | Meaning |
|------|-----------|---------|
| `Session`, `SessionId`, `SessionStatus` | `Session`, `SessionId`, `SessionStatus` | Same concept, same name |
| `MemoryState`, `AttentionState`, `Phase` | `MemoryState`, `AttentionState`, `Phase` | Same enum values |
| `SessionCreated`, `SessionKilled`, `SessionResumed` | Same event names in API responses | Same domain events |
| `WorkspacePath` | `workspacePath` | Same concept |

## Dependency rule

```
Infrastructure -> Application -> Domain
   (adapters)     (use cases)    (core)
```

- `domain/` imports nothing from `application/` or `infrastructure/`
- `application/` imports from `domain/` and port traits only
- `infrastructure/` imports from `domain/` and `application/` ports
- `main.rs` imports from `infrastructure/` (composition) and `application/` (command types)

## Not in scope

- Taskmaster / recommendation engine
- Capacity model and persistence
- Peon refactoring (module stays as-is; loop management stays in `AppState`)
- Provider refactoring
- Harness refactoring
- UI panel changes
- `.orkworks/recommendations/` creation
- Any new external dependencies
