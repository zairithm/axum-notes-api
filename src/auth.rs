//! Passwords, tokens and the `AuthUser` extractor.
//!
//! - Passwords are hashed with Argon2id (random salt per user). Hashing is slow on
//!   purpose, so it runs on a blocking thread, not on the async runtime.
//! - Login returns a JWT (HS256) whose `sub` is the user id and which expires after
//!   [`TOKEN_TTL_SECS`].
//! - Handlers that take an [`AuthUser`] only run for requests with a valid
//!   `Authorization: Bearer <token>` header.

use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// How long a login token stays valid.
pub const TOKEN_TTL_SECS: u64 = 24 * 60 * 60;

/// Signing keys for login tokens, derived from one secret.
pub struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }

    /// Keys from 32 random bytes. Tokens stop working when the process restarts.
    pub fn random() -> Self {
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        Self::new(&secret)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// User id.
    sub: u64,
    /// Expiry, seconds since the Unix epoch.
    exp: u64,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after 1970")
        .as_secs()
}

/// Issues a token for `user_id` that expires after [`TOKEN_TTL_SECS`].
pub fn issue_token(keys: &Keys, user_id: u64) -> String {
    let claims = Claims {
        sub: user_id,
        exp: now_secs() + TOKEN_TTL_SECS,
    };
    encode(&Header::default(), &claims, &keys.encoding).expect("HS256 encoding cannot fail")
}

/// Returns the user id in `token` if it is signed by `keys` and not expired.
pub fn verify_token(keys: &Keys, token: &str) -> Option<u64> {
    decode::<Claims>(token, &keys.decoding, &Validation::default())
        .ok()
        .map(|data| data.claims.sub)
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hashing with default params cannot fail")
        .to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

/// A hash to check against when the username doesn't exist, so a failed login
/// takes about as long whether or not the user is real.
pub fn dummy_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| hash_password("not a real password"))
}

/// The logged-in user, taken from the `Authorization: Bearer <token>` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthUser(pub u64);

impl<S> FromRequestParts<S> for AuthUser
where
    Arc<Keys>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let keys = Arc::<Keys>::from_ref(state);
        parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .and_then(|token| verify_token(&keys, token.trim()))
            .map(AuthUser)
            .ok_or(AppError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_round_trip() {
        let keys = Keys::new(b"secret");
        let token = issue_token(&keys, 7);
        assert_eq!(verify_token(&keys, &token), Some(7));
    }

    #[test]
    fn token_from_other_keys_is_rejected() {
        let token = issue_token(&Keys::new(b"one"), 7);
        assert_eq!(verify_token(&Keys::new(b"two"), &token), None);
        assert_eq!(verify_token(&Keys::new(b"one"), "not.a.token"), None);
    }

    #[test]
    fn expired_token_is_rejected() {
        let keys = Keys::new(b"secret");
        let old = Claims { sub: 1, exp: 1_000 };
        let token = encode(&Header::default(), &old, &keys.encoding).unwrap();
        assert_eq!(verify_token(&keys, &token), None);
    }

    #[test]
    fn password_hash_and_verify() {
        let hash = hash_password("correct horse");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("correct horse", &hash));
        assert!(!verify_password("wrong", &hash));
        assert!(!verify_password("anything", "not a hash"));
        assert_ne!(hash, hash_password("correct horse"), "salt is random");
    }
}
