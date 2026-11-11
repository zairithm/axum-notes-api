use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

pub mod models;

/// Builds the application router.
pub fn app() -> Router {
    Router::new().route("/health", get(health))
    // TODO(#1): add the /notes routes.
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
