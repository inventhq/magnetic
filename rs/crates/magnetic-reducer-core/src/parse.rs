use crate::Action;

/// Fixed-size extracted string.
struct SmallStr {
    data: [u8; 256],
    len: usize,
}

impl SmallStr {
    const fn empty() -> Self { Self { data: [0u8; 256], len: 0 } }
    fn push(&mut self, b: u8) { if self.len < 256 { self.data[self.len] = b; self.len += 1; } }
    fn as_bytes(&self) -> &[u8] { &self.data[..self.len] }
}

/// Minimal JSON action parser. No alloc.
/// Expected input: `{"action":"name","payload":{...}}`
pub fn parse_action(input: &[u8]) -> Action {
    let mut name = SmallStr::empty();
    if !extract_string_field(input, b"\"action\"", &mut name) {
        return Action::Unknown;
    }
    match name.as_bytes() {
        b"increment" => Action::Increment,
        b"decrement" => Action::Decrement,
        b"send_message" => {
            let mut text = SmallStr::empty();
            // Find "payload" then "text" inside it
            if let Some(pos) = find_subslice(input, b"\"payload\"") {
                let rest = &input[pos..];
                if let Some(bp) = find_byte(rest, b'{') {
                    extract_string_field(&rest[bp..], b"\"text\"", &mut text);
                }
            }
            let mut text_buf = [0u8; 256];
            let tlen = if text.len > 256 { 256 } else { text.len };
            text_buf[..tlen].copy_from_slice(&text.data[..tlen]);
            Action::SendMessage { text_buf, text_len: tlen }
        }
        _ => Action::Unknown,
    }
}

/// Extract a JSON string field value into `out`. Returns true if found.
fn extract_string_field(json: &[u8], key: &[u8], out: &mut SmallStr) -> bool {
    let pos = match find_subslice(json, key) {
        Some(p) => p,
        None => return false,
    };
    let after_key = pos + key.len();
    let rest = match skip_ws_and_colon(&json[after_key..]) {
        Some(r) => r,
        None => return false,
    };
    if rest.is_empty() || rest[0] != b'"' {
        return false;
    }
    extract_quoted_string(&rest[1..], out)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > haystack.len() { return None; }
    let mut i = 0;
    while i + needle.len() <= haystack.len() {
        if &haystack[i..i + needle.len()] == needle { return Some(i); }
        i += 1;
    }
    None
}

fn find_byte(s: &[u8], b: u8) -> Option<usize> {
    let mut i = 0;
    while i < s.len() { if s[i] == b { return Some(i); } i += 1; }
    None
}

fn skip_ws_and_colon(s: &[u8]) -> Option<&[u8]> {
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t' || s[i] == b'\n' || s[i] == b'\r') { i += 1; }
    if i >= s.len() || s[i] != b':' { return None; }
    i += 1;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t' || s[i] == b'\n' || s[i] == b'\r') { i += 1; }
    Some(&s[i..])
}

/// Extract bytes from a quoted JSON string into `out`. Returns true on success.
fn extract_quoted_string(s: &[u8], out: &mut SmallStr) -> bool {
    let mut i = 0;
    while i < s.len() {
        match s[i] {
            b'"' => return true,
            b'\\' if i + 1 < s.len() => {
                match s[i + 1] {
                    b'"' => out.push(b'"'),
                    b'\\' => out.push(b'\\'),
                    b'n' => out.push(b'\n'),
                    c => { out.push(b'\\'); out.push(c); }
                }
                i += 2;
            }
            c => { out.push(c); i += 1; }
        }
    }
    false
}
