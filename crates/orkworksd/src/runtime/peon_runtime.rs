use crate::workspace_runtime::iso_now;
use crate::{metadata, peon, providers, AppState};
use std::sync::Arc;

pub(crate) async fn peon_loop(state: Arc<AppState>) {
    let interval = state.peon.config.interval_secs;
    tracing::info!(interval_secs = interval, harness = %state.peon.config.harness, "peon started");

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let candidates = {
            let scheduler = state.peon.scheduler.read().unwrap();
            let idle_deadline = std::time::Instant::now()
                - std::time::Duration::from_secs(state.peon.config.idle_timeout_secs);
            scheduler.due_normal_ids(std::time::Instant::now() - std::time::Duration::from_secs(interval))
                .into_iter()
                .filter(|id| scheduler.has_pending_label(id)
                    || scheduler.output_at(id).is_none_or(|at| at > idle_deadline))
                .collect::<Vec<String>>()
        };

        for session_id in candidates {
            if !state.sessions.lock().unwrap().get(&session_id)
                .is_some_and(|handle| handle.info.lifecycle_phase == "active")
            {
                continue;
            }
            let Some((lease, hint)) = state.peon.scheduler.write().unwrap()
                .claim_due_normal_inference(&session_id, std::time::Instant::now() - std::time::Duration::from_secs(interval))
            else {
                continue;
            };
            let output_snapshot = {
                let sessions = state.sessions.lock().unwrap();
                match sessions.get(&session_id) {
                    Some(handle) => handle.output_buffer.snapshot(),
                    None => {
                        state.peon.scheduler.write().unwrap().abandon_empty_inference(&lease);
                        continue;
                    }
                }
            };

            if output_snapshot.is_empty() && hint.is_none() {
                state.peon.scheduler.write().unwrap().abandon_empty_inference(&lease);
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
                if !state_clone.peon.scheduler.read().unwrap().lease_is_current(&lease)
                    || !state_clone.sessions.lock().unwrap().get(&id)
                        .is_some_and(|handle| handle.info.lifecycle_phase == "active")
                {
                    return;
                }
                let provider_result = state_clone.providers.run_inference(providers::PeonScope::Session, &output_snapshot);
                let inference = provider_result.inference;
                let now_iso = iso_now();
                let inferred_idle = matches!(
                    inference.as_ref().and_then(|inf| inf.observed_status.as_deref()),
                    Some("idle"),
                );

                // Check terminal status before moving inference below
                let reached_terminal = matches!(
                    inference
                        .as_ref()
                        .and_then(|inf| inf.observed_status.as_deref()),
                    Some("done" | "idle" | "stale")
                );

                let mut inference_persisted = false;
                let permanent_hold = false;
                {
                    // The scheduler write lock is the lease authority. Ending acquires the
                    // same lock before changing lifecycle, so normal writes are either fully
                    // committed before ending begins or discarded.
                    let mut scheduler = state_clone.peon.scheduler.write().unwrap();
                    if !scheduler.lease_is_current(&lease)
                        || !state_clone.sessions.lock().unwrap().get(&id)
                            .is_some_and(|handle| handle.info.lifecycle_phase == "active")
                    {
                        return;
                    }
                    let ws_guard = state_clone.workspace.lock().unwrap();
                    let label_update = if let Some(ref ws) = *ws_guard {
                        if let Some(ref obs) = provider_result.observation {
                            ws.metadata.persist_provider_context(&id, obs);
                        }
                        if let Some(ref inf) = inference {
                            let should_write = ws.metadata.read_session(&id)
                                .map(|m| {
                                    let age = ws.metadata.session_modified_secs_ago(&id);
                                    peon::peon_should_overwrite(&m.metadata_source, age)
                                })
                                .unwrap_or(true);
                            if should_write {
                                match ws.metadata.merge_peon_inference(
                                    &id,
                                    &inf,
                                    &now_iso,
                                    provider_result.observation.as_ref(),
                                ) {
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
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    drop(ws_guard);
                    if let Some(label) = label_update {
                        if let Some(handle) = state_clone.sessions.lock().unwrap().get_mut(&id) {
                            handle.info.label = label;
                        }
                    }
                    state_clone.peon.last_inference.write().unwrap().insert(id.clone(), now_iso);
                    if inference_persisted {
                        scheduler.complete_normal_inference(&lease, inferred_idle, None);
                    } else if reached_terminal && permanent_hold {
                        scheduler.complete_normal_inference(&lease, false, None);
                    } else {
                        scheduler.complete_normal_inference(&lease, false, Some(std::time::Instant::now()));
                    }
                }
            });
        }

        // Timer-based idle detection: mark sessions that have been silent
        // for idle_timeout_secs as idle, without waiting for the LLM.
        {
            let idle_timeout = state.peon.config.idle_timeout_secs;
            let session_states: Vec<_> = state.sessions.lock().unwrap().iter()
                .map(|(id, handle)| (id.clone(), handle.info.status.clone(), handle.info.lifecycle_phase.clone(), handle.info.observed_status.clone()))
                .collect();
            let (silent_ids, missing_last_output_ids): (Vec<String>, Vec<String>) = {
                let scheduler = state.peon.scheduler.read().unwrap();
                let mut silent_ids = Vec::new();
                let mut missing_last_output_ids = Vec::new();

                for (id, status, lifecycle_phase, observed_status) in &session_states {
                    if status != "running"
                        || lifecycle_phase != "active"
                        || observed_status.as_deref().is_some_and(|status| status != "working")
                        || scheduler.state_for(id) == peon::PeonSchedulerState::IdleWaitingForUserInput
                    {
                        continue;
                    }

                    match scheduler.output_at(id) {
                        Some(t) if t <= std::time::Instant::now() - std::time::Duration::from_secs(idle_timeout) => silent_ids.push(id.clone()),
                        Some(_) => {}
                        None => missing_last_output_ids.push(id.clone()),
                    }
                }

                (silent_ids, missing_last_output_ids)
            };
            if !missing_last_output_ids.is_empty() {
                let now = std::time::Instant::now();
                let mut scheduler = state.peon.scheduler.write().unwrap();
                for id in missing_last_output_ids {
                    // Self-heal the transient gap where a session is visible
                    // as running before its startup idle timer origin exists.
                    scheduler.ensure_output_origin(&id, now);
                }
            }

            if !silent_ids.is_empty() {
                let idle_candidates: Vec<String> = {
                    let ws_guard = state.workspace.lock().unwrap();
                    match &*ws_guard {
                        Some(ws) => silent_ids.into_iter().filter(|id| {
                            ws.metadata.read_session(id).is_none_or(|meta| {
                                matches!(meta.observed_status.as_deref(), None | Some("working"))
                            })
                        }).collect(),
                        None => silent_ids,
                    }
                };
                let idle_deadline = std::time::Instant::now()
                    - std::time::Duration::from_secs(idle_timeout);
                let held_ids: Vec<String> = {
                    let mut scheduler = state.peon.scheduler.write().unwrap();
                    let mut held = Vec::new();
                    for id in idle_candidates {
                        if scheduler.state_for(&id) != peon::PeonSchedulerState::IdleWaitingForUserInput
                            && scheduler.output_at(&id).is_some_and(|at| at <= idle_deadline)
                        {
                            scheduler.hold_idle(&id);
                            held.push(id);
                        }
                    }
                    held
                };
                {
                    let ws_guard = state.workspace.lock().unwrap();
                    if let Some(ref ws) = *ws_guard {
                        for id in &held_ids {
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
                for id in &held_ids {
                    if let Some(handle) = sessions.get_mut(id) {
                        handle.info.observed_status = Some("idle".into());
                        handle.info.metadata_source = Some("process".into());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness;
    use crate::test_support::*;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::AtomicU16;
    use std::sync::{Arc, Mutex, RwLock};

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
            std::fs::set_permissions(&harness_path, std::fs::Permissions::from_mode(0o755))
                .unwrap();
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                command: harness::CommandSpec {
                    program: "/bin/sh".into(),
                    args: vec!["-i".into(), "-l".into()],
                    cwd: "/tmp".into(),
                },
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
                debug_injection: None,
            };
            handle.output_buffer.push("running cargo test...".into());
            handle
                .output_buffer
                .push("test result: ok. 5 passed; 0 failed;".into());
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
                    debug_injection: None,
                });
            }
        }

        // Set last_output to trigger inference (5s ago = past debounce interval)
        state.peon.scheduler.write().unwrap().request_observation_from_output(
            session_id.clone(),
            std::time::Instant::now() - std::time::Duration::from_secs(5),
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                command: harness::CommandSpec {
                    program: "/bin/sh".into(),
                    args: vec!["-i".into(), "-l".into()],
                    cwd: "/tmp".into(),
                },
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
                debug_injection: None,
            };
            handle.output_buffer.push("quiet output".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        state.peon.scheduler.write().unwrap().request_observation_from_output(
            session_id,
            std::time::Instant::now() - std::time::Duration::from_secs(5),
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                debug_injection: None,
            };
            handle.output_buffer.push("unchanged output".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        state.peon.scheduler.write().unwrap().request_observation_from_output(
            session_id,
            std::time::Instant::now() - std::time::Duration::from_secs(5),
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                debug_injection: None,
            };
            handle.output_buffer.push("quiet output".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        state.peon.scheduler.write().unwrap().request_observation_from_output(
            session_id.clone(),
            std::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

        assert!(
            state
                .peon
                .last_inference
                .read()
                .unwrap()
                .contains_key(&session_id),
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                debug_injection: None,
            };
            handle.output_buffer.push("some past output".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        // Set last_output to 5 seconds ago (well past the 1s idle timeout)
        state.peon.scheduler.write().unwrap().request_observation_from_output(
            session_id.clone(),
            std::time::Instant::now() - std::time::Duration::from_secs(5),
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
                    debug_injection: None,
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                    debug_injection: None,
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
                    debug_injection: None,
                });
            }
        }

        state.peon.scheduler.write().unwrap().request_observation_from_output(
            session_id.clone(),
            std::time::Instant::now() - std::time::Duration::from_secs(5),
        );
        state
            .peon
            .last_inference
            .write()
            .unwrap()
            .insert(session_id.clone(), "before".into());

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
            sessions
                .get(&session_id)
                .unwrap()
                .info
                .observed_status
                .as_deref(),
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                debug_injection: None,
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
                    debug_injection: None,
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
        assert_eq!(
            sessions.get(&session_id).unwrap().info.observed_status,
            None
        );
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                debug_injection: None,
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
                    debug_injection: None,
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
        assert_eq!(
            sessions.get(&session_id).unwrap().info.observed_status,
            None
        );
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                debug_injection: None,
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
                    debug_injection: None,
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
            sessions
                .get(&session_id)
                .unwrap()
                .info
                .observed_status
                .as_deref(),
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                debug_injection: None,
            };
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
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
                    debug_injection: None,
                });
            }
        }

        state.peon.scheduler.write().unwrap().request_observation_from_output(
            session_id.clone(),
            std::time::Instant::now() - std::time::Duration::from_secs(5),
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                vec![providers::FakeProvider::new("opencode").stdout(
                    r#"{"status":"working","summary":"should-not-run","confidence":0.85}"#,
                )],
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
                debug_injection: None,
            };
            handle.info.lifecycle_phase = "ending".into();
            handle.output_buffer.push("finishing up".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
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
                    debug_injection: None,
                });
            }
        }

        state.peon.scheduler.write().unwrap().request_observation_from_output(
            session_id.clone(),
            std::time::Instant::now() - std::time::Duration::from_secs(5),
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
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                debug_injection: None,
            };
            handle.output_buffer.push("some terminal output".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        // Plant last_output 5s in the past — past the 1s interval, so session is
        // immediately eligible as a candidate.
        let before_test = std::time::Instant::now();
        state.peon.scheduler.write().unwrap().request_observation_from_output(
            session_id.clone(),
            before_test - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

        let updated_at = state.peon.scheduler.read().unwrap()
            .output_at(&session_id).expect("last_output entry removed");
        assert!(
            updated_at >= before_test,
            "last_output should be refreshed even when persist is skipped and inference was terminal; \
             session must remain eligible for retry, not silently exit the candidate pool"
        );
    }

    #[tokio::test]
    async fn peon_loop_retries_when_persist_skipped_despite_non_terminal_inference() {
        let dir = tempfile::tempdir().unwrap();

        let state = Arc::new(crate::AppState {
            session_module: crate::infrastructure::session_module::SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None), // no workspace → persist is always skipped
            peon: crate::PeonState {
                scheduler: std::sync::RwLock::new(crate::peon::PeonScheduler::default()),
                last_inference: RwLock::new(HashMap::new()),
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
                    .stdout(r#"{"status":"working","confidence":0.85}"#)],
            ),
        });

        let session_id = "peon-retry-non-terminal-persist-skipped-test".to_string();
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
                debug_injection: None,
            };
            handle.output_buffer.push("some terminal output".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
        }

        let before_test = std::time::Instant::now();
        state.peon.scheduler.write().unwrap().request_observation_from_output(
            session_id.clone(),
            before_test - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

        let updated_at = state.peon.scheduler.read().unwrap().output_at(&session_id).expect("last_output entry removed");
        assert!(
            updated_at >= before_test,
            "last_output should be refreshed even when persist is skipped and inference was non-terminal; \
             session must remain eligible for retry after the debounce window"
        );
    }
}
