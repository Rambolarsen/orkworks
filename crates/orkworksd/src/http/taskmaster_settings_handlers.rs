use crate::http::ErrorResponse;
use crate::providers::{native_inference::NativeProfile, ProviderOperationErrorResponse};
use crate::taskmaster::provider_catalog::{self, TaskmasterProvider};
use crate::taskmaster::runtime::{TaskmasterRuntime, TaskmasterSettings, TaskmasterStatus};
use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

#[derive(serde::Serialize)]
struct SettingsStatus {
    #[serde(flatten)]
    status: TaskmasterStatus,
    providers: Vec<TaskmasterProvider>,
}

impl std::ops::Deref for SettingsStatus {
    type Target = TaskmasterStatus;
    fn deref(&self) -> &Self::Target {
        &self.status
    }
}

fn workspace_path(state: &AppState) -> Option<std::path::PathBuf> {
    state
        .workspace
        .lock()
        .expect("workspace lock poisoned")
        .as_ref()
        .map(|workspace| workspace.path.clone())
}

pub(super) fn runtime_for(state: &AppState) -> TaskmasterRuntime {
    crate::taskmaster::assessment_access::runtime_for(state)
}

pub(super) fn authorize_taskmaster_request(headers: &HeaderMap) -> Result<(), StatusCode> {
    let Ok(token) = std::env::var("ORKWORKS_OPEN_PLAN_TOKEN") else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };
    if token.is_empty() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    (Some(token.as_str())
        == headers
            .get("x-orkworks-open-plan-token")
            .and_then(|value| value.to_str().ok()))
    .then_some(())
    .ok_or(StatusCode::UNAUTHORIZED)
}

fn status_for(state: &AppState, runtime: &TaskmasterRuntime) -> SettingsStatus {
    let mut status = runtime.status(workspace_path(state).as_deref());
    let trust = super::inference_trust_handlers::trust_store(state).ok();
    let catalog = provider_catalog::inspect(&state.harness_store, trust.as_ref());
    if status.analysis_status == "ready" {
        status.analysis_status = match (&catalog, &status.effective_settings.selection) {
            (Ok(providers), Some(selection)) => {
                provider_catalog::evaluation_availability(providers, selection).as_str()
            }
            _ => "unavailable",
        }
        .into();
    }
    if status.analysis_status == "ready" && runtime.brain_knowledge_availability().is_err() {
        status.analysis_status = "knowledge_unavailable".into();
    }
    SettingsStatus {
        status,
        providers: catalog.unwrap_or_default(),
    }
}

pub(crate) async fn get_taskmaster_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(status) = authorize_taskmaster_request(&headers) {
        return status.into_response();
    }
    Json(status_for(&state, &runtime_for(&state))).into_response()
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TaskmasterModelsRequest {
    provider: String,
    #[serde(default)]
    ollama_base_url: Option<String>,
}

#[derive(serde::Serialize)]
struct TaskmasterModelsResponse {
    models: Vec<String>,
}

pub(crate) async fn refresh_taskmaster_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<TaskmasterModelsRequest>,
) -> Response {
    if let Err(status) = authorize_taskmaster_request(&headers) {
        return status.into_response();
    }
    let snapshot = match state.harness_store.snapshot() {
        Ok(snapshot) => snapshot,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let Some(profile) = NativeProfile::resolve(&snapshot, request.provider.trim()) else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: "Taskmaster model discovery is available only for built-in Codex and Ollama providers".into(),
            }),
        )
            .into_response();
    };
    if profile == NativeProfile::Claude {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: "Taskmaster model discovery is not supported for Claude Code".into(),
            }),
        )
            .into_response();
    }
    let providers = state.providers.clone();
    match tokio::task::spawn_blocking(move || {
        providers.discover_taskmaster_models(profile, request.ollama_base_url.as_deref())
    })
    .await
    {
        Ok(Ok(models)) => Json(TaskmasterModelsResponse { models }).into_response(),
        Ok(Err(error)) => (
            StatusCode::BAD_GATEWAY,
            Json(ProviderOperationErrorResponse { error }),
        )
            .into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

pub(crate) async fn get_taskmaster_run_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(status) = authorize_taskmaster_request(&headers) {
        return status.into_response();
    }
    let runtime = runtime_for(&state);
    match runtime.run_status(workspace_path(&state).as_deref()) {
        Ok(status) => {
            let Ok(mut value) = serde_json::to_value(status) else {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            };
            value["assessment"] =
                crate::taskmaster::assessment_access::status_for(&state, &runtime);
            Json(value).into_response()
        }
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse { error }),
        )
            .into_response(),
    }
}

pub(crate) async fn set_taskmaster_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(settings): Json<TaskmasterSettings>,
) -> Response {
    if let Err(status) = authorize_taskmaster_request(&headers) {
        return status.into_response();
    }
    let runtime = runtime_for(&state);
    let trust = super::inference_trust_handlers::trust_store(&state).ok();
    let validation =
        provider_catalog::inspect(&state.harness_store, trust.as_ref())
            .map_err(|_| "Taskmaster provider configuration is unavailable".to_string())
            .and_then(|providers| {
                use crate::taskmaster::runtime::SelectionOverride;
                let selections = settings.selection.iter().chain(
                    settings
                        .workspace_overrides
                        .values()
                        .filter_map(|override_| match &override_.selection {
                            SelectionOverride::Set(selection) => Some(selection),
                            _ => None,
                        }),
                );
                for selection in selections {
                    use crate::providers::native_inference::NativeProfile;
                    use provider_catalog::Transport;
                    if providers.iter().find(|provider| provider.id == selection.provider)
                        .is_some_and(|provider| matches!(&provider.transport, Transport::Native(revision) if matches!(revision.profile, NativeProfile::Codex | NativeProfile::Claude)))
                        && !crate::providers::valid_native_model(&selection.model)
                    {
                        return Err("Native Taskmaster model IDs must use ASCII letters, digits, or -._/:@+ (1–256 bytes)".into());
                    }
                    if selection.reasoning_effort.is_some()
                        && providers
                            .iter()
                            .find(|provider| provider.id == selection.provider)
                            .is_some_and(|provider| !provider.supports_reasoning_effort)
                    {
                        return Err("Taskmaster provider does not support reasoning effort".into());
                    }
                }
                runtime.replace_settings_with_reconcile(settings, |_, settings| {
                    crate::taskmaster::assessment_access::reconcile_settings(&state, settings)
                })
            });
    match validation {
        Ok(()) => Json(status_for(&state, &runtime)).into_response(),
        Err(error) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse { error }),
        )
            .into_response(),
    }
}

#[cfg(test)]
#[derive(Clone)]
pub(crate) struct KnowledgeTestKey(pub(crate) &'static str);

pub(crate) async fn post_taskmaster_knowledge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    #[cfg(test)] fixture_key: Option<axum::Extension<KnowledgeTestKey>>,
    body: axum::body::Bytes,
) -> Response {
    if let Err(status) = authorize_taskmaster_request(&headers) {
        return status.into_response();
    }
    let runtime = runtime_for(&state);
    #[cfg(test)]
    let runtime = fixture_key.map_or(runtime, |key| {
        TaskmasterRuntime::open_with_test_key(runtime_for(&state).root_path().into(), key.0 .0)
    });
    match runtime.activate_knowledge(&body) {
        Ok(()) => Json(status_for(&state, &runtime)).into_response(),
        Err(error) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse { error }),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_app_state_with_workspace;
    use axum::http::HeaderMap;

    #[test]
    fn analysis_status_uses_shared_capabilities_without_overriding_disabled() {
        let directory = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(directory.path());
        let runtime = runtime_for(&state);
        for provider in [
            "ollama",
            "codex",
            "claude-code",
            "copilot",
            "opencode",
            "aider",
            "unknown",
        ] {
            let mut settings = TaskmasterSettings::default();
            settings.enabled = true;
            settings.selection = Some(
                serde_json::from_value(serde_json::json!({
                    "provider": provider, "model": "selected-model"
                }))
                .unwrap(),
            );
            runtime.replace_settings(settings.clone()).unwrap();
            let expected = if state.providers.supports_inference_only(provider) {
                "knowledge_unavailable"
            } else {
                "unsupported_capability"
            };
            assert_eq!(
                status_for(&state, &runtime).analysis_status,
                expected,
                "{provider}"
            );
            settings.enabled = false;
            runtime.replace_settings(settings).unwrap();
            assert_eq!(status_for(&state, &runtime).analysis_status, "disabled");
        }
    }

    #[test]
    fn stored_ollama_effort_is_not_reported_ready() {
        let directory = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(directory.path());
        let runtime = runtime_for(&state);
        let mut settings = TaskmasterSettings::default();
        settings.enabled = true;
        settings.selection = Some(
            serde_json::from_value(
                serde_json::json!({"provider":"ollama","model":"opaque","reasoningEffort":"high"}),
            )
            .unwrap(),
        );
        runtime.replace_settings(settings).unwrap();
        assert_eq!(
            status_for(&state, &runtime).analysis_status,
            "unsupported_reasoning_effort"
        );
    }

    #[tokio::test]
    async fn settings_reject_an_invalid_daily_limit_before_persistence() {
        let _token = crate::test_support::open_plan_token::set("taskmaster-test-token").await;
        let directory = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(directory.path());
        let mut settings = TaskmasterSettings::default();
        settings.daily_evaluation_limit = 0;

        assert_eq!(
            set_taskmaster_settings(State(state.clone()), authorized_headers(), Json(settings))
                .await
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            runtime_for(&state)
                .status(Some(directory.path()))
                .settings
                .daily_evaluation_limit,
            8
        );
    }

    #[tokio::test]
    async fn settings_replacement_reconciles_assessment_access_before_returning() {
        let _token = crate::test_support::open_plan_token::set("taskmaster-test-token").await;
        let directory = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(directory.path());
        let metadata_root = {
            let workspace_guard = state.workspace.lock().unwrap();
            let workspace = workspace_guard.as_ref().unwrap();
            let metadata_root = workspace.metadata.root_path();
            let record = crate::taskmaster::assessment_report::AssessmentRecord {
                format_version: 1,
                input_manifest: None,
                phase: None,
                id: "assessment-settings-test".into(),
                state: crate::taskmaster::assessment_report::AssessmentState::Succeeded,
                started_at: "2026-10-17T10:00:00Z".into(),
                completed_at: Some("2026-10-17T10:01:00Z".into()),
                report: Some(crate::taskmaster::assessment_report::AssessmentReport {
                    id: "assessment-settings-test".into(),
                    workspace_path: workspace.path.display().to_string(),
                    workspace_instance: workspace.workflow_observations.instance_id(),
                    observed_at: "2026-10-17T10:00:00Z".into(),
                    provider: "codex".into(),
                    model: "fixture".into(),
                    bundle_version: "v1".into(),
                    knowledge_identity: serde_json::json!({"version":"v1"}),
                    input_fingerprint: "f".repeat(64),
                    context_level: crate::taskmaster::runtime::ContextLevel::SourceCode,
                    excluded_paths: Vec::new(),
                    outcome: "proposed".into(),
                    summary: "Source-derived assessment summary".into(),
                    proposal: Some(crate::taskmaster::assessment_report::AssessmentProposal {
                        target_surface: crate::taskmaster::TargetSurface::Documentation,
                        title: "Improve the guide".into(),
                        summary: "Document the workflow".into(),
                    }),
                    no_proposal_reason: None,
                    unverified_checks: vec!["Check the generated workflow".into()],
                    repository_evidence: vec![crate::taskmaster::RepositoryEvidence {
                        path: "src/private.rs".into(),
                        sha256: "a".repeat(64),
                        excerpt: "Private source excerpt".into(),
                        observed_at: "2026-10-17T10:00:00Z".into(),
                    }],
                    knowledge_evidence: Vec::new(),
                    recommendation_id: None,
                }),
                pending_recommendation: None,
                error_summary: None,
                reused: false,
            };
            crate::taskmaster::assessment_store::AssessmentStore::new(&metadata_root)
                .save(&record)
                .unwrap();
            metadata_root
        };

        let response = set_taskmaster_settings(
            State(state.clone()),
            authorized_headers(),
            Json(TaskmasterSettings::default()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let saved = crate::taskmaster::assessment_store::AssessmentStore::new(&metadata_root)
            .load::<crate::taskmaster::assessment_report::AssessmentRecord>()
            .unwrap()
            .unwrap();
        let report = saved.report.unwrap();
        assert_eq!(
            report.summary,
            "Assessment evidence was redacted after context access changed."
        );
        assert!(report.proposal.is_none());
        assert!(report.unverified_checks.is_empty());
        assert!(report.repository_evidence.is_empty());
    }

    fn authorized_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-orkworks-open-plan-token",
            "taskmaster-test-token".parse().unwrap(),
        );
        headers
    }

    #[tokio::test]
    async fn analyze_reports_the_privacy_prerequisite_even_with_background_disabled() {
        let _token = crate::test_support::open_plan_token::set("taskmaster-test-token").await;
        let directory = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(directory.path());
        let runtime = runtime_for(&state);
        let mut settings = TaskmasterSettings::default();
        settings.enabled = false;
        settings.selection = Some(
            serde_json::from_value(serde_json::json!({
                "provider":"ollama", "model":"test-model"
            }))
            .unwrap(),
        );
        runtime.replace_settings(settings).unwrap();
        let response = super::super::taskmaster_handlers::analyze_taskmaster(
            State(state),
            authorized_headers(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["status"], "unavailable");
        assert!(body["message"]
            .as_str()
            .unwrap()
            .contains("verified reference knowledge"));
        assert_eq!(
            runtime.status(Some(directory.path())).remaining_evaluations,
            8
        );
        assert!(runtime
            .run_status(Some(directory.path()))
            .unwrap()
            .active_attempt
            .is_none());
    }

    #[tokio::test]
    async fn unavailable_manual_analysis_retries_deferred_run_recovery() {
        let _token = crate::test_support::open_plan_token::set("taskmaster-test-token").await;
        use crate::taskmaster::runtime::{TaskmasterRunOutcomeState, TaskmasterRunTrigger};
        use std::sync::atomic::{AtomicUsize, Ordering};
        for running in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let mut state = test_app_state_with_workspace(directory.path());
            let calls = Arc::new(AtomicUsize::new(0));
            Arc::get_mut(&mut state).unwrap().providers =
                crate::providers::ProviderManager::for_tests(
                    crate::providers::ProviderSettingsPayload::default(),
                    vec![crate::providers::FakeProvider::new("ollama").with_counter(calls.clone())],
                );
            let runtime = runtime_for(&state);
            let mut settings = TaskmasterSettings::default();
            settings.enabled = false;
            settings.selection = Some(
                serde_json::from_value(serde_json::json!({
                    "provider":"ollama", "model":"test-model"
                }))
                .unwrap(),
            );
            runtime.replace_settings(settings).unwrap();
            let mut other_instance_lease = Some(runtime.try_analysis_lease().unwrap().unwrap());
            let id = runtime
                .queue_run(
                    directory.path(),
                    TaskmasterRunTrigger::Background,
                    "ollama",
                    "test-model",
                )
                .unwrap();
            if running {
                assert!(runtime
                    .mark_run_running(directory.path(), id, "ollama", "test-model")
                    .unwrap());
            }
            for lease_available in [false, true] {
                if lease_available {
                    drop(other_instance_lease.take());
                }
                let response = super::super::taskmaster_handlers::analyze_taskmaster(
                    State(state.clone()),
                    authorized_headers(),
                )
                .await;
                assert_eq!(response.status(), StatusCode::OK);
                let body = axum::body::to_bytes(response.into_body(), 4096)
                    .await
                    .unwrap();
                let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(body["status"], "unavailable");
                assert!(body["message"]
                    .as_str()
                    .unwrap()
                    .contains("verified reference knowledge"));
                let status = runtime.run_status(Some(directory.path())).unwrap();
                if lease_available {
                    assert!(status.active_attempt.is_none());
                    if running {
                        assert_eq!(
                            status.latest_outcome.unwrap().state,
                            TaskmasterRunOutcomeState::Interrupted
                        );
                    } else {
                        assert!(status.latest_outcome.is_none());
                    }
                } else {
                    assert_eq!(status.active_attempt.unwrap().id, id);
                }
            }
            drop(other_instance_lease);
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(
                runtime.status(Some(directory.path())).remaining_evaluations,
                8
            );
        }
    }

    #[tokio::test]
    async fn run_status_is_authenticated_and_reports_no_workspace_or_corrupt_ledger_unavailable() {
        let _token = crate::test_support::open_plan_token::set("taskmaster-test-token").await;
        let _ = authorized_headers();
        let directory = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(directory.path());
        let status_response =
            get_taskmaster_run_status(State(state.clone()), authorized_headers()).await;
        let status_body = axum::body::to_bytes(status_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let status_value: serde_json::Value = serde_json::from_slice(&status_body).unwrap();
        assert!(status_value.get("assessment").is_some());
        assert_eq!(status_value["assessment"]["state"], "idle");
        assert!(status_value["assessment"].get("report").is_some());
        assert!(status_value["assessment"].get("errorSummary").is_some());
        assert!(status_value["assessment"].get("reused").is_some());
        let mut unauthorized = HeaderMap::new();
        unauthorized.insert("x-orkworks-open-plan-token", "wrong".parse().unwrap());
        assert_eq!(
            get_taskmaster_run_status(State(state.clone()), unauthorized)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );

        state.workspace.lock().unwrap().take();
        assert_eq!(
            get_taskmaster_run_status(State(state.clone()), authorized_headers())
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );

        let state = test_app_state_with_workspace(&directory.path().join("corrupt"));
        let root = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path()
            .join("taskmaster");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("evaluations.json"), "corrupt").unwrap();
        assert_eq!(
            get_taskmaster_run_status(State(state), authorized_headers())
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn model_refresh_rejects_unauthorized_and_non_builtin_providers() {
        let _token = crate::test_support::open_plan_token::set("taskmaster-test-token").await;
        let _ = authorized_headers();
        let directory = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(directory.path());
        let request = || {
            Json(TaskmasterModelsRequest {
                provider: "custom-provider".into(),
                ollama_base_url: None,
            })
        };
        assert_eq!(
            refresh_taskmaster_models(State(state.clone()), HeaderMap::new(), request())
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            refresh_taskmaster_models(State(state.clone()), authorized_headers(), request())
                .await
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            refresh_taskmaster_models(
                State(state),
                authorized_headers(),
                Json(TaskmasterModelsRequest {
                    provider: "claude-code".into(),
                    ollama_base_url: None
                })
            )
            .await
            .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[tokio::test]
    async fn model_validation_uses_resolved_transport_not_provider_id() {
        let _token = crate::test_support::open_plan_token::set("taskmaster-test-token").await;
        for provider in ["codex", "claude-code"] {
            let directory = tempfile::tempdir().unwrap();
            let state = test_app_state_with_workspace(directory.path());
            let mut settings = TaskmasterSettings::default();
            settings.selection = Some(
                serde_json::from_value(serde_json::json!({
                    "provider": provider, "model": "vendor/模型 model"
                }))
                .unwrap(),
            );
            assert_eq!(
                set_taskmaster_settings(
                    State(state.clone()),
                    authorized_headers(),
                    Json(settings.clone())
                )
                .await
                .status(),
                StatusCode::UNPROCESSABLE_ENTITY
            );
            assert!(runtime_for(&state)
                .status(None)
                .settings
                .selection
                .is_none());
            state.harness_store.mutate(&state.harness_catalog, |document| {
                document.overrides.insert(provider.into(), serde_json::from_value(serde_json::json!({
                    "inference":{"kind":"command","command":std::env::current_exe().unwrap(),"args":["{model}"],"input":"stdin","output":"result-json-v1"}
                })).unwrap());
                Ok(())
            }).unwrap();
            assert_eq!(
                set_taskmaster_settings(State(state.clone()), authorized_headers(), Json(settings))
                    .await
                    .status(),
                StatusCode::OK
            );
            assert_eq!(
                runtime_for(&state)
                    .status(None)
                    .settings
                    .selection
                    .unwrap()
                    .model,
                "vendor/模型 model"
            );
        }
    }

    #[tokio::test]
    async fn settings_reject_undeclared_custom_effort_before_persisting() {
        let _token = crate::test_support::open_plan_token::set("taskmaster-test-token").await;
        let directory = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(directory.path());
        state.harness_store.mutate(&state.harness_catalog, |document| {
            document.custom.push(crate::harness::definition::parse_custom_definition(
                &serde_json::to_vec(&serde_json::json!({
                    "id":"custom-infer", "name":"Custom", "launch":{"kind":"platform-shell","login":false},
                    "inference":{"kind":"command","command":std::env::current_exe().unwrap(),"args":["{model}"],"input":"stdin","output":"result-json-v1"}
                })).unwrap()
            ).unwrap());
            Ok(())
        }).unwrap();
        let mut settings = TaskmasterSettings::default();
        settings.selection = Some(serde_json::from_value(serde_json::json!({"provider":"custom-infer","model":"opaque/model","reasoningEffort":"high"})).unwrap());
        let response =
            set_taskmaster_settings(State(state.clone()), authorized_headers(), Json(settings))
                .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(runtime_for(&state)
            .status(None)
            .settings
            .selection
            .is_none());
    }

    #[test]
    fn taskmaster_catalog_lists_inference_without_peon_and_requires_approval_for_ready() {
        use crate::taskmaster::inference_approval::{self, ApprovalAction, ApprovalRequest};
        use crate::taskmaster::inference_trust::InferenceTrustStore;
        let directory = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(directory.path());
        state.harness_store.mutate(&state.harness_catalog, |document| {
            document.custom.push(crate::harness::definition::parse_custom_definition(
                &serde_json::to_vec(&serde_json::json!({
                    "id":"custom-infer", "name":"Custom inference", "launch":{"kind":"platform-shell","login":false},
                    "models":{"kind":"static","models":["vendor/opaque model"]},
                    "inference":{"kind":"command","command":std::env::current_exe().unwrap(),"args":["{model}"],"input":"stdin","output":"result-json-v1"}
                })).unwrap()
            ).unwrap());
            Ok(())
        }).unwrap();
        let runtime = runtime_for(&state);
        let mut settings = TaskmasterSettings::default();
        settings.enabled = true;
        settings.selection = Some(
            serde_json::from_value(
                serde_json::json!({"provider":"custom-infer","model":"vendor/opaque model"}),
            )
            .unwrap(),
        );
        runtime.replace_settings(settings).unwrap();
        let status = serde_json::to_value(status_for(&state, &runtime)).unwrap();
        assert_eq!(status["analysisStatus"], "approval_required");
        let custom = status["providers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|provider| provider["id"] == "custom-infer")
            .unwrap();
        assert_eq!(custom["models"], serde_json::json!(["vendor/opaque model"]));
        assert_eq!(custom["supportsReasoningEffort"], false);
        assert!(!state.providers.supports_inference_only("custom-infer"));
        let trust = InferenceTrustStore::new(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .root_path()
                .join("taskmaster"),
        );
        let view = inference_approval::inspect_adapters(&state.harness_store, &trust)
            .unwrap()
            .pop()
            .unwrap();
        inference_approval::change_approval(
            &state.harness_store,
            &trust,
            ApprovalRequest {
                harness_id: view.id,
                action: ApprovalAction::Approve,
                expected_revision: view.revision,
            },
        )
        .unwrap();
        assert_eq!(
            status_for(&state, &runtime).analysis_status,
            "knowledge_unavailable"
        );
        assert_eq!(runtime.status(None).remaining_evaluations, 8);
    }
}

#[cfg(test)]
mod knowledge_route_tests;
