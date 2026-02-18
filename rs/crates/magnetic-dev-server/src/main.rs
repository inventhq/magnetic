use magnetic_reducer_core::{AppState, Buf, process, render, render_html};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

/// Shared server state.
struct Server {
    app: Mutex<AppState>,
    buf: Mutex<Buf>,
    /// SSE clients: Vec of TcpStream clones (we write snapshots to each).
    sse_clients: Mutex<Vec<TcpStream>>,
    /// Static files directory (demo/).
    static_dir: String,
    /// Path to the WASM file.
    wasm_path: Option<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port = find_arg(&args, "--port").unwrap_or_else(|| "3000".to_string());
    let static_dir = find_arg(&args, "--demo").unwrap_or_else(|| "demo".to_string());
    let wasm_path = find_arg(&args, "--wasm");

    let server = Arc::new(Server {
        app: Mutex::new(AppState::new()),
        buf: Mutex::new(Buf::new()),
        sse_clients: Mutex::new(Vec::new()),
        static_dir,
        wasm_path,
    });

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    eprintln!("[magnetic-dev-server] listening on http://localhost:{}", port);

    // Send initial snapshot to newly connected SSE clients
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[err] accept: {}", e);
                continue;
            }
        };
        let server = Arc::clone(&server);
        thread::spawn(move || {
            if let Err(e) = handle_connection(stream, &server) {
                // Connection closed or error — expected for SSE clients
                let _ = e;
            }
        });
    }
}

fn find_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn handle_connection(mut stream: TcpStream, server: &Server) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }
    let method = parts[0];
    let path = parts[1];

    // Read all headers
    let mut headers = HashMap::new();
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim().to_lowercase();
            let val = v.trim().to_string();
            if key == "content-length" {
                content_length = val.parse().unwrap_or(0);
            }
            headers.insert(key, val);
        }
    }

    match (method, path) {
        ("GET", "/sse") => handle_sse(stream, server),
        ("POST", p) if p.starts_with("/actions/") => {
            let mut body = vec![0u8; content_length];
            reader.read_exact(&mut body)?;
            handle_action(&mut stream, server, p, &body)
        }
        ("GET", p) => handle_static(&mut stream, server, p),
        ("OPTIONS", _) => {
            // CORS preflight
            let resp = "HTTP/1.1 204 No Content\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Access-Control-Allow-Headers: Content-Type, Idempotency-Key\r\n\
                Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                \r\n";
            stream.write_all(resp.as_bytes())
        }
        _ => {
            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
            stream.write_all(resp.as_bytes())
        }
    }
}

fn handle_sse(mut stream: TcpStream, server: &Server) -> std::io::Result<()> {
    // Send SSE headers
    let headers = "HTTP/1.1 200 OK\r\n\
        Content-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\n\
        Connection: keep-alive\r\n\
        Access-Control-Allow-Origin: *\r\n\
        \r\n";
    stream.write_all(headers.as_bytes())?;

    // Send initial snapshot
    {
        let state = server.app.lock().unwrap();
        let mut buf = server.buf.lock().unwrap();
        render(&state, &mut buf);
        write_sse_event(&mut stream, buf.as_bytes())?;
    }

    // Register this client for future updates
    let client = stream.try_clone()?;
    server.sse_clients.lock().unwrap().push(client);

    // Keep the connection alive — block this thread
    // (SSE is long-lived; we detect close when broadcast fails)
    loop {
        thread::sleep(std::time::Duration::from_secs(30));
        // Send keepalive comment
        if stream.write_all(b": keepalive\n\n").is_err() {
            break;
        }
    }
    Ok(())
}

fn handle_action(
    stream: &mut TcpStream,
    server: &Server,
    _path: &str,
    body: &[u8],
) -> std::io::Result<()> {
    // Process action through reducer
    let snapshot_bytes: Vec<u8> = {
        let mut state = server.app.lock().unwrap();
        let mut buf = server.buf.lock().unwrap();
        process(&mut state, body, &mut buf);
        buf.as_bytes().to_vec()
    };

    // Return snapshot directly in POST response (single round-trip)
    let resp = format!(
        "HTTP/1.1 200 OK\r\n\
        Content-Type: application/json\r\n\
        Content-Length: {}\r\n\
        Access-Control-Allow-Origin: *\r\n\
        Access-Control-Allow-Headers: Content-Type\r\n\
        \r\n",
        snapshot_bytes.len()
    );
    stream.write_all(resp.as_bytes())?;
    stream.write_all(&snapshot_bytes)?;

    // Broadcast to other SSE clients (multi-client sync)
    broadcast_snapshot(server, &snapshot_bytes);

    Ok(())
}

fn broadcast_snapshot(server: &Server, snapshot: &[u8]) {
    let mut clients = server.sse_clients.lock().unwrap();
    let mut alive = Vec::new();
    for mut client in clients.drain(..) {
        if write_sse_event(&mut client, snapshot).is_ok() {
            alive.push(client);
        }
        // Dead clients are dropped
    }
    *clients = alive;
}

fn write_sse_event(stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    stream.write_all(b"event: message\ndata: ")?;
    stream.write_all(data)?;
    stream.write_all(b"\n\n")?;
    stream.flush()
}

fn handle_static(
    stream: &mut TcpStream,
    server: &Server,
    path: &str,
) -> std::io::Result<()> {
    // Serve WASM file
    if path == "/magnetic-reducer.wasm" {
        if let Some(ref wasm_path) = server.wasm_path {
            return serve_file(stream, wasm_path, "application/wasm");
        }
    }

    // SSR: for index.html, inject pre-rendered HTML into the page
    if path == "/" || path == "/index.html" {
        return serve_ssr_page(stream, server);
    }

    let file_path = format!("{}{}", server.static_dir, path);

    // Security: prevent path traversal
    let canonical = std::fs::canonicalize(&file_path);
    let base = std::fs::canonicalize(&server.static_dir);
    if let (Ok(canon), Ok(base_canon)) = (canonical, base) {
        if !canon.starts_with(&base_canon) {
            let resp = "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n";
            return stream.write_all(resp.as_bytes());
        }
    }

    let content_type = guess_content_type(&file_path);
    serve_file(stream, &file_path, content_type)
}

/// Serve index.html with SSR-injected HTML from current app state.
/// Replaces <!--SSR--> marker with pre-rendered HTML.
fn serve_ssr_page(stream: &mut TcpStream, server: &Server) -> std::io::Result<()> {
    let template_path = format!("{}/index.html", server.static_dir);
    let template = match std::fs::read_to_string(&template_path) {
        Ok(t) => t,
        Err(_) => {
            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
            return stream.write_all(resp.as_bytes());
        }
    };

    // Render current state as HTML
    let ssr_html = {
        let state = server.app.lock().unwrap();
        let mut buf = server.buf.lock().unwrap();
        render_html(&state, &mut buf);
        String::from_utf8_lossy(buf.as_bytes()).into_owned()
    };

    // Replace the SSR marker with pre-rendered HTML
    let page = template.replace("<!--SSR-->", &ssr_html);

    let resp = format!(
        "HTTP/1.1 200 OK\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n\
        Access-Control-Allow-Origin: *\r\n\
        \r\n",
        page.len()
    );
    stream.write_all(resp.as_bytes())?;
    stream.write_all(page.as_bytes())
}

fn serve_file(stream: &mut TcpStream, path: &str, content_type: &str) -> std::io::Result<()> {
    match std::fs::read(path) {
        Ok(data) => {
            let resp = format!(
                "HTTP/1.1 200 OK\r\n\
                Content-Type: {}\r\n\
                Content-Length: {}\r\n\
                Access-Control-Allow-Origin: *\r\n\
                \r\n",
                content_type,
                data.len()
            );
            stream.write_all(resp.as_bytes())?;
            stream.write_all(&data)
        }
        Err(_) => {
            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
            stream.write_all(resp.as_bytes())
        }
    }
}

fn guess_content_type(path: &str) -> &str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".wasm") {
        "application/wasm"
    } else {
        "application/octet-stream"
    }
}
