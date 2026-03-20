use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use std::sync::Arc;

use crate::store::{RegisterPeer, Store};

pub fn router(store: Arc<Store>) -> Router {
    Router::new()
        .route("/peers", post(register_peer))
        .route("/peers", get(list_peers))
        .route("/peers/{id}", delete(remove_peer))
        .route("/health", get(health))
        .with_state(store)
}

async fn health() -> &'static str {
    "ok"
}

async fn register_peer(
    State(store): State<Arc<Store>>,
    Json(req): Json<RegisterPeer>,
) -> impl IntoResponse {
    match store.register_peer(&req) {
        Ok(peer) => (StatusCode::OK, Json(serde_json::json!(peer))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn list_peers(State(store): State<Arc<Store>>) -> impl IntoResponse {
    match store.list_peers() {
        Ok(peers) => (StatusCode::OK, Json(serde_json::json!({"peers": peers}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn remove_peer(
    State(store): State<Arc<Store>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match store.remove_peer(&id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "peer not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
