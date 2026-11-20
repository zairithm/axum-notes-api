//! PostgreSQL note storage with `sqlx`.
//!
//! Queries are checked at run time (not with `query!` macros), so building the
//! project never needs a database. The schema lives in `migrations/` and is applied
//! on connect.

use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::models::{NewNote, Note};

type Row = (i64, String, String);

fn to_note((id, title, body): Row) -> Note {
    Note {
        // ids come from BIGSERIAL, so they are always positive
        id: id as u64,
        title,
        body,
    }
}

/// A cheap-to-clone handle to a connection pool.
#[derive(Debug, Clone)]
pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    /// Connects and runs any pending migrations.
    pub async fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
    }

    /// The underlying pool, e.g. for tests that need to reset the table.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn create(&self, new: NewNote) -> sqlx::Result<Note> {
        sqlx::query_as::<_, Row>(
            "INSERT INTO notes (title, body) VALUES ($1, $2) RETURNING id, title, body",
        )
        .bind(new.title)
        .bind(new.body)
        .fetch_one(&self.pool)
        .await
        .map(to_note)
    }

    pub async fn list(&self) -> sqlx::Result<Vec<Note>> {
        let rows = sqlx::query_as::<_, Row>("SELECT id, title, body FROM notes ORDER BY id")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(to_note).collect())
    }

    pub async fn get(&self, id: u64) -> sqlx::Result<Option<Note>> {
        let Ok(id) = i64::try_from(id) else {
            return Ok(None);
        };
        sqlx::query_as::<_, Row>("SELECT id, title, body FROM notes WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(to_note))
    }

    pub async fn update(&self, id: u64, new: NewNote) -> sqlx::Result<Option<Note>> {
        let Ok(id) = i64::try_from(id) else {
            return Ok(None);
        };
        sqlx::query_as::<_, Row>(
            "UPDATE notes SET title = $2, body = $3 WHERE id = $1 RETURNING id, title, body",
        )
        .bind(id)
        .bind(new.title)
        .bind(new.body)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(to_note))
    }

    pub async fn delete(&self, id: u64) -> sqlx::Result<bool> {
        let Ok(id) = i64::try_from(id) else {
            return Ok(false);
        };
        let result = sqlx::query("DELETE FROM notes WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
