//! Session-scoped authenticated Git inspection; no caller-supplied filesystem paths.
use crate::{
    git_changes::{self, Comparison},
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CaptureQuery {
    comparison: Comparison,
    base_ref: Option<String>,
}
fn failure(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(super::ErrorResponse {
            error: message.into(),
        }),
    )
        .into_response()
}
fn subject(state: &AppState, id: &str) -> Result<PathBuf, String> {
    if id.is_empty()
        || id.len() > 256
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
    {
        return Err("Invalid session ID.".into());
    }
    let info = {
        let ws = state.workspace.lock().unwrap();
        let ws = ws.as_ref().ok_or("Workspace unavailable.")?;
        let live = state.sessions.lock().unwrap();
        live.get(id)
            .map(|h| h.info.clone())
            .or_else(|| {
                ws.metadata
                    .read_session(id)
                    .map(|m| crate::session_types::SessionInfo::baseline_from_metadata(&m))
            })
            .ok_or("Session unavailable.")?
    };
    let cwds = crate::session_view::resolve_effective_cwds(
        &[info],
        &state.peon.reported_cwd.read().unwrap(),
        &state.session_pids.lock().unwrap(),
        crate::procfs::live_cwds,
    );
    let cwd = cwds.get(id).ok_or("Session directory unavailable.")?;
    git_changes_root(std::path::Path::new(cwd))
}
fn git_changes_root(cwd: &std::path::Path) -> Result<PathBuf, String> {
    crate::git::worktree_root(cwd)
        .ok_or("No available Git worktree for this session.")?
        .canonicalize()
        .map_err(|_| "Worktree unavailable.".into())
}
enum Operation {
    Bases,
    Capture(CaptureQuery),
    File(String, String),
}
async fn request(
    state: Arc<AppState>,
    id: String,
    headers: HeaderMap,
    operation: Operation,
) -> Response {
    if let Err(status) = super::session_handlers::authorize_plan_request(&headers) {
        return failure(status, "Desktop authorization required.");
    }
    let (store, busy) = {
        let cache = state.git_context_cache.lock().unwrap();
        (cache.changes.clone(), cache.changes_busy.clone())
    };
    let capturing = matches!(operation, Operation::Capture(_));
    if capturing
        && busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
    {
        return failure(
            StatusCode::TOO_MANY_REQUESTS,
            "Inspection busy; retry shortly.",
        );
    }
    struct Admission(Arc<std::sync::atomic::AtomicBool>);
    impl Drop for Admission {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }
    let permit = capturing.then(|| Admission(busy));
    tokio::task::spawn_blocking(move||{
        let _permit=permit;
        let root=match subject(&state,&id){Ok(r)=>r,Err(e)=>return failure(StatusCode::NOT_FOUND,e)};
        let result:Result<serde_json::Value,String>=(||{
            match operation{
                Operation::Bases=>serde_json::to_value(git_changes::bases(&root)?).map_err(|e|e.to_string()),
                Operation::Capture(q)=>{
                    let mut capture=git_changes::capture(&root,q.comparison,q.base_ref.as_deref())?;capture.session_id=id.clone();
                    let value=serde_json::to_value(&capture).map_err(|e|e.to_string())?;
                    store.lock().unwrap().insert(capture);Ok(value)
                }
                Operation::File(snapshot,file)=>{
                    let store=store.lock().unwrap();let c=store.find(&id,&root,&snapshot)?;
                    let f=c.files.iter().find(|f|f.id==file).ok_or("File unavailable; refresh.")?;
                    Ok(serde_json::json!({"snapshotId":snapshot,"fileId":file,"patch":c.patches.get(&file).cloned().unwrap_or_default(),"notice":f.notice}))
                }
            }
        })();
        // Workspace adoption replaces the store Arc: even A→B→A invalidates prior requests.
        if !Arc::ptr_eq(&store,&state.git_context_cache.lock().unwrap().changes)||subject(&state,&id).ok().as_ref()!=Some(&root){return failure(StatusCode::CONFLICT,"Inspection source changed; refresh.")}
        match result{Ok(value)=>Json(value).into_response(),Err(e)=>failure(StatusCode::UNPROCESSABLE_ENTITY,e)}
    }).await.unwrap_or_else(|_|failure(StatusCode::INTERNAL_SERVER_ERROR,"Inspection failed; retry."))
}
pub(crate) async fn get_bases(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    request(state, id, headers, Operation::Bases).await
}
pub(crate) async fn get_changes(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(q): Query<CaptureQuery>,
) -> Response {
    request(state, id, headers, Operation::Capture(q)).await
}
pub(crate) async fn get_file(
    State(state): State<Arc<AppState>>,
    Path((id, snapshot, file)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    request(state, id, headers, Operation::File(snapshot, file)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn authenticated_router_binds_captures_to_current_registered_session_and_generation() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("repo");
        std::fs::create_dir(&root).unwrap();
        let git = std::process::Command::new("git")
            .args(["init", "-qb", "main"])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(git.status.success());
        std::fs::write(root.join("new.txt"), "new content\n").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(tmp.path());
        let mut meta = crate::test_support::test_session_metadata(
            "git-subject",
            "Subject",
            &root.display().to_string(),
            "ended",
            "2026-10-24T00:00:00Z",
            "2026-10-24T00:00:00Z",
        );
        meta.cwd = root.display().to_string();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        let _token = crate::test_support::open_plan_token::set("git-reader-test-token").await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let router = crate::build_router(state.clone());
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        struct Server(tokio::task::JoinHandle<()>);
        impl Drop for Server {
            fn drop(&mut self) {
                self.0.abort();
            }
        }
        let _server = Server(server);
        let client = reqwest::Client::new();
        let url = format!("http://{addr}/sessions/git-subject/git-changes?comparison=uncommitted");
        assert_eq!(
            client
                .get(&url)
                .header("x-orkworks-open-plan-token", "wrong")
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNAUTHORIZED
        );
        let response = client
            .get(&url)
            .header("x-orkworks-open-plan-token", "git-reader-test-token")
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["sessionId"], "git-subject");
        assert_eq!(body["files"][0]["path"], "new.txt");
        let snapshot = body["snapshotId"].as_str().unwrap();
        let file = body["files"][0]["id"].as_str().unwrap();
        let patch_url =
            format!("http://{addr}/sessions/git-subject/git-changes/{snapshot}/files/{file}");
        let patch: serde_json::Value = client
            .get(&patch_url)
            .header("x-orkworks-open-plan-token", "git-reader-test-token")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(patch["patch"].as_str().unwrap().contains("+new content"));
        let busy = state.git_context_cache.lock().unwrap().changes_busy.clone();
        busy.store(true, Ordering::Release);
        assert_eq!(
            client
                .get(&url)
                .header("x-orkworks-open-plan-token", "git-reader-test-token")
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::TOO_MANY_REQUESTS
        );
        busy.store(false, Ordering::Release);
        state.git_context_cache.lock().unwrap().clear();
        assert_eq!(
            client
                .get(&patch_url)
                .header("x-orkworks-open-plan-token", "git-reader-test-token")
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session("git-subject")
                .unwrap()
                .cwd,
            meta.cwd
        );
    }
    #[tokio::test]
    async fn unauthenticated_requests_cannot_read_git_and_invalid_sessions_cannot_select_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(tmp.path());
        let response = get_bases(
            State(state.clone()),
            Path("unknown".into()),
            HeaderMap::new(),
        )
        .await;
        assert!([StatusCode::UNAUTHORIZED, StatusCode::SERVICE_UNAVAILABLE]
            .contains(&response.status()));
        assert!(subject(&state, "../../outside").is_err());
        assert!(subject(&state, "unknown").is_err());
    }
}
