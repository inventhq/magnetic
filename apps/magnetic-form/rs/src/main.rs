use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::fmt::Write as FmtWrite;

// ---------------------------------------------------------------------------
// Form state
// ---------------------------------------------------------------------------

struct FormState {
    error_name: bool,
    error_email: bool,
    error_message: bool,
    submitted: bool,
    submitting: bool,
    // Track shake generation to let client detect NEW errors vs existing
    shake_gen: u32,
}

impl FormState {
    fn new() -> Self {
        Self {
            error_name: false, error_email: false, error_message: false,
            submitted: false, submitting: false, shake_gen: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// JSON DOM snapshot renderer
// ---------------------------------------------------------------------------

fn render_snapshot(state: &FormState) -> String {
    let mut s = String::with_capacity(4096);

    // Root card
    s.push_str(r#"{"root":{"tag":"div","key":"form-card","attrs":{"class":"form-card"},"children":["#);

    // ── Header ──
    s.push_str(r#"{"tag":"div","key":"header","attrs":{"class":"form-header"},"children":["#);
    s.push_str(r#"{"tag":"h1","text":"Get in touch"},"#);
    s.push_str(r#"{"tag":"p","attrs":{"class":"form-subtitle"},"text":"We\u2019d love to hear from you. Send us a message."}"#);
    s.push_str("]}"); // close header

    // ── Form ──
    s.push_str(r#",{"tag":"form","key":"form","attrs":{"class":"contact-form","novalidate":"true"},"events":{"submit":"on_submit"},"children":["#);

    // Name field
    render_field(&mut s, "name", "text", "Full name", "Name is required",
        state.error_name, false, state.shake_gen);
    s.push(',');

    // Email field
    render_field(&mut s, "email", "email", "Email address", "Valid email is required",
        state.error_email, false, state.shake_gen);
    s.push(',');

    // Message field
    render_field(&mut s, "message", "textarea", "Your message", "Message is required",
        state.error_message, true, state.shake_gen);

    // Submit button
    let btn_class = if state.submitting { "submit-btn loading" } else { "submit-btn" };
    let btn_text = if state.submitting { "Sending…" } else { "Send message" };
    write!(s, r#",{{"tag":"button","key":"btn","attrs":{{"type":"submit","class":"{}"}},"children":["#, btn_class).unwrap();
    write!(s, r#"{{"tag":"span","key":"btn-text","text":"{}"}}"#, btn_text).unwrap();
    s.push_str("]}"); // close button

    s.push_str("]}"); // close form

    // ── Success overlay ──
    let overlay_class = if state.submitted { "success-overlay visible" } else { "success-overlay" };
    write!(s, r#",{{"tag":"div","key":"success","attrs":{{"class":"{}","data-shake-gen":"{}"}},"children":["#, overlay_class, state.shake_gen).unwrap();
    s.push_str(r#"{"tag":"div","key":"check","attrs":{"class":"success-check"},"children":["#);
    s.push_str(r#"{"tag":"span","text":"\u2713"}"#);
    s.push_str("]}"); // close check
    s.push_str(r#",{"tag":"h2","key":"succ-h","text":"Message sent!"}"#);
    s.push_str(r#",{"tag":"p","key":"succ-p","attrs":{"class":"success-sub"},"text":"We\u2019ll get back to you shortly."}"#);
    s.push_str("]}"); // close success overlay

    s.push_str("]}}"); // close form-card + root
    s
}

fn render_field(s: &mut String, name: &str, input_type: &str, label: &str,
    error_msg: &str, has_error: bool, is_textarea: bool, shake_gen: u32)
{
    let mut cls = String::from("field");
    if is_textarea { cls.push_str(" is-textarea"); }
    if has_error { cls.push_str(" has-error"); }

    write!(s, r#"{{"tag":"div","key":"f-{}","attrs":{{"class":"{}","data-shake-gen":"{}"}},"children":["#,
        name, cls, shake_gen).unwrap();

    if is_textarea {
        write!(s, r#"{{"tag":"textarea","key":"i-{}","attrs":{{"name":"{}","placeholder":" ","id":"{}","rows":"4"}},"events":{{"input":"on_input_{}"}}}},"#,
            name, name, name, name).unwrap();
    } else {
        write!(s, r#"{{"tag":"input","key":"i-{}","attrs":{{"type":"{}","name":"{}","placeholder":" ","id":"{}","autocomplete":"{}"}},"events":{{"input":"on_input_{}"}}}},"#,
            name, input_type, name, name, name, name).unwrap();
    }

    write!(s, r#"{{"tag":"label","attrs":{{"for":"{}"}},"text":"{}"}}"#, name, label).unwrap();

    let err_class = if has_error { "error-text visible" } else { "error-text" };
    write!(s, r#",{{"tag":"div","key":"e-{}","attrs":{{"class":"{}"}},"text":"{}"}}"#,
        name, err_class, error_msg).unwrap();

    s.push_str("]}"); // close field
}

// ---------------------------------------------------------------------------
// SSR HTML renderer
// ---------------------------------------------------------------------------

fn render_ssr_html(state: &FormState) -> String {
    // For SSR, just render the initial form structure as HTML
    let mut s = String::with_capacity(2048);
    s.push_str(r#"<div class="form-card" data-key="form-card">"#);
    s.push_str(r#"<div class="form-header" data-key="header">"#);
    s.push_str(r#"<h1>Get in touch</h1>"#);
    s.push_str(r#"<p class="form-subtitle">We'd love to hear from you. Send us a message.</p>"#);
    s.push_str("</div>");
    s.push_str(r#"<form class="contact-form" data-key="form" data-a_submit="on_submit" novalidate>"#);

    render_field_html(&mut s, "name", "text", "Full name", "Name is required", false);
    render_field_html(&mut s, "email", "email", "Email address", "Valid email is required", false);
    render_field_html(&mut s, "message", "textarea", "Your message", "Message is required", true);

    s.push_str(r#"<button type="submit" class="submit-btn" data-key="btn"><span data-key="btn-text">Send message</span></button>"#);
    s.push_str("</form>");
    s.push_str(r#"<div class="success-overlay" data-key="success">"#);
    s.push_str(r#"<div class="success-check" data-key="check"><span>✓</span></div>"#);
    s.push_str(r#"<h2 data-key="succ-h">Message sent!</h2>"#);
    s.push_str(r#"<p class="success-sub" data-key="succ-p">We'll get back to you shortly.</p>"#);
    s.push_str("</div></div>");
    s
}

fn render_field_html(s: &mut String, name: &str, input_type: &str, label: &str,
    error_msg: &str, is_textarea: bool)
{
    let cls = if is_textarea { "field is-textarea" } else { "field" };
    write!(s, r#"<div class="{}" data-key="f-{}" data-shake-gen="0">"#, cls, name).unwrap();
    if is_textarea {
        write!(s, r#"<textarea name="{}" placeholder=" " id="{}" rows="4" data-key="i-{}" data-a_input="on_input_{}"></textarea>"#,
            name, name, name, name).unwrap();
    } else {
        write!(s, r#"<input type="{}" name="{}" placeholder=" " id="{}" autocomplete="{}" data-key="i-{}" data-a_input="on_input_{}">"#,
            input_type, name, name, name, name, name).unwrap();
    }
    write!(s, r#"<label for="{}">{}</label>"#, name, label).unwrap();
    write!(s, r#"<div class="error-text" data-key="e-{}">{}</div>"#, name, error_msg).unwrap();
    s.push_str("</div>");
}

// ---------------------------------------------------------------------------
// Action processing
// ---------------------------------------------------------------------------

fn process_action(state: &mut FormState, body: &[u8]) -> String {
    let text = std::str::from_utf8(body).unwrap_or("");
    if let Some(action) = extract_json_str(text, "action") {
        match action {
            "on_submit" => {
                let name = extract_json_str(text, "name").unwrap_or("");
                let email = extract_json_str(text, "email").unwrap_or("");
                let message = extract_json_str(text, "message").unwrap_or("");

                state.error_name = name.trim().is_empty();
                state.error_email = email.trim().is_empty() || !email.contains('@');
                state.error_message = message.trim().is_empty();
                state.shake_gen += 1;

                if !state.error_name && !state.error_email && !state.error_message {
                    state.submitted = true;
                }
            }
            "reset" => {
                *state = FormState::new();
            }
            _ => {} // on_input_* actions — no server-side state change needed
        }
    }
    render_snapshot(state)
}

fn extract_json_str<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\"", key);
    let pos = text.find(&needle)?;
    let after_key = pos + needle.len();
    let rest = &text[after_key..];
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    if after_colon.starts_with('"') {
        let start = 1;
        let end = after_colon[start..].find('"')?;
        Some(&after_colon[start..start + end])
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// HTTP server (same pattern as infinite-scroll)
// ---------------------------------------------------------------------------

struct Server {
    app: Mutex<FormState>,
    sse_clients: Mutex<Vec<TcpStream>>,
    static_dir: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port = find_arg(&args, "--port").unwrap_or("3002".into());
    let static_dir = find_arg(&args, "--public").unwrap_or("public".into());

    let server = Arc::new(Server {
        app: Mutex::new(FormState::new()),
        sse_clients: Mutex::new(Vec::new()),
        static_dir,
    });

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    eprintln!("[magnetic-form] http://localhost:{}", port);

    for stream in listener.incoming() {
        let stream = match stream { Ok(s) => s, Err(_) => continue };
        let server = Arc::clone(&server);
        thread::spawn(move || { let _ = handle(stream, &server); });
    }
}

fn handle(mut stream: TcpStream, server: &Server) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 { return Ok(()); }
    let method = parts[0];
    let path = parts[1];

    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line.trim().is_empty() { break; }
        if let Some((k, v)) = line.trim().split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
    }

    match (method, path) {
        ("GET", "/sse") => handle_sse(stream, server),
        ("POST", p) if p.starts_with("/actions/") => {
            let mut body = vec![0u8; content_length];
            reader.read_exact(&mut body)?;
            handle_action(&mut stream, server, &body)
        }
        ("GET", "/") | ("GET", "/index.html") => serve_ssr(&mut stream, server),
        ("GET", p) => serve_static(&mut stream, &server.static_dir, p),
        ("OPTIONS", _) => {
            stream.write_all(b"HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Content-Type\r\nAccess-Control-Allow-Methods: GET,POST,OPTIONS\r\n\r\n")
        }
        _ => stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"),
    }
}

fn handle_sse(mut stream: TcpStream, server: &Server) -> std::io::Result<()> {
    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n")?;
    let snap = { let state = server.app.lock().unwrap(); render_snapshot(&state) };
    write_sse(&mut stream, snap.as_bytes())?;
    let client = stream.try_clone()?;
    server.sse_clients.lock().unwrap().push(client);
    loop {
        thread::sleep(std::time::Duration::from_secs(30));
        if stream.write_all(b": keepalive\n\n").is_err() { break; }
    }
    Ok(())
}

fn handle_action(stream: &mut TcpStream, server: &Server, body: &[u8]) -> std::io::Result<()> {
    let snapshot = {
        let mut state = server.app.lock().unwrap();
        process_action(&mut state, body)
    };
    let snap_bytes = snapshot.as_bytes();
    let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Content-Type\r\n\r\n", snap_bytes.len());
    stream.write_all(resp.as_bytes())?;
    stream.write_all(snap_bytes)?;
    broadcast(server, snap_bytes);
    Ok(())
}

fn broadcast(server: &Server, data: &[u8]) {
    let mut clients = server.sse_clients.lock().unwrap();
    let mut alive = Vec::new();
    for mut c in clients.drain(..) {
        if write_sse(&mut c, data).is_ok() { alive.push(c); }
    }
    *clients = alive;
}

fn write_sse(s: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    s.write_all(b"event: message\ndata: ")?;
    s.write_all(data)?;
    s.write_all(b"\n\n")?;
    s.flush()
}

fn serve_ssr(stream: &mut TcpStream, server: &Server) -> std::io::Result<()> {
    let tpl = match std::fs::read_to_string(format!("{}/index.html", server.static_dir)) {
        Ok(t) => t,
        Err(_) => return stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"),
    };
    let html = { let state = server.app.lock().unwrap(); render_ssr_html(&state) };
    let page = tpl.replace("<!--SSR-->", &html);
    let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n", page.len());
    stream.write_all(resp.as_bytes())?;
    stream.write_all(page.as_bytes())
}

fn serve_static(stream: &mut TcpStream, dir: &str, path: &str) -> std::io::Result<()> {
    let file_path = format!("{}{}", dir, path);
    let ct = if path.ends_with(".js") { "application/javascript" }
        else if path.ends_with(".css") { "text/css" }
        else if path.ends_with(".html") { "text/html; charset=utf-8" }
        else { "application/octet-stream" };
    match std::fs::read(&file_path) {
        Ok(data) => {
            let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n", ct, data.len());
            stream.write_all(resp.as_bytes())?;
            stream.write_all(&data)
        }
        Err(_) => stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"),
    }
}

fn find_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}
