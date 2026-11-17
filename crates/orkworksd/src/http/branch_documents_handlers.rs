//! Thin authenticated document-kind HTTP adapter.
use crate::{branch_documents::DocumentKind, session_application::SessionApplication, AppState};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DocumentQuery {
    pub kind: Option<DocumentKind>,
    pub selection_id: Option<String>,
}

impl DocumentQuery {
    fn valid(&self) -> bool {
        self.selection_id.as_ref().is_none_or(|id| {
            self.kind.is_some()
                && id.len() == 64
                && id
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        })
    }
}
fn failure(message: String) -> Response {
    (
        StatusCode::CONFLICT,
        Json(super::ErrorResponse { error: message }),
    )
        .into_response()
}

pub(crate) async fn content(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    query: Result<Query<DocumentQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    if let Err(status) = super::session_handlers::authorize_plan_request(&headers) {
        return status.into_response();
    }
    let Query(query) = match query {
        Ok(q) => q,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    if !query.valid() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    tokio::task::spawn_blocking(move || {
        match SessionApplication::new(state).read_document_content(
            &id,
            query.kind,
            query.selection_id.as_deref(),
        ) {
            Ok(content) => Json(serde_json::json!({"content":content})).into_response(),
            Err(e) => failure(e),
        }
    })
    .await
    .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
pub(crate) async fn review(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    query: Result<Query<DocumentQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    if let Err(status) = super::session_handlers::authorize_plan_request(&headers) {
        return status.into_response();
    }
    let Query(query) = match query {
        Ok(q) => q,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    if !query.valid() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    match SessionApplication::new(state)
        .request_document_review(&id, query.kind, query.selection_id.as_deref())
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => failure(e),
    }
}
pub(crate) async fn use_latest(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    query: Result<Query<DocumentQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    if let Err(status) = super::session_handlers::authorize_plan_request(&headers) {
        return status.into_response();
    }
    let query = match query {
        Ok(Query(query)) if query.valid() && query.kind.is_some() => query,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let Some(kind) = query.kind else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    tokio::task::spawn_blocking(move || {
        match SessionApplication::new(state).reset_document_pin(
            &id,
            kind,
            query.selection_id.as_deref(),
        ) {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => failure(e),
        }
    })
    .await
    .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResetRequest {
    recovery_id: String,
}

pub(crate) async fn reset_invalid(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request: Result<Json<ResetRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Err(status) = super::session_handlers::authorize_plan_request(&headers) {
        return status.into_response();
    }
    let Ok(Json(request)) = request else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if request.recovery_id.len() != 64
        || !request
            .recovery_id
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    tokio::task::spawn_blocking(move || {
        match SessionApplication::new(state).reset_invalid_documents(&id, &request.recovery_id) {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(error) => failure(error),
        }
    })
    .await
    .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn branch_document_routes_recover_both_slots_with_desktop_authentication() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        repo.set_head("refs/heads/task").unwrap();
        for (path, text) in [
            ("specs/design.md", "# Design"),
            ("docs/superpowers/plans/plan.md", "# Implementation"),
        ] {
            std::fs::create_dir_all(dir.path().join(path).parent().unwrap()).unwrap();
            std::fs::write(dir.path().join(path), text).unwrap();
        }
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let mut meta = crate::test_support::test_session_metadata(
            "document-subject",
            "Documents",
            dir.path().to_str().unwrap(),
            "ended",
            "2026-11-01T00:00:00Z",
            "2026-11-01T00:00:00Z",
        );
        meta.cwd = dir.path().to_string_lossy().into_owned();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        let _token = crate::test_support::open_plan_token::set("document-reader-token").await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let router = crate::build_router(state.clone());
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = reqwest::Client::new();
        let url = format!("http://{addr}/sessions/document-subject/plan-content");
        assert_eq!(
            client
                .get(format!("{url}?kind=design"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNAUTHORIZED
        );
        for (kind, text) in [
            ("design", "# Design"),
            ("implementation", "# Implementation"),
        ] {
            let response = client
                .get(format!("{url}?kind={kind}"))
                .header("x-orkworks-open-plan-token", "document-reader-token")
                .send()
                .await
                .unwrap();
            assert!(
                response.status().is_success(),
                "{kind}: {}",
                response.status()
            );
            let json: serde_json::Value = response.json().await.unwrap();
            assert_eq!(json["content"], text);
            assert!(json.get("path").is_none());
        }
        assert_eq!(
            client
                .get(format!("{url}?kind=../../secret"))
                .header("x-orkworks-open-plan-token", "document-reader-token")
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::BAD_REQUEST
        );
        assert_eq!(
            client
                .get(format!("{url}?kind=design&selectionId={}", "0".repeat(64)))
                .header("x-orkworks-open-plan-token", "document-reader-token")
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::CONFLICT
        );
        {
            let workspace = state.workspace.lock().unwrap();
            let store = &workspace.as_ref().unwrap().metadata;
            store
                .mutate_session(
                    "document-subject",
                    crate::metadata::session_transactions::WriteScope::NONE,
                    |meta| {
                        meta.plan_path = Some("specs/design.md".into());
                        meta.worktree_binding = Some(crate::session_location::WorktreeBinding {
                            root: dir.path().join("missing").to_string_lossy().into_owned(),
                            common_dir: repo.commondir().to_string_lossy().into_owned(),
                        });
                        Ok(())
                    },
                )
                .unwrap();
        }
        assert_eq!(
            client
                .get(&url)
                .header("x-orkworks-open-plan-token", "document-reader-token")
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::CONFLICT
        );
        server.abort();
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    #[tokio::test]
    async fn invalid_record_reset_preserves_bytes_and_restores_discovery() {
        let root = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(root.path()).unwrap();
        repo.set_head("refs/heads/recovery").unwrap();
        std::fs::create_dir(root.path().join("specs")).unwrap();
        std::fs::write(root.path().join("specs/design.md"), "# Preserved document").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let mut meta = crate::test_support::test_session_metadata(
            "recovery-session",
            "Recovery",
            root.path().to_str().unwrap(),
            "ended",
            "now",
            "now",
        );
        meta.cwd = root.path().to_string_lossy().into_owned();
        let (store, storage) = {
            let mut workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_mut().unwrap();
            let storage = workspace.metadata.root_path();
            workspace.lease = Some(Arc::new(
                crate::workspace_runtime::WorkspaceLease::acquire(&storage).unwrap(),
            ));
            workspace.metadata.write_session(&meta);
            (workspace.metadata.branch_documents.clone(), storage)
        };
        let identity = crate::branch_documents::identity(root.path()).unwrap();
        store
            .report(
                &identity,
                crate::branch_documents::document(
                    root.path(),
                    "specs/design.md",
                    crate::metadata::PlanSource::UserSelected,
                    true,
                )
                .unwrap(),
            )
            .unwrap();
        let record = std::fs::read_dir(storage.join("branch-documents"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let original = b"{truncated record";
        std::fs::write(&record, original).unwrap();
        let metadata_before =
            std::fs::read(storage.join("sessions/recovery-session.json")).unwrap();
        let projection = crate::session_projection::SessionProjection::new(state.clone())
            .list()
            .unwrap();
        let row = projection
            .iter()
            .find(|s| s.id == "recovery-session")
            .unwrap();
        let json = serde_json::to_value(row).unwrap();
        let recovery_id = json["planDocuments"]["recoveryId"]
            .as_str()
            .expect("invalid record must expose a safe reset snapshot");
        let _token = crate::test_support::open_plan_token::set("recovery-desktop-token").await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let router = crate::build_router(state.clone());
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = reqwest::Client::new();
        let url = format!("http://{addr}/sessions/recovery-session/plan-document-reset");
        assert_eq!(
            client
                .post(&url)
                .json(&serde_json::json!({"recoveryId":recovery_id}))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNAUTHORIZED
        );
        for body in [
            serde_json::json!({"recoveryId":"../record"}),
            serde_json::json!({"recoveryId":recovery_id,"path":"/private"}),
            serde_json::json!({"recoveryId":recovery_id,"branch":"other"}),
        ] {
            assert_eq!(
                client
                    .post(&url)
                    .header("x-orkworks-open-plan-token", "recovery-desktop-token")
                    .json(&body)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                reqwest::StatusCode::BAD_REQUEST
            );
        }
        let response = client
            .post(&url)
            .header("x-orkworks-open-plan-token", "recovery-desktop-token")
            .json(&serde_json::json!({"recoveryId":recovery_id}))
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            reqwest::StatusCode::NO_CONTENT,
            "{}",
            response.text().await.unwrap()
        );
        assert!(store.load(&identity).unwrap().is_empty());
        let preserved: Vec<_> = std::fs::read_dir(record.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".preserved"))
            .collect();
        assert_eq!(preserved.len(), 1);
        assert_eq!(std::fs::read(preserved[0].path()).unwrap(), original);
        assert_eq!(
            std::fs::read(storage.join("sessions/recovery-session.json")).unwrap(),
            metadata_before
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("specs/design.md")).unwrap(),
            "# Preserved document"
        );
        assert_eq!(
            client
                .post(&url)
                .header("x-orkworks-open-plan-token", "recovery-desktop-token")
                .json(&serde_json::json!({"recoveryId":recovery_id}))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::CONFLICT
        );
        let projection = crate::session_projection::SessionProjection::new(state)
            .list()
            .unwrap();
        let row = projection
            .iter()
            .find(|s| s.id == "recovery-session")
            .unwrap();
        assert!(row
            .plan_documents
            .as_ref()
            .unwrap()
            .steps
            .iter()
            .any(|s| s.available));
        server.abort();
    }
}
