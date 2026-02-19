//! auth/ — Magnetic Auth Middleware
//!
//! Passwordless authentication, built once.
//! Supports: OAuth2/OIDC (code flow), magic-link, OTP.
//! No username/password — only modern passwordless flows.
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

    /// Provider type helper.
    pub fn provider(&self) -> &str {
        &self.config.provider
    }

    /// Whether this is an OAuth2/OIDC code flow provider.
    pub fn is_oauth(&self) -> bool {
        matches!(self.provider(), "oidc" | "oauth2")
    }

    /// Whether this is a magic-link provider (token arrives in callback URL).
    pub fn is_magic_link(&self) -> bool {
        self.provider() == "magic-link"
    }

    /// Whether this is an OTP provider (code verified via API).
    pub fn is_otp(&self) -> bool {
        self.provider() == "otp"
    }

    /// Get session cookie name.
    pub fn cookie_name(&self) -> &str {
        self.config.session.as_ref()
            .and_then(|s| s.cookie.as_deref())
            .unwrap_or("magnetic_session")
    }

    /// Configured token field name in verify responses.
    fn token_field(&self) -> &str {
        self.config.token_field.as_deref().unwrap_or("session_token")
    }

    /// Default token expiry if provider doesn't return one.
    fn default_expires(&self) -> u64 {
        self.config.token_expires_in.unwrap_or(3600)
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

        // Check if token is expired
        if session.is_expired() {
            // Only OAuth2/OIDC supports refresh tokens
            if self.is_oauth() {
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
                        }
                    }
                }
            }
            // Token expired and can't refresh — remove session
            self.sessions.remove(&session_id);
            return None;
        }

        Some(session.access_token.clone())
    }

    // ── Login URL generation ─────────────────────────────────────────

    /// Build the login URL. Behavior depends on provider type:
    /// - oauth2/oidc: redirect to authorization endpoint
    /// - magic-link: redirect to a custom login page (app provides UI)
    /// - otp: redirect to a custom login page (app provides UI)
    pub fn login_url(&self, state: &str) -> String {
        match self.provider() {
            "oidc" | "oauth2" => self.oauth_login_url(state),
            // For magic-link and OTP, the developer provides a login page
            // that collects the email and POSTs to /auth/send
            _ => {
                // Return a redirect to a login page the developer builds
                // (or the app root with ?login=true as a hint)
                format!("/?login=true&state={}", urlencoding(state))
            }
        }
    }

    fn oauth_login_url(&self, state: &str) -> String {
        let issuer = self.config.issuer.as_deref().unwrap_or("");
        let client_id = resolve_env(self.config.client_id.as_deref().unwrap_or(""));
        let redirect_uri = self.config.redirect_uri.as_deref().unwrap_or("/auth/callback");
        let scopes = self.config.scopes.join(" ");

        let auth_endpoint = if self.config.provider == "oidc" {
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

    // ── Token exchange (multi-provider) ──────────────────────────────

    /// Exchange code/token for session. Branches by provider type.
    pub fn exchange_code(&self, code: &str) -> Result<(String, Option<String>, u64), String> {
        match self.provider() {
            "oidc" | "oauth2" => {
                let issuer = self.config.issuer.as_deref().unwrap_or("");
                let client_id = resolve_env(self.config.client_id.as_deref().unwrap_or(""));
                let client_secret = resolve_env(self.config.client_secret.as_deref().unwrap_or(""));
                let redirect_uri = self.config.redirect_uri.as_deref().unwrap_or("/auth/callback");
                oauth2::exchange_code(issuer, &self.config.provider, &client_id, &client_secret, redirect_uri, code)
            }
            "magic-link" => {
                // For magic-link, "code" is actually the token from the callback URL
                let verify_url = resolve_env(self.config.verify_url.as_deref().unwrap_or(""));
                if verify_url.is_empty() {
                    return Err("magic-link provider requires verify_url".into());
                }
                oauth2::verify_magic_link_token(
                    &verify_url, code, self.token_field(), self.default_expires(),
                )
            }
            "otp" => {
                Err("OTP provider uses /auth/verify endpoint, not /auth/callback".into())
            }
            other => Err(format!("Unknown auth provider: {}", other)),
        }
    }

    // ── Magic-link / OTP: send step ──────────────────────────────────

    /// Send a magic-link or OTP to the given email address.
    /// Called from POST /auth/send with JSON body { "email": "..." }
    pub fn send_auth_email(&self, email: &str) -> Result<serde_json::Value, String> {
        let login_url = resolve_env(self.config.login_url.as_deref().unwrap_or(""));
        if login_url.is_empty() {
            return Err(format!("{} provider requires login_url", self.provider()));
        }
        let client_id = resolve_env(self.config.client_id.as_deref().unwrap_or(""));
        let client_secret = resolve_env(self.config.client_secret.as_deref().unwrap_or(""));

        oauth2::send_otp(&login_url, email, &client_id, &client_secret)
    }

    /// Verify an OTP code. Called from POST /auth/verify with JSON body { "code": "...", "method_id": "..." }
    pub fn verify_otp_code(&self, code: &str, method_id: &str) -> Result<(String, Option<String>, u64), String> {
        let verify_url = resolve_env(self.config.verify_url.as_deref().unwrap_or(""));
        if verify_url.is_empty() {
            return Err("OTP provider requires verify_url".into());
        }
        let client_id = resolve_env(self.config.client_id.as_deref().unwrap_or(""));
        let client_secret = resolve_env(self.config.client_secret.as_deref().unwrap_or(""));

        oauth2::verify_otp(
            &verify_url, code, method_id,
            &client_id, &client_secret,
            self.token_field(), self.default_expires(),
        )
    }

    /// Refresh an access token using a refresh token (OAuth2/OIDC only).
    fn refresh_token(&self, refresh_token: &str) -> Result<(String, Option<String>, u64), String> {
        let issuer = self.config.issuer.as_deref().unwrap_or("");
        let client_id = resolve_env(self.config.client_id.as_deref().unwrap_or(""));
        let client_secret = resolve_env(self.config.client_secret.as_deref().unwrap_or(""));

        oauth2::refresh_token(issuer, &self.config.provider, &client_id, &client_secret, refresh_token)
    }

    // ── Session management ───────────────────────────────────────────

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
