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
    #[serde(default)]
    pub data: Vec<DataSourceConfig>,
    #[serde(default)]
    pub actions: Vec<ActionMappingConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub provider: String,
    pub issuer: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub redirect_uri: Option<String>,
    pub session: Option<SessionConfig>,
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
pub fn fetch_data_source(source: &DataSourceConfig, auth_token: Option<&str>) -> Result<serde_json::Value, String> {
    let url = resolve_env_vars(&source.url);
    eprintln!("[data] fetching '{}' from {}", source.key, url);

    let mut req = ureq::get(&url)
        .set("Accept", "application/json");

    if source.auth {
        if let Some(token) = auth_token {
            req = req.set("Authorization", &format!("Bearer {}", token));
        }
    }

    let resp = req.call()
        .map_err(|e| format!("fetch '{}': {}", source.key, e))?;

    let body = resp.into_string()
        .map_err(|e| format!("read '{}': {}", source.key, e))?;

    serde_json::from_str(&body)
        .map_err(|e| format!("parse '{}': {}", source.key, e))
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
