use crate::harness_registry::{save_harnesses, HarnessConfig};
use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use std::sync::Arc;

pub(crate) async fn list_harnesses(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let harnesses = state.harnesses.read().await;
    Json(harnesses.clone())
}

pub(crate) async fn create_harness(
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<HarnessConfig>,
) -> impl IntoResponse {
    req.is_builtin = false;
    if req.id.is_empty() {
        req.id = uuid::Uuid::new_v4().to_string();
    }
    let mut harnesses = state.harnesses.write().await;
    harnesses.push(req.clone());
    save_harnesses(&harnesses);
    (axum::http::StatusCode::CREATED, Json(req)).into_response()
}

pub(crate) async fn update_harness(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<HarnessConfig>,
) -> impl IntoResponse {
    let mut harnesses = state.harnesses.write().await;
    if let Some(pos) = harnesses.iter().position(|h| h.id == id) {
        let is_builtin = harnesses[pos].is_builtin;
        harnesses[pos] = HarnessConfig {
            id: id.clone(),
            is_builtin,
            ..req
        };
        save_harnesses(&harnesses);
        Json(harnesses[pos].clone()).into_response()
    } else {
        axum::http::StatusCode::NOT_FOUND.into_response()
    }
}

pub(crate) async fn delete_harness(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut harnesses = state.harnesses.write().await;
    if let Some(pos) = harnesses.iter().position(|h| h.id == id) {
        if harnesses[pos].is_builtin {
            return (axum::http::StatusCode::CONFLICT, "Cannot delete a built-in harness")
                .into_response();
        }
        harnesses.remove(pos);
        save_harnesses(&harnesses);
        axum::http::StatusCode::OK.into_response()
    } else {
        axum::http::StatusCode::NOT_FOUND.into_response()
    }
}
