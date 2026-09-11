use crate::{
    http::ErrorResponse,
    taskmaster::{
        inference_approval::{self, ApprovalError, ApprovalRequest},
        inference_trust::InferenceTrustStore,
    },
    AppState,
};
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

pub(crate) async fn get_inference_trust(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(status) = super::taskmaster_settings_handlers::authorize_taskmaster_request(&headers)
    {
        return status.into_response();
    }
    let trust = match trust_store(&state) {
        Ok(trust) => trust,
        Err(error) => return failure(error),
    };
    match tokio::task::spawn_blocking(move || {
        inference_approval::inspect_adapters(&state.harness_store, &trust)
    })
    .await
    {
        Ok(Ok(adapters)) => Json(serde_json::json!({"adapters": adapters})).into_response(),
        Ok(Err(error)) => failure(error),
        Err(_) => failure(ApprovalError::Unavailable),
    }
}
pub(crate) async fn post_inference_trust(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(status) = super::taskmaster_settings_handlers::authorize_taskmaster_request(&headers)
    {
        return status.into_response();
    }
    let request: ApprovalRequest =
        match crate::harness::definition::parse_strict_json(&body, 8 * 1024) {
            Ok(request) => request,
            Err(_) => return failure(ApprovalError::Invalid),
        };
    let trust = match trust_store(&state) {
        Ok(trust) => trust,
        Err(error) => return failure(error),
    };
    match tokio::task::spawn_blocking(move || {
        inference_approval::change_approval(&state.harness_store, &trust, request)
    })
    .await
    {
        Ok(Ok(())) => Json(serde_json::json!({"ok": true})).into_response(),
        Ok(Err(error)) => failure(error),
        Err(_) => failure(ApprovalError::Unavailable),
    }
}

pub(super) fn trust_store(_state: &AppState) -> Result<InferenceTrustStore, ApprovalError> {
    #[cfg(test)]
    let root = _state
        .workspace
        .lock()
        .map_err(|_| ApprovalError::Unavailable)?
        .as_ref()
        .map(|workspace| workspace.metadata.root_path().join("taskmaster"));
    #[cfg(not(test))]
    let root = crate::taskmaster::runtime::taskmaster_global_dir();
    root.map(InferenceTrustStore::new)
        .ok_or(ApprovalError::Unavailable)
}

fn failure(error: ApprovalError) -> Response {
    let (status, message) = match error {
        ApprovalError::Conflict => (
            StatusCode::CONFLICT,
            "Adapter or approval changed. Refresh and review it again.",
        ),
        ApprovalError::Invalid => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid inference approval request.",
        ),
        ApprovalError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Inference approval data is unavailable.",
        ),
    };
    (
        status,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_app_state_with_workspace;
    use serde_json::{json, Value};

    async fn adapter_view(state: Arc<AppState>, headers: HeaderMap) -> Value {
        let response = get_inference_trust(State(state), headers).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        serde_json::from_slice::<Value>(&bytes).unwrap()["adapters"][0].clone()
    }

    async fn change(state: Arc<AppState>, headers: HeaderMap, request: &Value) -> StatusCode {
        post_inference_trust(
            State(state),
            headers,
            Bytes::from(serde_json::to_vec(request).unwrap()),
        )
        .await
        .status()
    }

    #[tokio::test]
    async fn inference_trust_routes_cover_approval_conflict_unavailable_revocation_and_corruption()
    {
        std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", "taskmaster-test-token");
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let mut raw = json!({"id":"custom-infer","name":"Custom","launch":{"kind":"platform-shell","login":false},"inference":{"kind":"command","command":std::env::current_exe().unwrap(),"args":["{model}"],"input":"stdin","output":"result-json-v1"}});
        let definition =
            crate::harness::definition::parse_custom_definition(&serde_json::to_vec(&raw).unwrap())
                .unwrap();
        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.custom.push(definition);
                Ok(())
            })
            .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-orkworks-open-plan-token",
            "taskmaster-test-token".parse().unwrap(),
        );
        let view = adapter_view(state.clone(), headers.clone()).await;
        assert_eq!(view["state"], "approval_required");
        let request =
            json!({"harnessId":view["id"],"action":"approve","expectedRevision":view["revision"]});
        let mut malformed = request.clone();
        malformed["expectedRevision"]["digest"] = json!("bad");
        assert_eq!(
            change(state.clone(), headers.clone(), &malformed).await,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            change(state.clone(), headers.clone(), &request).await,
            StatusCode::OK
        );
        assert_eq!(
            change(state.clone(), headers.clone(), &request).await,
            StatusCode::CONFLICT
        );
        assert_eq!(
            adapter_view(state.clone(), headers.clone()).await["state"],
            "approved"
        );

        raw["inference"]["command"] = json!(dir.path().join("missing-executable"));
        let definition =
            crate::harness::definition::parse_custom_definition(&serde_json::to_vec(&raw).unwrap())
                .unwrap();
        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.custom[0] = definition;
                Ok(())
            })
            .unwrap();
        let view = adapter_view(state.clone(), headers.clone()).await;
        assert_eq!(view["state"], "unavailable");
        assert!(view["revision"]["digest"].is_null());
        let request =
            json!({"harnessId":view["id"],"action":"revoke","expectedRevision":view["revision"]});
        assert_eq!(
            change(state.clone(), headers.clone(), &request).await,
            StatusCode::OK
        );
        let view = adapter_view(state.clone(), headers.clone()).await;
        let request =
            json!({"harnessId":view["id"],"action":"revoke","expectedRevision":view["revision"]});
        let trust_path = dir
            .path()
            .join(".orkworks-test/taskmaster/inference-trust.json");
        std::fs::write(&trust_path, b"corrupt").unwrap();
        assert_eq!(
            get_inference_trust(State(state.clone()), headers.clone())
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            change(state, headers, &request).await,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(std::fs::read(&trust_path).unwrap(), b"corrupt");
    }

    #[tokio::test]
    async fn inference_trust_routes_reject_session_authority_and_malformed_privileged_requests() {
        std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", "taskmaster-test-token");
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let mut session = HeaderMap::new();
        session.insert(
            "authorization",
            "Bearer session-report-token".parse().unwrap(),
        );
        assert_eq!(
            get_inference_trust(State(state.clone()), session.clone())
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            post_inference_trust(State(state.clone()), session, Bytes::from_static(b"{}"))
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-orkworks-open-plan-token",
            "taskmaster-test-token".parse().unwrap(),
        );
        assert_eq!(
            get_inference_trust(State(state.clone()), headers.clone())
                .await
                .status(),
            StatusCode::OK
        );
        for raw in [
            r#"{"action":"approve","action":"revoke"}"#,
            r#"{"trusted":true}"#,
        ] {
            assert_eq!(
                post_inference_trust(
                    State(state.clone()),
                    headers.clone(),
                    Bytes::copy_from_slice(raw.as_bytes())
                )
                .await
                .status(),
                StatusCode::UNPROCESSABLE_ENTITY
            );
        }
    }
}
