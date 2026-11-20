//! In-memory note storage, shared by all requests.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use crate::models::{NewNote, Note};

/// Notes keyed by id. `BTreeMap` keeps them ordered, so listing is stable.
#[derive(Debug, Default)]
struct Inner {
    notes: BTreeMap<u64, Note>,
    next_id: u64,
}

/// A cheap-to-clone handle to the shared store. Locks are held only for the
/// length of one map operation, never across an `.await`.
#[derive(Debug, Clone, Default)]
pub struct Store {
    inner: Arc<RwLock<Inner>>,
}

impl Store {
    pub fn create(&self, new: NewNote) -> Note {
        let mut inner = self.inner.write().unwrap();
        inner.next_id += 1;
        let note = Note {
            id: inner.next_id,
            title: new.title,
            body: new.body,
        };
        inner.notes.insert(note.id, note.clone());
        note
    }

    pub fn list(&self) -> Vec<Note> {
        self.inner.read().unwrap().notes.values().cloned().collect()
    }

    pub fn get(&self, id: u64) -> Option<Note> {
        self.inner.read().unwrap().notes.get(&id).cloned()
    }

    /// Replaces the title and body. `None` if there is no such note.
    pub fn update(&self, id: u64, new: NewNote) -> Option<Note> {
        let mut inner = self.inner.write().unwrap();
        let note = inner.notes.get_mut(&id)?;
        note.title = new.title;
        note.body = new.body;
        Some(note.clone())
    }

    /// Returns true if a note was deleted.
    pub fn delete(&self, id: u64) -> bool {
        self.inner.write().unwrap().notes.remove(&id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new(title: &str) -> NewNote {
        NewNote {
            title: title.into(),
            body: String::new(),
        }
    }

    #[test]
    fn ids_increase_and_are_not_reused() {
        let store = Store::default();
        assert_eq!(store.create(new("a")).id, 1);
        assert_eq!(store.create(new("b")).id, 2);
        assert!(store.delete(2));
        assert_eq!(store.create(new("c")).id, 3);
    }

    #[test]
    fn update_and_delete_unknown_ids() {
        let store = Store::default();
        assert!(store.update(9, new("x")).is_none());
        assert!(!store.delete(9));
    }

    #[test]
    fn clones_share_the_same_notes() {
        let store = Store::default();
        let other = store.clone();
        store.create(new("shared"));
        assert_eq!(other.list().len(), 1);
    }
}
