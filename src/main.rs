use axum_notes_api::auth::Keys;
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
            println!("DATABASE_URL is not set, so data is kept in memory only");
            Store::default()
        }
    };

    let keys = match std::env::var("JWT_SECRET") {
        Ok(secret) if secret.len() >= 32 => Keys::new(secret.as_bytes()),
        Ok(_) => return Err("JWT_SECRET must be at least 32 characters".into()),
        Err(_) => {
            println!("JWT_SECRET is not set, so login tokens stop working on restart");
            Keys::random()
        }
    };

    let addr = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("listening on http://{addr}");
    axum::serve(listener, axum_notes_api::app_with(store, keys)).await?;
    Ok(())
}
