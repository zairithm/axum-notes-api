//! REST API for notes.
//!
//! | Method | Path          | Result                              |
//! |--------|---------------|-------------------------------------|
//! | GET    | `/health`     | `{"status":"ok"}`                   |
//! | POST   | `/notes`      | 201 with the created note           |
//! | GET    | `/notes`      | all notes, oldest first             |
//! | GET    | `/notes/{id}` | the note, or 404                    |
//! | PUT    | `/notes/{id}` | the updated note, or 404            |
//! | DELETE | `/notes/{id}` | 204, or 404                         |
//!
//! Every error is JSON: `{"error": "..."}` with 400 (bad JSON or path),
//! 404 (unknown note or route) or 422 (a field is not acceptable).

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

pub mod error;
pub mod memory;
pub mod models;
pub mod postgres;
pub mod store;

use error::{AppError, AppJson, AppPath};
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
        .fallback(not_found)
        .with_state(store)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn not_found() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "no such route" })),
    )
}

async fn create_note(
    State(store): State<Store>,
    AppJson(new): AppJson<NewNote>,
) -> Result<(StatusCode, Json<Note>), AppError> {
    let new = new.validate()?;
    Ok((StatusCode::CREATED, Json(store.create(new).await?)))
}

async fn list_notes(State(store): State<Store>) -> Result<Json<Vec<Note>>, AppError> {
    Ok(Json(store.list().await?))
}

async fn get_note(
    State(store): State<Store>,
    AppPath(id): AppPath<u64>,
) -> Result<Json<Note>, AppError> {
    Ok(Json(store.get(id).await?))
}

async fn update_note(
    State(store): State<Store>,
    AppPath(id): AppPath<u64>,
    AppJson(new): AppJson<NewNote>,
) -> Result<Json<Note>, AppError> {
    let new = new.validate()?;
    Ok(Json(store.update(id, new).await?))
}

async fn delete_note(
    State(store): State<Store>,
    AppPath(id): AppPath<u64>,
) -> Result<StatusCode, AppError> {
    store.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
