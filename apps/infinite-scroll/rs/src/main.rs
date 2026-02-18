use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::fmt::Write as FmtWrite;

// ---------------------------------------------------------------------------
// Feed item model
// ---------------------------------------------------------------------------

struct FeedItem {
    id: usize,
    author: String,
    avatar_hue: u16,       // 0-360 for HSL color
    title: String,
    body: String,
    likes: u32,
    comments: u32,
    timestamp: String,
    card_height: u32,      // variable height in px (120-320)
    has_image: bool,
    image_hue: u16,
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

struct AppState {
    items: Vec<FeedItem>,
    total_count: usize,
    scroll_top: f64,
    viewport_height: f64,
    overscan: usize,       // extra items above/below viewport
}

impl AppState {
    fn new(count: usize) -> Self {
        let items = generate_items(count);
        Self {
            total_count: items.len(),
            items,
            scroll_top: 0.0,
            viewport_height: 800.0,
            overscan: 5,
        }
    }

    /// Compute prefix sum of heights for O(1) offset lookup.
    fn prefix_heights(&self) -> Vec<f64> {
        let mut acc = Vec::with_capacity(self.items.len() + 1);
        acc.push(0.0);
        for item in &self.items {
            let last = *acc.last().unwrap();
            acc.push(last + item.card_height as f64);
        }
        acc
    }

    /// Total scrollable height.
    fn total_height(&self, prefix: &[f64]) -> f64 {
        *prefix.last().unwrap_or(&0.0)
    }

    /// Visible window: (start_idx, end_idx) — indices of items to render.
    fn visible_range(&self, prefix: &[f64]) -> (usize, usize) {
        let n = self.items.len();
        if n == 0 { return (0, 0); }

        // Binary search for first visible item
        let mut lo = 0usize;
        let mut hi = n;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if prefix[mid + 1] <= self.scroll_top {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let start = if lo >= self.overscan { lo - self.overscan } else { 0 };

        // Find last visible item
        let bottom = self.scroll_top + self.viewport_height;
        let mut end = lo;
        while end < n && prefix[end] < bottom {
            end += 1;
        }
        let end = (end + self.overscan).min(n);

        (start, end)
    }
}

// ---------------------------------------------------------------------------
// Deterministic item generation (seeded pseudo-random)
// ---------------------------------------------------------------------------

fn generate_items(count: usize) -> Vec<FeedItem> {
    let mut items = Vec::with_capacity(count);
    let mut seed: u64 = 42;

    let authors = [
        "Alice Chen", "Bob Martinez", "Carol Kim", "David O'Brien", "Eva Kowalski",
        "Frank Yamada", "Grace Okafor", "Henry Johansson", "Iris Patel", "Jack Thompson",
        "Kira Novak", "Leo Fernandez", "Maya Singh", "Noah Williams", "Olivia Reyes",
    ];
    let titles = [
        "Building server-driven UIs at scale",
        "Why we moved to Rust for our backend",
        "The future of real-time web applications",
        "Lessons learned from migrating 10k components",
        "Zero-allocation rendering: a deep dive",
        "How we reduced our bundle to under 2KB",
        "Rethinking state management for the AI era",
        "Performance benchmarks: framework comparison",
        "Virtual scrolling without a virtual DOM",
        "SSE vs WebSockets: our production experience",
        "Offline-first architecture with WASM",
        "Why server-side rendering is back",
        "Designing for 120fps on mobile",
        "The end of JavaScript framework fatigue",
        "Memory-efficient lists with 100k+ items",
        "Cross-platform UI from a single codebase",
        "Streaming state machines for AI agents",
        "How we eliminated loading spinners",
        "Rust compile times: tips and tricks",
        "The architecture behind our real-time feed",
    ];
    let bodies = [
        "We've been experimenting with a new approach to building user interfaces that moves all state management to the server. The results have been remarkable — our client bundle dropped to under 2KB while maintaining full interactivity.",
        "After years of dealing with JavaScript bundle bloat and hydration issues, we decided to take a radically different approach. By treating the UI as a pure function of server state, we eliminated an entire class of bugs.",
        "The key insight is that most application logic doesn't need to run on the client. When you move reducers to the server, the client becomes a thin rendering shell that's trivially fast to load and execute.",
        "Our benchmarks show consistent 60fps scrolling with 10,000+ items in the feed. The secret? Server-side virtualization — we only send the visible window over the wire.",
        "Memory usage stays flat regardless of list size because the DOM only contains visible items. The server handles all the bookkeeping for scroll position and item layout.",
        "We measured a 94% reduction in JavaScript bundle size compared to our previous React implementation. Time to Interactive dropped from 3.2s to 180ms on 3G connections.",
        "The architecture naturally supports AI-powered features. Since all state lives on the server, AI agents can directly manipulate the application state without going through a frontend API layer.",
        "Error handling becomes trivial when there's a single source of truth. No more inconsistent state between client and server. No more optimistic update rollbacks.",
    ];

    for i in 0..count {
        seed = lcg(seed);
        let author = authors[(seed % authors.len() as u64) as usize];
        seed = lcg(seed);
        let title = titles[(seed % titles.len() as u64) as usize];
        seed = lcg(seed);
        let body = bodies[(seed % bodies.len() as u64) as usize];
        seed = lcg(seed);
        let has_image = (seed % 3) != 0; // ~66% of cards have images
        seed = lcg(seed);
        let image_height: u32 = if has_image { 100 + (seed % 120) as u32 } else { 0 };
        let base_height: u32 = 130 + (body.len() as u32 / 4).min(60);
        let card_height = base_height + image_height;
        seed = lcg(seed);
        let avatar_hue = (seed % 360) as u16;
        seed = lcg(seed);
        let image_hue = (seed % 360) as u16;
        seed = lcg(seed);
        let likes = (seed % 2000) as u32;
        seed = lcg(seed);
        let comments = (seed % 150) as u32;
        let hours = (count - i) * 2;
        let timestamp = if hours < 24 {
            format!("{}h ago", hours)
        } else {
            format!("{}d ago", hours / 24)
        };

        items.push(FeedItem {
            id: i,
            author: author.to_string(),
            avatar_hue,
            title: title.to_string(),
            body: body.to_string(),
            likes,
            comments,
            timestamp,
            card_height,
            has_image,
            image_hue,
        });
    }
    items
}

fn lcg(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

// ---------------------------------------------------------------------------
// JSON DOM snapshot renderer
// ---------------------------------------------------------------------------

fn render_snapshot(state: &AppState) -> String {
    let prefix = state.prefix_heights();
    let total_h = state.total_height(&prefix);
    let (start, end) = state.visible_range(&prefix);

    let mut s = String::with_capacity(16384);
    // Scroller: keyed — root wrapper {"root":{ needs ]}} at the end (2 objects)
    s.push_str(r#"{"root":{"tag":"div","key":"scroller","attrs":{"class":"feed-root","id":"scroller"},"children":["#);

    // Sentinel: 1 object + 1 array opened → close with ]}
    write!(s, r#"{{"tag":"div","key":"sentinel","attrs":{{"class":"sentinel","style":"height:{}px;position:relative"}},"children":["#,
        total_h as u64).unwrap();

    for i in start..end {
        if i > start { s.push(','); }
        let top = prefix[i];
        render_card(&mut s, &state.items[i], top);
    }

    s.push_str("]}");  // close sentinel (1 obj + 1 arr)

    // Bench: leaf node, no children — 1 object opened by {{ closed by }}
    write!(s, r#",{{"tag":"div","key":"bench","attrs":{{"class":"bench-data","data-total":"{}","data-visible":"{}","data-start":"{}","data-end":"{}","data-total-h":"{}"}}}}"#,
        state.total_count, end - start, start, end, total_h as u64).unwrap();

    s.push_str("]}}");  // close scroller children + scroller obj + root wrapper
    s
}

fn render_card(s: &mut String, item: &FeedItem, top: f64) {
    // card: 1 obj + 1 arr opened → close with ]}
    write!(s, r#"{{"tag":"div","key":"card-{}","attrs":{{"class":"card","style":"position:absolute;top:{}px;left:1rem;right:1rem;height:{}px"}},"children":["#,
        item.id, top as u64, item.card_height).unwrap();

    // card-header: 1 obj + 1 arr → ]}
    write!(s, r#"{{"tag":"div","attrs":{{"class":"card-header"}},"children":["#).unwrap();
    // avatar: leaf, 1 obj → }}
    write!(s, r#"{{"tag":"div","attrs":{{"class":"avatar","style":"background:hsl({},65%,55%)"}}}}"#, item.avatar_hue).unwrap();
    // card-meta: 1 obj + 1 arr → ]}
    write!(s, r#",{{"tag":"div","attrs":{{"class":"card-meta"}},"children":["#).unwrap();
    // author span: leaf → }}
    write!(s, r#"{{"tag":"span","attrs":{{"class":"author"}},"text":"{}"}}"#, item.author).unwrap();
    // time span: leaf → }}
    write!(s, r#",{{"tag":"span","attrs":{{"class":"time"}},"text":"{}"}}"#, item.timestamp).unwrap();
    s.push_str("]}");  // close card-meta
    s.push_str("]}");  // close card-header

    // title: leaf → }}
    write!(s, r#",{{"tag":"h3","attrs":{{"class":"card-title"}},"text":"{}"}}"#, item.title).unwrap();

    if item.has_image {
        // img: leaf → }}
        write!(s, r#",{{"tag":"div","attrs":{{"class":"card-img","style":"background:linear-gradient(135deg,hsl({},50%,30%),hsl({},60%,45%))"}}}}"#,
            item.image_hue, (item.image_hue + 40) % 360).unwrap();
    }

    // body: leaf → }}
    write!(s, r#",{{"tag":"p","attrs":{{"class":"card-body"}},"text":"{}"}}"#, item.body).unwrap();

    // card-footer: 1 obj + 1 arr → ]}
    write!(s, r#",{{"tag":"div","attrs":{{"class":"card-footer"}},"children":["#).unwrap();
    write!(s, r#"{{"tag":"span","attrs":{{"class":"likes"}},"text":"{} likes"}}"#, item.likes).unwrap();
    write!(s, r#",{{"tag":"span","attrs":{{"class":"comments"}},"text":"{} comments"}}"#, item.comments).unwrap();
    s.push_str("]}");  // close card-footer

    s.push_str("]}");  // close card
}
// SSR HTML renderer
// ---------------------------------------------------------------------------

fn render_ssr_html(state: &AppState) -> String {
    let prefix = state.prefix_heights();
    let total_h = state.total_height(&prefix);
    let (start, end) = state.visible_range(&prefix);

    let mut s = String::with_capacity(16384);
    write!(s, r#"<div class="feed-root" id="scroller" data-key="scroller">"#).unwrap();
    write!(s, r#"<div class="sentinel" data-key="sentinel" style="height:{}px;position:relative">"#, total_h as u64).unwrap();

    for i in start..end {
        let top = prefix[i];
        render_card_html(&mut s, &state.items[i], top);
    }

    s.push_str("</div>"); // close sentinel
    write!(s, r#"<div class="bench-data" data-key="bench" data-total="{}" data-visible="{}" data-start="{}" data-end="{}" data-total-h="{}"></div>"#,
        state.total_count, end - start, start, end, total_h as u64).unwrap();
    s.push_str("</div>");
    s
}

fn render_card_html(s: &mut String, item: &FeedItem, top: f64) {
    write!(s, r#"<div class="card" data-key="card-{}" style="position:absolute;top:{}px;left:1rem;right:1rem;height:{}px">"#, item.id, top as u64, item.card_height).unwrap();
    write!(s, r#"<div class="card-header"><div class="avatar" style="background:hsl({},65%,55%)"></div>"#, item.avatar_hue).unwrap();
    write!(s, r#"<div class="card-meta"><span class="author">{}</span><span class="time">{}</span></div></div>"#, item.author, item.timestamp).unwrap();
    write!(s, r#"<h3 class="card-title">{}</h3>"#, item.title).unwrap();
    if item.has_image {
        write!(s, r#"<div class="card-img" style="background:linear-gradient(135deg,hsl({},50%,30%),hsl({},60%,45%))"></div>"#,
            item.image_hue, (item.image_hue + 40) % 360).unwrap();
    }
    write!(s, r#"<p class="card-body">{}</p>"#, item.body).unwrap();
    write!(s, r#"<div class="card-footer"><span class="likes">{} likes</span><span class="comments">{} comments</span></div>"#, item.likes, item.comments).unwrap();
    s.push_str("</div>");
}

// ---------------------------------------------------------------------------
// Action processing (reducer)
// ---------------------------------------------------------------------------

fn process_action(state: &mut AppState, body: &[u8]) -> String {
    // Minimal JSON parse for {"action":"name","payload":{...}}
    let text = std::str::from_utf8(body).unwrap_or("");
    eprintln!("[action] body: {}", &text[..text.len().min(120)]);
    if let Some(action) = extract_json_str(text, "action") {
        match action {
            "on_scroll" => {
                if let Some(st) = extract_json_num(text, "scrollTop") {
                    state.scroll_top = st;
                    eprintln!("[action] on_scroll scrollTop={}", st);
                }
                if let Some(vh) = extract_json_num(text, "viewportHeight") {
                    state.viewport_height = vh;
                }
            }
            "load_more" => {
                let current = state.items.len();
                let new_items = generate_items_range(current, current + 200);
                state.items.extend(new_items);
                state.total_count = state.items.len();
            }
            _ => {}
        }
    }
    render_snapshot(state)
}

fn generate_items_range(start: usize, end: usize) -> Vec<FeedItem> {
    let full = generate_items(end);
    full.into_iter().skip(start).collect()
}

fn extract_json_str<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\"", key);
    let pos = text.find(&needle)?;
    let after = &text[pos + needle.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    if rest.starts_with('"') {
        let start = 1;
        let end = rest[1..].find('"')?;
        Some(&rest[start..start + end])
    } else {
        None
    }
}

fn extract_json_num(text: &str, key: &str) -> Option<f64> {
    let needle = format!("\"{}\"", key);
    let pos = text.find(&needle)?;
    let after = &text[pos + needle.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    // Read number chars
    let num_end = rest.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(rest.len());
    rest[..num_end].parse().ok()
}

// ---------------------------------------------------------------------------
// HTTP Server
// ---------------------------------------------------------------------------

struct Server {
    app: Mutex<AppState>,
    sse_clients: Mutex<Vec<TcpStream>>,
    static_dir: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port = find_arg(&args, "--port").unwrap_or_else(|| "3001".into());
    let static_dir = find_arg(&args, "--public").unwrap_or_else(|| "public".into());
    let item_count: usize = find_arg(&args, "--items")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1500);

    eprintln!("[infinite-scroll] generating {} items...", item_count);
    let server = Arc::new(Server {
        app: Mutex::new(AppState::new(item_count)),
        sse_clients: Mutex::new(Vec::new()),
        static_dir,
    });

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    eprintln!("[infinite-scroll] http://localhost:{}", port);

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        let server = Arc::clone(&server);
        thread::spawn(move || { let _ = handle(stream, &server); });
    }
}

fn find_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
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
    eprintln!("[sse] client registered, total={}", server.sse_clients.lock().unwrap().len());
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
    let count = clients.len();
    let mut alive = Vec::new();
    for mut c in clients.drain(..) {
        match write_sse(&mut c, data) {
            Ok(_) => { alive.push(c); }
            Err(e) => { eprintln!("[broadcast] write failed: {}", e); }
        }
    }
    eprintln!("[broadcast] clients={} alive={} data_len={}", count, alive.len(), data.len());
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
        else if path.ends_with(".wasm") { "application/wasm" }
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
