use crate::{git, metadata, migration, watcher, AppState, WorkspaceState};
use crate::workspace_runtime::{iso_now, orkworks_global_dir};
use crate::http::session_handlers::{
    AttentionReportRequest, CreateSessionRequest, TerminalPlanSelectionRequest,
};
use axum::response::{IntoResponse, Response};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SessionError {
    BadRequest(&'static str),
    Conflict,
    NotFound,
    Internal(&'static str),
}

pub(crate) struct WorkspaceSnapshot {
    pub(crate) path: String,
    pub(crate) repo_root: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) dirty: Option<bool>,
    pub(crate) last_active_session_id: Option<String>,
    pub(crate) active_harness_ids: Vec<String>,
}

pub(crate) struct SessionApplication {
    state: Arc<AppState>,
}

pub(crate) type SessionSnapshot = Response;

pub(crate) struct CreateSessionCommand {
    pub(crate) harness_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) initial_prompt: Option<String>,
}

pub(crate) struct AttentionSignal {
    pub(crate) status: String,
    pub(crate) message: Option<String>,
    pub(crate) plan_path: metadata::PlanPathUpdate,
    pub(crate) observed_at: Option<String>,
    pub(crate) cwd: Option<String>,
}

pub(crate) struct PlanSelection {
    pub(crate) printed_path: String,
    pub(crate) token: String,
}

impl SessionApplication {
    pub(crate) fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub(crate) fn open_workspace(
        &self,
        path: PathBuf,
    ) -> Result<WorkspaceSnapshot, SessionError> {
        if !path.is_dir() {
            return Err(SessionError::BadRequest("not a directory"));
        }
        let global_dir = orkworks_global_dir(&path)
            .ok_or(SessionError::Internal("no home directory"))?;
        for dir in &["sessions", "events", "capacity", "skills"] {
            if let Err(error) = std::fs::create_dir_all(global_dir.join(dir)) {
                tracing::warn!(path = %global_dir.display(), dir, %error, "failed to create metadata dir");
            }
        }

        let store = metadata::MetadataStore::new(&global_dir);
        migration::migrate_if_needed(&path, &global_dir);
        let memory = store.read_workspace_memory();
        let last_active_session_id = memory
            .as_ref()
            .and_then(|memory| memory.last_active_session_id.clone());
        let active_harness_ids = memory.map(|memory| memory.active_harness_ids).unwrap_or_default();
        let watcher = watcher::MetadataWatcher::start(&global_dir.join("sessions"));

        let mut workspace = self.state.workspace.lock().unwrap();
        *workspace = Some(WorkspaceState { path: path.clone(), metadata: store, watcher });
        self.state.bump_harness_probe_generation();

        if let Some(workspace) = workspace.as_ref() {
            let now = iso_now();
            let live_ids: std::collections::HashSet<String> = self
                .state
                .sessions
                .lock()
                .unwrap()
                .keys()
                .cloned()
                .collect();
            for session in workspace.metadata.read_all_sessions() {
                if (session.status == "running" || session.status == "creating")
                    && !live_ids.contains(&session.id)
                {
                    workspace
                        .metadata
                        .write_session(&metadata::reconcile_orphaned_session(session, &now));
                }
            }
        }

        let git_context = git::detect(&path);
        Ok(WorkspaceSnapshot {
            path: path.display().to_string(),
            repo_root: git_context.repo_root,
            branch: git_context.branch,
            dirty: Some(git_context.dirty),
            last_active_session_id,
            active_harness_ids,
        })
    }

    pub(crate) async fn create_session(
        &self,
        request: CreateSessionCommand,
    ) -> Result<SessionSnapshot, SessionError> {
        Ok(crate::http::session_handlers::create_session_legacy(
            axum::extract::State(self.state.clone()),
            axum::Json(CreateSessionRequest {
                harness_id: request.harness_id,
                model: request.model,
                initial_prompt: request.initial_prompt,
            }),
        )
        .await
        .into_response())
    }

    pub(crate) async fn resume_session(&self, id: &str) -> Result<SessionSnapshot, SessionError> {
        Ok(crate::http::session_handlers::resume_session_legacy(
            axum::extract::State(self.state.clone()),
            axum::extract::Path(id.to_string()),
        )
        .await
        .into_response())
    }

    pub(crate) async fn report_attention(
        &self,
        id: &str,
        signal: AttentionSignal,
    ) -> Result<(), SessionError> {
        let response = crate::http::session_handlers::report_attention_legacy(
            axum::extract::State(self.state.clone()),
            axum::extract::Path(id.to_string()),
            axum::Json(AttentionReportRequest {
                status: signal.status,
                message: signal.message,
                plan_path: signal.plan_path,
                observed_at: signal.observed_at,
                cwd: signal.cwd,
            }),
        )
        .await
        .into_response();
        if response.status().is_success() {
            Ok(())
        } else {
            Err(error_for_status(response.status()))
        }
    }

    pub(crate) async fn select_plan(
        &self,
        id: &str,
        selection: PlanSelection,
    ) -> Result<SessionSnapshot, SessionError> {
        let PlanSelection { printed_path, token } = selection;
        Ok(crate::http::session_handlers::select_terminal_plan_legacy(
            axum::extract::State(self.state.clone()),
            axum::extract::Path(id.to_string()),
            {
                let mut headers = axum::http::HeaderMap::new();
                if let Ok(value) = axum::http::HeaderValue::from_str(&token) {
                    headers.insert("x-orkworks-open-plan-token", value);
                }
                headers
            },
            axum::Json(TerminalPlanSelectionRequest {
                printed_path,
            }),
        )
        .await
        .into_response())
    }

    pub(crate) async fn delete_session(
        &self,
        id: &str,
        forget: bool,
    ) -> Result<SessionSnapshot, SessionError> {
        let response = if forget {
            crate::http::session_handlers::forget_session_legacy(
                axum::extract::State(self.state.clone()),
                axum::extract::Path(id.to_string()),
            )
            .await
            .into_response()
        } else {
            crate::http::session_handlers::delete_session_legacy(
                axum::extract::State(self.state.clone()),
                axum::extract::Path(id.to_string()),
            )
            .await
            .into_response()
        };
        Ok(response)
    }
}

fn error_for_status(status: axum::http::StatusCode) -> SessionError {
    match status {
        axum::http::StatusCode::BAD_REQUEST => SessionError::BadRequest("invalid request"),
        axum::http::StatusCode::NOT_FOUND => SessionError::NotFound,
        axum::http::StatusCode::CONFLICT => SessionError::Conflict,
        _ => SessionError::Internal("application operation failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_a_workspace_returns_its_application_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state);

        let snapshot = application.open_workspace(root.path().to_path_buf()).unwrap();

        assert_eq!(snapshot.path, root.path().to_string_lossy());
    }
}
