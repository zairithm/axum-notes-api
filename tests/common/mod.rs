//! Helpers shared by the integration tests.
#![allow(dead_code)] // each test file uses a different subset

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;

/// Sends one request and returns the status and the JSON body (`Null` if empty).
pub async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = match body {
        Some(v) => {
            req = req.header(header::CONTENT_TYPE, "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let res = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, json)
}

/// A request without a token.
pub async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    request(app, method, uri, None, body).await
}

/// A request as a logged-in user.
pub async fn call_as(
    app: &Router,
    token: &str,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    request(app, method, uri, Some(token), body).await
}

/// Signs up `username` (password `password123`) and returns a login token.
pub async fn login_as(app: &Router, username: &str) -> String {
    let creds = json!({"username": username, "password": "password123"});
    let (status, _) = call(app, Method::POST, "/signup", Some(creds.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "signup {username}");
    let (status, body) = call(app, Method::POST, "/login", Some(creds)).await;
    assert_eq!(status, StatusCode::OK, "login {username}");
    body["token"].as_str().unwrap().to_string()
}
