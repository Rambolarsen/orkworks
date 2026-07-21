use crate::runtime::session_runtime::STARTUP_PENDING_INPUT_BYTES as QUEUED_INPUT_CAP_BYTES;
use crate::session_view::{connectivity_for_status, terminal_outcome_for_status};
use crate::workspace_runtime::iso_now;
use crate::{metadata, peon, providers, AppState};
use axum::extract::ws::{Message, WebSocket};
use std::future::Future;
use std::pin::Pin;
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

/// Buffers terminal actions that arrive while a previous command is still being sent to
/// the PTY runtime, so the websocket read loop can keep polling for Close without silently
/// dropping keystrokes typed during that (normally sub-millisecond) window.
///
/// Input and Resize entries preserve arrival order (the PTY writer applies RuntimeCommands in
/// strict send order, and a full-screen app can misbehave if it sees keys/output at the wrong
/// size), while consecutive same-type pushes still coalesce into one entry. Kill always drains
/// last so nothing typed or resized before it is discarded in favor of ending the session.
#[derive(Debug, PartialEq)]
enum QueuedItem {
    Input(String),
    Resize { rows: u16, cols: u16 },
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct PendingActionQueue {
    items: std::collections::VecDeque<QueuedItem>,
    input_bytes: usize,
    kill: bool,
}

impl PendingActionQueue {
    /// Returns `true` if this action's input was dropped for exceeding the queue's cap.
    pub(crate) fn push(&mut self, action: TerminalAction) -> bool {
        match action {
            TerminalAction::Input(data) => {
                if self.input_bytes + data.len() > QUEUED_INPUT_CAP_BYTES {
                    return true;
                }
                self.input_bytes += data.len();
                if let Some(QueuedItem::Input(existing)) = self.items.back_mut() {
                    existing.push_str(&data);
                } else {
                    self.items.push_back(QueuedItem::Input(data));
                }
                false
            }
            TerminalAction::Resize { rows, cols } => {
                if let Some(QueuedItem::Resize { rows: r, cols: c }) = self.items.back_mut() {
                    *r = rows;
                    *c = cols;
                } else {
                    self.items.push_back(QueuedItem::Resize { rows, cols });
                }
                false
            }
            TerminalAction::Kill => {
                self.kill = true;
                false
            }
            TerminalAction::Noop => false,
        }
    }

    pub(crate) fn take_next(&mut self) -> Option<TerminalAction> {
        if let Some(item) = self.items.pop_front() {
            return Some(match item {
                QueuedItem::Input(data) => {
                    self.input_bytes -= data.len();
                    TerminalAction::Input(data)
                }
                QueuedItem::Resize { rows, cols } => TerminalAction::Resize { rows, cols },
            });
        }
        if self.kill {
            self.kill = false;
            return Some(TerminalAction::Kill);
        }
        None
    }
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

type PendingCommandFuture = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

fn spawn_command_future(
    state: Arc<AppState>,
    id: String,
    action: TerminalAction,
) -> Option<PendingCommandFuture> {
    match action {
        TerminalAction::Input(data) => Some(Box::pin(async move {
            crate::runtime::session_runtime::send_runtime_command(
                &state,
                &id,
                crate::runtime::session_runtime::RuntimeCommand::Input(data),
            ).await
        })),
        TerminalAction::Resize { rows, cols } => Some(Box::pin(async move {
            crate::runtime::session_runtime::update_runtime_size(&state, &id, rows, cols).await
        })),
        TerminalAction::Kill => Some(Box::pin(async move {
            crate::runtime::session_runtime::send_runtime_command(
                &state,
                &id,
                crate::runtime::session_runtime::RuntimeCommand::Kill,
            ).await
        })),
        TerminalAction::Noop => None,
    }
}

fn terminal_input_data(action: &TerminalAction) -> Option<String> {
    match action {
        TerminalAction::Input(data) if !data.is_empty() => Some(data.clone()),
        _ => None,
    }
}

/// Whether the terminal currently looks like it's mid password-prompt. Must
/// be captured before an input is dispatched to the PTY, not after delivery
/// completes — by the time an async send resolves, the child may already
/// have echoed enough new output to scroll the prompt out of the detection
/// window, which would misclassify a submitted secret as safe to persist.
fn snapshot_input_sensitivity(state: &Arc<AppState>, id: &str) -> bool {
    let sessions = state.sessions.lock().unwrap();
    sessions
        .get(id)
        .map(|h| peon::looks_like_password_prompt(&h.output_buffer.last_n(5)))
        .unwrap_or(false)
}

/// The PTY control channel is the delivery authority. Only advance metadata
/// after its command future succeeds; a closed channel means the user input
/// was rejected and must leave any prompt state intact.
fn record_input_after_delivery(
    state: &Arc<AppState>,
    id: &str,
    pending: Option<&(String, bool)>,
    result: &Result<(), ()>,
) {
    if result.is_ok() {
        if let Some((input, is_sensitive)) = pending {
            record_peon_input_side_effects(state, id, input, *is_sensitive);
        }
    }
}

pub(crate) fn collect_input_line(buf: &mut String, data: &str) -> Option<String> {
    let mut result: Option<String> = None;
    let mut chars = data.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' | '\n' => {
                // Full, untruncated line. The only caller (record_terminal_input)
                // truncates it to a display-bounded label before persisting —
                // the full line is not retained anywhere past this point.
                let line = buf.trim().to_string();
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

fn mark_usage_limit_recheck_on_input(state: &Arc<AppState>, id: &str) {
    let mut sessions = state.sessions.lock().unwrap();
    let Some(handle) = sessions.get_mut(id) else {
        return;
    };
    if !handle.at_usage_limit_latched || handle.capacity_check_pending || handle.resume_scan_origin.is_some() {
        return;
    }
    handle.resume_scan_origin = Some((handle.output_lines_seen, handle.scan_bytes_seen));
}

/// Records accepted terminal input for usage-limit rechecks, labels, and pending work signals.
/// Call only once delivery is actually accepted — never for input dropped by
/// `PendingActionQueue`.
fn record_peon_input_side_effects(state: &Arc<AppState>, id: &str, data: &str, is_sensitive: bool) {
    let _ = record_terminal_input_impl(state, id, data, Some(is_sensitive));
}

/// Test-only convenience wrapper that live-checks sensitivity against the
/// current output buffer, bypassing the snapshot-before-dispatch that
/// production callers must use (see `record_peon_input_side_effects`) since
/// the buffer can move on during the real async PTY round-trip.
#[cfg(test)]
pub(crate) fn record_terminal_input(
    state: &Arc<AppState>,
    id: &str,
    data: &str,
) -> Option<()> {
    record_terminal_input_impl(state, id, data, None)
}

fn record_terminal_input_impl(
    state: &Arc<AppState>,
    id: &str,
    data: &str,
    sensitivity_override: Option<bool>,
) -> Option<()> {
    if !data.is_empty() {
        mark_usage_limit_recheck_on_input(state, id);
        mark_committed_input_working(state, id);
    }

    let collected_line = {
        let mut bufs = state.peon.input_buf.write().unwrap();
        let buf = bufs.entry(id.to_string()).or_default();
        collect_input_line(buf, data)
    };

    let line = collected_line?;
    // Labels are display-bounded; echo-gating below uses the full `line`.
    let label_line: String = line.chars().take(100).collect();
    let is_sensitive = sensitivity_override.unwrap_or_else(|| snapshot_input_sensitivity(state, id));
    let label_worthy = !is_sensitive && peon::is_descriptive_input(&label_line);

    if !is_sensitive {
        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(mut meta) = ws.metadata.read_session(id) {
                if label_worthy {
                    meta.label = label_line.clone();
                }
                meta.last_user_input = Some(label_line.clone());
                ws.metadata.write_session(&meta);
            }
        }
    }

    if label_worthy {
        if let Some(handle) = state.sessions.lock().unwrap().get_mut(id) {
            handle.info.label = label_line;
        }
    }

    Some(())
}

/// Caller contract, not enforced here: only call this for a non-empty frame
/// whose delivery to the PTY was actually accepted. That is direct evidence
/// the live session is working, stronger than any stale prompt metadata or
/// later PTY-output heuristic — but this function trusts the caller for it.
fn mark_committed_input_working(state: &Arc<AppState>, id: &str) {
    let already_working = {
        let mut sessions = state.sessions.lock().unwrap();
        let Some(handle) = sessions.get_mut(id) else {
            return;
        };
        if handle.info.lifecycle != "alive" {
            return;
        }
        let already = handle.info.observed_status.as_deref() == Some("working")
            && handle.info.attention.as_deref() == Some("working")
            && handle.info.metadata_source.as_deref() == Some("process")
            && handle.info.metadata_confidence == Some(1.0)
            && handle.info.needs_user_input.is_none()
            && handle.info.detected_question.is_none()
            && handle.info.suggested_options.is_none()
            && handle.pending_work_signal.is_none();
        if !already {
            handle.info.observed_status = Some("working".into());
            handle.info.attention = Some("working".into());
            handle.info.metadata_source = Some("process".into());
            handle.info.metadata_confidence = Some(1.0);
            handle.info.needs_user_input = None;
            handle.info.detected_question = None;
            handle.info.suggested_options = None;
            handle.pending_work_signal = None;
        }
        already
    };

    // Accepted input is activity: refresh the idle baseline regardless of
    // whether metadata needed a rewrite, so a hookless session mid-command
    // doesn't get flagged idle by the next Peon tick (peon_runtime.rs) just
    // because it hasn't produced output since before this input arrived.
    state.peon.last_output.write().unwrap().insert(id.to_string(), tokio::time::Instant::now());

    if already_working {
        return;
    }

    let ws_guard = state.workspace.lock().unwrap();
    let Some(ref ws) = *ws_guard else {
        return;
    };
    let Some(mut meta) = ws.metadata.read_session(id) else {
        return;
    };
    if meta.lifecycle != "alive" {
        return;
    }
    meta.observed_status = Some("working".into());
    meta.attention = Some("working".into());
    meta.metadata_source = "process".into();
    meta.metadata_confidence = 1.0;
    meta.needs_user_input = None;
    meta.detected_question = None;
    meta.suggested_options = None;
    ws.metadata.write_session(&meta);
}

pub(crate) fn should_forward_terminal_env(key: &str) -> bool {
    key != "NODE_OPTIONS"
        && key != "VSCODE_INSPECTOR_OPTIONS"
        && !key.starts_with("VSCODE_")
        && !key.starts_with("ELECTRON_")
}

#[cfg(unix)]
pub(crate) fn make_pty_system() -> UnixPtySystem {
    UnixPtySystem {}
}
#[cfg(windows)]
pub(crate) fn make_pty_system() -> ConPtySystem {
    ConPtySystem {}
}

/// Applies a status transition to the in-memory handle and persisted metadata.
///
/// Terminal statuses ("killed"/"ended"/"error") transition the session into the
/// "ending" lifecycle phase. That transition is applied at most once: repeated
/// terminal calls for a session already in "ending" or "ended" are no-ops, so
/// racing exit paths (e.g. DELETE handler + kill-signal branch) cannot clobber
/// the captured ending snapshot or re-open a finalized session. Returns whether
/// the transition was applied — callers schedule finalization only on `true`.
pub(crate) fn set_session_status(state: &Arc<AppState>, id: &str, status: &str) -> bool {
    let is_terminal = matches!(status, "killed" | "ended" | "error");
    let (handle_decision, session_resume, entered_running, entered_terminal) = {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(id) {
            let entered_running = !is_terminal
                && status == "running"
                && handle.info.status != "running";
            if is_terminal && matches!(handle.info.lifecycle_phase.as_str(), "ending" | "ended") {
                return false;
            }
            if is_terminal {
                handle.info.status = "running".to_string();
                handle.info.lifecycle_phase = "ending".to_string();
                handle.info.lifecycle = "stopping".to_string();
                handle.info.attention = None;
                handle.info.connectivity = Some(connectivity_for_status("running").to_string());
                handle.info.terminal_outcome = None;
            } else {
                handle.info.status = status.to_string();
                handle.info.lifecycle_phase = if status == "creating" {
                    "creating".to_string()
                } else {
                    "active".to_string()
                };
                handle.info.lifecycle = if status == "creating" { "creating" } else { "alive" }.to_string();
                handle.info.connectivity = Some(connectivity_for_status(status).to_string());
                handle.info.terminal_outcome = terminal_outcome_for_status(status);
            }
            handle.info.last_activity_at = Some(iso_now());
            if is_terminal {
                handle.info.observed_status = None;
            }
            (
                Some(true),
                (handle.info.resume.clone(), handle.info.resumed_from.clone()),
                entered_running,
                is_terminal,
            )
        } else {
            (None, (None, None), false, false)
        }
    };
    if entered_terminal {
        state.peon.last_output.write().unwrap().remove(id);
    } else if entered_running && state.peon.config.enabled {
        state.peon.last_output.write().unwrap()
            .entry(id.to_string())
            .or_insert_with(tokio::time::Instant::now);
    }
    let now = iso_now();
    let mut applied = handle_decision.unwrap_or(false);
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some(mut meta) = ws.metadata.read_session(id) {
            // With no in-memory handle, the persisted lifecycle is the guard authority.
            if handle_decision.is_none()
                && is_terminal
                && matches!(meta.lifecycle_phase.as_str(), "ending" | "ended")
            {
                return false;
            }
            applied = true;
            if is_terminal {
                meta.status = "running".to_string();
                meta.lifecycle_phase = "ending".to_string();
                meta.lifecycle = "stopping".to_string();
                meta.attention = None;
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
                meta.lifecycle = if status == "creating" { "creating" } else { "alive" }.to_string();
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
        if applied {
            ws.metadata.append_event(id, &metadata::Event {
                event_type: "session.status".into(),
                timestamp: now,
                status: status.to_string(),
                observed_status: None,
                confidence: None,
            });
        }
    }
    applied
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
        .unwrap_or_else(|| metadata::canonical_null_snapshot("recovery", Some(observed_at.to_string())))
}

/// `fallback_terminal_status` is the terminal status the exit path intended;
/// it is used when metadata is unavailable (no workspace open, file missing),
/// since `pending_terminal_status` is only persisted there.
pub(crate) fn complete_session_ending(
    state: &Arc<AppState>,
    id: &str,
    final_snapshot: metadata::ObservedStatusSnapshotMetadata,
    fallback_terminal_status: &str,
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
                    .unwrap_or_else(|| fallback_terminal_status.into());
                meta.status = pending.clone();
                meta.lifecycle_phase = "ended".into();
                meta.lifecycle = "dead".into();
                meta.attention = None;
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

    let pending = final_status.unwrap_or_else(|| fallback_terminal_status.into());
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(handle) = sessions.get_mut(id) {
        if handle.info.lifecycle_phase == "ended" {
            return;
        }
        handle.info.status = pending.clone();
        handle.info.lifecycle_phase = "ended".into();
        handle.info.lifecycle = "dead".into();
        handle.info.attention = None;
        handle.info.connectivity = Some(connectivity_for_status(&pending).to_string());
        handle.info.terminal_outcome = terminal_outcome_for_status(&pending);
        handle.info.observed_status = None;
        handle.info.final_observed_status = final_snapshot.value.clone();
        handle.info.last_activity_at = Some(now);
    }
}

pub(crate) async fn finalize_session_ending(
    state: Arc<AppState>,
    id: String,
    fallback_terminal_status: String,
) {
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
    let inferred_snapshot =
        final_snapshot_from_inference(scan_result.as_ref().and_then(|result| result.inference.as_ref()), &now);
    let final_snapshot = {
        let ws_guard = state.workspace.lock().unwrap();
        let meta = ws_guard
            .as_ref()
            .and_then(|ws| ws.metadata.read_session(&id));

        if meta.as_ref().is_some_and(|m| m.lifecycle_phase == "ended") {
            return;
        }

        if let (Some(ref ws), Some(ref result)) = (ws_guard.as_ref(), scan_result.as_ref()) {
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

        inferred_snapshot.unwrap_or_else(|| match meta {
            Some(ref meta) => fallback_final_snapshot(meta, &now),
            // Metadata unavailable: still complete the ending so the in-memory
            // session does not stay stuck in the "ending" phase.
            None => metadata::canonical_null_snapshot("recovery", Some(now.clone())),
        })
    };

    complete_session_ending(&state, &id, final_snapshot, &fallback_terminal_status);
}

pub(crate) fn schedule_session_ending_finalization(
    state: Arc<AppState>,
    id: String,
    fallback_terminal_status: String,
) {
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(0)).await;
        finalize_session_ending(state, id, fallback_terminal_status).await;
    });
}

pub(crate) async fn handle_session_terminal(mut ws: WebSocket, id: String, state: Arc<AppState>) {
    let attachment = match crate::runtime::session_runtime::claim_attachment(&state, &id) {
        Some(claim) => claim,
        None => {
            tracing::warn!(session_id = %id, "rejected terminal WebSocket: session unavailable for attach");
            let _ = ws.send(Message::Text(
                serde_json::json!({
                    "type": "terminal-unavailable",
                    "reason": "already-attached"
                }).to_string().into()
            )).await;
            let _ = ws.close().await;
            return;
        }
    };
    let _ = ws.send(Message::Text(
        serde_json::json!({
            "type": "replay-start",
            "cursor": attachment.replay_from,
        }).to_string().into()
    )).await;
    for (_, chunk) in &attachment.replay_chunks {
        if ws.send(Message::Binary(chunk.clone().into())).await.is_err() {
            crate::runtime::session_runtime::release_attachment(&state, &id, attachment.generation);
            let _ = ws.close().await;
            return;
        }
    }
    let _ = ws.send(Message::Text(
        serde_json::json!({
            "type": "replay-end",
            "cursor": attachment.replay_to,
        }).to_string().into()
    )).await;

    let generation = attachment.generation;
    let mut events = attachment.events;
    let mut pending_command: Option<PendingCommandFuture> = None;
    let mut pending_input: Option<(String, bool)> = None;
    let mut queue = PendingActionQueue::default();

    loop {
        tokio::select! {
            result = async {
                pending_command
                    .as_mut()
                    .expect("pending command branch requires a command")
                    .await
            }, if pending_command.is_some() => {
                if result.is_err() {
                    // This future has already been polled to completion —
                    // clear it before breaking so the post-loop drain below
                    // doesn't re-poll an already-resolved future (a panic
                    // for a compiler-generated async-block state machine).
                    pending_command = None;
                    pending_input = None;
                    break;
                }
                record_input_after_delivery(&state, &id, pending_input.as_ref(), &result);
                pending_input = None;
                pending_command = None;
                let next_action = queue.take_next();
                if let Some(action) = next_action {
                    // Sensitivity is captured here, right before dispatch —
                    // not later when the delivery result comes back — so a
                    // password prompt scrolling out of view during the PTY
                    // round-trip can't misclassify a submitted secret.
                    pending_input = terminal_input_data(&action)
                        .map(|data| {
                            let is_sensitive = snapshot_input_sensitivity(&state, &id);
                            (data, is_sensitive)
                        });
                    pending_command = spawn_command_future(state.clone(), id.clone(), action);
                }
            }
            event = events.recv() => {
                match event {
                    Ok(crate::runtime::session_runtime::RuntimeEvent::Output { chunk, .. }) => {
                        if ws.send(Message::Binary(chunk.into())).await.is_err() {
                            break;
                        }
                    }
                    Ok(crate::runtime::session_runtime::RuntimeEvent::Ended { status }) => {
                        let _ = ws.send(Message::Text(
                            serde_json::json!({
                                "type": "ended",
                                "status": status,
                            }).to_string().into()
                        )).await;
                        break;
                    }
                    Ok(crate::runtime::session_runtime::RuntimeEvent::Error { code, message }) => {
                        let _ = ws.send(Message::Text(
                            serde_json::json!({
                                "type": "error",
                                "code": code,
                                "message": message,
                            }).to_string().into()
                        )).await;
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = ws.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let val: serde_json::Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let action = dispatch_terminal_message(&val);

                        if pending_command.is_some() {
                            // A command is already in flight (normally resolves in well under
                            // a millisecond); queue this one instead of dropping it so
                            // keystrokes typed during that window aren't silently lost. See
                            // #159 review finding on terminal_runtime.rs re: parking Close
                            // detection, which is why we don't just `.await` inline here. Peon
                            // side effects for queued input are recorded later, when the
                            // queued action is actually dispatched to the PTY (see the
                            // pending_command resolution branch above) — not here, so
                            // metadata doesn't claim input the PTY never received.
                            if queue.push(action) {
                                tracing::warn!(session_id = %id, "dropped terminal input: queue cap exceeded");
                                let _ = ws.send(Message::Text(
                                    serde_json::json!({ "type": "input-dropped" }).to_string().into()
                                )).await;
                            }
                        } else {
                            // Sensitivity captured before dispatch — see the
                            // comment at the queued-dispatch site above.
                            pending_input = terminal_input_data(&action)
                                .map(|data| {
                                    let is_sensitive = snapshot_input_sensitivity(&state, &id);
                                    (data, is_sensitive)
                                });
                            pending_command = spawn_command_future(state.clone(), id.clone(), action);
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => break,
                }
            }
        }
    }

    // Every exit from the loop above (Close/EOF, Ended, Error, a send
    // failure, or the broadcast channel closing) can race a still-pending
    // PTY command: if it was actually delivered, the session goes on
    // running detached (ADR 0022) and must not be left showing stale
    // `needs_you` just because nobody was left to observe the result. Drain
    // it here instead of dropping it silently at each break site.
    if let Some(cmd) = pending_command.take() {
        let result = cmd.await;
        record_input_after_delivery(&state, &id, pending_input.as_ref(), &result);
    }

    crate::runtime::session_runtime::release_attachment(&state, &id, generation);
    let _ = ws.close().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::{metadata, providers};
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex, RwLock};
    use std::sync::atomic::AtomicU16;

    fn prompted_session_state(session_id: &str) -> (Arc<crate::AppState>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();
        let state = Arc::new(crate::AppState {
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
                input_buf: RwLock::new(HashMap::new()),
                config: crate::peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let mut info = test_session_info(
            session_id.to_string(),
            "Prompted session",
            dir.path().display().to_string(),
            "running",
            "now",
        );
        info.attention = Some("needs_you".into());
        info.observed_status = Some("waiting_for_input".into());
        info.metadata_source = Some("agent".into());
        info.metadata_confidence = Some(1.0);
        info.needs_user_input = Some(true);
        info.detected_question = Some("Proceed?".into());
        info.suggested_options = Some(vec!["yes".into(), "no".into()]);
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.into(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
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

        let mut meta = test_session_metadata(
            session_id,
            "Prompted session",
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
        meta.observed_status = Some("waiting_for_input".into());
        meta.metadata_source = "agent".into();
        meta.metadata_confidence = 1.0;
        meta.needs_user_input = Some(true);
        meta.detected_question = Some("Proceed?".into());
        meta.suggested_options = Some(vec!["yes".into(), "no".into()]);
        state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&meta);

        (state, dir)
    }

    fn assert_prompt_is_cleared_as_working(state: &Arc<crate::AppState>, session_id: &str) {
        let info = state.sessions.lock().unwrap()[session_id].info.clone();
        assert_eq!(info.attention.as_deref(), Some("working"));
        assert_eq!(info.observed_status.as_deref(), Some("working"));
        assert_eq!(info.metadata_source.as_deref(), Some("process"));
        assert_eq!(info.metadata_confidence, Some(1.0));
        assert_eq!(info.needs_user_input, None);
        assert_eq!(info.detected_question, None);
        assert_eq!(info.suggested_options, None);

        let meta = state.workspace.lock().unwrap().as_ref().unwrap()
            .metadata.read_session(session_id).unwrap();
        assert_eq!(meta.attention.as_deref(), Some("working"));
        assert_eq!(meta.observed_status.as_deref(), Some("working"));
        assert_eq!(meta.metadata_source, "process");
        assert_eq!(meta.metadata_confidence, 1.0);
        assert_eq!(meta.needs_user_input, None);
        assert_eq!(meta.detected_question, None);
        assert_eq!(meta.suggested_options, None);
    }

    #[test]
    fn committed_single_key_immediately_clears_prompt_to_working() {
        let session_id = "committed-single-key";
        let (state, _dir) = prompted_session_state(session_id);

        assert_eq!(record_terminal_input(&state, session_id, "y"), None);

        assert_prompt_is_cleared_as_working(&state, session_id);
    }

    #[test]
    fn committed_newline_terminated_input_immediately_clears_prompt_to_working() {
        let session_id = "committed-newline";
        let (state, _dir) = prompted_session_state(session_id);

        assert_eq!(record_terminal_input(&state, session_id, "yes\r"), Some(()));

        assert_prompt_is_cleared_as_working(&state, session_id);
    }

    #[test]
    fn committed_input_refreshes_idle_baseline() {
        let session_id = "committed-idle-baseline";
        let (state, _dir) = prompted_session_state(session_id);
        // Simulate a stale baseline, as if the session had been silent for a
        // while before the user's response arrived.
        state.peon.last_output.write().unwrap().insert(
            session_id.to_string(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(600),
        );
        let stale = *state.peon.last_output.read().unwrap().get(session_id).unwrap();

        record_terminal_input(&state, session_id, "y");

        let refreshed = *state.peon.last_output.read().unwrap().get(session_id).unwrap();
        assert!(
            refreshed > stale,
            "accepted input must refresh the idle baseline, or the next Peon tick can flag a \
             just-resumed hookless session idle before its command produces output"
        );
    }

    #[test]
    fn already_working_input_skips_redundant_metadata_rewrite() {
        let session_id = "already-working-skip";
        let (state, _dir) = prompted_session_state(session_id);
        // First input performs the real transition to working.
        record_terminal_input(&state, session_id, "y");
        assert_prompt_is_cleared_as_working(&state, session_id);

        // Diverge the on-disk record from memory with a canary value that
        // the "already working" fast path does not check against — if the
        // fix regresses to unconditionally rewriting metadata, this canary
        // gets clobbered back to "working" by the second input below.
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = ws.as_ref().unwrap().metadata.read_session(session_id).unwrap();
            meta.attention = Some("idle-canary".into());
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        record_terminal_input(&state, session_id, "z");

        let meta = state.workspace.lock().unwrap().as_ref().unwrap()
            .metadata.read_session(session_id).unwrap();
        assert_eq!(
            meta.attention.as_deref(),
            Some("idle-canary"),
            "input arriving while the in-memory handle already reads \"working\" must not \
             re-read and rewrite persisted metadata"
        );
    }

    #[test]
    fn committed_input_overrides_user_source_and_active_work_hook() {
        let session_id = "committed-user-source";
        let (state, _dir) = prompted_session_state(session_id);
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.info.metadata_source = Some("user".into());
            handle.active_work_hook = true;
        }
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = ws.as_ref().unwrap().metadata.read_session(session_id).unwrap();
            meta.metadata_source = "user".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        assert_eq!(record_terminal_input(&state, session_id, "1"), None);

        assert_prompt_is_cleared_as_working(&state, session_id);
    }

    #[test]
    fn empty_input_does_not_clear_a_prompt() {
        let session_id = "empty-input";
        let (state, _dir) = prompted_session_state(session_id);

        assert_eq!(record_terminal_input(&state, session_id, ""), None);

        let info = state.sessions.lock().unwrap()[session_id].info.clone();
        assert_eq!(info.attention.as_deref(), Some("needs_you"));
        assert_eq!(info.observed_status.as_deref(), Some("waiting_for_input"));
        assert_eq!(info.metadata_source.as_deref(), Some("agent"));
    }

    #[test]
    fn rejected_input_does_not_clear_a_prompt() {
        let session_id = "rejected-input";
        let (state, _dir) = prompted_session_state(session_id);

        record_input_after_delivery(&state, session_id, Some(&("y".to_string(), false)), &Err(()));

        let info = state.sessions.lock().unwrap()[session_id].info.clone();
        assert_eq!(info.attention.as_deref(), Some("needs_you"));
        assert_eq!(info.observed_status.as_deref(), Some("waiting_for_input"));
        assert_eq!(info.metadata_source.as_deref(), Some("agent"));
    }

    #[test]
    fn sensitivity_snapshot_survives_a_moved_on_output_buffer() {
        // Regression for the race this snapshot-before-dispatch design fixes:
        // if a captured pre-dispatch `is_sensitive=true` decision were
        // ignored in favor of re-checking the *current* buffer, a password
        // that already scrolled out of the detection window would be
        // misclassified as safe and persisted in plaintext.
        let session_id = "password-race";
        let (state, _dir) = prompted_session_state(session_id);
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            // The live buffer no longer shows the password prompt — it has
            // already scrolled past by the time this bookkeeping runs.
            handle.output_buffer.push("Login successful".to_string());
            handle.output_buffer.push("$ ".to_string());
        }

        record_peon_input_side_effects(&state, session_id, "hunter2\r", true);

        let ws = state.workspace.lock().unwrap();
        let meta = ws.as_ref().unwrap().metadata.read_session(session_id).unwrap();
        assert_ne!(
            meta.last_user_input.as_deref(),
            Some("hunter2"),
            "a pre-dispatch sensitive decision must not be overridden by a moved-on live buffer"
        );
        assert_ne!(meta.label, "hunter2");
        drop(ws);

        let info = state.sessions.lock().unwrap()[session_id].info.clone();
        assert_ne!(info.label, "hunter2");
    }

    #[test]
    fn non_sensitive_snapshot_still_records_label_and_hint() {
        let session_id = "non-sensitive-input";
        let (state, _dir) = prompted_session_state(session_id);

        record_peon_input_side_effects(&state, session_id, "add retry logic\r", false);

        let ws = state.workspace.lock().unwrap();
        let meta = ws.as_ref().unwrap().metadata.read_session(session_id).unwrap();
        assert_eq!(meta.last_user_input.as_deref(), Some("add retry logic"));
    }

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
    fn pending_action_queue_coalesces_consecutive_input() {
        let mut queue = PendingActionQueue::default();
        assert!(!queue.push(TerminalAction::Input("h".into())));
        assert!(!queue.push(TerminalAction::Input("i".into())));
        assert_eq!(queue.take_next(), Some(TerminalAction::Input("hi".into())));
        assert_eq!(queue.take_next(), None);
    }

    #[test]
    fn pending_action_queue_replaces_resize_with_latest() {
        let mut queue = PendingActionQueue::default();
        queue.push(TerminalAction::Resize { rows: 10, cols: 20 });
        queue.push(TerminalAction::Resize { rows: 30, cols: 40 });
        assert_eq!(queue.take_next(), Some(TerminalAction::Resize { rows: 30, cols: 40 }));
        assert_eq!(queue.take_next(), None);
    }

    #[test]
    fn pending_action_queue_drains_input_and_resize_before_kill_so_nothing_typed_is_lost() {
        let mut queue = PendingActionQueue::default();
        queue.push(TerminalAction::Input("x".into()));
        queue.push(TerminalAction::Resize { rows: 10, cols: 20 });
        queue.push(TerminalAction::Kill);
        assert_eq!(queue.take_next(), Some(TerminalAction::Input("x".into())));
        assert_eq!(queue.take_next(), Some(TerminalAction::Resize { rows: 10, cols: 20 }));
        assert_eq!(queue.take_next(), Some(TerminalAction::Kill));
        assert_eq!(queue.take_next(), None);
    }

    #[test]
    fn pending_action_queue_preserves_arrival_order_across_input_and_resize() {
        // A resize queued before input must still be applied before that input, since the
        // PTY writer applies RuntimeCommands in strict send order and a full-screen app can
        // misbehave if it receives keys/output at the wrong size.
        let mut queue = PendingActionQueue::default();
        queue.push(TerminalAction::Resize { rows: 10, cols: 20 });
        queue.push(TerminalAction::Input("x".into()));
        assert_eq!(queue.take_next(), Some(TerminalAction::Resize { rows: 10, cols: 20 }));
        assert_eq!(queue.take_next(), Some(TerminalAction::Input("x".into())));
        assert_eq!(queue.take_next(), None);
    }

    #[test]
    fn pending_action_queue_coalesces_only_consecutive_same_type_entries() {
        // Input -> Resize -> Input must stay in that order (three items), not merge the two
        // Input pushes together across the intervening Resize.
        let mut queue = PendingActionQueue::default();
        queue.push(TerminalAction::Input("a".into()));
        queue.push(TerminalAction::Resize { rows: 10, cols: 20 });
        queue.push(TerminalAction::Input("b".into()));
        assert_eq!(queue.take_next(), Some(TerminalAction::Input("a".into())));
        assert_eq!(queue.take_next(), Some(TerminalAction::Resize { rows: 10, cols: 20 }));
        assert_eq!(queue.take_next(), Some(TerminalAction::Input("b".into())));
        assert_eq!(queue.take_next(), None);
    }

    #[test]
    fn pending_action_queue_take_next_returns_none_when_empty() {
        let mut queue = PendingActionQueue::default();
        assert_eq!(queue.take_next(), None);
    }

    #[test]
    fn pending_action_queue_drops_input_past_cap_without_growing() {
        let mut queue = PendingActionQueue::default();
        let chunk = "x".repeat(QUEUED_INPUT_CAP_BYTES);
        assert!(!queue.push(TerminalAction::Input(chunk.clone())));
        assert!(queue.push(TerminalAction::Input("overflow".into())));
        assert_eq!(queue.take_next(), Some(TerminalAction::Input(chunk)));
    }

    #[test]
    fn mark_usage_limit_recheck_on_input_sets_origin_once() {
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: crate::peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "codex-wrapper-latched".to_string();
        let mut output_buffer = crate::peon::RingBuffer::new(200);
        output_buffer.push("You've hit your usage limit".into());
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info: crate::test_support::test_session_info(id.clone(), "Codex Wrapper", "/tmp", "running", "now"),
                kill_tx,
                output_buffer,
                scan_buf: "abc".into(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: true,
                capacity_check_pending: false,
                output_lines_seen: 1,
                scan_bytes_seen: 3,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );
        state.sessions.lock().unwrap().get_mut(&id).unwrap().info.harness_id = Some("codex-wrapper".into());

        mark_usage_limit_recheck_on_input(&state, &id);
        let first_origin = state.sessions.lock().unwrap().get(&id).unwrap().resume_scan_origin;
        assert_eq!(first_origin, Some((1, 3)));

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(&id).unwrap();
            handle.output_buffer.push("more output".into());
            handle.scan_buf.push_str("def");
            handle.output_lines_seen += 1;
            handle.scan_bytes_seen += 3;
        }

        mark_usage_limit_recheck_on_input(&state, &id);
        let second_origin = state.sessions.lock().unwrap().get(&id).unwrap().resume_scan_origin;
        assert_eq!(second_origin, first_origin);
    }

    #[test]
    fn set_session_status_updates_registry() {
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
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
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
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
    fn set_session_status_seeds_peon_last_output_when_session_enters_running() {
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: crate::peon::PeonConfig {
                    enabled: true,
                    ..crate::peon::PeonConfig::from_env()
                },
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "running-seed-test".to_string();
        let mut info = test_session_info(id.clone(), "Test", "/tmp", "creating", "now");
        info.lifecycle_phase = "creating".into();
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
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

        assert!(state.peon.last_output.read().unwrap().get(&id).is_none());

        set_session_status(&state, &id, "running");

        assert!(
            state.peon.last_output.read().unwrap().get(&id).is_some(),
            "entering running should seed peon idle timing"
        );
    }

    #[test]
    fn set_session_status_running_does_not_reset_existing_peon_last_output() {
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: crate::peon::PeonConfig {
                    enabled: true,
                    ..crate::peon::PeonConfig::from_env()
                },
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "running-seed-idempotent-test".to_string();
        let mut info = test_session_info(id.clone(), "Test", "/tmp", "running", "now");
        info.lifecycle_phase = "active".into();
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
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

        let seeded_at = tokio::time::Instant::now() - std::time::Duration::from_secs(3);
        state.peon.last_output.write().unwrap().insert(id.clone(), seeded_at);

        set_session_status(&state, &id, "running");

        let actual = *state.peon.last_output.read().unwrap().get(&id).unwrap();
        assert_eq!(actual, seeded_at);
    }

    #[test]
    fn terminal_status_exit_paths_should_transition_through_ending_lifecycle() {
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
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
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
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

        set_session_status(&state, "test-ending", "ended");
        let session = state.sessions.lock().unwrap().get("test-ending").unwrap().info.clone();
        assert_eq!(session.status, "running");
        assert_eq!(session.lifecycle_phase, "ending");
    }

    #[test]
    fn repeated_terminal_status_is_a_noop_and_preserves_the_ending_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
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
                input_buf: RwLock::new(HashMap::new()),
                config: crate::peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let session_id = "ending-idempotent".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = test_session_info(session_id.clone(), "Test", dir.path().display().to_string(), "running", "now");
        info.lifecycle_phase = "active".into();
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
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
                lifecycle_phase: "active".into(),
                lifecycle: "alive".into(),
                attention: None,
                connectivity: "online".into(),
                terminal_outcome: None,
                pending_terminal_status: None,
                observed_status: Some("blocked".into()),
                ending_observed_status_snapshot: None,
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
                metadata_source: "peon".into(),
                metadata_confidence: 0.8,
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

        // First exit path wins and captures the observed status.
        assert!(set_session_status(&state, &session_id, "killed"));
        // A racing exit path (e.g. the kill-signal branch) must not re-snapshot.
        assert!(!set_session_status(&state, &session_id, "killed"));

        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        let meta = ws.metadata.read_session(&session_id).unwrap();
        assert_eq!(meta.lifecycle_phase, "ending");
        assert_eq!(meta.pending_terminal_status.as_deref(), Some("killed"));
        assert_eq!(
            meta.ending_observed_status_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.value.as_deref()),
            Some("blocked")
        );
    }

    #[test]
    fn complete_session_ending_sets_terminal_status_and_clears_pending_state() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
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
                input_buf: RwLock::new(HashMap::new()),
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
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
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
                    lifecycle: "stopping".into(),
                    attention: None,
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
            "killed",
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
                input_buf: RwLock::new(HashMap::new()),
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
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
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
                    lifecycle: "stopping".into(),
                    attention: None,
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

        finalize_session_ending(state.clone(), session_id.clone(), "ended".to_string()).await;

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

    #[test]
    fn collect_input_line_returns_none_until_newline() {
        let mut buf = String::new();
        assert!(collect_input_line(&mut buf, "hel").is_none());
        assert!(collect_input_line(&mut buf, "lo").is_none());
        assert_eq!(collect_input_line(&mut buf, "\r\n").as_deref(), Some("hello"));
        assert!(buf.is_empty());
    }

    #[test]
    fn collect_input_line_strips_trailing_whitespace() {
        let mut buf = String::new();
        assert_eq!(collect_input_line(&mut buf, "cargo build  \n").as_deref(), Some("cargo build"));
    }

    #[test]
    fn collect_input_line_empty_line_is_none() {
        let mut buf = String::new();
        assert!(collect_input_line(&mut buf, "\n").is_none());
    }

}
