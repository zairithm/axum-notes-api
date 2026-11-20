//! PostgreSQL storage with `sqlx`.
//!
//! Queries are checked at run time (not with `query!` macros), so building the
//! project never needs a database. The schema lives in `migrations/` and is applied
//! on connect. Every note query filters on `user_id`, so users only see their own.

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

/// Ids are `u64` in the API but `BIGINT` in the database. Anything above `i64::MAX`
/// can't exist, so callers treat a failed conversion as "not found".
fn db_id(id: u64) -> Option<i64> {
    i64::try_from(id).ok()
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

    /// The underlying pool, e.g. for tests that need to reset the tables.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Registers a user. `Ok(None)` if the username is taken.
    pub async fn create_user(
        &self,
        username: &str,
        password_hash: &str,
    ) -> sqlx::Result<Option<u64>> {
        let result = sqlx::query_scalar::<_, i64>(
            "INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind(username)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await;
        match result {
            Ok(id) => Ok(Some(id as u64)),
            Err(e)
                if e.as_database_error()
                    .is_some_and(|d| d.is_unique_violation()) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// The id and password hash for `username`.
    pub async fn user_by_name(&self, username: &str) -> sqlx::Result<Option<(u64, String)>> {
        sqlx::query_as::<_, (i64, String)>(
            "SELECT id, password_hash FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(|(id, hash)| (id as u64, hash)))
    }

    pub async fn create(&self, owner: u64, new: NewNote) -> sqlx::Result<Note> {
        sqlx::query_as::<_, Row>(
            "INSERT INTO notes (user_id, title, body) VALUES ($1, $2, $3) \
             RETURNING id, title, body",
        )
        .bind(owner as i64)
        .bind(new.title)
        .bind(new.body)
        .fetch_one(&self.pool)
        .await
        .map(to_note)
    }

    pub async fn list(&self, owner: u64) -> sqlx::Result<Vec<Note>> {
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, title, body FROM notes WHERE user_id = $1 ORDER BY id",
        )
        .bind(owner as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(to_note).collect())
    }

    pub async fn get(&self, owner: u64, id: u64) -> sqlx::Result<Option<Note>> {
        let Some(id) = db_id(id) else {
            return Ok(None);
        };
        sqlx::query_as::<_, Row>("SELECT id, title, body FROM notes WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(owner as i64)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(to_note))
    }

    pub async fn update(&self, owner: u64, id: u64, new: NewNote) -> sqlx::Result<Option<Note>> {
        let Some(id) = db_id(id) else {
            return Ok(None);
        };
        sqlx::query_as::<_, Row>(
            "UPDATE notes SET title = $3, body = $4 WHERE id = $1 AND user_id = $2 \
             RETURNING id, title, body",
        )
        .bind(id)
        .bind(owner as i64)
        .bind(new.title)
        .bind(new.body)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(to_note))
    }

    pub async fn delete(&self, owner: u64, id: u64) -> sqlx::Result<bool> {
        let Some(id) = db_id(id) else {
            return Ok(false);
        };
        let result = sqlx::query("DELETE FROM notes WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(owner as i64)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
