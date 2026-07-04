use crate::harness_registry::default_shell_command;
use crate::session_view::{connectivity_for_status, terminal_outcome_for_status};
use crate::workspace_runtime::iso_now;
use crate::{harness, metadata, peon, providers, AppState};
use axum::extract::ws::{Message, WebSocket};
use portable_pty::{CommandBuilder, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

#[cfg(unix)]
use portable_pty::unix::UnixPtySystem;
#[cfg(windows)]
use portable_pty::win::conpty::ConPtySystem;

pub(crate) fn terminal_env_overrides() -> Vec<(String, String)> {
    vec![
        ("TERM".into(), "xterm-256color".into()),
        ("COLORTERM".into(), "truecolor".into()),
        ("FORCE_COLOR".into(), "1".into()),
        ("CLICOLOR".into(), "1".into()),
        ("TERM_PROGRAM".into(), "OrkWorks".into()),
    ]
}

pub(crate) fn session_env_overrides(session_id: &str, port: Option<u16>) -> Vec<(String, String)> {
    let mut env = vec![("ORKWORKS_SESSION_ID".into(), session_id.to_string())];
    if let Some(port) = port {
        env.push(("ORKWORKS_PORT".into(), port.to_string()));
    }
    env
}

pub(crate) fn codex_thread_id_from_jsonl_line(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("type").and_then(|v| v.as_str()) != Some("thread.started") {
        return None;
    }
    value.get("thread_id").and_then(|v| v.as_str()).map(str::to_string)
}

#[derive(Debug, PartialEq)]
pub(crate) enum TerminalAction {
    Input(String),
    Resize { rows: u16, cols: u16 },
    Kill,
    Noop,
}

pub(crate) fn dispatch_terminal_message(msg: &serde_json::Value) -> TerminalAction {
    match msg["type"].as_str() {
        Some("input") => {
            let data = msg["data"].as_str().unwrap_or("").to_string();
            TerminalAction::Input(data)
        }
        Some("resize") => {
            let rows = msg["rows"].as_u64().unwrap_or(24) as u16;
            let cols = msg["cols"].as_u64().unwrap_or(80) as u16;
            TerminalAction::Resize { rows, cols }
        }
        Some("kill") => TerminalAction::Kill,
        _ => TerminalAction::Noop,
    }
}

pub(crate) fn collect_input_line(buf: &mut String, data: &str) -> Option<String> {
    let mut result: Option<String> = None;
    let mut chars = data.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' | '\n' => {
                let raw: String = buf.chars().take(100).collect();
                let line = raw.trim().to_string();
                buf.clear();
                if !line.is_empty() && result.is_none() {
                    result = Some(line);
                }
            }
            '\x7f' => { buf.pop(); }
            '\x03' | '\x04' => { buf.clear(); }
            '\x1b' => {
                match chars.peek().copied() {
                    Some('[') => {
                        // CSI: ESC [ params letter/~
                        chars.next();
                        while let Some(&c) = chars.peek() {
                            chars.next();
                            if c.is_ascii_alphabetic() || c == '~' { break; }
                        }
                    }
                    Some('O') => {
                        // SS3: ESC O letter (arrows/F1-F4 in application cursor mode)
                        chars.next();
                        if chars.peek().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
                            chars.next();
                        }
                    }
                    Some(']') => {
                        // OSC: ESC ] ... BEL or ESC \
                        chars.next();
                        while let Some(c) = chars.next() {
                            if c == '\x07' { break; }
                            if c == '\x1b' {
                                if chars.peek() == Some(&'\\') { chars.next(); }
                                break;
                            }
                        }
                    }
                    Some(_) => { chars.next(); } // alt-key: ESC + one char
                    None => {}                   // bare ESC at end of frame
                }
            }
            ch if !ch.is_ascii_control() => { buf.push(ch); }
            _ => {}
        }
    }
    result
}

pub(crate) fn should_forward_terminal_env(key: &str) -> bool {
    key != "NODE_OPTIONS"
        && key != "VSCODE_INSPECTOR_OPTIONS"
        && !key.starts_with("VSCODE_")
        && !key.starts_with("ELECTRON_")
}

#[cfg(unix)]
fn make_pty_system() -> UnixPtySystem {
    UnixPtySystem {}
}
#[cfg(windows)]
fn make_pty_system() -> ConPtySystem {
    ConPtySystem {}
}

pub(crate) fn set_session_status(state: &Arc<AppState>, id: &str, status: &str) {
    let is_terminal = matches!(status, "killed" | "ended" | "error");
    let session_resume = {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(id) {
            if is_terminal {
                handle.info.status = "running".to_string();
                handle.info.lifecycle_phase = "ending".to_string();
                handle.info.connectivity = Some(connectivity_for_status("running").to_string());
                handle.info.terminal_outcome = None;
            } else {
                handle.info.status = status.to_string();
                handle.info.lifecycle_phase = if status == "creating" {
                    "creating".to_string()
                } else {
                    "active".to_string()
                };
                handle.info.connectivity = Some(connectivity_for_status(status).to_string());
                handle.info.terminal_outcome = terminal_outcome_for_status(status);
            }
            handle.info.last_activity_at = Some(iso_now());
            if is_terminal {
                handle.info.observed_status = None;
            }
            (handle.info.resume.clone(), handle.info.resumed_from.clone())
        } else {
            (None, None)
        }
    };
    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some(mut meta) = ws.metadata.read_session(id) {
            if is_terminal {
                meta.status = "running".to_string();
                meta.lifecycle_phase = "ending".to_string();
                meta.connectivity = connectivity_for_status("running").to_string();
                meta.terminal_outcome = None;
                meta.pending_terminal_status = Some(status.to_string());
                meta.ending_observed_status_snapshot = Some(metadata::ObservedStatusSnapshotMetadata {
                    value: meta.observed_status.clone(),
                    source: meta.metadata_source.clone(),
                    confidence: Some(meta.metadata_confidence),
                    observed_at: Some(now.clone()),
                });
            } else {
                meta.status = status.to_string();
                meta.lifecycle_phase = if status == "creating" {
                    "creating".to_string()
                } else {
                    "active".to_string()
                };
                meta.connectivity = connectivity_for_status(status).to_string();
                meta.terminal_outcome = terminal_outcome_for_status(status);
            }
            meta.last_activity = now.clone();
            if is_terminal {
                meta.observed_status = None;
            }
            if session_resume.0.is_some() {
                meta.resume = session_resume.0;
            }
            if session_resume.1.is_some() {
                meta.resumed_from = session_resume.1;
            }
            ws.metadata.write_session(&meta);
        }
        ws.metadata.append_event(id, &metadata::Event {
            event_type: "session.status".into(),
            timestamp: now,
            status: status.to_string(),
            observed_status: None,
            confidence: None,
        });
    }
}

fn canonical_null_snapshot(
    source: &str,
    observed_at: Option<String>,
) -> metadata::ObservedStatusSnapshotMetadata {
    metadata::ObservedStatusSnapshotMetadata {
        value: None,
        source: source.to_string(),
        confidence: None,
        observed_at,
    }
}

fn final_snapshot_from_inference(
    inference: Option<&peon::PeonInference>,
    observed_at: &str,
) -> Option<metadata::ObservedStatusSnapshotMetadata> {
    inference.and_then(|inf| {
        let value = inf.observed_status.clone()?;
        Some(metadata::ObservedStatusSnapshotMetadata {
        value: Some(value),
        source: "peon".into(),
        confidence: Some(inf.confidence),
        observed_at: Some(observed_at.to_string()),
    })
    })
}

fn fallback_final_snapshot(
    meta: &metadata::SessionMetadata,
    observed_at: &str,
) -> metadata::ObservedStatusSnapshotMetadata {
    meta.ending_observed_status_snapshot
        .clone()
        .or_else(|| meta.final_observed_status_snapshot.clone())
        .unwrap_or_else(|| canonical_null_snapshot("recovery", Some(observed_at.to_string())))
}

pub(crate) fn complete_session_ending(
    state: &Arc<AppState>,
    id: &str,
    final_snapshot: metadata::ObservedStatusSnapshotMetadata,
) {
    let now = iso_now();
    let mut final_status: Option<String> = None;

    {
        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(mut meta) = ws.metadata.read_session(id) {
                if meta.lifecycle_phase == "ended" {
                    return;
                }
                let pending = meta
                    .pending_terminal_status
                    .clone()
                    .unwrap_or_else(|| "error".into());
                meta.status = pending.clone();
                meta.lifecycle_phase = "ended".into();
                meta.connectivity = connectivity_for_status(&pending).to_string();
                meta.terminal_outcome = terminal_outcome_for_status(&pending);
                meta.pending_terminal_status = None;
                meta.ending_observed_status_snapshot = None;
                meta.final_observed_status_snapshot = Some(final_snapshot.clone());
                meta.observed_status = None;
                meta.last_activity = now.clone();
                ws.metadata.write_session(&meta);
                ws.metadata.append_event(id, &metadata::Event {
                    event_type: "session.status".into(),
                    timestamp: now.clone(),
                    status: pending.clone(),
                    observed_status: final_snapshot.value.clone(),
                    confidence: final_snapshot.confidence,
                });
                final_status = Some(pending);
            }
        }
    }

    let pending = final_status.unwrap_or_else(|| "error".into());
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(handle) = sessions.get_mut(id) {
        if handle.info.lifecycle_phase == "ended" {
            return;
        }
        handle.info.status = pending.clone();
        handle.info.lifecycle_phase = "ended".into();
        handle.info.connectivity = Some(connectivity_for_status(&pending).to_string());
        handle.info.terminal_outcome = terminal_outcome_for_status(&pending);
        handle.info.observed_status = None;
        handle.info.final_observed_status = final_snapshot.value.clone();
        handle.info.last_activity_at = Some(now);
    }
}

pub(crate) async fn finalize_session_ending(state: Arc<AppState>, id: String) {
    let output_snapshot = {
        let sessions = state.sessions.lock().unwrap();
        match sessions.get(&id) {
            Some(handle) if handle.info.lifecycle_phase == "ending" => handle.output_buffer.snapshot(),
            _ => return,
        }
    };

    let scan_result = if output_snapshot.is_empty() {
        None
    } else {
        let timeout_secs = state.peon.config.final_scan_timeout_secs;
        let state_clone = state.clone();
        let output_clone = output_snapshot.clone();
        match tokio::task::spawn_blocking(move || {
            state_clone.providers.run_inference_with_timeout(
                providers::PeonScope::Session,
                &output_clone,
                Some(timeout_secs),
            )
        })
        .await
        {
            Ok(result) => Some(result),
            Err(error) => {
                tracing::warn!(session_id = %id, %error, "final peon scan task failed");
                None
            }
        }
    };

    let now = iso_now();
    let final_snapshot = {
        let ws_guard = state.workspace.lock().unwrap();
        let Some(ref ws) = *ws_guard else {
            return;
        };
        let Some(meta) = ws.metadata.read_session(&id) else {
            return;
        };

        if meta.lifecycle_phase == "ended" {
            return;
        }

        if let Some(ref result) = scan_result {
            if let Some(ref observation) = result.observation {
                ws.metadata.persist_provider_context(&id, observation);
            }
            if result.inference.is_none() {
                tracing::warn!(
                    session_id = %id,
                    timeout_secs = state.peon.config.final_scan_timeout_secs,
                    "final peon scan returned no inference; finalizing with fallback snapshot"
                );
            }
        }

        final_snapshot_from_inference(scan_result.as_ref().and_then(|result| result.inference.as_ref()), &now)
            .unwrap_or_else(|| fallback_final_snapshot(&meta, &now))
    };

    complete_session_ending(&state, &id, final_snapshot);
}

pub(crate) fn schedule_session_ending_finalization(state: Arc<AppState>, id: String) {
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(0)).await;
        finalize_session_ending(state, id).await;
    });
}

pub(crate) async fn handle_session_terminal(mut ws: WebSocket, id: String, state: Arc<AppState>) {
    let kill_result = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&id).map(|h| h.kill_tx.subscribe())
    };

    let mut kill_rx = match kill_result {
        Some(rx) => rx,
        None => {
            let _ = ws.close().await;
            return;
        }
    };

    if *kill_rx.borrow() {
        set_session_status(&state, &id, "killed");
        let _ = ws.close().await;
        return;
    }

    {
        let should_reject = {
            let sessions = state.sessions.lock().unwrap();
            sessions
                .get(&id)
                .map(|h| {
                    let s = &h.info.status;
                    s == "killed" || s == "ended" || s == "error"
                })
                .unwrap_or(false)
        };
        if should_reject {
            tracing::warn!(session_id = %id, "rejected terminal WebSocket: session in terminal state");
            let _ = ws.close().await;
            return;
        }
    }

    let cwd = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(&id)
            .map(|h| h.info.cwd.clone())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "/".into())
            })
    };

    let command = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(&id)
            .map(|h| h.command.clone())
            .unwrap_or_else(|| default_shell_command(cwd.clone()))
    };

    let pty_sys = make_pty_system();

    // Wait for the frontend's initial resize message so the PTY opens at the
    // actual terminal dimensions, not the 24x80 fallback.  Without this, the
    // spawned command writes its first prompt / banner at 80 columns while the
    // frontend is already displaying at window width, producing choppy text.
    // Non-resize messages that arrive first (e.g. a keypress racing the resize)
    // are saved and replayed into the main loop after the PTY is ready.
    let mut pending_first_msg: Option<String> = None;
    let (initial_rows, initial_cols) = tokio::select! {
        _ = kill_rx.changed() => {
            if *kill_rx.borrow() {
                set_session_status(&state, &id, "killed");
                schedule_session_ending_finalization(state.clone(), id.clone());
                let _ = ws.close().await;
                return;
            }
            (24u16, 80u16)
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
            (24u16, 80u16)
        }
        msg = ws.recv() => {
            match msg {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                        if val.get("type").and_then(|v| v.as_str()) == Some("resize") {
                            let r = val.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;
                            let c = val.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
                            (r, c)
                        } else {
                            pending_first_msg = Some(text);
                            (24u16, 80u16)
                        }
                    } else {
                        pending_first_msg = Some(text);
                        (24u16, 80u16)
                    }
                }
                Some(Ok(Message::Close(_))) | None => {
                    let _ = ws.close().await;
                    return;
                }
                _ => (24u16, 80u16),
            }
        }
    };

    let pty_size = PtySize {
        rows: initial_rows,
        cols: initial_cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = match pty_sys.openpty(pty_size) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "failed to open PTY");
            set_session_status(&state, &id, "error");
            schedule_session_ending_finalization(state.clone(), id.clone());
            let _ = ws.close().await;
            return;
        }
    };

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
    let port = match state.bound_port.load(Ordering::Relaxed) {
        0 => None,
        value => Some(value),
    };
    for (key, value) in session_env_overrides(&id, port) {
        cmd.env(&key, &value);
    }

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to spawn shell");
            set_session_status(&state, &id, "error");
            schedule_session_ending_finalization(state.clone(), id.clone());
            let _ = ws.close().await;
            return;
        }
    };

    let mut reader = match pair.master.try_clone_reader() {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "failed to clone PTY reader");
            set_session_status(&state, &id, "error");
            schedule_session_ending_finalization(state.clone(), id.clone());
            let _ = ws.close().await;
            return;
        }
    };

    let mut writer = match pair.master.take_writer() {
        Ok(w) => w,
        Err(e) => {
            tracing::error!(error = %e, "failed to take PTY writer");
            set_session_status(&state, &id, "error");
            schedule_session_ending_finalization(state.clone(), id.clone());
            let _ = ws.close().await;
            return;
        }
    };

    set_session_status(&state, &id, "running");

    // Send initial prompt to the PTY if one was set on session creation
    {
        let initial_prompt = {
            let sessions = state.sessions.lock().unwrap();
            sessions.get(&id).and_then(|h| h.initial_prompt.clone())
        };
        if let Some(prompt) = initial_prompt {
            let prompt_bytes = format!("{}\n", prompt).into_bytes();
            if let Err(e) = writer.write_all(&prompt_bytes) {
                tracing::warn!(session_id = %id, error = %e, "failed to write initial prompt");
            }
        }
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let id_for_reader = id.clone();

    tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 4096];
    loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!(session_id = %id_for_reader, error = %e, "PTY read error");
                    break;
                }
            }
        }
    });

    // Serial persistence writer: drains lines from an unbounded channel so that
    // append + trim never race and chunks are persisted in arrival order.
    let (persist_tx, mut persist_rx) =
        tokio::sync::mpsc::unbounded_channel::<Vec<String>>();
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

    // Byte-level buffer of unflushed terminal output. We split on raw '\n' so a
    // chunk that splits a multi-byte UTF-8 sequence or breaks a line in the middle
    // doesn't corrupt persistence: only complete lines are written, the rest
    // stays here until more bytes arrive.
    let mut persist_buffer: Vec<u8> = Vec::new();
    let mut input_buf = String::new();

    if let Some(text) = pending_first_msg {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
            if let TerminalAction::Input(data) = dispatch_terminal_message(&val) {
                let _ = writer.write_all(data.as_bytes());
                let _ = writer.flush();
                if let Some(line) = collect_input_line(&mut input_buf, &data) {
                    let is_sensitive = {
                        let sessions = state.sessions.lock().unwrap();
                        sessions.get(&id)
                            .map(|h| peon::looks_like_password_prompt(&h.output_buffer.last_n(5)))
                            .unwrap_or(false)
                    };
                    if !is_sensitive {
                        let ws_guard = state.workspace.lock().unwrap();
                        if let Some(ref ws) = *ws_guard {
                            if let Some(mut meta) = ws.metadata.read_session(&id) {
                                meta.label = line.clone();
                                meta.last_user_input = Some(line.clone());
                                ws.metadata.write_session(&meta);
                            }
                        }
                    }
                    let mut sessions = state.sessions.lock().unwrap();
                    if let Some(handle) = sessions.get_mut(&id) {
                        if !is_sensitive {
                            handle.info.label = line.clone();
                        }
                        if peon::is_terminal_observed_status(handle.info.observed_status.as_deref()) {
                            handle.info.observed_status = None;
                        }
                    }
                    drop(sessions);
                    // Clear stale idle/done/stale state in metadata when user types.
                    {
                        let ws_guard = state.workspace.lock().unwrap();
                        if let Some(ref ws) = *ws_guard {
                            if let Some(mut meta) = ws.metadata.read_session(&id) {
                                if peon::is_terminal_observed_status(meta.observed_status.as_deref()) {
                                    meta.observed_status = None;
                                    meta.metadata_source = "process".into();
                                    ws.metadata.write_session(&meta);
                                }
                            }
                        }
                    }
                    if state.peon.config.enabled && line.len() > 10 && !is_sensitive {
                        state.peon.label_hint.write().unwrap().insert(id.clone(), line);
                        state.peon.label_pending.write().unwrap().insert(id.clone());
                    }
                }
                if state.peon.config.enabled && !data.is_empty() {
                    state.peon.last_output.write().unwrap()
                        .insert(id.clone(), tokio::time::Instant::now());
                    state.peon.last_inference.write().unwrap().remove(&id);
                }
            }
        }
    }

    loop {
        tokio::select! {
            _ = kill_rx.changed() => {
                if *kill_rx.borrow() {
                    tracing::info!(session_id = %id, "kill signal received");
                    let _ = child.kill();
                    set_session_status(&state, &id, "killed");
                    schedule_session_ending_finalization(state.clone(), id.clone());
                    break;
                }
            }
            data = rx.recv() => {
                match data {
                    Some(data) => {
                persist_buffer.extend_from_slice(&data);

                let mut raw_persist_lines: Vec<String> = Vec::new();
                while let Some(nl) = persist_buffer.iter().position(|&b| b == b'\n') {
                    let line: Vec<u8> = persist_buffer.drain(..=nl).collect();
                    // Strip the trailing \n (and a preceding \r if present) so persisted
                    // lines are bare content; replay re-adds line terminators.
                    let end = if line.ends_with(b"\r\n") {
                        line.len() - 2
                    } else {
                        line.len() - 1
                    };
                    raw_persist_lines.push(String::from_utf8_lossy(&line[..end]).into_owned());
                }

                let mut codex_thread_id: Option<String> = None;
                if state.peon.config.enabled {
                    let mut sessions = state.sessions.lock().unwrap();
                    if let Some(handle) = sessions.get_mut(&id) {
                        for raw in &raw_persist_lines {
                            let trimmed = raw.trim();
                            if !trimmed.is_empty() {
                                handle.output_buffer.push(trimmed.to_string());
                            }
                        }
                        // Also feed raw PTY chunk into scan_buf for TUI apps (cursor-positioned, no newlines).
                        let stripped = peon::strip_ansi(&String::from_utf8_lossy(&data));
                        handle.scan_buf.push_str(&stripped);
                        const MAX_SCAN: usize = 8192;
                        if handle.scan_buf.len() > MAX_SCAN {
                            let drop = handle.scan_buf.len() - MAX_SCAN;
                            let drop = (drop..drop + 4).find(|&i| handle.scan_buf.is_char_boundary(i)).unwrap_or(drop);
                            handle.scan_buf.drain(..drop);
                        }
                        if peon::is_terminal_observed_status(handle.info.observed_status.as_deref()) {
                            handle.info.observed_status = None;
                        }
                        if handle.info.harness_id.as_deref() == Some("codex") {
                            codex_thread_id = raw_persist_lines.iter()
                                .find_map(|line| codex_thread_id_from_jsonl_line(line));
                        }
                    }
                }

                if let Some(thread_id) = codex_thread_id {
                    let ws_guard = state.workspace.lock().unwrap();
                    if let Some(ref ws) = *ws_guard {
                        let report = metadata::HarnessSessionReport {
                            harness_session_id: thread_id,
                            source: "codex_jsonl".into(),
                            confidence: 0.99,
                        };
                        let _ = ws.metadata.merge_harness_session_report(&id, &report, &iso_now());
                    }
                }

                // Any PTY traffic at all counts as activity for Peon debounce —
                // including chunks that haven't yet completed a line.
                if state.peon.config.enabled {
                    state.peon.last_output.write().unwrap()
                        .insert(id.clone(), tokio::time::Instant::now());
                    state.peon.last_inference.write().unwrap().remove(&id);
                }

                // New terminal output means the session is no longer idle.
                // Clear any stale terminal observed status in metadata.
                {
                    let ws_guard = state.workspace.lock().unwrap();
                    if let Some(ref ws) = *ws_guard {
                        if let Some(mut meta) = ws.metadata.read_session(&id) {
                            if peon::is_terminal_observed_status(meta.observed_status.as_deref()) {
                                meta.observed_status = None;
                                meta.metadata_source = "process".into();
                                ws.metadata.write_session(&meta);
                            }
                        }
                    }
                }

                if !raw_persist_lines.is_empty() {
                    let _ = persist_tx.send(raw_persist_lines);
                }

                if ws.send(Message::Binary(data)).await.is_err() {
                    break;
                }
                    }
                    None => {
                        // PTY reader channel closed: child process exited (e.g. user typed "exit").
                        // Reap the child and clean up so the frontend's WebSocket onclose fires.
                        let _ = child.kill();
                        set_session_status(&state, &id, "ended");
                        schedule_session_ending_finalization(state.clone(), id.clone());
                        break;
                    }
                }
            }
            msg = ws.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let val: serde_json::Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        match dispatch_terminal_message(&val) {
                            TerminalAction::Input(data) => {
                                let _ = writer.write_all(data.as_bytes());
                                let _ = writer.flush();

                                let mut triggered_label = false;
                                if let Some(line) = collect_input_line(&mut input_buf, &data) {
                                    let is_sensitive = {
                                        let sessions = state.sessions.lock().unwrap();
                                        sessions.get(&id)
                                            .map(|h| peon::looks_like_password_prompt(&h.output_buffer.last_n(5)))
                                            .unwrap_or(false)
                                    };
                                    if !is_sensitive {
                                        let ws_guard = state.workspace.lock().unwrap();
                                        if let Some(ref ws) = *ws_guard {
                                            if let Some(mut meta) = ws.metadata.read_session(&id) {
                                                meta.label = line.clone();
                                                meta.last_user_input = Some(line.clone());
                                                ws.metadata.write_session(&meta);
                                            }
                                        }
                                    }
                                    {
                                        let mut sessions = state.sessions.lock().unwrap();
                                        if let Some(handle) = sessions.get_mut(&id) {
                                            if !is_sensitive {
                                                handle.info.label = line.clone();
                                            }
                                        }
                                    }
                                    if state.peon.config.enabled && line.len() > 10 && !is_sensitive {
                                        state.peon.label_hint.write().unwrap().insert(id.clone(), line);
                                        state.peon.label_pending.write().unwrap().insert(id.clone());
                                        triggered_label = true;
                                    }
                                }

                                if state.peon.config.enabled && !data.is_empty() {
                                    let ts = if triggered_label {
                                        tokio::time::Instant::now() - std::time::Duration::from_secs(3600)
                                    } else {
                                        tokio::time::Instant::now()
                                    };
                                    state.peon.last_output.write().unwrap().insert(id.clone(), ts);
                                    state.peon.last_inference.write().unwrap().remove(&id);
                                }
                            }
                            TerminalAction::Resize { rows, cols } => {
                                if let Err(e) = pair.master.resize(PtySize {
                                    rows,
                                    cols,
                                    pixel_width: 0,
                                    pixel_height: 0,
                                }) {
                                    tracing::warn!(error = %e, "PTY resize error");
                                }
                            }
                            TerminalAction::Kill => {
                                tracing::info!(session_id = %id, "kill message received");
                                let _ = child.kill();
                                set_session_status(&state, &id, "killed");
                                schedule_session_ending_finalization(state.clone(), id.clone());
                                break;
                            }
                            TerminalAction::Noop => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        let _ = child.kill();
                        if *kill_rx.borrow() {
                            set_session_status(&state, &id, "killed");
                        } else {
                            set_session_status(&state, &id, "ended");
                        }
                        schedule_session_ending_finalization(state.clone(), id.clone());
                        break;
                    }
                    _ => {
                        let _ = child.kill();
                        set_session_status(&state, &id, "error");
                        schedule_session_ending_finalization(state.clone(), id.clone());
                        break;
                    }
                }
            }
        }
    }

    {
        // Flush any unterminated tail so the user's last visible line survives.
        if !persist_buffer.is_empty() {
            let tail = String::from_utf8_lossy(&persist_buffer).into_owned();
            let _ = persist_tx.send(vec![tail]);
        }
        // Close the channel and let the serial writer drain all pending appends
        // before the trim runs, so trimming never races with a write.
        drop(persist_tx);
        let _ = persist_writer.await;

        state.peon.last_output.write().unwrap().remove(&id);
        state.peon.last_inference.write().unwrap().remove(&id);

        let state_clone = state.clone();
        let id_clone = id.clone();
        tokio::task::spawn_blocking(move || {
            let ws_guard = state_clone.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                ws.metadata.trim_terminal_output(&id_clone, metadata::TERMINAL_OUTPUT_MAX_LINES);
            }
        });
    }

    tracing::info!(session_id = %id, "session terminal ended");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::{metadata, providers};
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex, RwLock};
    use std::sync::atomic::AtomicU16;

    #[test]
    fn terminal_env_overrides_force_color_capability() {
        let overrides = terminal_env_overrides();

        assert!(overrides.contains(&("TERM".into(), "xterm-256color".into())));
        assert!(overrides.contains(&("COLORTERM".into(), "truecolor".into())));
        assert!(overrides.contains(&("FORCE_COLOR".into(), "1".into())));
        assert!(overrides.contains(&("CLICOLOR".into(), "1".into())));
        assert!(overrides.contains(&("TERM_PROGRAM".into(), "OrkWorks".into())));
    }

    #[test]
    fn session_env_overrides_include_orkworks_session_and_port() {
        let overrides = session_env_overrides("session-123", Some(5173));
        assert!(overrides.contains(&("ORKWORKS_SESSION_ID".into(), "session-123".into())));
        assert!(overrides.contains(&("ORKWORKS_PORT".into(), "5173".into())));
    }

    #[test]
    fn session_env_overrides_omit_port_when_unknown() {
        let overrides = session_env_overrides("session-123", None);
        assert!(overrides.contains(&("ORKWORKS_SESSION_ID".into(), "session-123".into())));
        assert!(!overrides.iter().any(|(key, _)| key == "ORKWORKS_PORT"));
    }

    #[test]
    fn opencode_reporter_script_posts_native_session_env() {
        let script = include_str!("../../scripts/report-opencode-session.sh");
        assert!(script.contains("OPENCODE_SESSION_ID"));
        assert!(script.contains("ORKWORKS_SESSION_ID"));
        assert!(script.contains("ORKWORKS_PORT"));
        assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/harness-session"));
        assert!(script.contains("\"source\":\"opencode_env\""));
    }

    #[test]
    fn opencode_reporter_script_bounds_curl_with_a_timeout() {
        let script = include_str!("../../scripts/report-opencode-session.sh");
        assert!(
            script.contains("--max-time"),
            "reporter must cap curl so a stuck orkworksd cannot hang the harness hook"
        );
    }

    #[test]
    fn codex_jsonl_parser_extracts_thread_started_id() {
        let line = r#"{"type":"thread.started","thread_id":"0199a213-81c0-7800-8aa1-bbab2a035a53"}"#;
        assert_eq!(
            codex_thread_id_from_jsonl_line(line).as_deref(),
            Some("0199a213-81c0-7800-8aa1-bbab2a035a53"),
        );
    }

    #[test]
    fn codex_jsonl_parser_ignores_other_events() {
        let line = r#"{"type":"turn.started"}"#;
        assert_eq!(codex_thread_id_from_jsonl_line(line), None);
    }

    #[test]
    fn claude_hook_reporter_extracts_session_id_and_posts() {
        let script = include_str!("../../scripts/report-claude-session-from-hook.sh");
        assert!(script.contains("session_id"));
        assert!(script.contains("ORKWORKS_SESSION_ID"));
        assert!(script.contains("ORKWORKS_PORT"));
        assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/harness-session"));
        assert!(script.contains("\"source\":\"claude_hook\""));
        assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/attention"));
    }

    #[test]
    fn claude_hook_reporter_script_bounds_both_curls_with_a_timeout() {
        let script = include_str!("../../scripts/report-claude-session-from-hook.sh");
        let max_time_count = script.matches("--max-time").count();
        assert_eq!(
            max_time_count, 2,
            "both curl calls must cap their own runtime so a stuck orkworksd cannot hang \
             the UserPromptSubmit/Notification hook until the harness's own default timeout"
        );
    }

    #[test]
    fn terminal_env_filter_removes_launcher_debug_variables() {
        assert!(!should_forward_terminal_env("NODE_OPTIONS"));
        assert!(!should_forward_terminal_env("VSCODE_INSPECTOR_OPTIONS"));
        assert!(!should_forward_terminal_env("VSCODE_PID"));
        assert!(!should_forward_terminal_env("ELECTRON_RUN_AS_NODE"));
    }

    #[test]
    fn terminal_env_filter_keeps_normal_shell_variables() {
        assert!(should_forward_terminal_env("PATH"));
        assert!(should_forward_terminal_env("HOME"));
        assert!(should_forward_terminal_env("SHELL"));
        assert!(should_forward_terminal_env("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn terminal_message_dispatches_kill() {
        let msg = serde_json::json!({"type": "kill"});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Kill);
    }

    #[test]
    fn terminal_message_dispatches_input() {
        let msg = serde_json::json!({"type": "input", "data": "hello"});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Input("hello".into()));
    }

    #[test]
    fn terminal_message_dispatches_resize() {
        let msg = serde_json::json!({"type": "resize", "rows": 40, "cols": 120});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Resize { rows: 40, cols: 120 });
    }

    #[test]
    fn terminal_message_dispatches_unknown_as_noop() {
        let msg = serde_json::json!({"type": "unknown"});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Noop);
    }

    #[test]
    fn set_session_status_updates_registry() {
        let state = Arc::new(crate::AppState {
            session_module: crate::infrastructure::session_module::SessionModule::new(),
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                config: crate::peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "test-2".to_string();
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info: test_session_info(id.clone(), "Test", "/tmp", "creating", "now"),
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
                initial_prompt: None,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
            },
        );

        set_session_status(&state, "test-2", "running");
        assert_eq!(
            state
                .sessions
                .lock()
                .unwrap()
                .get("test-2")
                .unwrap()
                .info
                .status,
            "running"
        );

        set_session_status(&state, "test-2", "ended");
        let ended = state
            .sessions
            .lock()
            .unwrap()
            .get("test-2")
            .unwrap()
            .info
            .clone();
        assert_eq!(ended.status, "running");
        assert_eq!(ended.lifecycle_phase, "ending");
    }

    #[test]
    fn terminal_status_exit_paths_should_transition_through_ending_lifecycle() {
        let state = Arc::new(crate::AppState {
            session_module: crate::infrastructure::session_module::SessionModule::new(),
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                config: crate::peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "test-ending".to_string();
        let mut info = test_session_info(id.clone(), "Test", "/tmp", "running", "now");
        info.lifecycle_phase = "active".into();
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
                initial_prompt: None,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
            },
        );

        set_session_status(&state, "test-ending", "ended");
        let session = state.sessions.lock().unwrap().get("test-ending").unwrap().info.clone();
        assert_eq!(session.status, "running");
        assert_eq!(session.lifecycle_phase, "ending");
    }

    #[test]
    fn complete_session_ending_sets_terminal_status_and_clears_pending_state() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            session_module: crate::infrastructure::session_module::SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                config: crate::peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let session_id = "ending-complete".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = test_session_info(session_id.clone(), "Test", dir.path().display().to_string(), "running", "now");
        info.lifecycle_phase = "ending".into();
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                command: crate::harness_registry::default_shell_command(dir.path().display().to_string()),
                initial_prompt: None,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
            },
        );

        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.write_session(&metadata::SessionMetadata {
                id: session_id.clone(),
                label: "Test".into(),
                workspace: dir.path().display().to_string(),
                task: "".into(),
                harness: "".into(),
                model: "".into(),
                cwd: dir.path().display().to_string(),
                status: "running".into(),
                work_phase: "unknown".into(),
                lifecycle_phase: "ending".into(),
                connectivity: "online".into(),
                terminal_outcome: None,
                pending_terminal_status: Some("killed".into()),
                observed_status: None,
                ending_observed_status_snapshot: Some(metadata::ObservedStatusSnapshotMetadata {
                    value: Some("blocked".into()),
                    source: "peon".into(),
                    confidence: Some(0.6),
                    observed_at: Some("before".into()),
                }),
                final_observed_status_snapshot: None,
                summary: None,
                next_action: None,
                needs_user_input: None,
                detected_question: None,
                suggested_options: None,
                blocker_description: None,
                failed_command: None,
                failed_test: None,
                capacity_hints: None,
                peon_last_inference: None,
                provider_id: None,
                provider_label: None,
                provider_model: None,
                provider_state: None,
                created_at: "now".into(),
                last_activity: "now".into(),
                metadata_source: "process".into(),
                metadata_confidence: 1.0,
                repo_root: None,
                branch: None,
                dirty: None,
                changed_files: None,
                is_worktree: None,
                resume: None,
                resume_options: vec![],
                harness_session_id_source: None,
                harness_session_id_confidence: None,
                harness_session_id_captured_at: None,
                resumed_from: None,
                last_user_input: None,
            });
        }

        complete_session_ending(
            &state,
            &session_id,
            metadata::ObservedStatusSnapshotMetadata {
                value: Some("done".into()),
                source: "peon".into(),
                confidence: Some(0.91),
                observed_at: Some("after".into()),
            },
        );

        let session = state.sessions.lock().unwrap().get(&session_id).unwrap().info.clone();
        assert_eq!(session.status, "killed");
        assert_eq!(session.lifecycle_phase, "ended");
        assert_eq!(session.terminal_outcome.as_deref(), Some("killed"));
        assert_eq!(session.final_observed_status.as_deref(), Some("done"));

        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        let meta = ws.metadata.read_session(&session_id).unwrap();
        assert_eq!(meta.status, "killed");
        assert_eq!(meta.lifecycle_phase, "ended");
        assert_eq!(meta.pending_terminal_status, None);
        assert_eq!(meta.ending_observed_status_snapshot, None);
        assert_eq!(
            meta.final_observed_status_snapshot.unwrap().value.as_deref(),
            Some("done")
        );
    }

    #[tokio::test]
    async fn finalize_session_ending_times_out_to_fallback_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let mut config = crate::peon::PeonConfig::from_env();
        config.final_scan_timeout_secs = 0;

        let state = Arc::new(crate::AppState {
            session_module: crate::infrastructure::session_module::SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                config,
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".to_string(),
                        enabled: true,
                        fallback_order: 0,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode")
                    .sleep_ms(50)
                    .stdout(r#"{"observedStatus":"done","confidence":0.9}"#)],
            ),
        });

        let session_id = "ending-timeout".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = test_session_info(session_id.clone(), "Test", dir.path().display().to_string(), "running", "now");
        info.lifecycle_phase = "ending".into();
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                command: crate::harness_registry::default_shell_command(dir.path().display().to_string()),
                initial_prompt: None,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
            },
        );
        state.sessions.lock().unwrap().get_mut(&session_id).unwrap().output_buffer.push("final line".into());

        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.write_session(&metadata::SessionMetadata {
                id: session_id.clone(),
                label: "Test".into(),
                workspace: dir.path().display().to_string(),
                task: "".into(),
                harness: "".into(),
                model: "".into(),
                cwd: dir.path().display().to_string(),
                status: "running".into(),
                work_phase: "unknown".into(),
                lifecycle_phase: "ending".into(),
                connectivity: "online".into(),
                terminal_outcome: None,
                pending_terminal_status: Some("ended".into()),
                observed_status: None,
                ending_observed_status_snapshot: Some(metadata::ObservedStatusSnapshotMetadata {
                    value: Some("blocked".into()),
                    source: "peon".into(),
                    confidence: Some(0.75),
                    observed_at: Some("before".into()),
                }),
                final_observed_status_snapshot: None,
                summary: None,
                next_action: None,
                needs_user_input: None,
                detected_question: None,
                suggested_options: None,
                blocker_description: None,
                failed_command: None,
                failed_test: None,
                capacity_hints: None,
                peon_last_inference: None,
                provider_id: None,
                provider_label: None,
                provider_model: None,
                provider_state: None,
                created_at: "now".into(),
                last_activity: "now".into(),
                metadata_source: "process".into(),
                metadata_confidence: 1.0,
                repo_root: None,
                branch: None,
                dirty: None,
                changed_files: None,
                is_worktree: None,
                resume: None,
                resume_options: vec![],
                harness_session_id_source: None,
                harness_session_id_confidence: None,
                harness_session_id_captured_at: None,
                resumed_from: None,
                last_user_input: None,
            });
        }

        finalize_session_ending(state.clone(), session_id.clone()).await;

        let session = state.sessions.lock().unwrap().get(&session_id).unwrap().info.clone();
        assert_eq!(session.status, "ended");
        assert_eq!(session.lifecycle_phase, "ended");
        assert_eq!(session.final_observed_status.as_deref(), Some("blocked"));

        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        let meta = ws.metadata.read_session(&session_id).unwrap();
        let snapshot = meta.final_observed_status_snapshot.unwrap();
        assert_eq!(meta.status, "ended");
        assert_eq!(snapshot.value.as_deref(), Some("blocked"));
        assert_eq!(snapshot.source, "peon");
    }
}
