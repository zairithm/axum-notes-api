//! Calls the router directly, without opening a port.

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;

/// Sends one request and returns the status and the JSON body (`Null` if empty).
async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
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

#[tokio::test]
async fn create_then_get() {
    let app = axum_notes_api::app();
    let (status, note) = call(
        &app,
        Method::POST,
        "/notes",
        Some(json!({"title": "Groceries", "body": "milk, eggs"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        note,
        json!({"id": 1, "title": "Groceries", "body": "milk, eggs"})
    );

    let (status, fetched) = call(&app, Method::GET, "/notes/1", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched, note);
}

#[tokio::test]
async fn list_returns_notes_in_order() {
    let app = axum_notes_api::app();
    for title in ["first", "second"] {
        call(
            &app,
            Method::POST,
            "/notes",
            Some(json!({"title": title, "body": ""})),
        )
        .await;
    }
    let (status, list) = call(&app, Method::GET, "/notes", None).await;
    assert_eq!(status, StatusCode::OK);
    let titles: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["first", "second"]);
}

#[tokio::test]
async fn update_replaces_title_and_body() {
    let app = axum_notes_api::app();
    call(
        &app,
        Method::POST,
        "/notes",
        Some(json!({"title": "old", "body": "old"})),
    )
    .await;
    let (status, note) = call(
        &app,
        Method::PUT,
        "/notes/1",
        Some(json!({"title": "new", "body": "new body"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(note, json!({"id": 1, "title": "new", "body": "new body"}));
}

#[tokio::test]
async fn delete_removes_the_note() {
    let app = axum_notes_api::app();
    call(
        &app,
        Method::POST,
        "/notes",
        Some(json!({"title": "temp", "body": ""})),
    )
    .await;
    let (status, _) = call(&app, Method::DELETE, "/notes/1", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::GET, "/notes/1", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unknown_ids_are_404() {
    let app = axum_notes_api::app();
    let update = Some(json!({"title": "x", "body": "y"}));
    assert_eq!(
        call(&app, Method::GET, "/notes/42", None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, Method::PUT, "/notes/42", update).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, Method::DELETE, "/notes/42", None).await.0,
        StatusCode::NOT_FOUND
    );
}
