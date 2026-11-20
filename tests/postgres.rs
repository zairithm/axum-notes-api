//! Runs the API against a real PostgreSQL database.
//!
//! Needs `DATABASE_URL`. Without it the test is skipped, unless `REQUIRE_DB` is set
//! (CI sets both, so the database path is always tested there).
//! Everything runs in one test so steps don't race on the shared table.

mod common;

use axum::http::{Method, StatusCode};
use axum_notes_api::postgres::PgStore;
use axum_notes_api::store::Store;
use common::call;
use serde_json::json;

#[tokio::test]
async fn crud_against_postgres() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        assert!(
            std::env::var("REQUIRE_DB").is_err(),
            "REQUIRE_DB is set but DATABASE_URL is missing"
        );
        eprintln!("skipping: DATABASE_URL is not set");
        return;
    };

    let pg = PgStore::connect(&url).await.expect("connect and migrate");
    sqlx::query("TRUNCATE notes RESTART IDENTITY")
        .execute(pg.pool())
        .await
        .unwrap();
    let app = axum_notes_api::app_with(Store::Postgres(pg.clone()));

    // create
    let (status, note) = call(
        &app,
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
    call(
        &app,
        Method::POST,
        "/notes",
        Some(json!({"title": "second"})),
    )
    .await;

    // list, in order
    let (_, list) = call(&app, Method::GET, "/notes", None).await;
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert_eq!(list[1]["title"], "second");

    // update
    let (status, updated) = call(
        &app,
        Method::PUT,
        "/notes/1",
        Some(json!({"title": "edited", "body": "new"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["title"], "edited");

    // data really is in the database: a second app on the same pool sees it
    let other = axum_notes_api::app_with(Store::Postgres(pg));
    let (_, fetched) = call(&other, Method::GET, "/notes/1", None).await;
    assert_eq!(fetched["body"], "new");

    // delete and not found
    let (status, _) = call(&app, Method::DELETE, "/notes/1", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, body) = call(&app, Method::GET, "/notes/1", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({"error": "note 1 not found"}));

    // ids larger than BIGINT are simply not found, not a server error
    let (status, _) = call(&app, Method::GET, &format!("/notes/{}", u64::MAX), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
