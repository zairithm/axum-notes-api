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

/// Request body for `/signup` and `/login`.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// A registered user as returned by `/signup`. Never includes the password hash.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: u64,
    pub username: String,
}

/// Response body for `/login`.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Token {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
}

impl Credentials {
    /// Usernames are 3 to 32 letters, digits or `_`, and stored in lowercase.
    /// Passwords are 8 to 128 characters.
    pub fn validate_new(self) -> Result<Self, AppError> {
        let username = self.normalized_username();
        let len = username.chars().count();
        if !(3..=32).contains(&len) {
            return Err(AppError::Invalid(
                "username must be 3 to 32 characters".into(),
            ));
        }
        if !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(AppError::Invalid(
                "username may only contain letters, digits and _".into(),
            ));
        }
        let pw_len = self.password.chars().count();
        if !(8..=128).contains(&pw_len) {
            return Err(AppError::Invalid(
                "password must be 8 to 128 characters".into(),
            ));
        }
        Ok(Self {
            username,
            password: self.password,
        })
    }

    /// The username as it is stored: trimmed and lowercase.
    pub fn normalized_username(&self) -> String {
        self.username.trim().to_lowercase()
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

    fn creds(u: &str, p: &str) -> Credentials {
        Credentials {
            username: u.into(),
            password: p.into(),
        }
    }

    #[test]
    fn usernames_are_normalised_and_checked() {
        let c = creds("  Zaid_01 ", "password1").validate_new().unwrap();
        assert_eq!(c.username, "zaid_01");
        assert!(creds("ab", "password1").validate_new().is_err());
        assert!(creds("has space", "password1").validate_new().is_err());
        assert!(creds("dash-name", "password1").validate_new().is_err());
        assert!(creds(&"a".repeat(33), "password1").validate_new().is_err());
    }

    #[test]
    fn short_password_is_rejected() {
        assert!(creds("zaid", "short").validate_new().is_err());
        assert!(creds("zaid", "12345678").validate_new().is_ok());
    }

    #[test]
    fn body_is_optional() {
        let n: NewNote = serde_json::from_str(r#"{"title":"only a title"}"#).unwrap();
        assert_eq!(n.body, "");
    }
}
