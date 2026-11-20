use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Longest title allowed, in characters.
pub const MAX_TITLE_LEN: usize = 200;
/// Longest body allowed, in characters.
pub const MAX_BODY_LEN: usize = 10_000;

/// A note as returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub id: u64,
    pub title: String,
    pub body: String,
}

/// Request body for creating or updating a note. `body` may be left out.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NewNote {
    pub title: String,
    #[serde(default)]
    pub body: String,
}

impl NewNote {
    /// Trims the title and checks both fields, explaining what is wrong.
    pub fn validate(self) -> Result<Self, AppError> {
        let title = self.title.trim().to_string();
        if title.is_empty() {
            return Err(AppError::Invalid("title must not be empty".into()));
        }
        if title.chars().count() > MAX_TITLE_LEN {
            return Err(AppError::Invalid(format!(
                "title must be at most {MAX_TITLE_LEN} characters"
            )));
        }
        if self.body.chars().count() > MAX_BODY_LEN {
            return Err(AppError::Invalid(format!(
                "body must be at most {MAX_BODY_LEN} characters"
            )));
        }
        Ok(Self {
            title,
            body: self.body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(title: &str, body: &str) -> NewNote {
        NewNote {
            title: title.into(),
            body: body.into(),
        }
    }

    #[test]
    fn trims_the_title() {
        assert_eq!(note("  hi  ", "x").validate().unwrap().title, "hi");
    }

    #[test]
    fn rejects_empty_and_long_fields() {
        assert!(note("   ", "").validate().is_err());
        assert!(note(&"t".repeat(MAX_TITLE_LEN + 1), "").validate().is_err());
        assert!(note(&"t".repeat(MAX_TITLE_LEN), "").validate().is_ok());
        assert!(note("ok", &"b".repeat(MAX_BODY_LEN + 1))
            .validate()
            .is_err());
    }

    #[test]
    fn body_is_optional() {
        let n: NewNote = serde_json::from_str(r#"{"title":"only a title"}"#).unwrap();
        assert_eq!(n.body, "");
    }
}
