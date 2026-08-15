use crate::runtime::observed_status::{
    apply_process_transition_to_handle, apply_process_transition_to_meta,
    process_transition_fields, ProcessTransition,
};
use crate::runtime::terminal_runtime::{
    make_pty_system, schedule_session_ending_finalization, session_env_overrides,
    set_session_status_for_generation, should_forward_terminal_env, terminal_env_overrides,
};
#[cfg(windows)]
use crate::runtime::terminal_runtime::resolve_windows_program;
use crate::{harness, metadata, peon, plan_handoff, AppState};
use chrono::{DateTime, Utc};
use portable_pty::{CommandBuilder, PtySize, PtySystem};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};

pub(crate) const DEFAULT_TERMINAL_ROWS: u16 = 24;
pub(crate) const DEFAULT_TERMINAL_COLS: u16 = 80;
const DEFAULT_REPLAY_CAPACITY: usize = 256;
const DRIVER_EVENT_BUFFER_CAPACITY: usize = 64;
const PERSIST_QUEUE_CAPACITY: usize = 64;
const CONTROL_CHANNEL_CAPACITY: usize = 64;
pub(crate) const STARTUP_PENDING_INPUT_BYTES: usize = 64 * 1024;
const MAX_PARTIAL_PERSIST_BYTES: usize = 64 * 1024;
const INITIAL_RESIZE_GRACE: std::time::Duration = std::time::Duration::from_millis(150);
const STARTUP_ATTENTION_GRACE: std::time::Duration = std::time::Duration::from_secs(2);
const WORK_SIGNAL_WINDOW: std::time::Duration = std::time::Duration::from_secs(10);
const OUTPUT_RECENCY_PERSIST_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

pub(crate) type RuntimeGeneration = u64;

static NEXT_RUNTIME_GENERATION: AtomicU64 = AtomicU64::new(1);

fn next_runtime_generation() -> RuntimeGeneration {
    NEXT_RUNTIME_GENERATION
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |generation| {
            generation.checked_add(1)
        })
        .expect("session runtime generation exhausted")
}

#[derive(Debug)]
pub(crate) struct PendingWorkSignal {
    remaining_echo: String,
    expires_at: tokio::time::Instant,
}

pub(crate) fn arm_pending_work_signal(
    submitted_line: &str,
    now: tokio::time::Instant,
) -> PendingWorkSignal {
    PendingWorkSignal {
        remaining_echo: submitted_line.to_string(),
        expires_at: now + WORK_SIGNAL_WINDOW,
    }
}

/// Extends an already-armed signal with newly typed (not yet echoed) text
/// and refreshes its expiry, rather than discarding whatever remains of a
/// prior keystroke's expected echo. A naive re-arm-per-keystroke that
/// replaces `remaining_echo` with the full composed-so-far buffer mismatches
/// how PTYs actually echo: each frame only echoes back the character(s)
/// just typed, not the accumulated line again every time. Appending keeps
/// `remaining_echo` an accurate expectation of the *next* echo chunk in
/// order, however much of a previous keystroke's echo has already been
/// absorbed.
pub(crate) fn extend_pending_work_signal(
    slot: &mut Option<PendingWorkSignal>,
    new_text: &str,
    now: tokio::time::Instant,
) {
    match slot {
        Some(signal) => {
            signal.remaining_echo.push_str(new_text);
            signal.expires_at = now + WORK_SIGNAL_WINDOW;
        }
        None => *slot = Some(arm_pending_work_signal(new_text, now)),
    }
}

/// A chunk only "counts" as visible output if it has at least one character
/// that isn't whitespace and isn't a control code (e.g. a bare BEL or other
/// C0 byte left over after ANSI stripping must not qualify as model output).
fn has_visible_character(s: &str) -> bool {
    s.chars().any(|c| !c.is_whitespace() && !c.is_control())
}

/// Consumes `output` against the armed signal in `slot`. The signal is cleared
/// entirely once it expires or once it qualifies as genuine (non-echo) visible
/// output; a non-qualifying chunk before either of those only trims the
/// remaining echo, so a spent or expired signal is never rechecked forever,
/// while a signal still inside its window stays armed for later output.
pub(crate) fn consume_pending_work_signal(
    slot: &mut Option<PendingWorkSignal>,
    output: &str,
    now: tokio::time::Instant,
) -> bool {
    let Some(signal) = slot.as_mut() else {
        return false;
    };
    if now >= signal.expires_at {
        *slot = None;
        return false;
    }

    let output = peon::strip_ansi(output);
    if !has_visible_character(&output) {
        return false;
    }
    let output = output
        .strip_prefix('\r')
        .or_else(|| output.strip_prefix('\n'))
        .unwrap_or(&output);
    if signal.remaining_echo.starts_with(output) {
        signal.remaining_echo.drain(..output.len());
        return false;
    }

    let visible_output = output
        .strip_prefix(&signal.remaining_echo)
        .unwrap_or(output)
        .trim();
    signal.remaining_echo.clear();
    let qualifies = has_visible_character(visible_output);
    if qualifies {
        *slot = None;
    }
    qualifies
}
/// Only a qualifying, recently submitted hookless user command can resume a
/// session to `working`; PTY output alone never changes the observed work state.
fn should_infer_working(
    lifecycle: &str,
    has_qualifying_work_signal: bool,
    active_work_hook: bool,
    startup_grace_ends_at: tokio::time::Instant,
) -> bool {
    lifecycle == "alive"
        && has_qualifying_work_signal
        && !active_work_hook
        && tokio::time::Instant::now() >= startup_grace_ends_at
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RuntimeEvent {
    Output { cursor: u64, chunk: Vec<u8> },
    Ended { status: String },
    Error { code: String, message: String },
}

#[derive(Debug)]
pub(crate) enum RuntimeCommand {
    Input {
        data: String,
        accepted: Option<tokio::sync::oneshot::Sender<Result<(), ()>>>,
    },
    Resize {
        rows: u16,
        cols: u16,
    },
    Kill,
}

#[derive(Debug)]
pub(crate) struct AttachmentClaim {
    pub(crate) generation: u64,
    pub(crate) replay_from: u64,
    pub(crate) replay_to: u64,
    pub(crate) replay_chunks: Vec<(u64, Vec<u8>)>,
    pub(crate) events: broadcast::Receiver<RuntimeEvent>,
}

#[derive(Debug)]
pub(crate) struct ReplayBuffer {
    capacity: usize,
    next_cursor: u64,
    chunks: VecDeque<(u64, Vec<u8>)>,
}

impl ReplayBuffer {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            next_cursor: 0,
            chunks: VecDeque::new(),
        }
    }

    pub(crate) fn push(&mut self, chunk: Vec<u8>) -> u64 {
        let cursor = self.next_cursor;
        self.next_cursor += 1;
        self.chunks.push_back((cursor, chunk));
        while self.chunks.len() > self.capacity {
            self.chunks.pop_front();
        }
        cursor
    }

    pub(crate) fn next_cursor(&self) -> u64 {
        self.next_cursor
    }

    pub(crate) fn start_cursor(&self) -> u64 {
        self.chunks
            .front()
            .map(|(cursor, _)| *cursor)
            .unwrap_or(self.next_cursor)
    }

    pub(crate) fn snapshot(&self) -> Vec<(u64, Vec<u8>)> {
        self.chunks.iter().cloned().collect()
    }
}

#[derive(Debug)]
pub(crate) struct SessionRuntime {
    run_generation: RuntimeGeneration,
    startup_spawned: bool,
    pub(crate) control_tx: mpsc::Sender<RuntimeCommand>,
    pub(crate) output_tx: broadcast::Sender<RuntimeEvent>,
    pub(crate) replay: ReplayBuffer,
    pub(crate) attachment_generation: u64,
    pub(crate) attached_generation: Option<u64>,
    pub(crate) last_rows: u16,
    pub(crate) last_cols: u16,
    pub(crate) input_generation: u64,
    pub(crate) accepted_input_at: Option<DateTime<Utc>>,
    pub(crate) last_hook_attention_at: Option<DateTime<Utc>>,
    pub(crate) usage_limit_latched_at: Option<DateTime<Utc>>,
    pub(crate) peon_output_revision: u64,
    pub(crate) min_peon_output_revision: u64,
    last_output_persisted_at: Option<tokio::time::Instant>,
    pending_output_at: Option<String>,
    output_flush_scheduled: bool,
}

impl SessionRuntime {
    pub(crate) fn live(rows: u16, cols: u16) -> (Self, mpsc::Receiver<RuntimeCommand>) {
        let (control_tx, control_rx) = mpsc::channel(CONTROL_CHANNEL_CAPACITY);
        let (output_tx, _) = broadcast::channel(256);
        (
            Self {
                run_generation: next_runtime_generation(),
                startup_spawned: false,
                control_tx,
                output_tx,
                replay: ReplayBuffer::new(DEFAULT_REPLAY_CAPACITY),
                attachment_generation: 0,
                attached_generation: None,
                last_rows: rows,
                last_cols: cols,
                input_generation: 0,
                accepted_input_at: None,
                last_hook_attention_at: None,
                usage_limit_latched_at: None,
                peon_output_revision: 0,
                min_peon_output_revision: 0,
                last_output_persisted_at: None,
                pending_output_at: None,
                output_flush_scheduled: false,
            },
            control_rx,
        )
    }

    #[cfg(test)]
    pub(crate) fn detached(rows: u16, cols: u16) -> Self {
        let (control_tx, _control_rx) = mpsc::channel(CONTROL_CHANNEL_CAPACITY);
        let (output_tx, _) = broadcast::channel(256);
        Self {
            run_generation: next_runtime_generation(),
            startup_spawned: false,
            control_tx,
            output_tx,
            replay: ReplayBuffer::new(DEFAULT_REPLAY_CAPACITY),
            attachment_generation: 0,
            attached_generation: None,
            last_rows: rows,
            last_cols: cols,
            input_generation: 0,
            accepted_input_at: None,
            last_hook_attention_at: None,
            usage_limit_latched_at: None,
            peon_output_revision: 0,
            min_peon_output_revision: 0,
            last_output_persisted_at: None,
            pending_output_at: None,
            output_flush_scheduled: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn detached_test() -> Self {
        Self::detached(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS)
    }

    pub(crate) fn run_generation(&self) -> RuntimeGeneration {
        self.run_generation
    }

    pub(crate) fn mark_startup_spawned(&mut self) {
        self.startup_spawned = true;
    }

    pub(crate) fn startup_spawned(&self) -> bool {
        self.startup_spawned
    }

    #[cfg(test)]
    pub(crate) fn attached_generation(&self) -> Option<u64> {
        self.attached_generation
    }

    #[cfg(test)]
    pub(crate) fn last_size(&self) -> (u16, u16) {
        (self.last_rows, self.last_cols)
    }

    fn record_output_recency(
        &mut self,
        timestamp: String,
        now: tokio::time::Instant,
    ) -> Option<String> {
        self.pending_output_at = Some(timestamp);
        let due = self
            .last_output_persisted_at
            .map(|previous| now.duration_since(previous) >= OUTPUT_RECENCY_PERSIST_INTERVAL)
            .unwrap_or(true);
        due.then(|| {
            self.last_output_persisted_at = Some(now);
            self.pending_output_at.take().expect("pending output timestamp set above")
        })
    }

    fn flush_output_recency(&mut self) -> Option<String> {
        self.output_flush_scheduled = false;
        self.pending_output_at.take()
    }

    fn schedule_output_recency_flush(&mut self, now: tokio::time::Instant) -> Option<std::time::Duration> {
        if self.pending_output_at.is_none() || self.output_flush_scheduled {
            return None;
        }
        self.output_flush_scheduled = true;
        let due_at = self.last_output_persisted_at.expect("pending output has a prior persistence")
            + OUTPUT_RECENCY_PERSIST_INTERVAL;
        Some(due_at.saturating_duration_since(now))
    }
}

enum DriverEvent {
    Output(Vec<u8>),
    Exited,
    WaitError(String),
}

fn output_recency_timestamp(data: &[u8], timestamp: String) -> Option<String> {
    (!data.is_empty()).then_some(timestamp)
}

fn should_persist_output_recency(existing: Option<&str>, incoming: &str) -> bool {
    let Ok(incoming) = DateTime::parse_from_rfc3339(incoming) else {
        return false;
    };
    let Some(existing) = existing else {
        return true;
    };
    let Ok(existing) = DateTime::parse_from_rfc3339(existing) else {
        return true;
    };
    incoming >= existing
}

fn persist_output_recency(state: &Arc<AppState>, id: &str, timestamp: String) {
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some(mut meta) = ws.metadata.read_session(id) {
            if should_persist_output_recency(meta.last_output_at.as_deref(), &timestamp) {
                meta.last_output_at = Some(timestamp);
                ws.metadata.write_session(&meta);
            }
        }
    }
}

async fn flush_output_recency(state: &Arc<AppState>, id: &str) {
    let timestamp = state
        .sessions
        .lock()
        .unwrap()
        .get_mut(id)
        .and_then(|handle| handle.runtime.flush_output_recency());
    if let Some(timestamp) = timestamp {
        let state = state.clone();
        let id = id.to_string();
        let _ = tokio::task::spawn_blocking(move || persist_output_recency(&state, &id, timestamp)).await;
    }
}

fn make_driver_event_channel() -> (mpsc::Sender<DriverEvent>, mpsc::Receiver<DriverEvent>) {
    mpsc::channel(DRIVER_EVENT_BUFFER_CAPACITY)
}

fn make_persist_channel() -> (
    mpsc::Sender<Vec<crate::metadata::TerminalOutputRecord>>,
    mpsc::Receiver<Vec<crate::metadata::TerminalOutputRecord>>,
) {
    mpsc::channel(PERSIST_QUEUE_CAPACITY)
}

fn partial_persist_flush_end(buffer: &[u8]) -> usize {
    let mut first_continuation = MAX_PARTIAL_PERSIST_BYTES;
    while first_continuation > MAX_PARTIAL_PERSIST_BYTES.saturating_sub(3)
        && buffer[first_continuation - 1] & 0b1100_0000 == 0b1000_0000
    {
        first_continuation -= 1;
    }

    let lead = first_continuation - 1;
    let expected_len = match buffer[lead] {
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => 1,
    };
    if expected_len > MAX_PARTIAL_PERSIST_BYTES - lead {
        lead
    } else {
        MAX_PARTIAL_PERSIST_BYTES
    }
}

fn drain_persist_records(buffer: &mut Vec<u8>) -> Vec<crate::metadata::TerminalOutputRecord> {
    let mut records = Vec::new();

    while let Some(nl) = buffer.iter().position(|&byte| byte == b'\n') {
        let line: Vec<u8> = buffer.drain(..=nl).collect();
        let (end, delimiter) = if line.ends_with(b"\r\n") {
            (line.len() - 2, "\r\n")
        } else {
            (line.len() - 1, "\n")
        };
        records.push(crate::metadata::TerminalOutputRecord::raw(
            String::from_utf8_lossy(&line[..end]).into_owned(),
            delimiter,
        ));
    }

    while buffer.len() > MAX_PARTIAL_PERSIST_BYTES {
        let flush_end = partial_persist_flush_end(buffer);
        records.push(crate::metadata::TerminalOutputRecord::raw(
            String::from_utf8_lossy(&buffer[..flush_end]).into_owned(),
            "",
        ));
        buffer.drain(..flush_end);
    }

    records
}

pub(crate) fn claim_attachment(state: &Arc<AppState>, id: &str) -> Option<AttachmentClaim> {
    let mut sessions = state.sessions.lock().unwrap();
    let handle = sessions.get_mut(id)?;
    let status = handle.info.status.as_str();
    let lifecycle_phase = handle.info.lifecycle_phase.as_str();
    if matches!(status, "killed" | "ended" | "error")
        || matches!(lifecycle_phase, "ending" | "ended")
        || handle.runtime.attached_generation.is_some()
    {
        return None;
    }
    handle.runtime.attachment_generation += 1;
    let generation = handle.runtime.attachment_generation;
    handle.runtime.attached_generation = Some(generation);
    handle.terminal_attached = true;
    let events = handle.runtime.output_tx.subscribe();
    let replay_from = handle.runtime.replay.start_cursor();
    let replay_to = handle.runtime.replay.next_cursor();
    let replay_chunks = handle.runtime.replay.snapshot();
    Some(AttachmentClaim {
        generation,
        replay_from,
        replay_to,
        replay_chunks,
        events,
    })
}

pub(crate) fn release_attachment(state: &Arc<AppState>, id: &str, generation: u64) {
    let mut sessions = state.sessions.lock().unwrap();
    let Some(handle) = sessions.get_mut(id) else {
        return;
    };
    if handle.runtime.attached_generation == Some(generation) {
        handle.runtime.attached_generation = None;
        handle.terminal_attached = false;
    }
}

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

pub(crate) async fn send_runtime_input(
    state: &Arc<AppState>,
    id: &str,
    data: String,
) -> Result<(), ()> {
    let tx = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(id)
            .map(|handle| handle.runtime.control_tx.clone())
    }
    .ok_or(())?;
    let (accepted_tx, accepted_rx) = tokio::sync::oneshot::channel();
    tx.send(RuntimeCommand::Input {
        data,
        accepted: Some(accepted_tx),
    })
    .await
    .map_err(|_| ())?;
    accepted_rx.await.map_err(|_| ())?
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

async fn capture_startup_runtime_state(
    control_rx: &mut mpsc::Receiver<RuntimeCommand>,
    mut initial_size: PtySize,
) -> (PtySize, Vec<RuntimeCommand>) {
    let mut pending_commands = Vec::new();
    let mut pending_input_bytes: usize = 0;
    let deadline = tokio::time::Instant::now() + INITIAL_RESIZE_GRACE;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        match tokio::time::timeout(remaining, control_rx.recv()).await {
            Ok(Some(RuntimeCommand::Resize { rows, cols })) => {
                initial_size.rows = rows;
                initial_size.cols = cols;
                break;
            }
            Ok(Some(command)) => {
                if pending_commands.len() >= CONTROL_CHANNEL_CAPACITY {
                    if let RuntimeCommand::Input {
                        accepted: Some(accepted),
                        ..
                    } = command
                    {
                        let _ = accepted.send(Err(()));
                    }
                    continue;
                }
                if let RuntimeCommand::Input { data, .. } = &command {
                    let Some(next_input_bytes) = pending_input_bytes.checked_add(data.len()) else {
                        if let RuntimeCommand::Input {
                            accepted: Some(accepted),
                            ..
                        } = command
                        {
                            let _ = accepted.send(Err(()));
                        }
                        continue;
                    };
                    if next_input_bytes > STARTUP_PENDING_INPUT_BYTES {
                        if let RuntimeCommand::Input {
                            accepted: Some(accepted),
                            ..
                        } = command
                        {
                            let _ = accepted.send(Err(()));
                        }
                        continue;
                    }
                    pending_input_bytes = next_input_bytes;
                }
                pending_commands.push(command);
            }
            Ok(None) | Err(_) => break,
        }
    }

    (initial_size, pending_commands)
}

/// Clears per-session in-memory side tables once a session's PTY process is
/// gone (naturally exited, wait-errored, or failed to finish setup). The pid
/// removal in particular matters: once the process is gone its pid could be
/// reused by an unrelated OS process, so a stale entry left behind risks a
/// future live-cwd probe (issue #241) attributing a stranger's cwd to this
/// session.
pub(crate) fn clear_ended_session_tracking(state: &AppState, id: &str) {
    state.peon.last_output.write().unwrap().remove(id);
    state.peon.last_inference.write().unwrap().remove(id);
    state.peon.input_buf.write().unwrap().remove(id);
    state.peon.reported_cwd.write().unwrap().remove(id);
    state.session_pids.lock().unwrap().remove(id);
}

/// Applies an exit callback only while its runtime generation still owns the
/// session ID. Marking the handle as ending first prevents resume admission
/// from replacing it while the remaining runtime-owned side tables are
/// cleared and output recency is flushed.
pub(crate) async fn handle_runtime_exit(
    state: &Arc<AppState>,
    id: &str,
    generation: RuntimeGeneration,
    status: &str,
) -> bool {
    let owns_generation = state
        .sessions
        .lock()
        .unwrap()
        .get(id)
        .is_some_and(|handle| handle.runtime.run_generation() == generation);
    if !owns_generation {
        return false;
    }

    // A user-initiated kill may already have moved this same generation to
    // `ending`. The driver still owns cleanup and finalization in that case;
    // only a generation mismatch makes the callback stale.
    let _ = set_session_status_for_generation(state, id, generation, status);
    {
        let mut sessions = state.sessions.lock().unwrap();
        let Some(handle) = sessions
            .get_mut(id)
            .filter(|handle| {
                handle.runtime.run_generation() == generation
                    && handle.info.lifecycle_phase == "ending"
            })
        else {
            return false;
        };
        handle.runtime.attached_generation = None;
        handle.terminal_attached = false;
    }
    clear_ended_session_tracking(state, id);
    flush_output_recency(state, id).await;
    schedule_session_ending_finalization(
        state.clone(),
        id.to_string(),
        generation,
        status.to_string(),
    );
    true
}

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
    let run_generation = state
        .sessions
        .lock()
        .unwrap()
        .get(&id)
        .map(|handle| handle.runtime.run_generation())
        .ok_or_else(|| "session runtime handle is not installed".to_string())?;
    let (initial_size, pending_commands) =
        capture_startup_runtime_state(&mut control_rx, initial_size).await;
    let pty_sys = make_pty_system();
    let pair = pty_sys.openpty(initial_size).map_err(|e| e.to_string())?;

    #[cfg(windows)]
    let program = {
        let raw = command.program.clone();
        tokio::task::spawn_blocking(move || resolve_windows_program(&raw))
            .await
            .unwrap_or_else(|_| command.program.clone())
    };
    #[cfg(not(windows))]
    let program = command.program.clone();

    let mut cmd = CommandBuilder::new(&program);
    cmd.args(&command.args);
    cmd.cwd(&command.cwd);
    for (key, value) in std::env::vars() {
        if should_forward_terminal_env(&key) {
            cmd.env(&key, &value);
        } else {
            cmd.env_remove(&key);
        }
    }
    for (key, value) in terminal_env_overrides() {
        cmd.env(&key, &value);
    }
    let port = match state.bound_port.load(std::sync::atomic::Ordering::Relaxed) {
        0 => None,
        value => Some(value),
    };
    for (key, value) in session_env_overrides(&id, port) {
        cmd.env(&key, &value);
    }

    let mut child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    let owns_spawned_generation = state
        .sessions
        .lock()
        .unwrap()
        .get_mut(&id)
        .filter(|handle| handle.runtime.run_generation() == run_generation)
        .map(|handle| handle.runtime.mark_startup_spawned())
        .is_some();
    if !owns_spawned_generation {
        let _ = child.kill();
        let _ = child.wait();
        return Err("session runtime was replaced during startup".into());
    }
    // Captured before `child` moves into the wait task below; used to probe
    // the process's live cwd (issue #241) rather than trusting the frozen
    // launch-time cwd forever.
    if let Some(pid) = child.process_id() {
        state.session_pids.lock().unwrap().insert(id.clone(), pid);
    }
    let startup_grace_ends_at = tokio::time::Instant::now() + STARTUP_ATTENTION_GRACE;
    // The PTY has spawned, so the lifecycle is alive before either background
    // task can observe and classify its first output chunk.
    if !set_session_status_for_generation(&state, &id, run_generation, "running") {
        let _ = child.kill();
        let _ = child.wait();
        state.session_pids.lock().unwrap().remove(&id);
        return Err("session runtime was replaced during startup".into());
    }

    let mut reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            state.session_pids.lock().unwrap().remove(&id);
            return Err(error.to_string());
        }
    };
    let writer = match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            state.session_pids.lock().unwrap().remove(&id);
            return Err(error.to_string());
        }
    };
    let master = Arc::new(Mutex::new(pair.master));
    let killer = Arc::new(Mutex::new(child.clone_killer()));

    let (driver_tx, mut driver_rx) = make_driver_event_channel();

    let reader_id = id.clone();
    let reader_tx = driver_tx.clone();
    tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if reader_tx
                        .blocking_send(DriverEvent::Output(buf[..n].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!(session_id = %reader_id, error = %e, "PTY read error");
                    break;
                }
            }
        }
    });

    let wait_tx = driver_tx.clone();
    tokio::task::spawn_blocking(move || {
        let result = child.wait();
        let event = match result {
            Ok(_) => DriverEvent::Exited,
            Err(e) => DriverEvent::WaitError(e.to_string()),
        };
        let _ = wait_tx.blocking_send(event);
    });

    let (persist_tx, mut persist_rx) = make_persist_channel();
    let persist_state = state.clone();
    let persist_id = id.clone();
    let persist_writer = tokio::spawn(async move {
        while let Some(lines) = persist_rx.recv().await {
            let st = persist_state.clone();
            let i = persist_id.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let ws_guard = st.workspace.lock().unwrap();
                if let Some(ref ws) = *ws_guard {
                    ws.metadata.append_terminal_output_records(&i, &lines);
                }
            })
            .await;
        }
    });

    let driver_state = state.clone();
    let driver_id = id.clone();
    let driver_output_tx = output_tx.clone();
    let driver_killer = killer.clone();
    tokio::spawn(async move {
        let mut writer = writer;
        let mut persist_buffer: Vec<u8> = Vec::new();
        let mut pending_persist_batches: VecDeque<Vec<crate::metadata::TerminalOutputRecord>> =
            VecDeque::new();
        let mut kill_requested = false;

        if let Some(prompt) = initial_prompt {
            let prompt_bytes = format!("{}\n", prompt).into_bytes();
            if let Err(e) = writer.write_all(&prompt_bytes) {
                tracing::warn!(session_id = %driver_id, error = %e, "failed to write initial prompt");
            }
        }

        for command in pending_commands {
            match command {
                RuntimeCommand::Input { data, accepted } => {
                    let result = writer
                        .write_all(data.as_bytes())
                        .and_then(|_| writer.flush())
                        .map_err(|_| ());
                    if let Some(accepted) = accepted {
                        let _ = accepted.send(result);
                    }
                }
                RuntimeCommand::Resize { rows, cols } => {
                    let _ = master.lock().unwrap().resize(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
                RuntimeCommand::Kill => {
                    kill_requested = true;
                    let _ = driver_killer.lock().unwrap().kill();
                }
            }
        }

        loop {
            tokio::select! {
                kill_change = kill_rx.changed() => {
                    match kill_change {
                        Ok(()) if *kill_rx.borrow() => {
                            kill_requested = true;
                            let _ = driver_killer.lock().unwrap().kill();
                        }
                        Ok(()) => {}
                        Err(_) => break,
                    }
                }
                reserve = persist_tx.clone().reserve_owned(), if !pending_persist_batches.is_empty() => {
                    match reserve {
                        Ok(permit) => {
                            permit.send(
                                pending_persist_batches
                                    .pop_front()
                                    .expect("pending persist batches should exist when reserve branch runs"),
                            );
                        }
                        Err(_) => {
                            pending_persist_batches.clear();
                        }
                    }
                }
                Some(command) = control_rx.recv() => {
                    match command {
                        RuntimeCommand::Input { data, accepted } => {
                            let result = writer.write_all(data.as_bytes()).and_then(|_| writer.flush()).map_err(|_| ());
                            if let Some(accepted) = accepted {
                                let _ = accepted.send(result);
                            }
                        }
                        RuntimeCommand::Resize { rows, cols } => {
                            let _ = master.lock().unwrap().resize(PtySize {
                                rows,
                                cols,
                                pixel_width: 0,
                                pixel_height: 0,
                            });
                        }
                        RuntimeCommand::Kill => {
                            kill_requested = true;
                            let _ = driver_killer.lock().unwrap().kill();
                        }
                    }
                }
                Some(event) = driver_rx.recv(), if pending_persist_batches.len() < DRIVER_EVENT_BUFFER_CAPACITY => {
                    match event {
                        DriverEvent::Output(data) => {
                            persist_buffer.extend_from_slice(&data);
                            let stripped = peon::strip_ansi(&String::from_utf8_lossy(&data));
                            let raw_persist_lines = drain_persist_records(&mut persist_buffer);
                            let output_at = output_recency_timestamp(
                                &data,
                                crate::workspace_runtime::iso_now(),
                            );

                            let mut promoted_working = false;
                            let mut output_recency_to_persist = None;
                            let mut output_flush_delay = None;
                            {
                                let mut sessions = driver_state.sessions.lock().unwrap();
                                if let Some(handle) = sessions.get_mut(&driver_id) {
                                    if let Some(output_at) = output_at {
                                        let output_persist_at = tokio::time::Instant::now();
                                        handle.info.last_output_at = Some(output_at.clone());
                                        output_recency_to_persist = handle
                                            .runtime
                                            .record_output_recency(output_at, output_persist_at);
                                        output_flush_delay = handle
                                            .runtime
                                            .schedule_output_recency_flush(output_persist_at);
                                    }
                                    for raw in &raw_persist_lines {
                                        let trimmed = raw.text().trim();
                                        if !trimmed.is_empty() {
                                            handle.runtime.peon_output_revision =
                                                handle.output_buffer.push(trimmed.to_string());
                                        }
                                    }
                                    handle.output_lines_seen += raw_persist_lines.len() as u64;
                                    handle.scan_bytes_seen += stripped.len() as u64;
                                    handle.scan_buf.push_str(&stripped);
                                    const MAX_SCAN: usize = 8192;
                                    if handle.scan_buf.len() > MAX_SCAN {
                                        let drop = handle.scan_buf.len() - MAX_SCAN;
                                        let drop = (drop..drop + 4).find(|&i| handle.scan_buf.is_char_boundary(i)).unwrap_or(drop);
                                        handle.scan_buf.drain(..drop);
                                    }
                                    let has_qualifying_work_signal = consume_pending_work_signal(
                                        &mut handle.pending_work_signal,
                                        &stripped,
                                        tokio::time::Instant::now(),
                                    );
                                    if should_infer_working(
                                        &handle.info.lifecycle,
                                        has_qualifying_work_signal,
                                        handle.active_work_hook,
                                        startup_grace_ends_at,
                                    ) {
                                        // Use the shared `ProcessTransition::CommittedWorking`
                                        // fields so promotion clears stale prompt fields
                                        // (`needs_user_input` / `detected_question` /
                                        // `suggested_options`) and sets `metadata_source`
                                        // + `metadata_confidence` on the live handle to
                                        // match the persisted record, identical to the
                                        // Enter-terminated `mark_committed_input_working`
                                        // path. The pre-fix site manually set only
                                        // `observed_status`/`attention`, leaving the live
                                        // handle's `metadata_source = "agent"` and the
                                        // answered prompt's question fields intact —
                                        // surfaced once #273 hooked the single-key path
                                        // onto this output-gated promotion.
                                        let fields = process_transition_fields(
                                            ProcessTransition::CommittedWorking,
                                        );
                                        apply_process_transition_to_handle(
                                            &mut handle.info,
                                            &fields,
                                        );
                                        promoted_working = true;
                                    }
                                    let cursor = handle.runtime.replay.push(data.clone());
                                    let _ = handle.runtime.output_tx.send(RuntimeEvent::Output { cursor, chunk: data.clone() });
                                }
                            }

                            if let Some(timestamp) = output_recency_to_persist {
                                let state = driver_state.clone();
                                let id = driver_id.clone();
                                tokio::task::spawn_blocking(move || {
                                    persist_output_recency(&state, &id, timestamp)
                                });
                            }
                            if let Some(delay) = output_flush_delay {
                                let state = driver_state.clone();
                                let id = driver_id.clone();
                                tokio::spawn(async move {
                                    tokio::time::sleep(delay).await;
                                    flush_output_recency(&state, &id).await;
                                });
                            }

                            if driver_state.peon.config.enabled {
                                driver_state.peon.last_output.write().unwrap()
                                    .insert(driver_id.clone(), tokio::time::Instant::now());
                                driver_state.peon.last_inference.write().unwrap().remove(&driver_id);
                            }

                            // Harness reports remain authoritative. This is only a
                            // fallback for agents that print a conventional plan path
                            // without sending an explicit planPath report.
                            if let Some(plan_path) = raw_persist_lines
                                .iter()
                                .find_map(|line| plan_handoff::printed_plan_path(line.text()))
                            {
                                let ws_guard = driver_state.workspace.lock().unwrap();
                                if let Some(ref ws) = *ws_guard {
                                    if plan_handoff::resolve_openable_plan(&ws.path, &plan_path).is_ok() {
                                        if let Some(mut meta) = ws.metadata.read_session(&driver_id) {
                                            if meta.plan_path.is_none()
                                                && !ws.metadata.plan_path_is_explicitly_cleared(&driver_id)
                                            {
                            meta.plan_path = Some(crate::metadata::PlanReference {
                                worktree_root: Some(ws.path.to_string_lossy().into_owned()),
                                relative_path: plan_path,
                                source: crate::metadata::PlanSource::TerminalFallback,
                            });
                                                ws.metadata.write_session(&meta);
                                            }
                                        }
                                    }
                                }
                            }

                            {
                                let ws_guard = driver_state.workspace.lock().unwrap();
                                if let Some(ref ws) = *ws_guard {
                                    if let Some(mut meta) = ws.metadata.read_session(&driver_id) {
                                        if promoted_working {
                                            let fields = process_transition_fields(
                                                ProcessTransition::CommittedWorking,
                                            );
                                            apply_process_transition_to_meta(&mut meta, &fields);
                                            ws.metadata.write_session(&meta);
                                        }
                                    }
                                }
                            }

                            if !raw_persist_lines.is_empty() {
                                match persist_tx.try_send(raw_persist_lines) {
                                    Ok(()) => {}
                                    Err(tokio::sync::mpsc::error::TrySendError::Full(lines)) => {
                                        pending_persist_batches.push_back(lines);
                                    }
                                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {}
                                }
                            }
                        }
                        DriverEvent::Exited => {
                            let mut final_persist_batches = pending_persist_batches;
                            if !persist_buffer.is_empty() {
                                final_persist_batches
                                    .push_back(vec![crate::metadata::TerminalOutputRecord::raw(
                                        String::from_utf8_lossy(&persist_buffer).into_owned(),
                                        "",
                                    )]);
                            }

                            let status = if kill_requested { "killed" } else { "ended" };
                            if !handle_runtime_exit(
                                &driver_state,
                                &driver_id,
                                run_generation,
                                status,
                            )
                            .await
                            {
                                drop(persist_tx);
                                break;
                            }
                            let _ = driver_output_tx.send(RuntimeEvent::Ended { status: status.to_string() });

                            let trim_state = driver_state.clone();
                            let trim_id = driver_id.clone();
                            tokio::spawn(async move {
                                while let Some(lines) = final_persist_batches.pop_front() {
                                    let _ = persist_tx.send(lines).await;
                                }
                                drop(persist_tx);
                                let _ = persist_writer.await;
                                let _ = tokio::task::spawn_blocking(move || {
                                    let ws_guard = trim_state.workspace.lock().unwrap();
                                    if let Some(ref ws) = *ws_guard {
                                        ws.metadata.trim_terminal_output(&trim_id, metadata::TERMINAL_OUTPUT_MAX_LINES);
                                    }
                                })
                                .await;
                            });
                            break;
                        }
                        DriverEvent::WaitError(error) => {
                            let mut final_persist_batches = pending_persist_batches;
                            if !persist_buffer.is_empty() {
                                final_persist_batches
                                    .push_back(vec![crate::metadata::TerminalOutputRecord::raw(
                                        String::from_utf8_lossy(&persist_buffer).into_owned(),
                                        "",
                                    )]);
                            }
                            if !handle_runtime_exit(
                                &driver_state,
                                &driver_id,
                                run_generation,
                                "error",
                            )
                            .await
                            {
                                drop(persist_tx);
                                break;
                            }
                            let _ = driver_output_tx.send(RuntimeEvent::Error {
                                code: "pty_wait_failed".into(),
                                message: error,
                            });
                            let trim_state = driver_state.clone();
                            let trim_id = driver_id.clone();
                            tokio::spawn(async move {
                                while let Some(lines) = final_persist_batches.pop_front() {
                                    let _ = persist_tx.send(lines).await;
                                }
                                drop(persist_tx);
                                let _ = persist_writer.await;
                                let _ = tokio::task::spawn_blocking(move || {
                                    let ws_guard = trim_state.workspace.lock().unwrap();
                                    if let Some(ref ws) = *ws_guard {
                                        ws.metadata.trim_terminal_output(&trim_id, metadata::TERMINAL_OUTPUT_MAX_LINES);
                                    }
                                })
                                .await;
                            });
                            break;
                        }
                    }
                }
                else => break,
            }
        }
    });

    // Give request-cancellation regressions a deterministic post-spawn window
    // after the child and driver are established but before the caller commits
    // its admission guard. This is test-only; production startup remains
    // uninterrupted here.
    #[cfg(test)]
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness;
    use crate::test_support::test_session_info;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::AtomicU16;
    use std::sync::{Arc, Mutex, RwLock};
    use std::time::Duration;

    fn test_state_with_runtime_session(id: &str) -> Arc<crate::AppState> {
        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                config: crate::peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            id.to_string(),
            crate::SessionHandle {
                info: test_session_info(id.to_string(), "Runtime Test", "/tmp", "running", "now"),
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: SessionRuntime::detached_test(),
            terminal_attached: false,
            resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        state
    }

    #[test]
    fn session_runtime_starts_detached() {
        let runtime = SessionRuntime::detached_test();
        assert!(runtime.attached_generation().is_none());
        assert_eq!(runtime.last_size(), (24, 80));
    }

    #[test]
    fn output_recency_updates_live_state_and_coalesces_persistence() {
        let mut runtime = SessionRuntime::detached_test();
        let first = "2026-07-29T10:00:00Z".to_string();
        let second = "2026-07-29T10:00:01Z".to_string();
        let start = tokio::time::Instant::now();

        assert_eq!(
            runtime.record_output_recency(first.clone(), start),
            Some(first.clone())
        );
        assert_eq!(
            runtime.record_output_recency(second.clone(), start + std::time::Duration::from_secs(1)),
            None
        );
        assert_eq!(
            runtime.schedule_output_recency_flush(start + std::time::Duration::from_secs(1)),
            Some(std::time::Duration::from_secs(4))
        );
        assert_eq!(runtime.flush_output_recency(), Some(second));
    }

    #[test]
    fn output_recency_persistence_never_replaces_a_newer_timestamp() {
        assert!(!should_persist_output_recency(
            Some("2026-07-29T10:00:01Z"),
            "2026-07-29T10:00:00Z",
        ));
        assert!(should_persist_output_recency(
            Some("2026-07-29T10:00:00Z"),
            "2026-07-29T10:00:01Z",
        ));
        assert!(!should_persist_output_recency(
            Some("2026-07-29T10:00:00Z"),
            "not-a-timestamp",
        ));
    }

    #[test]
    fn output_recency_ignores_empty_driver_frames() {
        let timestamp = "2026-07-29T10:00:00Z".to_string();
        assert_eq!(output_recency_timestamp(&[], timestamp.clone()), None);
        assert_eq!(
            output_recency_timestamp(b"progress", timestamp.clone()),
            Some(timestamp),
        );
    }

    #[test]
    fn live_duplicate_attach_is_rejected() {
        let state = test_state_with_runtime_session("runtime-attach");
        let first = claim_attachment(&state, "runtime-attach");
        assert!(first.is_some());
        let second = claim_attachment(&state, "runtime-attach");
        assert!(second.is_none());
    }

    #[test]
    fn stale_cleanup_is_owner_scoped() {
        let state = test_state_with_runtime_session("runtime-release");
        let first = claim_attachment(&state, "runtime-release").unwrap();
        let wrong_generation = first.generation + 1;

        release_attachment(&state, "runtime-release", wrong_generation);
        let still_attached = state
            .sessions
            .lock()
            .unwrap()
            .get("runtime-release")
            .unwrap()
            .runtime
            .attached_generation();
        assert_eq!(still_attached, Some(first.generation));

        release_attachment(&state, "runtime-release", first.generation);
        let detached = state
            .sessions
            .lock()
            .unwrap()
            .get("runtime-release")
            .unwrap()
            .runtime
            .attached_generation();
        assert_eq!(detached, None);
    }

    #[test]
    fn replay_cursor_advances_monotonically() {
        let mut replay = ReplayBuffer::new(3);
        let first = replay.push(vec![1]);
        let second = replay.push(vec![2]);
        let third = replay.push(vec![3]);

        assert!(first < second);
        assert!(second < third);
        assert_eq!(replay.next_cursor(), third + 1);
    }

    #[test]
    fn startup_grace_keeps_visible_output_idle() {
        assert!(!should_infer_working(
            "alive",
            false,
            false,
            tokio::time::Instant::now() + STARTUP_ATTENTION_GRACE,
        ));
    }

    #[test]
    fn qualifying_signal_after_startup_grace_is_working() {
        assert!(should_infer_working(
            "alive",
            true,
            false,
            tokio::time::Instant::now() - std::time::Duration::from_millis(1),
        ));
    }

    #[test]
    fn qualifying_signal_can_resume_working() {
        assert!(should_infer_working(
            "alive",
            true,
            false,
            tokio::time::Instant::now() - std::time::Duration::from_millis(1),
        ));
    }

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
            send_runtime_command(
                &state,
                session_id,
                RuntimeCommand::Input {
                    data: "x".into(),
                    accepted: None,
                },
            )
            .await
            .unwrap();
        }

        // The channel is now full; one more send should not resolve until something drains it.
        let state_clone = state.clone();
        let blocked_send = tokio::spawn(async move {
            send_runtime_command(
                &state_clone,
                session_id,
                RuntimeCommand::Input {
                    data: "overflow".into(),
                    accepted: None,
                },
            )
            .await
        });

        let mut blocked_send = blocked_send;
        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut blocked_send)
                .await
                .is_err(),
            "send on a full bounded channel should not resolve while the channel is full"
        );

        // Draining one slot should let the pending send complete.
        let _ = control_rx.recv().await;
        let result = tokio::time::timeout(Duration::from_secs(1), blocked_send)
            .await
            .expect("blocked send should complete once a slot frees up")
            .unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn startup_buffer_caps_input_bytes() {
        let (tx, mut rx) = mpsc::channel(3);
        let chunk = "x".repeat(STARTUP_PENDING_INPUT_BYTES / 2);
        for _ in 0..3 {
            tx.send(RuntimeCommand::Input {
                data: chunk.clone(),
                accepted: None,
            })
            .await
            .unwrap();
        }
        drop(tx);

        let (_, pending) = capture_startup_runtime_state(
            &mut rx,
            PtySize {
                rows: DEFAULT_TERMINAL_ROWS,
                cols: DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
        .await;

        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn observer_only_output_cannot_resume_finished_states() {
        let past_grace = tokio::time::Instant::now() - std::time::Duration::from_millis(1);
        for status in [
            "idle",
            "waiting_for_input",
            "blocked",
            "failed",
            "stale",
            "done",
        ] {
            assert!(
                !should_infer_working("alive", false, false, past_grace),
                "observer-only output should not resume {status} to working"
            );
        }
    }

    #[test]
    fn extending_on_next_keystroke_does_not_falsely_qualify_its_own_echo() {
        let now = tokio::time::Instant::now();
        // First keystroke 'h': armed fresh; its echo arrives and is
        // correctly absorbed as non-qualifying.
        let mut signal = Some(arm_pending_work_signal("h", now));
        assert!(!consume_pending_work_signal(&mut signal, "h", now));
        // Second keystroke 'e': the single-key arming site in
        // terminal_runtime.rs extends the signal with only the newly typed
        // delta ("e"), not the whole accumulated buffer — matching how a
        // PTY actually echoes back just the character just typed. This must
        // still not qualify as genuine model output while composing.
        extend_pending_work_signal(&mut signal, "e", now);
        assert!(
            !consume_pending_work_signal(&mut signal, "e", now),
            "echo of a later keystroke must not be mistaken for genuine model output"
        );
    }

    #[test]
    fn split_echo_does_not_qualify_until_new_visible_output_arrives() {
        let now = tokio::time::Instant::now();
        let mut signal = Some(arm_pending_work_signal("fix status", now));
        assert!(!consume_pending_work_signal(&mut signal, "fix ", now));
        assert!(!consume_pending_work_signal(&mut signal, "status\r\n", now));
        assert!(consume_pending_work_signal(&mut signal, "Thinking…", now));
    }

    #[test]
    fn one_leading_line_ending_is_ignored_when_matching_echo() {
        let now = tokio::time::Instant::now();
        for leading in ['\r', '\n'] {
            let mut signal = Some(arm_pending_work_signal("fix", now));

            assert!(!consume_pending_work_signal(
                &mut signal,
                &format!("{leading}fix"),
                now,
            ));
            assert!(consume_pending_work_signal(&mut signal, "Thinking…", now));
        }
    }

    #[test]
    fn ansi_only_output_and_expired_submission_do_not_qualify() {
        let now = tokio::time::Instant::now();
        let mut signal = Some(arm_pending_work_signal("fix", now));
        assert!(!consume_pending_work_signal(&mut signal, "\x1b[2K\r", now));
        assert!(!consume_pending_work_signal(
            &mut signal,
            "model output",
            now + std::time::Duration::from_secs(10),
        ));
        assert!(
            signal.is_none(),
            "expired signal must be cleared, not rechecked forever"
        );
    }

    #[test]
    fn ansi_only_output_preserves_pending_echo_for_following_output() {
        let now = tokio::time::Instant::now();
        let mut signal = Some(arm_pending_work_signal("fix", now));

        assert!(!consume_pending_work_signal(&mut signal, "\x1b[2K\r", now));
        assert!(!consume_pending_work_signal(&mut signal, "fix\r\n", now));
        assert!(consume_pending_work_signal(&mut signal, "Thinking…", now));
    }

    #[test]
    fn control_only_output_does_not_qualify_as_visible() {
        let now = tokio::time::Instant::now();
        let mut signal = Some(arm_pending_work_signal("fix", now));
        assert!(!consume_pending_work_signal(&mut signal, "fix\r\n", now));
        // A bare BEL (or other C0 control byte) surviving ANSI-stripping must
        // not be mistaken for genuine model output.
        assert!(!consume_pending_work_signal(&mut signal, "\x07", now));
        assert!(consume_pending_work_signal(&mut signal, "Thinking…", now));
    }

    #[test]
    fn submitted_terminal_input_immediately_marks_live_session_working_without_pending_signal() {
        let session_id = "terminal-input-work-signal";
        let state = test_state_with_runtime_session(session_id);

        assert!(crate::runtime::terminal_runtime::record_terminal_input(
            &state, session_id, "fix\r"
        )
        .is_some());
        let sessions = state.sessions.lock().unwrap();
        let handle = &sessions[session_id];
        assert_eq!(handle.info.attention.as_deref(), Some("working"));
        assert!(handle.pending_work_signal.is_none());
    }

    #[test]
    fn ansi_arrow_key_does_not_arm_work_signal_after_single_key_input() {
        let session_id = "single-key-arrow-key";
        let state = test_state_with_runtime_session(session_id);

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.info.attention = Some("needs_you".into());
            handle.info.metadata_source = Some("agent".into());
        }

        // A prior accepted response leaves an in-progress echo prefix. Model a
        // later arrow-key edit after its original work signal expired.
        crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y");
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut(session_id)
            .unwrap()
            .pending_work_signal = None;

        // collect_input_line parses ESC [ A as a control sequence. It must not
        // re-arm the fallback merely because the raw frame contains '[' and 'A'.
        crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "\x1b[A");

        assert!(
            state.sessions.lock().unwrap()[session_id]
                .pending_work_signal
                .is_none(),
            "ANSI arrow-key input must not arm the work signal"
        );
    }

    #[test]
    fn newline_input_keeps_working_without_pending_signal() {
        let session_id = "multi-char-enter";
        let state = test_state_with_runtime_session(session_id);

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.info.attention = Some("needs_you".into());
            handle.info.metadata_source = Some("agent".into());
        }

        assert!(
            crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "fix")
                .is_none()
        );
        crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "\r")
            .expect("Enter submits the line and record_terminal_input returns Some(())");
        let sessions = state.sessions.lock().unwrap();
        let handle = &sessions[session_id];
        assert_eq!(handle.info.attention.as_deref(), Some("working"));
        assert!(handle.pending_work_signal.is_none());
    }

    #[test]
    fn single_key_does_not_re_arm_when_attention_is_working() {
        let session_id = "single-key-no-noise-on-working";
        let state = test_state_with_runtime_session(session_id);

        // Session is already working (process-sourced — e.g. the model IS generating).
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.info.attention = Some("working".into());
            handle.info.metadata_source = Some("process".into());
        }

        // A printable keystroke arrives mid-working. It must NOT arm a work
        // signal — the session is already working and re-arming would
        // introduce noise.
        assert!(
            crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y")
                .is_none()
        );

        let sessions = state.sessions.lock().unwrap();
        assert!(
            sessions[session_id].pending_work_signal.is_none(),
            "keystroke during working must not re-arm the work signal"
        );
    }

    #[test]
    fn bare_keystroke_preserves_peon_sourced_needs_you() {
        let session_id = "single-key-not-for-peon-needs-you";
        let state = test_state_with_runtime_session(session_id);

        // A Peon-inferred prompt must remain visible while the user is still
        // composing input.
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.info.attention = Some("needs_you".into());
            handle.info.observed_status = Some("waiting_for_input".into());
            handle.info.metadata_source = Some("peon".into());
        }

        assert!(
            crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y")
                .is_none()
        );

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[session_id].info.attention.as_deref(),
            Some("needs_you")
        );
        assert_eq!(
            sessions[session_id].info.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        assert!(
            sessions[session_id].pending_work_signal.is_none(),
            "bare input must not create an output-gated working transition"
        );
    }

    #[test]
    fn bare_keystroke_preserves_hook_sourced_needs_you() {
        let session_id = "single-key-not-for-capable-hook";
        let state = test_state_with_runtime_session(session_id);

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.active_work_hook = true;
            handle.info.attention = Some("needs_you".into());
            handle.info.observed_status = Some("waiting_for_input".into());
            handle.info.metadata_source = Some("agent".into());
        }

        assert!(
            crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y")
                .is_none()
        );

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[session_id].info.attention.as_deref(),
            Some("needs_you")
        );
        assert_eq!(
            sessions[session_id].info.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        assert!(
            sessions[session_id].pending_work_signal.is_none(),
            "bare input must not create an output-gated working transition"
        );
    }

    /// Pins the narrow single-key work-signal arming (#273, restoring the #179
    /// fix that `31f9b4e` reverted). Claude Code's Notification hook sets
    /// `needs_you` with `metadata_source = "agent"`; its prompts take single
    /// keystrokes (y/n/1/2/3) with no Enter. The bare keystroke must arm
    /// `pending_work_signal` so the next visible PTY output can promote to
    /// `working`. This was the test that demonstrated the regression on
    /// pre-fix `main` (post-`31f9b4e`); it now stays green on `main`.
    #[test]
    fn bare_keystroke_arms_work_signal_for_hookless_agent_sourced_needs_you() {
        let session_id = "single-key-arms-for-hookless-agent-needs-you";
        let state = test_state_with_runtime_session(session_id);

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            // `test_state_with_runtime_session` defaults `active_work_hook=false`.
            handle.info.attention = Some("needs_you".into());
            handle.info.observed_status = Some("waiting_for_input".into());
            handle.info.metadata_source = Some("agent".into());
        }

        assert!(
            crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y")
                .is_none(),
            "bare input does not complete a line — record_terminal_input returns None for it"
        );

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[session_id].info.attention.as_deref(),
            Some("needs_you"),
            "attention must stay needs_you until visible output promotes it"
        );
        assert!(
            sessions[session_id].pending_work_signal.is_some(),
            "bare printable key on a hookless agent-sourced needs_you must arm the work signal"
        );
    }

    /// Pins the end-to-end promotion path for the narrow single-key arming
    /// (#273): hook-sourced `needs_you` answered with a single key, followed
    /// by visible PTY output, must promote attention and observed status to
    /// `working` on both the live handle and persisted metadata, with
    /// `metadata_source = "process"` taking over from the prior `"agent"`.
    /// Mirrors the existing
    /// `submitted_input_at_hook_sourced_needs_you_is_working_before_visible_output`
    /// but replaces the Enter-terminated `"y\r"` with a bare `"y"` — the
    /// Claude Code interaction shape that `31f9b4e` inadvertently broke. This
    /// was the test that demonstrated the regression on pre-fix `main`
    /// (post-`31f9b4e`); it now stays green on `main`.
    #[tokio::test]
    async fn single_key_at_hook_sourced_needs_you_promotes_to_working_on_visible_output() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = "single-key-e2e-arming-promote";
        let state = test_state_with_runtime_session(session_id);
        let metadata_root = dir.path().join(".orkworks-test");
        *state.workspace.lock().unwrap() = Some(crate::WorkspaceState {
            path: dir.path().to_path_buf(),
            metadata: crate::metadata::MetadataStore::new(&metadata_root),
            watcher: crate::watcher::MetadataWatcher::start(&metadata_root.join("sessions")),
        });

        // Simulate a Claude Code Notification hook POST: needs_you with
        // metadata_source=agent and the prompt's question fields populated
        // (`needs_user_input` / `detected_question` / `suggested_options`),
        // matching what `merge_agent_attention_signal_with_plan` persists on
        // a real `waiting_for_input` hook report. Persist the same state to
        // disk so the runtime's output handler — which only writes back via
        // the metadata store when workspace is wired — finds a base session
        // record to merge into. Seeding the question fields here is what
        // lets the post-promotion assertions prove the output-gated path
        // clears them (the Codex #1 concern): without that, they would simply
        // stay `None` and the assertions would prove nothing.
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.info.attention = Some("needs_you".into());
            handle.info.metadata_source = Some("agent".into());
            handle.info.metadata_confidence = Some(1.0);
            handle.info.lifecycle = "alive".into();
            handle.info.needs_user_input = Some(true);
            handle.info.detected_question = Some("Proceed?".into());
            handle.info.suggested_options = Some(vec!["yes".into(), "no".into()]);
        }
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = crate::test_support::test_session_metadata(
                session_id,
                "Runtime Test",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle_phase = "active".into();
            meta.lifecycle = "alive".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            meta.attention = Some("needs_you".into());
            meta.metadata_source = "agent".into();
            meta.metadata_confidence = 1.0;
            meta.needs_user_input = Some(true);
            meta.detected_question = Some("Proceed?".into());
            meta.suggested_options = Some(vec!["yes".into(), "no".into()]);
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        // Single-key acceptance (e.g. `y`). No Enter: it must not commit the
        // working transition directly, but must arm `pending_work_signal` so
        // the next visible PTY chunk can promote.
        assert!(
            crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y")
                .is_none(),
            "bare input without Enter returns None — no completed line to label-record"
        );
        {
            let sessions = state.sessions.lock().unwrap();
            assert_eq!(
                sessions[session_id].info.attention.as_deref(),
                Some("needs_you"),
                "the bare keystroke must not promote attention on its own"
            );
            assert!(
                sessions[session_id].pending_work_signal.is_some(),
                "the bare keystroke must have armed the work signal"
            );
        }

        // Spin up a real PTY that sleeps briefly (past the 2s startup grace)
        // then emits visible output. The output flows through
        // start_session_runtime's DriverEvent::Output handler, which calls
        // consume_pending_work_signal against the armed signal and promotes
        // attention to working + metadata_source to process.
        let (runtime, control_rx) =
            SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
        let output_tx = runtime.output_tx.clone();
        let mut events = output_tx.subscribe();

        // Deliberately `-c`, not `-lc`: this assertion depends on the marker
        // being the *first* visible PTY chunk (consume_pending_work_signal
        // qualifies on the first non-echo visible output it sees). A login
        // shell sources profile scripts that can print their own banner
        // before this printf runs — e.g. an nvm init script — which then
        // gets consumed as the "genuine" signal instead of the real marker,
        // so the promotion this test exists to verify never fires. The test
        // only needs a shell that can sleep and printf; it has no PATH/login
        // dependency, so `-c` is both sufficient and immune to this class of
        // environment-specific test failure.
        let command = harness::CommandSpec {
            program: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "sleep 2.2; printf 'model-output-after-bare-key\\n'; sleep 1".into(),
            ],
            cwd: dir.path().display().to_string(),
        };

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.runtime = runtime;
        }

        let (kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        start_session_runtime(
            state.clone(),
            session_id.to_string(),
            command,
            None,
            control_rx,
            output_tx,
            kill_rx,
            PtySize {
                rows: DEFAULT_TERMINAL_ROWS,
                cols: DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
        .await
        .unwrap();

        // Wait for the model-output marker to arrive. 3s window covers the 2.2s
        // sleep + printf + runtime latency.
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match events.recv().await {
                    Ok(RuntimeEvent::Output { chunk, .. })
                        if String::from_utf8_lossy(&chunk)
                            .contains("model-output-after-bare-key") =>
                    {
                        break;
                    }
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(error) => panic!("unexpected runtime event error: {error}"),
                }
            }
        })
        .await
        .expect("model output should arrive within the 3s window");

        // Yield once so the runtime's output handler finishes its multi-lock
        // sequence (sessions lock for the in-memory promoted_working write, then
        // the workspace lock for the persisted metadata write-back).
        tokio::task::yield_now().await;

        let sessions = state.sessions.lock().unwrap();
        let handle = sessions.get(session_id).unwrap();
        assert_eq!(
            handle.info.attention.as_deref(),
            Some("working"),
            "visible model output after a single-key answer must promote attention to working"
        );
        assert!(
            handle.pending_work_signal.is_none(),
            "qualifying output must consume the armed work signal"
        );
        assert_eq!(
            handle.info.observed_status.as_deref(),
            Some("working"),
            "the output-gated promotion must set observed_status to working on the live handle"
        );
        assert_eq!(
            handle.info.metadata_source.as_deref(),
            Some("process"),
            "the output-gated promotion must take over metadata_source from agent to process on \
             the live handle (Codex #1: previously the inline field-set left the stale agent \
             source intact while attention flipped to working)"
        );
        assert_eq!(handle.info.metadata_confidence, Some(1.0));
        assert_eq!(
            handle.info.needs_user_input, None,
            "promotion must clear needs_user_input on the live handle (Codex #1)"
        );
        assert_eq!(
            handle.info.detected_question, None,
            "promotion must clear detected_question on the live handle (Codex #1)"
        );
        assert!(
            handle.info.suggested_options.is_none(),
            "promotion must clear suggested_options on the live handle (Codex #1)"
        );
        drop(sessions);

        let ws = state.workspace.lock().unwrap();
        let meta = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session(session_id)
            .expect("session metadata should be persisted after promotion");
        assert_eq!(
            meta.attention.as_deref(),
            Some("working"),
            "persisted attention must reflect the post-output promotion"
        );
        assert_eq!(
            meta.metadata_source, "process",
            "the process-sourced output path owns the persisted promotion, not the agent hook"
        );
        assert_eq!(meta.metadata_confidence, 1.0);
        assert_eq!(
            meta.needs_user_input, None,
            "promotion must clear persisted needs_user_input (Codex #1)"
        );
        assert_eq!(
            meta.detected_question, None,
            "promotion must clear persisted detected_question (Codex #1)"
        );
        assert!(
            meta.suggested_options.is_none(),
            "promotion must clear persisted suggested_options (Codex #1)"
        );
        drop(ws);

        kill_tx.send(true).unwrap();
    }

    #[tokio::test]
    async fn submitted_input_at_hook_sourced_needs_you_is_working_before_visible_output() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = "single-key-e2e-promote";
        let state = test_state_with_runtime_session(session_id);
        let metadata_root = dir.path().join(".orkworks-test");
        *state.workspace.lock().unwrap() = Some(crate::WorkspaceState {
            path: dir.path().to_path_buf(),
            metadata: crate::metadata::MetadataStore::new(&metadata_root),
            watcher: crate::watcher::MetadataWatcher::start(&metadata_root.join("sessions")),
        });

        // Simulate a hook report: needs_you, metadata_source=agent. Persist the
        // same state to disk so the runtime's output handler — which only
        // writes back via the metadata store when workspace is wired — finds a
        // base session record to merge into.
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.info.attention = Some("needs_you".into());
            handle.info.metadata_source = Some("agent".into());
            handle.info.lifecycle = "alive".into();
        }
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = crate::test_support::test_session_metadata(
                session_id,
                "Runtime Test",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle_phase = "active".into();
            meta.lifecycle = "alive".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            meta.attention = Some("needs_you".into());
            meta.metadata_source = "agent".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        // An accepted submitted line is sufficient evidence of resumed work;
        // no PTY output is needed to clear the prompt.
        assert!(
            crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y\r")
                .is_some()
        );

        {
            let sessions = state.sessions.lock().unwrap();
            assert_eq!(
                sessions[session_id].info.attention.as_deref(),
                Some("working")
            );
            assert!(sessions[session_id].pending_work_signal.is_none());
        }

        // Spin up a real PTY that sleeps briefly (past the 2s startup grace)
        // then emits visible output. The output flows through
        // start_session_runtime's DriverEvent::Output handler, which calls
        // consume_pending_work_signal and promotes attention to working +
        // metadata_source to process.
        let (runtime, control_rx) =
            SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
        let output_tx = runtime.output_tx.clone();
        let mut events = output_tx.subscribe();

        let command = harness::CommandSpec {
            program: "/bin/sh".into(),
            args: vec![
                "-lc".into(),
                "sleep 2.2; printf 'model-output-after-single-key\\n'; sleep 1".into(),
            ],
            cwd: dir.path().display().to_string(),
        };

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.runtime = runtime;
        }

        let (kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        start_session_runtime(
            state.clone(),
            session_id.to_string(),
            command,
            None,
            control_rx,
            output_tx,
            kill_rx,
            PtySize {
                rows: DEFAULT_TERMINAL_ROWS,
                cols: DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
        .await
        .unwrap();

        // Wait for the model-output marker to arrive. 3s window covers the 2.2s
        // sleep + printf + runtime latency.
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match events.recv().await {
                    Ok(RuntimeEvent::Output { chunk, .. })
                        if String::from_utf8_lossy(&chunk)
                            .contains("model-output-after-single-key") =>
                    {
                        break;
                    }
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(error) => panic!("unexpected runtime event error: {error}"),
                }
            }
        })
        .await
        .expect("model output should arrive within the 3s window");

        // Yield once so the runtime's output handler finishes its multi-lock
        // sequence (sessions lock for the in-memory promoted_working write, then
        // the workspace lock for the persisted metadata write-back).
        tokio::task::yield_now().await;

        let sessions = state.sessions.lock().unwrap();
        let handle = sessions.get(session_id).unwrap();
        assert_eq!(
            handle.info.attention.as_deref(),
            Some("working"),
            "later output must not undo the immediate input transition"
        );
        assert!(
            handle.pending_work_signal.is_none(),
            "accepted input must not leave an output-gated work signal behind"
        );
        drop(sessions);

        // The metadata transition is also immediate; later output merely leaves
        // that current state intact.
        let ws = state.workspace.lock().unwrap();
        let meta = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session(session_id)
            .expect("session metadata should be persisted after promotion");
        assert_eq!(
            meta.attention.as_deref(),
            Some("working"),
            "persisted attention must reflect the immediate transition"
        );
        assert_eq!(
            meta.metadata_source, "process",
            "committed input sets metadata_source=process before output"
        );
        drop(ws);

        kill_tx.send(true).unwrap();
    }

    #[test]
    fn long_submission_immediately_marks_working_without_fallback() {
        let session_id = "terminal-input-long-line";
        let state = test_state_with_runtime_session(session_id);
        let long_command = "x".repeat(150);

        crate::runtime::terminal_runtime::record_terminal_input(
            &state,
            session_id,
            &format!("{long_command}\r"),
        )
        .expect("completed terminal input should be accepted");

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[session_id].info.attention.as_deref(),
            Some("working")
        );
        assert!(sessions[session_id].pending_work_signal.is_none());
    }

    #[test]
    fn capable_hook_sessions_never_infer_working_from_pty_output() {
        let past_grace = tokio::time::Instant::now() - std::time::Duration::from_millis(1);

        assert!(!should_infer_working("alive", true, true, past_grace));
    }

    #[test]
    fn terminal_input_overwrites_stale_observed_attention_in_memory_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = "terminal-input-preserves-attention";
        let state = test_state_with_runtime_session(session_id);
        let metadata_root = dir.path().join(".orkworks-test");
        *state.workspace.lock().unwrap() = Some(crate::WorkspaceState {
            path: dir.path().to_path_buf(),
            metadata: crate::metadata::MetadataStore::new(&metadata_root),
            watcher: crate::watcher::MetadataWatcher::start(&metadata_root.join("sessions")),
        });
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.info.observed_status = Some("waiting_for_input".into());
            handle.info.attention = Some("waiting_for_input".into());
            handle.info.metadata_source = Some("peon".into());
            handle.info.needs_user_input = Some(true);
            handle.info.detected_question = Some("Continue?".into());
            handle.info.suggested_options = Some(vec!["yes".into(), "no".into()]);
        }
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = crate::test_support::test_session_metadata(
                session_id,
                "Runtime Test",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.observed_status = Some("waiting_for_input".into());
            meta.attention = Some("waiting_for_input".into());
            meta.metadata_source = "peon".into();
            meta.needs_user_input = Some(true);
            meta.detected_question = Some("Continue?".into());
            meta.suggested_options = Some(vec!["yes".into(), "no".into()]);
            meta.lifecycle_phase = "active".into();
            meta.lifecycle = "alive".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "continue\r")
            .expect("completed terminal input should be accepted");

        {
            let sessions = state.sessions.lock().unwrap();
            let handle = &sessions[session_id];
            assert_eq!(handle.info.observed_status.as_deref(), Some("working"));
            assert_eq!(handle.info.attention.as_deref(), Some("working"));
            assert_eq!(handle.info.needs_user_input, None);
            assert_eq!(handle.info.detected_question, None);
            assert_eq!(handle.info.suggested_options, None);
            assert_eq!(handle.runtime.input_generation, 1);
            assert!(handle.runtime.accepted_input_at.is_some());
            assert_eq!(
                handle.runtime.min_peon_output_revision,
                handle.runtime.peon_output_revision
            );
        }
        let ws = state.workspace.lock().unwrap();
        let meta = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session(session_id)
            .unwrap();
        assert_eq!(meta.observed_status.as_deref(), Some("working"));
        assert_eq!(meta.attention.as_deref(), Some("working"));
        assert_eq!(meta.needs_user_input, None);
        assert_eq!(meta.detected_question, None);
        assert_eq!(meta.suggested_options, None);
    }

    #[test]
    fn terminal_input_without_observed_status_marks_working_without_churning_seeded_label() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = "terminal-input-without-observed-status";
        let state = test_state_with_runtime_session(session_id);
        let metadata_root = dir.path().join(".orkworks-test");
        *state.workspace.lock().unwrap() = Some(crate::WorkspaceState {
            path: dir.path().to_path_buf(),
            metadata: crate::metadata::MetadataStore::new(&metadata_root),
            watcher: crate::watcher::MetadataWatcher::start(&metadata_root.join("sessions")),
        });
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = crate::test_support::test_session_metadata(
                session_id,
                "Runtime Test",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle_phase = "active".into();
            meta.lifecycle = "alive".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let line = "describe the next implementation step";
        let _ = crate::runtime::terminal_runtime::record_terminal_input(
            &state,
            session_id,
            &format!("{line}\r"),
        );

        let sessions = state.sessions.lock().unwrap();
        let handle = &sessions[session_id];
        // The label was already seeded ("Runtime Test" is not the creation
        // placeholder) — further input must not churn it (ADR 0029).
        assert_eq!(handle.info.label, "Runtime Test");
        assert_eq!(handle.info.attention.as_deref(), Some("working"));
        assert_eq!(handle.info.observed_status.as_deref(), Some("working"));
        drop(sessions);
        assert!(state
            .peon
            .label_hint
            .read()
            .unwrap()
            .get(session_id)
            .is_none());
        assert!(!state
            .peon
            .label_pending
            .read()
            .unwrap()
            .contains(session_id));

        let ws = state.workspace.lock().unwrap();
        let meta = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session(session_id)
            .unwrap();
        assert_eq!(meta.label, "Runtime Test");
        assert_eq!(meta.last_user_input.as_deref(), Some(line));
        assert_eq!(meta.attention.as_deref(), Some("working"));
        assert_eq!(meta.observed_status.as_deref(), Some("working"));
    }

    #[tokio::test]
    async fn output_within_startup_grace_is_replayed_without_marking_attention_working() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = "runtime-startup-grace";
        let state = test_state_with_runtime_session(session_id);

        let (runtime, control_rx) =
            SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
        let output_tx = runtime.output_tx.clone();
        let mut events = output_tx.subscribe();

        // start_session_runtime waits INITIAL_RESIZE_GRACE before it spawns the
        // child. This emits 1.9 seconds after spawn: within the full two-second
        // grace, but after the old deadline that started before the resize wait.
        let command = harness::CommandSpec {
            program: "/bin/sh".into(),
            args: vec![
                "-lc".into(),
                "sleep 1.9; printf 'startup-grace-output\\n'; sleep 1".into(),
            ],
            cwd: dir.path().display().to_string(),
        };

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.runtime = runtime;
            handle.pending_work_signal = Some(arm_pending_work_signal(
                "submitted command",
                tokio::time::Instant::now(),
            ));
        }

        let (kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        start_session_runtime(
            state.clone(),
            session_id.to_string(),
            command,
            None,
            control_rx,
            output_tx,
            kill_rx,
            PtySize {
                rows: DEFAULT_TERMINAL_ROWS,
                cols: DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
        .await
        .unwrap();

        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match events.recv().await {
                    Ok(RuntimeEvent::Output { chunk, .. })
                        if String::from_utf8_lossy(&chunk).contains("startup-grace-output") =>
                    {
                        break;
                    }
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(error) => panic!("unexpected runtime event error: {error}"),
                }
            }
        })
        .await
        .expect("process should emit startup output within the grace window");

        let handle = state.sessions.lock().unwrap();
        let session = handle.get(session_id).unwrap();
        assert!(session
            .runtime
            .replay
            .snapshot()
            .iter()
            .any(|(_, chunk)| String::from_utf8_lossy(chunk).contains("startup-grace-output")));
        assert!(session
            .output_buffer
            .snapshot()
            .iter()
            .any(|line| line.contains("startup-grace-output")));
        assert_ne!(session.info.attention.as_deref(), Some("working"));
        assert_ne!(session.info.observed_status.as_deref(), Some("working"));
        assert!(session.pending_work_signal.is_none());
        drop(handle);

        kill_tx.send(true).unwrap();
    }

    #[tokio::test]
    async fn partial_hookless_terminal_input_does_not_promote_before_submission() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = "runtime-hookless-working";
        let state = test_state_with_runtime_session(session_id);
        let metadata_root = dir.path().join(".orkworks-test");
        *state.workspace.lock().unwrap() = Some(crate::WorkspaceState {
            path: dir.path().to_path_buf(),
            metadata: crate::metadata::MetadataStore::new(&metadata_root),
            watcher: crate::watcher::MetadataWatcher::start(&metadata_root.join("sessions")),
        });
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = crate::test_support::test_session_metadata(
                session_id,
                "Runtime Test",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle_phase = "active".into();
            meta.lifecycle = "alive".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let (runtime, control_rx) =
            SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
        let output_tx = runtime.output_tx.clone();
        let mut events = output_tx.subscribe();
        let command = harness::CommandSpec {
            program: "/bin/sh".into(),
            args: vec![
                "-lc".into(),
                "sleep 2.2; printf 'unsolicited-output\\n'; read -r command; printf 'qualifying-output\\n'; sleep 1".into(),
            ],
            cwd: dir.path().display().to_string(),
        };
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.runtime = runtime;
        }

        let (_kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        start_session_runtime(
            state.clone(),
            session_id.to_string(),
            command,
            None,
            control_rx,
            output_tx,
            kill_rx,
            PtySize {
                rows: DEFAULT_TERMINAL_ROWS,
                cols: DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
        .await
        .unwrap();

        assert!(crate::runtime::terminal_runtime::record_terminal_input(
            &state, session_id, "work",
        )
        .is_none());
        send_runtime_command(
            &state,
            session_id,
            RuntimeCommand::Input {
                data: "work".into(),
                accepted: None,
            },
        )
        .await
        .unwrap();

        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match events.recv().await {
                    Ok(RuntimeEvent::Output { chunk, .. })
                        if String::from_utf8_lossy(&chunk).contains("unsolicited-output") =>
                    {
                        break;
                    }
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(error) => panic!("unexpected runtime event error: {error}"),
                }
            }
        })
        .await
        .expect("process should produce unsolicited output after startup grace");
        assert_eq!(
            state.sessions.lock().unwrap()[session_id]
                .info
                .observed_status
                .as_deref(),
            None,
            "accepted partial terminal input must not mark the session working before Enter"
        );

        assert!(crate::runtime::terminal_runtime::record_terminal_input(
            &state, session_id, " now\r",
        )
        .is_some());
        send_runtime_command(
            &state,
            session_id,
            RuntimeCommand::Input {
                data: " now\r".into(),
                accepted: None,
            },
        )
        .await
        .unwrap();

        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match events.recv().await {
                    Ok(RuntimeEvent::Output { chunk, .. })
                        if String::from_utf8_lossy(&chunk).contains("qualifying-output") =>
                    {
                        break;
                    }
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(error) => panic!("unexpected runtime event error: {error}"),
                }
            }
        })
        .await
        .expect("submitted terminal command should produce qualifying output");

        let sessions = state.sessions.lock().unwrap();
        let handle = sessions.get(session_id).unwrap();
        assert_eq!(handle.info.observed_status.as_deref(), Some("working"));
        assert_eq!(handle.info.attention.as_deref(), Some("working"));
        drop(sessions);

        let ws = state.workspace.lock().unwrap();
        let meta = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session(session_id)
            .unwrap();
        assert_eq!(meta.observed_status.as_deref(), Some("working"));
        assert_eq!(meta.attention.as_deref(), Some("working"));
        assert_eq!(meta.metadata_source, "process");
    }

    #[tokio::test]
    async fn capable_terminal_input_immediately_marks_working_without_hook_signal() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = "runtime-capable-work-signal";
        let state = test_state_with_runtime_session(session_id);
        let (runtime, control_rx) =
            SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
        let output_tx = runtime.output_tx.clone();
        let mut events = output_tx.subscribe();
        let command = harness::CommandSpec {
            program: "/bin/sh".into(),
            args: vec![
                "-lc".into(),
                "read -r command; printf 'capable-output\\n'; sleep 1".into(),
            ],
            cwd: dir.path().display().to_string(),
        };
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.active_work_hook = true;
            handle.runtime = runtime;
        }

        let (_kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        start_session_runtime(
            state.clone(),
            session_id.to_string(),
            command,
            None,
            control_rx,
            output_tx,
            kill_rx,
            PtySize {
                rows: DEFAULT_TERMINAL_ROWS,
                cols: DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
        .await
        .unwrap();

        tokio::time::sleep(STARTUP_ATTENTION_GRACE + Duration::from_millis(50)).await;
        assert!(crate::runtime::terminal_runtime::record_terminal_input(
            &state,
            session_id,
            "work now\r",
        )
        .is_some());
        send_runtime_command(
            &state,
            session_id,
            RuntimeCommand::Input {
                data: "work now\r".into(),
                accepted: None,
            },
        )
        .await
        .unwrap();

        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match events.recv().await {
                    Ok(RuntimeEvent::Output { chunk, .. })
                        if String::from_utf8_lossy(&chunk).contains("capable-output") =>
                    {
                        break;
                    }
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(error) => panic!("unexpected runtime event error: {error}"),
                }
            }
        })
        .await
        .expect("capable terminal command should produce output");

        let handle = state.sessions.lock().unwrap();
        assert!(handle[session_id].pending_work_signal.is_none());
        assert_eq!(
            handle[session_id].info.observed_status.as_deref(),
            Some("working")
        );
        assert_eq!(
            handle[session_id].info.attention.as_deref(),
            Some("working")
        );
    }

    #[tokio::test]
    async fn early_resize_after_start_sets_initial_pty_size_before_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("pty-size.txt");
        let session_id = "runtime-size";
        let state = test_state_with_runtime_session(session_id);

        let (runtime, control_rx) =
            SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
        let output_tx = runtime.output_tx.clone();
        let control_tx = runtime.control_tx.clone();

        let command = harness::CommandSpec {
            program: "/bin/sh".into(),
            args: vec![
                "-lc".into(),
                format!("stty size > {}", output_path.display()),
            ],
            cwd: dir.path().display().to_string(),
        };

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.runtime = runtime;
        }

        let (_kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        let runtime_task = tokio::spawn(start_session_runtime(
            state,
            session_id.to_string(),
            command,
            None,
            control_rx,
            output_tx,
            kill_rx,
            PtySize {
                rows: DEFAULT_TERMINAL_ROWS,
                cols: DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        ));

        tokio::time::sleep(Duration::from_millis(100)).await;
        control_tx
            .send(RuntimeCommand::Resize {
                rows: 40,
                cols: 120,
            })
            .await
            .unwrap();

        runtime_task.await.unwrap().unwrap();

        for _ in 0..20 {
            if output_path.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let size = std::fs::read_to_string(&output_path).unwrap();
        assert_eq!(size.trim(), "40 120");
    }

    #[tokio::test]
    async fn session_pid_is_captured_and_probes_the_real_process_cwd_then_clears_on_exit() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = "runtime-live-cwd";
        let state = test_state_with_runtime_session(session_id);

        let (runtime, control_rx) =
            SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
        let output_tx = runtime.output_tx.clone();

        let command = harness::CommandSpec {
            program: "/bin/sh".into(),
            args: vec!["-lc".into(), "sleep 0.3".into()],
            cwd: dir.path().display().to_string(),
        };

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.runtime = runtime;
        }

        let (_kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        let runtime_task = tokio::spawn(start_session_runtime(
            state.clone(),
            session_id.to_string(),
            command,
            None,
            control_rx,
            output_tx,
            kill_rx,
            PtySize {
                rows: DEFAULT_TERMINAL_ROWS,
                cols: DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        ));

        // Spawn happens asynchronously inside the task; poll for the pid to land.
        let pid = {
            let mut found = None;
            for _ in 0..50 {
                if let Some(&pid) = state.session_pids.lock().unwrap().get(session_id) {
                    found = Some(pid);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            found.expect("session_pids should contain the spawned child's pid")
        };

        // Real end-to-end proof (issue #241): the captured pid resolves, via the
        // actual sysinfo-backed probe, to the real cwd the child was spawned in.
        let resolved = crate::procfs::live_cwds(&[pid])
            .remove(&pid)
            .expect("live_cwds should resolve the running child");
        let resolved_path = std::path::PathBuf::from(resolved);
        let expected = dir
            .path()
            .canonicalize()
            .unwrap_or_else(|_| dir.path().to_path_buf());
        assert_eq!(
            resolved_path.canonicalize().unwrap_or(resolved_path),
            expected
        );

        runtime_task.await.unwrap().unwrap();

        // start_session_runtime's own future resolves once initial setup
        // succeeds, not once the child has exited (matches the established
        // pattern in early_resize_after_start_sets_initial_pty_size_before_spawn
        // above); poll for the async exit-driven cleanup to land.
        let mut still_tracked = true;
        for _ in 0..100 {
            if !state.session_pids.lock().unwrap().contains_key(session_id) {
                still_tracked = false;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            !still_tracked,
            "session_pids entry should be cleared once the process has exited"
        );
    }

    #[tokio::test]
    async fn session_exit_clears_pending_input_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = "runtime-exit-clears-input-buf";
        let state = test_state_with_runtime_session(session_id);

        // A stale, unterminated keystroke left over from before the process exited.
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert(session_id.to_string(), "a".into());

        let (runtime, control_rx) =
            SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
        let output_tx = runtime.output_tx.clone();
        let mut events = output_tx.subscribe();

        let command = harness::CommandSpec {
            program: "/bin/sh".into(),
            args: vec!["-lc".into(), "exit 0".into()],
            cwd: dir.path().display().to_string(),
        };

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.runtime = runtime;
        }

        let (_kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        let _runtime_task = tokio::spawn(start_session_runtime(
            state.clone(),
            session_id.to_string(),
            command,
            None,
            control_rx,
            output_tx,
            kill_rx,
            PtySize {
                rows: DEFAULT_TERMINAL_ROWS,
                cols: DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        ));

        // The event-processing loop (where the input_buf cleanup happens) runs
        // in a task detached from start_session_runtime's own returned future,
        // so wait for the Ended event rather than the outer future resolving.
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match events.recv().await {
                    Ok(RuntimeEvent::Ended { .. }) => break,
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(err) => panic!("unexpected runtime event error: {err}"),
                }
            }
        })
        .await
        .expect("ended event should be emitted for a command that exits immediately");

        assert!(state
            .peon
            .input_buf
            .read()
            .unwrap()
            .get(session_id)
            .is_none());
    }

    #[tokio::test]
    async fn session_exit_persists_and_replays_unterminated_output_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = "runtime-exit-persists-suffix";
        let state = test_state_with_runtime_session(session_id);
        let metadata_root = dir.path().join(".orkworks-test");
        *state.workspace.lock().unwrap() = Some(crate::WorkspaceState {
            path: dir.path().to_path_buf(),
            metadata: crate::metadata::MetadataStore::new(&metadata_root),
            watcher: crate::watcher::MetadataWatcher::start(&metadata_root.join("sessions")),
        });
        let replay_store = crate::metadata::MetadataStore::new(&metadata_root);

        let (runtime, control_rx) =
            SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
        let output_tx = runtime.output_tx.clone();
        let mut events = output_tx.subscribe();
        let command = harness::CommandSpec {
            program: "/bin/sh".into(),
            args: vec!["-lc".into(), "printf 'one\\ntwo\\nthree'".into()],
            cwd: dir.path().display().to_string(),
        };

        {
            let mut sessions = state.sessions.lock().unwrap();
            sessions.get_mut(session_id).unwrap().runtime = runtime;
        }

        let (_kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        tokio::spawn(start_session_runtime(
            state,
            session_id.to_string(),
            command,
            None,
            control_rx,
            output_tx,
            kill_rx,
            PtySize {
                rows: DEFAULT_TERMINAL_ROWS,
                cols: DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        ));

        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match events.recv().await {
                    Ok(RuntimeEvent::Ended { .. }) => break,
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(error) => panic!("unexpected runtime event error: {error}"),
                }
            }
        })
        .await
        .expect("ended event should be emitted for the completed command");

        let replay = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let replay = replay_store.read_terminal_output(session_id, 10);
                if replay.iter().any(|record| record.text() == "three") {
                    break replay;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("persisted replay should contain the final unterminated suffix");

        assert!(replay.iter().any(|record| {
            record == &crate::metadata::TerminalOutputRecord::raw("three", "")
        }));
    }

    #[tokio::test]
    async fn backpressure_flooding_runtime_still_exits_promptly_on_kill() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = "runtime-flood";
        let state = test_state_with_runtime_session(session_id);

        let (runtime, control_rx) =
            SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
        let output_tx = runtime.output_tx.clone();
        let mut events = output_tx.subscribe();

        let command = harness::CommandSpec {
            program: "/bin/sh".into(),
            args: vec![
                "-lc".into(),
                "i=0; while :; do printf 'flood%06d\\n' \"$i\"; i=$((i+1)); done".into(),
            ],
            cwd: dir.path().display().to_string(),
        };

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.runtime = runtime;
        }

        let (kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        let runtime_task = tokio::spawn(start_session_runtime(
            state,
            session_id.to_string(),
            command,
            None,
            control_rx,
            output_tx,
            kill_rx,
            PtySize {
                rows: DEFAULT_TERMINAL_ROWS,
                cols: DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        ));

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                match events.recv().await {
                    Ok(RuntimeEvent::Output { .. }) => break,
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(err) => panic!("unexpected runtime event error before kill: {err}"),
                }
            }
        })
        .await
        .expect("flooding process should emit output quickly");

        kill_tx.send(true).unwrap();

        tokio::time::timeout(Duration::from_secs(3), runtime_task)
            .await
            .expect("kill should stop a flooding runtime promptly")
            .unwrap()
            .unwrap();

        let ended_status = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                match events.recv().await {
                    Ok(RuntimeEvent::Ended { status }) => break status,
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(err) => panic!("unexpected runtime event error after kill: {err}"),
                }
            }
        })
        .await
        .expect("ended event should be emitted after kill");

        assert_eq!(ended_status, "killed");
    }

    #[test]
    fn backpressure_driver_event_channel_is_bounded() {
        let (tx, mut rx) = make_driver_event_channel();

        for _ in 0..DRIVER_EVENT_BUFFER_CAPACITY {
            tx.try_send(DriverEvent::Output(vec![1]))
                .expect("driver queue should accept up to its configured capacity");
        }

        assert!(
            matches!(
                tx.try_send(DriverEvent::Output(vec![2])),
                Err(tokio::sync::mpsc::error::TrySendError::Full(
                    DriverEvent::Output(_)
                ))
            ),
            "driver queue must apply backpressure once full"
        );

        assert!(matches!(rx.try_recv(), Ok(DriverEvent::Output(_))));
    }

    #[test]
    fn backpressure_persist_channel_is_bounded() {
        let (tx, mut rx) = make_persist_channel();

        for _ in 0..PERSIST_QUEUE_CAPACITY {
            tx.try_send(vec!["line".into()])
                .expect("persist queue should accept up to its configured capacity");
        }

        assert!(
            matches!(
                tx.try_send(vec!["overflow".into()]),
                Err(tokio::sync::mpsc::error::TrySendError::Full(_))
            ),
            "persist queue must apply backpressure once full"
        );

        assert!(matches!(rx.try_recv(), Ok(lines) if lines == vec!["line".to_string()]));
    }

    #[test]
    fn persist_records_keep_newline_delimited_output_unchanged() {
        let mut buffer = b"one\r\ntwo\nthree".to_vec();

        assert_eq!(
            drain_persist_records(&mut buffer),
            vec![
                crate::metadata::TerminalOutputRecord::raw("one", "\r\n"),
                crate::metadata::TerminalOutputRecord::raw("two", "\n"),
            ],
        );
        assert_eq!(buffer, b"three");
    }

    #[test]
    fn persist_records_flush_a_newline_free_suffix_at_each_byte_cap() {
        let mut buffer = vec![b'x'; MAX_PARTIAL_PERSIST_BYTES * 2 + 5];

        assert_eq!(
            drain_persist_records(&mut buffer),
            vec![
                crate::metadata::TerminalOutputRecord::raw(
                    "x".repeat(MAX_PARTIAL_PERSIST_BYTES),
                    "",
                ),
                crate::metadata::TerminalOutputRecord::raw(
                    "x".repeat(MAX_PARTIAL_PERSIST_BYTES),
                    "",
                ),
            ],
        );
        assert_eq!(buffer, vec![b'x'; 5]);
    }

    #[test]
    fn persist_records_keep_complete_lines_before_flushing_a_capped_suffix() {
        let mut buffer = b"first\n".to_vec();
        buffer.extend(vec![b'x'; MAX_PARTIAL_PERSIST_BYTES]);

        assert_eq!(
            drain_persist_records(&mut buffer),
            vec!["first".to_string()],
        );
        assert_eq!(buffer, vec![b'x'; MAX_PARTIAL_PERSIST_BYTES]);
    }

    #[test]
    fn persist_records_keep_an_exact_cap_partial_until_its_newline_arrives() {
        let mut buffer = vec![b'x'; MAX_PARTIAL_PERSIST_BYTES];

        assert!(drain_persist_records(&mut buffer).is_empty());
        assert_eq!(buffer, vec![b'x'; MAX_PARTIAL_PERSIST_BYTES]);

        buffer.push(b'\n');
        assert_eq!(
            drain_persist_records(&mut buffer),
            vec!["x".repeat(MAX_PARTIAL_PERSIST_BYTES)],
        );
        assert!(buffer.is_empty());
    }

    #[test]
    fn persist_records_keep_crlf_split_across_chunks_intact() {
        let mut buffer = b"first\r".to_vec();

        assert!(drain_persist_records(&mut buffer).is_empty());
        buffer.extend_from_slice(b"\nsecond\n");
        assert_eq!(
            drain_persist_records(&mut buffer),
            vec!["first".to_string(), "second".to_string()],
        );
        assert!(buffer.is_empty());
    }

    #[test]
    fn persist_records_keep_a_split_utf8_character_for_the_next_chunk() {
        let mut buffer = vec![b'x'; MAX_PARTIAL_PERSIST_BYTES - 1];
        buffer.extend_from_slice(&[0xE2, 0x82]);

        assert_eq!(
            drain_persist_records(&mut buffer),
            vec!["x".repeat(MAX_PARTIAL_PERSIST_BYTES - 1)],
        );
        assert_eq!(buffer, vec![0xE2, 0x82]);

        buffer.push(0xAC);
        assert!(drain_persist_records(&mut buffer).is_empty());
        assert_eq!(String::from_utf8(buffer).unwrap(), "€");
    }

    #[test]
    fn persist_records_keep_a_split_utf8_character_after_invalid_bytes() {
        let mut buffer = vec![0xFF];
        buffer.extend(vec![b'x'; MAX_PARTIAL_PERSIST_BYTES - 3]);
        buffer.extend_from_slice(&[0xE2, 0x82]);

        assert_eq!(drain_persist_records(&mut buffer), Vec::<String>::new(),);
        let mut expected = vec![0xFF];
        expected.extend(vec![b'x'; MAX_PARTIAL_PERSIST_BYTES - 3]);
        expected.extend_from_slice(&[0xE2, 0x82]);
        assert_eq!(buffer, expected);

        buffer.push(0xAC);
        assert_eq!(
            drain_persist_records(&mut buffer),
            vec![format!("�{}", "x".repeat(MAX_PARTIAL_PERSIST_BYTES - 3))],
        );
        assert_eq!(String::from_utf8(buffer).unwrap(), "€");
    }
}
