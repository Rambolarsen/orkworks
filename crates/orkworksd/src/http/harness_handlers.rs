use crate::harness::definition::{
    parse_custom_definition, parse_strict_json, HarnessDefinition, HarnessDiagnostic, HarnessPatch,
};
use crate::harness::store::HarnessStoreError;
use crate::AppState;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
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
    body: Bytes,
) -> impl IntoResponse {
    let definition = match parse_custom_definition(&body) {
        Ok(definition) => definition,
        Err(diagnostics) => return store_error(HarnessStoreError::Validation(diagnostics)),
    };
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
        Ok(Ok(registry)) => {
            state.bump_harness_probe_generation();
            registry
                .get(&id)
                .map(|harness| {
                    (StatusCode::CREATED, Json(harness.definition.clone())).into_response()
                })
                .unwrap_or_else(|| internal_error("created harness was not resolved"))
        }
        Ok(Err(error)) => store_error(error),
        Err(_) => internal_error("harness update task failed"),
    }
}

pub(crate) async fn update_harness(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Bytes,
) -> impl IntoResponse {
    let request = match parse_update_request(&body) {
        Ok(request) => request,
        Err(diagnostics) => return store_error(HarnessStoreError::Validation(diagnostics)),
    };
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
        Ok(Ok(registry)) => {
            state.bump_harness_probe_generation();
            registry
                .get(&id)
                .map(|harness| Json(harness.definition.clone()).into_response())
                .unwrap_or_else(|| internal_error("updated harness was not resolved"))
        }
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
            if document.remove_custom_definition(&requested_id) {
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
        Ok(Ok(_)) => {
            state.bump_harness_probe_generation();
            StatusCode::NO_CONTENT.into_response()
        }
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
                path: None,
            }],
        ),
        HarnessStoreError::Io(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            vec![HarnessDiagnostic {
                harness_id: None,
                code: "write_failed".into(),
                message: error.to_string(),
                path: None,
            }],
        ),
        HarnessStoreError::Parse(error) => (
            StatusCode::BAD_REQUEST,
            vec![HarnessDiagnostic {
                harness_id: None,
                code: "invalid_document".into(),
                message: error.to_string(),
                path: None,
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

fn parse_update_request(bytes: &[u8]) -> Result<UpdateHarnessRequest, Vec<HarnessDiagnostic>> {
    let value = parse_strict_json::<serde_json::Value>(bytes, 256 * 1024)
        .map_err(|diagnostic| vec![diagnostic])?;
    let object = value.as_object().ok_or_else(|| {
        vec![HarnessDiagnostic::document(
            "invalid_schema",
            "Harness update request must be a JSON object.",
            Some("$"),
        )]
    })?;
    let kind = object
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            vec![HarnessDiagnostic::document(
                "missing_field",
                "Harness update request requires kind.",
                Some("$.kind"),
            )]
        })?;
    let allowed = match kind {
        "BuiltinPatch" => &["kind", "patch"][..],
        "CustomReplace" => &["kind", "definition"][..],
        _ => {
            return Err(vec![HarnessDiagnostic::document(
                "invalid_schema",
                "Harness update kind must be BuiltinPatch or CustomReplace.",
                Some("$.kind"),
            )]);
        }
    };
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(vec![HarnessDiagnostic::document(
            "unknown_field",
            &format!("Unknown harness update field {field}."),
            Some(&format!("$.{field}")),
        )]);
    }
    match kind {
        "BuiltinPatch" => {
            let patch = object.get("patch").cloned().ok_or_else(|| {
                vec![HarnessDiagnostic::document(
                    "missing_field",
                    "BuiltinPatch requires patch.",
                    Some("$.patch"),
                )]
            })?;
            serde_json::from_value::<HarnessPatch>(patch)
                .map(|patch| UpdateHarnessRequest::BuiltinPatch { patch })
                .map_err(|error| {
                    vec![HarnessDiagnostic::document(
                        "invalid_schema",
                        &error.to_string(),
                        Some("$.patch"),
                    )]
                })
        }
        "CustomReplace" => {
            let definition = object.get("definition").cloned().ok_or_else(|| {
                vec![HarnessDiagnostic::document(
                    "missing_field",
                    "CustomReplace requires definition.",
                    Some("$.definition"),
                )]
            })?;
            let serialized = serde_json::to_vec(&definition).map_err(|error| {
                vec![HarnessDiagnostic::document(
                    "invalid_schema",
                    &error.to_string(),
                    Some("$.definition"),
                )]
            })?;
            parse_custom_definition(&serialized)
                .map(|definition| UpdateHarnessRequest::CustomReplace { definition })
        }
        _ => unreachable!("request kind was validated above"),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_parser_rejects_duplicate_keys() {
        let diagnostics =
            parse_update_request(br#"{"kind":"BuiltinPatch","kind":"BuiltinPatch","patch":{}}"#)
                .expect_err("duplicate request keys must be rejected");
        assert_eq!(diagnostics[0].code, "duplicate_key");
    }

    #[test]
    fn update_parser_uses_the_restricted_custom_schema() {
        let diagnostics = parse_update_request(
            br#"{"kind":"CustomReplace","definition":{"id":"local","name":"Local","launch":{"kind":"command-template","command":"local","args":[],"integration":{"kind":"copilot"}}}}"#,
        )
        .expect_err("custom replacement must reject compiled fields");
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field"
                && diagnostic.path.as_deref() == Some("$.launch.integration")
        }));
    }

    #[test]
    fn update_parser_rejects_unknown_envelope_fields() {
        let diagnostics =
            parse_update_request(br#"{"kind":"BuiltinPatch","patch":{},"unexpected":true}"#)
                .expect_err("unknown request fields must be rejected");
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field" && diagnostic.path.as_deref() == Some("$.unexpected")
        }));
    }
}
