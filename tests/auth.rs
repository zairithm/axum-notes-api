//! Sign-up, login, tokens and note ownership.

mod common;

use axum::http::{Method, StatusCode};
use common::{call, call_as, login_as, request};
use serde_json::json;

#[tokio::test]
async fn signup_returns_the_user_without_the_password() {
    let app = axum_notes_api::app();
    let (status, user) = call(
        &app,
        Method::POST,
        "/signup",
        Some(json!({"username": "  Zaid ", "password": "password123"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(user, json!({"id": 1, "username": "zaid"}));
}

#[tokio::test]
async fn duplicate_username_is_409() {
    let app = axum_notes_api::app();
    login_as(&app, "zaid").await;
    let (status, body) = call(
        &app,
        Method::POST,
        "/signup",
        Some(json!({"username": "ZAID", "password": "another123"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body, json!({"error": "that username is taken"}));
}

#[tokio::test]
async fn weak_signup_is_422() {
    let app = axum_notes_api::app();
    let (status, _) = call(
        &app,
        Method::POST,
        "/signup",
        Some(json!({"username": "zaid", "password": "short"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn login_gives_a_bearer_token() {
    let app = axum_notes_api::app();
    login_as(&app, "zaid").await;
    let (status, body) = call(
        &app,
        Method::POST,
        "/login",
        Some(json!({"username": "Zaid", "password": "password123"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["token_type"], "Bearer");
    assert_eq!(body["expires_in"], 86_400);
    assert!(body["token"].as_str().unwrap().split('.').count() == 3);
}

#[tokio::test]
async fn wrong_password_and_unknown_user_look_the_same() {
    let app = axum_notes_api::app();
    login_as(&app, "zaid").await;
    let wrong_pw = call(
        &app,
        Method::POST,
        "/login",
        Some(json!({"username": "zaid", "password": "wrong-password"})),
    )
    .await;
    let no_user = call(
        &app,
        Method::POST,
        "/login",
        Some(json!({"username": "ghost", "password": "password123"})),
    )
    .await;
    assert_eq!(wrong_pw.0, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong_pw, no_user);
}

#[tokio::test]
async fn notes_need_a_valid_token() {
    let app = axum_notes_api::app();
    let (status, body) = call(&app, Method::GET, "/notes", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].as_str().unwrap().contains("Bearer"));

    let (status, _) = request(&app, Method::GET, "/notes", Some("garbage"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // A token signed by a different server is not accepted either.
    let other_server = axum_notes_api::app();
    let foreign = login_as(&other_server, "zaid").await;
    let (status, _) = call_as(&app, &foreign, Method::GET, "/notes", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn users_only_see_their_own_notes() {
    let app = axum_notes_api::app();
    let alice = login_as(&app, "alice").await;
    let bob = login_as(&app, "bob").await;

    let (_, note) = call_as(
        &app,
        &alice,
        Method::POST,
        "/notes",
        Some(json!({"title": "alice's secret"})),
    )
    .await;
    let uri = format!("/notes/{}", note["id"]);

    let (_, bobs_list) = call_as(&app, &bob, Method::GET, "/notes", None).await;
    assert_eq!(bobs_list, json!([]));
    for (method, body) in [
        (Method::GET, None),
        (Method::PUT, Some(json!({"title": "hacked"}))),
        (Method::DELETE, None),
    ] {
        let (status, _) = call_as(&app, &bob, method, &uri, body).await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "bob must not reach alice's note"
        );
    }

    let (status, still) = call_as(&app, &alice, Method::GET, &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(still["title"], "alice's secret");
}
