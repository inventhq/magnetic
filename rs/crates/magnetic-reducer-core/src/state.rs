// ---------------------------------------------------------------------------
// std builds: dynamic Vec-backed state (server / production)
// ---------------------------------------------------------------------------
#[cfg(feature = "std")]
extern crate alloc;

#[cfg(feature = "std")]
pub struct Message {
    pub author: alloc::vec::Vec<u8>,
    pub text: alloc::vec::Vec<u8>,
}

#[cfg(feature = "std")]
impl Message {
    pub fn new(author: &[u8], text: &[u8]) -> Self {
        Self { author: author.to_vec(), text: text.to_vec() }
    }
    pub fn author_bytes(&self) -> &[u8] { &self.author }
    pub fn text_bytes(&self) -> &[u8] { &self.text }
}

#[cfg(feature = "std")]
pub struct AppState {
    pub count: i32,
    pub messages: alloc::vec::Vec<Message>,
}

#[cfg(feature = "std")]
impl AppState {
    pub fn new() -> Self {
        Self { count: 0, messages: alloc::vec::Vec::new() }
    }

    pub fn msg_count(&self) -> usize { self.messages.len() }

    pub fn msg_at(&self, i: usize) -> &Message { &self.messages[i] }

    pub fn push_message(&mut self, author: &[u8], text: &[u8]) {
        self.messages.push(Message::new(author, text));
        // Cap at 500 messages in production
        if self.messages.len() > 500 {
            self.messages.remove(0);
        }
    }
}

#[cfg(feature = "std")]
impl Default for AppState {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// no_std builds: fixed-buffer state (WASM offline fallback)
// ---------------------------------------------------------------------------
#[cfg(not(feature = "std"))]
pub struct Message {
    pub author: [u8; 32],
    pub author_len: usize,
    pub text: [u8; 256],
    pub text_len: usize,
}

#[cfg(not(feature = "std"))]
impl Message {
    pub const fn empty() -> Self {
        Self { author: [0u8; 32], author_len: 0, text: [0u8; 256], text_len: 0 }
    }

    pub fn set(&mut self, author: &[u8], text: &[u8]) {
        let alen = if author.len() > 32 { 32 } else { author.len() };
        self.author[..alen].copy_from_slice(&author[..alen]);
        self.author_len = alen;
        let tlen = if text.len() > 256 { 256 } else { text.len() };
        self.text[..tlen].copy_from_slice(&text[..tlen]);
        self.text_len = tlen;
    }

    pub fn author_bytes(&self) -> &[u8] { &self.author[..self.author_len] }
    pub fn text_bytes(&self) -> &[u8] { &self.text[..self.text_len] }
}

#[cfg(not(feature = "std"))]
const MAX_MESSAGES: usize = 20;

#[cfg(not(feature = "std"))]
pub struct AppState {
    pub count: i32,
    messages: [Message; MAX_MESSAGES],
    msg_len: usize,
}

#[cfg(not(feature = "std"))]
impl AppState {
    pub const fn new() -> Self {
        Self {
            count: 0,
            messages: { const E: Message = Message::empty(); [E; MAX_MESSAGES] },
            msg_len: 0,
        }
    }

    pub fn msg_count(&self) -> usize { self.msg_len }

    pub fn msg_at(&self, i: usize) -> &Message { &self.messages[i] }

    pub fn push_message(&mut self, author: &[u8], text: &[u8]) {
        if self.msg_len >= MAX_MESSAGES {
            let mut i = 0;
            while i < MAX_MESSAGES - 1 {
                self.messages[i].author = self.messages[i + 1].author;
                self.messages[i].author_len = self.messages[i + 1].author_len;
                self.messages[i].text = self.messages[i + 1].text;
                self.messages[i].text_len = self.messages[i + 1].text_len;
                i += 1;
            }
            self.msg_len = MAX_MESSAGES - 1;
        }
        self.messages[self.msg_len].set(author, text);
        self.msg_len += 1;
    }
}
