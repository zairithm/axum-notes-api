//! Where notes are kept: in memory or in PostgreSQL.
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
    pub async fn create(&self, new: NewNote) -> Result<Note, AppError> {
        match self {
            Self::Memory(m) => Ok(m.create(new)),
            Self::Postgres(p) => p.create(new).await.map_err(internal),
        }
    }

    pub async fn list(&self) -> Result<Vec<Note>, AppError> {
        match self {
            Self::Memory(m) => Ok(m.list()),
            Self::Postgres(p) => p.list().await.map_err(internal),
        }
    }

    pub async fn get(&self, id: u64) -> Result<Note, AppError> {
        let found = match self {
            Self::Memory(m) => m.get(id),
            Self::Postgres(p) => p.get(id).await.map_err(internal)?,
        };
        found.ok_or(AppError::NotFound(id))
    }

    pub async fn update(&self, id: u64, new: NewNote) -> Result<Note, AppError> {
        let updated = match self {
            Self::Memory(m) => m.update(id, new),
            Self::Postgres(p) => p.update(id, new).await.map_err(internal)?,
        };
        updated.ok_or(AppError::NotFound(id))
    }

    pub async fn delete(&self, id: u64) -> Result<(), AppError> {
        let deleted = match self {
            Self::Memory(m) => m.delete(id),
            Self::Postgres(p) => p.delete(id).await.map_err(internal)?,
        };
        if deleted {
            Ok(())
        } else {
            Err(AppError::NotFound(id))
        }
    }
}
