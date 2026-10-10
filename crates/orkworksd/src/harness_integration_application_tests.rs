use super::{
    GroupedIntegrationStatus, HarnessIntegrationApplication, IntegrationApplicationError,
    IntegrationInspection, IntegrationMutation, IntegrationMutationRequest,
    IntegrationRevisionExpectation, IntegrationTarget,
};
use crate::harness::definition::{HarnessPatch, LaunchPatch, VersionRequirement};
use crate::harness::integration::{
    IntegrationActivation, IntegrationCoverage, IntegrationError, IntegrationKey,
    IntegrationRegistration,
};
use crate::session_application::SessionApplication;
use crate::test_support::{
    make_test_executable, swap_workspace, test_app_state_with_workspace, FakeHome, FakePath,
};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

fn init_git_workspace_with_claude_settings_ignored(workspace: &std::path::Path) {
    git2::Repository::init(workspace).unwrap();
    std::fs::write(
        workspace.join(".gitignore"),
        ".claude/settings.local.json\n",
    )
    .unwrap();
}

fn init_git_workspace_with_copilot_settings_ignored(workspace: &std::path::Path) {
    git2::Repository::init(workspace).unwrap();
    std::fs::write(
        workspace.join(".gitignore"),
        ".github/copilot/settings.local.json\n",
    )
    .unwrap();
}

fn install_fake_command(dir: &std::path::Path, command: &str, body: &str) {
    let name = if cfg!(windows) {
        format!("{command}.exe")
    } else {
        command.to_owned()
    };
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    make_test_executable(&path);
}

fn insert_live_prompt_authority_session(
    state: &crate::AppState,
    workspace_path: &std::path::Path,
    session_id: &str,
    source: &str,
) {
    let mut metadata = crate::test_support::test_session_metadata(
        session_id,
        "Claude prompt",
        &workspace_path.display().to_string(),
        "running",
        "now",
        "now",
    );
    metadata.harness = "claude-code".into();
    metadata.lifecycle = "alive".into();
    metadata.lifecycle_phase = "active".into();
    metadata.metadata_source = source.into();
    metadata.observed_status = Some("waiting_for_input".into());
    metadata.attention = Some("needs_you".into());
    metadata.needs_user_input = Some(true);
    metadata.detected_question = Some("Should I continue?".into());
    metadata.suggested_options = Some(vec!["Continue".into(), "Stop".into()]);
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .write_session(&metadata);

    let mut info = crate::test_support::test_session_info(
        session_id,
        "Claude prompt",
        workspace_path.display().to_string(),
        "running",
        "now",
    );
    info.harness = Some("claude-code".into());
    info.harness_id = Some("claude-code".into());
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    info.metadata_source = Some(source.into());
    info.observed_status = metadata.observed_status.clone();
    info.attention = metadata.attention.clone();
    info.needs_user_input = metadata.needs_user_input;
    info.detected_question = metadata.detected_question.clone();
    info.suggested_options = metadata.suggested_options.clone();
    let (kill_tx, _) = tokio::sync::watch::channel(false);
    state.sessions.lock().unwrap().insert(
        session_id.into(),
        crate::SessionHandle {
            info,
            active_work_hook: false,
            kill_tx,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
            ),
            terminal_attached: false,
            resume_in_progress: false,
            capacity: crate::capacity_state::CapacityState::default(),
        },
    );
}

fn grouped_request(
    state: &crate::AppState,
    key: IntegrationKey,
    mutation: IntegrationMutation,
) -> IntegrationMutationRequest {
    let document_revision = state.harness_store.snapshot().unwrap().document_revision;
    let active_harness_revision = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|workspace| workspace.metadata.read_workspace_memory())
        .unwrap()
        .active_harness_revision;
    IntegrationMutationRequest::Group {
        key,
        mutation,
        expected: IntegrationRevisionExpectation {
            document_revision,
            active_harness_revision,
            workspace_path: None,
        },
    }
}

#[tokio::test]
async fn inspect_returns_the_real_absent_state_for_a_fresh_workspace() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());

    let result = HarnessIntegrationApplication::new(state)
        .inspect(IntegrationTarget::Harness("claude-code".into()))
        .await
        .unwrap();
    let IntegrationInspection::Harness(status) = result else {
        panic!("expected harness status");
    };
    assert_eq!(status.registration, IntegrationRegistration::Absent);
    assert!(!status.enabled);
}

#[tokio::test]
async fn inspect_retains_missing_workspace_and_unknown_harness_errors() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    let app = HarnessIntegrationApplication::new(state.clone());
    *state.workspace.lock().unwrap() = None;
    assert!(matches!(
        app.inspect(IntegrationTarget::Harness("claude-code".into()))
            .await,
        Err(IntegrationApplicationError::NoWorkspace)
    ));
    let next = tempfile::tempdir().unwrap();
    let second_state = test_app_state_with_workspace(next.path());
    assert!(matches!(
        HarnessIntegrationApplication::new(second_state)
            .inspect(IntegrationTarget::Harness("not-a-real-harness".into()))
            .await,
        Err(IntegrationApplicationError::UnknownHarness(_))
    ));
}

#[tokio::test]
async fn inspect_does_not_mutate_absent_configuration() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    let settings = dir.path().join(".claude/settings.local.json");

    HarnessIntegrationApplication::new(state)
        .inspect(IntegrationTarget::Harness("claude-code".into()))
        .await
        .unwrap();

    assert!(!settings.exists());
}

#[tokio::test]
async fn grouped_mutation_requires_revisions_and_preserves_group_identity() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["copilot".into()])
        .unwrap();
    let key = IntegrationKey {
        adapter_id: "copilot".into(),
        target_id: "workspace".into(),
    };

    let result = HarnessIntegrationApplication::new(state.clone())
        .mutate(grouped_request(
            &state,
            key.clone(),
            IntegrationMutation::Install,
        ))
        .await
        .unwrap();
    let IntegrationInspection::Group(GroupedIntegrationStatus {
        key: actual_key,
        consumers,
        status,
    }) = result
    else {
        panic!("expected grouped status");
    };
    assert_eq!(actual_key, key);
    assert_eq!(consumers.len(), 1);
    assert_eq!(status.registration, IntegrationRegistration::Installed);
}

#[tokio::test]
async fn grouped_mutation_rejects_a_stale_revision_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["copilot".into()])
        .unwrap();
    let key = IntegrationKey {
        adapter_id: "copilot".into(),
        target_id: "workspace".into(),
    };
    let IntegrationMutationRequest::Group {
        key,
        mutation,
        mut expected,
    } = grouped_request(&state, key, IntegrationMutation::Install)
    else {
        panic!()
    };
    expected.active_harness_revision += 1;
    let result = HarnessIntegrationApplication::new(state.clone())
        .mutate(IntegrationMutationRequest::Group {
            key,
            mutation,
            expected,
        })
        .await;
    assert!(matches!(
        result,
        Err(IntegrationApplicationError::RevisionConflict(_))
    ));
    assert!(!dir
        .path()
        .join(".github/copilot/settings.local.json")
        .exists());
}

#[tokio::test]
async fn grouped_mutation_rejects_document_and_selection_changes_during_probe() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["copilot".into()])
        .unwrap();
    let key = IntegrationKey {
        adapter_id: "copilot".into(),
        target_id: "workspace".into(),
    };
    let request = grouped_request(&state, key, IntegrationMutation::Install);
    let update_state = state.clone();
    let app = HarnessIntegrationApplication {
        state: state.clone(),
        revocation_result_hook: None,
        group_revalidation_hook: None,
        probe_revalidation_hook: Some(Arc::new(move |harness_id| {
            assert_eq!(harness_id, "copilot");
            update_state
                .harness_store
                .mutate(&update_state.harness_catalog, |document| {
                    document.overrides.insert(
                        "copilot".into(),
                        HarnessPatch {
                            min_version: Some(Some(VersionRequirement { min: (1, 0, 0) })),
                            ..Default::default()
                        },
                    );
                    Ok(())
                })
                .unwrap();
            SessionApplication::new(update_state.clone())
                .set_active_harnesses(vec!["claude-code".into()])
                .unwrap();
        })),
    };

    let result = app.mutate(request).await;

    let Err(IntegrationApplicationError::RevisionConflict(conflict)) = result else {
        panic!("expected a current-revision conflict after both revisions changed");
    };
    let current_document_revision = state.harness_store.snapshot().unwrap().document_revision;
    let current_active_revision = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .read_workspace_memory()
        .unwrap()
        .active_harness_revision;
    assert_eq!(conflict.document_revision, current_document_revision);
    assert_eq!(conflict.active_harness_revision, current_active_revision);
    assert!(!dir
        .path()
        .join(".github/copilot/settings.local.json")
        .exists());
}

#[tokio::test]
async fn grouped_install_and_uninstall_preserve_shared_identity_and_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["copilot".into()])
        .unwrap();
    let app = HarnessIntegrationApplication::new(state.clone());
    let key = IntegrationKey {
        adapter_id: "copilot".into(),
        target_id: "workspace".into(),
    };
    let installed = app
        .mutate(grouped_request(
            &state,
            key.clone(),
            IntegrationMutation::Install,
        ))
        .await
        .unwrap();
    let IntegrationInspection::Group(installed) = installed else {
        panic!()
    };
    assert_eq!(installed.key, key);
    assert_eq!(
        installed.status.registration,
        IntegrationRegistration::Installed
    );
    let removed = app
        .mutate(grouped_request(
            &state,
            key.clone(),
            IntegrationMutation::Uninstall,
        ))
        .await
        .unwrap();
    let IntegrationInspection::Group(removed) = removed else {
        panic!()
    };
    assert_eq!(removed.key, key);
    assert_eq!(removed.status.registration, IntegrationRegistration::Absent);
}

#[tokio::test]
async fn grouped_uninstall_reports_manual_cleanup_for_foreign_copilot_hook() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let settings = dir.path().join(".github/copilot/settings.local.json");
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    let foreign = serde_json::json!({
        "version": 1,
        "hooks": {"notification": [{
            "type": "command",
            "bash": "foreign-reporter",
            "env": {"ORKWORKS_INTEGRATION_MARKER": "orkworks:harness-integration:v2:foreign"}
        }]}
    });
    let original = serde_json::to_vec_pretty(&foreign).unwrap();
    std::fs::write(&settings, &original).unwrap();
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["copilot".into()])
        .unwrap();
    let result = HarnessIntegrationApplication::new(state.clone())
        .mutate(grouped_request(
            &state,
            IntegrationKey {
                adapter_id: "copilot".into(),
                target_id: "workspace".into(),
            },
            IntegrationMutation::Uninstall,
        ))
        .await
        .unwrap();
    let IntegrationInspection::Group(result) = result else {
        panic!()
    };
    assert!(result
        .status
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.action.as_deref() == Some("cleanup-needed")));
    assert_eq!(std::fs::read(&settings).unwrap(), original);
}

#[tokio::test]
async fn ambiguous_foreign_codex_hooks_are_preserved_on_uninstall() {
    let dir = tempfile::tempdir().unwrap();
    git2::Repository::init(dir.path()).unwrap();
    std::fs::write(dir.path().join(".gitignore"), ".codex/hooks.json\n").unwrap();
    let hooks = dir.path().join(".codex/hooks.json");
    std::fs::create_dir_all(hooks.parent().unwrap()).unwrap();
    let foreign = r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"/path/to/report-harness-event.sh --marker 'orkworks:harness-integration:v2:claude-code'"}]}]}}"#;
    std::fs::write(&hooks, foreign).unwrap();
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    let result = HarnessIntegrationApplication::new(state)
        .mutate(IntegrationMutationRequest::Harness {
            harness_id: "codex".into(),
            mutation: IntegrationMutation::Uninstall,
        })
        .await;
    assert!(matches!(
        result,
        Err(IntegrationApplicationError::Configuration(
            IntegrationError::OwnershipAmbiguous
        ))
    ));
    assert_eq!(std::fs::read_to_string(hooks).unwrap(), foreign);
}

#[test]
fn packaged_reporter_scripts_are_preferred_with_manifest_fallback() {
    let packaged = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(packaged.path().join("scripts")).unwrap();
    let manifest = tempfile::tempdir().unwrap();
    assert_eq!(
        super::resolve_scripts_source_dir(Some(packaged.path().to_path_buf()), manifest.path()),
        packaged.path().join("scripts")
    );
    assert_eq!(
        super::resolve_scripts_source_dir(None, manifest.path()),
        manifest.path().join("scripts")
    );
}

#[tokio::test]
async fn inspect_revocation_failure_overrides_a_successful_status() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["claude-code".into()])
        .unwrap();
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let observed_attempts = attempts.clone();
    let app = HarnessIntegrationApplication {
        state,
        revocation_result_hook: Some(Arc::new(move |real_result| {
            observed_attempts.lock().unwrap().push(real_result);
            false
        })),
        group_revalidation_hook: None,
        probe_revalidation_hook: None,
    };

    let result = app
        .inspect(IntegrationTarget::Group(IntegrationKey {
            adapter_id: "claude".into(),
            target_id: "workspace".into(),
        }))
        .await;

    assert!(matches!(
        result,
        Err(IntegrationApplicationError::AuthorityRevocation)
    ));
    assert_eq!(*attempts.lock().unwrap(), [true]);
}

#[tokio::test]
async fn inspect_revokes_real_authority_and_clears_the_non_user_prompt_tuple() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["claude-code".into()])
        .unwrap();
    let session_id = "inspect-demotes-live-claude-authority";
    insert_live_prompt_authority_session(&state, dir.path(), session_id, "claude_hook");
    let authority = crate::runtime::prompt_authority::registry();
    authority.issue_with_native_id(
        session_id,
        "claude-code",
        "inspect-demotion-generation",
        Some("native"),
    );
    assert!(authority.activate(
        session_id,
        "claude-code",
        "inspect-demotion-generation",
        "native"
    ));

    let result = HarnessIntegrationApplication::new(state.clone())
        .inspect(IntegrationTarget::Group(IntegrationKey {
            adapter_id: "claude".into(),
            target_id: "workspace".into(),
        }))
        .await;

    assert!(result.is_ok());
    assert!(!authority.is_active(session_id));
    assert!(!authority.generation_matches(
        session_id,
        "claude-code",
        "inspect-demotion-generation"
    ));
    let persisted = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .read_session(session_id)
        .unwrap();
    assert_eq!(persisted.observed_status, None);
    assert_eq!(persisted.attention, None);
    assert_eq!(persisted.needs_user_input, None);
    assert_eq!(persisted.detected_question, None);
    assert_eq!(persisted.suggested_options, None);
    let sessions = state.sessions.lock().unwrap();
    let live = &sessions.get(session_id).unwrap().info;
    assert_eq!(live.observed_status, None);
    assert_eq!(live.attention, None);
    assert_eq!(live.needs_user_input, None);
    assert_eq!(live.detected_question, None);
    assert_eq!(live.suggested_options, None);
    drop(sessions);
    authority.remove(session_id);
}

#[tokio::test]
async fn inspect_revokes_real_authority_but_preserves_the_user_prompt_tuple() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["claude-code".into()])
        .unwrap();
    let session_id = "inspect-preserves-user-authority-tuple";
    insert_live_prompt_authority_session(&state, dir.path(), session_id, "user");
    let authority = crate::runtime::prompt_authority::registry();
    authority.issue_with_native_id(
        session_id,
        "claude-code",
        "inspect-user-generation",
        Some("native"),
    );
    assert!(authority.activate(
        session_id,
        "claude-code",
        "inspect-user-generation",
        "native"
    ));

    let result = HarnessIntegrationApplication::new(state.clone())
        .inspect(IntegrationTarget::Group(IntegrationKey {
            adapter_id: "claude".into(),
            target_id: "workspace".into(),
        }))
        .await;

    assert!(result.is_ok());
    assert!(!authority.is_active(session_id));
    assert!(!authority.generation_matches(session_id, "claude-code", "inspect-user-generation"));
    let persisted = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .read_session(session_id)
        .unwrap();
    assert_eq!(persisted.metadata_source, "user");
    assert_eq!(
        persisted.observed_status.as_deref(),
        Some("waiting_for_input")
    );
    assert_eq!(persisted.attention.as_deref(), Some("needs_you"));
    assert_eq!(persisted.needs_user_input, Some(true));
    assert_eq!(
        persisted.detected_question.as_deref(),
        Some("Should I continue?")
    );
    assert_eq!(
        persisted.suggested_options,
        Some(vec!["Continue".into(), "Stop".into()])
    );
    let sessions = state.sessions.lock().unwrap();
    let live = &sessions.get(session_id).unwrap().info;
    assert_eq!(live.metadata_source.as_deref(), Some("user"));
    assert_eq!(live.observed_status.as_deref(), Some("waiting_for_input"));
    assert_eq!(live.attention.as_deref(), Some("needs_you"));
    assert_eq!(live.needs_user_input, Some(true));
    assert_eq!(
        live.detected_question.as_deref(),
        Some("Should I continue?")
    );
    assert_eq!(
        live.suggested_options,
        Some(vec!["Continue".into(), "Stop".into()])
    );
    drop(sessions);
    authority.remove(session_id);
}

#[tokio::test]
async fn inspect_revocation_failure_overrides_an_adapter_error_status() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let settings = dir.path().join(".claude/settings.local.json");
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(&settings, b"{").unwrap();
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["claude-code".into()])
        .unwrap();
    let app = HarnessIntegrationApplication {
        state,
        revocation_result_hook: Some(Arc::new(|real_result| {
            assert!(real_result);
            false
        })),
        group_revalidation_hook: None,
        probe_revalidation_hook: None,
    };

    let result = app
        .inspect(IntegrationTarget::Group(IntegrationKey {
            adapter_id: "claude".into(),
            target_id: "workspace".into(),
        }))
        .await;

    assert!(matches!(
        result,
        Err(IntegrationApplicationError::AuthorityRevocation)
    ));
}

#[tokio::test]
async fn workspace_listing_attempts_each_captured_revocation_in_order() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["claude-code".into(), "copilot".into()])
        .unwrap();
    let order = Arc::new(Mutex::new(Vec::new()));
    let next = Arc::new(Mutex::new(0));
    let observed_order = order.clone();
    let observed_next = next.clone();
    let app = HarnessIntegrationApplication {
        state,
        revocation_result_hook: Some(Arc::new(move |real_result| {
            assert!(real_result);
            let mut next = observed_next.lock().unwrap();
            observed_order.lock().unwrap().push(*next);
            *next += 1;
            *next != 1
        })),
        group_revalidation_hook: None,
        probe_revalidation_hook: None,
    };

    let result = app.list_workspace().await;

    assert!(matches!(
        result,
        Err(IntegrationApplicationError::AuthorityRevocation)
    ));
    assert_eq!(*order.lock().unwrap(), [0, 1]);
}

#[tokio::test]
async fn listing_revocation_failure_overrides_a_later_revalidation_error() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["claude-code".into(), "copilot".into()])
        .unwrap();
    let switch_state = state.clone();
    let revocations = Arc::new(Mutex::new(0));
    let observed_revocations = revocations.clone();
    let app = HarnessIntegrationApplication {
        state,
        revocation_result_hook: Some(Arc::new(move |real_result| {
            assert!(real_result);
            *observed_revocations.lock().unwrap() += 1;
            false
        })),
        group_revalidation_hook: Some(Arc::new(move |index| {
            if index == 1 {
                *switch_state.workspace.lock().unwrap() = None;
            }
        })),
        probe_revalidation_hook: None,
    };

    let result = app.list_workspace().await;

    assert!(matches!(
        result,
        Err(IntegrationApplicationError::AuthorityRevocation)
    ));
    assert_eq!(*revocations.lock().unwrap(), 1);
}

#[tokio::test]
async fn listing_returns_later_revalidation_error_when_revocation_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["claude-code".into(), "copilot".into()])
        .unwrap();
    let switch_state = state.clone();
    let app = HarnessIntegrationApplication {
        state,
        revocation_result_hook: Some(Arc::new(|real_result| real_result)),
        group_revalidation_hook: Some(Arc::new(move |index| {
            if index == 1 {
                *switch_state.workspace.lock().unwrap() = None;
            }
        })),
        probe_revalidation_hook: None,
    };

    let result = app.list_workspace().await;

    assert!(matches!(
        result,
        Err(IntegrationApplicationError::NoWorkspace)
    ));
}

#[tokio::test]
async fn failed_required_revocation_keeps_published_configuration() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    let app = HarnessIntegrationApplication {
        state: state.clone(),
        revocation_result_hook: Some(Arc::new(|real_result| {
            assert!(real_result);
            false
        })),
        group_revalidation_hook: None,
        probe_revalidation_hook: None,
    };

    let result = app
        .mutate(IntegrationMutationRequest::Harness {
            harness_id: "claude-code".into(),
            mutation: IntegrationMutation::Install,
        })
        .await;

    assert!(matches!(
        result,
        Err(IntegrationApplicationError::AuthorityRevocation)
    ));
    let reinspection = HarnessIntegrationApplication::new(state)
        .inspect(IntegrationTarget::Harness("claude-code".into()))
        .await
        .unwrap();
    let IntegrationInspection::Harness(status) = reinspection else {
        panic!("expected harness status");
    };
    assert_eq!(status.registration, IntegrationRegistration::Installed);
}

#[tokio::test]
async fn install_revokes_authority_that_predates_an_unready_hook_repair() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["claude-code".into()])
        .unwrap();
    let session_id = "integration-install-stale-authority";
    let mut metadata = crate::test_support::test_session_metadata(
        session_id,
        "Claude prompt",
        &dir.path().display().to_string(),
        "running",
        "now",
        "now",
    );
    metadata.harness = "claude-code".into();
    metadata.lifecycle = "alive".into();
    metadata.lifecycle_phase = "active".into();
    metadata.metadata_source = "claude_hook".into();
    metadata.observed_status = Some("working".into());
    metadata.attention = Some("working".into());
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .write_session(&metadata);
    let mut info = crate::test_support::test_session_info(
        session_id,
        "Claude prompt",
        dir.path().display().to_string(),
        "running",
        "now",
    );
    info.harness = Some("claude-code".into());
    info.harness_id = Some("claude-code".into());
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    info.metadata_source = Some("claude_hook".into());
    let (kill_tx, _) = tokio::sync::watch::channel(false);
    state.sessions.lock().unwrap().insert(
        session_id.into(),
        crate::SessionHandle {
            info,
            active_work_hook: false,
            kill_tx,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
            ),
            terminal_attached: false,
            resume_in_progress: false,
            capacity: crate::capacity_state::CapacityState::default(),
        },
    );
    let authority = crate::runtime::prompt_authority::registry();
    authority.issue_with_native_id(
        session_id,
        "claude-code",
        "stale-install-generation",
        Some("native"),
    );
    assert!(authority.activate(
        session_id,
        "claude-code",
        "stale-install-generation",
        "native"
    ));

    HarnessIntegrationApplication::new(state.clone())
        .mutate(IntegrationMutationRequest::Harness {
            harness_id: "claude-code".into(),
            mutation: IntegrationMutation::Install,
        })
        .await
        .unwrap();

    assert!(HarnessIntegrationApplication::new(state.clone())
        .prompt_attention_launch_ready("claude-code", "claude"));
    assert!(!authority.is_active(session_id));
    authority.remove(session_id);
}

#[tokio::test]
async fn unreferenced_cleanup_retains_complete_outcome_for_untracked_target() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    let outcomes = HarnessIntegrationApplication::new(state)
        .reconcile_unreferenced(
            BTreeSet::from([IntegrationKey {
                adapter_id: "copilot".into(),
                target_id: "workspace".into(),
            }]),
            Some(dir.path().to_path_buf()),
        )
        .await;

    assert_eq!(outcomes.status, "complete");
    assert!(outcomes.errors.is_empty());
    assert_eq!(outcomes.outcomes.len(), 1);
    assert_eq!(
        outcomes.outcomes[0].status.registration,
        IntegrationRegistration::Absent
    );
}

#[test]
fn readiness_is_a_conservative_query_without_revocation() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    let app = HarnessIntegrationApplication::new(state);

    assert!(!app.prompt_attention_launch_ready("claude-code", "claude"));
}

#[tokio::test]
async fn owned_install_reports_disabled_until_the_harness_is_active() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    let app = HarnessIntegrationApplication::new(state.clone());
    app.mutate(IntegrationMutationRequest::Harness {
        harness_id: "claude-code".into(),
        mutation: IntegrationMutation::Install,
    })
    .await
    .unwrap();

    let inactive = app
        .inspect(IntegrationTarget::Harness("claude-code".into()))
        .await
        .unwrap();
    let IntegrationInspection::Harness(inactive) = inactive else {
        panic!()
    };
    assert_eq!(inactive.registration, IntegrationRegistration::Installed);
    assert!(!inactive.enabled);
    assert_eq!(inactive.activation, IntegrationActivation::Disabled);

    SessionApplication::new(state)
        .set_active_harnesses(vec!["claude-code".into()])
        .unwrap();
    let active = app
        .inspect(IntegrationTarget::Harness("claude-code".into()))
        .await
        .unwrap();
    let IntegrationInspection::Harness(active) = active else {
        panic!()
    };
    assert!(active.enabled);
}

#[tokio::test]
async fn legacy_copilot_alias_uses_canonical_active_selection_for_status_and_install() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["copilot".into()])
        .unwrap();
    let settings_path = dir.path().join(".github/copilot/settings.local.json");
    std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    let unrelated_hook = serde_json::json!({
        "type": "command",
        "bash": "unrelated-command",
    });
    let original = serde_json::json!({
        "version": 1,
        "hooks": { "unrelated-event": [unrelated_hook.clone()] }
    });
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&original).unwrap(),
    )
    .unwrap();
    let app = HarnessIntegrationApplication::new(state);

    let installed = app
        .mutate(IntegrationMutationRequest::Harness {
            harness_id: "gh-copilot".into(),
            mutation: IntegrationMutation::Install,
        })
        .await
        .unwrap();
    let IntegrationInspection::Harness(installed) = installed else {
        panic!("expected the legacy alias to resolve to the Copilot harness");
    };
    assert_eq!(installed.harness_id, "copilot");
    assert_eq!(installed.registration, IntegrationRegistration::Installed);
    assert!(installed.enabled);

    let inspected = app
        .inspect(IntegrationTarget::Harness("gh-copilot".into()))
        .await
        .unwrap();
    let IntegrationInspection::Harness(inspected) = inspected else {
        panic!("expected the legacy alias to resolve to the Copilot status");
    };
    assert_eq!(inspected.harness_id, "copilot");
    assert!(inspected.enabled);

    let settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(settings_path).unwrap()).unwrap();
    assert_eq!(
        settings["hooks"]["unrelated-event"],
        serde_json::json!([unrelated_hook])
    );
}

#[tokio::test]
#[cfg(unix)]
async fn codex_owned_install_preserves_needs_trust_with_compatible_tool() {
    let dir = tempfile::tempdir().unwrap();
    git2::Repository::init(dir.path()).unwrap();
    std::fs::write(dir.path().join(".gitignore"), ".codex/hooks.json\n").unwrap();
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["codex".into()])
        .unwrap();
    let bin_dir = tempfile::tempdir().unwrap();
    install_fake_command(
        bin_dir.path(),
        "codex",
        "#!/bin/sh\necho 'codex-cli 0.114.0'\n",
    );
    let _fake_path = FakePath::prepend(bin_dir.path());
    let app = HarnessIntegrationApplication::new(state);

    app.mutate(IntegrationMutationRequest::Harness {
        harness_id: "codex".into(),
        mutation: IntegrationMutation::Install,
    })
    .await
    .unwrap();
    let result = app
        .inspect(IntegrationTarget::Harness("codex".into()))
        .await
        .unwrap();
    let IntegrationInspection::Harness(status) = result else {
        panic!()
    };
    assert_eq!(status.registration, IntegrationRegistration::Installed);
    assert_eq!(status.activation, IntegrationActivation::NeedsTrust);
    assert!(status.tool_detected);
    assert!(!status
        .diagnostics
        .iter()
        .any(|d| d.code == "unsupported_tool_version"));
}

#[tokio::test]
#[cfg(unix)]
async fn version_gate_reports_below_and_above_minimum_without_changing_registration() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["copilot".into()])
        .unwrap();
    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            document.overrides.insert(
                "copilot".into(),
                HarnessPatch {
                    min_version: Some(Some(VersionRequirement { min: (99, 0, 0) })),
                    ..Default::default()
                },
            );
            Ok(())
        })
        .unwrap();
    let bin_dir = tempfile::tempdir().unwrap();
    install_fake_command(
        bin_dir.path(),
        "copilot",
        "#!/bin/sh\necho 'copilot-cli 1.0.0'\n",
    );
    let _fake_path = FakePath::prepend(bin_dir.path());
    let app = HarnessIntegrationApplication::new(state.clone());
    app.mutate(IntegrationMutationRequest::Harness {
        harness_id: "copilot".into(),
        mutation: IntegrationMutation::Install,
    })
    .await
    .unwrap();
    let result = app
        .inspect(IntegrationTarget::Harness("copilot".into()))
        .await
        .unwrap();
    let IntegrationInspection::Harness(status) = result else {
        panic!()
    };
    assert!(status.tool_detected);
    assert_eq!(status.registration, IntegrationRegistration::Installed);
    assert_eq!(status.activation, IntegrationActivation::Unknown);
    assert!(status
        .diagnostics
        .iter()
        .any(|d| d.code == "unsupported_tool_version"));

    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            document.overrides.insert(
                "copilot".into(),
                HarnessPatch {
                    min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                    ..Default::default()
                },
            );
            Ok(())
        })
        .unwrap();
    let result = app
        .inspect(IntegrationTarget::Harness("copilot".into()))
        .await
        .unwrap();
    let IntegrationInspection::Harness(status) = result else {
        panic!()
    };
    assert_eq!(status.registration, IntegrationRegistration::Installed);
    assert_eq!(status.activation, IntegrationActivation::Active);
}

#[tokio::test]
#[cfg(unix)]
async fn repeated_inspection_reuses_version_probe_and_workspace_switch_invalidates_it() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            document.overrides.insert(
                "copilot".into(),
                HarnessPatch {
                    min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                    ..Default::default()
                },
            );
            Ok(())
        })
        .unwrap();
    let bin_dir = tempfile::tempdir().unwrap();
    let counter = bin_dir.path().join("probe-count.txt");
    install_fake_command(
        bin_dir.path(),
        "copilot",
        &format!(
            "#!/bin/sh\nprintf '1.2.3\\n'\nprintf 'probe\\n' >> '{}'\n",
            counter.display()
        ),
    );
    let _fake_path = FakePath::prepend(bin_dir.path());
    let app = HarnessIntegrationApplication::new(state.clone());
    for _ in 0..2 {
        app.inspect(IntegrationTarget::Harness("copilot".into()))
            .await
            .unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&counter).unwrap().lines().count(),
        1
    );
    let next_workspace = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(next_workspace.path());
    swap_workspace(&state, next_workspace.path());
    app.inspect(IntegrationTarget::Harness("copilot".into()))
        .await
        .unwrap();
    assert_eq!(std::fs::read_to_string(counter).unwrap().lines().count(), 2);
}

#[tokio::test]
#[cfg(unix)]
async fn harness_definition_edit_invalidates_the_version_probe_cache() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            document.overrides.insert(
                "copilot".into(),
                HarnessPatch {
                    min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                    ..Default::default()
                },
            );
            Ok(())
        })
        .unwrap();
    let bin_dir = tempfile::tempdir().unwrap();
    let counter = bin_dir.path().join("probe-count.txt");
    install_fake_command(
        bin_dir.path(),
        "copilot",
        &format!(
            "#!/bin/sh\nprintf '1.2.3\\n'\nprintf 'probe\\n' >> '{}'\n",
            counter.display()
        ),
    );
    let _fake_path = FakePath::prepend(bin_dir.path());
    let app = HarnessIntegrationApplication::new(state.clone());
    app.inspect(IntegrationTarget::Harness("copilot".into()))
        .await
        .unwrap();
    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            document.overrides.insert(
                "copilot".into(),
                HarnessPatch {
                    min_version: Some(Some(VersionRequirement { min: (0, 0, 2) })),
                    ..Default::default()
                },
            );
            Ok(())
        })
        .unwrap();
    state.bump_harness_probe_generation();
    app.inspect(IntegrationTarget::Harness("copilot".into()))
        .await
        .unwrap();
    assert_eq!(std::fs::read_to_string(counter).unwrap().lines().count(), 2);
}

#[tokio::test]
#[cfg(unix)]
async fn slow_version_probe_does_not_block_an_unrelated_inspection() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            document.overrides.insert(
                "copilot".into(),
                HarnessPatch {
                    min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                    ..Default::default()
                },
            );
            Ok(())
        })
        .unwrap();
    let bin_dir = tempfile::tempdir().unwrap();
    install_fake_command(bin_dir.path(), "copilot", "#!/bin/sh\nexec sleep 30\n");
    let _fake_path = FakePath::prepend(bin_dir.path());
    let app = Arc::new(HarnessIntegrationApplication::new(state));
    let slow_app = app.clone();
    let slow = tokio::spawn(async move {
        slow_app
            .inspect(IntegrationTarget::Harness("copilot".into()))
            .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let start = std::time::Instant::now();
    let concurrent = app
        .inspect(IntegrationTarget::Harness("claude-code".into()))
        .await;
    assert!(concurrent.is_ok());
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
    assert!(slow.await.unwrap().is_ok());
}

#[tokio::test]
async fn unsupported_and_limited_integrations_keep_their_declared_coverage() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["aider".into(), "generic-shell".into()])
        .unwrap();
    let app = HarnessIntegrationApplication::new(state);
    let aider = app
        .inspect(IntegrationTarget::Harness("aider".into()))
        .await
        .unwrap();
    let IntegrationInspection::Harness(aider) = aider else {
        panic!()
    };
    assert_eq!(aider.coverage, IntegrationCoverage::Limited);
    assert!(aider.enabled);
    let shell = app
        .inspect(IntegrationTarget::Harness("generic-shell".into()))
        .await
        .unwrap();
    let IntegrationInspection::Harness(shell) = shell else {
        panic!()
    };
    assert_eq!(shell.registration, IntegrationRegistration::Unsupported);
    assert!(shell.enabled);
    assert_eq!(shell.activation, IntegrationActivation::NotApplicable);
}

#[tokio::test]
async fn grouped_listing_projects_one_shared_target_to_its_active_consumers() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            let builtins = crate::harness::definition::BuiltinDocument::parse(
                crate::harness::definition::EMBEDDED_BUILTINS,
            )
            .unwrap();
            let mut local = builtins
                .builtins
                .iter()
                .find(|h| h.id == "copilot")
                .unwrap()
                .clone();
            local.id = "copilot-local".into();
            local.name = "Copilot Local".into();
            local.session_signals = None;
            local.integration = None;
            document.custom.push(local);
            document
                .set_compatibility_profile(
                    "copilot-local",
                    crate::harness::compatibility::CompatibilityProfile::Copilot,
                )
                .unwrap();
            Ok(())
        })
        .unwrap();
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["copilot".into(), "copilot-local".into()])
        .unwrap();
    let app = HarnessIntegrationApplication::new(state.clone());
    let all = app.list_workspace().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].key.adapter_id, "copilot");
    assert_eq!(
        all[0]
            .consumers
            .iter()
            .map(|c| c.harness_id.as_str())
            .collect::<Vec<_>>(),
        ["copilot", "copilot-local"]
    );
    SessionApplication::new(state)
        .set_active_harnesses(vec!["copilot-local".into()])
        .unwrap();
    let one = app.list_workspace().await.unwrap();
    assert_eq!(one[0].consumers.len(), 1);
    assert_eq!(one[0].consumers[0].harness_id, "copilot-local");
}

#[tokio::test]
async fn workspace_switch_after_probe_rejects_the_old_target() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    let next = tempfile::tempdir().unwrap();
    init_git_workspace_with_claude_settings_ignored(next.path());
    let next_path = next.path().to_path_buf();
    let switch_state = state.clone();
    let app = HarnessIntegrationApplication {
        state,
        revocation_result_hook: None,
        group_revalidation_hook: None,
        probe_revalidation_hook: Some(Arc::new(move |_| swap_workspace(&switch_state, &next_path))),
    };
    let result = app
        .inspect(IntegrationTarget::Harness("claude-code".into()))
        .await;
    assert!(matches!(
        result,
        Err(IntegrationApplicationError::WorkspaceChanged)
    ));
    assert!(!next.path().join(".claude/settings.local.json").exists());
}

#[tokio::test]
async fn harness_definition_change_after_probe_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());
    let edit_state = state.clone();
    let app = HarnessIntegrationApplication {
        state,
        revocation_result_hook: None,
        group_revalidation_hook: None,
        probe_revalidation_hook: Some(Arc::new(move |_| {
            edit_state
                .harness_store
                .mutate(&edit_state.harness_catalog, |document| {
                    document.overrides.insert(
                        "copilot".into(),
                        HarnessPatch {
                            launch: Some(LaunchPatch {
                                command: Some("changed-copilot".into()),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    );
                    Ok(())
                })
                .unwrap();
        })),
    };
    let result = app
        .inspect(IntegrationTarget::Harness("copilot".into()))
        .await;
    assert!(matches!(
        result,
        Err(IntegrationApplicationError::DefinitionChanged)
    ));
}
