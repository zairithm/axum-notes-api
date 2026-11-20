//! One error type for the whole API, always answered as `{"error": "..."}`.

use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::FromRequest;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, PartialEq, Eq)]
pub enum AppError {
    /// No note with this id.
    NotFound(u64),
    /// The request was well-formed but a field is not acceptable.
    Invalid(String),
    /// The body was not valid JSON for this endpoint.
    BadJson(String),
    /// The path did not match, e.g. `/notes/abc`.
    BadPath(String),
    /// Missing, malformed or expired login token.
    Unauthorized,
    /// Login with an unknown username or the wrong password (deliberately the same message).
    BadLogin,
    /// The username is already registered.
    UsernameTaken,
    /// Something failed on our side (e.g. the database). Details are logged, not sent.
    Internal,
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::BadJson(_) | Self::BadPath(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized | Self::BadLogin => StatusCode::UNAUTHORIZED,
            Self::UsernameTaken => StatusCode::CONFLICT,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::NotFound(id) => format!("note {id} not found"),
            Self::Invalid(msg) | Self::BadJson(msg) | Self::BadPath(msg) => msg.clone(),
            Self::Unauthorized => "log in first: send Authorization: Bearer <token>".into(),
            Self::BadLogin => "wrong username or password".into(),
            Self::UsernameTaken => "that username is taken".into(),
            Self::Internal => "internal error".into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status(), Json(json!({ "error": self.message() }))).into_response()
    }
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        Self::BadJson(rejection.body_text())
    }
}

impl From<PathRejection> for AppError {
    fn from(rejection: PathRejection) -> Self {
        Self::BadPath(rejection.body_text())
    }
}

/// Like `axum::Json`, but a bad body becomes an [`AppError`] (JSON) instead of plain text.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct AppJson<T>(pub T);

/// Like `axum::extract::Path`, but a bad path becomes an [`AppError`].
#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(AppError))]
pub struct AppPath<T>(pub T);
