use crate::harness::definition::parse_strict_json;
use crate::harness::integration::{IntegrationError, IntegrationKey};
use crate::harness::store::HarnessDocumentRevision;
use crate::harness_integration_application::{
    HarnessIntegrationApplication, IntegrationApplicationError, IntegrationInspection,
    IntegrationMutation, IntegrationMutationRequest, IntegrationRevisionExpectation,
    IntegrationTarget,
};
use crate::http::ErrorResponse;
use crate::AppState;
use axum::body::Bytes;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IntegrationRevisionConflictResponse {
    error: &'static str,
    code: &'static str,
    document_revision: Option<HarnessDocumentRevision>,
    active_harness_revision: u64,
}

fn integration_error_response(error: IntegrationError) -> axum::response::Response {
    let status = match &error {
        IntegrationError::NoWorkspace
        | IntegrationError::RevisionChanged
        | IntegrationError::OwnershipAmbiguous => StatusCode::CONFLICT,
        IntegrationError::UnsafeTarget { .. } | IntegrationError::InvalidConfig(_) => {
            StatusCode::BAD_REQUEST
        }
        IntegrationError::LaunchConflict | IntegrationError::Io(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    (
        status,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}

fn application_error_response(error: IntegrationApplicationError) -> axum::response::Response {
    match error {
        IntegrationApplicationError::NoWorkspace => {
            integration_error_response(IntegrationError::NoWorkspace)
        }
        IntegrationApplicationError::UnknownHarness(id) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("unknown harness id \"{id}\""),
            }),
        )
            .into_response(),
        IntegrationApplicationError::UnknownIntegration(key) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!(
                    "unknown integration key {}/{}",
                    key.adapter_id, key.target_id
                ),
            }),
        )
            .into_response(),
        IntegrationApplicationError::WorkspaceChanged => (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "workspace changed during this request; retry".into(),
            }),
        )
            .into_response(),
        IntegrationApplicationError::DefinitionChanged => (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "harness definition changed during this request; retry".into(),
            }),
        )
            .into_response(),
        IntegrationApplicationError::RevisionConflict(conflict) => (
            StatusCode::CONFLICT,
            Json(IntegrationRevisionConflictResponse {
                error: "Integration state changed; reload before retrying.",
                code: "integration_revision_changed",
                document_revision: conflict.document_revision,
                active_harness_revision: conflict.active_harness_revision,
            }),
        )
            .into_response(),
        IntegrationApplicationError::Configuration(error) => integration_error_response(error),
        IntegrationApplicationError::Infrastructure(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error }),
        )
            .into_response(),
        IntegrationApplicationError::AuthorityRevocation => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "integration changed, but prompt authority could not be safely revoked"
                    .into(),
            }),
        )
            .into_response(),
    }
}

fn invalid_integration_request(message: &str) -> axum::response::Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
        .into_response()
}

fn parse_integration_mutation_request(
    body: &Bytes,
) -> Result<IntegrationRevisionExpectation, axum::response::Response> {
    let value = parse_strict_json::<serde_json::Value>(body, 64 * 1024)
        .map_err(|diagnostic| invalid_integration_request(&diagnostic.message))?;
    let object = value.as_object().ok_or_else(|| {
        invalid_integration_request("Integration mutation request must be an object.")
    })?;
    for field in object.keys() {
        if !matches!(
            field.as_str(),
            "expectedDocumentRevision" | "expectedActiveHarnessRevision"
        ) {
            return Err(invalid_integration_request(&format!(
                "Unknown integration mutation field {field}."
            )));
        }
    }
    let document_revision = object
        .get("expectedDocumentRevision")
        .ok_or_else(|| {
            invalid_integration_request("Integration mutation requires expectedDocumentRevision.")
        })
        .and_then(|value| {
            serde_json::from_value(value.clone()).map_err(|error| {
                invalid_integration_request(&format!(
                    "expectedDocumentRevision must be a revision string or null: {error}"
                ))
            })
        })?;
    let active_harness_revision = object
        .get("expectedActiveHarnessRevision")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            invalid_integration_request(
                "Integration mutation requires an unsigned expectedActiveHarnessRevision.",
            )
        })?;
    Ok(IntegrationRevisionExpectation {
        document_revision,
        active_harness_revision,
        workspace_path: None,
    })
}

pub(crate) async fn get_workspace_integrations(
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    match HarnessIntegrationApplication::new(state)
        .list_workspace()
        .await
    {
        Ok(result) => Json(result).into_response(),
        Err(IntegrationApplicationError::AuthorityRevocation) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error:
                    "integration status changed, but prompt authority could not be safely revoked"
                        .into(),
            }),
        )
            .into_response(),
        Err(error) => application_error_response(error),
    }
}

pub(crate) async fn get_grouped_integration_status(
    State(state): State<Arc<AppState>>,
    AxumPath((adapter_id, target_id)): AxumPath<(String, String)>,
) -> axum::response::Response {
    let target = IntegrationTarget::Group(IntegrationKey {
        adapter_id,
        target_id,
    });
    match HarnessIntegrationApplication::new(state)
        .inspect(target)
        .await
    {
        Ok(IntegrationInspection::Group(status)) => Json(status).into_response(),
        Ok(IntegrationInspection::Harness(_)) => unreachable!("group inspection keeps its type"),
        Err(error) => application_error_response(error),
    }
}

pub(crate) async fn install_grouped_integration(
    State(state): State<Arc<AppState>>,
    AxumPath((adapter_id, target_id)): AxumPath<(String, String)>,
    body: Bytes,
) -> axum::response::Response {
    mutate_grouped(
        state,
        adapter_id,
        target_id,
        body,
        IntegrationMutation::Install,
    )
    .await
}

pub(crate) async fn repair_grouped_integration(
    State(state): State<Arc<AppState>>,
    AxumPath((adapter_id, target_id)): AxumPath<(String, String)>,
    body: Bytes,
) -> axum::response::Response {
    mutate_grouped(
        state,
        adapter_id,
        target_id,
        body,
        IntegrationMutation::Repair,
    )
    .await
}

async fn mutate_grouped(
    state: Arc<AppState>,
    adapter_id: String,
    target_id: String,
    body: Bytes,
    mutation: IntegrationMutation,
) -> axum::response::Response {
    let expected = match parse_integration_mutation_request(&body) {
        Ok(expected) => expected,
        Err(response) => return response,
    };
    let request = IntegrationMutationRequest::Group {
        key: IntegrationKey {
            adapter_id,
            target_id,
        },
        mutation,
        expected,
    };
    match HarnessIntegrationApplication::new(state)
        .mutate(request)
        .await
    {
        Ok(IntegrationInspection::Group(status)) => Json(status).into_response(),
        Ok(IntegrationInspection::Harness(_)) => unreachable!("group mutation keeps its type"),
        Err(error) => application_error_response(error),
    }
}

pub(crate) async fn uninstall_grouped_integration(
    State(state): State<Arc<AppState>>,
    AxumPath((adapter_id, target_id)): AxumPath<(String, String)>,
    body: Bytes,
) -> axum::response::Response {
    mutate_grouped(
        state,
        adapter_id,
        target_id,
        body,
        IntegrationMutation::Uninstall,
    )
    .await
}

pub(crate) async fn get_integration_status(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> axum::response::Response {
    match HarnessIntegrationApplication::new(state)
        .inspect(IntegrationTarget::Harness(harness_id))
        .await
    {
        Ok(IntegrationInspection::Harness(status)) => Json(status).into_response(),
        Ok(IntegrationInspection::Group(_)) => unreachable!("harness inspection keeps its type"),
        Err(error) => application_error_response(error),
    }
}

pub(crate) async fn install_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> axum::response::Response {
    mutate_harness(state, harness_id, IntegrationMutation::Install).await
}

pub(crate) async fn uninstall_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> axum::response::Response {
    mutate_harness(state, harness_id, IntegrationMutation::Uninstall).await
}

async fn mutate_harness(
    state: Arc<AppState>,
    harness_id: String,
    mutation: IntegrationMutation,
) -> axum::response::Response {
    let request = IntegrationMutationRequest::Harness {
        harness_id,
        mutation,
    };
    match HarnessIntegrationApplication::new(state)
        .mutate(request)
        .await
    {
        Ok(IntegrationInspection::Harness(status)) => Json(status).into_response(),
        Ok(IntegrationInspection::Group(_)) => unreachable!("harness mutation keeps its type"),
        Err(error) => application_error_response(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::integration::IntegrationRegistration;
    use crate::session_application::SessionApplication;
    use crate::test_support::{test_app_state_with_workspace, FakeHome};
    use axum::extract::Path;
    use axum::response::IntoResponse;

    fn init_git_workspace(workspace: &std::path::Path, ignore: &str) {
        git2::Repository::init(workspace).unwrap();
        std::fs::write(workspace.join(".gitignore"), ignore).unwrap();
    }

    async fn body(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[test]
    fn grouped_mutation_body_requires_both_revision_fields() {
        let error = parse_integration_mutation_request(&Bytes::from_static(b"{}")).unwrap_err();
        assert_eq!(error.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn grouped_mutation_body_rejects_unknown_fields() {
        let error = parse_integration_mutation_request(&Bytes::from_static(
            br#"{"expectedDocumentRevision":null,"expectedActiveHarnessRevision":7,"force":true}"#,
        ))
        .unwrap_err();
        assert_eq!(error.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn revision_conflict_keeps_the_legacy_error_shape() {
        let response = application_error_response(IntegrationApplicationError::RevisionConflict(
            crate::harness_integration_application::IntegrationRevisionConflict {
                document_revision: None,
                active_harness_revision: 7,
            },
        ));
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn missing_workspace_and_unknown_harness_keep_transport_statuses() {
        assert_eq!(
            application_error_response(IntegrationApplicationError::NoWorkspace).status(),
            StatusCode::CONFLICT,
        );
        assert_eq!(
            application_error_response(IntegrationApplicationError::UnknownHarness(
                "missing-tool".into(),
            ))
            .status(),
            StatusCode::NOT_FOUND,
        );
    }

    #[tokio::test]
    async fn legacy_adapter_failure_is_http_error_and_grouped_failure_is_status() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace(
            dir.path(),
            ".claude/settings.local.json\n.github/copilot/settings.local.json\n",
        );
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        std::fs::write(dir.path().join(".claude/settings.local.json"), b"{").unwrap();
        let foreign_path = dir.path().join(".github/copilot/settings.local.json");
        std::fs::create_dir_all(foreign_path.parent().unwrap()).unwrap();
        std::fs::write(
            foreign_path,
            br#"{
                "version": 1,
                "hooks": {
                    "notification": [{
                        "type": "command",
                        "bash": "foreign-reporter",
                        "env": {
                            "ORKWORKS_INTEGRATION_MARKER": "orkworks:harness-integration:v2:foreign"
                        }
                    }]
                }
            }"#,
        )
        .unwrap();
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());
        SessionApplication::new(state.clone())
            .set_active_harnesses(vec!["copilot".into()])
            .unwrap();
        let snapshot = state.harness_store.snapshot().unwrap();
        let active_revision = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|workspace| workspace.metadata.read_workspace_memory())
            .unwrap()
            .active_harness_revision;
        let request = Bytes::from(
            serde_json::json!({
                "expectedDocumentRevision": snapshot.document_revision,
                "expectedActiveHarnessRevision": active_revision,
            })
            .to_string(),
        );

        let legacy = install_integration(State(state.clone()), Path("claude-code".into()))
            .await
            .into_response();
        assert_eq!(legacy.status(), StatusCode::BAD_REQUEST);

        let grouped = uninstall_grouped_integration(
            State(state),
            Path(("copilot".into(), "workspace".into())),
            request,
        )
        .await
        .into_response();
        assert_eq!(grouped.status(), StatusCode::OK);
        let grouped_status = body(grouped).await;
        assert_eq!(
            grouped_status["status"]["registration"],
            serde_json::to_value(IntegrationRegistration::Error).unwrap()
        );
    }
}
