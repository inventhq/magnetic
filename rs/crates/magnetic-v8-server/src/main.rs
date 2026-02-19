//! magnetic-v8-server — Rust HTTP/SSE server with embedded V8
//!
//! Feature parity with TypeScript server:
//!   - Pluggable middleware chain (logger, CORS, rate-limit)
//!   - Error boundaries (V8 TryCatch, fallback DomNode)
//!   - Asset pipeline (content-hashing, immutable cache headers, manifest)
//!   - Head/meta extraction from DomNode
//!   - SSR, SSE, POST actions, static files, navigation
//!
//! Usage:
//!   magnetic-v8-server --bundle dist/app.js --port 3003 --static public/
//!   magnetic-v8-server --bundle dist/app.js --render kotlin --out app.kt
//!   magnetic-v8-server --platform --port 3003 --data-dir data/apps

mod platform;
pub mod data;
pub mod auth;

use magnetic_dom::DomNode;
use magnetic_render_html::{render_to_html, render_page, PageOptions};
use magnetic_render_kotlin::render_to_kotlin;
use magnetic_render_swift::render_to_swift;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// ═══════════════════════════════════════════════════════════════════
// 0. EMBEDDED FRAMEWORK ASSETS
// ═══════════════════════════════════════════════════════════════════

/// Client runtime — embedded at compile time. Never exists as a user-visible file.
const EMBEDDED_MAGNETIC_JS: &[u8] = include_bytes!("../assets/magnetic.min.js");

/// WASM transport — embedded at compile time. Never exists as a user-visible file.
const EMBEDDED_TRANSPORT_WASM: &[u8] = include_bytes!("../assets/transport.wasm");

/// Serve an embedded asset with proper headers. Returns true if handled.
pub fn serve_embedded(
    stream: &mut TcpStream,
    filename: &str,
    extra_headers: &HashMap<String, String>,
) -> Option<std::io::Result<()>> {
    let (data, content_type): (&[u8], &str) = match filename {
        "magnetic.js" => (EMBEDDED_MAGNETIC_JS, "application/javascript"),
        "transport.wasm" => (EMBEDDED_TRANSPORT_WASM, "application/wasm"),
        _ => return None,
    };

    let eh = format_extra_headers(extra_headers);
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
        Cache-Control: public, max-age=31536000, immutable\r\n{}\r\n",
        content_type, data.len(), eh
    );
    Some((|| {
        stream.write_all(resp.as_bytes())?;
        stream.write_all(data)
    })())
}

// ═══════════════════════════════════════════════════════════════════
// 1. MIDDLEWARE
// ═══════════════════════════════════════════════════════════════════

/// HTTP context flowing through the middleware chain
pub struct MagneticContext {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub action: Option<String>,
    pub payload: Option<String>,
    pub status: u16,
    pub response_headers: HashMap<String, String>,
    pub body: Option<String>,
    pub state: HashMap<String, String>,
    pub start_time: Instant,
}

impl MagneticContext {
    pub fn from_request(method: &str, url: &str, headers: HashMap<String, String>) -> Self {
        let (path, qs) = url.split_once('?').unwrap_or((url, ""));
        let mut query = HashMap::new();
        if !qs.is_empty() {
            for pair in qs.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    query.insert(urlencoding_decode(k), urlencoding_decode(v));
                }
            }
        }
        MagneticContext {
            method: method.to_string(),
            path: path.to_string(),
            query,
            headers,
            action: None,
            payload: None,
            status: 200,
            response_headers: HashMap::new(),
            body: None,
            state: HashMap::new(),
            start_time: Instant::now(),
        }
    }
}

pub type MiddlewareFn = Box<dyn Fn(&mut MagneticContext) + Send + Sync>;

pub struct MiddlewareStack {
    fns: Vec<MiddlewareFn>,
}

impl MiddlewareStack {
    pub fn new() -> Self { Self { fns: Vec::new() } }

    pub fn add(&mut self, f: MiddlewareFn) { self.fns.push(f); }

    pub fn run(&self, ctx: &mut MagneticContext) {
        for f in &self.fns {
            f(ctx);
            if ctx.body.is_some() { return; } // short-circuit
        }
    }
}

/// Logger middleware — logs method + path + status + timing
pub fn logger_middleware() -> MiddlewareFn {
    Box::new(|_ctx: &mut MagneticContext| {
        // Pre-hook: start_time is set in MagneticContext::from_request.
        // Actual log line emitted after handler completes in handle_connection.
    })
}

/// CORS middleware — sets Access-Control-Allow-* headers
pub fn cors_middleware(origins: &str) -> MiddlewareFn {
    let origin = origins.to_string();
    Box::new(move |ctx: &mut MagneticContext| {
        ctx.response_headers.insert(
            "Access-Control-Allow-Origin".into(), origin.clone(),
        );
        ctx.response_headers.insert(
            "Access-Control-Allow-Headers".into(), "Content-Type".into(),
        );
        ctx.response_headers.insert(
            "Access-Control-Allow-Methods".into(), "GET, POST, OPTIONS".into(),
        );
        if ctx.method == "OPTIONS" {
            ctx.status = 204;
            ctx.body = Some(String::new());
        }
    })
}

/// Rate-limit middleware — per-IP sliding window
pub fn rate_limit_middleware(window_ms: u64, max_requests: u32) -> MiddlewareFn {
    let hits: Arc<Mutex<HashMap<String, (u32, u64)>>> = Arc::new(Mutex::new(HashMap::new()));
    Box::new(move |ctx: &mut MagneticContext| {
        let ip = ctx.headers.get("x-forwarded-for")
            .or_else(|| ctx.headers.get("x-real-ip"))
            .cloned()
            .unwrap_or_else(|| "unknown".into());

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut map = hits.lock().unwrap();
        let entry = map.entry(ip).or_insert((0, now + window_ms));

        if now > entry.1 {
            *entry = (0, now + window_ms);
        }
        entry.0 += 1;

        if entry.0 > max_requests {
            ctx.status = 429;
            ctx.body = Some("{\"error\":\"Too many requests\"}".into());
        }
    })
}

// ═══════════════════════════════════════════════════════════════════
// 2. ASSET PIPELINE
// ═══════════════════════════════════════════════════════════════════

/// Asset manifest: original filename → hashed filename
pub struct AssetManifest {
    pub files: HashMap<String, String>,    // original → hashed
    pub reverse: HashMap<String, String>,  // hashed → original
}

impl AssetManifest {
    pub fn new() -> Self {
        AssetManifest { files: HashMap::new(), reverse: HashMap::new() }
    }
}

/// Build content-hashed asset manifest from a source directory.
/// Copies files to out_dir with hashed names. Returns manifest.
pub fn build_assets(src_dir: &str, out_dir: &str, passthrough: &[&str]) -> AssetManifest {
    let mut manifest = AssetManifest::new();
    let hash_exts = [".css", ".js", ".wasm"];

    let src = std::path::Path::new(src_dir);
    let out = std::path::Path::new(out_dir);
    if !src.exists() { return manifest; }
    if !out.exists() { let _ = std::fs::create_dir_all(out); }

    let entries = match std::fs::read_dir(src) {
        Ok(e) => e,
        Err(_) => return manifest,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) { continue; }

        let src_path = entry.path();

        // Passthrough files — copy without hashing
        if passthrough.contains(&name.as_str()) {
            let _ = std::fs::copy(&src_path, out.join(&name));
            manifest.files.insert(name.clone(), name.clone());
            manifest.reverse.insert(name.clone(), name);
            continue;
        }

        let ext = std::path::Path::new(&name)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();

        if !hash_exts.contains(&ext.as_str()) {
            // Non-hashable — copy as-is
            let _ = std::fs::copy(&src_path, out.join(&name));
            manifest.files.insert(name.clone(), name.clone());
            manifest.reverse.insert(name.clone(), name);
            continue;
        }

        // Read file, compute MD5 hash (first 8 hex chars)
        let content = match std::fs::read(&src_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let hash = md5_hex(&content);
        let stem = std::path::Path::new(&name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let hashed_name = format!("{}.{}{}", stem, &hash[..8], ext);

        let _ = std::fs::copy(&src_path, out.join(&hashed_name));
        manifest.files.insert(name.clone(), hashed_name.clone());
        manifest.reverse.insert(hashed_name, name);
    }

    manifest
}

/// Simple MD5 implementation (sufficient for content hashing)
fn md5_hex(data: &[u8]) -> String {
    // Use a simple hash: FNV-1a 128-bit split into hex
    // For production parity we want deterministic content hashing.
    // We'll use a basic approach: sum bytes with mixing.
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let mut h2: u64 = 0x84222325cbf29ce4;
    for &b in data.iter().rev() {
        h2 ^= b as u64;
        h2 = h2.wrapping_mul(0x1b3_0000_0001);
    }
    format!("{:016x}{:016x}", h, h2)
}

// ═══════════════════════════════════════════════════════════════════
// 3. ERROR BOUNDARIES (V8 TryCatch)
// ═══════════════════════════════════════════════════════════════════

/// Result of a V8 call: either JSON string or an error message
pub enum V8Result {
    Ok(String),
    Err(String),
}

/// Default fallback DomNode when render fails
pub fn error_fallback(error_msg: &str, action: Option<&str>) -> DomNode {
    let mut children = vec![
        DomNode::text("h2", "Something went wrong"),
        DomNode::text("p", error_msg),
    ];
    if let Some(act) = action {
        children.push(DomNode::text("p", &format!("Action: {}", act)));
    }
    DomNode {
        tag: "div".into(),
        key: Some("error-boundary".into()),
        attrs: Some(HashMap::from([("class".into(), "magnetic-error".into())])),
        events: None,
        text: None,
        children: Some(children),
    }
}

// ═══════════════════════════════════════════════════════════════════
// 4. V8 THREAD (with TryCatch error boundaries)
// ═══════════════════════════════════════════════════════════════════

pub enum V8Request {
    Render { path: String, reply: Arc<Reply> },
    Reduce { action: String, payload: String, path: String, reply: Arc<Reply> },
    /// Inject data context into V8 (calls MagneticApp.setData(json))
    SetData { json: String, reply: Arc<Reply> },
    /// Inject data then render (combined for atomicity)
    RenderWithData { path: String, data_json: String, reply: Arc<Reply> },
    /// Call an API route handler (server/api/*.ts)
    ApiCall { method: String, path: String, body: String, reply: Arc<Reply> },
    /// Call renderWithCSS(path) — returns {root: DomNode, css: string}
    /// Falls back to render(path) if renderWithCSS is not exported
    RenderWithCSS { path: String, reply: Arc<Reply> },
    /// Inject data then call renderWithCSS (combined for SSR with data)
    RenderWithDataAndCSS { path: String, data_json: String, reply: Arc<Reply> },
}

pub struct Reply {
    pub data: Mutex<Option<V8Result>>,
    pub ready: Condvar,
}

impl Reply {
    pub fn new() -> Arc<Self> {
        Arc::new(Reply {
            data: Mutex::new(None),
            ready: Condvar::new(),
        })
    }

    pub fn send(&self, value: V8Result) {
        *self.data.lock().unwrap() = Some(value);
        self.ready.notify_one();
    }

    pub fn recv(&self) -> V8Result {
        let mut guard = self.data.lock().unwrap();
        while guard.is_none() {
            guard = self.ready.wait(guard).unwrap();
        }
        guard.take().unwrap()
    }
}

pub fn v8_thread(js_source: String, rx: mpsc::Receiver<V8Request>) {
    use std::sync::Once;
    static V8_INIT: Once = Once::new();
    V8_INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });

    let mut isolate = v8::Isolate::new(v8::CreateParams::default());

    let global_context;
    {
        let handle_scope = &mut v8::HandleScope::new(&mut isolate);
        let context = v8::Context::new(handle_scope, Default::default());
        global_context = v8::Global::new(handle_scope, context);
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let code = v8::String::new(scope, &js_source).unwrap();
        let script = v8::Script::compile(scope, code, None)
            .expect("Failed to compile JS bundle");
        script.run(scope).expect("Failed to execute JS bundle");
    }

    eprintln!("[magnetic-v8] V8 runtime initialized");

    for req in rx {
        match req {
            V8Request::Render { path, reply } => {
                let result = v8_call_render(&mut isolate, &global_context, &path);
                reply.send(result);
            }
            V8Request::Reduce { action, payload, path, reply } => {
                let reduce_result = v8_call_reduce(
                    &mut isolate, &global_context, &action, &payload,
                );
                if let V8Result::Err(e) = reduce_result {
                    eprintln!("[magnetic-v8] reduce error on \"{}\": {}", action, e);
                }
                let result = v8_call_render(&mut isolate, &global_context, &path);
                reply.send(result);
            }
            V8Request::SetData { json, reply } => {
                let result = v8_call_set_data(&mut isolate, &global_context, &json);
                reply.send(result);
            }
            V8Request::RenderWithData { path, data_json, reply } => {
                // Inject data context first, then render
                let set_result = v8_call_set_data(&mut isolate, &global_context, &data_json);
                if let V8Result::Err(e) = set_result {
                    eprintln!("[magnetic-v8] setData error: {}", e);
                }
                let result = v8_call_render(&mut isolate, &global_context, &path);
                reply.send(result);
            }
            V8Request::ApiCall { method, path, body, reply } => {
                let result = v8_call_api(&mut isolate, &global_context, &method, &path, &body);
                reply.send(result);
            }
            V8Request::RenderWithCSS { path, reply } => {
                let result = v8_call_render_with_css(&mut isolate, &global_context, &path);
                reply.send(result);
            }
            V8Request::RenderWithDataAndCSS { path, data_json, reply } => {
                let set_result = v8_call_set_data(&mut isolate, &global_context, &data_json);
                if let V8Result::Err(e) = set_result {
                    eprintln!("[magnetic-v8] setData error: {}", e);
                }
                let result = v8_call_render_with_css(&mut isolate, &global_context, &path);
                reply.send(result);
            }
        }
    }
}

/// Call renderWithCSS(path) — returns JSON string of {root: DomNode, css: string}
/// Falls back to render(path) wrapped as {root: DomNode} if renderWithCSS is not available
fn v8_call_render_with_css(
    isolate: &mut v8::OwnedIsolate,
    context: &v8::Global<v8::Context>,
    path: &str,
) -> V8Result {
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Local::new(handle_scope, context);
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    // Try renderWithCSS first, fall back to render if not available
    let call_code = format!(
        r#"(function() {{ try {{ if (typeof globalThis.MagneticApp.renderWithCSS === 'function') {{ return JSON.stringify(globalThis.MagneticApp.renderWithCSS("{0}")); }} else {{ var dom = globalThis.MagneticApp.render("{0}"); return JSON.stringify({{root: dom, css: null}}); }} }} catch(e) {{ return JSON.stringify({{__error: e.message || String(e)}}); }} }})()"#,
        path.replace('\\', "\\\\").replace('"', "\\\"")
    );

    let code = v8::String::new(scope, &call_code).unwrap();
    let script = match v8::Script::compile(scope, code, None) {
        Some(s) => s,
        None => return V8Result::Err("Failed to compile renderWithCSS call".into()),
    };
    match script.run(scope) {
        Some(result) => {
            let json = result.to_rust_string_lossy(scope);
            if json.contains("\"__error\"") {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json) {
                    if let Some(msg) = val.get("__error").and_then(|v| v.as_str()) {
                        return V8Result::Err(msg.to_string());
                    }
                }
            }
            V8Result::Ok(json)
        }
        None => V8Result::Err("renderWithCSS() returned undefined".into()),
    }
}

/// Call render(path) with TryCatch error boundary
fn v8_call_render(
    isolate: &mut v8::OwnedIsolate,
    context: &v8::Global<v8::Context>,
    path: &str,
) -> V8Result {
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Local::new(handle_scope, context);
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    let call_code = format!(
        r#"(function() {{ try {{ return JSON.stringify(globalThis.MagneticApp.render("{}")); }} catch(e) {{ return JSON.stringify({{__error: e.message || String(e)}}); }} }})()"#,
        path.replace('\\', "\\\\").replace('"', "\\\"")
    );

    let code = v8::String::new(scope, &call_code).unwrap();
    let script = match v8::Script::compile(scope, code, None) {
        Some(s) => s,
        None => return V8Result::Err("Failed to compile render call".into()),
    };
    match script.run(scope) {
        Some(result) => {
            let json = result.to_rust_string_lossy(scope);
            // Check for error marker
            if json.contains("\"__error\"") {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json) {
                    if let Some(msg) = val.get("__error").and_then(|v| v.as_str()) {
                        return V8Result::Err(msg.to_string());
                    }
                }
            }
            V8Result::Ok(json)
        }
        None => V8Result::Err("render() returned undefined".into()),
    }
}

/// Call setData(json) — inject fetched data context before render
pub fn v8_call_set_data(
    isolate: &mut v8::OwnedIsolate,
    context: &v8::Global<v8::Context>,
    data_json: &str,
) -> V8Result {
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Local::new(handle_scope, context);
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    // setData is optional — apps without data config won't have it
    let call_code = format!(
        r#"(function() {{ try {{ if (globalThis.MagneticApp && globalThis.MagneticApp.setData) {{ globalThis.MagneticApp.setData(JSON.parse('{}')); }} return "ok"; }} catch(e) {{ return JSON.stringify({{__error: e.message || String(e)}}); }} }})()"#,
        data_json.replace('\\', "\\\\").replace('\'', "\\'")
    );

    let code = v8::String::new(scope, &call_code).unwrap();
    let script = match v8::Script::compile(scope, code, None) {
        Some(s) => s,
        None => return V8Result::Err("Failed to compile setData call".into()),
    };
    match script.run(scope) {
        Some(result) => {
            let out = result.to_rust_string_lossy(scope);
            if out.contains("\"__error\"") {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&out) {
                    if let Some(msg) = val.get("__error").and_then(|v| v.as_str()) {
                        return V8Result::Err(msg.to_string());
                    }
                }
            }
            V8Result::Ok(out)
        }
        None => V8Result::Err("setData() returned undefined".into()),
    }
}

/// Call reduce(action, payload) with TryCatch error boundary
fn v8_call_reduce(
    isolate: &mut v8::OwnedIsolate,
    context: &v8::Global<v8::Context>,
    action: &str,
    payload: &str,
) -> V8Result {
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Local::new(handle_scope, context);
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    let inner_json = format!(
        r#"{{"action":"{}","payload":{}}}"#,
        action.replace('\\', "\\\\").replace('"', "\\\""),
        payload
    ).replace('\'', "\\'");

    let call_code = format!(
        r#"(function() {{ try {{ globalThis.MagneticApp.reduce(JSON.parse('{}')); return "ok"; }} catch(e) {{ return JSON.stringify({{__error: e.message || String(e)}}); }} }})()"#,
        inner_json
    );

    let code = v8::String::new(scope, &call_code).unwrap();
    let script = match v8::Script::compile(scope, code, None) {
        Some(s) => s,
        None => return V8Result::Err("Failed to compile reduce call".into()),
    };
    match script.run(scope) {
        Some(result) => {
            let out = result.to_rust_string_lossy(scope);
            if out.contains("\"__error\"") {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&out) {
                    if let Some(msg) = val.get("__error").and_then(|v| v.as_str()) {
                        return V8Result::Err(msg.to_string());
                    }
                }
            }
            V8Result::Ok(out)
        }
        None => V8Result::Err("reduce() returned undefined".into()),
    }
}

/// Call handleApi(method, path, body) — API route handler in V8
fn v8_call_api(
    isolate: &mut v8::OwnedIsolate,
    context: &v8::Global<v8::Context>,
    method: &str,
    path: &str,
    body: &str,
) -> V8Result {
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Local::new(handle_scope, context);
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    let safe_method = method.replace('\\', "\\\\").replace('"', "\\\"");
    let safe_path = path.replace('\\', "\\\\").replace('"', "\\\"");
    let safe_body = body.replace('\\', "\\\\").replace('\'', "\\'");

    let call_code = format!(
        r#"(function() {{ try {{ if (!globalThis.MagneticApp || !globalThis.MagneticApp.handleApi) return JSON.stringify({{__error:"No API routes defined",__status:404}}); return globalThis.MagneticApp.handleApi("{}", "{}", '{}'); }} catch(e) {{ return JSON.stringify({{__error: e.message || String(e), __status: 500}}); }} }})()"#,
        safe_method, safe_path, safe_body
    );

    let code = v8::String::new(scope, &call_code).unwrap();
    let script = match v8::Script::compile(scope, code, None) {
        Some(s) => s,
        None => return V8Result::Err("Failed to compile handleApi call".into()),
    };
    match script.run(scope) {
        Some(result) => V8Result::Ok(result.to_rust_string_lossy(scope)),
        None => V8Result::Err("handleApi() returned undefined".into()),
    }
}

// ═══════════════════════════════════════════════════════════════════
// 5. SERVER STATE
// ═══════════════════════════════════════════════════════════════════

struct Server {
    v8_tx: mpsc::Sender<V8Request>,
    sse_clients: Mutex<Vec<TcpStream>>,
    static_dir: String,
    asset_dir: String,
    current_path: Mutex<String>,
    inline_css: Option<String>,
    middleware: MiddlewareStack,
    manifest: AssetManifest,
}

// ═══════════════════════════════════════════════════════════════════
// 6. MAIN
// ═══════════════════════════════════════════════════════════════════

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Platform mode: multi-tenant hosting
    if args.iter().any(|a| a == "--platform") {
        platform::run_platform(&args);
        return;
    }

    let bundle_path = find_arg(&args, "--bundle").expect("--bundle <path.js> required");
    let port = find_arg(&args, "--port").unwrap_or_else(|| "3003".to_string());
    let static_dir = find_arg(&args, "--static").unwrap_or_else(|| "public".to_string());
    let render_mode = find_arg(&args, "--render");
    let out_path = find_arg(&args, "--out");
    let cors_origin = find_arg(&args, "--cors").unwrap_or_else(|| "*".to_string());
    let rate_limit_max: u32 = find_arg(&args, "--rate-limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let js_source = std::fs::read_to_string(&bundle_path)
        .unwrap_or_else(|e| panic!("Cannot read bundle {}: {}", bundle_path, e));

    // Code generation mode (single-shot, no server)
    if let Some(mode) = &render_mode {
        let (tx, rx) = mpsc::channel();
        let js = js_source.clone();
        thread::spawn(move || v8_thread(js, rx));

        let reply = Reply::new();
        tx.send(V8Request::Render { path: "/".into(), reply: reply.clone() }).unwrap();
        let dom_json = match reply.recv() {
            V8Result::Ok(j) => j,
            V8Result::Err(e) => panic!("render() error: {}", e),
        };

        let dom: DomNode = serde_json::from_str(&dom_json)
            .unwrap_or_else(|e| panic!("Failed to parse DomNode: {}", e));

        let output = match mode.as_str() {
            "kotlin" => render_to_kotlin(&dom, "MagneticApp"),
            "swift" => render_to_swift(&dom, "MagneticAppView"),
            "html" => render_to_html(&dom),
            _ => panic!("Unknown render mode: {}. Use: html, kotlin, swift", mode),
        };

        if let Some(path) = &out_path {
            std::fs::write(path, &output)
                .unwrap_or_else(|e| panic!("Cannot write {}: {}", path, e));
            eprintln!("[magnetic-v8] Wrote {} ({} bytes)", path, output.len());
        } else {
            print!("{}", output);
        }
        return;
    }

    // Start V8 thread
    let (tx, rx) = mpsc::channel();
    let js = js_source;
    thread::spawn(move || v8_thread(js, rx));

    // Build asset pipeline
    let asset_dir = format!("{}/.hashed", static_dir);
    let manifest = build_assets(
        &static_dir, &asset_dir,
        &["index.html"],
    );
    eprintln!("[magnetic-v8] Asset pipeline: {} files hashed", manifest.files.len());
    for (orig, hashed) in &manifest.files {
        if orig != hashed {
            eprintln!("  {} → {}", orig, hashed);
        }
    }

    // Load inline CSS (use hashed path if available)
    let css_hashed = manifest.files.get("style.css").cloned();
    let css_path = if let Some(ref h) = css_hashed {
        format!("{}/{}", asset_dir, h)
    } else {
        format!("{}/style.css", static_dir)
    };
    let inline_css = std::fs::read_to_string(&css_path).ok();

    // Build middleware stack
    let mut middleware = MiddlewareStack::new();
    middleware.add(logger_middleware());
    middleware.add(cors_middleware(&cors_origin));
    middleware.add(rate_limit_middleware(60_000, rate_limit_max));

    let server = Arc::new(Server {
        v8_tx: tx,
        sse_clients: Mutex::new(Vec::new()),
        static_dir: static_dir.clone(),
        asset_dir,
        current_path: Mutex::new("/".to_string()),
        inline_css,
        middleware,
        manifest,
    });

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    eprintln!("[magnetic-v8] http://localhost:{}", port);
    eprintln!("[magnetic-v8] Rust HTTP/SSE + V8 TSX rendering");
    eprintln!("[magnetic-v8] Bundle: {}", bundle_path);
    eprintln!("[magnetic-v8] Middleware: logger, cors({}), rate-limit({}/min)", cors_origin, rate_limit_max);

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => { eprintln!("[err] accept: {}", e); continue; }
        };
        let server = Arc::clone(&server);
        thread::spawn(move || {
            if let Err(e) = handle_connection(stream, &server) {
                let _ = e;
            }
        });
    }
}

pub fn find_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}

// ═══════════════════════════════════════════════════════════════════
// 7. HTTP HANDLER
// ═══════════════════════════════════════════════════════════════════

fn handle_connection(mut stream: TcpStream, server: &Server) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 { return Ok(()); }
    let method = parts[0];
    let path = parts[1];

    // Read headers
    let mut raw_headers = HashMap::new();
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let trimmed = line.trim();
        if trimmed.is_empty() { break; }
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim().to_lowercase();
            let val = v.trim().to_string();
            if key == "content-length" {
                content_length = val.parse().unwrap_or(0);
            }
            raw_headers.insert(key, val);
        }
    }

    // Run middleware
    let mut ctx = MagneticContext::from_request(method, path, raw_headers);

    server.middleware.run(&mut ctx);

    // Log request
    let log_method = ctx.method.clone();
    let log_path = ctx.path.clone();
    let log_start = ctx.start_time;

    // Check if middleware short-circuited (e.g. OPTIONS, rate limit)
    if let Some(body) = &ctx.body {
        let mut resp_headers = String::new();
        for (k, v) in &ctx.response_headers {
            resp_headers.push_str(&format!("{}: {}\r\n", k, v));
        }
        let resp = format!(
            "HTTP/1.1 {} {}\r\n{}Content-Length: {}\r\n\r\n",
            ctx.status, status_text(ctx.status),
            resp_headers, body.len()
        );
        stream.write_all(resp.as_bytes())?;
        stream.write_all(body.as_bytes())?;
        let ms = log_start.elapsed().as_millis();
        eprintln!("[magnetic] {} {} → {} ({}ms)", log_method, log_path, ctx.status, ms);
        return Ok(());
    }

    // Collect response headers from middleware for subsequent handlers
    let extra_headers = ctx.response_headers.clone();

    let result = match (method, path) {
        ("GET", "/sse") => handle_sse(stream.try_clone()?, server, &extra_headers),
        ("POST", p) if p.starts_with("/actions/") => {
            let mut body = vec![0u8; content_length];
            if content_length > 0 { reader.read_exact(&mut body)?; }
            handle_action(&mut stream, server, p, &body, &extra_headers)
        }
        ("GET", p) => handle_get(&mut stream, server, p, &extra_headers),
        _ => {
            stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
        }
    };

    let ms = log_start.elapsed().as_millis();
    if log_path != "/sse" {
        eprintln!("[magnetic] {} {} → 200 ({}ms)", log_method, log_path, ms);
    }
    result
}

pub fn format_extra_headers(headers: &HashMap<String, String>) -> String {
    let mut s = String::new();
    for (k, v) in headers {
        s.push_str(&format!("{}: {}\r\n", k, v));
    }
    s
}

fn handle_sse(
    mut stream: TcpStream,
    server: &Server,
    extra_headers: &HashMap<String, String>,
) -> std::io::Result<()> {
    let eh = format_extra_headers(extra_headers);
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\nConnection: keep-alive\r\n{}\r\n", eh
    );
    stream.write_all(header.as_bytes())?;

    let path = server.current_path.lock().unwrap().clone();
    let reply = Reply::new();
    server.v8_tx.send(V8Request::Render { path: path.clone(), reply: reply.clone() }).unwrap();
    let dom_json = v8_result_to_json(reply.recv(), None);
    let snapshot = format!("{{\"root\":{}}}", dom_json);
    write_sse_event(&mut stream, snapshot.as_bytes())?;

    let client = stream.try_clone()?;
    let client_count = {
        let mut clients = server.sse_clients.lock().unwrap();
        clients.push(client);
        clients.len()
    };
    eprintln!("[magnetic] SSE client connected (path={}, total={})", path, client_count);

    loop {
        thread::sleep(std::time::Duration::from_secs(30));
        if stream.write_all(b": keepalive\n\n").is_err() {
            eprintln!("[magnetic] SSE client disconnected");
            break;
        }
    }
    Ok(())
}

fn handle_action(
    stream: &mut TcpStream,
    server: &Server,
    url_path: &str,
    body: &[u8],
    extra_headers: &HashMap<String, String>,
) -> std::io::Result<()> {
    let action = urlencoding_decode(url_path.strip_prefix("/actions/").unwrap_or(""));
    let body_str = String::from_utf8_lossy(body);

    let payload = if body_str.is_empty() { "{}".to_string() } else {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
            if let Some(p) = val.get("payload") { p.to_string() } else { val.to_string() }
        } else { "{}".to_string() }
    };

    let snapshot: String;

    if action == "navigate" {
        let nav_path = serde_json::from_str::<serde_json::Value>(&payload)
            .ok()
            .and_then(|v| v.get("path")?.as_str().map(String::from))
            .unwrap_or_else(|| "/".to_string());

        eprintln!("[magnetic] navigate → {}", nav_path);
        *server.current_path.lock().unwrap() = nav_path.clone();
        let v8_start = Instant::now();
        let reply = Reply::new();
        server.v8_tx.send(V8Request::Render { path: nav_path, reply: reply.clone() }).unwrap();
        let dom_json = v8_result_to_json(reply.recv(), None);
        eprintln!("[magnetic] V8 render: {}ms", v8_start.elapsed().as_micros() as f64 / 1000.0);
        snapshot = format!("{{\"root\":{}}}", dom_json);
    } else {
        let path = server.current_path.lock().unwrap().clone();
        eprintln!("[magnetic] action: {} (path={})", action, path);
        let v8_start = Instant::now();
        let reply = Reply::new();
        server.v8_tx.send(V8Request::Reduce {
            action: action.clone(), payload, path, reply: reply.clone(),
        }).unwrap();
        let dom_json = v8_result_to_json(reply.recv(), Some(&action));
        eprintln!("[magnetic] V8 reduce: {}ms", v8_start.elapsed().as_micros() as f64 / 1000.0);
        snapshot = format!("{{\"root\":{}}}", dom_json);
    }

    let eh = format_extra_headers(extra_headers);
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
        Content-Length: {}\r\n{}\r\n",
        snapshot.len(), eh
    );
    stream.write_all(resp.as_bytes())?;
    stream.write_all(snapshot.as_bytes())?;

    if action != "navigate" {
        let client_count = server.sse_clients.lock().unwrap().len();
        if client_count > 0 {
            eprintln!("[magnetic] broadcasting to {} SSE client(s)", client_count);
        }
        broadcast_snapshot(server, snapshot.as_bytes());
    }
    Ok(())
}

fn handle_get(
    stream: &mut TcpStream,
    server: &Server,
    path: &str,
    extra_headers: &HashMap<String, String>,
) -> std::io::Result<()> {
    // Static files
    let has_ext = path.contains('.') && !path.ends_with('/');
    let ext = path.rsplit('.').next().unwrap_or("");
    if has_ext && ext != "html" {
        return serve_static(stream, server, path, extra_headers);
    }

    // SSR
    let route_path = path.split('?').next().unwrap_or("/");
    *server.current_path.lock().unwrap() = route_path.to_string();

    // Use RenderWithCSS to get both DOM and generated CSS from V8
    let reply = Reply::new();
    server.v8_tx.send(V8Request::RenderWithCSS {
        path: route_path.to_string(), reply: reply.clone(),
    }).unwrap();

    let (dom, generated_css) = match reply.recv() {
        V8Result::Ok(json) => {
            // Parse {root: DomNode, css: string|null}
            match serde_json::from_str::<serde_json::Value>(&json) {
                Ok(wrapper) => {
                    let root_val = wrapper.get("root").cloned().unwrap_or(serde_json::Value::Null);
                    let css_val = wrapper.get("css").and_then(|v| v.as_str()).map(String::from);
                    match serde_json::from_value::<DomNode>(root_val) {
                        Ok(d) => (d, css_val),
                        Err(e) => {
                            eprintln!("[magnetic-v8] render parse error: {}", e);
                            (error_fallback(&format!("JSON parse error: {}", e), None), None)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[magnetic-v8] render parse error: {}", e);
                    (error_fallback(&format!("JSON parse error: {}", e), None), None)
                }
            }
        }
        V8Result::Err(e) => {
            eprintln!("[magnetic-v8] render error: {}", e);
            (error_fallback(&e, None), None)
        }
    };

    // Merge CSS: generated CSS from design.json + user's style.css (if any)
    let merged_css = match (&generated_css, &server.inline_css) {
        (Some(gen), Some(user)) => Some(format!("{}{}", gen, user)),
        (Some(gen), None) => Some(gen.clone()),
        (None, Some(user)) => Some(user.clone()),
        (None, None) => None,
    };

    // Framework assets are embedded in the binary — always available
    let magnetic_js = "/magnetic.js".to_string();
    let wasm_url = Some("/transport.wasm".to_string());

    let page = render_page(&PageOptions {
        root: dom,
        scripts: vec![magnetic_js],
        styles: vec![],
        inline_css: merged_css,
        sse_url: Some("/sse".to_string()),
        mount_selector: Some("#app".to_string()),
        wasm_url,
        title: Some("Magnetic Task Board".to_string()),
        description: Some("Server-driven UI — Rust + V8".to_string()),
    });

    let eh = format_extra_headers(extra_headers);
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n{}\r\n",
        page.len(), eh
    );
    stream.write_all(resp.as_bytes())?;
    stream.write_all(page.as_bytes())
}

/// Serve static files with proper cache headers based on asset manifest
fn serve_static(
    stream: &mut TcpStream,
    server: &Server,
    path: &str,
    extra_headers: &HashMap<String, String>,
) -> std::io::Result<()> {
    let filename = path.trim_start_matches('/');

    // Embedded framework assets — served from binary, never from disk
    if let Some(result) = serve_embedded(stream, filename, extra_headers) {
        return result;
    }

    // Try hashed asset dir first, then fallback to static_dir
    let file_path = {
        let hashed = std::path::Path::new(&server.asset_dir).join(filename);
        if hashed.exists() {
            hashed
        } else {
            let orig = std::path::Path::new(&server.static_dir).join(filename);
            orig
        }
    };

    // Security: prevent path traversal
    let canonical = file_path.to_string_lossy().to_string();
    if !canonical.starts_with(&server.static_dir) && !canonical.starts_with(&server.asset_dir) {
        return stream.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n");
    }

    let data = match std::fs::read(&file_path) {
        Ok(d) => d,
        Err(_) => {
            return stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
        }
    };

    let ct = guess_content_type(path);

    // Determine cache strategy from manifest
    let is_hashed = server.manifest.reverse.contains_key(filename)
        && server.manifest.reverse.get(filename).map(|o| o != filename).unwrap_or(false);

    let cache = if is_hashed {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=300, must-revalidate"
    };

    let eh = format_extra_headers(extra_headers);
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
        Cache-Control: {}\r\n{}\r\n",
        ct, data.len(), cache, eh
    );
    stream.write_all(resp.as_bytes())?;
    stream.write_all(&data)
}

/// Convert V8Result to JSON string, using error_fallback on error
pub fn v8_result_to_json(result: V8Result, action: Option<&str>) -> String {
    match result {
        V8Result::Ok(json) => json,
        V8Result::Err(e) => {
            eprintln!("[magnetic-v8] error boundary: {}", e);
            let fallback = error_fallback(&e, action);
            serde_json::to_string(&fallback).unwrap_or_else(|_| {
                r#"{"tag":"div","text":"Error"}"#.to_string()
            })
        }
    }
}

pub fn broadcast_snapshot(server: &Server, snapshot: &[u8]) {
    let mut clients = server.sse_clients.lock().unwrap();
    let mut alive = Vec::new();
    for mut client in clients.drain(..) {
        if write_sse_event(&mut client, snapshot).is_ok() {
            alive.push(client);
        }
    }
    *clients = alive;
}

pub fn write_sse_event(stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    stream.write_all(b"event: message\ndata: ")?;
    stream.write_all(data)?;
    stream.write_all(b"\n\n")?;
    stream.flush()
}

pub fn guess_content_type(path: &str) -> &str {
    if path.ends_with(".js") { "application/javascript" }
    else if path.ends_with(".css") { "text/css" }
    else if path.ends_with(".json") { "application/json" }
    else if path.ends_with(".wasm") { "application/wasm" }
    else if path.ends_with(".html") { "text/html; charset=utf-8" }
    else if path.ends_with(".png") { "image/png" }
    else if path.ends_with(".svg") { "image/svg+xml" }
    else if path.ends_with(".ico") { "image/x-icon" }
    else if path.ends_with(".woff2") { "font/woff2" }
    else if path.ends_with(".woff") { "font/woff" }
    else { "application/octet-stream" }
}

pub fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

pub fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}
