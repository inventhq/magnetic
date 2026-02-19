//! auth/ — Magnetic Auth Middleware
//!
//! Production-grade OAuth2/OIDC authentication, built once.
//! Handles login flow, token exchange, session management, and token injection
//! into data source requests.

pub mod session;
pub mod oauth2;

use crate::data::AuthConfig;
use session::{SessionStore, SessionData};
use std::collections::HashMap;

// ── Auth middleware context ──────────────────────────────────────────

pub struct AuthMiddleware {
    pub config: AuthConfig,
    pub sessions: SessionStore,
}

impl AuthMiddleware {
    pub fn new(config: AuthConfig) -> Self {
        let ttl_secs = config.session.as_ref()
            .and_then(|s| s.ttl.as_ref())
            .map(|t| parse_ttl(t))
            .unwrap_or(86400); // 24h default

        Self {
            config,
            sessions: SessionStore::new(ttl_secs),
        }
    }

    /// Get session cookie name.
    pub fn cookie_name(&self) -> &str {
        self.config.session.as_ref()
            .and_then(|s| s.cookie.as_deref())
            .unwrap_or("magnetic_session")
    }

    /// Extract session ID from request cookies.
    pub fn session_from_cookies(&self, headers: &HashMap<String, String>) -> Option<String> {
        let cookie_header = headers.get("cookie")?;
        let name = self.cookie_name();
        for part in cookie_header.split(';') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix(&format!("{}=", name)) {
                return Some(val.to_string());
            }
        }
        None
    }

    /// Get session data for a request (if valid session exists).
    pub fn get_session(&self, headers: &HashMap<String, String>) -> Option<SessionData> {
        let session_id = self.session_from_cookies(headers)?;
        self.sessions.get(&session_id)
    }

    /// Get the access token for a valid session, refreshing if needed.
    pub fn get_access_token(&self, headers: &HashMap<String, String>) -> Option<String> {
        let session_id = self.session_from_cookies(headers)?;
        let session = self.sessions.get(&session_id)?;

        // Check if token is expired and refresh if possible
        if session.is_expired() {
            if let Some(ref refresh_token) = session.refresh_token {
                match self.refresh_token(refresh_token) {
                    Ok((new_access, new_refresh, expires_in)) => {
                        self.sessions.update_tokens(
                            &session_id,
                            &new_access,
                            new_refresh.as_deref(),
                            expires_in,
                        );
                        return Some(new_access);
                    }
                    Err(e) => {
                        eprintln!("[auth] token refresh failed: {}", e);
                        self.sessions.remove(&session_id);
                        return None;
                    }
                }
            } else {
                self.sessions.remove(&session_id);
                return None;
            }
        }

        Some(session.access_token.clone())
    }

    /// Build the OAuth2 authorization URL for login redirect.
    pub fn login_url(&self, state: &str) -> String {
        let issuer = self.config.issuer.as_deref().unwrap_or("");
        let client_id = resolve_env(self.config.client_id.as_deref().unwrap_or(""));
        let redirect_uri = self.config.redirect_uri.as_deref().unwrap_or("/auth/callback");
        let scopes = self.config.scopes.join(" ");

        let auth_endpoint = if self.config.provider == "oidc" {
            // OIDC: discover from .well-known
            oauth2::discover_auth_endpoint(issuer).unwrap_or_else(|_| {
                format!("{}/authorize", issuer)
            })
        } else {
            format!("{}/authorize", issuer)
        };

        format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            auth_endpoint,
            urlencoding(&client_id),
            urlencoding(redirect_uri),
            urlencoding(&scopes),
            urlencoding(state),
        )
    }

    /// Exchange authorization code for tokens.
    pub fn exchange_code(&self, code: &str) -> Result<(String, Option<String>, u64), String> {
        let issuer = self.config.issuer.as_deref().unwrap_or("");
        let client_id = resolve_env(self.config.client_id.as_deref().unwrap_or(""));
        let client_secret = resolve_env(self.config.client_secret.as_deref().unwrap_or(""));
        let redirect_uri = self.config.redirect_uri.as_deref().unwrap_or("/auth/callback");

        oauth2::exchange_code(issuer, &self.config.provider, &client_id, &client_secret, redirect_uri, code)
    }

    /// Refresh an access token using a refresh token.
    fn refresh_token(&self, refresh_token: &str) -> Result<(String, Option<String>, u64), String> {
        let issuer = self.config.issuer.as_deref().unwrap_or("");
        let client_id = resolve_env(self.config.client_id.as_deref().unwrap_or(""));
        let client_secret = resolve_env(self.config.client_secret.as_deref().unwrap_or(""));

        oauth2::refresh_token(issuer, &self.config.provider, &client_id, &client_secret, refresh_token)
    }

    /// Create a new session after successful login.
    pub fn create_session(
        &self,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_in: u64,
    ) -> (String, String) {
        let session_id = self.sessions.create(access_token, refresh_token, expires_in);
        let cookie = format!(
            "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
            self.cookie_name(),
            session_id,
            self.sessions.ttl_secs,
        );
        (session_id, cookie)
    }

    /// Remove session (logout).
    pub fn logout(&self, headers: &HashMap<String, String>) -> String {
        if let Some(session_id) = self.session_from_cookies(headers) {
            self.sessions.remove(&session_id);
        }
        format!(
            "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
            self.cookie_name()
        )
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn parse_ttl(s: &str) -> u64 {
    let s = s.trim();
    if s.ends_with('h') {
        s.trim_end_matches('h').parse::<u64>().unwrap_or(24) * 3600
    } else if s.ends_with('d') {
        s.trim_end_matches('d').parse::<u64>().unwrap_or(1) * 86400
    } else if s.ends_with('m') {
        s.trim_end_matches('m').parse::<u64>().unwrap_or(60) * 60
    } else {
        s.parse::<u64>().unwrap_or(86400)
    }
}

fn resolve_env(s: &str) -> String {
    if s.starts_with("${env.") && s.ends_with('}') {
        let var = &s[6..s.len() - 1];
        std::env::var(var).unwrap_or_default()
    } else {
        s.to_string()
    }
}

fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}
