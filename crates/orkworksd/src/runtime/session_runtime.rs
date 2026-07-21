use crate::runtime::terminal_runtime::{
    codex_thread_id_from_jsonl_line, make_pty_system, schedule_session_ending_finalization,
    session_env_overrides, set_session_status, should_forward_terminal_env, terminal_env_overrides,
};
use crate::workspace_runtime::iso_now;
use crate::{AppState, harness, metadata, peon};
use portable_pty::{CommandBuilder, PtySize, PtySystem};
use std::collections::VecDeque;
use std::io::{Read, Write};
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
    Resize { rows: u16, cols: u16 },
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

    #[cfg(test)]
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

    #[cfg(test)]
    pub(crate) fn detached_test() -> Self {
        Self::detached(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS)
    }

    #[cfg(test)]
    pub(crate) fn attached_generation(&self) -> Option<u64> {
        self.attached_generation
    }

    #[cfg(test)]
    pub(crate) fn last_size(&self) -> (u16, u16) {
        (self.last_rows, self.last_cols)
    }
}

enum DriverEvent {
    Output(Vec<u8>),
    Exited,
    WaitError(String),
}

fn make_driver_event_channel() -> (mpsc::Sender<DriverEvent>, mpsc::Receiver<DriverEvent>) {
    mpsc::channel(DRIVER_EVENT_BUFFER_CAPACITY)
}

fn make_persist_channel() -> (mpsc::Sender<Vec<String>>, mpsc::Receiver<Vec<String>>) {
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

fn drain_persist_records(buffer: &mut Vec<u8>) -> Vec<String> {
    let mut records = Vec::new();

    while let Some(nl) = buffer.iter().position(|&byte| byte == b'\n') {
        let line: Vec<u8> = buffer.drain(..=nl).collect();
        let end = if line.ends_with(b"\r\n") {
            line.len() - 2
        } else {
            line.len() - 1
        };
        records.push(String::from_utf8_lossy(&line[..end]).into_owned());
    }

    while buffer.len() > MAX_PARTIAL_PERSIST_BYTES {
        let flush_end = partial_persist_flush_end(buffer);
        records.push(String::from_utf8_lossy(&buffer[..flush_end]).into_owned());
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
                    if let RuntimeCommand::Input { accepted: Some(accepted), .. } = command {
                        let _ = accepted.send(Err(()));
                    }
                    continue;
                }
                if let RuntimeCommand::Input { data, .. } = &command {
                    let Some(next_input_bytes) = pending_input_bytes.checked_add(data.len()) else {
                        if let RuntimeCommand::Input { accepted: Some(accepted), .. } = command {
                            let _ = accepted.send(Err(()));
                        }
                        continue;
                    };
                    if next_input_bytes > STARTUP_PENDING_INPUT_BYTES {
                        if let RuntimeCommand::Input { accepted: Some(accepted), .. } = command {
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
    let (initial_size, pending_commands) =
        capture_startup_runtime_state(&mut control_rx, initial_size).await;
    let pty_sys = make_pty_system();
    let pair = pty_sys.openpty(initial_size).map_err(|e| e.to_string())?;

    let mut cmd = CommandBuilder::new(&command.program);
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
    let startup_grace_ends_at = tokio::time::Instant::now() + STARTUP_ATTENTION_GRACE;
    // The PTY has spawned, so the lifecycle is alive before either background
    // task can observe and classify its first output chunk.
    set_session_status(&state, &id, "running");

    let mut reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.to_string());
        }
    };
    let writer = match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
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
                    ws.metadata.append_terminal_output_lines(&i, &lines);
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
        let mut pending_persist_batches: VecDeque<Vec<String>> = VecDeque::new();
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

                            let mut codex_thread_id: Option<String> = None;
                            let mut promoted_working = false;
                            {
                                let mut sessions = driver_state.sessions.lock().unwrap();
                                if let Some(handle) = sessions.get_mut(&driver_id) {
                                    for raw in &raw_persist_lines {
                                        let trimmed = raw.trim();
                                        if !trimmed.is_empty() {
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
                                        handle.info.observed_status = Some("working".into());
                                        handle.info.attention = Some("working".into());
                                        promoted_working = true;
                                    }
                                    if handle.info.harness_id.as_deref() == Some("codex") {
                                        codex_thread_id = raw_persist_lines.iter()
                                            .find_map(|line| codex_thread_id_from_jsonl_line(line));
                                    }
                                    let cursor = handle.runtime.replay.push(data.clone());
                                    let _ = handle.runtime.output_tx.send(RuntimeEvent::Output { cursor, chunk: data.clone() });
                                }
                            }

                            if let Some(thread_id) = codex_thread_id {
                                let ws_guard = driver_state.workspace.lock().unwrap();
                                if let Some(ref ws) = *ws_guard {
                                    let report = metadata::HarnessSessionReport {
                                        harness_session_id: thread_id,
                                        source: "codex_jsonl".into(),
                                        confidence: 0.99,
                                    };
                                    let _ = ws.metadata.merge_harness_session_report(&driver_id, &report, &iso_now());
                                }
                            }

                            if driver_state.peon.config.enabled {
                                driver_state.peon.last_output.write().unwrap()
                                    .insert(driver_id.clone(), tokio::time::Instant::now());
                                driver_state.peon.last_inference.write().unwrap().remove(&driver_id);
                            }

                            {
                                let ws_guard = driver_state.workspace.lock().unwrap();
                                if let Some(ref ws) = *ws_guard {
                                    if let Some(mut meta) = ws.metadata.read_session(&driver_id) {
                                        if promoted_working {
                                            meta.observed_status = Some("working".into());
                                            meta.attention = Some("working".into());
                                            meta.metadata_source = "process".into();
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
                                    .push_back(vec![String::from_utf8_lossy(&persist_buffer).into_owned()]);
                            }

                            driver_state.peon.last_output.write().unwrap().remove(&driver_id);
                            driver_state.peon.last_inference.write().unwrap().remove(&driver_id);
                            driver_state.peon.input_buf.write().unwrap().remove(&driver_id);

                            {
                                let mut sessions = driver_state.sessions.lock().unwrap();
                                if let Some(handle) = sessions.get_mut(&driver_id) {
                                    handle.runtime.attached_generation = None;
                                    handle.terminal_attached = false;
                                }
                            }

                            let status = if kill_requested { "killed" } else { "ended" };
                            let _ = driver_output_tx.send(RuntimeEvent::Ended { status: status.to_string() });
                            let _ = set_session_status(&driver_state, &driver_id, status);
                            schedule_session_ending_finalization(
                                driver_state.clone(),
                                driver_id.clone(),
                                status.to_string(),
                            );

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
                                    .push_back(vec![String::from_utf8_lossy(&persist_buffer).into_owned()]);
                            }
                            driver_state.peon.last_output.write().unwrap().remove(&driver_id);
                            driver_state.peon.last_inference.write().unwrap().remove(&driver_id);
                            driver_state.peon.input_buf.write().unwrap().remove(&driver_id);
                            {
                                let mut sessions = driver_state.sessions.lock().unwrap();
                                if let Some(handle) = sessions.get_mut(&driver_id) {
                                    handle.runtime.attached_generation = None;
                                    handle.terminal_attached = false;
                                }
                            }
                            let _ = driver_output_tx.send(RuntimeEvent::Error {
                                code: "pty_wait_failed".into(),
                                message: error,
                            });
                            let _ = set_session_status(&driver_state, &driver_id, "error");
                            schedule_session_ending_finalization(
                                driver_state.clone(),
                                driver_id.clone(),
                                "error".to_string(),
                            );
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
            workspace: Mutex::new(None),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                input_buf: RwLock::new(HashMap::new()),
                config: crate::peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
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
            send_runtime_command(&state, session_id, RuntimeCommand::Input { data: "x".into(), accepted: None })
                .await
                .unwrap();
        }

        // The channel is now full; one more send should not resolve until something drains it.
        let state_clone = state.clone();
        let blocked_send = tokio::spawn(async move {
            send_runtime_command(
                &state_clone,
                session_id,
                RuntimeCommand::Input { data: "overflow".into(), accepted: None },
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
            tx.send(RuntimeCommand::Input { data: chunk.clone(), accepted: None }).await.unwrap();
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
        for status in ["idle", "waiting_for_input", "blocked", "failed", "stale", "done"] {
            assert!(
                !should_infer_working("alive", false, false, past_grace),
                "observer-only output should not resume {status} to working"
            );
        }
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
        assert!(signal.is_none(), "expired signal must be cleared, not rechecked forever");
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
    fn terminal_input_immediately_marks_live_session_working_without_pending_signal() {
        let session_id = "terminal-input-work-signal";
        let state = test_state_with_runtime_session(session_id);

        assert!(crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "fix").is_none());
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
        state.sessions.lock().unwrap()
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

        assert!(crate::runtime::terminal_runtime::record_terminal_input(
            &state,
            session_id,
            "fix"
        )
        .is_none());
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
    fn single_key_does_not_arm_when_needs_you_is_peon_sourced() {
        let session_id = "single-key-not-for-peon-needs-you";
        let state = test_state_with_runtime_session(session_id);

        // Peon scraped the terminal and inferred needs_you; the metadata source
        // is "peon", not "agent" — the narrow-scope gate must exclude it so
        // shell-mode sessions where the terminal echoes each keystroke don't
        // false-positive.
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.info.attention = Some("needs_you".into());
            handle.info.metadata_source = Some("peon".into());
        }

        assert!(
            crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y")
                .is_none()
        );

        let sessions = state.sessions.lock().unwrap();
        assert!(
            sessions[session_id].pending_work_signal.is_none(),
            "Peon-sourced needs_you must not arm via the single-key path"
        );
    }

    #[test]
    fn single_key_does_not_arm_when_active_work_hook_is_true() {
        let session_id = "single-key-not-for-capable-hook";
        let state = test_state_with_runtime_session(session_id);

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.active_work_hook = true;
            handle.info.attention = Some("needs_you".into());
            handle.info.metadata_source = Some("agent".into());
        }

        assert!(
            crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y")
                .is_none()
        );

        let sessions = state.sessions.lock().unwrap();
        assert!(
            sessions[session_id].pending_work_signal.is_none(),
            "capable-hook sessions must not arm via the single-key path (hook-driven only)"
        );
    }

    #[tokio::test]
    async fn single_key_at_hook_sourced_needs_you_is_working_before_visible_output() {
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

        // Accepted input is sufficient evidence of resumed work; no PTY output
        // is needed to clear the prompt.
        assert!(crate::runtime::terminal_runtime::record_terminal_input(
            &state,
            session_id,
            "y"
        )
        .is_none());

        {
            let sessions = state.sessions.lock().unwrap();
            assert_eq!(sessions[session_id].info.attention.as_deref(), Some("working"));
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
        assert_eq!(sessions[session_id].info.attention.as_deref(), Some("working"));
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
        }
        let ws = state.workspace.lock().unwrap();
        let meta = ws.as_ref().unwrap().metadata.read_session(session_id).unwrap();
        assert_eq!(meta.observed_status.as_deref(), Some("working"));
        assert_eq!(meta.attention.as_deref(), Some("working"));
    }

    #[test]
    fn terminal_input_without_observed_status_records_label_and_marks_working() {
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
        assert_eq!(handle.info.label, line);
        assert_eq!(handle.info.attention.as_deref(), Some("working"));
        assert_eq!(handle.info.observed_status.as_deref(), Some("working"));
        drop(sessions);
        assert!(state.peon.label_hint.read().unwrap().get(session_id).is_none());
        assert!(!state.peon.label_pending.read().unwrap().contains(session_id));

        let ws = state.workspace.lock().unwrap();
        let meta = ws.as_ref().unwrap().metadata.read_session(session_id).unwrap();
        assert_eq!(meta.label, line);
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
        assert!(
            session
                .runtime
                .replay
                .snapshot()
                .iter()
                .any(|(_, chunk)| String::from_utf8_lossy(chunk).contains("startup-grace-output"))
        );
        assert!(
            session
                .output_buffer
                .snapshot()
                .iter()
                .any(|line| line.contains("startup-grace-output"))
        );
        assert_ne!(session.info.attention.as_deref(), Some("working"));
        assert_ne!(session.info.observed_status.as_deref(), Some("working"));
        assert!(session.pending_work_signal.is_none());
        drop(handle);

        kill_tx.send(true).unwrap();
    }

    #[tokio::test]
    async fn partial_hookless_terminal_input_immediately_promotes_memory_and_metadata() {
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
            &state,
            session_id,
            "work",
        )
        .is_none());
        send_runtime_command(&state, session_id, RuntimeCommand::Input { data: "work".into(), accepted: None })
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
            state.sessions.lock().unwrap()[session_id].info.observed_status.as_deref(),
            Some("working"),
            "accepted partial terminal input immediately marks the session working"
        );

        assert!(crate::runtime::terminal_runtime::record_terminal_input(
            &state,
            session_id,
            " now\r",
        )
        .is_some());
        send_runtime_command(&state, session_id, RuntimeCommand::Input { data: " now\r".into(), accepted: None })
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
        let meta = ws.as_ref().unwrap().metadata.read_session(session_id).unwrap();
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
        send_runtime_command(&state, session_id, RuntimeCommand::Input { data: "work now\r".into(), accepted: None })
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
        assert_eq!(handle[session_id].info.observed_status.as_deref(), Some("working"));
        assert_eq!(handle[session_id].info.attention.as_deref(), Some("working"));
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
        let mut buffer = b"first\nsecond\r\npartial".to_vec();

        assert_eq!(
            drain_persist_records(&mut buffer),
            vec!["first".to_string(), "second".to_string()],
        );
        assert_eq!(buffer, b"partial");
    }

    #[test]
    fn persist_records_flush_a_newline_free_suffix_at_each_byte_cap() {
        let mut buffer = vec![b'x'; MAX_PARTIAL_PERSIST_BYTES * 2 + 5];

        assert_eq!(
            drain_persist_records(&mut buffer),
            vec![
                "x".repeat(MAX_PARTIAL_PERSIST_BYTES),
                "x".repeat(MAX_PARTIAL_PERSIST_BYTES),
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

        assert_eq!(
            drain_persist_records(&mut buffer),
            Vec::<String>::new(),
        );
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
