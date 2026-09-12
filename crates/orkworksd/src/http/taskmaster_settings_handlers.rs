use crate::http::ErrorResponse;
use crate::taskmaster::provider_catalog::{self, TaskmasterProvider};
use crate::taskmaster::runtime::{
    KnowledgeBundle, TaskmasterRuntime, TaskmasterSettings, TaskmasterStatus,
};
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

fn runtime_for(state: &AppState) -> TaskmasterRuntime {
    #[cfg(test)]
    let root = state
        .workspace
        .lock()
        .expect("workspace lock poisoned")
        .as_ref()
        .map(|workspace| workspace.metadata.root_path().join("taskmaster"))
        .unwrap_or_else(|| std::env::temp_dir().join("orkworks-taskmaster-test"));
    #[cfg(not(test))]
    let root = crate::taskmaster::runtime::taskmaster_global_dir()
        .unwrap_or_else(|| std::env::temp_dir().join("orkworks-taskmaster"));
    TaskmasterRuntime::open(root)
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
                runtime.replace_settings(settings)
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

pub(crate) async fn post_taskmaster_knowledge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(bundle): Json<KnowledgeBundle>,
) -> Response {
    if let Err(status) = authorize_taskmaster_request(&headers) {
        return status.into_response();
    }
    let runtime = runtime_for(&state);
    match runtime.activate_knowledge(bundle) {
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
                "ready"
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

    fn authorized_headers() -> HeaderMap {
        std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", "taskmaster-test-token");
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-orkworks-open-plan-token",
            "taskmaster-test-token".parse().unwrap(),
        );
        headers
    }

    #[tokio::test]
    async fn model_validation_uses_resolved_transport_not_provider_id() {
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
        assert_eq!(status_for(&state, &runtime).analysis_status, "ready");
        assert_eq!(runtime.status(None).remaining_evaluations, 8);
    }
}
