#![no_std]

use core::cell::UnsafeCell;
use core::fmt::{self, Write};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// ═══════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════

const BUF_CAP: usize = 65536;
const INPUT_CAP: usize = 1024;
const MAX_ITEMS: usize = 2000;
const OVERSCAN: usize = 5;
const DEFAULT_COUNT: usize = 1500;

// ═══════════════════════════════════════════════════════════════════
// Output buffer (64 KB — fits ~15 visible cards at ~1 KB each)
// ═══════════════════════════════════════════════════════════════════

struct Buf {
    data: [u8; BUF_CAP],
    len: usize,
}

impl Buf {
    const fn new() -> Self {
        Self { data: [0; BUF_CAP], len: 0 }
    }
    fn clear(&mut self) {
        self.len = 0;
    }
    fn push_str(&mut self, s: &str) {
        let b = s.as_bytes();
        let avail = BUF_CAP - self.len;
        let n = if b.len() < avail { b.len() } else { avail };
        let dst = &mut self.data[self.len..self.len + n];
        dst.copy_from_slice(&b[..n]);
        self.len += n;
    }
    fn push_byte(&mut self, b: u8) {
        if self.len < BUF_CAP {
            self.data[self.len] = b;
            self.len += 1;
        }
    }
}

impl Write for Buf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// Per-item compact data (generated once at init, 14 bytes/item)
// ═══════════════════════════════════════════════════════════════════

#[derive(Clone, Copy)]
struct ItemData {
    height: u32,
    author_idx: u8,
    title_idx: u8,
    body_idx: u8,
    has_image: bool,
    avatar_hue: u16,
    image_hue: u16,
    likes: u16,
    comments: u16,
}

const ZERO_ITEM: ItemData = ItemData {
    height: 0, author_idx: 0, title_idx: 0, body_idx: 0,
    has_image: false, avatar_hue: 0, image_hue: 0, likes: 0, comments: 0,
};

// ═══════════════════════════════════════════════════════════════════
// State — holds all item metadata + prefix sums for O(log n) lookup
// ═══════════════════════════════════════════════════════════════════

struct State {
    count: usize,
    scroll_top: f64,
    viewport_height: f64,
    items: [ItemData; MAX_ITEMS],
    prefix: [u32; MAX_ITEMS + 1],
}

impl State {
    const fn new() -> Self {
        Self {
            count: 0,
            scroll_top: 0.0,
            viewport_height: 800.0,
            items: [ZERO_ITEM; MAX_ITEMS],
            prefix: [0u32; MAX_ITEMS + 1],
        }
    }

    fn generate(&mut self, count: usize) {
        let count = if count > MAX_ITEMS { MAX_ITEMS } else { count };
        self.count = count;
        let mut seed: u64 = 42;

        let mut i = 0;
        while i < count {
            seed = lcg(seed);
            let author_idx = (seed % AUTHORS.len() as u64) as u8;
            seed = lcg(seed);
            let title_idx = (seed % TITLES.len() as u64) as u8;
            seed = lcg(seed);
            let body_idx = (seed % BODIES.len() as u64) as u8;
            seed = lcg(seed);
            let has_image = (seed % 3) != 0;
            seed = lcg(seed);
            let img_h: u32 = if has_image { 100 + (seed % 120) as u32 } else { 0 };
            let body_len = BODIES[body_idx as usize].len() as u32;
            let base: u32 = 130 + min_u32(body_len / 4, 60);
            let height = base + img_h;
            seed = lcg(seed);
            let avatar_hue = (seed % 360) as u16;
            seed = lcg(seed);
            let image_hue = (seed % 360) as u16;
            seed = lcg(seed);
            let likes = (seed % 2000) as u16;
            seed = lcg(seed);
            let comments = (seed % 150) as u16;

            self.items[i] = ItemData {
                height, author_idx, title_idx, body_idx,
                has_image, avatar_hue, image_hue, likes, comments,
            };
            i += 1;
        }

        // Prefix sums for O(1) offset lookup
        self.prefix[0] = 0;
        let mut j = 0;
        while j < count {
            self.prefix[j + 1] = self.prefix[j] + self.items[j].height;
            j += 1;
        }
    }

    fn total_height(&self) -> u32 {
        self.prefix[self.count]
    }

    fn visible_range(&self) -> (usize, usize) {
        let n = self.count;
        if n == 0 { return (0, 0); }

        // Binary search for first item whose bottom edge > scroll_top
        let st = self.scroll_top as u32;
        let mut lo: usize = 0;
        let mut hi: usize = n;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.prefix[mid + 1] <= st { lo = mid + 1; } else { hi = mid; }
        }
        let start = if lo >= OVERSCAN { lo - OVERSCAN } else { 0 };

        // Walk forward to find last visible item
        let bottom = (self.scroll_top + self.viewport_height) as u32;
        let mut end = lo;
        while end < n && self.prefix[end] < bottom { end += 1; }
        let end = if end + OVERSCAN < n { end + OVERSCAN } else { n };

        (start, end)
    }
}

fn min_u32(a: u32, b: u32) -> u32 { if a < b { a } else { b } }

fn lcg(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

// ═══════════════════════════════════════════════════════════════════
// Static text arrays — identical to the server for byte-perfect match
// ═══════════════════════════════════════════════════════════════════

const AUTHORS: &[&str] = &[
    "Alice Chen", "Bob Martinez", "Carol Kim", "David O'Brien", "Eva Kowalski",
    "Frank Yamada", "Grace Okafor", "Henry Johansson", "Iris Patel", "Jack Thompson",
    "Kira Novak", "Leo Fernandez", "Maya Singh", "Noah Williams", "Olivia Reyes",
];

const TITLES: &[&str] = &[
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

const BODIES: &[&str] = &[
    "We\u{2019}ve been experimenting with a new approach to building user interfaces that moves all state management to the server. The results have been remarkable \u{2014} our client bundle dropped to under 2KB while maintaining full interactivity.",
    "After years of dealing with JavaScript bundle bloat and hydration issues, we decided to take a radically different approach. By treating the UI as a pure function of server state, we eliminated an entire class of bugs.",
    "The key insight is that most application logic doesn\u{2019}t need to run on the client. When you move reducers to the server, the client becomes a thin rendering shell that\u{2019}s trivially fast to load and execute.",
    "Our benchmarks show consistent 60fps scrolling with 10,000+ items in the feed. The secret? Server-side virtualization \u{2014} we only send the visible window over the wire.",
    "Memory usage stays flat regardless of list size because the DOM only contains visible items. The server handles all the bookkeeping for scroll position and item layout.",
    "We measured a 94% reduction in JavaScript bundle size compared to our previous React implementation. Time to Interactive dropped from 3.2s to 180ms on 3G connections.",
    "The architecture naturally supports AI-powered features. Since all state lives on the server, AI agents can directly manipulate the application state without going through a frontend API layer.",
    "Error handling becomes trivial when there\u{2019}s a single source of truth. No more inconsistent state between client and server. No more optimistic update rollbacks.",
];

// ═══════════════════════════════════════════════════════════════════
// Minimal JSON number extraction (no alloc, no serde)
// ═══════════════════════════════════════════════════════════════════

fn extract_num(text: &[u8], key: &[u8]) -> Option<f64> {
    let klen = key.len();
    if text.len() < klen + 4 { return None; }
    let mut i: usize = 0;
    while i + klen + 3 < text.len() {
        // Match "key":
        if text[i] == b'"' {
            let s = i + 1;
            if s + klen < text.len()
                && &text[s..s + klen] == key
                && text[s + klen] == b'"'
            {
                let mut j = s + klen + 1;
                // skip whitespace and colon
                while j < text.len() && (text[j] == b' ' || text[j] == b':') { j += 1; }
                // parse number
                let num_start = j;
                while j < text.len() && (text[j] == b'.' || (text[j] >= b'0' && text[j] <= b'9')) {
                    j += 1;
                }
                if j > num_start {
                    return Some(parse_f64(&text[num_start..j]));
                }
            }
        }
        i += 1;
    }
    None
}

fn parse_f64(b: &[u8]) -> f64 {
    let mut result: f64 = 0.0;
    let mut frac = false;
    let mut div: f64 = 1.0;
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'.' { frac = true; i += 1; continue; }
        if b[i] < b'0' || b[i] > b'9' { break; }
        let d = (b[i] - b'0') as f64;
        if frac {
            div *= 10.0;
            result += d / div;
        } else {
            result = result * 10.0 + d;
        }
        i += 1;
    }
    result
}

// ═══════════════════════════════════════════════════════════════════
// Process action → update state → render snapshot
// ═══════════════════════════════════════════════════════════════════

fn process(state: &mut State, input: &[u8], buf: &mut Buf) {
    if let Some(st) = extract_num(input, b"scrollTop") {
        state.scroll_top = st;
    }
    if let Some(vh) = extract_num(input, b"viewportHeight") {
        state.viewport_height = vh;
    }
    render(state, buf);
}

// ═══════════════════════════════════════════════════════════════════
// JSON DOM snapshot renderer — byte-perfect match with server output
//
// Brace rules (learned the hard way):
//   Leaf (no children):    {{ ... }}  →  { ... }    close = implicit in write!
//   Node with children:    {{ ... ,"children":[   close = ]}
//   Root wrapper:          {"root":{"tag":...,"children":[  close = ]}}
// ═══════════════════════════════════════════════════════════════════

fn render(state: &State, buf: &mut Buf) {
    buf.clear();
    let total_h = state.total_height();
    let (start, end) = state.visible_range();
    let visible = end - start;

    // Root wrapper + scroller (2 objects + 1 array opened → close ]}} )
    buf.push_str(r#"{"root":{"tag":"div","key":"scroller","attrs":{"class":"feed-root","id":"scroller"},"children":["#);

    // Sentinel (1 obj + 1 arr → close ]} )
    let _ = write!(buf,
        r#"{{"tag":"div","key":"sentinel","attrs":{{"class":"sentinel","style":"height:{}px;position:relative"}},"children":["#,
        total_h);

    // Visible cards
    let mut i = start;
    while i < end {
        if i > start { buf.push_byte(b','); }
        render_card(state, buf, i);
        i += 1;
    }

    buf.push_str("]}"); // close sentinel

    // Bench data (leaf: 1 obj opened by {{ closed by }} )
    let _ = write!(buf,
        r#",{{"tag":"div","key":"bench","attrs":{{"class":"bench-data","data-total":"{}","data-visible":"{}","data-start":"{}","data-end":"{}","data-total-h":"{}"}}}}"#,
        state.count, visible, start, end, total_h);

    buf.push_str("]}}"); // close scroller children + scroller obj + root wrapper
}

fn render_card(state: &State, buf: &mut Buf, idx: usize) {
    let item = &state.items[idx];
    let top = state.prefix[idx];

    // Card (1 obj + 1 arr → ]} )
    let _ = write!(buf,
        r#"{{"tag":"div","key":"card-{}","attrs":{{"class":"card","style":"position:absolute;top:{}px;left:1rem;right:1rem;height:{}px"}},"children":["#,
        idx, top, item.height);

    // ── Header (1 obj + 1 arr → ]} ) ──
    buf.push_str(r#"{"tag":"div","attrs":{"class":"card-header"},"children":["#);

    // Avatar (leaf → self-closed by write!)
    let _ = write!(buf,
        r#"{{"tag":"div","attrs":{{"class":"avatar","style":"background:hsl({},65%,55%)"}}}}"#,
        item.avatar_hue);

    // Card-meta (1 obj + 1 arr → ]} )
    buf.push_str(r#",{"tag":"div","attrs":{"class":"card-meta"},"children":["#);

    // Author span (leaf)
    let _ = write!(buf,
        r#"{{"tag":"span","attrs":{{"class":"author"}},"text":"{}"}}"#,
        AUTHORS[item.author_idx as usize]);

    // Time span (leaf)
    let hours = (state.count - idx) * 2;
    if hours < 24 {
        let _ = write!(buf,
            r#",{{"tag":"span","attrs":{{"class":"time"}},"text":"{}h ago"}}"#, hours);
    } else {
        let _ = write!(buf,
            r#",{{"tag":"span","attrs":{{"class":"time"}},"text":"{}d ago"}}"#, hours / 24);
    }

    buf.push_str("]}"); // close card-meta
    buf.push_str("]}"); // close card-header

    // ── Title (leaf) ──
    let _ = write!(buf,
        r#",{{"tag":"h3","attrs":{{"class":"card-title"}},"text":"{}"}}"#,
        TITLES[item.title_idx as usize]);

    // ── Image (leaf, conditional) ──
    if item.has_image {
        let _ = write!(buf,
            r#",{{"tag":"div","attrs":{{"class":"card-img","style":"background:linear-gradient(135deg,hsl({},50%,30%),hsl({},60%,45%))"}}}}"#,
            item.image_hue, (item.image_hue + 40) % 360);
    }

    // ── Body (leaf) ──
    let _ = write!(buf,
        r#",{{"tag":"p","attrs":{{"class":"card-body"}},"text":"{}"}}"#,
        BODIES[item.body_idx as usize]);

    // ── Footer (1 obj + 1 arr → ]} ) ──
    buf.push_str(r#",{"tag":"div","attrs":{"class":"card-footer"},"children":["#);

    let _ = write!(buf,
        r#"{{"tag":"span","attrs":{{"class":"likes"}},"text":"{} likes"}}"#,
        item.likes);
    let _ = write!(buf,
        r#",{{"tag":"span","attrs":{{"class":"comments"}},"text":"{} comments"}}"#,
        item.comments);

    buf.push_str("]}"); // close card-footer
    buf.push_str("]}"); // close card
}

// ═══════════════════════════════════════════════════════════════════
// WASM ABI — matches magnetic-reducer exports exactly
// ═══════════════════════════════════════════════════════════════════

struct Globals {
    state: UnsafeCell<State>,
    buf: UnsafeCell<Buf>,
    input: UnsafeCell<[u8; INPUT_CAP]>,
}
unsafe impl Sync for Globals {}

static G: Globals = Globals {
    state: UnsafeCell::new(State::new()),
    buf: UnsafeCell::new(Buf::new()),
    input: UnsafeCell::new([0u8; INPUT_CAP]),
};

#[no_mangle]
pub extern "C" fn input_ptr() -> *mut u8 {
    unsafe { (*G.input.get()).as_mut_ptr() }
}

#[no_mangle]
pub extern "C" fn init() -> *const u8 {
    unsafe {
        let state = &mut *G.state.get();
        let buf = &mut *G.buf.get();
        state.generate(DEFAULT_COUNT);
        render(state, buf);
        buf.data.as_ptr()
    }
}

#[no_mangle]
pub extern "C" fn reduce(len: u32) -> *const u8 {
    unsafe {
        let input = &(&*G.input.get())[..len as usize];
        let state = &mut *G.state.get();
        let buf = &mut *G.buf.get();
        process(state, input, buf);
        buf.data.as_ptr()
    }
}

#[no_mangle]
pub extern "C" fn snapshot_len() -> u32 {
    unsafe { (*G.buf.get()).len as u32 }
}
