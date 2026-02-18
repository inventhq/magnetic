use crate::buf::Buf;
use crate::state::AppState;

/// Write i32 as decimal into buf.
fn write_i32(buf: &mut Buf, mut n: i32) {
    if n == 0 {
        buf.push(b'0');
        return;
    }
    if n < 0 {
        buf.push(b'-');
        n = -n;
    }
    let start = buf.len;
    while n > 0 {
        buf.push(b'0' + (n % 10) as u8);
        n /= 10;
    }
    // Reverse the digits we just wrote
    let end = buf.len;
    let slice = &mut buf.data[start..end];
    slice.reverse();
}

/// Write a JSON-escaped string (bytes) into buf, surrounded by quotes.
fn write_str(buf: &mut Buf, s: &[u8]) {
    buf.push(b'"');
    for &b in s {
        match b {
            b'"' => { buf.push(b'\\'); buf.push(b'"'); }
            b'\\' => { buf.push(b'\\'); buf.push(b'\\'); }
            b'\n' => { buf.push(b'\\'); buf.push(b'n'); }
            c => buf.push(c),
        }
    }
    buf.push(b'"');
}

/// Render the full DOM snapshot JSON directly into the buffer.
/// This avoids building an intermediate tree and eliminates all heap allocation.
pub fn render_snapshot(state: &AppState, buf: &mut Buf) {
    buf.clear();
    buf.extend(b"{\"root\":");

    // Root div.app
    open_tag(buf, b"div", None);
    write_attrs_1(buf, b"class", b"app");
    buf.extend(b",\"children\":[");

    //-- Child 0: h1 with count
    open_tag(buf, b"h1", Some(b"title"));
    buf.extend(b",\"text\":\"Count: ");
    write_i32(buf, state.count);
    buf.extend(b"\"}");

    buf.push(b',');

    //-- Child 1: controls div
    open_tag(buf, b"div", None);
    write_attrs_1(buf, b"class", b"controls");
    buf.extend(b",\"children\":[");
    // Decrement button
    open_tag(buf, b"button", None);
    write_events_1(buf, b"click", b"decrement");
    buf.extend(b",\"text\":\"-\"}");
    buf.push(b',');
    // Increment button
    open_tag(buf, b"button", None);
    write_events_1(buf, b"click", b"increment");
    buf.extend(b",\"text\":\"+\"}");
    buf.extend(b"]}"); // close children + controls div

    buf.push(b',');

    //-- Child 2: messages div
    open_tag(buf, b"div", Some(b"messages"));
    write_attrs_1(buf, b"class", b"messages");
    buf.extend(b",\"children\":[");
    let mut i = 0;
    while i < state.msg_count() {
        if i > 0 { buf.push(b','); }
        let m = state.msg_at(i);
        open_tag(buf, b"p", None);
        write_attrs_1(buf, b"class", b"msg");
        buf.extend(b",\"text\":\"");
        write_escaped(buf, m.author_bytes());
        buf.extend(b": ");
        write_escaped(buf, m.text_bytes());
        buf.extend(b"\"}");
        i += 1;
    }
    buf.extend(b"]}"); // close children + messages div

    buf.push(b',');

    //-- Child 3: form
    open_tag(buf, b"form", Some(b"msg-form"));
    write_events_1(buf, b"submit", b"send_message");
    buf.extend(b",\"children\":[");
    // Input element (4 attrs)
    open_tag(buf, b"input", Some(b"msg-input"));
    buf.extend(b",\"attrs\":{");
    write_kv(buf, b"type", b"text"); buf.push(b',');
    write_kv(buf, b"name", b"text"); buf.push(b',');
    write_kv(buf, b"placeholder", b"Type a message..."); buf.push(b',');
    write_kv(buf, b"autocomplete", b"off");
    buf.extend(b"}}"); // close attrs + input node
    buf.push(b',');
    // Submit button
    open_tag(buf, b"button", None);
    write_attrs_1(buf, b"type", b"submit");
    buf.extend(b",\"text\":\"Send\"}");
    buf.extend(b"]}"); // close children + form

    buf.extend(b"]}"); // close children + root div
    buf.push(b'}'); // close snapshot wrapper
}

fn write_escaped(buf: &mut Buf, s: &[u8]) {
    for &b in s {
        match b {
            b'"' => { buf.push(b'\\'); buf.push(b'"'); }
            b'\\' => { buf.push(b'\\'); buf.push(b'\\'); }
            b'\n' => { buf.push(b'\\'); buf.push(b'n'); }
            c => buf.push(c),
        }
    }
}

/// Write opening of a node object: {"tag":"...", optionally "key":"..."
fn open_tag(buf: &mut Buf, tag: &[u8], key: Option<&[u8]>) {
    buf.extend(b"{\"tag\":");
    write_str(buf, tag);
    if let Some(k) = key {
        buf.extend(b",\"key\":");
        write_str(buf, k);
    }
}

/// Write a single key:value pair (both JSON-escaped strings).
fn write_kv(buf: &mut Buf, key: &[u8], val: &[u8]) {
    write_str(buf, key);
    buf.push(b':');
    write_str(buf, val);
}

/// Write ,"attrs":{"k":"v"} — complete, closed, for single-attr nodes.
fn write_attrs_1(buf: &mut Buf, key: &[u8], val: &[u8]) {
    buf.extend(b",\"attrs\":{");
    write_kv(buf, key, val);
    buf.push(b'}');
}

/// Write ,"events":{"ev":"action"} — complete, closed.
fn write_events_1(buf: &mut Buf, ev: &[u8], action: &[u8]) {
    buf.extend(b",\"events\":{");
    write_kv(buf, ev, action);
    buf.push(b'}');
}

// ---------------------------------------------------------------------------
// SSR: render state as HTML string for first-paint (no JS needed)
// ---------------------------------------------------------------------------

/// Write HTML-escaped text into buf.
fn write_html_escaped(buf: &mut Buf, s: &[u8]) {
    for &b in s {
        match b {
            b'<' => buf.extend(b"&lt;"),
            b'>' => buf.extend(b"&gt;"),
            b'&' => buf.extend(b"&amp;"),
            b'"' => buf.extend(b"&quot;"),
            c => buf.push(c),
        }
    }
}

/// Render HTML for SSR — injected into the initial page load.
/// Uses data-a_* attributes so event delegation works immediately once JS loads.
pub fn render_html(state: &AppState, buf: &mut Buf) {
    buf.clear();
    buf.extend(b"<div class=\"app\">");

    // h1: count
    buf.extend(b"<h1 data-key=\"title\">Count: ");
    write_i32(buf, state.count);
    buf.extend(b"</h1>");

    // Controls
    buf.extend(b"<div class=\"controls\">");
    buf.extend(b"<button data-a_click=\"decrement\">-</button>");
    buf.extend(b"<button data-a_click=\"increment\">+</button>");
    buf.extend(b"</div>");

    // Messages
    buf.extend(b"<div class=\"messages\" data-key=\"messages\">");
    let mut i = 0;
    while i < state.msg_count() {
        let m = state.msg_at(i);
        buf.extend(b"<p class=\"msg\">");
        write_html_escaped(buf, m.author_bytes());
        buf.extend(b": ");
        write_html_escaped(buf, m.text_bytes());
        buf.extend(b"</p>");
        i += 1;
    }
    buf.extend(b"</div>");

    // Form
    buf.extend(b"<form data-key=\"msg-form\" data-a_submit=\"send_message\">");
    buf.extend(b"<input type=\"text\" name=\"text\" placeholder=\"Type a message...\" autocomplete=\"off\" data-key=\"msg-input\">");
    buf.extend(b"<button type=\"submit\">Send</button>");
    buf.extend(b"</form>");

    buf.extend(b"</div>");
}
