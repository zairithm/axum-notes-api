//! Runs the API against a real PostgreSQL database.
//!
//! Needs `DATABASE_URL`. Without it the test is skipped, unless `REQUIRE_DB` is set
//! (CI sets both, so the database path is always tested there).
//! Everything runs in one test so steps don't race on the shared tables.

mod common;

use axum::http::{Method, StatusCode};
use axum_notes_api::auth::Keys;
use axum_notes_api::postgres::PgStore;
use axum_notes_api::store::Store;
use common::{call, call_as, login_as};
use serde_json::json;

#[tokio::test]
async fn full_flow_against_postgres() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        assert!(
            std::env::var("REQUIRE_DB").is_err(),
            "REQUIRE_DB is set but DATABASE_URL is missing"
        );
        eprintln!("skipping: DATABASE_URL is not set");
        return;
    };

    let pg = PgStore::connect(&url).await.expect("connect and migrate");
    sqlx::query("TRUNCATE notes, users RESTART IDENTITY CASCADE")
        .execute(pg.pool())
        .await
        .unwrap();
    let secret = b"a test secret that is long enough!";
    let app = axum_notes_api::app_with(Store::Postgres(pg.clone()), Keys::new(secret));

    // accounts
    let alice = login_as(&app, "alice").await;
    let bob = login_as(&app, "bob").await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/signup",
        Some(json!({"username": "ALICE", "password": "password123"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "unique index on username");

    // create
    let (status, note) = call_as(
        &app,
        &alice,
        Method::POST,
        "/notes",
        Some(json!({"title": "  from postgres ", "body": "hello"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        note,
        json!({"id": 1, "title": "from postgres", "body": "hello"})
    );
    call_as(
        &app,
        &alice,
        Method::POST,
        "/notes",
        Some(json!({"title": "second"})),
    )
    .await;

    // list, in order, and only for the owner
    let (_, list) = call_as(&app, &alice, Method::GET, "/notes", None).await;
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert_eq!(list[1]["title"], "second");
    let (_, bobs) = call_as(&app, &bob, Method::GET, "/notes", None).await;
    assert_eq!(bobs, json!([]));
    let (status, _) = call_as(&app, &bob, Method::GET, "/notes/1", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // update
    let (status, updated) = call_as(
        &app,
        &alice,
        Method::PUT,
        "/notes/1",
        Some(json!({"title": "edited", "body": "new"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["title"], "edited");

    // data really is in the database: a second app with the same secret sees it
    let other = axum_notes_api::app_with(Store::Postgres(pg), Keys::new(secret));
    let (_, fetched) = call_as(&other, &alice, Method::GET, "/notes/1", None).await;
    assert_eq!(fetched["body"], "new");

    // delete and not found
    let (status, _) = call_as(&app, &alice, Method::DELETE, "/notes/1", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, body) = call_as(&app, &alice, Method::GET, "/notes/1", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({"error": "note 1 not found"}));

    // ids larger than BIGINT are simply not found, not a server error
    let huge = format!("/notes/{}", u64::MAX);
    let (status, _) = call_as(&app, &alice, Method::GET, &huge, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
