//! Where users and notes are kept: in memory or in PostgreSQL.
//!
//! An enum rather than a trait object, because the methods are `async` and there
//! are only two backends. Handlers call `Store` and never know which one is in use.

use crate::error::AppError;
use crate::memory::MemoryStore;
use crate::models::{NewNote, Note};
use crate::postgres::PgStore;

#[derive(Debug, Clone)]
pub enum Store {
    Memory(MemoryStore),
    Postgres(PgStore),
}

impl Default for Store {
    fn default() -> Self {
        Self::Memory(MemoryStore::default())
    }
}

/// Database failures are logged and reported as a plain 500, without details.
fn internal(err: sqlx::Error) -> AppError {
    eprintln!("database error: {err}");
    AppError::Internal
}

impl Store {
    /// Registers a user and returns the new id.
    pub async fn create_user(
        &self,
        username: &str,
        password_hash: String,
    ) -> Result<u64, AppError> {
        let id = match self {
            Self::Memory(m) => m.create_user(username, password_hash),
            Self::Postgres(p) => p
                .create_user(username, &password_hash)
                .await
                .map_err(internal)?,
        };
        id.ok_or(AppError::UsernameTaken)
    }

    /// The id and password hash for `username`, if registered.
    pub async fn user_by_name(&self, username: &str) -> Result<Option<(u64, String)>, AppError> {
        match self {
            Self::Memory(m) => Ok(m.user_by_name(username)),
            Self::Postgres(p) => p.user_by_name(username).await.map_err(internal),
        }
    }

    pub async fn create(&self, owner: u64, new: NewNote) -> Result<Note, AppError> {
        match self {
            Self::Memory(m) => Ok(m.create(owner, new)),
            Self::Postgres(p) => p.create(owner, new).await.map_err(internal),
        }
    }

    pub async fn list(&self, owner: u64) -> Result<Vec<Note>, AppError> {
        match self {
            Self::Memory(m) => Ok(m.list(owner)),
            Self::Postgres(p) => p.list(owner).await.map_err(internal),
        }
    }

    pub async fn get(&self, owner: u64, id: u64) -> Result<Note, AppError> {
        let found = match self {
            Self::Memory(m) => m.get(owner, id),
            Self::Postgres(p) => p.get(owner, id).await.map_err(internal)?,
        };
        found.ok_or(AppError::NotFound(id))
    }

    pub async fn update(&self, owner: u64, id: u64, new: NewNote) -> Result<Note, AppError> {
        let updated = match self {
            Self::Memory(m) => m.update(owner, id, new),
            Self::Postgres(p) => p.update(owner, id, new).await.map_err(internal)?,
        };
        updated.ok_or(AppError::NotFound(id))
    }

    pub async fn delete(&self, owner: u64, id: u64) -> Result<(), AppError> {
        let deleted = match self {
            Self::Memory(m) => m.delete(owner, id),
            Self::Postgres(p) => p.delete(owner, id).await.map_err(internal)?,
        };
        if deleted {
            Ok(())
        } else {
            Err(AppError::NotFound(id))
        }
    }
}
