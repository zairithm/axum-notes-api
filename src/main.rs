use axum_notes_api::postgres::PgStore;
use axum_notes_api::store::Store;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let store = match std::env::var("DATABASE_URL") {
        Ok(url) => {
            let store = PgStore::connect(&url).await?;
            println!("using PostgreSQL (migrations applied)");
            Store::Postgres(store)
        }
        Err(_) => {
            println!("DATABASE_URL is not set, so notes are kept in memory only");
            Store::default()
        }
    };

    let addr = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("listening on http://{addr}");
    axum::serve(listener, axum_notes_api::app_with(store)).await?;
    Ok(())
}
