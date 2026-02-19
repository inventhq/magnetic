//! session.rs — Server-side session store
//!
//! Sessions are stored in-memory (HashMap). Each session holds:
//! - Access token (for injecting into data source requests)
//! - Refresh token (for automatic token refresh)
//! - Expiry time
//!
//! Sessions are identified by an opaque random ID (never contains tokens).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ── Session data ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct SessionData {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: Instant,
    pub created_at: Instant,
}

impl SessionData {
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.token_expires_at
    }
}

// ── Session store ───────────────────────────────────────────────────

pub struct SessionStore {
    sessions: Mutex<HashMap<String, SessionData>>,
    pub ttl_secs: u64,
}

impl SessionStore {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            ttl_secs,
        }
    }

    /// Create a new session, return the session ID.
    pub fn create(
        &self,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_in_secs: u64,
    ) -> String {
        let session_id = generate_session_id();
        let data = SessionData {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.map(|s| s.to_string()),
            token_expires_at: Instant::now() + Duration::from_secs(expires_in_secs),
            created_at: Instant::now(),
        };
        self.sessions.lock().unwrap().insert(session_id.clone(), data);
        session_id
    }

    /// Get session data by ID. Returns None if not found or session TTL expired.
    pub fn get(&self, session_id: &str) -> Option<SessionData> {
        let sessions = self.sessions.lock().unwrap();
        let data = sessions.get(session_id)?;
        // Check session-level TTL (not token expiry — that's handled by refresh)
        if data.created_at.elapsed() > Duration::from_secs(self.ttl_secs) {
            drop(sessions);
            self.remove(session_id);
            return None;
        }
        Some(data.clone())
    }

    /// Update tokens for an existing session (after refresh).
    pub fn update_tokens(
        &self,
        session_id: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_in_secs: u64,
    ) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(data) = sessions.get_mut(session_id) {
            data.access_token = access_token.to_string();
            if let Some(rt) = refresh_token {
                data.refresh_token = Some(rt.to_string());
            }
            data.token_expires_at = Instant::now() + Duration::from_secs(expires_in_secs);
        }
    }

    /// Remove a session (logout or expired).
    pub fn remove(&self, session_id: &str) {
        self.sessions.lock().unwrap().remove(session_id);
    }

    /// Count active sessions.
    pub fn count(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }

    /// Prune expired sessions (call periodically from reaper).
    pub fn prune(&self) {
        let ttl = Duration::from_secs(self.ttl_secs);
        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|_, data| data.created_at.elapsed() < ttl);
    }
}

/// Generate a cryptographically-ish random session ID.
/// Uses system time + process-level counter for uniqueness.
fn generate_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);

    // FNV-1a hash of time + counter for a 128-bit session ID
    let mut h1: u64 = 0xcbf29ce484222325;
    for b in now.to_le_bytes() {
        h1 ^= b as u64;
        h1 = h1.wrapping_mul(0x100000001b3);
    }
    for b in count.to_le_bytes() {
        h1 ^= b as u64;
        h1 = h1.wrapping_mul(0x100000001b3);
    }

    let mut h2: u64 = 0x84222325cbf29ce4;
    for b in count.to_le_bytes().iter().rev() {
        h2 ^= *b as u64;
        h2 = h2.wrapping_mul(0x1b300000001);
    }
    for b in now.to_le_bytes().iter().rev() {
        h2 ^= *b as u64;
        h2 = h2.wrapping_mul(0x1b300000001);
    }

    format!("{:016x}{:016x}", h1, h2)
}
