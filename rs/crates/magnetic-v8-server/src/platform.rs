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
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::Instant;

use magnetic_dom::DomNode;
use magnetic_render_html::{render_page, PageOptions};

use crate::{
    V8Request, V8Result, Reply, AssetManifest,
    MagneticContext, MiddlewareStack, MiddlewareFn,
    v8_thread, v8_result_to_json, error_fallback,
    write_sse_event, broadcast_snapshot, guess_content_type,
    format_extra_headers, status_text, urlencoding_decode,
    cors_middleware, rate_limit_middleware, logger_middleware,
    build_assets, find_arg,
};

// ── Per-app handle ──────────────────────────────────────────────────

struct AppHandle {
    name: String,
    v8_tx: mpsc::Sender<V8Request>,
    sse_clients: Mutex<Vec<TcpStream>>,
    current_path: Mutex<String>,
    static_dir: String,
    asset_dir: String,
    inline_css: Option<String>,
    manifest: AssetManifest,
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

    // Build middleware
    let mut middleware = MiddlewareStack::new();
    middleware.add(logger_middleware());
    middleware.add(cors_middleware(&cors_origin));
    middleware.add(rate_limit_middleware(60_000, rate_limit_max));

    let platform = Arc::new(Platform {
        apps: RwLock::new(HashMap::new()),
        data_dir: data_dir.clone(),
        middleware,
    });

    // Load existing apps from data directory
    if let Ok(entries) = std::fs::read_dir(&data_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().to_string();
                let bundle_path = entry.path().join("bundle.js");
                if bundle_path.exists() {
                    match load_app(&name, &data_dir) {
                        Ok(handle) => {
                            eprintln!("[platform] Loaded app: {}", name);
                            platform.apps.write().unwrap().insert(name, Arc::new(handle));
                        }
                        Err(e) => eprintln!("[platform] Failed to load {}: {}", name, e),
                    }
                }
            }
        }
    }

    let app_count = platform.apps.read().unwrap().len();
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    eprintln!("[platform] http://localhost:{}", port);
    eprintln!("[platform] Magnetic Platform Server — multi-tenant V8 hosting");
    eprintln!("[platform] Data dir: {}", data_dir);
    eprintln!("[platform] Apps loaded: {}", app_count);
    eprintln!("[platform] Deploy: POST /api/apps/<name>/deploy");
    eprintln!("[platform] Access: GET /apps/<name>/");

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
    let public_dir = format!("{}/public", app_dir);

    let js_source = std::fs::read_to_string(&bundle_path)
        .map_err(|e| format!("Cannot read bundle: {}", e))?;

    // Start V8 thread for this app
    let (tx, rx) = mpsc::channel();
    let js = js_source;
    thread::spawn(move || v8_thread(js, rx));

    // Build asset pipeline
    let asset_dir = format!("{}/.hashed", public_dir);
    let manifest = build_assets(
        &public_dir, &asset_dir,
        &["magnetic.js", "transport.wasm", "index.html"],
    );

    // Load CSS
    let css_path = manifest.files.get("style.css")
        .map(|h| format!("{}/{}", asset_dir, h))
        .unwrap_or_else(|| format!("{}/style.css", public_dir));
    let inline_css = std::fs::read_to_string(&css_path).ok();

    Ok(AppHandle {
        name: name.to_string(),
        v8_tx: tx,
        sse_clients: Mutex::new(Vec::new()),
        current_path: Mutex::new("/".to_string()),
        static_dir: public_dir,
        asset_dir,
        inline_css,
        manifest,
    })
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
                ("GET", "/sse") => {
                    return handle_app_sse(stream, &app, &extra_headers);
                }
                ("POST", p) if p.starts_with("/actions/") => {
                    let mut body = vec![0u8; content_length];
                    if content_length > 0 { reader.read_exact(&mut body)?; }
                    let result = handle_app_action(
                        &mut stream, &app, p, &body, &extra_headers,
                    );
                    let ms = log_start.elapsed().as_millis();
                    eprintln!("[platform] {} /apps/{}{} → ({}ms)", method, app_name, p, ms);
                    return result;
                }
                ("GET", p) => {
                    let result = handle_app_get(
                        &mut stream, &app, app_name, p, &extra_headers,
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
            let mut apps = platform.apps.write().unwrap();
            // Old app handle is dropped, V8 thread will exit when channel closes
            apps.insert(name.clone(), Arc::new(handle));
            drop(apps);

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
) -> std::io::Result<()> {
    let eh = format_extra_headers(extra_headers);
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\nConnection: keep-alive\r\n{}\r\n", eh
    );
    stream.write_all(header.as_bytes())?;

    let path = app.current_path.lock().unwrap().clone();
    let reply = Reply::new();
    app.v8_tx.send(V8Request::Render { path, reply: reply.clone() }).unwrap();
    let dom_json = v8_result_to_json(reply.recv(), None);
    let snapshot = format!("{{\"root\":{}}}", dom_json);
    write_sse_event(&mut stream, snapshot.as_bytes())?;

    let client = stream.try_clone()?;
    app.sse_clients.lock().unwrap().push(client);

    loop {
        thread::sleep(std::time::Duration::from_secs(30));
        if stream.write_all(b": keepalive\n\n").is_err() { break; }
    }
    Ok(())
}

fn handle_app_action(
    stream: &mut TcpStream,
    app: &AppHandle,
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

        *app.current_path.lock().unwrap() = nav_path.clone();
        let reply = Reply::new();
        app.v8_tx.send(V8Request::Render { path: nav_path, reply: reply.clone() }).unwrap();
        let dom_json = v8_result_to_json(reply.recv(), None);
        snapshot = format!("{{\"root\":{}}}", dom_json);
    } else {
        let path = app.current_path.lock().unwrap().clone();
        let reply = Reply::new();
        app.v8_tx.send(V8Request::Reduce {
            action: action.clone(), payload, path, reply: reply.clone(),
        }).unwrap();
        let dom_json = v8_result_to_json(reply.recv(), Some(&action));
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
        let mut clients = app.sse_clients.lock().unwrap();
        let mut alive = Vec::new();
        for mut client in clients.drain(..) {
            if write_sse_event(&mut client, snapshot.as_bytes()).is_ok() {
                alive.push(client);
            }
        }
        *clients = alive;
    }
    Ok(())
}

fn handle_app_get(
    stream: &mut TcpStream,
    app: &AppHandle,
    app_name: &str,
    path: &str,
    extra_headers: &HashMap<String, String>,
) -> std::io::Result<()> {
    // Static files
    let has_ext = path.contains('.') && !path.ends_with('/');
    let ext = path.rsplit('.').next().unwrap_or("");
    if has_ext && ext != "html" {
        let filename = path.trim_start_matches('/');
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
    let route_path = path.split('?').next().unwrap_or("/");
    *app.current_path.lock().unwrap() = route_path.to_string();

    let reply = Reply::new();
    app.v8_tx.send(V8Request::Render {
        path: route_path.to_string(), reply: reply.clone(),
    }).unwrap();

    let dom = match reply.recv() {
        V8Result::Ok(json) => {
            match serde_json::from_str::<DomNode>(&json) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("[platform:{}] render parse error: {}", app_name, e);
                    error_fallback(&format!("JSON parse error: {}", e), None)
                }
            }
        }
        V8Result::Err(e) => {
            eprintln!("[platform:{}] render error: {}", app_name, e);
            error_fallback(&e, None)
        }
    };

    // Resolve asset URLs — prefix with /apps/<name>/
    let app_prefix = format!("/apps/{}", app_name);
    let magnetic_js = resolve_app_asset(app, &app_prefix, "magnetic.js");
    let wasm_url = if app.manifest.files.contains_key("transport.wasm") {
        Some(resolve_app_asset(app, &app_prefix, "transport.wasm"))
    } else {
        None
    };

    let page = render_page(&PageOptions {
        root: dom,
        scripts: vec![magnetic_js],
        styles: vec![],
        inline_css: app.inline_css.clone(),
        sse_url: Some(format!("{}/sse", app_prefix)),
        mount_selector: Some("#app".to_string()),
        wasm_url,
        title: Some(format!("{} | Magnetic", app_name)),
        description: Some("Server-driven UI — Magnetic Platform".to_string()),
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

fn resolve_app_asset(app: &AppHandle, prefix: &str, filename: &str) -> String {
    if let Some(hashed) = app.manifest.files.get(filename) {
        format!("{}/{}", prefix, hashed)
    } else {
        format!("{}/{}", prefix, filename)
    }
}
