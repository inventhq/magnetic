//! data.rs — Declarative data layer
//!
//! Parses magnetic config (data sources + action mappings), fetches data from
//! remote APIs, and provides the data context that gets injected into V8 before
//! each render.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

// ── Config types (deserialized from magnetic.json config field) ──────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataLayerConfig {
    #[serde(default)]
    pub auth: Option<AuthConfig>,
    #[serde(default, deserialize_with = "deserialize_data_sources")]
    pub data: Vec<DataSourceConfig>,
    #[serde(default)]
    pub actions: Vec<ActionMappingConfig>,
}

/// Accept data sources as either:
///   - Array: [{"key":"events","url":"...","type":"sse"}]   (CLI serialized)
///   - Map:   {"events":{"url":"...","type":"sse"}}         (raw magnetic.json)
fn deserialize_data_sources<'de, D>(deserializer: D) -> Result<Vec<DataSourceConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DataSourcesFormat {
        Array(Vec<DataSourceConfig>),
        Map(HashMap<String, MapDataSource>),
    }

    #[derive(Deserialize)]
    struct MapDataSource {
        url: String,
        #[serde(default = "default_source_type")]
        #[serde(rename = "type")]
        source_type: String,
        refresh: Option<String>,
        #[serde(default = "default_page")]
        page: String,
        #[serde(default)]
        auth: bool,
        timeout: Option<String>,
        #[serde(default)]
        retries: u32,
        #[serde(default)]
        buffer: usize,
        target: Option<String>,
    }

    match DataSourcesFormat::deserialize(deserializer) {
        Ok(DataSourcesFormat::Array(arr)) => Ok(arr),
        Ok(DataSourcesFormat::Map(map)) => {
            Ok(map.into_iter().map(|(key, src)| DataSourceConfig {
                key,
                url: src.url,
                source_type: src.source_type,
                refresh: src.refresh,
                page: src.page,
                auth: src.auth,
                timeout: src.timeout,
                retries: src.retries,
                buffer: src.buffer,
                target: src.target,
            }).collect())
        }
        Err(e) => Err(e),
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    /// Provider type: "oidc", "oauth2", "magic-link", "otp"
    pub provider: String,
    /// Issuer URL (for oauth2/oidc)
    pub issuer: Option<String>,
    /// OAuth client ID (also used as project ID for Stytch etc.)
    pub client_id: Option<String>,
    /// OAuth client secret
    pub client_secret: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub redirect_uri: Option<String>,
    pub session: Option<SessionConfig>,
    /// Login URL for magic-link / OTP providers (where the initial request goes)
    pub login_url: Option<String>,
    /// Verify/authenticate URL for magic-link / OTP providers
    pub verify_url: Option<String>,
    /// JSON field name containing the session token in verify response (default: "session_token")
    pub token_field: Option<String>,
    /// Token lifetime in seconds if provider doesn't return expires_in (default: 3600)
    pub token_expires_in: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionConfig {
    pub cookie: Option<String>,
    pub ttl: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataSourceConfig {
    pub key: String,
    pub url: String,
    #[serde(default = "default_source_type")]
    #[serde(rename = "type")]
    pub source_type: String,
    pub refresh: Option<String>,
    #[serde(default = "default_page")]
    pub page: String,
    #[serde(default)]
    pub auth: bool,
    /// SSR timeout: if fetch takes longer, render without this data.
    /// Format: "100ms", "1s". Default: no timeout (blocking).
    pub timeout: Option<String>,
    /// Number of retry attempts on fetch failure. Default: 0 (no retries).
    #[serde(default)]
    pub retries: u32,
    /// For SSE/WS sources: keep last N events as a JSON array. Default: 0 (replace mode).
    #[serde(default)]
    pub buffer: usize,
    /// For delta mode: the data-key of the container element to insert into.
    /// When set, SSE events are sent as lightweight deltas instead of full DOM snapshots.
    pub target: Option<String>,
}

fn default_source_type() -> String { "fetch".into() }
fn default_page() -> String { "*".into() }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActionMappingConfig {
    pub name: String,
    pub method: String,
    pub url: String,
    pub target: Option<String>,
    pub debounce: Option<u64>,
}

// ── Data context (fetched data stored per-app) ──────────────────────

pub struct DataContext {
    /// Current fetched data: key → JSON value
    pub values: RwLock<HashMap<String, serde_json::Value>>,
    /// Config for this app's data layer
    pub config: DataLayerConfig,
    /// Last fetch time per data source key
    last_fetch: Mutex<HashMap<String, Instant>>,
}

impl DataContext {
    pub fn new(config: DataLayerConfig) -> Self {
        Self {
            values: RwLock::new(HashMap::new()),
            config,
            last_fetch: Mutex::new(HashMap::new()),
        }
    }

    /// Get data sources that should be active for a given page path.
    pub fn sources_for_page(&self, path: &str) -> Vec<&DataSourceConfig> {
        self.config.data.iter().filter(|d| {
            if d.page == "*" {
                return true;
            }
            // Exact match or prefix match
            if path == d.page {
                return true;
            }
            // Prefix match: "/settings" matches "/settings/billing"
            if path.starts_with(&d.page) && (
                d.page.ends_with('/') || path.as_bytes().get(d.page.len()) == Some(&b'/')
            ) {
                return true;
            }
            false
        }).collect()
    }

    /// Build a merged JSON object of all data values for a given page.
    /// Only includes data sources whose page scope matches.
    pub fn data_json_for_page(&self, path: &str) -> String {
        let sources = self.sources_for_page(path);
        let values = self.values.read().unwrap();
        let mut obj = serde_json::Map::new();
        for src in sources {
            if let Some(val) = values.get(&src.key) {
                obj.insert(src.key.clone(), val.clone());
            }
        }
        serde_json::Value::Object(obj).to_string()
    }

    /// Check if an action name maps to an external API.
    pub fn find_action(&self, action_name: &str) -> Option<&ActionMappingConfig> {
        self.config.actions.iter().find(|a| a.name == action_name)
    }

    /// Store a fetched value for a data source key.
    pub fn set_value(&self, key: &str, value: serde_json::Value) {
        self.values.write().unwrap().insert(key.to_string(), value);
        self.last_fetch.lock().unwrap().insert(key.to_string(), Instant::now());
    }
}

// ── Data fetcher ────────────────────────────────────────────────────

/// Resolve ${env.XXX} placeholders in a string.
fn resolve_env_vars(s: &str) -> String {
    let mut result = s.to_string();
    while let Some(start) = result.find("${env.") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 6..start + end];
            let replacement = std::env::var(var_name).unwrap_or_default();
            result = format!("{}{}{}", &result[..start], replacement, &result[start + end + 1..]);
        } else {
            break;
        }
    }
    result
}

/// Interpolate ${payload.xxx} in a URL template.
pub fn interpolate_url(template: &str, payload: &serde_json::Value) -> String {
    let mut result = template.to_string();
    while let Some(start) = result.find("${payload.") {
        if let Some(end) = result[start..].find('}') {
            let field = &result[start + 10..start + end];
            let replacement = payload.get(field)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            result = format!("{}{}{}", &result[..start], replacement, &result[start + end + 1..]);
        } else {
            break;
        }
    }
    resolve_env_vars(&result)
}

/// Fetch a single data source. Returns the parsed JSON value.
/// If the source has `auth: true` and a token is provided, it's sent as Bearer.
/// Retries up to `source.retries` times with exponential backoff (200ms, 400ms, 800ms...).
pub fn fetch_data_source(source: &DataSourceConfig, auth_token: Option<&str>) -> Result<serde_json::Value, String> {
    let url = resolve_env_vars(&source.url);
    let max_attempts = 1 + source.retries; // 0 retries = 1 attempt
    let mut last_err = String::new();

    for attempt in 0..max_attempts {
        if attempt > 0 {
            let backoff = Duration::from_millis(200 * (1 << (attempt - 1).min(4)));
            eprintln!("[data] retrying '{}' (attempt {}/{}, backoff {:?})", source.key, attempt + 1, max_attempts, backoff);
            thread::sleep(backoff);
        } else {
            eprintln!("[data] fetching '{}' from {}", source.key, url);
        }

        let mut req = ureq::get(&url)
            .set("Accept", "application/json");

        if source.auth {
            if let Some(token) = auth_token {
                req = req.set("Authorization", &format!("Bearer {}", token));
            }
        }

        match req.call() {
            Ok(resp) => {
                match resp.into_string() {
                    Ok(body) => {
                        return serde_json::from_str(&body)
                            .map_err(|e| format!("parse '{}': {}", source.key, e));
                    }
                    Err(e) => { last_err = format!("read '{}': {}", source.key, e); }
                }
            }
            Err(e) => { last_err = format!("fetch '{}': {}", source.key, e); }
        }
    }

    Err(last_err)
}

/// Fetch all data sources matching a page scope.
/// Returns number of sources fetched.
pub fn fetch_page_data(ctx: &DataContext, path: &str) -> usize {
    fetch_page_data_with_token(ctx, path, None)
}

/// Fetch all data sources matching a page scope, with optional auth token.
pub fn fetch_page_data_with_token(ctx: &DataContext, path: &str, auth_token: Option<&str>) -> usize {
    let sources: Vec<DataSourceConfig> = ctx.sources_for_page(path)
        .into_iter()
        .cloned()
        .collect();

    let mut count = 0;
    for source in &sources {
        // SSE sources are handled by start_sse_threads, not regular fetch.
        // Attempting to HTTP GET an SSE endpoint blocks forever (stream never ends).
        if source.source_type == "sse" { continue; }
        match fetch_data_source(&source, auth_token) {
            Ok(value) => {
                ctx.set_value(&source.key, value);
                count += 1;
            }
            Err(e) => {
                eprintln!("[data] error: {}", e);
                ctx.set_value(&source.key, serde_json::json!({
                    "__error": e
                }));
            }
        }
    }
    count
}

/// Fetch data with timeout support for streaming SSR.
/// Sources with a `timeout` value are fetched in parallel threads.
/// If any source exceeds its timeout, the result is returned without it
/// (the source key is set to null + __loading flag), and the timed-out
/// sources are returned in the second Vec for background completion.
pub fn fetch_page_data_streaming(
    ctx: &DataContext,
    path: &str,
    auth_token: Option<&str>,
) -> Vec<DataSourceConfig> {
    let sources: Vec<DataSourceConfig> = ctx.sources_for_page(path)
        .into_iter()
        .cloned()
        .collect();

    let mut pending: Vec<DataSourceConfig> = Vec::new();
    let mut handles: Vec<(DataSourceConfig, std::sync::mpsc::Receiver<Result<serde_json::Value, String>>)> = Vec::new();

    for source in &sources {
        // SSE sources are handled by start_sse_threads, not regular fetch.
        if source.source_type == "sse" { continue; }
        let timeout = source.timeout.as_ref().map(|t| parse_duration(t));
        if let Some(dur) = timeout {
            if !dur.is_zero() {
                // Fetch in background thread with timeout
                let (tx, rx) = std::sync::mpsc::channel();
                let src = source.clone();
                let token = auth_token.map(String::from);
                thread::spawn(move || {
                    let result = fetch_data_source(&src, token.as_deref());
                    let _ = tx.send(result);
                });
                handles.push((source.clone(), rx));
                continue;
            }
        }
        // No timeout — fetch synchronously (blocking)
        match fetch_data_source(source, auth_token) {
            Ok(value) => ctx.set_value(&source.key, value),
            Err(e) => {
                eprintln!("[data] error: {}", e);
                ctx.set_value(&source.key, serde_json::json!({ "__error": e }));
            }
        }
    }

    // Wait for timed-out sources
    for (source, rx) in handles {
        let timeout = parse_duration(source.timeout.as_deref().unwrap_or("100ms"));
        match rx.recv_timeout(timeout) {
            Ok(Ok(value)) => ctx.set_value(&source.key, value),
            Ok(Err(e)) => {
                eprintln!("[data] error: {}", e);
                ctx.set_value(&source.key, serde_json::json!({ "__error": e }));
            }
            Err(_) => {
                // Timeout — mark as loading, add to pending for background completion
                eprintln!("[data] '{}' timed out, rendering with loading state", source.key);
                ctx.set_value(&source.key, serde_json::Value::Null);
                pending.push(source);
            }
        }
    }

    // Set __loading flag if any sources are pending
    if !pending.is_empty() {
        let loading_keys: Vec<String> = pending.iter().map(|s| s.key.clone()).collect();
        ctx.set_value("__loading", serde_json::json!(loading_keys));
    } else {
        // Remove loading flag if it was set from a previous render
        ctx.values.write().unwrap().remove("__loading");
    }

    pending
}

/// Forward an action to an external API endpoint.
/// Returns the API response body (or error).
pub fn forward_action(
    mapping: &ActionMappingConfig,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let url = interpolate_url(&mapping.url, payload);
    eprintln!("[data] forwarding action '{}' → {} {}", mapping.name, mapping.method, url);

    let resp = match mapping.method.as_str() {
        "GET" => ureq::get(&url)
            .set("Accept", "application/json")
            .call(),
        "POST" => ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_string(&payload.to_string()),
        "PUT" => ureq::put(&url)
            .set("Content-Type", "application/json")
            .send_string(&payload.to_string()),
        "PATCH" => ureq::patch(&url)
            .set("Content-Type", "application/json")
            .send_string(&payload.to_string()),
        "DELETE" => ureq::delete(&url)
            .set("Accept", "application/json")
            .call(),
        other => return Err(format!("unsupported method: {}", other)),
    };

    let resp = resp.map_err(|e| format!("action '{}': {}", mapping.name, e))?;
    let body = resp.into_string()
        .map_err(|e| format!("read action response '{}': {}", mapping.name, e))?;

    // If the action has a target, the response replaces that data source
    if body.is_empty() {
        Ok(serde_json::Value::Null)
    } else {
        serde_json::from_str(&body)
            .map_err(|e| format!("parse action response '{}': {}", mapping.name, e))
    }
}

/// Start background poll threads for data sources with refresh intervals.
/// Each poll thread periodically re-fetches and signals when data changes.
pub fn start_poll_threads(
    ctx: Arc<DataContext>,
    on_change: Arc<dyn Fn() + Send + Sync>,
) {
    for source in &ctx.config.data {
        if source.source_type != "poll" {
            continue;
        }
        let interval = parse_duration(&source.refresh.clone().unwrap_or_default());
        if interval.is_zero() {
            continue;
        }

        let source = source.clone();
        let ctx = Arc::clone(&ctx);
        let on_change = Arc::clone(&on_change);

        thread::spawn(move || {
            eprintln!("[data] poll thread started for '{}' (every {:?})", source.key, interval);
            loop {
                thread::sleep(interval);
                match fetch_data_source(&source, None) {
                    Ok(new_value) => {
                        let old = ctx.values.read().unwrap().get(&source.key).cloned();
                        let changed = old.as_ref() != Some(&new_value);
                        ctx.set_value(&source.key, new_value);
                        if changed {
                            eprintln!("[data] '{}' changed, triggering re-render", source.key);
                            on_change();
                        }
                    }
                    Err(e) => eprintln!("[data] poll error: {}", e),
                }
            }
        });
    }
}

/// Start background SSE client threads for data sources with type "sse".
/// Each thread opens a persistent connection, parses text/event-stream frames,
/// updates the DataContext, and calls on_change() when new data arrives.
/// Auto-reconnects on disconnect with exponential backoff (1s → 30s cap).
///
/// If `source.buffer > 0`, events are accumulated in a JSON array (last N).
/// If `source.buffer == 0` (default), each event replaces the previous value.
/// Events with `event: lag` are skipped (server-side lag notifications).
/// Delta event info passed to the on_sse_event callback.
/// Contains everything the platform needs to send a lightweight delta to browsers.
pub struct SseDelta {
    pub key: String,
    pub value: serde_json::Value,
    pub buffer_size: usize,
    pub target: String,
}

pub fn start_sse_threads(
    ctx: Arc<DataContext>,
    on_change: Arc<dyn Fn() + Send + Sync>,
    on_sse_event: Option<Arc<dyn Fn(SseDelta) + Send + Sync>>,
) {
    for source in &ctx.config.data {
        if source.source_type != "sse" {
            continue;
        }

        let source = source.clone();
        let ctx = Arc::clone(&ctx);
        let on_change = Arc::clone(&on_change);
        let on_sse_event = on_sse_event.as_ref().map(Arc::clone);

        thread::spawn(move || {
            let url = resolve_env_vars(&source.url);
            let buffer_size = source.buffer;
            let mut backoff_ms: u64 = 1000;
            let mut last_event_id = String::new();
            // Ring buffer for accumulation mode
            let mut ring: std::collections::VecDeque<serde_json::Value> =
                std::collections::VecDeque::with_capacity(if buffer_size > 0 { buffer_size } else { 0 });

            loop {
                eprintln!("[data:sse] connecting '{}' → {}", source.key, url);

                let mut req = ureq::get(&url)
                    .set("Accept", "text/event-stream")
                    .set("Cache-Control", "no-cache");

                if !last_event_id.is_empty() {
                    req = req.set("Last-Event-ID", &last_event_id);
                }

                match req.call() {
                    Ok(resp) => {
                        backoff_ms = 1000; // reset on successful connect
                        eprintln!("[data:sse] connected '{}' (buffer={})", source.key, buffer_size);

                        let reader = resp.into_reader();
                        let buf = std::io::BufReader::new(reader);
                        use std::io::BufRead;

                        let mut data_buf = String::new();
                        let mut event_type = String::new();

                        for line_result in buf.lines() {
                            match line_result {
                                Ok(line) => {
                                    if line.is_empty() {
                                        // Empty line = end of event, dispatch
                                        if !data_buf.is_empty() {
                                            // Skip lag notifications
                                            if event_type == "lag" {
                                                eprintln!("[data:sse] lag: {}", data_buf);
                                                data_buf.clear();
                                                event_type.clear();
                                                continue;
                                            }

                                            let trimmed = data_buf.trim_end_matches('\n');
                                            let value = match serde_json::from_str::<serde_json::Value>(trimmed) {
                                                Ok(v) => v,
                                                Err(_) => serde_json::Value::String(trimmed.to_string()),
                                            };

                                            // Delta mode: if source has a target and on_sse_event
                                            // is registered, send the raw event directly to browsers
                                            // instead of re-rendering the entire page in V8.
                                            let use_delta = on_sse_event.is_some()
                                                && source.target.is_some()
                                                && buffer_size > 0;

                                            if buffer_size > 0 {
                                                // Buffer mode: accumulate in ring, store as array
                                                if ring.len() >= buffer_size {
                                                    ring.pop_front();
                                                }
                                                ring.push_back(value.clone());
                                                let arr = serde_json::Value::Array(ring.iter().cloned().collect());
                                                ctx.set_value(&source.key, arr);
                                            } else {
                                                // Replace mode: store latest value
                                                ctx.set_value(&source.key, value.clone());
                                            }

                                            // Always notify on_change (debounced V8 re-render).
                                            // This keeps V8 state fresh for new connections.
                                            on_change();

                                            // Also send immediate delta if available.
                                            // Arrives before the debounced snapshot — gives
                                            // instant visual feedback. Keyed reconciliation
                                            // in the browser handles the overlap correctly.
                                            if use_delta {
                                                on_sse_event.as_ref().unwrap()(SseDelta {
                                                    key: source.key.clone(),
                                                    value,
                                                    buffer_size,
                                                    target: source.target.clone().unwrap(),
                                                });
                                            }
                                            data_buf.clear();
                                            event_type.clear();
                                        }
                                    } else if let Some(rest) = line.strip_prefix("data:") {
                                        let rest = rest.strip_prefix(' ').unwrap_or(rest);
                                        if !data_buf.is_empty() {
                                            data_buf.push('\n');
                                        }
                                        data_buf.push_str(rest);
                                    } else if let Some(rest) = line.strip_prefix("id:") {
                                        last_event_id = rest.strip_prefix(' ').unwrap_or(rest).to_string();
                                    } else if let Some(rest) = line.strip_prefix("event:") {
                                        event_type = rest.strip_prefix(' ').unwrap_or(rest).to_string();
                                    } else if line.starts_with(':') {
                                        // Comment / keepalive — ignore
                                    } else if let Some(rest) = line.strip_prefix("retry:") {
                                        if let Ok(ms) = rest.trim().parse::<u64>() {
                                            backoff_ms = ms.max(500).min(30000);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[data:sse] read error '{}': {}", source.key, e);
                                    break;
                                }
                            }
                        }
                        eprintln!("[data:sse] disconnected '{}'", source.key);
                    }
                    Err(e) => {
                        eprintln!("[data:sse] connect error '{}': {}", source.key, e);
                    }
                }

                // Reconnect with backoff
                eprintln!("[data:sse] reconnecting '{}' in {}ms", source.key, backoff_ms);
                thread::sleep(Duration::from_millis(backoff_ms));
                backoff_ms = (backoff_ms * 2).min(30000);
            }
        });
    }
}

/// Parse a duration string like "5s", "10s", "1m", "500ms".
fn parse_duration(s: &str) -> Duration {
    let s = s.trim();
    if s.ends_with("ms") {
        let n: u64 = s.trim_end_matches("ms").parse().unwrap_or(0);
        Duration::from_millis(n)
    } else if s.ends_with('s') {
        let n: u64 = s.trim_end_matches('s').parse().unwrap_or(0);
        Duration::from_secs(n)
    } else if s.ends_with('m') {
        let n: u64 = s.trim_end_matches('m').parse().unwrap_or(0);
        Duration::from_secs(n * 60)
    } else {
        Duration::from_secs(0)
    }
}

/// Parse a config JSON string (from deploy payload or disk).
pub fn parse_config(json: &str) -> Result<DataLayerConfig, String> {
    serde_json::from_str(json).map_err(|e| format!("parse config: {}", e))
}
