use crate::harness::compatibility::CompatibilityProfile;
use crate::harness::definition::{
    parse_custom_definition, parse_strict_json, DefinitionOrigin, HarnessDefinition,
    HarnessDiagnostic, HarnessPatch, IntegrationBinding, SessionSignalBinding,
};
use crate::harness::store::{HarnessDocumentRevision, HarnessSnapshot, HarnessStoreError};
use crate::AppState;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{http::StatusCode, Json};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug)]
enum UpdateHarnessChange {
    BuiltinPatch { patch: HarnessPatch },
    CustomReplace { definition: HarnessDefinition },
}

#[derive(Debug)]
pub(crate) struct UpdateHarnessRequest {
    expected_revision: Option<HarnessDocumentRevision>,
    change: UpdateHarnessChange,
}

#[derive(Debug)]
struct CreateHarnessRequest {
    definition: HarnessDefinition,
    expected_revision: Option<HarnessDocumentRevision>,
    duplicate_source_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessesResponse {
    document_revision: Option<HarnessDocumentRevision>,
    harnesses: Vec<HarnessConfigEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessMutationResponse {
    document_revision: HarnessDocumentRevision,
    harness: HarnessConfigEntry,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateHarnessResponse {
    document_revision: Option<HarnessDocumentRevision>,
    definition: serde_json::Value,
    proposed_id: String,
    proposed_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessConfigEntry {
    definition: HarnessDefinition,
    origin: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    stored_override: Option<HarnessPatch>,
    compatibility: CompatibilityResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompatibilityResponse {
    profile: Option<CompatibilityProfile>,
    session_signals: Option<SessionSignalBinding>,
    integration: Option<IntegrationBinding>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessErrorResponse {
    error: String,
    diagnostics: Vec<HarnessDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_revision: Option<HarnessDocumentRevision>,
}

pub(crate) async fn list_harnesses(State(state): State<Arc<AppState>>) -> axum::response::Response {
    let snapshot = match state.harness_store.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => return store_error(error),
    };
    Json(HarnessesResponse {
        document_revision: snapshot.document_revision.clone(),
        harnesses: snapshot_entries(&snapshot),
    })
    .into_response()
}

pub(crate) async fn create_harness(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> axum::response::Response {
    let request = match parse_create_request(&body) {
        Ok(request) => request,
        Err(diagnostics) => return store_error(HarnessStoreError::Validation(diagnostics)),
    };
    let id = request.definition.id.clone();
    let duplicate_profile = if let Some(source_id) = request.duplicate_source_id.as_deref() {
        let snapshot = match state.harness_store.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => return store_error(error),
        };
        match duplicate_profile_for_source(&snapshot, source_id) {
            Ok(profile) => profile,
            Err(diagnostic) => return store_error(HarnessStoreError::Mutation(diagnostic)),
        }
    } else {
        None
    };
    let store = state.harness_store.clone();
    let catalog = state.harness_catalog.clone();
    let profile_target_id = id.clone();
    let result = tokio::task::spawn_blocking(move || {
        store.mutate_at(&catalog, request.expected_revision, |document| {
            if document
                .custom
                .iter()
                .any(|custom| custom.id == request.definition.id)
                || document.overrides.contains_key(&request.definition.id)
            {
                return Err(HarnessDiagnostic::for_id(
                    &request.definition.id,
                    "custom_id_collision",
                    "Harness ID already exists.",
                ));
            }
            document.custom.push(request.definition);
            if let Some(profile) = duplicate_profile {
                document.set_compatibility_profile(&profile_target_id, profile)?;
            }
            Ok(())
        })
    })
    .await;
    match result {
        Ok(Ok(mutation)) => {
            state.bump_harness_probe_generation();
            state.providers.reconcile_harness_provider_settings();
            if mutation.registry.get(&id).is_some() {
                mutation_response(mutation, &id, StatusCode::CREATED)
            } else {
                internal_error("created harness was not resolved")
            }
        }
        Ok(Err(error)) => store_error(error),
        Err(_) => internal_error("harness update task failed"),
    }
}

pub(crate) async fn update_harness(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Bytes,
) -> axum::response::Response {
    let snapshot = match state.harness_store.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => return store_error(error),
    };
    if snapshot.registry.get(&id).is_none() {
        return not_found(&id);
    }
    let request = match parse_update_request(&body) {
        Ok(request) => request,
        Err(diagnostics) => return store_error(HarnessStoreError::Validation(diagnostics)),
    };
    let store = state.harness_store.clone();
    let catalog = state.harness_catalog.clone();
    let requested_id = id.clone();
    let result = tokio::task::spawn_blocking(move || {
        store.mutate_at(
            &catalog,
            request.expected_revision,
            |document| match request.change {
                UpdateHarnessChange::BuiltinPatch { patch } => {
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
                UpdateHarnessChange::CustomReplace { mut definition } => {
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
            },
        )
    })
    .await;
    match result {
        Ok(Ok(mutation)) => {
            state.bump_harness_probe_generation();
            state.providers.reconcile_harness_provider_settings();
            if mutation.registry.get(&id).is_some() {
                mutation_response(mutation, &id, StatusCode::OK)
            } else {
                internal_error("updated harness was not resolved")
            }
        }
        Ok(Err(error)) => store_error(error),
        Err(_) => internal_error("harness update task failed"),
    }
}

pub(crate) async fn delete_harness(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Bytes,
) -> axum::response::Response {
    let expected = match parse_expected_revision(&body) {
        Ok(expected) => expected,
        Err(diagnostics) => return store_error(HarnessStoreError::Validation(diagnostics)),
    };
    delete_harness_at(state, id, expected).await
}

async fn delete_harness_at(
    state: Arc<AppState>,
    id: String,
    expected_revision: Option<HarnessDocumentRevision>,
) -> axum::response::Response {
    let snapshot = match state.harness_store.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => return store_error(error),
    };
    if snapshot.registry.get(&id).is_none() {
        return not_found(&id);
    }

    if state
        .workspace
        .lock()
        .expect("workspace lock poisoned")
        .as_ref()
        .is_some_and(|workspace| {
            workspace
                .metadata
                .read_workspace_memory()
                .is_some_and(|memory| {
                    memory
                        .active_harness_ids
                        .iter()
                        .any(|active_id| active_id == &id)
                })
        })
    {
        return store_error(HarnessStoreError::Mutation(HarnessDiagnostic::for_id(
            &id,
            "active_harness_delete_forbidden",
            "Disable this coding tool in the current workspace before deleting it.",
        )));
    }
    let store = state.harness_store.clone();
    let catalog = state.harness_catalog.clone();
    let requested_id = id.clone();
    let result = tokio::task::spawn_blocking(move || {
        store.mutate_at(&catalog, expected_revision, |document| {
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
            state.providers.reconcile_harness_provider_settings();
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(Err(error)) => store_error(error),
        Err(_) => internal_error("harness update task failed"),
    }
}

pub(crate) async fn duplicate_harness(
    State(state): State<Arc<AppState>>,
    Path(source_id): Path<String>,
) -> axum::response::Response {
    let snapshot = match state.harness_store.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => return store_error(error),
    };
    let Some(source) = snapshot.registry.get(&source_id) else {
        return not_found(&source_id);
    };
    let (proposed_id, proposed_name) =
        duplicate_identity(&snapshot, source_id.as_str(), &source.definition.name);
    let mut definition = source.definition.clone();
    definition.id = proposed_id.clone();
    definition.name = proposed_name.clone();
    definition.session_signals = None;
    definition.integration = None;
    Json(DuplicateHarnessResponse {
        document_revision: snapshot.document_revision,
        definition: editable_definition_value(&definition),
        proposed_id,
        proposed_name,
    })
    .into_response()
}

pub(crate) async fn remove_harness_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Bytes,
) -> axum::response::Response {
    let expected_revision = match parse_expected_revision(&body) {
        Ok(expected) => expected,
        Err(diagnostics) => return store_error(HarnessStoreError::Validation(diagnostics)),
    };
    let store = state.harness_store.clone();
    let catalog = state.harness_catalog.clone();
    let requested_id = id.clone();
    let result = tokio::task::spawn_blocking(move || {
        store.mutate_at(&catalog, expected_revision, |document| {
            let Some(position) = document
                .custom
                .iter()
                .position(|definition| definition.id == requested_id)
            else {
                return Err(HarnessDiagnostic::for_id(
                    &requested_id,
                    "custom_not_found",
                    "Custom harness was not found.",
                ));
            };
            if document.compatibility_profile(&requested_id).is_none() {
                return Err(HarnessDiagnostic::for_id(
                    &requested_id,
                    "compatibility_profile_not_found",
                    "Custom harness has no compatibility profile.",
                ));
            }
            let definition = document.custom[position].clone();
            document.remove_custom_definition(&requested_id);
            document.custom.insert(position, definition);
            Ok(())
        })
    })
    .await;
    match result {
        Ok(Ok(mutation)) => {
            state.bump_harness_probe_generation();
            state.providers.reconcile_harness_provider_settings();
            mutation_response(mutation, &id, StatusCode::OK)
        }
        Ok(Err(error)) => store_error(error),
        Err(_) => internal_error("harness update task failed"),
    }
}

fn snapshot_entries(snapshot: &HarnessSnapshot) -> Vec<HarnessConfigEntry> {
    snapshot
        .registry
        .ids()
        .filter_map(|id| snapshot.registry.get(id).map(|harness| (id, harness)))
        .map(|(id, harness)| HarnessConfigEntry {
            definition: harness.definition.clone(),
            origin: origin_name(harness.origin),
            stored_override: snapshot.stored_patches.get(id).cloned(),
            compatibility: CompatibilityResponse {
                profile: harness.compatibility.profile,
                session_signals: harness.compatibility.session_signals.clone(),
                integration: harness.compatibility.integration.clone(),
            },
        })
        .collect()
}

fn mutation_response(
    mutation: crate::harness::store::HarnessMutation,
    id: &str,
    status: StatusCode,
) -> axum::response::Response {
    let Some(harness) = mutation.registry.get(id) else {
        return internal_error("updated harness was not resolved");
    };
    (
        status,
        Json(HarnessMutationResponse {
            document_revision: mutation.document_revision,
            harness: HarnessConfigEntry {
                definition: harness.definition.clone(),
                origin: origin_name(harness.origin),
                stored_override: mutation.stored_patches.get(id).cloned(),
                compatibility: CompatibilityResponse {
                    profile: harness.compatibility.profile,
                    session_signals: harness.compatibility.session_signals.clone(),
                    integration: harness.compatibility.integration.clone(),
                },
            },
        }),
    )
        .into_response()
}

fn origin_name(origin: DefinitionOrigin) -> &'static str {
    match origin {
        DefinitionOrigin::Builtin => "builtin",
        DefinitionOrigin::Override => "override",
        DefinitionOrigin::Custom => "custom",
    }
}

fn editable_definition_value(definition: &HarnessDefinition) -> serde_json::Value {
    let mut value = serde_json::to_value(definition).expect("harness definitions serialize");
    if let Some(object) = value.as_object_mut() {
        object.remove("sessionSignals");
        object.remove("integration");
    }
    value
}

fn duplicate_profile_for_source(
    snapshot: &HarnessSnapshot,
    source_id: &str,
) -> Result<Option<CompatibilityProfile>, HarnessDiagnostic> {
    let source = snapshot.registry.get(source_id).ok_or_else(|| {
        HarnessDiagnostic::for_id(source_id, "harness_not_found", "Harness was not found.")
    })?;
    if let Some(profile) = source.compatibility.profile {
        return Ok(Some(profile));
    }
    Ok(matches!(
        (
            source.compatibility.session_signals.as_ref(),
            source.compatibility.integration.as_ref(),
        ),
        (
            Some(SessionSignalBinding::Copilot),
            Some(IntegrationBinding::Copilot)
        )
    )
    .then_some(CompatibilityProfile::Copilot))
}

fn duplicate_identity(
    snapshot: &HarnessSnapshot,
    source_id: &str,
    source_name: &str,
) -> (String, String) {
    let base_id = if source_id == "copilot" {
        "copilot-local".to_owned()
    } else {
        format!("{source_id}-copy")
    };
    let base_name = if source_id == "copilot" {
        "Copilot Local".to_owned()
    } else {
        format!("{source_name} Copy")
    };
    let mut number = 2;
    let mut id = base_id.clone();
    let mut name = base_name.clone();
    while snapshot.registry.get(&id).is_some() {
        id = format!("{base_id}-{number}");
        name = format!("{base_name} {number}");
        number += 1;
    }
    (id, name)
}

fn parse_create_request(bytes: &[u8]) -> Result<CreateHarnessRequest, Vec<HarnessDiagnostic>> {
    let object = strict_request_object(bytes, "Harness create request")?;
    reject_unknown_fields(
        &object,
        &["definition", "expectedRevision", "duplicateSourceId"],
    )?;
    let definition = parse_request_definition(&object, "definition")?;
    let expected_revision = parse_optional_revision(&object)?;
    let duplicate_source_id = match object.get("duplicateSourceId") {
        Some(serde_json::Value::String(id)) => Some(id.clone()),
        Some(serde_json::Value::Null) | None => None,
        Some(_) => {
            return Err(vec![HarnessDiagnostic::document(
                "invalid_schema",
                "duplicateSourceId must be a string or null.",
                Some("$.duplicateSourceId"),
            )]);
        }
    };
    Ok(CreateHarnessRequest {
        definition,
        expected_revision,
        duplicate_source_id,
    })
}

fn parse_expected_revision(
    bytes: &[u8],
) -> Result<Option<HarnessDocumentRevision>, Vec<HarnessDiagnostic>> {
    let object = strict_request_object(bytes, "Harness revision request")?;
    reject_unknown_fields(&object, &["expectedRevision"])?;
    parse_optional_revision(&object)
}

fn not_found(id: &str) -> axum::response::Response {
    (
        StatusCode::NOT_FOUND,
        Json(HarnessErrorResponse {
            error: "Harness was not found.".into(),
            diagnostics: vec![HarnessDiagnostic::for_id(
                id,
                "harness_not_found",
                "Harness was not found.",
            )],
            document_revision: None,
        }),
    )
        .into_response()
}

fn store_error(error: HarnessStoreError) -> axum::response::Response {
    let (status, diagnostics, document_revision) = match error {
        HarnessStoreError::Validation(diagnostics) => (StatusCode::BAD_REQUEST, diagnostics, None),
        HarnessStoreError::Mutation(diagnostic) => (StatusCode::CONFLICT, vec![diagnostic], None),
        HarnessStoreError::RevisionChanged { current } => (
            StatusCode::CONFLICT,
            vec![HarnessDiagnostic {
                harness_id: None,
                code: "harness_config_revision_changed".into(),
                message: "Harness configuration changed; retry the request.".into(),
                path: None,
            }],
            current,
        ),
        HarnessStoreError::Io(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            vec![HarnessDiagnostic {
                harness_id: None,
                code: "write_failed".into(),
                message: error.to_string(),
                path: None,
            }],
            None,
        ),
        HarnessStoreError::Parse(error) => (
            StatusCode::BAD_REQUEST,
            vec![HarnessDiagnostic {
                harness_id: None,
                code: "invalid_document".into(),
                message: error.to_string(),
                path: None,
            }],
            None,
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
            document_revision,
        }),
    )
        .into_response()
}

fn parse_update_request(bytes: &[u8]) -> Result<UpdateHarnessRequest, Vec<HarnessDiagnostic>> {
    let object = strict_request_object(bytes, "Harness update request")?;
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
        "BuiltinPatch" => &["kind", "patch", "expectedRevision"][..],
        "CustomReplace" => &["kind", "definition", "expectedRevision"][..],
        _ => {
            return Err(vec![HarnessDiagnostic::document(
                "invalid_schema",
                "Harness update kind must be BuiltinPatch or CustomReplace.",
                Some("$.kind"),
            )]);
        }
    };
    reject_unknown_fields(&object, allowed)?;
    let expected_revision = parse_optional_revision(&object)?;
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
                .map(|patch| UpdateHarnessRequest {
                    expected_revision,
                    change: UpdateHarnessChange::BuiltinPatch { patch },
                })
                .map_err(|error| {
                    vec![HarnessDiagnostic::document(
                        "invalid_schema",
                        &error.to_string(),
                        Some("$.patch"),
                    )]
                })
        }
        "CustomReplace" => {
            parse_request_definition(&object, "definition").map(|definition| UpdateHarnessRequest {
                expected_revision,
                change: UpdateHarnessChange::CustomReplace { definition },
            })
        }
        _ => unreachable!("request kind was validated above"),
    }
}

fn strict_request_object(
    bytes: &[u8],
    description: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, Vec<HarnessDiagnostic>> {
    let value = parse_strict_json::<serde_json::Value>(bytes, 256 * 1024)
        .map_err(|diagnostic| vec![diagnostic])?;
    value.as_object().cloned().ok_or_else(|| {
        vec![HarnessDiagnostic::document(
            "invalid_schema",
            &format!("{description} must be a JSON object."),
            Some("$"),
        )]
    })
}

fn reject_unknown_fields(
    object: &serde_json::Map<String, serde_json::Value>,
    allowed: &[&str],
) -> Result<(), Vec<HarnessDiagnostic>> {
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(vec![HarnessDiagnostic::document(
            "unknown_field",
            &format!("Unknown harness request field {field}."),
            Some(&format!("$.{field}")),
        )]);
    }
    Ok(())
}

fn parse_request_definition(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<HarnessDefinition, Vec<HarnessDiagnostic>> {
    let definition = object.get(field).cloned().ok_or_else(|| {
        vec![HarnessDiagnostic::document(
            "missing_field",
            &format!("Harness request requires {field}."),
            Some(&format!("$.{field}")),
        )]
    })?;
    let serialized = serde_json::to_vec(&definition).map_err(|error| {
        vec![HarnessDiagnostic::document(
            "invalid_schema",
            &error.to_string(),
            Some(&format!("$.{field}")),
        )]
    })?;
    parse_custom_definition(&serialized)
}

fn parse_optional_revision(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<HarnessDocumentRevision>, Vec<HarnessDiagnostic>> {
    match object.get("expectedRevision") {
        Some(value) => serde_json::from_value(value.clone()).map_err(|error| {
            vec![HarnessDiagnostic::document(
                "invalid_schema",
                &format!("expectedRevision must be a revision string or null: {error}"),
                Some("$.expectedRevision"),
            )]
        }),
        None => Ok(None),
    }
}

fn internal_error(message: &str) -> axum::response::Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(HarnessErrorResponse {
            error: message.into(),
            diagnostics: Vec::new(),
            document_revision: None,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn json_body(response: axum::response::Response) -> serde_json::Value {
        serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap()
    }

    fn custom_definition(id: &str, name: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "name": name,
            "launch": {
                "kind": "command-template",
                "command": id,
                "args": [],
                "modelPrefix": null
            }
        })
    }

    #[tokio::test]
    async fn stale_harness_write_returns_the_current_revision_without_clobbering() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let first_snapshot =
            json_body(list_harnesses(State(state.clone())).await.into_response()).await;
        let second_snapshot =
            json_body(list_harnesses(State(state.clone())).await.into_response()).await;

        let created = create_harness(
            State(state.clone()),
            Bytes::from(
                serde_json::json!({
                    "definition": custom_definition("local", "Local"),
                    "expectedRevision": first_snapshot["documentRevision"],
                })
                .to_string(),
            ),
        )
        .await
        .into_response();
        assert_eq!(created.status(), StatusCode::CREATED);
        let created = json_body(created).await;

        let stale = update_harness(
            State(state.clone()),
            Path("local".into()),
            Bytes::from(
                serde_json::json!({
                    "kind": "CustomReplace",
                    "definition": custom_definition("local", "Stale Local"),
                    "expectedRevision": second_snapshot["documentRevision"],
                })
                .to_string(),
            ),
        )
        .await
        .into_response();
        let stale_status = stale.status();
        let stale = json_body(stale).await;
        assert_eq!(stale_status, StatusCode::CONFLICT, "{stale}");
        assert_eq!(
            stale["diagnostics"][0]["code"],
            "harness_config_revision_changed"
        );
        assert_eq!(stale["documentRevision"], created["documentRevision"]);

        let snapshot = json_body(list_harnesses(State(state)).await.into_response()).await;
        assert!(snapshot["harnesses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|harness| {
                harness["definition"]["id"] == "local" && harness["definition"]["name"] == "Local"
            }));
    }

    #[tokio::test]
    async fn duplicate_preview_does_not_mutate_and_final_save_derives_and_preserves_profile() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let before = json_body(list_harnesses(State(state.clone())).await.into_response()).await;

        let preview = duplicate_harness(State(state.clone()), Path("copilot".into()))
            .await
            .into_response();
        assert_eq!(preview.status(), StatusCode::OK);
        let preview = json_body(preview).await;
        assert_eq!(preview["definition"]["id"], "copilot-local");
        let after_preview =
            json_body(list_harnesses(State(state.clone())).await.into_response()).await;
        assert_eq!(
            after_preview["documentRevision"],
            before["documentRevision"]
        );
        assert_eq!(after_preview["harnesses"], before["harnesses"]);

        let created = create_harness(
            State(state.clone()),
            Bytes::from(
                serde_json::json!({
                    "definition": preview["definition"].clone(),
                    "duplicateSourceId": "copilot",
                    "expectedRevision": preview["documentRevision"],
                })
                .to_string(),
            ),
        )
        .await
        .into_response();
        assert_eq!(created.status(), StatusCode::CREATED);
        let created = json_body(created).await;
        assert_eq!(created["harness"]["compatibility"]["profile"], "copilot");
        assert!(state
            .providers
            .get_providers_response()
            .providers
            .iter()
            .any(|provider| provider.id == "copilot-local"));

        let replaced = update_harness(
            State(state.clone()),
            Path("copilot-local".into()),
            Bytes::from(
                serde_json::json!({
                    "kind": "CustomReplace",
                    "definition": custom_definition("copilot-local", "Renamed Local"),
                    "expectedRevision": created["documentRevision"],
                })
                .to_string(),
            ),
        )
        .await
        .into_response();
        let replaced_status = replaced.status();
        let replaced = json_body(replaced).await;
        assert_eq!(replaced_status, StatusCode::OK, "{replaced}");
        assert_eq!(replaced["harness"]["compatibility"]["profile"], "copilot");
    }

    #[tokio::test]
    async fn delete_rejects_active_custom_harness_and_removes_its_profile_when_inactive() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let revision = json_body(list_harnesses(State(state.clone())).await.into_response()).await
            ["documentRevision"]
            .clone();
        let created = create_harness(
            State(state.clone()),
            Bytes::from(
                serde_json::json!({
                    "definition": custom_definition("copilot-local", "Copilot Local"),
                    "duplicateSourceId": "copilot",
                    "expectedRevision": revision,
                })
                .to_string(),
            ),
        )
        .await
        .into_response();
        let created = json_body(created).await;

        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_workspace_memory(&crate::metadata::WorkspaceMemory {
                last_active_session_id: None,
                last_active_at: None,
                active_harness_ids: vec!["copilot-local".into()],
                active_harness_revision: 1,
            });
        let active_delete = delete_harness(
            State(state.clone()),
            Path("copilot-local".into()),
            Bytes::from(
                serde_json::json!({ "expectedRevision": created["documentRevision"] }).to_string(),
            ),
        )
        .await;
        assert_eq!(active_delete.status(), StatusCode::CONFLICT);
        let active_delete = json_body(active_delete).await;
        assert_eq!(
            active_delete["diagnostics"][0]["code"],
            "active_harness_delete_forbidden"
        );

        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_workspace_memory(&crate::metadata::WorkspaceMemory {
                last_active_session_id: None,
                last_active_at: None,
                active_harness_ids: vec![],
                active_harness_revision: 2,
            });

        let inactive_delete = delete_harness(
            State(state.clone()),
            Path("copilot-local".into()),
            Bytes::from(
                serde_json::json!({ "expectedRevision": created["documentRevision"] }).to_string(),
            ),
        )
        .await;
        let inactive_delete_status = inactive_delete.status();
        assert_eq!(inactive_delete_status, StatusCode::NO_CONTENT);
        assert!(state
            .harness_store
            .snapshot()
            .unwrap()
            .compatibility_profiles
            .get("copilot-local")
            .is_none());
        assert!(!state
            .providers
            .get_providers_response()
            .providers
            .iter()
            .any(|provider| provider.id == "copilot-local"));
    }

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
