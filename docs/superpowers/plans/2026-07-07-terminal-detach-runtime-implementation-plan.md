# Terminal Detach Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decouple PTY/process lifetime from terminal WebSocket attachment so live sessions survive UI detach, can be reattached safely, and allow the renderer to dispose inactive terminals without killing work.

**Architecture:** Introduce a sidecar-owned session runtime layer in `orkworksd` that starts and drains PTYs independently of any WebSocket attachment, then refactor the terminal socket into an attach/detach transport with explicit ownership tokens and replay cursors. After the backend is stable, simplify the renderer to keep only the active terminal attached so hidden xterm instances stop consuming render budget.

**Tech Stack:** Rust, Axum WebSockets, Tokio, portable-pty, React, TypeScript, xterm.js, existing `orkworksd` unit tests

## Global Constraints

- Preserve the single interactive attachment rule from the July 5 design; no multi-view interactive terminals.
- PTY lifetime must become session-runtime-owned, not WebSocket-owned.
- Detach must not change `lifecyclePhase`; only process exit, kill, or runtime error may drive `ending`/`ended`.
- Detached runtimes must continue draining PTY output, persisting terminal history, and feeding Peon.
- The first implementation must reject live duplicate attaches; superseding a live attachment is out of scope.
- App-restart PTY persistence is out of scope; persisted sessions from an earlier `orkworksd` process still reconcile through existing metadata rules.
- Use TDD for behavior changes: write the failing test first, run it red, then implement the minimal fix.
- Update ADRs and repo docs required by `AGENTS.md` before closing the work.

---

### Task 1: Record the architecture decision and repo docs

**Files:**
- Create: `docs/adr/0022-session-runtime-owned-pty-lifetime.md` (or the next free ADR number if `0022` is taken at execution time)
- Modify: `docs/adr/README.md`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/agents/architecture.md`

**Interfaces:**
- Consumes: `docs/superpowers/specs/2026-07-07-terminal-detach-runtime-design.md`, `docs/adr/0010-pty-portable-pty-xtermjs.md`, `docs/adr/0013-single-active-context-primitive.md`, `docs/adr/0021-session-lifecycle-phases.md`
- Produces: ADR 0022 documenting session-runtime-owned PTY lifetime and the preserved single-attachment constraint

- [ ] **Step 1: Write ADR 0022**

Add an ADR with this skeleton:

```md
# Session-runtime-owned PTY lifetime

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-07-07

## Context

Terminal WebSocket lifetime currently owns PTY lifetime. That makes detach destructive and prevents safe renderer-side terminal disposal.

## Decision

- PTY/process lifetime is owned by the sidecar session runtime.
- WebSocket attachment is an independent `detached`/`attached` concern.
- One interactive attachment per session remains the rule.
- Detach does not change `lifecyclePhase`; only process exit, kill, or runtime error does.
- App-restart PTY persistence is out of scope for the initial design.

## Consequences

- Detached sessions keep running and keep feeding persistence/Peon.
- Frontend can dispose inactive terminals safely.
- Replay and owner-scoped attach cleanup become required runtime behaviors.
```

- [ ] **Step 2: Update the ADR index and repo docs**

Reflect the new runtime behavior in:

```md
- `docs/adr/README.md`: add ADR 0022 to the table
- `README.md`: note that session runtimes can survive terminal detach while the sidecar stays alive
- `AGENTS.md`: add the new ADR reference where architecture/runtime conventions are summarized
- `docs/agents/architecture.md`: update the terminal/PTY data-flow section so PTY lifetime is sidecar-owned
```

- [ ] **Step 3: Verify the docs diff is self-consistent**

Run: `git diff -- docs/adr/0022-session-runtime-owned-pty-lifetime.md docs/adr/README.md README.md AGENTS.md docs/agents/architecture.md`
Expected: The docs all describe the same ownership model and still preserve single-active-context language.

### Task 2: Introduce an explicit session runtime abstraction in Rust

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/runtime/mod.rs`
- Create: `crates/orkworksd/src/runtime/session_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`

**Interfaces:**
- Consumes: `SessionHandle`, existing PTY/env helpers from `terminal_runtime.rs`
- Produces: `SessionRuntime`, `AttachmentState`, `ReplayCursor`, `start_session_runtime(...)`, `claim_attachment(...)`

- [ ] **Step 1: Write the failing runtime ownership tests**

Add tests in `crates/orkworksd/src/runtime/session_runtime.rs` covering:

```rust
#[test]
fn session_runtime_starts_detached() { /* runtime exists before websocket attach */ }

#[test]
fn live_duplicate_attach_is_rejected() { /* second attach rejected while first token held */ }

#[test]
fn stale_cleanup_is_owner_scoped() { /* wrong token cannot clear current owner */ }

#[test]
fn replay_cursor_advances_monotonically() { /* replay/live handoff has a stable cursor */ }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_runtime_ -- --nocapture`
Expected: FAIL because `SessionRuntime`, attachment tokens, and replay cursor state do not exist yet.

- [ ] **Step 3: Create the runtime abstraction**

Implement focused runtime-owned types:

```rust
pub(crate) struct ReplayCursor(pub u64);

pub(crate) struct ReplayRing {
    pub start_cursor: ReplayCursor,
    pub chunks: VecDeque<(ReplayCursor, Vec<u8>)>,
}

pub(crate) struct AttachmentState {
    pub generation: u64,
    pub attached: bool,
}

pub(crate) struct SessionRuntime {
    pub writer_tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    pub output_tx: tokio::sync::broadcast::Sender<RuntimeEvent>,
    pub reader_task: tokio::task::JoinHandle<()>,
    pub persist_task: tokio::task::JoinHandle<()>,
    pub wait_task: tokio::task::JoinHandle<()>,
    pub last_rows: u16,
    pub last_cols: u16,
    pub replay: ReplayRing,
    pub replay_cursor: ReplayCursor,
    pub attachment: AttachmentState,
}

pub(crate) enum RuntimeEvent {
    Output { cursor: ReplayCursor, chunk: Vec<u8> },
    Ended { status: String },
    Error { code: String, message: String },
}
```

Update `SessionHandle` so it holds runtime-owned state instead of a bare `terminal_attached: bool`:

```rust
struct SessionHandle {
    info: SessionInfo,
    kill_tx: tokio::sync::watch::Sender<bool>,
    output_buffer: peon::RingBuffer,
    scan_buf: String,
    command: harness::CommandSpec,
    initial_prompt: Option<String>,
    runtime: SessionRuntime,
    at_usage_limit_latched: bool,
    capacity_check_pending: bool,
    output_lines_seen: u64,
    scan_bytes_seen: u64,
    resume_scan_origin: Option<(u64, u64)>,
    pending_capacity_visible_once: bool,
}
```

- [ ] **Step 4: Run the runtime ownership tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_runtime_ -- --nocapture`
Expected: PASS

### Task 3: Start and drain PTYs independently of terminal WebSockets

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Modify: `crates/orkworksd/src/main.rs`

**Interfaces:**
- Consumes: `create_session(...)`, `resume_session(...)`, `kill_tx`, existing PTY read/persist/Peon logic
- Produces: detached runtime startup on create/resume, `24x80` fallback sizing, persistent PTY drain independent of WebSocket, explicit startup-failure behavior, child wait/reap ownership, and kill/delete integration

- [ ] **Step 1: Write the failing detached-runtime tests**

Add tests covering:

```rust
#[tokio::test]
async fn create_session_starts_runtime_before_attach() { /* session becomes live without websocket */ }

#[tokio::test]
async fn detached_runtime_keeps_output_draining() { /* output_buffer/history advance without attachment */ }

#[tokio::test]
async fn detached_runtime_updates_peon_inputs() { /* last_output / scan buffer advance while detached */ }

#[tokio::test]
async fn detached_start_uses_fallback_size() { /* runtime starts 24x80 when no prior size exists */ }

#[tokio::test]
async fn create_session_returns_error_when_runtime_start_fails() { /* no half-live session on PTY open/spawn failure */ }

#[tokio::test]
async fn detached_runtime_reaps_child_and_finalizes_on_natural_exit() { /* wait task finalizes without websocket */ }

#[tokio::test]
async fn delete_session_kills_detached_runtime_and_finalizes() { /* delete/kill does not depend on attachment */ }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml detached_runtime_ -- --nocapture`
Expected: FAIL because PTYs are still spawned inside `handle_session_terminal(...)`.

- [ ] **Step 3: Move PTY startup to create/resume**

Refactor session creation/resume so runtime startup happens there:

```rust
let runtime = start_session_runtime(
    state.clone(),
    id.clone(),
    command.clone(),
    initial_prompt.clone(),
    PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 },
).await?;

handle.runtime = runtime;
handle.info.status = "running".to_string();
handle.info.lifecycle_phase = "active".to_string();
```

The runtime task must keep:

```rust
- reading PTY bytes
- appending persisted terminal output
- updating output_buffer / scan_buf / replay cursor
- feeding Peon timing (`last_output`, `last_inference`)
- waiting/reaping the child process independently of any websocket
- scheduling terminal finalization on natural exit, kill, or runtime error
```

Startup error contract:

```rust
- if PTY open/spawn fails during create_session or resume_session, return an API error before exposing a live runtime
- do not leave a session in "active" with no running PTY
- if metadata/session records are created before startup finishes, finalize them consistently as failed rather than leaving orphaned "running" state
```

- [ ] **Step 4: Run the detached-runtime tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml detached_runtime_ -- --nocapture`
Expected: PASS

### Task 4: Refactor terminal WebSocket handling into attach, replay, and explicit end signaling

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_http.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`

**Interfaces:**
- Consumes: `SessionRuntime`, `ReplayCursor`, runtime attachment token/generation
- Produces: attach-only `handle_session_terminal(...)`, replay handoff, explicit typed control messages, and websocket detach semantics independent of process lifetime

- [ ] **Step 1: Write the failing attach/replay tests**

Add tests covering:

```rust
#[tokio::test]
async fn detach_does_not_end_session() { /* websocket close only clears attachment */ }

#[tokio::test]
async fn reattach_replays_from_cursor_then_continues_live() { /* no dropped output across handoff */ }

#[tokio::test]
async fn terminal_end_signal_marks_session_ending() { /* explicit runtime end still drives finalization */ }

#[tokio::test]
async fn terminal_unavailable_message_does_not_mark_session_ended() { /* duplicate attach conflict is not an end event */ }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml detach_does_not_end_session -- --nocapture`
Expected: FAIL because socket close currently kills the child and the socket task still owns finalization.

- [ ] **Step 3: Make the WebSocket an attach transport**

Refactor `handle_session_terminal(...)` around a runtime attach flow:

```rust
let attachment = claim_attachment(&state, &id)?;
send_replay_from_cursor(&mut ws, &id, attachment.cursor).await?;

loop {
    tokio::select! {
        msg = ws.recv() => { /* input + resize only */ }
        event = attachment.events.recv() => { /* binary PTY output or explicit end event */ }
    }
}
```

Key rules:

```rust
- websocket close clears only the matching attachment token
- runtime end sends an explicit terminal-end event before close/final detach
- duplicate live attach returns conflict / "terminal unavailable"
- replay/live handoff uses one cursor model, not timing-based best effort
```

Define the websocket control schema before frontend work starts:

```json
{ "type": "replay-start", "cursor": 42 }
{ "type": "replay-end", "cursor": 57 }
{ "type": "ended", "status": "ended" }
{ "type": "error", "code": "pty_spawn_failed", "message": "..." }
{ "type": "terminal-unavailable", "reason": "already-attached" }
```

Binary frames remain PTY output only.

- [ ] **Step 4: Run the attach/replay tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml detach_does_not_end_session -- --nocapture`
Expected: PASS

- [ ] **Step 5: Run the remaining attach/replay tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml reattach_replays_from_cursor_then_continues_live -- --nocapture`
Expected: PASS

- [ ] **Step 6: Run the terminal end/control-message tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_end_signal_ -- --nocapture`
Expected: PASS

### Task 5: Simplify the renderer to keep only the active terminal attached

**Files:**
- Modify: `apps/desktop/src/terminalStore.ts`
- Modify: `apps/desktop/src/components/CenterPanel.tsx`
- Modify: `apps/desktop/src/App.tsx`

**Interfaces:**
- Consumes: terminal WebSocket explicit end signaling, replay-on-attach backend behavior
- Produces: `detachTerminal(id: string)`, active-session-only attachment, no automatic “socket close means session ended”

- [ ] **Step 1: Write the failing frontend tests**

Add concrete renderer-side regressions around terminal lifecycle, for example:

```ts
test("detaching the active terminal does not mark the session ended", () => { /* close without end event */ });
test("switching sessions detaches the old terminal and attaches the new one", () => { /* active-only attach */ });
test("explicit ended control message marks the terminal ended", () => { /* ended message flips ended state */ });
test("after switching sessions only one terminal handle remains live", () => { /* old handle disposed/detached */ });
```

- [ ] **Step 2: Run the frontend regression to verify it fails**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: FAIL in the new terminal lifecycle assertion because `onclose` still marks any closed socket as ended.

- [ ] **Step 3: Refactor terminal attachment state in the renderer**

Split detach from final disposal:

```ts
export function detachTerminal(id: string): void {
  const handle = terminals.get(id);
  if (!handle) return;
  handle.ws.close();
  handle.resizeObserver.disconnect();
  handle.wrapper.remove();
  terminals.delete(id);
}
```

Update close handling so only an explicit backend end signal marks the terminal ended:

```ts
ws.onmessage = (e) => {
  if (typeof e.data === "string") {
    const msg = JSON.parse(e.data);
    if (msg.type === "ended") handle.ended = true;
    if (msg.type === "terminal-unavailable") handle.ended = false;
    return;
  }
  term.write(new Uint8Array(e.data));
};
```

Switching sessions in `CenterPanel` / `App.tsx` should detach the previously active terminal before attaching the new one so only one xterm stays live in the DOM.

- [ ] **Step 4: Run the frontend regression**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: PASS

### Task 6: Final verification and repo currency checks

**Files:**
- Verify only

**Interfaces:**
- Consumes: all runtime, frontend, ADR, and docs changes
- Produces: evidence that detach/reattach works without violating lifecycle or single-attachment rules

- [ ] **Step 1: Run focused Rust tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_runtime_ -- --nocapture`
Expected: PASS

- [ ] **Step 2: Run focused detached-runtime tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml detached_runtime_ -- --nocapture`
Expected: PASS

- [ ] **Step 3: Run focused attach/replay tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml detach_does_not_end_session -- --nocapture`
Expected: PASS

- [ ] **Step 4: Run focused replay/control tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml reattach_replays_from_cursor_then_continues_live -- --nocapture`
Expected: PASS

- [ ] **Step 5: Run focused terminal end/control-message tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_end_signal_ -- --nocapture`
Expected: PASS

- [ ] **Step 6: Run the full Rust test suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: PASS

- [ ] **Step 7: Run the desktop tests**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: PASS

- [ ] **Step 8: Run type-check for the renderer**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 9: Run the doc currency check**

Run: `bash .claude/hooks/doc-check.sh`
Expected: exit 0, or actionable doc follow-ups listed and addressed
