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
#[serde(deny_unknown_fields)]
pub(crate) struct DocumentQuery {
    pub kind: Option<DocumentKind>,
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
    tokio::task::spawn_blocking(move || {
        match SessionApplication::new(state).read_document_content(&id, query.kind) {
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
    match SessionApplication::new(state)
        .request_document_review(&id, query.kind)
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
    let kind = match query {
        Ok(Query(DocumentQuery { kind: Some(kind) })) => kind,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    tokio::task::spawn_blocking(move || {
        match SessionApplication::new(state).reset_document_pin(&id, kind) {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => failure(e),
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
        server.abort();
    }
}
