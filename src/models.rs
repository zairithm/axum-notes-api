use serde::{Deserialize, Serialize};

/// A note as returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub id: u64,
    pub title: String,
    pub body: String,
}

/// Request body for creating or updating a note.
#[derive(Debug, Deserialize)]
pub struct NewNote {
    pub title: String,
    pub body: String,
}
