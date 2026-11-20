//! In-memory storage, used when no database is configured (and in tests).

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

use crate::models::{NewNote, Note};

#[derive(Debug)]
struct StoredNote {
    owner: u64,
    note: Note,
}

#[derive(Debug)]
struct StoredUser {
    id: u64,
    password_hash: String,
}

#[derive(Debug, Default)]
struct Inner {
    /// Keyed by id. `BTreeMap` keeps them ordered, so listing is stable.
    notes: BTreeMap<u64, StoredNote>,
    next_note_id: u64,
    /// Keyed by (already lowercased) username.
    users: HashMap<String, StoredUser>,
    next_user_id: u64,
}

/// A cheap-to-clone handle to the shared store. Locks are held only for the
/// length of one map operation, never across an `.await`.
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    inner: Arc<RwLock<Inner>>,
}

impl MemoryStore {
    /// Registers a user. `None` if the username is taken.
    pub fn create_user(&self, username: &str, password_hash: String) -> Option<u64> {
        let mut inner = self.inner.write().unwrap();
        if inner.users.contains_key(username) {
            return None;
        }
        inner.next_user_id += 1;
        let id = inner.next_user_id;
        inner
            .users
            .insert(username.to_string(), StoredUser { id, password_hash });
        Some(id)
    }

    /// The id and password hash for `username`.
    pub fn user_by_name(&self, username: &str) -> Option<(u64, String)> {
        let inner = self.inner.read().unwrap();
        inner
            .users
            .get(username)
            .map(|u| (u.id, u.password_hash.clone()))
    }

    pub fn create(&self, owner: u64, new: NewNote) -> Note {
        let mut inner = self.inner.write().unwrap();
        inner.next_note_id += 1;
        let note = Note {
            id: inner.next_note_id,
            title: new.title,
            body: new.body,
        };
        inner.notes.insert(
            note.id,
            StoredNote {
                owner,
                note: note.clone(),
            },
        );
        note
    }

    pub fn list(&self, owner: u64) -> Vec<Note> {
        let inner = self.inner.read().unwrap();
        inner
            .notes
            .values()
            .filter(|s| s.owner == owner)
            .map(|s| s.note.clone())
            .collect()
    }

    pub fn get(&self, owner: u64, id: u64) -> Option<Note> {
        let inner = self.inner.read().unwrap();
        inner
            .notes
            .get(&id)
            .filter(|s| s.owner == owner)
            .map(|s| s.note.clone())
    }

    /// Replaces the title and body. `None` if there is no such note for this owner.
    pub fn update(&self, owner: u64, id: u64, new: NewNote) -> Option<Note> {
        let mut inner = self.inner.write().unwrap();
        let stored = inner.notes.get_mut(&id).filter(|s| s.owner == owner)?;
        stored.note.title = new.title;
        stored.note.body = new.body;
        Some(stored.note.clone())
    }

    /// Returns true if a note was deleted.
    pub fn delete(&self, owner: u64, id: u64) -> bool {
        let mut inner = self.inner.write().unwrap();
        if inner.notes.get(&id).is_some_and(|s| s.owner == owner) {
            inner.notes.remove(&id);
            true
        } else {
            false
        }
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
        let store = MemoryStore::default();
        assert_eq!(store.create(1, new("a")).id, 1);
        assert_eq!(store.create(1, new("b")).id, 2);
        assert!(store.delete(1, 2));
        assert_eq!(store.create(1, new("c")).id, 3);
    }

    #[test]
    fn notes_are_private_to_their_owner() {
        let store = MemoryStore::default();
        let note = store.create(1, new("mine"));
        assert_eq!(store.list(2), vec![]);
        assert_eq!(store.get(2, note.id), None);
        assert_eq!(store.update(2, note.id, new("hacked")), None);
        assert!(!store.delete(2, note.id));
        assert_eq!(store.get(1, note.id).unwrap().title, "mine");
    }

    #[test]
    fn usernames_are_unique() {
        let store = MemoryStore::default();
        assert_eq!(store.create_user("zaid", "h1".into()), Some(1));
        assert_eq!(store.create_user("zaid", "h2".into()), None);
        assert_eq!(store.user_by_name("zaid"), Some((1, "h1".into())));
        assert_eq!(store.user_by_name("nobody"), None);
    }

    #[test]
    fn clones_share_the_same_data() {
        let store = MemoryStore::default();
        let other = store.clone();
        store.create(1, new("shared"));
        assert_eq!(other.list(1).len(), 1);
    }
}
