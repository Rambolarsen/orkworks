use crate::harness::definition::parse_strict_json;
use crate::harness::integration::{
    binding_for_key, IntegrationContext, IntegrationError, IntegrationKey, IntegrationOwnership,
    IntegrationRegistration, ReporterAssetResolver,
};
use crate::harness::registry::ResolvedHarness;
use crate::harness::store::HarnessDocumentRevision;
use crate::http::ErrorResponse;
use crate::AppState;
use axum::body::Bytes;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GroupedIntegrationStatus {
    pub key: IntegrationKey,
    pub consumers: Vec<crate::harness::integration::IntegrationConsumer>,
    pub status: crate::harness::integration::IntegrationStatus,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationCleanupResponse {
    pub status: &'static str,
    pub outcomes: Vec<GroupedIntegrationStatus>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

#[derive(Clone, Debug)]
struct IntegrationRevisionExpectation {
    document_revision: Option<HarnessDocumentRevision>,
    active_harness_revision: u64,
    workspace_path: Option<PathBuf>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IntegrationRevisionConflictResponse {
    error: &'static str,
    code: &'static str,
    document_revision: Option<HarnessDocumentRevision>,
    active_harness_revision: u64,
}

fn resolve_scripts_source_dir(exe_dir: Option<PathBuf>, manifest_dir: &Path) -> PathBuf {
    if let Some(dir) = exe_dir {
        let packaged = dir.join("scripts");
        if packaged.is_dir() {
            return packaged;
        }
    }
    manifest_dir.join("scripts")
}

fn scripts_source_dir() -> PathBuf {
    resolve_scripts_source_dir(
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf)),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
}

fn stable_hook_scripts_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".orkworks").join("hook-scripts"))
}

fn reporter_assets() -> Result<ReporterAssetResolver, String> {
    let stable_dir = stable_hook_scripts_dir()
        .ok_or_else(|| "couldn't resolve home directory for the reporter scripts".to_string())?;
    Ok(ReporterAssetResolver {
        source_dir: scripts_source_dir(),
        stable_dir,
    })
}

fn workspace_harness_enabled(workspace: &crate::WorkspaceState, harness_id: &str) -> bool {
    workspace
        .metadata
        .read_workspace_memory()
        .is_some_and(|memory| memory.active_harness_ids.iter().any(|id| id == harness_id))
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
    let message = error.to_string();
    (status, Json(ErrorResponse { error: message })).into_response()
}

async fn with_revalidated_integration_target<R>(
    state: &Arc<AppState>,
    harness_id: &str,
    action: impl FnOnce(
        &ResolvedHarness,
        &crate::WorkspaceState,
        Option<&crate::harness::integration::DetectedTool>,
    ) -> R,
) -> Result<R, axum::response::Response> {
    let workspace_path_at_start = {
        let ws_guard = state.workspace.lock().unwrap();
        let Some(ws) = ws_guard.as_ref() else {
            return Err(integration_error_response(IntegrationError::NoWorkspace));
        };
        ws.path.clone()
    };
    let harness: ResolvedHarness = {
        let registry = state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned");
        let Some(harness) = registry.get(harness_id) else {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("unknown harness id \"{harness_id}\""),
                }),
            )
                .into_response());
        };
        harness.clone()
    };

    let detected_tool = crate::harness::detect::resolve_tool_gate(
        &state.integration_probe_cache,
        &harness.definition.id,
        &harness.launch_command(),
        harness.definition.min_version.as_ref(),
    )
    .await;

    {
        let registry = state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned");
        match registry.get(harness_id) {
            Some(current) if current.definition == harness.definition => {}
            _ => {
                return Err((
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: "harness definition changed during this request; retry".into(),
                    }),
                )
                    .into_response());
            }
        }
    }

    let ws_guard = state.workspace.lock().unwrap();
    let Some(ref ws) = *ws_guard else {
        return Err(integration_error_response(IntegrationError::NoWorkspace));
    };
    if ws.path != workspace_path_at_start {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "workspace changed during this request; retry".into(),
            }),
        )
            .into_response());
    }

    Ok(action(&harness, ws, detected_tool.as_ref()))
}

async fn run_integration_action(
    state: &Arc<AppState>,
    harness_id: &str,
    action: impl FnOnce(
        &ResolvedHarness,
        &IntegrationContext<'_>,
    ) -> Result<crate::harness::integration::IntegrationStatus, IntegrationError>,
) -> axum::response::Response {
    match with_revalidated_integration_target(state, harness_id, |harness, ws, detected_tool| {
        let reporter_assets = match reporter_assets() {
            Ok(resolver) => resolver,
            Err(error) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse { error }),
                )
                    .into_response();
            }
        };

        let orkworks_root = match dirs::home_dir() {
            Some(home) => home.join(".orkworks"),
            None => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "couldn't resolve home directory".into(),
                    }),
                )
                    .into_response();
            }
        };

        let ctx = IntegrationContext {
            workspace: &ws.path,
            workspace_metadata: Some(&ws.metadata),
            orkworks_root: &orkworks_root,
            enabled: workspace_harness_enabled(ws, &harness.definition.id),
            detected_tool,
            reporter_assets: &reporter_assets,
        };

        match action(harness, &ctx) {
            Ok(status) => Json(status).into_response(),
            Err(error) => integration_error_response(error),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(response) => response,
    }
}

fn active_workspace_snapshot(
    state: &Arc<AppState>,
) -> Result<(Vec<String>, u64), axum::response::Response> {
    let workspace = state.workspace.lock().unwrap();
    let Some(workspace) = workspace.as_ref() else {
        return Err(integration_error_response(IntegrationError::NoWorkspace));
    };
    let memory = workspace
        .metadata
        .read_workspace_memory()
        .unwrap_or_default();
    Ok((memory.active_harness_ids, memory.active_harness_revision))
}

fn document_revision_snapshot(
    state: &Arc<AppState>,
) -> Result<Option<HarnessDocumentRevision>, axum::response::Response> {
    state
        .harness_store
        .snapshot()
        .map(|snapshot| snapshot.document_revision)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "couldn't load harness configuration".into(),
                }),
            )
                .into_response()
        })
}

fn integration_revision_conflict(state: &Arc<AppState>) -> axum::response::Response {
    let document_revision = document_revision_snapshot(state).unwrap_or(None);
    let active_harness_revision = active_workspace_snapshot(state)
        .map(|(_, revision)| revision)
        .unwrap_or_default();
    (
        StatusCode::CONFLICT,
        Json(IntegrationRevisionConflictResponse {
            error: "Integration state changed; reload before retrying.",
            code: "integration_revision_changed",
            document_revision,
            active_harness_revision,
        }),
    )
        .into_response()
}

fn grouped_integration_error_status(
    group: &crate::harness::registry::ResolvedIntegrationGroup,
    error: &IntegrationError,
    action: &'static str,
) -> crate::harness::integration::IntegrationStatus {
    let enabled = !group.consumers.is_empty();
    crate::harness::integration::IntegrationStatus {
        harness_id: group.representative.definition.id.clone(),
        enabled,
        tool_detected: false,
        registration: crate::harness::integration::IntegrationRegistration::Error,
        ownership: crate::harness::integration::IntegrationOwnership::None,
        activation: if enabled {
            crate::harness::integration::IntegrationActivation::Unknown
        } else {
            crate::harness::integration::IntegrationActivation::Disabled
        },
        coverage: crate::harness::integration::IntegrationCoverage::None,
        diagnostics: vec![crate::harness::integration::IntegrationDiagnostic {
            code: error.code().into(),
            message: error.to_string(),
            action: Some(action.into()),
        }],
        confirmation: None,
    }
}

async fn with_revalidated_integration_key(
    state: &Arc<AppState>,
    key: &IntegrationKey,
    expected: Option<&IntegrationRevisionExpectation>,
    action: impl FnOnce(
        &ResolvedHarness,
        &IntegrationContext<'_>,
    ) -> Result<crate::harness::integration::IntegrationStatus, IntegrationError>,
) -> Result<
    (
        crate::harness::registry::ResolvedIntegrationGroup,
        Result<crate::harness::integration::IntegrationStatus, IntegrationError>,
    ),
    axum::response::Response,
> {
    let workspace_path_at_start = {
        let ws_guard = state.workspace.lock().unwrap();
        let Some(ws) = ws_guard.as_ref() else {
            return Err(integration_error_response(IntegrationError::NoWorkspace));
        };
        ws.path.clone()
    };
    if expected.is_some_and(|expected| {
        expected
            .workspace_path
            .as_deref()
            .is_some_and(|path| path != workspace_path_at_start.as_path())
    }) {
        return Err(integration_revision_conflict(state));
    }
    if binding_for_key(key).is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!(
                    "unknown integration key {}/{}",
                    key.adapter_id, key.target_id
                ),
            }),
        )
            .into_response());
    }

    let initial_document_revision = document_revision_snapshot(state)?;
    let (initial_active_ids, initial_active_harness_revision) = active_workspace_snapshot(state)?;
    if expected.is_some_and(|expected| {
        expected.document_revision != initial_document_revision
            || expected.active_harness_revision != initial_active_harness_revision
    }) {
        return Err(integration_revision_conflict(state));
    }

    let group = {
        let registry = state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned");
        registry
            .integration_group_for_key(key, &initial_active_ids)
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: format!(
                            "unknown integration key {}/{}",
                            key.adapter_id, key.target_id
                        ),
                    }),
                )
                    .into_response()
            })?
    };
    let harness = group.representative.clone();
    let enabled = !group.consumers.is_empty();
    let detected_tool = crate::harness::detect::resolve_tool_gate(
        &state.integration_probe_cache,
        &harness.definition.id,
        &harness.launch_command(),
        harness.definition.min_version.as_ref(),
    )
    .await;

    // Active-harness writes, workspace switches, and harness document
    // mutations all take this projection lock. Acquire it before the final
    // checks and hold it through the synchronous adapter action so the
    // external file mutation is one coherent projection operation.
    let _projection = state
        .projection_lock
        .lock()
        .expect("projection lock poisoned");
    let current_document_revision = document_revision_snapshot(state)?;
    let (current_active_ids, current_active_harness_revision) = active_workspace_snapshot(state)?;
    if current_document_revision != initial_document_revision
        || current_active_harness_revision != initial_active_harness_revision
        || expected.is_some_and(|expected| {
            expected.document_revision != current_document_revision
                || expected.active_harness_revision != current_active_harness_revision
        })
    {
        return Err(integration_revision_conflict(state));
    }
    let registry = state
        .harness_catalog
        .read()
        .expect("harness catalog lock poisoned");
    match registry.integration_group_for_key(key, &current_active_ids) {
        Some(current) if current.representative.definition == harness.definition => {}
        _ => return Err(integration_revision_conflict(state)),
    }
    drop(registry);

    let ws_guard = state.workspace.lock().unwrap();
    let Some(ref ws) = *ws_guard else {
        return Err(integration_error_response(IntegrationError::NoWorkspace));
    };
    if ws.path != workspace_path_at_start
        || expected.is_some_and(|expected| {
            expected
                .workspace_path
                .as_deref()
                .is_some_and(|path| path != ws.path.as_path())
        })
    {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "workspace changed during this request; retry".into(),
            }),
        )
            .into_response());
    }
    let reporter_assets = reporter_assets().map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error }),
        )
            .into_response()
    })?;
    let orkworks_root = dirs::home_dir()
        .map(|home| home.join(".orkworks"))
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "couldn't resolve home directory".into(),
                }),
            )
                .into_response()
        })?;
    let ctx = IntegrationContext {
        workspace: &ws.path,
        workspace_metadata: Some(&ws.metadata),
        orkworks_root: &orkworks_root,
        enabled,
        detected_tool: detected_tool.as_ref(),
        reporter_assets: &reporter_assets,
    };
    Ok((group, action(&harness, &ctx)))
}

async fn run_integration_key_action(
    state: &Arc<AppState>,
    key: &IntegrationKey,
    expected: Option<IntegrationRevisionExpectation>,
    failure_action: &'static str,
    action: impl FnOnce(
        &ResolvedHarness,
        &IntegrationContext<'_>,
    ) -> Result<crate::harness::integration::IntegrationStatus, IntegrationError>,
) -> axum::response::Response {
    match with_revalidated_integration_key(state, key, expected.as_ref(), action).await {
        Ok((group, Ok(status))) => Json(GroupedIntegrationStatus {
            key: key.clone(),
            consumers: group.consumers,
            status,
        })
        .into_response(),
        Ok((group, Err(error))) => {
            let status = grouped_integration_error_status(&group, &error, failure_action);
            Json(GroupedIntegrationStatus {
                key: key.clone(),
                consumers: group.consumers,
                status,
            })
            .into_response()
        }
        Err(response) => response,
    }
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

fn invalid_integration_request(message: &str) -> axum::response::Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
        .into_response()
}

/// Reconciles adapter keys that may have lost their last active consumer.
/// The caller has already committed the harness/workspace projection; this
/// operation only removes OrkWorks-owned fragments from now-unreferenced
/// adapter targets. A failed uninstall is represented in the mutation result
/// as `cleanup-needed`, so the persisted user selection is never rolled back.
pub(crate) async fn reconcile_unreferenced_integrations(
    state: &Arc<AppState>,
    keys: BTreeSet<IntegrationKey>,
    expected_workspace_path: Option<PathBuf>,
) -> IntegrationCleanupResponse {
    let expected = match (
        document_revision_snapshot(state),
        active_workspace_snapshot(state),
    ) {
        (Ok(document_revision), Ok((_, active_harness_revision))) => {
            Some(IntegrationRevisionExpectation {
                document_revision,
                active_harness_revision,
                workspace_path: expected_workspace_path,
            })
        }
        _ => None,
    };
    let mut outcomes = Vec::new();
    let mut errors = Vec::new();
    for key in keys {
        match with_revalidated_integration_key(state, &key, expected.as_ref(), |harness, ctx| {
            if ctx.enabled {
                harness.integration_status(ctx)
            } else {
                let status = harness.integration_status(ctx)?;
                if status.registration == IntegrationRegistration::Absent {
                    Ok(status)
                } else if status.registration == IntegrationRegistration::Error
                    || status.ownership == IntegrationOwnership::Ambiguous
                {
                    Ok(cleanup_needed_status(status))
                } else {
                    harness.integration_uninstall(ctx)
                }
            }
        })
        .await
        {
            Ok((group, Ok(status))) => outcomes.push(GroupedIntegrationStatus {
                key,
                consumers: group.consumers,
                status,
            }),
            Ok((group, Err(error))) => {
                let action = if group.consumers.is_empty() {
                    "cleanup-needed"
                } else {
                    "retry"
                };
                let status = grouped_integration_error_status(&group, &error, action);
                outcomes.push(GroupedIntegrationStatus {
                    key,
                    consumers: group.consumers,
                    status,
                });
            }
            Err(_) => errors.push(format!(
                "Could not reconcile integration {}/{}; retry cleanup.",
                key.adapter_id, key.target_id
            )),
        }
    }
    let status = if !errors.is_empty()
        || outcomes.iter().any(|outcome| {
            outcome
                .status
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.action.as_deref() == Some("cleanup-needed"))
        }) {
        "cleanup-needed"
    } else {
        "complete"
    };
    IntegrationCleanupResponse {
        status,
        outcomes,
        errors,
    }
}

fn cleanup_needed_status(
    mut status: crate::harness::integration::IntegrationStatus,
) -> crate::harness::integration::IntegrationStatus {
    if status.diagnostics.is_empty() {
        status
            .diagnostics
            .push(crate::harness::integration::IntegrationDiagnostic {
                code: "cleanup_needed".into(),
                message: "This integration needs manual cleanup before it can be reconciled."
                    .into(),
                action: Some("cleanup-needed".into()),
            });
    } else {
        for diagnostic in &mut status.diagnostics {
            diagnostic.action = Some("cleanup-needed".into());
        }
    }
    status
}

pub(crate) async fn get_workspace_integrations(
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    let active_ids = {
        let workspace = state.workspace.lock().unwrap();
        let Some(workspace) = workspace.as_ref() else {
            return integration_error_response(IntegrationError::NoWorkspace);
        };
        workspace
            .metadata
            .read_workspace_memory()
            .map(|memory| memory.active_harness_ids)
            .unwrap_or_default()
    };
    let groups = {
        let registry = state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned");
        match registry.integration_groups(&active_ids) {
            Ok(groups) => groups,
            Err(error) => return integration_error_response(error),
        }
    };
    let mut result = Vec::with_capacity(groups.len());
    for group in groups {
        let key = group.key.clone();
        let (group, action_result) =
            match with_revalidated_integration_key(&state, &key, None, |harness, ctx| {
                harness.integration_status(ctx)
            })
            .await
            {
                Ok(result) => result,
                Err(response) => return response,
            };
        let status = action_result
            .unwrap_or_else(|error| grouped_integration_error_status(&group, &error, "retry"));
        result.push(GroupedIntegrationStatus {
            key,
            consumers: group.consumers,
            status,
        });
    }
    Json(result).into_response()
}

pub(crate) async fn get_grouped_integration_status(
    State(state): State<Arc<AppState>>,
    AxumPath((adapter_id, target_id)): AxumPath<(String, String)>,
) -> axum::response::Response {
    let key = IntegrationKey {
        adapter_id,
        target_id,
    };
    run_integration_key_action(&state, &key, None, "retry", |harness, ctx| {
        harness.integration_status(ctx)
    })
    .await
}

pub(crate) async fn install_grouped_integration(
    State(state): State<Arc<AppState>>,
    AxumPath((adapter_id, target_id)): AxumPath<(String, String)>,
    body: Bytes,
) -> axum::response::Response {
    let expected = match parse_integration_mutation_request(&body) {
        Ok(expected) => expected,
        Err(response) => return response,
    };
    let key = IntegrationKey {
        adapter_id,
        target_id,
    };
    run_integration_key_action(
        &state,
        &key,
        Some(expected),
        "action-needed",
        |harness, ctx| harness.integration_install(ctx),
    )
    .await
}

pub(crate) async fn repair_grouped_integration(
    State(state): State<Arc<AppState>>,
    AxumPath((adapter_id, target_id)): AxumPath<(String, String)>,
    body: Bytes,
) -> axum::response::Response {
    install_grouped_integration(State(state), AxumPath((adapter_id, target_id)), body).await
}

pub(crate) async fn uninstall_grouped_integration(
    State(state): State<Arc<AppState>>,
    AxumPath((adapter_id, target_id)): AxumPath<(String, String)>,
    body: Bytes,
) -> axum::response::Response {
    let expected = match parse_integration_mutation_request(&body) {
        Ok(expected) => expected,
        Err(response) => return response,
    };
    let key = IntegrationKey {
        adapter_id,
        target_id,
    };
    run_integration_key_action(
        &state,
        &key,
        Some(expected),
        "cleanup-needed",
        |harness, ctx| harness.integration_uninstall(ctx),
    )
    .await
}

pub(crate) async fn get_integration_status(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| {
        harness.integration_status(ctx)
    })
    .await
}

pub(crate) async fn install_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| {
        harness.integration_install(ctx)
    })
    .await
}

pub(crate) async fn uninstall_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| {
        harness.integration_uninstall(ctx)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_application::SessionApplication;
    use crate::test_support::{test_app_state_with_workspace, FakeHome};
    use serde_json::Value;

    // Claude's handler is a JsonHookHandler: `require_local_or_ignored_untracked`
    // (harness/integration.rs) refuses to read or write its config file unless
    // the workspace is a Git repository with that file gitignored. Every test
    // below that reaches Claude's handler must set this up first, or it hits
    // `UnsafeTarget { code: "not_git_workspace" }` (mapped to 400) instead of
    // the behavior the test name describes — `status()` swallows that into an
    // `"error"` registration, and `install`/`uninstall` return it as an error.
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

    fn init_git_workspace_with_codex_hooks_ignored(workspace: &std::path::Path) {
        git2::Repository::init(workspace).unwrap();
        std::fs::write(workspace.join(".gitignore"), ".codex/hooks.json\n").unwrap();
    }

    #[test]
    fn grouped_mutations_require_the_document_and_active_revisions() {
        let missing = parse_integration_mutation_request(&Bytes::from_static(b"{}")).unwrap_err();
        assert_eq!(missing.status(), StatusCode::BAD_REQUEST);

        let expected = parse_integration_mutation_request(&Bytes::from_static(
            br#"{"expectedDocumentRevision":null,"expectedActiveHarnessRevision":7}"#,
        ))
        .unwrap();
        assert_eq!(expected.document_revision, None);
        assert_eq!(expected.active_harness_revision, 7);
    }

    // Pins the packaged-vs-dev fallback that used to be covered by
    // hook_handlers.rs's resolve_claude_hook_script_path_* tests (deleted
    // along with that file) — this is the same AppImage/packaging-sensitive
    // logic, just resolving a scripts directory instead of one script file.
    #[test]
    fn resolve_scripts_source_dir_prefers_packaged_layout_when_present() {
        let exe_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(exe_dir.path().join("scripts")).unwrap();
        let manifest_dir = tempfile::tempdir().unwrap();

        let resolved =
            resolve_scripts_source_dir(Some(exe_dir.path().to_path_buf()), manifest_dir.path());

        assert_eq!(resolved, exe_dir.path().join("scripts"));
    }

    #[test]
    fn resolve_scripts_source_dir_falls_back_to_dev_manifest_dir() {
        let exe_dir = tempfile::tempdir().unwrap();
        let manifest_dir = tempfile::tempdir().unwrap();

        let resolved =
            resolve_scripts_source_dir(Some(exe_dir.path().to_path_buf()), manifest_dir.path());

        assert_eq!(resolved, manifest_dir.path().join("scripts"));
    }

    #[tokio::test]
    async fn status_reports_absent_for_a_fresh_workspace() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["registration"], "absent");
        assert_eq!(body["enabled"], false);
    }

    #[tokio::test]
    async fn grouped_status_projects_one_shared_copilot_target_to_both_consumers() {
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
                    .find(|definition| definition.id == "copilot")
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

        let response = get_workspace_integrations(State(state.clone()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.as_array().unwrap().len(), 1);
        assert_eq!(body[0]["key"]["adapterId"], "copilot");
        assert_eq!(body[0]["key"]["targetId"], "workspace");
        assert_eq!(
            body[0]["consumers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|consumer| consumer["harnessId"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["copilot", "copilot-local"]
        );

        SessionApplication::new(state.clone())
            .set_active_harnesses(vec!["copilot-local".into()])
            .unwrap();
        let response = get_workspace_integrations(State(state.clone()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body[0]["consumers"][0]["harnessId"], "copilot-local");

        SessionApplication::new(state.clone())
            .set_active_harnesses(vec![])
            .unwrap();
        let response = get_workspace_integrations(State(state))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(body.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn grouped_copilot_mutations_return_shared_identity_and_cleanup() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
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
        let body = || {
            Bytes::from(
                serde_json::json!({
                    "expectedDocumentRevision": snapshot.document_revision,
                    "expectedActiveHarnessRevision": active_revision,
                })
                .to_string(),
            )
        };

        let install = install_grouped_integration(
            State(state.clone()),
            AxumPath(("copilot".into(), "workspace".into())),
            body(),
        )
        .await
        .into_response();
        assert_eq!(install.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(install.into_body(), usize::MAX)
            .await
            .unwrap();
        let installed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(installed["key"]["adapterId"], "copilot");
        assert_eq!(installed["key"]["targetId"], "workspace");
        assert_eq!(installed["consumers"][0]["harnessId"], "copilot");
        assert_eq!(installed["status"]["registration"], "installed");

        let uninstall = uninstall_grouped_integration(
            State(state),
            AxumPath(("copilot".into(), "workspace".into())),
            body(),
        )
        .await
        .into_response();
        assert_eq!(uninstall.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(uninstall.into_body(), usize::MAX)
            .await
            .unwrap();
        let removed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(removed["key"]["adapterId"], "copilot");
        assert_eq!(removed["status"]["registration"], "absent");
    }

    #[tokio::test]
    async fn grouped_cleanup_reports_manual_action_for_a_foreign_hook() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let settings = dir.path().join(".github/copilot/settings.local.json");
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        let foreign = serde_json::json!({
            "version": 1,
            "hooks": {
                "notification": [{
                    "type": "command",
                    "bash": "foreign-reporter",
                    "env": {"ORKWORKS_INTEGRATION_MARKER": "orkworks:harness-integration:v2:foreign"}
                }]
            }
        });
        let original = serde_json::to_vec_pretty(&foreign).unwrap();
        std::fs::write(&settings, &original).unwrap();
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());
        let response = uninstall_grouped_integration(
            State(state),
            AxumPath(("copilot".into(), "workspace".into())),
            Bytes::from(
                serde_json::json!({
                    "expectedDocumentRevision": null,
                    "expectedActiveHarnessRevision": 0,
                })
                .to_string(),
            ),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["status"]["diagnostics"][0]["action"], "cleanup-needed");
        assert_eq!(std::fs::read(&settings).unwrap(), original);
    }

    #[tokio::test]
    async fn grouped_mutation_rejects_a_stale_active_harness_revision_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let response = install_grouped_integration(
            State(test_app_state_with_workspace(dir.path())),
            AxumPath(("copilot".into(), "workspace".into())),
            Bytes::from(
                serde_json::json!({
                    "expectedDocumentRevision": null,
                    "expectedActiveHarnessRevision": 1,
                })
                .to_string(),
            ),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["code"], "integration_revision_changed");
        assert!(!dir
            .path()
            .join(".github/copilot/settings.local.json")
            .exists());
    }

    #[tokio::test]
    async fn install_then_status_reports_installed() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let install_response =
            install_integration(State(state.clone()), AxumPath("claude-code".into()))
                .await
                .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let status_response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(status_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["registration"], "installed");
    }

    #[tokio::test]
    async fn codex_install_then_status_reports_installed_via_http() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_codex_hooks_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());
        SessionApplication::new(state.clone())
            .set_active_harnesses(vec!["codex".into()])
            .unwrap();

        let install_response = install_integration(State(state.clone()), AxumPath("codex".into()))
            .await
            .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let status_response = get_integration_status(State(state), AxumPath("codex".into()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(status_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["registration"], "installed");
    }

    #[tokio::test]
    async fn codex_status_preserves_needs_trust_for_an_installed_compatible_tool() {
        use crate::test_support::{make_test_executable, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_codex_hooks_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());
        SessionApplication::new(state.clone())
            .set_active_harnesses(vec!["codex".into()])
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) { "codex.exe" } else { "codex" };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\necho 'codex-cli 0.114.0'\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let install_response = install_integration(State(state.clone()), AxumPath("codex".into()))
            .await
            .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let response = get_integration_status(State(state), AxumPath("codex".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["activation"], "needs_trust");
        assert!(body["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|diagnostic| diagnostic["code"] != "unsupported_tool_version"));
    }

    #[tokio::test]
    async fn install_then_uninstall_reports_absent() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());
        SessionApplication::new(state.clone())
            .set_active_harnesses(vec!["claude-code".into()])
            .unwrap();

        let install_response =
            install_integration(State(state.clone()), AxumPath("claude-code".into()))
                .await
                .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);
        let uninstall_response =
            uninstall_integration(State(state), AxumPath("claude-code".into()))
                .await
                .into_response();
        assert_eq!(uninstall_response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(uninstall_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["registration"], "absent");
    }

    #[tokio::test]
    async fn status_without_a_workspace_returns_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        *state.workspace.lock().unwrap() = None;

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn status_for_an_unknown_harness_id_returns_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = get_integration_status(State(state), AxumPath("not-a-real-harness".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn install_rejects_malformed_existing_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("settings.local.json"), "not json").unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = install_integration(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn detected_tool_reflects_probe_result_for_a_resolvable_command() {
        use crate::test_support::{make_test_executable, FakePath};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());
        SessionApplication::new(state.clone())
            .set_active_harnesses(vec!["claude-code".into()])
            .unwrap();

        let install_response =
            install_integration(State(state.clone()), AxumPath("claude-code".into()))
                .await
                .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let fake_bin_dir = tempfile::tempdir().unwrap();
        // On Windows, probe_installed_tool searches PATHEXT candidates
        // (claude.exe, claude.cmd, ...) for a bare "claude" — a plain
        // extensionless file wouldn't match any of them.
        let bin_name = if cfg!(windows) {
            "claude.exe"
        } else {
            "claude"
        };
        let bin = fake_bin_dir.path().join(bin_name);
        fs::write(&bin, "#!/bin/sh\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], true);
        assert_eq!(body["activation"], "active");
    }

    #[tokio::test]
    async fn min_version_gating_marks_an_installed_below_threshold_binary_as_unknown() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

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
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (99, 0, 0) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) {
            "copilot.exe"
        } else {
            "copilot"
        };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\necho 'copilot-cli 1.0.0'\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let install_response =
            install_integration(State(state.clone()), AxumPath("copilot".into()))
                .await
                .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let response = get_integration_status(State(state), AxumPath("copilot".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], true);
        assert_eq!(body["activation"], "unknown");
        assert!(body["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "unsupported_tool_version"));
    }

    #[tokio::test]
    async fn min_version_gating_leaves_an_above_threshold_binary_fully_active() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

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
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) {
            "copilot.exe"
        } else {
            "copilot"
        };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\necho 'copilot-cli 1.0.0'\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        // Active (as opposed to Absent/Disabled) only applies once the hook
        // is actually Installed — status_from_document's activation match
        // only reaches self.contract.activation via the `registration ==
        // Installed` arm, so an install must happen first, exactly like the
        // pre-existing Claude reference test this one is modeled on
        // (detected_tool_reflects_probe_result_for_a_resolvable_command).
        // Copilot (like Claude, unlike Gemini) declares
        // activation: IntegrationActivation::Active on its contract
        // (harness/integrations/copilot.rs) — Gemini's own contract
        // declares Unknown even when fully installed and detected (its
        // coverage is Limited by design), which is unrelated to min_version
        // and would make this assertion fail no matter what this task
        // wires up.
        let install_response =
            install_integration(State(state.clone()), AxumPath("copilot".into()))
                .await
                .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let response = get_integration_status(State(state), AxumPath("copilot".into()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], true);
        assert_eq!(body["activation"], "active");
    }

    #[tokio::test]
    async fn repeated_status_polls_reuse_one_version_probe_within_ttl() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakeHome, FakePath};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let counter = fake_bin_dir.path().join("probe-count.txt");
        let bin_name = if cfg!(windows) {
            "copilot.exe"
        } else {
            "copilot"
        };
        let bin = fake_bin_dir.path().join(bin_name);
        fs::write(
            &bin,
            format!(
                "#!/bin/sh\nprintf '1.2.3\\n'\nprintf 'probe\\n' >> '{}'\n",
                counter.display()
            ),
        )
        .unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let first = get_integration_status(State(state.clone()), AxumPath("copilot".into()))
            .await
            .into_response();
        let second = get_integration_status(State(state), AxumPath("copilot".into()))
            .await
            .into_response();

        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(second.status(), StatusCode::OK);
        assert_eq!(fs::read_to_string(counter).unwrap().lines().count(), 1);
    }

    #[tokio::test]
    async fn workspace_switch_forces_a_fresh_version_probe() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, swap_workspace, FakeHome, FakePath};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let counter = fake_bin_dir.path().join("probe-count.txt");
        let bin_name = if cfg!(windows) {
            "copilot.exe"
        } else {
            "copilot"
        };
        let bin = fake_bin_dir.path().join(bin_name);
        fs::write(
            &bin,
            format!(
                "#!/bin/sh\nprintf '1.2.3\\n'\nprintf 'probe\\n' >> '{}'\n",
                counter.display()
            ),
        )
        .unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let _ = get_integration_status(State(state.clone()), AxumPath("copilot".into()))
            .await
            .into_response();

        let other_workspace = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(other_workspace.path());
        swap_workspace(&state, other_workspace.path());

        let _ = get_integration_status(State(state), AxumPath("copilot".into()))
            .await
            .into_response();

        assert_eq!(fs::read_to_string(counter).unwrap().lines().count(), 2);
    }

    #[tokio::test]
    async fn harness_edit_forces_a_fresh_version_probe() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakeHome, FakePath};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let counter = fake_bin_dir.path().join("probe-count.txt");
        let bin_name = if cfg!(windows) {
            "copilot.exe"
        } else {
            "copilot"
        };
        let bin = fake_bin_dir.path().join(bin_name);
        fs::write(
            &bin,
            format!(
                "#!/bin/sh\nprintf '1.2.3\\n'\nprintf 'probe\\n' >> '{}'\n",
                counter.display()
            ),
        )
        .unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let _ = get_integration_status(State(state.clone()), AxumPath("copilot".into()))
            .await
            .into_response();

        let expected_revision = state.harness_store.snapshot().unwrap().document_revision;

        let update_response = crate::http::harness_handlers::update_harness(
            State(state.clone()),
            AxumPath("copilot".into()),
            axum::body::Bytes::from(
                serde_json::to_vec(&serde_json::json!({
                    "kind": "BuiltinPatch",
                    "patch": {
                        "minVersion": { "min": [0, 0, 2] }
                    },
                    "expectedRevision": expected_revision
                }))
                .unwrap(),
            ),
        )
        .await
        .into_response();
        assert_eq!(update_response.status(), StatusCode::OK);

        let _ = get_integration_status(State(state), AxumPath("copilot".into()))
            .await
            .into_response();

        assert_eq!(fs::read_to_string(counter).unwrap().lines().count(), 2);
    }

    #[tokio::test]
    async fn a_slow_version_probe_does_not_block_a_concurrent_workspace_request() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) {
            "copilot.exe"
        } else {
            "copilot"
        };
        let bin = fake_bin_dir.path().join(bin_name);
        // `exec` matters here exactly as it does in detect.rs's kill-on-
        // timeout test: without it, `sh` forks `sleep` as a grandchild that
        // kill_on_drop's signal never reaches, leaking it independently of
        // (and in addition to) whatever detect.rs's own test covers — that
        // test only exercises its own spawned binary, not this one.
        std::fs::write(&bin, "#!/bin/sh\nexec sleep 30\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let slow_state = state.clone();
        let slow_request = tokio::spawn(async move {
            get_integration_status(State(slow_state), AxumPath("copilot".into()))
                .await
                .into_response()
        });
        // Give the slow request a head start into its probe before firing
        // the concurrent one.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let start = std::time::Instant::now();
        let concurrent_response =
            get_integration_status(State(state.clone()), AxumPath("claude-code".into()))
                .await
                .into_response();
        let elapsed = start.elapsed();

        assert_eq!(concurrent_response.status(), StatusCode::OK);
        // 2s of margin below the probe's 3s timeout: generous enough to
        // absorb scheduling jitter on a loaded CI runner while still being a
        // meaningful signal that the concurrent request didn't queue behind
        // the slow one. If this proves flaky in practice, widen the margin
        // rather than add retry logic — a single fixed threshold is enough
        // signal for what this test is checking.
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "a concurrent request must not wait behind the slow probe's 3s timeout, took {elapsed:?}"
        );

        // Clean up: let the slow request finish so the test doesn't leak a
        // background task past its own scope.
        let slow_response = slow_request.await.unwrap();
        assert_eq!(slow_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn with_revalidated_integration_target_rejects_workspace_switch() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, swap_workspace, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) {
            "copilot.exe"
        } else {
            "copilot"
        };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\nexec sleep 30\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let slow_state = state.clone();
        let request = tokio::spawn(async move {
            with_revalidated_integration_target(&slow_state, "copilot", |_h, _ws, _detected| {
                StatusCode::OK
            })
            .await
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let other_dir = tempfile::tempdir().unwrap();
        swap_workspace(&state, other_dir.path());

        let result = request.await.unwrap();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn with_revalidated_integration_target_rejects_harness_definition_change() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) {
            "copilot.exe"
        } else {
            "copilot"
        };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\nexec sleep 30\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let slow_state = state.clone();
        let request = tokio::spawn(async move {
            with_revalidated_integration_target(&slow_state, "copilot", |_h, _ws, _detected| {
                StatusCode::OK
            })
            .await
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let mut changed_document = crate::harness::definition::HarnessUserDocument::default();
        changed_document.overrides.insert(
            "copilot".to_string(),
            HarnessPatch {
                min_version: Some(Some(VersionRequirement { min: (99, 0, 0) })),
                ..Default::default()
            },
        );
        let builtins = crate::harness::definition::BuiltinDocument::parse(
            crate::harness::definition::EMBEDDED_BUILTINS,
        )
        .unwrap();
        let changed_registry =
            crate::harness::registry::resolve_document(&builtins, &changed_document).unwrap();
        *state
            .harness_catalog
            .write()
            .expect("harness catalog lock poisoned") = std::sync::Arc::new(changed_registry);

        let result = request.await.unwrap();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn with_revalidated_integration_target_returns_harness_and_probe_data() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let result = with_revalidated_integration_target(&state, "claude-code", |h, _ws, _tool| {
            h.definition.id.clone()
        })
        .await;
        assert_eq!(result.unwrap(), "claude-code");
    }

    #[tokio::test]
    async fn a_workspace_switch_during_the_probe_is_rejected_instead_of_targeting_the_new_one() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, swap_workspace, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) {
            "copilot.exe"
        } else {
            "copilot"
        };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\nexec sleep 30\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let slow_state = state.clone();
        let slow_request = tokio::spawn(async move {
            get_integration_status(State(slow_state), AxumPath("copilot".into()))
                .await
                .into_response()
        });
        // Give the slow request a head start into its probe, then switch
        // the active workspace to a different directory while it's still
        // in flight.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let other_dir = tempfile::tempdir().unwrap();
        swap_workspace(&state, other_dir.path());

        let response = slow_request.await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::CONFLICT,
            "a request whose workspace changed mid-flight must be rejected, not silently \
             completed against the new workspace"
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            payload["error"],
            "workspace changed during this request; retry"
        );
    }

    #[tokio::test]
    async fn a_harness_definition_change_during_the_probe_is_rejected_instead_of_using_stale_data()
    {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) {
            "copilot.exe"
        } else {
            "copilot"
        };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\nexec sleep 30\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let slow_state = state.clone();
        let slow_request = tokio::spawn(async move {
            get_integration_status(State(slow_state), AxumPath("copilot".into()))
                .await
                .into_response()
        });
        // Give the slow request a head start into its probe, then change the
        // harness definition it's probing against while it's still in
        // flight. This swaps the resolved registry directly rather than
        // going through HarnessStore::mutate a second time on the same
        // store — a second sequential mutate() call on one store hits an
        // unrelated pre-existing bug (issue #230, reproduced with a plain
        // .name patch too, nothing to do with min_version) where the store
        // fails to read back what it just wrote. Swapping the registry
        // directly still exercises exactly what this test needs: the
        // harness_catalog RwLock's content changing between this request's
        // clone and its re-validation.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let mut changed_document = crate::harness::definition::HarnessUserDocument::default();
        changed_document.overrides.insert(
            "copilot".to_string(),
            HarnessPatch {
                min_version: Some(Some(VersionRequirement { min: (99, 0, 0) })),
                ..Default::default()
            },
        );
        let builtins = crate::harness::definition::BuiltinDocument::parse(
            crate::harness::definition::EMBEDDED_BUILTINS,
        )
        .unwrap();
        let changed_registry =
            crate::harness::registry::resolve_document(&builtins, &changed_document).unwrap();
        *state
            .harness_catalog
            .write()
            .expect("harness catalog lock poisoned") = std::sync::Arc::new(changed_registry);

        let response = slow_request.await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::CONFLICT,
            "a request whose harness definition changed mid-flight must be rejected, not \
             silently completed against the stale pre-patch definition"
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            payload["error"],
            "harness definition changed during this request; retry"
        );
    }

    // This one already passes before the fix too (detected_tool was always
    // None, so activation was always forced to Unknown regardless of the
    // real command) — it's a regression guard for the genuinely-not-found
    // path, not a red/green driver. Kept alongside the test above for
    // coverage of both outcomes of the same wiring.
    //
    // It overrides claude-code's launch command to an unresolvable name
    // rather than relying on the ambient PATH lacking a real `claude`
    // binary: FakePath::prepend only prepends (by design — see Task 1), so
    // it can't hide a real `claude` install elsewhere on PATH, and this
    // test must still pass on a machine that has Claude Code installed.
    #[tokio::test]
    async fn status_reports_disabled_for_an_owned_integration_until_the_tool_is_active() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let install_response =
            install_integration(State(state.clone()), AxumPath("claude-code".into()))
                .await
                .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let disabled_response =
            get_integration_status(State(state.clone()), AxumPath("claude-code".into()))
                .await
                .into_response();
        let disabled_bytes = axum::body::to_bytes(disabled_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let disabled_body: Value = serde_json::from_slice(&disabled_bytes).unwrap();
        assert_eq!(disabled_body["registration"], "installed");
        assert_eq!(disabled_body["enabled"], false);
        assert_eq!(disabled_body["activation"], "disabled");

        SessionApplication::new(state.clone())
            .set_active_harnesses(vec!["claude-code".into()])
            .unwrap();
        let enabled_response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        let enabled_bytes = axum::body::to_bytes(enabled_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let enabled_body: Value = serde_json::from_slice(&enabled_bytes).unwrap();
        assert_eq!(enabled_body["registration"], "installed");
        assert_eq!(enabled_body["enabled"], true);
    }

    #[tokio::test]
    async fn aider_status_reports_limited_coverage_and_truthful_enablement() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());
        SessionApplication::new(state.clone())
            .set_active_harnesses(vec!["aider".into()])
            .unwrap();

        let install_response = install_integration(State(state.clone()), AxumPath("aider".into()))
            .await
            .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let enabled_response =
            get_integration_status(State(state.clone()), AxumPath("aider".into()))
                .await
                .into_response();
        let enabled_bytes = axum::body::to_bytes(enabled_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let enabled_body: Value = serde_json::from_slice(&enabled_bytes).unwrap();
        assert_eq!(enabled_body["registration"], "installed");
        assert_eq!(enabled_body["enabled"], true);
        assert_eq!(enabled_body["coverage"], "limited");

        SessionApplication::new(state.clone())
            .set_active_harnesses(vec![])
            .unwrap();
        let disabled_response = get_integration_status(State(state), AxumPath("aider".into()))
            .await
            .into_response();
        let disabled_bytes = axum::body::to_bytes(disabled_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let disabled_body: Value = serde_json::from_slice(&disabled_bytes).unwrap();
        assert_eq!(disabled_body["registration"], "installed");
        assert_eq!(disabled_body["enabled"], false);
        assert_eq!(disabled_body["activation"], "disabled");
        assert_eq!(disabled_body["coverage"], "limited");
    }

    #[tokio::test]
    async fn unsupported_generic_shell_status_reports_unsupported_without_hardcoded_enablement() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let disabled_response =
            get_integration_status(State(state.clone()), AxumPath("generic-shell".into()))
                .await
                .into_response();
        assert_eq!(disabled_response.status(), StatusCode::OK);
        let disabled_bytes = axum::body::to_bytes(disabled_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let disabled_body: Value = serde_json::from_slice(&disabled_bytes).unwrap();
        assert_eq!(disabled_body["registration"], "unsupported");
        assert_eq!(disabled_body["enabled"], false);
        assert_eq!(disabled_body["activation"], "not_applicable");

        SessionApplication::new(state.clone())
            .set_active_harnesses(vec!["generic-shell".into()])
            .unwrap();
        let enabled_response =
            get_integration_status(State(state), AxumPath("generic-shell".into()))
                .await
                .into_response();
        let enabled_bytes = axum::body::to_bytes(enabled_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let enabled_body: Value = serde_json::from_slice(&enabled_bytes).unwrap();
        assert_eq!(enabled_body["registration"], "unsupported");
        assert_eq!(enabled_body["enabled"], true);
        assert_eq!(enabled_body["activation"], "not_applicable");
    }

    #[tokio::test]
    async fn uninstall_preserves_ambiguous_foreign_codex_hooks() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_codex_hooks_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let hooks_dir = dir.path().join(".codex");
        std::fs::create_dir_all(&hooks_dir).unwrap();
        let hooks_path = hooks_dir.join("hooks.json");
        let foreign = r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"/path/to/report-harness-event.sh --marker 'orkworks:harness-integration:v2:claude-code'"}]}]}}"#;
        std::fs::write(&hooks_path, foreign).unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = uninstall_integration(State(state), AxumPath("codex".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(std::fs::read_to_string(&hooks_path).unwrap(), foreign);
    }

    #[tokio::test]
    async fn detected_tool_stays_absent_when_the_command_is_not_on_path() {
        use crate::harness::definition::{HarnessPatch, LaunchPatch};
        use crate::test_support::FakePath;

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());
        SessionApplication::new(state.clone())
            .set_active_harnesses(vec!["claude-code".into()])
            .unwrap();

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "claude-code".to_string(),
                    HarnessPatch {
                        name: None,
                        launch: Some(LaunchPatch {
                            kind: None,
                            command: Some("definitely-not-a-real-binary-xyz".to_string()),
                            args: None,
                            model_prefix: None,
                            login: None,
                        }),
                        default_model: None,
                        resume: None,
                        models: None,
                        peon: None,
                        capacity: None,
                        session_signals: None,
                        integration: None,
                        voice: None,
                        min_version: None,
                        label_reset_commands: None,
                    },
                );
                Ok(())
            })
            .unwrap();

        let empty_bin_dir = tempfile::tempdir().unwrap();
        let _fake_path = FakePath::prepend(empty_bin_dir.path());

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], false);
        assert_eq!(body["activation"], "unknown");
        assert!(body["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "tool_not_detected"));
    }
}
