use crate::{
    session_application::{SessionApplication, TitleAction},
    AppState,
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TitleRequest {
    workspace_identity: String,
    action: TitleAction,
    title: Option<String>,
}

pub(crate) async fn session_title(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<TitleRequest>,
) -> Response {
    if let Err(status) = super::session_handlers::authorize_plan_request(&headers) {
        return status.into_response();
    }
    match tokio::task::spawn_blocking(move || {
        SessionApplication::new(state).session_title(
            &id,
            &request.workspace_identity,
            request.action,
            request.title,
        )
    })
    .await
    {
        Ok(Ok(title)) => Json(title).into_response(),
        Ok(Err(error)) => super::session_handlers::application_error_response(error),
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    #[tokio::test]
    async fn title_route_requires_desktop_authority_and_preserves_user_title() {
        let _token = crate::test_support::open_plan_token::set("title-test-token").await;
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let mut meta = crate::test_support::test_session_metadata(
            "title-route",
            "Fix login",
            dir.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        meta.label_source = crate::metadata::LabelSource::Peon;
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        let app = crate::build_router(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}/sessions/title-route/title",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = reqwest::Client::new();
        let body = serde_json::json!({"workspaceIdentity": dir.path().canonicalize().unwrap(), "action": "set", "title": "Repair login redirect"}).to_string();
        let request = |authorized: bool| {
            let mut req = client.post(&url).header("content-type", "application/json");
            if authorized {
                req = req.header("x-orkworks-open-plan-token", "title-test-token");
            }
            req.body(body.clone())
        };
        assert_eq!(
            request(false).send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        let response = request(true).send().await.unwrap();
        server.abort();
        assert_eq!(response.status(), StatusCode::OK);
        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session("title-route")
            .unwrap();
        assert_eq!(stored.label, "Repair login redirect");
        assert_eq!(stored.label_source, crate::metadata::LabelSource::User);
        assert_eq!(stored.last_activity, "now");
    }

    #[tokio::test]
    async fn invalid_titles_and_stale_workspace_cannot_mutate_a_title() {
        let _token = crate::test_support::open_plan_token::set("invalid-title-token").await;
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let meta = crate::test_support::test_session_metadata(
            "title-validation",
            "Original title",
            dir.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        let app = crate::session_application::SessionApplication::new(state.clone());
        let workspace = dir.path().canonicalize().unwrap().display().to_string();
        for title in [
            "".into(),
            " \t ".into(),
            "x".repeat(101),
            "bad\nname".into(),
            "bad\u{7f}name".into(),
        ] {
            assert!(matches!(
                app.session_title(
                    "title-validation",
                    &workspace,
                    crate::session_application::TitleAction::Set,
                    Some(title)
                ),
                Err(crate::session_application::SessionError::BadRequest(_))
            ));
        }
        assert!(matches!(
            app.session_title(
                "title-validation",
                "/different-workspace",
                crate::session_application::TitleAction::Set,
                Some("Wrong workspace".into())
            ),
            Err(crate::session_application::SessionError::Conflict)
        ));
        let selected = app
            .session_title(
                "title-validation",
                &workspace,
                crate::session_application::TitleAction::Read,
                None,
            )
            .unwrap();
        assert_eq!(selected.title, "Original title");
        let selected = app
            .session_title(
                "title-validation",
                &workspace,
                crate::session_application::TitleAction::Set,
                Some("🛠".repeat(100)),
            )
            .unwrap();
        assert_eq!(selected.title.chars().count(), 100);
    }
}
