use crate::workspace_runtime::iso_now;
use crate::{metadata, peon, providers, AppState};
use std::sync::Arc;

pub(crate) async fn peon_loop(state: Arc<AppState>) {
    let interval = state.peon.config.interval_secs;
    tracing::info!(interval_secs = interval, harness = %state.peon.config.harness, "peon started");

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let now = tokio::time::Instant::now();
        let deadline = now - std::time::Duration::from_secs(interval);

        // Sessions with a pending label inference (input-triggered, no debounce)
        let pending: Vec<String> = state.peon.label_pending.write().unwrap().drain().collect();

        // Sessions with new output that has gone silent
        let mut candidates: Vec<String> = {
            let last_output = state.peon.last_output.read().unwrap();
            let in_flight = state.peon.in_flight.read().unwrap();
            let sessions = state.sessions.lock().unwrap();

            last_output.iter()
                .filter(|(id, &t)| {
                    t <= deadline
                        && !in_flight.contains(*id)
                        && sessions
                            .get(*id)
                            .map(|handle| handle.info.lifecycle_phase == "active")
                            .unwrap_or(false)
                })
                .map(|(id, _)| id.clone())
                .collect()
        };

        for id in pending {
            if !state.peon.in_flight.read().unwrap().contains(&id) && !candidates.contains(&id) {
                candidates.push(id);
            }
        }

        for session_id in candidates {
            {
                let mut in_flight = state.peon.in_flight.write().unwrap();
                if !in_flight.insert(session_id.clone()) {
                    continue;
                }
            }

            let hint = state.peon.label_hint.write().unwrap().remove(&session_id);
            let output_snapshot = {
                let sessions = state.sessions.lock().unwrap();
                match sessions.get(&session_id) {
                    Some(handle) => handle.output_buffer.snapshot(),
                    None => {
                        state.peon.in_flight.write().unwrap().remove(&session_id);
                        continue;
                    }
                }
            };

            if output_snapshot.is_empty() && hint.is_none() {
                state.peon.in_flight.write().unwrap().remove(&session_id);
                continue;
            }

            let output_snapshot = if let Some(ref h) = hint {
                let mut lines = vec![format!("[User input]: {}", h)];
                lines.extend(output_snapshot);
                lines
            } else {
                output_snapshot
            };

            let state_clone = state.clone();
            let id = session_id.clone();

            tokio::task::spawn_blocking(move || {
                let provider_result = state_clone.providers.run_inference(providers::PeonScope::Session, &output_snapshot);
                let inference = provider_result.inference;
                let now_iso = iso_now();

                // Check terminal status before moving inference below
                let reached_terminal = matches!(
                    inference.as_ref().and_then(|inf| inf.observed_status.as_deref()),
                    Some("done" | "idle" | "stale")
                );

                if let Some(ref obs) = provider_result.observation {
                    let ws_guard = state_clone.workspace.lock().unwrap();
                    if let Some(ref ws) = *ws_guard {
                        ws.metadata.persist_provider_context(&id, obs);
                    }
                }

                let mut inference_persisted = false;
                let mut permanent_hold = false;
                if let Some(inf) = inference {
                    // Collect label update while holding workspace lock, then drop before taking sessions.
                    let label_update: Option<String> = {
                        let ws_guard = state_clone.workspace.lock().unwrap();
                        if let Some(ref ws) = *ws_guard {
                            let (should_write, is_permanent) = ws.metadata.read_session(&id)
                                .map(|m| {
                                    let age = ws.metadata.session_modified_secs_ago(&id);
                                    let overwrite = peon::peon_should_overwrite(&m.metadata_source, age);
                                    (overwrite, m.metadata_source == "user")
                                })
                                .unwrap_or((true, false));
                            if should_write {
                                match ws.metadata.merge_peon_inference(&id, &inf, &now_iso, provider_result.observation.as_ref()) {
                                    Ok(()) => {
                                        inference_persisted = true;
                                        inf.summary.as_ref().map(|s| s.chars().take(100).collect())
                                    }
                                    Err(error) => {
                                        tracing::warn!(session_id = %id, %error, "peon: inference not persisted");
                                        None
                                    }
                                }
                            } else {
                                tracing::debug!(session_id = %id, "peon: skipping, higher-priority source exists");
                                permanent_hold = is_permanent;
                                None
                            }
                        } else {
                            None
                        }
                    }; // ws_guard dropped
                    if let Some(label) = label_update {
                        if let Some(handle) = state_clone.sessions.lock().unwrap().get_mut(&id) {
                            handle.info.label = label;
                        }
                    }
                }

                let mut last_inf = state_clone.peon.last_inference.write().unwrap();
                last_inf.insert(id.clone(), now_iso);
                drop(last_inf);

                // Three scheduling outcomes:
                // 1. Persisted + terminal: don't update last_output; lifecycle change removes session.
                // 2. Permanent hold (user source) + terminal: remove from pool entirely; new PTY
                //    output via terminal_runtime re-adds when the session becomes active again.
                // 3. A persisted non-terminal inference waits for new terminal output. A
                //    failed write or transient hold remains eligible for retry.
                if reached_terminal && inference_persisted {
                    // outcome 1: leave last_output unchanged
                } else if reached_terminal && permanent_hold {
                    state_clone.peon.last_output.write().unwrap().remove(&id);
                } else if inference_persisted {
                    state_clone.peon.last_output.write().unwrap().remove(&id);
                } else {
                    state_clone.peon.last_output.write().unwrap()
                        .insert(id.clone(), tokio::time::Instant::now());
                }
                state_clone.peon.in_flight.write().unwrap().remove(&id);
            });
        }

        // Timer-based idle detection: mark sessions that have been silent
        // for idle_timeout_secs as idle, without waiting for the LLM.
        {
            let idle_timeout = state.peon.config.idle_timeout_secs;
            let idle_deadline = tokio::time::Instant::now()
                - std::time::Duration::from_secs(idle_timeout);
            let last_output = state.peon.last_output.read().unwrap();

            let (silent_ids, missing_last_output_ids): (Vec<String>, Vec<String>) = {
                let sessions = state.sessions.lock().unwrap();
                let mut silent_ids = Vec::new();
                let mut missing_last_output_ids = Vec::new();

                for (id, handle) in sessions.iter() {
                    if handle.info.status != "running"
                        || handle.info.lifecycle_phase != "active"
                        || !matches!(handle.info.observed_status.as_deref(), None | Some("working"))
                    {
                        continue;
                    }

                    match last_output.get(id) {
                        Some(&t) if t <= idle_deadline => silent_ids.push(id.clone()),
                        Some(_) => {}
                        None => missing_last_output_ids.push(id.clone()),
                    }
                }

                (silent_ids, missing_last_output_ids)
            };
            drop(last_output);

            if !missing_last_output_ids.is_empty() {
                let now = tokio::time::Instant::now();
                let mut last_output = state.peon.last_output.write().unwrap();
                for id in missing_last_output_ids {
                    // Self-heal the transient gap where a session is visible
                    // as running before its startup idle timer origin exists.
                    last_output.entry(id).or_insert(now);
                }
            }

            if !silent_ids.is_empty() {
                {
                    let ws_guard = state.workspace.lock().unwrap();
                    if let Some(ref ws) = *ws_guard {
                        for id in &silent_ids {
                            if let Some(mut meta) = ws.metadata.read_session(id) {
                                if matches!(meta.observed_status.as_deref(), None | Some("working")) {
                                    meta.observed_status = Some("idle".into());
                                    meta.metadata_source = "process".into();
                                    ws.metadata.write_session(&meta);
                                }
                            }
                        }
                    }
                } // ws_guard dropped
                let mut sessions = state.sessions.lock().unwrap();
                for id in &silent_ids {
                    if let Some(handle) = sessions.get_mut(id) {
                        handle.info.observed_status = Some("idle".into());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::harness;
    use std::sync::{Arc, Mutex, RwLock};
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::AtomicU16;

    #[tokio::test]
    async fn test_peon_inference_writes_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        // Create a mock harness script that echoes known JSON
        let harness_path = dir.path().join("mock-harness.sh");
        std::fs::write(&harness_path, "#!/bin/bash\necho '{\"status\":\"working\",\"summary\":\"test\",\"confidence\":0.85}'\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&harness_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

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
                config: peon::PeonConfig::from_env(),
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
                    .stdout(r#"{"status":"working","summary":"test","confidence":0.85}"#)],
            ),
        });

        // Create a session with some output in the ring buffer
        let session_id = "peon-test-1".to_string();
        {
            let mut sessions = state.sessions.lock().unwrap();
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
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
            };
            handle.output_buffer.push("running cargo test...".into());
            handle.output_buffer.push("test result: ok. 5 passed; 0 failed;".into());
            sessions.insert(session_id.clone(), handle);
        }

        // Write initial metadata
        {
            let ws = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws {
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
                    observed_status: None,
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
        }

        // Set last_output to trigger inference (5s ago = past debounce interval)
        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        // Run peon_loop in background
        tokio::spawn(peon_loop(state.clone()));

        // Wait for metadata to be updated (poll up to 10 seconds)
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let ws = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws {
                if let Some(meta) = ws.metadata.read_session("peon-test-1") {
                    if meta.metadata_source == "peon" {
                        // Verify metadata was updated correctly
                        assert_eq!(meta.status, "running");
                        assert_eq!(meta.observed_status, Some("working".into()));
                        assert_eq!(meta.summary, Some("test".into()));
                        assert_eq!(meta.peon_last_inference.is_some(), true);
                        assert_eq!(meta.metadata_source, "peon");
                        assert!((meta.metadata_confidence - 0.85).abs() < 0.001);
                        return; // test passes
                    }
                }
            }
        }

        panic!("Peon did not update metadata within 10 seconds");
    }

    #[tokio::test]
    async fn peon_loop_does_not_start_duplicate_inference_while_in_flight() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let call_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

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
                config: peon::PeonConfig::from_env(),
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
                    .stdout(r#"{"observedStatus":"working","confidence":0.85}"#)
                    .sleep_ms(3000)
                    .with_counter(call_counter.clone())],
            ),
        });

        let session_id = "peon-duplicate-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
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
            };
            handle.output_buffer.push("quiet output".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
        }

        state.peon.last_output.write().unwrap().insert(
            session_id,
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2300)).await;
        task.abort();

        let count = call_counter.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn peon_loop_does_not_repeat_inference_without_new_output() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let call_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

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
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 30,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
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
                    .stdout(r#"{"observedStatus":"working","summary":"still working","confidence":0.85}"#)
                    .with_counter(call_counter.clone())],
            ),
        });

        let session_id = "peon-no-repeat-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
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
            };
            handle.output_buffer.push("unchanged output".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
        }

        state.peon.last_output.write().unwrap().insert(
            session_id,
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2600)).await;
        task.abort();

        let count = call_counter.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(count, 1, "Peon should not re-infer without new output");
    }

    #[tokio::test]
    async fn peon_loop_records_failed_inference_attempt() {
        let dir = tempfile::tempdir().unwrap();

        let state = Arc::new(crate::AppState {
            session_module: crate::infrastructure::session_module::SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 15,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::new(),
        });

        let session_id = "peon-failed-attempt-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
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
            };
            handle.output_buffer.push("quiet output".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
        }

        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

        assert!(
            state.peon.last_inference.read().unwrap().contains_key(&session_id),
            "failed Peon attempts should still be recorded in last_inference"
        );
    }

    #[tokio::test]
    async fn peon_loop_marks_idle_when_silent() {
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
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 1, // fast idle detection for test
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-idle-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
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
            };
            handle.output_buffer.push("some past output".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
        }

        // Set last_output to 5 seconds ago (well past the 1s idle timeout)
        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        // Initialize session metadata so the idle timer can write observed_status.
        {
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
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
                    observed_status: None,
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
        }

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        task.abort();

        // Check metadata: observed_status should be "idle"
        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status.as_deref(), Some("idle"));
                assert_eq!(meta.metadata_source, "process");
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }
    }

    #[tokio::test]
    async fn peon_loop_marks_silent_working_session_idle() {
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
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 1,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-working-idle-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut info = test_session_info(
                session_id.clone(),
                "Test",
                dir.path().display().to_string(),
                "running",
                "now",
            );
            info.lifecycle_phase = "active".into();
            info.metadata_source = Some("peon".into());
            info.metadata_confidence = Some(0.85);
            info.observed_status = Some("working".into());
            state.sessions.lock().unwrap().insert(
                session_id.clone(),
                crate::SessionHandle {
                    info,
                    kill_tx,
                    output_buffer: peon::RingBuffer::new(200),
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
        }

        {
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
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
                    observed_status: Some("working".into()),
                    ending_observed_status_snapshot: None,
                    final_observed_status_snapshot: None,
                    summary: Some("Still working".into()),
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    peon_last_inference: Some("before".into()),
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
                    metadata_source: "peon".into(),
                    metadata_confidence: 0.85,
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
        }

        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );
        state.peon.last_inference.write().unwrap().insert(session_id.clone(), "before".into());

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        task.abort();

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status.as_deref(), Some("idle"));
                assert_eq!(meta.metadata_source, "process");
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions.get(&session_id).unwrap().info.observed_status.as_deref(),
            Some("idle")
        );
    }

    #[tokio::test]
    async fn peon_loop_does_not_mark_recently_started_silent_session_idle() {
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
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 5,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-recent-start-test".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = test_session_info(
            session_id.clone(),
            "Test",
            dir.path().display().to_string(),
            "creating",
            "now",
        );
        info.lifecycle_phase = "creating".into();
        info.metadata_source = Some("process".into());
        info.metadata_confidence = Some(1.0);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
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
            if let Some(ref ws) = *ws_guard {
                ws.metadata.write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Test".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "creating".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "creating".into(),
                    lifecycle: "creating".into(),
                    attention: None,
                    connectivity: "online".into(),
                    terminal_outcome: None,
                    pending_terminal_status: None,
                    observed_status: None,
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
        }

        crate::runtime::terminal_runtime::set_session_status(&state, &session_id, "running");

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status, None);
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(sessions.get(&session_id).unwrap().info.observed_status, None);
    }

    #[tokio::test]
    async fn peon_loop_does_not_mark_running_session_without_last_output_idle() {
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
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 5,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-missing-last-output-test".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = test_session_info(
            session_id.clone(),
            "Test",
            dir.path().display().to_string(),
            "running",
            "now",
        );
        info.lifecycle_phase = "active".into();
        info.metadata_source = Some("process".into());
        info.metadata_confidence = Some(1.0);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
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
            if let Some(ref ws) = *ws_guard {
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
                    observed_status: None,
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
        }

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status, None);
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(sessions.get(&session_id).unwrap().info.observed_status, None);
    }

    #[tokio::test]
    async fn peon_loop_eventually_marks_running_session_without_last_output_idle() {
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
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 1,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-missing-last-output-eventual-idle-test".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = test_session_info(
            session_id.clone(),
            "Test",
            dir.path().display().to_string(),
            "running",
            "now",
        );
        info.lifecycle_phase = "active".into();
        info.metadata_source = Some("process".into());
        info.metadata_confidence = Some(1.0);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
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
            if let Some(ref ws) = *ws_guard {
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
                    observed_status: None,
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
        }

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        task.abort();

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status.as_deref(), Some("idle"));
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions.get(&session_id).unwrap().info.observed_status.as_deref(),
            Some("idle"),
        );
    }

    #[tokio::test]
    async fn peon_loop_does_not_overwrite_existing_observed_status_with_idle() {
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
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 1,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-idle-no-overwrite-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    observed_status: Some("blocked".into()),
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
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
            };
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
        }

        {
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
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
        }

        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        task.abort();

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status.as_deref(), Some("blocked"));
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }
    }

    #[tokio::test]
    async fn peon_loop_skips_sessions_in_ending_lifecycle() {
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
                config: peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
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
                    .stdout(r#"{"status":"working","summary":"should-not-run","confidence":0.85}"#)],
            ),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-ending-skip-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
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
            };
            handle.info.lifecycle_phase = "ending".into();
            handle.output_buffer.push("finishing up".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
        }

        {
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
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
        }

        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        task.abort();

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert!(meta.peon_last_inference.is_none());
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }
    }

    // Regression: persist skipped must not drop session from candidate pool (issue #87).
    #[tokio::test]
    async fn peon_loop_retries_when_persist_skipped_despite_terminal_inference() {
        let dir = tempfile::tempdir().unwrap();

        let state = Arc::new(crate::AppState {
            session_module: crate::infrastructure::session_module::SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None), // no workspace → persist is always skipped
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec![],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 30,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: AtomicU16::new(0),
            // FakeProvider returns "idle" → reached_terminal=true inside spawn_blocking
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
                    .stdout(r#"{"status":"idle","confidence":0.85}"#)],
            ),
        });

        let session_id = "peon-retry-persist-skipped-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                command: crate::harness_registry::default_shell_command(
                    dir.path().display().to_string(),
                ),
                initial_prompt: None,
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
            };
            handle.output_buffer.push("some terminal output".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
        }

        // Plant last_output 5s in the past — past the 1s interval, so session is
        // immediately eligible as a candidate.
        let before_test = tokio::time::Instant::now();
        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            before_test - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

        let lo = state.peon.last_output.read().unwrap();
        let updated_at = lo.get(&session_id).copied().expect("last_output entry removed");
        assert!(
            updated_at >= before_test,
            "last_output should be refreshed even when persist is skipped and inference was terminal; \
             session must remain eligible for retry, not silently exit the candidate pool"
        );
    }
}
