/// Fixed-size write buffer — no alloc needed.
pub struct Buf {
    pub data: [u8; 4096],
    pub len: usize,
}

impl Buf {
    pub const fn new() -> Self {
        Self { data: [0u8; 4096], len: 0 }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn push(&mut self, b: u8) {
        if self.len < self.data.len() {
            self.data[self.len] = b;
            self.len += 1;
        }
    }

    pub fn extend(&mut self, s: &[u8]) {
        for &b in s {
            self.push(b);
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len]
    }
}
