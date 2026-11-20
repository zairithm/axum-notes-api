//! REST API for notes, with accounts.
//!
//! | Method | Path          | Auth | Result                              |
//! |--------|---------------|------|-------------------------------------|
//! | GET    | `/health`     |      | `{"status":"ok"}`                   |
//! | POST   | `/signup`     |      | 201 with `{id, username}`           |
//! | POST   | `/login`      |      | `{token, token_type, expires_in}`   |
//! | POST   | `/notes`      | yes  | 201 with the created note           |
//! | GET    | `/notes`      | yes  | your notes, oldest first            |
//! | GET    | `/notes/{id}` | yes  | the note, or 404                    |
//! | PUT    | `/notes/{id}` | yes  | the updated note, or 404            |
//! | DELETE | `/notes/{id}` | yes  | 204, or 404                         |
//!
//! "Auth" routes need `Authorization: Bearer <token>` from `/login`. Someone else's
//! note answers 404, the same as a note that doesn't exist.
//!
//! Every error is JSON: `{"error": "..."}` with 400 (bad JSON or path), 401 (not
//! logged in, or wrong password), 404, 409 (username taken) or 422 (invalid field).

use std::sync::Arc;

use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub mod auth;
pub mod error;
pub mod memory;
pub mod models;
pub mod postgres;
pub mod store;

use auth::{AuthUser, Keys};
use error::{AppError, AppJson, AppPath};
use models::{Credentials, NewNote, Note, Token, User};
use store::Store;

/// Everything the handlers share.
#[derive(Clone, FromRef)]
pub struct AppState {
    pub store: Store,
    pub keys: Arc<Keys>,
}

/// Builds the router with a fresh in-memory store and random signing keys.
pub fn app() -> Router {
    app_with(Store::default(), Keys::random())
}

/// Builds the router around an existing store and signing keys.
pub fn app_with(store: Store, keys: Keys) -> Router {
    let state = AppState {
        store,
        keys: Arc::new(keys),
    };
    Router::new()
        .route("/health", get(health))
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/notes", get(list_notes).post(create_note))
        .route(
            "/notes/{id}",
            get(get_note).put(update_note).delete(delete_note),
        )
        .fallback(not_found)
        .with_state(state)
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

/// Runs CPU-heavy password work off the async runtime.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AppError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|_| AppError::Internal)
}

async fn signup(
    State(store): State<Store>,
    AppJson(creds): AppJson<Credentials>,
) -> Result<(StatusCode, Json<User>), AppError> {
    let Credentials { username, password } = creds.validate_new()?;
    let hash = blocking(move || auth::hash_password(&password)).await?;
    let id = store.create_user(&username, hash).await?;
    Ok((StatusCode::CREATED, Json(User { id, username })))
}

async fn login(
    State(state): State<AppState>,
    AppJson(creds): AppJson<Credentials>,
) -> Result<Json<Token>, AppError> {
    let user = state
        .store
        .user_by_name(&creds.normalized_username())
        .await?;
    // Check a password even when the user doesn't exist, so both cases take as long.
    let (id, hash) = match user {
        Some((id, hash)) => (Some(id), hash),
        None => (None, auth::dummy_hash().to_string()),
    };
    let password = creds.password;
    let ok = blocking(move || auth::verify_password(&password, &hash)).await?;
    match id {
        Some(id) if ok => Ok(Json(Token {
            token: auth::issue_token(&state.keys, id),
            token_type: "Bearer".into(),
            expires_in: auth::TOKEN_TTL_SECS,
        })),
        _ => Err(AppError::BadLogin),
    }
}

async fn create_note(
    AuthUser(user): AuthUser,
    State(store): State<Store>,
    AppJson(new): AppJson<NewNote>,
) -> Result<(StatusCode, Json<Note>), AppError> {
    let new = new.validate()?;
    Ok((StatusCode::CREATED, Json(store.create(user, new).await?)))
}

async fn list_notes(
    AuthUser(user): AuthUser,
    State(store): State<Store>,
) -> Result<Json<Vec<Note>>, AppError> {
    Ok(Json(store.list(user).await?))
}

async fn get_note(
    AuthUser(user): AuthUser,
    State(store): State<Store>,
    AppPath(id): AppPath<u64>,
) -> Result<Json<Note>, AppError> {
    Ok(Json(store.get(user, id).await?))
}

async fn update_note(
    AuthUser(user): AuthUser,
    State(store): State<Store>,
    AppPath(id): AppPath<u64>,
    AppJson(new): AppJson<NewNote>,
) -> Result<Json<Note>, AppError> {
    let new = new.validate()?;
    Ok(Json(store.update(user, id, new).await?))
}

async fn delete_note(
    AuthUser(user): AuthUser,
    State(store): State<Store>,
    AppPath(id): AppPath<u64>,
) -> Result<StatusCode, AppError> {
    store.delete(user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
