//! REST API for notes.
//!
//! | Method | Path          | Result                         |
//! |--------|---------------|--------------------------------|
//! | GET    | `/health`     | `{"status":"ok"}`              |
//! | POST   | `/notes`      | 201 with the created note      |
//! | GET    | `/notes`      | all notes, oldest first        |
//! | GET    | `/notes/{id}` | the note, or 404               |
//! | PUT    | `/notes/{id}` | the updated note, or 404       |
//! | DELETE | `/notes/{id}` | 204, or 404                    |

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

pub mod models;
pub mod store;

use models::{NewNote, Note};
use store::Store;

/// Builds the application router with a fresh, empty store.
pub fn app() -> Router {
    app_with(Store::default())
}

/// Builds the router around an existing store.
pub fn app_with(store: Store) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/notes", get(list_notes).post(create_note))
        .route(
            "/notes/{id}",
            get(get_note).put(update_note).delete(delete_note),
        )
        .with_state(store)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn create_note(
    State(store): State<Store>,
    Json(new): Json<NewNote>,
) -> (StatusCode, Json<Note>) {
    (StatusCode::CREATED, Json(store.create(new)))
}

async fn list_notes(State(store): State<Store>) -> Json<Vec<Note>> {
    Json(store.list())
}

async fn get_note(
    State(store): State<Store>,
    Path(id): Path<u64>,
) -> Result<Json<Note>, StatusCode> {
    store.get(id).map(Json).ok_or(StatusCode::NOT_FOUND)
}

async fn update_note(
    State(store): State<Store>,
    Path(id): Path<u64>,
    Json(new): Json<NewNote>,
) -> Result<Json<Note>, StatusCode> {
    store.update(id, new).map(Json).ok_or(StatusCode::NOT_FOUND)
}

async fn delete_note(State(store): State<Store>, Path(id): Path<u64>) -> StatusCode {
    if store.delete(id) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}
