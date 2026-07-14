# Terminal Input Backpressure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bound memory growth on the renderer-to-PTY terminal input path so a disconnected transport or a stalled PTY writer cannot accumulate unbounded input in memory, without silently dropping normal interactive keystrokes.

**Architecture:** Two independent changes. (1) Sidecar: replace the unbounded `RuntimeCommand` control channel with a bounded one and make its two send functions `async`, so a full queue makes the affected session's own websocket read loop wait for room (real backpressure, no data loss, no cross-session impact). (2) Renderer: cap the `pendingInput` buffer used while the terminal WebSocket is disconnected at 64 KiB via a new pure helper, dropping further input past the cap and surfacing one visible warning line per disconnect.

**Tech Stack:** Rust (tokio `mpsc`, Axum, `portable_pty`) for the sidecar; TypeScript (`@xterm/xterm`, native WebSocket) for the renderer; `cargo test` and Node's built-in `node --experimental-strip-types --test` test runner.

**Spec:** `docs/superpowers/specs/2026-07-14-terminal-input-backpressure-design.md`

---

### Task 1: Bound the sidecar `RuntimeCommand` control channel

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:17` (add constant)
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:104-159` (`SessionRuntime` struct + `live`/`detached`)
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:214-244` (`send_runtime_command`, `update_runtime_size`)
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:246-284` (`capture_startup_runtime_state`, `start_session_runtime` signature)
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:627-648` (3 call sites)
- Test: `crates/orkworksd/src/runtime/session_runtime.rs` (`mod tests`)

- [ ] **Step 1: Write the failing test for bounded capacity**

Add this test inside `mod tests` in `crates/orkworksd/src/runtime/session_runtime.rs`, right after the existing `already_working_output_stays_working` test (or any convenient point in the same `mod tests` block):

```rust
    #[tokio::test]
    async fn send_runtime_command_blocks_until_capacity_available_then_succeeds() {
        let session_id = "runtime-capacity-test";
        let state = test_state_with_runtime_session(session_id);
        let (runtime, mut control_rx) =
            SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
        {
            let mut sessions = state.sessions.lock().unwrap();
            sessions.get_mut(session_id).unwrap().runtime = runtime;
        }

        // Fill the bounded channel to capacity without draining it.
        for _ in 0..CONTROL_CHANNEL_CAPACITY {
            send_runtime_command(&state, session_id, RuntimeCommand::Input("x".into()))
                .await
                .unwrap();
        }

        // The channel is now full; one more send should not resolve until something drains it.
        let state_clone = state.clone();
        let blocked_send = tokio::spawn(async move {
            send_runtime_command(
                &state_clone,
                session_id,
                RuntimeCommand::Input("overflow".into()),
            )
            .await
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !blocked_send.is_finished(),
            "send on a full bounded channel should not resolve immediately"
        );

        // Draining one slot should let the pending send complete.
        let _ = control_rx.recv().await;
        let result = tokio::time::timeout(Duration::from_secs(1), blocked_send)
            .await
            .expect("blocked send should complete once a slot frees up")
            .unwrap();
        assert!(result.is_ok());
    }
```

This test will not compile yet because `send_runtime_command` is not `async` and `CONTROL_CHANNEL_CAPACITY` does not exist. That is the expected RED state for this step.

- [ ] **Step 2: Run the test to confirm it fails to compile**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml send_runtime_command_blocks_until_capacity_available_then_succeeds`
Expected: compile error, something like `no method named 'await' found` or `cannot find value 'CONTROL_CHANNEL_CAPACITY'`.

- [ ] **Step 3: Add the capacity constant**

In `crates/orkworksd/src/runtime/session_runtime.rs`, find:

```rust
const DRIVER_EVENT_BUFFER_CAPACITY: usize = 64;
const PERSIST_QUEUE_CAPACITY: usize = 64;
```

Change to:

```rust
const DRIVER_EVENT_BUFFER_CAPACITY: usize = 64;
const PERSIST_QUEUE_CAPACITY: usize = 64;
const CONTROL_CHANNEL_CAPACITY: usize = 64;
```

- [ ] **Step 4: Bound the channel in `SessionRuntime`**

In the same file, find:

```rust
#[derive(Debug)]
pub(crate) struct SessionRuntime {
    pub(crate) control_tx: mpsc::UnboundedSender<RuntimeCommand>,
    pub(crate) output_tx: broadcast::Sender<RuntimeEvent>,
    pub(crate) replay: ReplayBuffer,
    pub(crate) attachment_generation: u64,
    pub(crate) attached_generation: Option<u64>,
    pub(crate) last_rows: u16,
    pub(crate) last_cols: u16,
}

impl SessionRuntime {
    pub(crate) fn live(rows: u16, cols: u16) -> (Self, mpsc::UnboundedReceiver<RuntimeCommand>) {
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        let (output_tx, _) = broadcast::channel(256);
        (
            Self {
                control_tx,
                output_tx,
                replay: ReplayBuffer::new(DEFAULT_REPLAY_CAPACITY),
                attachment_generation: 0,
                attached_generation: None,
                last_rows: rows,
                last_cols: cols,
            },
            control_rx,
        )
    }

    pub(crate) fn detached(rows: u16, cols: u16) -> Self {
        let (control_tx, _control_rx) = mpsc::unbounded_channel();
        let (output_tx, _) = broadcast::channel(256);
        Self {
            control_tx,
            output_tx,
            replay: ReplayBuffer::new(DEFAULT_REPLAY_CAPACITY),
            attachment_generation: 0,
            attached_generation: None,
            last_rows: rows,
            last_cols: cols,
        }
    }
```

Replace with:

```rust
#[derive(Debug)]
pub(crate) struct SessionRuntime {
    pub(crate) control_tx: mpsc::Sender<RuntimeCommand>,
    pub(crate) output_tx: broadcast::Sender<RuntimeEvent>,
    pub(crate) replay: ReplayBuffer,
    pub(crate) attachment_generation: u64,
    pub(crate) attached_generation: Option<u64>,
    pub(crate) last_rows: u16,
    pub(crate) last_cols: u16,
}

impl SessionRuntime {
    pub(crate) fn live(rows: u16, cols: u16) -> (Self, mpsc::Receiver<RuntimeCommand>) {
        let (control_tx, control_rx) = mpsc::channel(CONTROL_CHANNEL_CAPACITY);
        let (output_tx, _) = broadcast::channel(256);
        (
            Self {
                control_tx,
                output_tx,
                replay: ReplayBuffer::new(DEFAULT_REPLAY_CAPACITY),
                attachment_generation: 0,
                attached_generation: None,
                last_rows: rows,
                last_cols: cols,
            },
            control_rx,
        )
    }

    pub(crate) fn detached(rows: u16, cols: u16) -> Self {
        let (control_tx, _control_rx) = mpsc::channel(CONTROL_CHANNEL_CAPACITY);
        let (output_tx, _) = broadcast::channel(256);
        Self {
            control_tx,
            output_tx,
            replay: ReplayBuffer::new(DEFAULT_REPLAY_CAPACITY),
            attachment_generation: 0,
            attached_generation: None,
            last_rows: rows,
            last_cols: cols,
        }
    }
```

- [ ] **Step 5: Make the send functions async**

In the same file, find:

```rust
pub(crate) fn send_runtime_command(
    state: &Arc<AppState>,
    id: &str,
    command: RuntimeCommand,
) -> Result<(), ()> {
    let tx = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(id)
            .map(|handle| handle.runtime.control_tx.clone())
    }
    .ok_or(())?;
    tx.send(command).map_err(|_| ())
}

pub(crate) fn update_runtime_size(
    state: &Arc<AppState>,
    id: &str,
    rows: u16,
    cols: u16,
) -> Result<(), ()> {
    let tx = {
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(id).ok_or(())?;
        handle.runtime.last_rows = rows;
        handle.runtime.last_cols = cols;
        handle.runtime.control_tx.clone()
    };
    tx.send(RuntimeCommand::Resize { rows, cols })
        .map_err(|_| ())
}
```

Replace with:

```rust
pub(crate) async fn send_runtime_command(
    state: &Arc<AppState>,
    id: &str,
    command: RuntimeCommand,
) -> Result<(), ()> {
    let tx = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(id)
            .map(|handle| handle.runtime.control_tx.clone())
    }
    .ok_or(())?;
    tx.send(command).await.map_err(|_| ())
}

pub(crate) async fn update_runtime_size(
    state: &Arc<AppState>,
    id: &str,
    rows: u16,
    cols: u16,
) -> Result<(), ()> {
    let tx = {
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(id).ok_or(())?;
        handle.runtime.last_rows = rows;
        handle.runtime.last_cols = cols;
        handle.runtime.control_tx.clone()
    };
    tx.send(RuntimeCommand::Resize { rows, cols })
        .await
        .map_err(|_| ())
}
```

- [ ] **Step 6: Update the receiver type in `capture_startup_runtime_state` and `start_session_runtime`**

In the same file, find:

```rust
async fn capture_startup_runtime_state(
    control_rx: &mut mpsc::UnboundedReceiver<RuntimeCommand>,
    mut initial_size: PtySize,
) -> (PtySize, Vec<RuntimeCommand>) {
```

Replace with:

```rust
async fn capture_startup_runtime_state(
    control_rx: &mut mpsc::Receiver<RuntimeCommand>,
    mut initial_size: PtySize,
) -> (PtySize, Vec<RuntimeCommand>) {
```

Then find:

```rust
pub(crate) async fn start_session_runtime(
    state: Arc<AppState>,
    id: String,
    command: harness::CommandSpec,
    initial_prompt: Option<String>,
    mut control_rx: mpsc::UnboundedReceiver<RuntimeCommand>,
    output_tx: broadcast::Sender<RuntimeEvent>,
    mut kill_rx: tokio::sync::watch::Receiver<bool>,
    initial_size: PtySize,
) -> Result<(), String> {
```

Replace with:

```rust
pub(crate) async fn start_session_runtime(
    state: Arc<AppState>,
    id: String,
    command: harness::CommandSpec,
    initial_prompt: Option<String>,
    mut control_rx: mpsc::Receiver<RuntimeCommand>,
    output_tx: broadcast::Sender<RuntimeEvent>,
    mut kill_rx: tokio::sync::watch::Receiver<bool>,
    initial_size: PtySize,
) -> Result<(), String> {
```

- [ ] **Step 7: Await the call sites in `terminal_runtime.rs`**

In `crates/orkworksd/src/runtime/terminal_runtime.rs`, find:

```rust
                                if crate::runtime::session_runtime::send_runtime_command(
                                    &state,
                                    &id,
                                    crate::runtime::session_runtime::RuntimeCommand::Input(data),
                                ).is_err() {
                                    break;
                                }
                            }
                            TerminalAction::Resize { rows, cols } => {
                                if crate::runtime::session_runtime::update_runtime_size(&state, &id, rows, cols).is_err() {
                                    break;
                                }
                            }
                            TerminalAction::Kill => {
                                if crate::runtime::session_runtime::send_runtime_command(
                                    &state,
                                    &id,
                                    crate::runtime::session_runtime::RuntimeCommand::Kill,
                                ).is_err() {
                                    break;
                                }
                            }
```

Replace with:

```rust
                                if crate::runtime::session_runtime::send_runtime_command(
                                    &state,
                                    &id,
                                    crate::runtime::session_runtime::RuntimeCommand::Input(data),
                                ).await.is_err() {
                                    break;
                                }
                            }
                            TerminalAction::Resize { rows, cols } => {
                                if crate::runtime::session_runtime::update_runtime_size(&state, &id, rows, cols).await.is_err() {
                                    break;
                                }
                            }
                            TerminalAction::Kill => {
                                if crate::runtime::session_runtime::send_runtime_command(
                                    &state,
                                    &id,
                                    crate::runtime::session_runtime::RuntimeCommand::Kill,
                                ).await.is_err() {
                                    break;
                                }
                            }
```

- [ ] **Step 8: Await the direct `control_tx.send()` call in the existing resize test**

`early_resize_after_start_sets_initial_pty_size_before_spawn` (in the same `mod tests` block) sends directly on a cloned `control_tx` without going through `send_runtime_command`, so it needs its own `.await`. Find:

```rust
        tokio::time::sleep(Duration::from_millis(100)).await;
        control_tx
            .send(RuntimeCommand::Resize {
                rows: 40,
                cols: 120,
            })
            .unwrap();
```

Replace with:

```rust
        tokio::time::sleep(Duration::from_millis(100)).await;
        control_tx
            .send(RuntimeCommand::Resize {
                rows: 40,
                cols: 120,
            })
            .await
            .unwrap();
```

- [ ] **Step 9: Run the new test to confirm it passes**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml send_runtime_command_blocks_until_capacity_available_then_succeeds`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 10: Run the full Rust test suite to check for regressions**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all 283 tests pass (282 existing + 1 new from Step 1), no failures, no compile errors. Steps 5-8 above already cover every production and test call site that needed `.await` added (confirmed by grepping the crate for `control_tx`, `send_runtime_command`, and `update_runtime_size` before writing this plan), so a clean pass here confirms nothing was missed rather than surfacing new breakage to fix ad hoc.

- [ ] **Step 11: Run clippy to check for new warnings**

Run: `cargo clippy --manifest-path crates/orkworksd/Cargo.toml --all-targets`
Expected: no new warnings introduced by this change (pre-existing warnings in unrelated files are fine).

- [ ] **Step 12: Commit**

```bash
git add crates/orkworksd/src/runtime/session_runtime.rs crates/orkworksd/src/runtime/terminal_runtime.rs
git commit -m "fix(sidecar): bound the terminal RuntimeCommand control channel

Replace the unbounded mpsc channel carrying Input/Resize/Kill commands
to the PTY writer with a bounded one, and make the two send functions
async so a full queue applies real backpressure (the affected
session's own websocket read loop waits for room) instead of growing
memory without limit if the PTY writer falls behind. Part of #159."
```

---

### Task 2: Add the `appendPendingInput` pure helper (frontend)

**Files:**
- Modify: `apps/desktop/src/terminalProtocol.ts`
- Test: `apps/desktop/tests/terminalProtocol.test.ts`

- [ ] **Step 1: Write the failing tests**

In `apps/desktop/tests/terminalProtocol.test.ts`, add this import to the existing import block:

```ts
import {
  parseTerminalControlMessage,
  shouldReplayTerminalOutputOnClose,
  appendPendingInput,
} from "../src/terminalProtocol.ts";
```

Then add these tests at the end of the file:

```ts
test("appendPendingInput appends while under the cap", () => {
  assert.deepEqual(appendPendingInput("abc", "def", 10), {
    next: "abcdef",
    dropped: false,
  });
});

test("appendPendingInput accepts a chunk that exactly fills the cap", () => {
  assert.deepEqual(appendPendingInput("abcde", "fghij", 10), {
    next: "abcdefghij",
    dropped: false,
  });
});

test("appendPendingInput drops the incoming chunk once it would exceed the cap", () => {
  assert.deepEqual(appendPendingInput("abcdefghij", "k", 10), {
    next: "abcdefghij",
    dropped: true,
  });
});

test("appendPendingInput keeps reporting dropped on repeated overflow without growing", () => {
  const first = appendPendingInput("abcdefghij", "k", 10);
  const second = appendPendingInput(first.next, "lmno", 10);
  assert.deepEqual(second, { next: "abcdefghij", dropped: true });
});
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/terminalProtocol.test.ts`
Expected: FAIL — `appendPendingInput` is not exported from `terminalProtocol.ts` yet (TypeError or "is not a function").

- [ ] **Step 3: Implement `appendPendingInput`**

In `apps/desktop/src/terminalProtocol.ts`, add this function at the end of the file:

```ts
export function appendPendingInput(
  current: string,
  incoming: string,
  maxLength: number,
): { next: string; dropped: boolean } {
  if (current.length + incoming.length > maxLength) {
    return { next: current, dropped: true };
  }
  return { next: current + incoming, dropped: false };
}
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/terminalProtocol.test.ts`
Expected: all tests in the file pass, including the 4 new ones.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/terminalProtocol.ts apps/desktop/tests/terminalProtocol.test.ts
git commit -m "feat(desktop): add appendPendingInput helper for bounded pending-input buffering

Pure, unit-tested helper that caps a growing string at maxLength and
reports whether the incoming chunk was dropped, used by terminalStore
to bound pendingInput while the terminal WebSocket is disconnected.
Part of #159."
```

---

### Task 3: Wire the cap into `terminalStore.ts`

**Files:**
- Modify: `apps/desktop/src/terminalStore.ts`

- [ ] **Step 1: Import the helper and add the cap constant**

In `apps/desktop/src/terminalStore.ts`, find:

```ts
import {
  parseTerminalControlMessage,
  shouldReplayTerminalOutputOnClose,
} from "./terminalProtocol";
```

Replace with:

```ts
import {
  parseTerminalControlMessage,
  shouldReplayTerminalOutputOnClose,
  appendPendingInput,
} from "./terminalProtocol";

const MAX_PENDING_INPUT_LENGTH = 64 * 1024;
```

- [ ] **Step 2: Add `pendingInputOverflowed` to `TerminalHandle`**

Find:

```ts
export interface TerminalHandle {
  id: string;
  terminal: Terminal;
  ws: WebSocket;
  fitAddon: FitAddon;
  wrapper: HTMLDivElement;
  ended: boolean;
  disposed: boolean;
  pendingInput: string;
  resizeObserver: ResizeObserver;
}
```

Replace with:

```ts
export interface TerminalHandle {
  id: string;
  terminal: Terminal;
  ws: WebSocket;
  fitAddon: FitAddon;
  wrapper: HTMLDivElement;
  ended: boolean;
  disposed: boolean;
  pendingInput: string;
  pendingInputOverflowed: boolean;
  resizeObserver: ResizeObserver;
}
```

- [ ] **Step 3: Initialize the new field**

Find:

```ts
  const handle: TerminalHandle = {
    id,
    terminal: term,
    ws,
    fitAddon,
    wrapper,
    ended: false,
    disposed: false,
    pendingInput: "",
    resizeObserver,
  };
```

Replace with:

```ts
  const handle: TerminalHandle = {
    id,
    terminal: term,
    ws,
    fitAddon,
    wrapper,
    ended: false,
    disposed: false,
    pendingInput: "",
    pendingInputOverflowed: false,
    resizeObserver,
  };
```

- [ ] **Step 4: Reset the overflow flag when pendingInput is flushed on reconnect**

Find:

```ts
  ws.onopen = () => {
    try {
      fitAddon.fit();
    } catch {
      /* ignore */
    }
    sendResize(ws, term);
    if (handle.pendingInput) {
      ws.send(JSON.stringify({ type: "input", data: handle.pendingInput }));
      handle.pendingInput = "";
    }
  };
```

Replace with:

```ts
  ws.onopen = () => {
    try {
      fitAddon.fit();
    } catch {
      /* ignore */
    }
    sendResize(ws, term);
    if (handle.pendingInput) {
      ws.send(JSON.stringify({ type: "input", data: handle.pendingInput }));
      handle.pendingInput = "";
    }
    handle.pendingInputOverflowed = false;
  };
```

- [ ] **Step 5: Cap `pendingInput` growth and warn once per overflow**

Find:

```ts
  term.onData((data) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "input", data }));
    } else {
      handle.pendingInput += data;
    }
  });
```

Replace with:

```ts
  term.onData((data) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "input", data }));
      return;
    }
    const { next, dropped } = appendPendingInput(
      handle.pendingInput,
      data,
      MAX_PENDING_INPUT_LENGTH,
    );
    handle.pendingInput = next;
    if (dropped && !handle.pendingInputOverflowed) {
      handle.pendingInputOverflowed = true;
      term.writeln(
        "\r\n[input buffer full while disconnected — further keystrokes are being dropped until reconnect]",
      );
    }
  });
```

- [ ] **Step 6: Type-check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 7: Run the frontend test suite**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: all tests pass, no regressions.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/src/terminalStore.ts
git commit -m "fix(desktop): cap pendingInput while the terminal transport is disconnected

Bound the buffer that accumulates keystrokes while the terminal
WebSocket is not open at 64 KiB via appendPendingInput. Once full,
further input is dropped and a one-time warning line is written into
the terminal pane per disconnect, so the loss is visible rather than
silent. Fixes #159."
```

---

### Task 4: Full verification and PR

**Files:** none (verification only)

- [ ] **Step 1: Run the full Rust suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all tests pass.

- [ ] **Step 2: Run the full frontend suite and type-check**

Run:
```bash
cd apps/desktop && npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```
Expected: no type errors, all tests pass.

- [ ] **Step 3: Run the doc-currency check**

Run: `bash .claude/hooks/doc-check.sh`
Expected: no flagged files, or address any that are flagged.

- [ ] **Step 4: Push the branch and open a PR**

```bash
git push -u origin terminal-input-backpressure
gh pr create --repo Rambolarsen/orkworks \
  --title "fix(terminal): add input backpressure while transport/PTY is unavailable" \
  --body "Fixes #159. See docs/superpowers/specs/2026-07-14-terminal-input-backpressure-design.md for the design."
```

- [ ] **Step 5: Run the required code review before merge**

Per this repo's `AGENTS.md`, PRs touching `apps/desktop/` or `crates/orkworksd/` need a `/code-review` pass before merge. This PR touches both a runtime/channel change (sidecar) and a small, well-isolated frontend buffering change — use at least medium effort given the channel/async-signature change is a concurrency-adjacent change per `AGENTS.md`'s escalation guidance.
