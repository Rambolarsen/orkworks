use crate::session_view::{connectivity_for_status, terminal_outcome_for_status};
use crate::workspace_runtime::iso_now;
use crate::{harness, metadata, peon, providers, AppState};
use axum::extract::ws::{Message, WebSocket};
use std::sync::Arc;
use std::time::Duration;

#[cfg(unix)]
use portable_pty::unix::UnixPtySystem;
#[cfg(windows)]
use portable_pty::win::conpty::ConPtySystem;
#[cfg(test)]
use crate::harness_registry::default_shell_command;

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

pub(crate) struct TerminalAttachGuard {
    state: Arc<AppState>,
    session_id: String,
}

impl Drop for TerminalAttachGuard {
    fn drop(&mut self) {
        let mut sessions = self.state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(&self.session_id) {
            handle.terminal_attached = false;
        }
    }
}

pub(crate) fn try_claim_terminal_attachment(
    state: &Arc<AppState>,
    id: &str,
) -> Option<TerminalAttachGuard> {
    let mut sessions = state.sessions.lock().unwrap();
    let handle = sessions.get_mut(id)?;
    let status = handle.info.status.as_str();
    let lifecycle_phase = handle.info.lifecycle_phase.as_str();

    if matches!(status, "killed" | "ended" | "error")
        || matches!(lifecycle_phase, "ending" | "ended")
        || handle.terminal_attached
    {
        return None;
    }

    handle.terminal_attached = true;
    Some(TerminalAttachGuard {
        state: state.clone(),
        session_id: id.to_string(),
    })
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
    let (handle_decision, session_resume, entered_running) = {
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
            (
                Some(true),
                (handle.info.resume.clone(), handle.info.resumed_from.clone()),
                entered_running,
            )
        } else {
            (None, (None, None), false)
        }
    };
    if entered_running && state.peon.config.enabled {
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
    let mut input_buf = String::new();
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

    loop {
        tokio::select! {
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
                        match dispatch_terminal_message(&val) {
                            TerminalAction::Input(data) => {
                                if !data.is_empty() {
                                    mark_usage_limit_recheck_on_input(&state, &id);
                                }

                                let mut triggered_label = false;
                                if let Some(line) = collect_input_line(&mut input_buf, &data) {
                                    let is_sensitive = {
                                        let sessions = state.sessions.lock().unwrap();
                                        sessions.get(&id)
                                            .map(|h| peon::looks_like_password_prompt(&h.output_buffer.last_n(5)))
                                            .unwrap_or(false)
                                    };
                                    let label_worthy = !is_sensitive && peon::is_descriptive_input(&line);
                                    if !is_sensitive {
                                        let ws_guard = state.workspace.lock().unwrap();
                                        if let Some(ref ws) = *ws_guard {
                                            if let Some(mut meta) = ws.metadata.read_session(&id) {
                                                if label_worthy {
                                                    meta.label = line.clone();
                                                }
                                                meta.last_user_input = Some(line.clone());
                                                ws.metadata.write_session(&meta);
                                            }
                                        }
                                    }
                                    if label_worthy {
                                        let mut sessions = state.sessions.lock().unwrap();
                                        if let Some(handle) = sessions.get_mut(&id) {
                                            handle.info.label = line.clone();
                                        }
                                    }
                                    if state.peon.config.enabled && line.len() > 10 && label_worthy {
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
                            TerminalAction::Noop => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => break,
                }
            }
        }
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
    fn terminal_attachment_claim_rejects_duplicate_owner() {
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
        let id = "terminal-owner".to_string();
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info: test_session_info(id.clone(), "Test", "/tmp", "running", "now"),
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
                initial_prompt: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
            },
        );

        let first = try_claim_terminal_attachment(&state, &id);
        assert!(first.is_some());
        let second = try_claim_terminal_attachment(&state, &id);
        assert!(second.is_none());
    }

    #[test]
    fn terminal_attachment_claim_rejects_ending_session() {
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
        let id = "terminal-ending".to_string();
        let mut info = test_session_info(id.clone(), "Test", "/tmp", "running", "now");
        info.lifecycle_phase = "ending".into();
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
                initial_prompt: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
            },
        );

        assert!(try_claim_terminal_attachment(&state, &id).is_none());
    }

    #[test]
    fn terminal_attachment_release_is_owner_scoped() {
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
        let id = "terminal-owner-scope".to_string();
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info: test_session_info(id.clone(), "Test", "/tmp", "running", "now"),
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
                initial_prompt: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
            },
        );

        let first = try_claim_terminal_attachment(&state, &id).unwrap();
        assert!(try_claim_terminal_attachment(&state, &id).is_none());
        assert!(state.sessions.lock().unwrap().get(&id).unwrap().terminal_attached);
        drop(first);
        assert!(!state.sessions.lock().unwrap().get(&id).unwrap().terminal_attached);
    }

    #[test]
    fn mark_usage_limit_recheck_on_input_sets_origin_once() {
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
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
                initial_prompt: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: true,
                capacity_check_pending: false,
                output_lines_seen: 1,
                scan_bytes_seen: 3,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
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
    fn set_session_status_seeds_peon_last_output_when_session_enters_running() {
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
                command: harness::CommandSpec {
                    program: "/bin/sh".into(),
                    args: vec!["-i".into(), "-l".into()],
                    cwd: "/tmp".into(),
                },
                initial_prompt: None,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
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
            session_module: crate::infrastructure::session_module::SessionModule::new(),
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
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
                command: harness::CommandSpec {
                    program: "/bin/sh".into(),
                    args: vec!["-i".into(), "-l".into()],
                    cwd: "/tmp".into(),
                },
                initial_prompt: None,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
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
    fn repeated_terminal_status_is_a_noop_and_preserves_the_ending_snapshot() {
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
                command: crate::harness_registry::default_shell_command(dir.path().display().to_string()),
                initial_prompt: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
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
                lifecycle_phase: "active".into(),
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
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
}
