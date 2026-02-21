use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::db::User;
use crate::error::AppError;
use crate::server::AppState;

const KEY_PREFIX: &str = "mk_";
const KEY_BYTES: usize = 32;

/// Generate a new API key: mk_<32 bytes base64url>
pub fn generate_api_key() -> String {
    let mut bytes = [0u8; KEY_BYTES];
    rand::thread_rng().fill(&mut bytes);
    let encoded = URL_SAFE_NO_PAD.encode(bytes);
    format!("{}{}", KEY_PREFIX, encoded)
}

/// SHA-256 hash of an API key, hex-encoded. Used for storage lookups.
pub fn hash_key(key: &str) -> String {
    let digest = Sha256::digest(key.as_bytes());
    format!("{:x}", digest)
}

/// Generate a short alphanumeric ID (for app subdomains, user IDs, node IDs).
pub fn generate_id(len: usize) -> String {
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

/// Tier limits
pub struct TierLimits {
    pub max_apps: i64,
    pub max_sse_clients: i64,
    pub max_requests_month: i64,
}

pub fn tier_limits(tier: &str) -> TierLimits {
    match tier {
        "pro" => TierLimits {
            max_apps: 20,
            max_sse_clients: 50,
            max_requests_month: 100_000,
        },
        "scale" => TierLimits {
            max_apps: 1_000,
            max_sse_clients: 500,
            max_requests_month: 1_000_000,
        },
        _ => TierLimits {
            max_apps: 100,
            max_sse_clients: 50,
            max_requests_month: 100_000,
        },
    }
}

/// Axum extractor: validates Bearer token and returns the authenticated user.
pub struct AuthUser(pub User);

impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = AppError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 Arc<AppState>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        let db = state.db.clone();
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        Box::pin(async move {
            let header = auth_header.ok_or(AppError::Unauthorized)?;
            let token = header.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
            if !token.starts_with(KEY_PREFIX) {
                return Err(AppError::Unauthorized);
            }
            let key_hash = hash_key(token);
            let user = db
                .get_user_by_key_hash(&key_hash)
                .await?
                .ok_or(AppError::Unauthorized)?;
            Ok(AuthUser(user))
        })
    }
}
