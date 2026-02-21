//! platform.rs — Multi-tenant hosting mode
//!
//! In platform mode, the server manages multiple apps. Each app gets:
//! - Its own V8 thread (dedicated isolate)
//! - Its own SSE clients, state, static assets
//! - A URL namespace: /apps/<name>/*
//!
//! Apps are deployed via POST /api/apps/<name>/deploy with JSON body:
//! { "bundle": "<js source>", "assets": { "file.css": "<content>", ... } }

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use magnetic_dom::DomNode;
use magnetic_render_html::{render_page, PageOptions};

use crate::{
    V8Request, V8Result, Reply, AssetManifest,
    MagneticContext, MiddlewareStack,
    v8_thread, v8_result_to_json, error_fallback,
    write_sse_event, write_sse_named, guess_content_type,
    format_extra_headers, status_text, urlencoding_decode,
    cors_middleware, rate_limit_middleware, logger_middleware,
    build_assets, find_arg, serve_embedded,
};
use crate::data::{DataContext, SseDelta, parse_config, fetch_page_data, fetch_page_data_with_token, fetch_page_data_streaming, forward_action, start_poll_threads, start_sse_threads, fetch_data_source};
use crate::auth::AuthMiddleware;

// ── Idle timeout for V8 parking ──────────────────────────────────────

const PARK_IDLE_SECS: u64 = 300; // 5 minutes
const REAPER_INTERVAL_SECS: u64 = 30;

// ── Per-app handle ──────────────────────────────────────────────────

struct AppHandle {
    name: String,
    v8_tx: Mutex<Option<mpsc::Sender<V8Request>>>,
    parked: AtomicBool,
    last_activity: Mutex<Instant>,
    /// Per-session SSE clients: session_id → list of TcpStream clones
    sse_clients: Mutex<HashMap<String, Vec<TcpStream>>>,
    /// Per-session current path: session_id → path
    session_paths: Mutex<HashMap<String, String>>,
    static_dir: String,
    asset_dir: String,
    inline_css: Option<String>,
    manifest: AssetManifest,
    data_dir: String,
    /// Declarative data layer context (if magnetic.json has data/actions config)
    data_ctx: Option<Arc<DataContext>>,
    /// Auth middleware (if magnetic.json has auth config)
    auth: Option<Arc<AuthMiddleware>>,
}

impl AppHandle {
    /// Touch activity timestamp.
    fn touch(&self) {
        *self.last_activity.lock().unwrap() = Instant::now();
    }

    /// Ensure V8 thread is available. Returns sender or error string.
    fn ensure_warm(&self) -> Result<mpsc::Sender<V8Request>, String> {
        let guard = self.v8_tx.lock().unwrap();
        if let Some(ref tx) = *guard {
            if self.parked.load(Ordering::Acquire) {
                self.parked.store(false, Ordering::Release);
                eprintln!("[platform:{}] unparked (request)", self.name);
            }
            return Ok(tx.clone());
        }
        Err(format!("V8 thread not available for '{}'", self.name))
    }

    /// Mark app as parked (idle). The V8 thread stays alive — V8's global
    /// platform cannot be reinitialized, so we never kill V8 threads.
    /// The thread blocks on rx.recv() which costs zero CPU when idle.
    fn park(&self) {
        if !self.parked.load(Ordering::Acquire) {
            self.parked.store(true, Ordering::Release);
            eprintln!("[platform:{}] parked (idle)", self.name);
        }
    }

    fn is_parked(&self) -> bool {
        self.parked.load(Ordering::Acquire)
    }

    fn sse_client_count(&self) -> usize {
        self.sse_clients.lock().unwrap().values().map(|v| v.len()).sum()
    }

    fn idle_secs(&self) -> u64 {
        self.last_activity.lock().unwrap().elapsed().as_secs()
    }
}

/// Start background data threads (poll + SSE) for an app.
/// The on_change callback re-renders for all active sessions and pushes via SSE.
fn start_data_threads(app: Arc<AppHandle>) {
    let ctx = match app.data_ctx {
        Some(ref ctx) => Arc::clone(ctx),
        None => return,
    };

    let has_poll = ctx.config.data.iter().any(|d| d.source_type == "poll");
    let has_sse = ctx.config.data.iter().any(|d| d.source_type == "sse");
    if !has_poll && !has_sse {
        return;
    }

    // Debounce flag: when SSE events arrive in rapid succession, we coalesce
    // multiple on_change calls into a single re-render after a short delay.
    // Without this, rapid-fire DOM snapshots overwhelm the browser WASM patcher.
    let pending = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let on_change: Arc<dyn Fn() + Send + Sync> = {
        let app = Arc::clone(&app);
        let pending = Arc::clone(&pending);
        Arc::new(move || {
            // If a render is already scheduled, skip — it will pick up our data
            if pending.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            let app = Arc::clone(&app);
            let pending = Arc::clone(&pending);
            thread::spawn(move || {
                // Wait briefly to coalesce rapid-fire events
                thread::sleep(std::time::Duration::from_millis(150));
                pending.store(false, std::sync::atomic::Ordering::SeqCst);

                // Re-render for each active session and push SSE updates
                let sessions: Vec<(String, String)> = {
                    let paths = app.session_paths.lock().unwrap();
                    paths.iter().map(|(sid, p)| (sid.clone(), p.clone())).collect()
                };
                if sessions.is_empty() {
                    return;
                }
                let tx = match app.ensure_warm() {
                    Ok(tx) => tx,
                    Err(_) => return,
                };
                let ctx = match app.data_ctx {
                    Some(ref ctx) => ctx,
                    None => return,
                };
                for (session_id, path) in &sessions {
                    let data_json = ctx.data_json_for_page(path);
                    let reply = Reply::new();
                    if tx.send(V8Request::RenderWithData {
                        path: path.clone(),
                        session_id: session_id.clone(),
                        data_json,
                        reply: reply.clone(),
                    }).is_err() {
                        continue;
                    }
                    let dom_json = v8_result_to_json(reply.recv(), None);
                    let snapshot = format!("{{\"root\":{}}}", dom_json);
                    let mut clients = app.sse_clients.lock().unwrap();
                    if let Some(list) = clients.get_mut(session_id) {
                        let mut alive = Vec::new();
                        for mut client in list.drain(..) {
                            if write_sse_event(&mut client, snapshot.as_bytes()).is_ok() {
                                alive.push(client);
                            }
                        }
                        if alive.is_empty() {
                            clients.remove(session_id);
                        } else {
                            *list = alive;
                        }
                    }
                }
            });
        })
    };

    // Delta callback: for SSE sources with `target`, send raw event data
    // directly to browser clients — completely bypasses V8 rendering.
    let on_sse_event: Arc<dyn Fn(SseDelta) + Send + Sync> = {
        let app = Arc::clone(&app);
        Arc::new(move |delta: SseDelta| {
            let sessions: Vec<String> = {
                let paths = app.session_paths.lock().unwrap();
                paths.keys().cloned().collect()
            };
            if sessions.is_empty() {
                eprintln!("[delta] no sessions, skipping delta for '{}'", delta.key);
                return;
            }

            // Delta message sent as regular "message" event with delta flag.
            // Format: {"delta":true,"k":"events","v":{...},"max":20,"t":"feed"}
            let msg = serde_json::json!({
                "delta": true,
                "k": delta.key,
                "v": delta.value,
                "max": delta.buffer_size,
                "t": delta.target,
            });
            let msg_bytes = msg.to_string();
            eprintln!("[delta] sending to {} sessions: {}", sessions.len(), &msg_bytes[..std::cmp::min(200, msg_bytes.len())]);

            let mut clients = app.sse_clients.lock().unwrap();
            for session_id in &sessions {
                if let Some(list) = clients.get_mut(session_id) {
                    let mut alive = Vec::new();
                    for mut client in list.drain(..) {
                        if write_sse_event(&mut client, msg_bytes.as_bytes()).is_ok() {
                            alive.push(client);
                        }
                    }
                    if alive.is_empty() {
                        clients.remove(session_id);
                    } else {
                        *list = alive;
                    }
                }
            }
        })
    };

    if has_poll {
        start_poll_threads(Arc::clone(&ctx), Arc::clone(&on_change));
    }
    if has_sse {
        start_sse_threads(Arc::clone(&ctx), on_change, Some(on_sse_event));
    }
}

// ── Platform state ──────────────────────────────────────────────────

pub struct Platform {
    apps: RwLock<HashMap<String, Arc<AppHandle>>>,
    data_dir: String,
    middleware: MiddlewareStack,
}

// ── Platform entry point ────────────────────────────────────────────

pub fn run_platform(args: &[String]) {
    let port = find_arg(args, "--port").unwrap_or_else(|| "3003".to_string());
    let data_dir = find_arg(args, "--data-dir").unwrap_or_else(|| "data/apps".to_string());
    let cors_origin = find_arg(args, "--cors").unwrap_or_else(|| "*".to_string());
    let rate_limit_max: u32 = find_arg(args, "--rate-limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);

    // Ensure data directory exists
    let _ = std::fs::create_dir_all(&data_dir);

    // Initialize V8 once on the main thread before spawning any app threads.
    // This prevents a race condition where concurrent v8_thread spawns
    // could SEGV if V8 internals aren't fully ready.
    crate::ensure_v8_initialized();

    // Build middleware
    let mut middleware = MiddlewareStack::new();
    middleware.add(logger_middleware());
    middleware.add(cors_middleware(&cors_origin));
    middleware.add(rate_limit_middleware(60_000, rate_limit_max));

    let park_idle = find_arg(args, "--park-idle")
        .and_then(|s| s.parse().ok())
        .unwrap_or(PARK_IDLE_SECS);

    let platform = Arc::new(Platform {
        apps: RwLock::new(HashMap::new()),
        data_dir: data_dir.clone(),
        middleware,
    });

    // Load existing apps from data directory.
    // Collect loaded apps first, start data threads AFTER all apps are loaded
    // and the HTTP server is ready. SSE sources can deliver events immediately,
    // and the on_change callback blocks on V8 — starting them during load
    // deadlocks if V8 is still initializing the bundle.
    let mut loaded_apps: Vec<Arc<AppHandle>> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&data_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().to_string();
                let bundle_path = entry.path().join("bundle.js");
                if bundle_path.exists() {
                    match load_app(&name, &data_dir) {
                        Ok(handle) => {
                            eprintln!("[platform] Loaded app: {}", name);
                            let app = Arc::new(handle);
                            platform.apps.write().unwrap().insert(name, Arc::clone(&app));
                            loaded_apps.push(app);
                        }
                        Err(e) => eprintln!("[platform] Failed to load {}: {}", name, e),
                    }
                }
            }
        }
    }

    // Start reaper thread for V8 isolate parking
    {
        let platform_ref = Arc::clone(&platform);
        thread::spawn(move || reaper_loop(platform_ref, park_idle));
    }

    let app_count = platform.apps.read().unwrap().len();
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    eprintln!("[platform] http://localhost:{}", port);
    eprintln!("[platform] Magnetic Platform Server — multi-tenant V8 hosting");
    eprintln!("[platform] Data dir: {}", data_dir);
    eprintln!("[platform] Apps loaded: {}", app_count);
    eprintln!("[platform] V8 park idle: {}s", park_idle);
    eprintln!("[platform] Deploy: POST /api/apps/<name>/deploy");
    eprintln!("[platform] Access: GET /apps/<name>/");

    // Start data threads (poll + SSE) AFTER server is listening.
    // SSE on_change callbacks send to V8 channels which block until V8
    // finishes initializing the bundle. Starting them earlier deadlocks.
    if !loaded_apps.is_empty() {
        let count = loaded_apps.len();
        thread::spawn(move || {
            for app in loaded_apps {
                start_data_threads(app);
            }
            eprintln!("[platform] Data threads started for {} app(s)", count);
        });
    }

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => { eprintln!("[err] accept: {}", e); continue; }
        };
        let platform = Arc::clone(&platform);
        thread::spawn(move || {
            if let Err(_) = handle_platform_connection(stream, &platform) {}
        });
    }
}

// ── Load an app from disk ───────────────────────────────────────────

fn load_app(name: &str, data_dir: &str) -> Result<AppHandle, String> {
    let app_dir = format!("{}/{}", data_dir, name);
    let bundle_path = format!("{}/bundle.js", app_dir);
    let config_path = format!("{}/config.json", app_dir);
    let public_dir = format!("{}/public", app_dir);

    let js_source = std::fs::read_to_string(&bundle_path)
        .map_err(|e| format!("Cannot read bundle: {}", e))?;

    // Start V8 thread for this app
    let (tx, rx) = mpsc::channel();
    let js = js_source;
    thread::spawn(move || v8_thread(js, rx));

    // Load data layer config (if present)
    let mut data_ctx: Option<Arc<DataContext>> = None;
    let mut auth_mw: Option<Arc<AuthMiddleware>> = None;

    if std::path::Path::new(&config_path).exists() {
        if let Ok(json) = std::fs::read_to_string(&config_path) {
            match parse_config(&json) {
                Ok(config) => {
                    // Initialize auth middleware if configured
                    if let Some(ref auth_cfg) = config.auth {
                        eprintln!("[platform:{}] auth: provider={}", name, auth_cfg.provider);
                        auth_mw = Some(Arc::new(AuthMiddleware::new(auth_cfg.clone())));
                    }

                    let has_data = !config.data.is_empty();
                    let has_actions = !config.actions.is_empty();
                    if has_data || has_actions {
                        eprintln!(
                            "[platform:{}] data layer: {} sources, {} actions",
                            name, config.data.len(), config.actions.len()
                        );
                        let ctx = Arc::new(DataContext::new(config));
                        // Fetch initial data for all global sources
                        let fetched = fetch_page_data(&ctx, "/");
                        if fetched > 0 {
                            let data_json = ctx.data_json_for_page("/");
                            let reply = Reply::new();
                            let _ = tx.send(V8Request::SetData {
                                json: data_json,
                                reply: reply.clone(),
                            });
                            match reply.recv_timeout(std::time::Duration::from_secs(10)) {
                                V8Result::Err(e) => eprintln!("[platform:{}] setData failed: {}", name, e),
                                _ => eprintln!("[platform:{}] injected {} data sources", name, fetched),
                            }
                        }
                        data_ctx = Some(ctx);
                    }
                }
                Err(e) => eprintln!("[platform:{}] config parse error: {}", name, e),
            }
        }
    }

    // Build asset pipeline
    let asset_dir = format!("{}/.hashed", public_dir);
    let manifest = build_assets(
        &public_dir, &asset_dir,
        &["index.html"],
    );

    // Load CSS
    let css_path = manifest.files.get("style.css")
        .map(|h| format!("{}/{}", asset_dir, h))
        .unwrap_or_else(|| format!("{}/style.css", public_dir));
    let inline_css = std::fs::read_to_string(&css_path).ok();

    Ok(AppHandle {
        name: name.to_string(),
        v8_tx: Mutex::new(Some(tx)),
        parked: AtomicBool::new(false),
        last_activity: Mutex::new(Instant::now()),
        sse_clients: Mutex::new(HashMap::new()),
        session_paths: Mutex::new(HashMap::new()),
        static_dir: public_dir,
        asset_dir,
        inline_css,
        manifest,
        data_dir: data_dir.to_string(),
        data_ctx,
        auth: auth_mw,
    })
}

// ── Reaper thread: parks idle V8 isolates ───────────────────────────

fn reaper_loop(platform: Arc<Platform>, idle_threshold: u64) {
    loop {
        thread::sleep(Duration::from_secs(REAPER_INTERVAL_SECS));
        let apps = platform.apps.read().unwrap();
        for (name, app) in apps.iter() {
            if app.is_parked() {
                continue;
            }
            let idle = app.idle_secs();
            let clients = app.sse_client_count();
            if idle >= idle_threshold && clients == 0 {
                eprintln!(
                    "[reaper] parking '{}' (idle {}s, 0 SSE clients)",
                    name, idle
                );
                app.park();
            }
        }
    }
}

// ── Platform HTTP handler ───────────────────────────────────────────

fn handle_platform_connection(
    mut stream: TcpStream,
    platform: &Platform,
) -> std::io::Result<()> {
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

    // Detect subdomain access.
    // Primary: X-Subdomain header (if Caddy sends it).
    // Fallback: parse subdomain from X-Forwarded-Host (Caddy always sets this).
    // e.g. X-Forwarded-Host: zf9at7gr.fujs.dev → subdomain = "zf9at7gr"
    let via_subdomain = raw_headers.get("x-subdomain").cloned()
        .or_else(|| {
            raw_headers.get("x-forwarded-host")
                .and_then(|host| {
                    let parts: Vec<&str> = host.split('.').collect();
                    // Must be at least 3 parts (sub.domain.tld) and not "api" or "www"
                    if parts.len() >= 3 && parts[0] != "api" && parts[0] != "www" {
                        Some(parts[0].to_string())
                    } else {
                        None
                    }
                })
        });

    // Keep a copy of headers for auth cookie extraction
    let req_headers = raw_headers.clone();

    // Run middleware
    let mut ctx = MagneticContext::from_request(method, path, raw_headers);
    platform.middleware.run(&mut ctx);
    let log_start = ctx.start_time;

    if let Some(body) = &ctx.body {
        let mut resp_headers = String::new();
        for (k, v) in &ctx.response_headers {
            resp_headers.push_str(&format!("{}: {}\r\n", k, v));
        }
        let resp = format!(
            "HTTP/1.1 {} {}\r\n{}Content-Length: {}\r\n\r\n",
            ctx.status, status_text(ctx.status), resp_headers, body.len()
        );
        stream.write_all(resp.as_bytes())?;
        stream.write_all(body.as_bytes())?;
        let ms = log_start.elapsed().as_millis();
        eprintln!("[platform] {} {} → {} ({}ms)", method, path, ctx.status, ms);
        return Ok(());
    }

    let extra_headers = ctx.response_headers.clone();

    // Route: deploy API
    if method == "POST" && path.starts_with("/api/apps/") && path.ends_with("/deploy") {
        let mut body = vec![0u8; content_length];
        if content_length > 0 { reader.read_exact(&mut body)?; }
        let result = handle_deploy(&mut stream, platform, path, &body, &extra_headers);
        let ms = log_start.elapsed().as_millis();
        eprintln!("[platform] {} {} → ({}ms)", method, path, ms);
        return result;
    }

    // Route: app status
    if method == "GET" && path.starts_with("/api/apps/") && path.ends_with("/status") {
        let name = path
            .strip_prefix("/api/apps/")
            .and_then(|s| s.strip_suffix("/status"))
            .unwrap_or("");
        let apps = platform.apps.read().unwrap();
        let json = if let Some(app) = apps.get(name) {
            format!(
                "{{\"name\":\"{}\",\"warm\":{},\"sse_clients\":{},\"idle_secs\":{}}}",
                name, !app.is_parked(), app.sse_client_count(), app.idle_secs()
            )
        } else {
            format!("{{\"error\":\"App '{}' not found\"}}", name)
        };
        let eh = format_extra_headers(&extra_headers);
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
            Content-Length: {}\r\n{}\r\n",
            json.len(), eh
        );
        stream.write_all(resp.as_bytes())?;
        return stream.write_all(json.as_bytes());
    }

    // Route: list apps
    if method == "GET" && path == "/api/apps" {
        let apps = platform.apps.read().unwrap();
        let names: Vec<&str> = apps.keys().map(|s| s.as_str()).collect();
        let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".into());
        let eh = format_extra_headers(&extra_headers);
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
            Content-Length: {}\r\n{}\r\n",
            json.len(), eh
        );
        stream.write_all(resp.as_bytes())?;
        return stream.write_all(json.as_bytes());
    }

    // Route: platform homepage
    if method == "GET" && (path == "/" || path == "") {
        let apps = platform.apps.read().unwrap();
        let mut html = String::from("<!DOCTYPE html><html><head><title>Magnetic Platform</title>\
            <style>body{font-family:system-ui;max-width:600px;margin:40px auto;padding:0 20px}\
            a{color:#0066cc}h1{color:#333}.app{padding:8px 0;border-bottom:1px solid #eee}</style></head>\
            <body><h1>Magnetic Platform</h1><p>Server-driven UI hosting</p>");
        if apps.is_empty() {
            html.push_str("<p>No apps deployed yet.</p>");
            html.push_str("<p>Deploy with: <code>magnetic push --server http://localhost:PORT --name my-app</code></p>");
        } else {
            html.push_str("<h2>Deployed Apps</h2>");
            for name in apps.keys() {
                html.push_str(&format!(
                    "<div class=\"app\"><a href=\"/apps/{}/\">{}</a></div>",
                    name, name
                ));
            }
        }
        html.push_str("</body></html>");
        let eh = format_extra_headers(&extra_headers);
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
            Content-Length: {}\r\n{}\r\n",
            html.len(), eh
        );
        stream.write_all(resp.as_bytes())?;
        return stream.write_all(html.as_bytes());
    }

    // Route: app requests /apps/<name>/*
    if path.starts_with("/apps/") {
        let rest = &path[6..]; // after "/apps/"
        let (app_name, app_path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };

        let apps = platform.apps.read().unwrap();
        if let Some(app) = apps.get(app_name) {
            let app = Arc::clone(app);
            drop(apps); // release read lock

            match (method, app_path) {
                // ── Auth routes ──────────────────────────────────
                ("GET", "/auth/login") if app.auth.is_some() => {
                    let auth = app.auth.as_ref().unwrap();
                    let state = "magnetic"; // TODO: CSRF state token
                    let url = auth.login_url(state);
                    let eh = format_extra_headers(&extra_headers);
                    let resp = format!(
                        "HTTP/1.1 302 Found\r\nLocation: {}\r\n{}\r\n",
                        url, eh
                    );
                    return stream.write_all(resp.as_bytes());
                }
                ("GET", p) if p.starts_with("/auth/callback") && app.auth.is_some() => {
                    let auth = app.auth.as_ref().unwrap();
                    // Extract ?code= (OAuth2) or ?token= (magic-link) from query string
                    let code = path.split("code=").nth(1)
                        .and_then(|s| s.split('&').next())
                        .unwrap_or("");
                    let token = path.split("token=").nth(1)
                        .and_then(|s| s.split('&').next())
                        .unwrap_or("");
                    // Use token for magic-link, code for OAuth2
                    let exchange_value = if !token.is_empty() { token } else { code };
                    if exchange_value.is_empty() {
                        let msg = "{\"error\":\"Missing authorization code or token\"}";
                        let resp = format!(
                            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                            msg.len()
                        );
                        stream.write_all(resp.as_bytes())?;
                        return stream.write_all(msg.as_bytes());
                    }
                    match auth.exchange_code(exchange_value) {
                        Ok((access_token, refresh_token, expires_in)) => {
                            let (_session_id, cookie) = auth.create_session(
                                &access_token,
                                refresh_token.as_deref(),
                                expires_in,
                            );
                            let redirect_to = if via_subdomain.is_some() {
                                "/".to_string()
                            } else {
                                format!("/apps/{}/", app_name)
                            };
                            let eh = format_extra_headers(&extra_headers);
                            let resp = format!(
                                "HTTP/1.1 302 Found\r\nLocation: {}\r\nSet-Cookie: {}\r\n{}\r\n",
                                redirect_to, cookie, eh
                            );
                            eprintln!("[platform:{}] auth callback: session created ({})", app_name, auth.provider());
                            return stream.write_all(resp.as_bytes());
                        }
                        Err(e) => {
                            eprintln!("[platform:{}] auth callback error: {}", app_name, e);
                            let msg = format!("{{\"error\":\"{}\"}}", e);
                            let resp = format!(
                                "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                                msg.len()
                            );
                            stream.write_all(resp.as_bytes())?;
                            return stream.write_all(msg.as_bytes());
                        }
                    }
                }
                // POST /auth/send — send magic-link or OTP email
                ("POST", "/auth/send") if app.auth.is_some() => {
                    let auth = app.auth.as_ref().unwrap();
                    if !auth.is_magic_link() && !auth.is_otp() {
                        let msg = "{\"error\":\"This auth provider does not support /auth/send\"}";
                        let resp = format!(
                            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                            msg.len()
                        );
                        stream.write_all(resp.as_bytes())?;
                        return stream.write_all(msg.as_bytes());
                    }
                    let mut body = vec![0u8; content_length];
                    if content_length > 0 { reader.read_exact(&mut body)?; }
                    let body_str = String::from_utf8_lossy(&body);
                    let email = serde_json::from_str::<serde_json::Value>(&body_str)
                        .ok()
                        .and_then(|v| v.get("email")?.as_str().map(String::from))
                        .unwrap_or_default();
                    if email.is_empty() {
                        let msg = "{\"error\":\"Missing email in request body\"}";
                        let resp = format!(
                            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                            msg.len()
                        );
                        stream.write_all(resp.as_bytes())?;
                        return stream.write_all(msg.as_bytes());
                    }
                    match auth.send_auth_email(&email) {
                        Ok(result) => {
                            let msg = result.to_string();
                            let eh = format_extra_headers(&extra_headers);
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{}\r\n",
                                msg.len(), eh
                            );
                            eprintln!("[platform:{}] auth send: {} to {}", app_name, auth.provider(), email);
                            stream.write_all(resp.as_bytes())?;
                            return stream.write_all(msg.as_bytes());
                        }
                        Err(e) => {
                            eprintln!("[platform:{}] auth send error: {}", app_name, e);
                            let msg = format!("{{\"error\":\"{}\"}}", e);
                            let resp = format!(
                                "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                                msg.len()
                            );
                            stream.write_all(resp.as_bytes())?;
                            return stream.write_all(msg.as_bytes());
                        }
                    }
                }
                // POST /auth/verify — verify OTP code
                ("POST", "/auth/verify") if app.auth.is_some() => {
                    let auth = app.auth.as_ref().unwrap();
                    if !auth.is_otp() {
                        let msg = "{\"error\":\"This auth provider does not support /auth/verify\"}";
                        let resp = format!(
                            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                            msg.len()
                        );
                        stream.write_all(resp.as_bytes())?;
                        return stream.write_all(msg.as_bytes());
                    }
                    let mut body = vec![0u8; content_length];
                    if content_length > 0 { reader.read_exact(&mut body)?; }
                    let body_str = String::from_utf8_lossy(&body);
                    let json_body = serde_json::from_str::<serde_json::Value>(&body_str)
                        .unwrap_or(serde_json::json!({}));
                    let code = json_body.get("code").and_then(|v| v.as_str()).unwrap_or("");
                    let method_id = json_body.get("method_id").and_then(|v| v.as_str()).unwrap_or("");
                    if code.is_empty() {
                        let msg = "{\"error\":\"Missing code in request body\"}";
                        let resp = format!(
                            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                            msg.len()
                        );
                        stream.write_all(resp.as_bytes())?;
                        return stream.write_all(msg.as_bytes());
                    }
                    match auth.verify_otp_code(code, method_id) {
                        Ok((access_token, refresh_token, expires_in)) => {
                            let (_session_id, cookie) = auth.create_session(
                                &access_token,
                                refresh_token.as_deref(),
                                expires_in,
                            );
                            let msg = "{\"ok\":true}";
                            let eh = format_extra_headers(&extra_headers);
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nSet-Cookie: {}\r\nContent-Length: {}\r\n{}\r\n",
                                cookie, msg.len(), eh
                            );
                            eprintln!("[platform:{}] auth verify: OTP verified, session created", app_name);
                            stream.write_all(resp.as_bytes())?;
                            return stream.write_all(msg.as_bytes());
                        }
                        Err(e) => {
                            eprintln!("[platform:{}] auth verify error: {}", app_name, e);
                            let msg = format!("{{\"error\":\"{}\"}}", e);
                            let resp = format!(
                                "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                                msg.len()
                            );
                            stream.write_all(resp.as_bytes())?;
                            return stream.write_all(msg.as_bytes());
                        }
                    }
                }
                ("POST", "/auth/logout") if app.auth.is_some() => {
                    let auth = app.auth.as_ref().unwrap();
                    let cookie = auth.logout(&req_headers);
                    let msg = "{\"ok\":true}";
                    let eh = format_extra_headers(&extra_headers);
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nSet-Cookie: {}\r\nContent-Length: {}\r\n{}\r\n",
                        cookie, msg.len(), eh
                    );
                    stream.write_all(resp.as_bytes())?;
                    return stream.write_all(msg.as_bytes());
                }
                // ── Standard app routes ──────────────────────────
                ("GET", "/sse") => {
                    return handle_app_sse(stream, &app, &extra_headers, &req_headers);
                }
                ("POST", p) if p.starts_with("/actions/") => {
                    let mut body = vec![0u8; content_length];
                    if content_length > 0 { reader.read_exact(&mut body)?; }
                    let result = handle_app_action(
                        &mut stream, &app, p, &body, &extra_headers, &req_headers,
                    );
                    let ms = log_start.elapsed().as_millis();
                    eprintln!("[platform] {} /apps/{}{} → ({}ms)", method, app_name, p, ms);
                    return result;
                }
                (m, p) if p.starts_with("/api/") => {
                    let mut body = vec![0u8; content_length];
                    if content_length > 0 { reader.read_exact(&mut body)?; }
                    let result = handle_app_api(
                        &mut stream, &app, m, p, &body, &extra_headers,
                    );
                    let ms = log_start.elapsed().as_millis();
                    eprintln!("[platform] {} /apps/{}{} → ({}ms)", m, app_name, p, ms);
                    return result;
                }
                ("GET", p) => {
                    let result = handle_app_get(
                        &mut stream, Arc::clone(&app), app_name, p, &extra_headers,
                        via_subdomain.is_some(), &req_headers,
                    );
                    let ms = log_start.elapsed().as_millis();
                    eprintln!("[platform] {} /apps/{}{} → ({}ms)", method, app_name, p, ms);
                    return result;
                }
                _ => {}
            }
        } else {
            let msg = format!("{{\"error\":\"App '{}' not found\"}}", app_name);
            let eh = format_extra_headers(&extra_headers);
            let resp = format!(
                "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\n\
                Content-Length: {}\r\n{}\r\n",
                msg.len(), eh
            );
            stream.write_all(resp.as_bytes())?;
            return stream.write_all(msg.as_bytes());
        }
    }

    stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
}

// ── Deploy handler ──────────────────────────────────────────────────

fn handle_deploy(
    stream: &mut TcpStream,
    platform: &Platform,
    url_path: &str,
    body: &[u8],
    extra_headers: &HashMap<String, String>,
) -> std::io::Result<()> {
    // Extract app name from /api/apps/<name>/deploy
    let name = url_path
        .strip_prefix("/api/apps/")
        .and_then(|s| s.strip_suffix("/deploy"))
        .unwrap_or("")
        .to_string();

    if name.is_empty() || name.contains('/') || name.contains("..") {
        let msg = "{\"error\":\"Invalid app name\"}";
        let resp = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\n\
            Content-Length: {}\r\n\r\n", msg.len()
        );
        stream.write_all(resp.as_bytes())?;
        return stream.write_all(msg.as_bytes());
    }

    // Parse deploy payload
    let body_str = String::from_utf8_lossy(body);
    let payload: serde_json::Value = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("{{\"error\":\"Invalid JSON: {}\"}}", e);
            let resp = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\n\
                Content-Length: {}\r\n\r\n", msg.len()
            );
            stream.write_all(resp.as_bytes())?;
            return stream.write_all(msg.as_bytes());
        }
    };

    let bundle = payload.get("bundle").and_then(|v| v.as_str()).unwrap_or("");
    if bundle.is_empty() {
        let msg = "{\"error\":\"Missing 'bundle' field\"}";
        let resp = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\n\
            Content-Length: {}\r\n\r\n", msg.len()
        );
        stream.write_all(resp.as_bytes())?;
        return stream.write_all(msg.as_bytes());
    }

    // Write bundle and assets to disk
    let app_dir = format!("{}/{}", platform.data_dir, name);
    let public_dir = format!("{}/public", app_dir);
    let _ = std::fs::create_dir_all(&public_dir);

    std::fs::write(format!("{}/bundle.js", app_dir), bundle)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Write data layer config (if present in payload)
    if let Some(config_str) = payload.get("config").and_then(|v| v.as_str()) {
        if !config_str.is_empty() && config_str != "null" {
            let _ = std::fs::write(format!("{}/config.json", app_dir), config_str);
            eprintln!("[platform] Saved data layer config for '{}'", name);
        }
    }

    // Write assets
    if let Some(assets) = payload.get("assets").and_then(|v| v.as_object()) {
        for (filename, content) in assets {
            if let Some(text) = content.as_str() {
                // Security: prevent path traversal
                if filename.contains("..") || filename.contains('/') { continue; }
                let _ = std::fs::write(format!("{}/{}", public_dir, filename), text);
            }
        }
    }

    eprintln!("[platform] Deploying app: {}", name);

    // Load (or reload) the app
    match load_app(&name, &platform.data_dir) {
        Ok(handle) => {
            let app = Arc::new(handle);
            let mut apps = platform.apps.write().unwrap();
            // Old app handle is dropped, V8 thread will exit when channel closes
            apps.insert(name.clone(), Arc::clone(&app));
            drop(apps);

            // Send the HTTP response BEFORE starting data threads.
            // SSE sources can deliver events immediately, and the on_change
            // callback sends to the V8 channel — which blocks if V8 is still
            // initializing the bundle. That would deadlock the deploy handler.
            let msg = format!(
                "{{\"ok\":true,\"name\":\"{}\",\"url\":\"/apps/{}/\"}}",
                name, name
            );
            let eh = format_extra_headers(extra_headers);
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                Content-Length: {}\r\n{}\r\n",
                msg.len(), eh
            );
            stream.write_all(resp.as_bytes())?;
            stream.write_all(msg.as_bytes())?;
            eprintln!("[platform] ✓ App '{}' deployed at /apps/{}/", name, name);

            // Start data threads (poll + SSE) after response is sent
            start_data_threads(app);
            Ok(())
        }
        Err(e) => {
            let msg = format!("{{\"error\":\"Deploy failed: {}\"}}", e);
            let resp = format!(
                "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\
                Content-Length: {}\r\n\r\n", msg.len()
            );
            stream.write_all(resp.as_bytes())?;
            stream.write_all(msg.as_bytes())
        }
    }
}

// ── Per-app request handlers ────────────────────────────────────────

fn handle_app_sse(
    mut stream: TcpStream,
    app: &AppHandle,
    extra_headers: &HashMap<String, String>,
    req_headers: &HashMap<String, String>,
) -> std::io::Result<()> {
    use crate::{extract_session_cookie, generate_session_id};

    // Get or create session ID from cookie
    let session_id = extract_session_cookie(req_headers)
        .unwrap_or_else(generate_session_id);

    let eh = format_extra_headers(extra_headers);
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\nConnection: keep-alive\r\n\
        Set-Cookie: magnetic_sid={}; Path=/; HttpOnly; SameSite=Lax\r\n{}\r\n",
        session_id, eh
    );
    stream.write_all(header.as_bytes())?;

    app.touch();
    let tx = app.ensure_warm().map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    let path = app.session_paths.lock().unwrap()
        .get(&session_id).cloned().unwrap_or_else(|| "/".to_string());

    // Inject fresh data from DataContext before rendering the initial snapshot.
    // In delta mode, on_sse_event bypasses V8, so V8's data can be stale.
    let data_json = app.data_ctx.as_ref()
        .map(|ctx| ctx.data_json_for_page(&path));

    let reply = Reply::new();
    let req = if let Some(dj) = data_json {
        V8Request::RenderWithData {
            path: path.clone(),
            session_id: session_id.clone(),
            data_json: dj,
            reply: reply.clone(),
        }
    } else {
        V8Request::Render { path: path.clone(), session_id: session_id.clone(), reply: reply.clone() }
    };
    if tx.send(req).is_err() {
        return stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n");
    }
    let dom_json = v8_result_to_json(reply.recv(), None);
    let snapshot = format!("{{\"root\":{}}}", dom_json);
    write_sse_event(&mut stream, snapshot.as_bytes())?;

    let client = stream.try_clone()?;
    {
        let mut clients = app.sse_clients.lock().unwrap();
        clients.entry(session_id.clone()).or_insert_with(Vec::new).push(client);
    }
    eprintln!("[platform:{}] SSE connected (session={}, path={})", app.name, &session_id[..8], path);

    loop {
        thread::sleep(std::time::Duration::from_secs(30));
        if stream.write_all(b": keepalive\n\n").is_err() {
            eprintln!("[platform:{}] SSE disconnected (session={})", app.name, &session_id[..8]);
            // Clean up this client
            let mut clients = app.sse_clients.lock().unwrap();
            if let Some(list) = clients.get_mut(&session_id) {
                list.retain(|mut c| c.write_all(b"").is_ok());
                if list.is_empty() {
                    clients.remove(&session_id);
                    // Drop session state in V8
                    if let Ok(tx) = app.ensure_warm() {
                        let _ = tx.send(V8Request::DropSession { session_id: session_id.clone() });
                    }
                    app.session_paths.lock().unwrap().remove(&session_id);
                }
            }
            break;
        }
    }
    Ok(())
}

fn handle_app_action(
    stream: &mut TcpStream,
    app: &AppHandle,
    url_path: &str,
    body: &[u8],
    extra_headers: &HashMap<String, String>,
    req_headers: &HashMap<String, String>,
) -> std::io::Result<()> {
    use crate::{extract_session_cookie};

    let action = urlencoding_decode(url_path.strip_prefix("/actions/").unwrap_or(""));
    let body_str = String::from_utf8_lossy(body);

    // Session ID from cookie (fall back to __default for cookieless requests)
    let session_id = extract_session_cookie(req_headers)
        .unwrap_or_else(|| "__default".to_string());

    let payload_str = if body_str.is_empty() { "{}".to_string() } else {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
            if let Some(p) = val.get("payload") { p.to_string() } else { val.to_string() }
        } else { "{}".to_string() }
    };

    let snapshot: String;

    app.touch();
    let tx = app.ensure_warm().map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    // Extract auth token from session (if auth middleware configured)
    let auth_token = app.auth.as_ref()
        .and_then(|auth| auth.get_access_token(req_headers));

    if action == "navigate" {
        let nav_path = serde_json::from_str::<serde_json::Value>(&payload_str)
            .ok()
            .and_then(|v| v.get("path")?.as_str().map(String::from))
            .unwrap_or_else(|| "/".to_string());

        eprintln!("[platform:{}] navigate → {} (session={})", app.name, nav_path, &session_id[..std::cmp::min(8, session_id.len())]);
        app.session_paths.lock().unwrap().insert(session_id.clone(), nav_path.clone());

        // On navigation, fetch page-scoped data sources for the new page
        if let Some(ref ctx) = app.data_ctx {
            fetch_page_data_with_token(ctx, &nav_path, auth_token.as_deref());
            let data_json = ctx.data_json_for_page(&nav_path);
            let reply = Reply::new();
            if tx.send(V8Request::RenderWithData {
                path: nav_path, data_json, session_id: session_id.clone(), reply: reply.clone(),
            }).is_err() {
                let msg = "{\"error\":\"V8 thread unavailable\"}";
                let resp = format!("HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", msg.len());
                stream.write_all(resp.as_bytes())?;
                return stream.write_all(msg.as_bytes());
            }
            let dom_json = v8_result_to_json(reply.recv(), None);
            snapshot = format!("{{\"root\":{}}}", dom_json);
        } else {
            let reply = Reply::new();
            if tx.send(V8Request::Render { path: nav_path, session_id: session_id.clone(), reply: reply.clone() }).is_err() {
                let msg = "{\"error\":\"V8 thread unavailable\"}";
                let resp = format!("HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", msg.len());
                stream.write_all(resp.as_bytes())?;
                return stream.write_all(msg.as_bytes());
            }
            let dom_json = v8_result_to_json(reply.recv(), None);
            snapshot = format!("{{\"root\":{}}}", dom_json);
        }
    } else {
        let path = app.session_paths.lock().unwrap()
            .get(&session_id).cloned().unwrap_or_else(|| "/".to_string());
        let payload_val: serde_json::Value = serde_json::from_str(&payload_str).unwrap_or_default();

        // Check if this action maps to an external API
        if let Some(ref ctx) = app.data_ctx {
            if let Some(mapping) = ctx.find_action(&action) {
                let mapping = mapping.clone();
                eprintln!("[platform:{}] external action '{}' → {} {}", app.name, action, mapping.method, mapping.url);

                // Forward to backend API
                match forward_action(&mapping, &payload_val) {
                    Ok(response_val) => {
                        // If action has a target, update that data source
                        if let Some(ref target) = mapping.target {
                            ctx.set_value(target, response_val);
                        }
                        // Re-fetch affected data sources for current page
                        fetch_page_data_with_token(ctx, &path, auth_token.as_deref());
                    }
                    Err(e) => {
                        eprintln!("[platform:{}] action forward error: {}", app.name, e);
                    }
                }

                // Render with updated data
                let data_json = ctx.data_json_for_page(&path);
                let reply = Reply::new();
                if tx.send(V8Request::RenderWithData {
                    path: path.clone(), data_json, session_id: session_id.clone(), reply: reply.clone(),
                }).is_err() {
                    let msg = "{\"error\":\"V8 thread unavailable\"}";
                    let resp = format!("HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", msg.len());
                    stream.write_all(resp.as_bytes())?;
                    return stream.write_all(msg.as_bytes());
                }
                let dom_json = v8_result_to_json(reply.recv(), Some(&action));
                snapshot = format!("{{\"root\":{}}}", dom_json);
            } else {
                // Not an external action — fall through to local reducer
                let reply = Reply::new();
                if tx.send(V8Request::Reduce {
                    action: action.clone(), payload: payload_str, path, session_id: session_id.clone(), reply: reply.clone(),
                }).is_err() {
                    let msg = "{\"error\":\"V8 thread unavailable\"}";
                    let resp = format!("HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", msg.len());
                    stream.write_all(resp.as_bytes())?;
                    return stream.write_all(msg.as_bytes());
                }
                let dom_json = v8_result_to_json(reply.recv(), Some(&action));
                snapshot = format!("{{\"root\":{}}}", dom_json);
            }
        } else {
            // No data layer — standard reducer path
            let reply = Reply::new();
            if tx.send(V8Request::Reduce {
                action: action.clone(), payload: payload_str, path, session_id: session_id.clone(), reply: reply.clone(),
            }).is_err() {
                let msg = "{\"error\":\"V8 thread unavailable\"}";
                let resp = format!("HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", msg.len());
                stream.write_all(resp.as_bytes())?;
                return stream.write_all(msg.as_bytes());
            }
            let dom_json = v8_result_to_json(reply.recv(), Some(&action));
            snapshot = format!("{{\"root\":{}}}", dom_json);
        }
    }

    let eh = format_extra_headers(extra_headers);
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
        Content-Length: {}\r\n{}\r\n",
        snapshot.len(), eh
    );
    stream.write_all(resp.as_bytes())?;
    stream.write_all(snapshot.as_bytes())?;

    // Broadcast only to this session's SSE clients (not all users)
    if action != "navigate" {
        let mut clients = app.sse_clients.lock().unwrap();
        if let Some(list) = clients.get_mut(&session_id) {
            let mut alive = Vec::new();
            for mut client in list.drain(..) {
                if write_sse_event(&mut client, snapshot.as_bytes()).is_ok() {
                    alive.push(client);
                }
            }
            if alive.is_empty() {
                clients.remove(&session_id);
            } else {
                *list = alive;
            }
        }
    }
    Ok(())
}

fn handle_app_api(
    stream: &mut TcpStream,
    app: &AppHandle,
    method: &str,
    path: &str,
    body: &[u8],
    extra_headers: &HashMap<String, String>,
) -> std::io::Result<()> {
    app.touch();
    let tx = app.ensure_warm().map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    let body_str = String::from_utf8_lossy(body).to_string();
    let reply = Reply::new();
    if tx.send(V8Request::ApiCall {
        method: method.to_string(),
        path: path.to_string(),
        body: body_str,
        reply: reply.clone(),
    }).is_err() {
        let msg = "{\"error\":\"V8 thread unavailable\"}";
        let resp = format!("HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", msg.len());
        stream.write_all(resp.as_bytes())?;
        return stream.write_all(msg.as_bytes());
    }

    let (status, response_body) = match reply.recv() {
        V8Result::Ok(json) => {
            // Check for __status and __error in response
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json) {
                let status = val.get("__status")
                    .and_then(|s| s.as_u64())
                    .unwrap_or(200) as u16;
                if val.get("__error").is_some() {
                    let err_msg = val.get("__error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                    (status, format!("{{\"error\":\"{}\"}}", err_msg))
                } else {
                    (200, json)
                }
            } else {
                (200, json)
            }
        }
        V8Result::Err(e) => {
            (500, format!("{{\"error\":\"{}\"}}", e))
        }
    };

    let status_line = status_text(status);
    let eh = format_extra_headers(extra_headers);
    let resp = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\n\
        Content-Length: {}\r\n{}\r\n",
        status, status_line, response_body.len(), eh
    );
    stream.write_all(resp.as_bytes())?;
    stream.write_all(response_body.as_bytes())
}

fn handle_app_get(
    stream: &mut TcpStream,
    app: Arc<AppHandle>,
    app_name: &str,
    path: &str,
    extra_headers: &HashMap<String, String>,
    via_subdomain: bool,
    req_headers: &HashMap<String, String>,
) -> std::io::Result<()> {
    // Static files
    let has_ext = path.contains('.') && !path.ends_with('/');
    let ext = path.rsplit('.').next().unwrap_or("");
    if has_ext && ext != "html" {
        let filename = path.trim_start_matches('/');

        // Embedded framework assets — served from binary, never from disk
        if let Some(result) = serve_embedded(stream, filename, extra_headers) {
            return result;
        }

        let file_path = {
            let hashed = std::path::Path::new(&app.asset_dir).join(filename);
            if hashed.exists() { hashed }
            else { std::path::Path::new(&app.static_dir).join(filename) }
        };

        let data = match std::fs::read(&file_path) {
            Ok(d) => d,
            Err(_) => {
                return stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
            }
        };

        let ct = guess_content_type(path);
        let is_hashed = app.manifest.reverse.contains_key(filename)
            && app.manifest.reverse.get(filename).map(|o| o != filename).unwrap_or(false);
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
        return stream.write_all(&data);
    }

    // SSR
    app.touch();
    let tx = app.ensure_warm().map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    let route_path = path.split('?').next().unwrap_or("/");
    let (session_id, is_new) = match crate::extract_session_cookie(req_headers) {
        Some(sid) => (sid, false),
        None => (crate::generate_session_id(), true),
    };
    app.session_paths.lock().unwrap().insert(session_id.clone(), route_path.to_string());

    // Extract auth token from session (if auth middleware configured)
    let auth_token = app.auth.as_ref()
        .and_then(|auth| auth.get_access_token(req_headers));

    // Fetch page-scoped data sources (with streaming timeout support)
    // Use CSS-aware render variants for SSR (RenderWithCSS / RenderWithDataAndCSS)
    // The deferred SSE update path still uses RenderWithData (bare DomNode) — unaffected
    let reply = Reply::new();
    let mut pending_sources: Vec<crate::data::DataSourceConfig> = Vec::new();
    if let Some(ref ctx) = app.data_ctx {
        pending_sources = fetch_page_data_streaming(ctx, route_path, auth_token.as_deref());
        let data_json = ctx.data_json_for_page(route_path);
        if tx.send(V8Request::RenderWithDataAndCSS {
            path: route_path.to_string(), data_json, session_id: session_id.clone(), reply: reply.clone(),
        }).is_err() {
            let msg = "<html><body><h1>503 — V8 thread unavailable</h1></body></html>";
            let resp = format!("HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n", msg.len());
            stream.write_all(resp.as_bytes())?;
            return stream.write_all(msg.as_bytes());
        }
    } else {
        if tx.send(V8Request::RenderWithCSS {
            path: route_path.to_string(), session_id: session_id.clone(), reply: reply.clone(),
        }).is_err() {
            let msg = "<html><body><h1>503 — V8 thread unavailable</h1></body></html>";
            let resp = format!("HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n", msg.len());
            stream.write_all(resp.as_bytes())?;
            return stream.write_all(msg.as_bytes());
        }
    }

    // If sources timed out, complete fetch in background and push update via SSE
    if !pending_sources.is_empty() {
        let deferred_app = Arc::clone(&app);
        let route = route_path.to_string();
        let token = auth_token.clone();
        let deferred_sid = session_id.clone();
        thread::spawn(move || {
            if let Some(ref ctx) = deferred_app.data_ctx {
                for source in &pending_sources {
                    match fetch_data_source(source, token.as_deref()) {
                        Ok(value) => ctx.set_value(&source.key, value),
                        Err(e) => {
                            eprintln!("[data] deferred fetch error: {}", e);
                            ctx.set_value(&source.key, serde_json::json!({ "__error": e }));
                        }
                    }
                }
                // Clear loading flag
                ctx.values.write().unwrap().remove("__loading");
                // Re-render with complete data and push via SSE
                if let Ok(tx) = deferred_app.ensure_warm() {
                    let data_json = ctx.data_json_for_page(&route);
                    let reply = Reply::new();
                    if tx.send(V8Request::RenderWithData {
                        path: route, data_json, session_id: deferred_sid.clone(), reply: reply.clone(),
                    }).is_ok() {
                        let dom_json = v8_result_to_json(reply.recv(), None);
                        let snapshot = format!("{{\"root\":{}}}", dom_json);
                        // Push SSE update to the session that triggered this render
                        let mut clients = deferred_app.sse_clients.lock().unwrap();
                        if let Some(list) = clients.get_mut(&deferred_sid) {
                            let mut alive = Vec::new();
                            for mut client in list.drain(..) {
                                if write_sse_event(&mut client, snapshot.as_bytes()).is_ok() {
                                    alive.push(client);
                                }
                            }
                            if alive.is_empty() { clients.remove(&deferred_sid); }
                            else { *list = alive; }
                        }
                        eprintln!("[data] deferred data ready, pushed SSE update (session={})", &deferred_sid[..8]);
                    }
                }
            }
        });
    }

    // Parse {root: DomNode, css: string|null} from renderWithCSS result
    let (dom, generated_css) = match reply.recv() {
        V8Result::Ok(json) => {
            match serde_json::from_str::<serde_json::Value>(&json) {
                Ok(wrapper) => {
                    let root_val = wrapper.get("root").cloned().unwrap_or(serde_json::Value::Null);
                    let css_val = wrapper.get("css").and_then(|v| v.as_str()).map(String::from);
                    match serde_json::from_value::<DomNode>(root_val) {
                        Ok(d) => (d, css_val),
                        Err(e) => {
                            eprintln!("[platform:{}] render parse error: {}", app_name, e);
                            (error_fallback(&format!("JSON parse error: {}", e), None), None)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[platform:{}] render parse error: {}", app_name, e);
                    (error_fallback(&format!("JSON parse error: {}", e), None), None)
                }
            }
        }
        V8Result::Err(e) => {
            eprintln!("[platform:{}] render error: {}", app_name, e);
            (error_fallback(&e, None), None)
        }
    };

    // Merge CSS: generated CSS from design.json + user's style.css (if any)
    let merged_css = match (&generated_css, &app.inline_css) {
        (Some(gen), Some(user)) => Some(format!("{}{}", gen, user)),
        (Some(gen), None) => Some(gen.clone()),
        (None, Some(user)) => Some(user.clone()),
        (None, None) => None,
    };

    // When accessed via subdomain, emit root-relative paths (Caddy rewrites
    // the URL so the platform sees /apps/{name}/... but the browser is at /).
    // When accessed directly via /apps/{name}/, use the prefixed paths.
    let prefix = if via_subdomain {
        String::new() // root-relative: /magnetic.js, /sse, etc.
    } else {
        format!("/apps/{}", app_name) // path-prefixed: /apps/{name}/magnetic.js
    };
    let magnetic_js = format!("{}/magnetic.js", prefix);
    let wasm_url = Some(format!("{}/transport.wasm", prefix));

    // Load client-side renderer scripts for delta mode (if present)
    let mut inline_scripts = Vec::new();
    if app.data_ctx.is_some() {
        let renderer_path = format!("{}/{}/public/renderer.js", app.data_dir, app_name);
        if let Ok(script) = std::fs::read_to_string(&renderer_path) {
            inline_scripts.push(script);
        }
    }

    let page = render_page(&PageOptions {
        root: dom,
        scripts: vec![magnetic_js],
        styles: vec![],
        inline_css: merged_css,
        sse_url: Some(format!("{}/sse", prefix)),
        mount_selector: Some("#app".to_string()),
        wasm_url,
        title: Some(format!("{} | Magnetic", app_name)),
        description: Some("Server-driven UI — Magnetic Platform".to_string()),
        inline_scripts,
    });

    let eh = format_extra_headers(extra_headers);
    let cookie_header = if is_new {
        format!("Set-Cookie: magnetic_sid={}; Path=/; HttpOnly; SameSite=Lax\r\n", session_id)
    } else {
        String::new()
    };
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n{}{}\r\n",
        page.len(), cookie_header, eh
    );
    stream.write_all(resp.as_bytes())?;
    stream.write_all(page.as_bytes())
}


