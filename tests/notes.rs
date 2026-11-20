//! Calls the router directly (in-memory store), without opening a port.

mod common;

use axum::http::{Method, StatusCode};
use common::{call, call_as, login_as};
use serde_json::json;

#[tokio::test]
async fn create_then_get() {
    let app = axum_notes_api::app();
    let t = login_as(&app, "zaid").await;
    let (status, note) = call_as(
        &app,
        &t,
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

    let (status, fetched) = call_as(&app, &t, Method::GET, "/notes/1", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched, note);
}

#[tokio::test]
async fn list_returns_notes_in_order() {
    let app = axum_notes_api::app();
    let t = login_as(&app, "zaid").await;
    for title in ["first", "second"] {
        call_as(
            &app,
            &t,
            Method::POST,
            "/notes",
            Some(json!({"title": title, "body": ""})),
        )
        .await;
    }
    let (status, list) = call_as(&app, &t, Method::GET, "/notes", None).await;
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
    let t = login_as(&app, "zaid").await;
    call_as(
        &app,
        &t,
        Method::POST,
        "/notes",
        Some(json!({"title": "old", "body": "old"})),
    )
    .await;
    let (status, note) = call_as(
        &app,
        &t,
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
    let t = login_as(&app, "zaid").await;
    call_as(
        &app,
        &t,
        Method::POST,
        "/notes",
        Some(json!({"title": "temp", "body": ""})),
    )
    .await;
    let (status, _) = call_as(&app, &t, Method::DELETE, "/notes/1", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call_as(&app, &t, Method::GET, "/notes/1", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unknown_ids_are_404() {
    let app = axum_notes_api::app();
    let t = login_as(&app, "zaid").await;
    let update = Some(json!({"title": "x", "body": "y"}));
    assert_eq!(
        call_as(&app, &t, Method::GET, "/notes/42", None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call_as(&app, &t, Method::PUT, "/notes/42", update).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call_as(&app, &t, Method::DELETE, "/notes/42", None).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn errors_are_json_with_the_right_status() {
    let app = axum_notes_api::app();
    let t = login_as(&app, "zaid").await;

    let (status, body) = call_as(&app, &t, Method::GET, "/notes/42", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({"error": "note 42 not found"}));

    let (status, body) = call_as(&app, &t, Method::GET, "/notes/abc", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].is_string());

    let (status, body) = call(&app, Method::GET, "/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({"error": "no such route"}));
}

#[tokio::test]
async fn invalid_fields_are_422() {
    let app = axum_notes_api::app();
    let t = login_as(&app, "zaid").await;
    let (status, body) = call_as(
        &app,
        &t,
        Method::POST,
        "/notes",
        Some(json!({"title": "   ", "body": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body, json!({"error": "title must not be empty"}));

    call_as(
        &app,
        &t,
        Method::POST,
        "/notes",
        Some(json!({"title": "ok"})),
    )
    .await;
    let (status, _) = call_as(
        &app,
        &t,
        Method::PUT,
        "/notes/1",
        Some(json!({"title": ""})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn bad_json_is_400_and_json() {
    let app = axum_notes_api::app();
    let t = login_as(&app, "zaid").await;
    // Missing the required title field.
    let (status, body) =
        call_as(&app, &t, Method::POST, "/notes", Some(json!({"body": "x"}))).await;
    assert!(status.is_client_error());
    assert!(body["error"].as_str().unwrap().contains("title"));

    // Unknown field.
    let (status, body) = call_as(
        &app,
        &t,
        Method::POST,
        "/notes",
        Some(json!({"title": "t", "colour": "red"})),
    )
    .await;
    assert!(status.is_client_error());
    assert!(body["error"].as_str().unwrap().contains("colour"));
}

#[tokio::test]
async fn title_is_trimmed_and_body_optional() {
    let app = axum_notes_api::app();
    let t = login_as(&app, "zaid").await;
    let (status, note) = call_as(
        &app,
        &t,
        Method::POST,
        "/notes",
        Some(json!({"title": "  tidy  "})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(note, json!({"id": 1, "title": "tidy", "body": ""}));
}
