use crate::harness::definition::{HarnessDefinition, HarnessDiagnostic, HarnessPatch};
use crate::harness::store::HarnessStoreError;
use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub(crate) enum UpdateHarnessRequest {
    BuiltinPatch { patch: HarnessPatch },
    CustomReplace { definition: HarnessDefinition },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessesResponse {
    harnesses: Vec<HarnessDefinition>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessErrorResponse {
    error: String,
    diagnostics: Vec<HarnessDiagnostic>,
}

pub(crate) async fn list_harnesses(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let registry = state
        .harness_catalog
        .read()
        .expect("harness catalog lock poisoned")
        .clone();
    Json(HarnessesResponse {
        harnesses: registry
            .ids()
            .filter_map(|id| registry.get(id).map(|harness| harness.definition.clone()))
            .collect(),
    })
}

pub(crate) async fn create_harness(
    State(state): State<Arc<AppState>>,
    Json(definition): Json<HarnessDefinition>,
) -> impl IntoResponse {
    let id = definition.id.clone();
    let store = state.harness_store.clone();
    let catalog = state.harness_catalog.clone();
    let result = tokio::task::spawn_blocking(move || {
        store.mutate(&catalog, |document| {
            if document
                .custom
                .iter()
                .any(|custom| custom.id == definition.id)
                || document.overrides.contains_key(&definition.id)
            {
                return Err(HarnessDiagnostic::for_id(
                    &definition.id,
                    "custom_id_collision",
                    "Harness ID already exists.",
                ));
            }
            document.custom.push(definition);
            Ok(())
        })
    })
    .await;
    match result {
        Ok(Ok(registry)) => registry
            .get(&id)
            .map(|harness| (StatusCode::CREATED, Json(harness.definition.clone())).into_response())
            .unwrap_or_else(|| internal_error("created harness was not resolved")),
        Ok(Err(error)) => store_error(error),
        Err(_) => internal_error("harness update task failed"),
    }
}

pub(crate) async fn update_harness(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<UpdateHarnessRequest>,
) -> impl IntoResponse {
    let store = state.harness_store.clone();
    let catalog = state.harness_catalog.clone();
    let requested_id = id.clone();
    let result = tokio::task::spawn_blocking(move || {
        store.mutate(&catalog, |document| match request {
            UpdateHarnessRequest::BuiltinPatch { patch } => {
                if document
                    .custom
                    .iter()
                    .any(|custom| custom.id == requested_id)
                {
                    return Err(HarnessDiagnostic::for_id(
                        &requested_id,
                        "custom_requires_replacement",
                        "Custom harnesses require a complete replacement.",
                    ));
                }
                document.overrides.insert(requested_id.clone(), patch);
                Ok(())
            }
            UpdateHarnessRequest::CustomReplace { mut definition } => {
                if definition.id != requested_id {
                    return Err(HarnessDiagnostic::for_id(
                        &requested_id,
                        "id_mismatch",
                        "Replacement definition ID must match the URL.",
                    ));
                }
                let Some(position) = document
                    .custom
                    .iter()
                    .position(|custom| custom.id == requested_id)
                else {
                    return Err(HarnessDiagnostic::for_id(
                        &requested_id,
                        "custom_not_found",
                        "Custom harness was not found.",
                    ));
                };
                definition.id = requested_id.clone();
                document.custom[position] = definition;
                Ok(())
            }
        })
    })
    .await;
    match result {
        Ok(Ok(registry)) => registry
            .get(&id)
            .map(|harness| Json(harness.definition.clone()).into_response())
            .unwrap_or_else(|| internal_error("updated harness was not resolved")),
        Ok(Err(error)) => store_error(error),
        Err(_) => internal_error("harness update task failed"),
    }
}

pub(crate) async fn delete_harness(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let exists = state
        .harness_catalog
        .read()
        .expect("harness catalog lock poisoned")
        .get(&id)
        .is_some();
    if !exists {
        return (
            StatusCode::NOT_FOUND,
            Json(HarnessErrorResponse {
                error: "Harness was not found.".into(),
                diagnostics: vec![HarnessDiagnostic::for_id(
                    &id,
                    "harness_not_found",
                    "Harness was not found.",
                )],
            }),
        )
            .into_response();
    }
    let store = state.harness_store.clone();
    let catalog = state.harness_catalog.clone();
    let requested_id = id.clone();
    let result = tokio::task::spawn_blocking(move || {
        store.mutate(&catalog, |document| {
            if let Some(position) = document
                .custom
                .iter()
                .position(|custom| custom.id == requested_id)
            {
                document.custom.remove(position);
                return Ok(());
            }
            if document.overrides.remove(&requested_id).is_some() {
                return Ok(());
            }
            Err(HarnessDiagnostic::for_id(
                &requested_id,
                "builtin_delete_forbidden",
                "Built-in harnesses cannot be deleted; delete an override instead.",
            ))
        })
    })
    .await;
    match result {
        Ok(Ok(_)) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(error)) => store_error(error),
        Err(_) => internal_error("harness update task failed"),
    }
}

fn store_error(error: HarnessStoreError) -> axum::response::Response {
    let (status, diagnostics) = match error {
        HarnessStoreError::Validation(diagnostics) => (StatusCode::BAD_REQUEST, diagnostics),
        HarnessStoreError::Mutation(diagnostic) => (StatusCode::CONFLICT, vec![diagnostic]),
        HarnessStoreError::RevisionChanged => (
            StatusCode::CONFLICT,
            vec![HarnessDiagnostic {
                harness_id: None,
                code: "revision_changed".into(),
                message: "Harness configuration changed; retry the request.".into(),
            }],
        ),
        HarnessStoreError::Io(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            vec![HarnessDiagnostic {
                harness_id: None,
                code: "write_failed".into(),
                message: error.to_string(),
            }],
        ),
        HarnessStoreError::Parse(error) => (
            StatusCode::BAD_REQUEST,
            vec![HarnessDiagnostic {
                harness_id: None,
                code: "invalid_document".into(),
                message: error.to_string(),
            }],
        ),
    };
    (
        status,
        Json(HarnessErrorResponse {
            error: diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone())
                .unwrap_or_else(|| "harness update failed".into()),
            diagnostics,
        }),
    )
        .into_response()
}

fn internal_error(message: &str) -> axum::response::Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(HarnessErrorResponse {
            error: message.into(),
            diagnostics: Vec::new(),
        }),
    )
        .into_response()
}
