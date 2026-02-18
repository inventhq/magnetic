#![no_std]

//! magnetic-transport — generic snapshot transport WASM
//!
//! App-agnostic. Same binary for every Magnetic app.
//! Role: cache snapshots, predict action results, dedup SSE updates.
//!
//! Exports (same ABI shape as magnetic-reducer for backward compat):
//!   input_ptr()        → *mut u8     JS writes action/snapshot bytes here
//!   init()             → *const u8   returns current snapshot ptr (initially empty)
//!   reduce(len)        → *const u8   predict: lookup (state_hash, action_hash) in cache
//!   snapshot_len()     → u32         length of last reduce() result (0 = cache miss)
//!   store(len)         → u32         store authoritative snapshot; 0=no change, 1=changed

use core::cell::UnsafeCell;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// ═══════════════════════════════════════════════════════════════════
// Tuning constants
// ═══════════════════════════════════════════════════════════════════

const INPUT_CAP: usize = 16384; // 16 KB shared input buffer
const SLOT_CAP: usize = 16384;  // 16 KB per snapshot slot
const CACHE_N: usize = 4;       // 4 prediction cache entries

// ═══════════════════════════════════════════════════════════════════
// FNV-1a hash — same algorithm as magnetic.js client-side
// ═══════════════════════════════════════════════════════════════════

fn fnv(data: &[u8]) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    let mut i = 0;
    while i < data.len() {
        h ^= data[i] as u32;
        h = h.wrapping_mul(0x01000193);
        i += 1;
    }
    h
}

// ═══════════════════════════════════════════════════════════════════
// Snapshot slot — fixed buffer holding one snapshot
// ═══════════════════════════════════════════════════════════════════

struct Slot {
    data: [u8; SLOT_CAP],
    len: u32,
    hash: u32,
}

impl Slot {
    const fn new() -> Self {
        Self { data: [0; SLOT_CAP], len: 0, hash: 0 }
    }

    fn write(&mut self, src: &[u8]) {
        let n = if src.len() < SLOT_CAP { src.len() } else { SLOT_CAP };
        let mut i = 0;
        while i < n {
            self.data[i] = src[i];
            i += 1;
        }
        self.len = n as u32;
        self.hash = fnv(&self.data[..n]);
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// ═══════════════════════════════════════════════════════════════════
// Prediction cache entry
// ═══════════════════════════════════════════════════════════════════

struct CacheEntry {
    key: u32, // fnv(state_hash ^ action_hash * golden_ratio)
    slot: Slot,
    valid: bool,
}

impl CacheEntry {
    const fn new() -> Self {
        Self { key: 0, slot: Slot::new(), valid: false }
    }
}

fn make_key(state_hash: u32, action_hash: u32) -> u32 {
    state_hash ^ action_hash.wrapping_mul(0x9e3779b9)
}

// ═══════════════════════════════════════════════════════════════════
// Transport state — all static, zero alloc
// ═══════════════════════════════════════════════════════════════════

struct Transport {
    input: [u8; INPUT_CAP],

    // Current authoritative snapshot
    current: Slot,

    // Prediction cache (round-robin)
    cache: [CacheEntry; CACHE_N],
    cache_cursor: usize,

    // Last reduce() result
    result_ptr: *const u8,
    result_len: u32,

    // Pending prediction metadata (for cache learning on store())
    predicted_hash: u32,
    pending_action_hash: u32,
    pending_pre_hash: u32,
    has_pending: bool,
}

impl Transport {
    const fn new() -> Self {
        Self {
            input: [0; INPUT_CAP],
            current: Slot::new(),
            cache: [
                CacheEntry::new(), CacheEntry::new(),
                CacheEntry::new(), CacheEntry::new(),
            ],
            cache_cursor: 0,
            result_ptr: core::ptr::null(),
            result_len: 0,
            predicted_hash: 0,
            pending_action_hash: 0,
            pending_pre_hash: 0,
            has_pending: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Global singleton
// ═══════════════════════════════════════════════════════════════════

struct Globals {
    t: UnsafeCell<Transport>,
}
unsafe impl Sync for Globals {}

static G: Globals = Globals {
    t: UnsafeCell::new(Transport::new()),
};

// ═══════════════════════════════════════════════════════════════════
// WASM exports
// ═══════════════════════════════════════════════════════════════════

/// Pointer to shared input buffer. JS writes action or snapshot bytes here.
#[no_mangle]
pub extern "C" fn input_ptr() -> *mut u8 {
    unsafe { (*G.t.get()).input.as_mut_ptr() }
}

/// Initialize. Returns pointer to current snapshot data (empty on first call).
#[no_mangle]
pub extern "C" fn init() -> *const u8 {
    unsafe { (*G.t.get()).current.data.as_ptr() }
}

/// Predict: look up (current.hash, action_hash) in cache.
/// Returns pointer to snapshot data.
/// Call snapshot_len() to check result: 0 = cache miss, >0 = hit.
#[no_mangle]
pub extern "C" fn reduce(action_len: u32) -> *const u8 {
    unsafe {
        let t = &mut *G.t.get();
        let action = &t.input[..action_len as usize];
        let action_hash = fnv(action);
        let key = make_key(t.current.hash, action_hash);

        // Record pending info so store() can learn
        t.pending_action_hash = action_hash;
        t.pending_pre_hash = t.current.hash;
        t.has_pending = true;

        // Search cache
        let mut i = 0;
        while i < CACHE_N {
            if t.cache[i].valid && t.cache[i].key == key {
                // Cache hit — return predicted snapshot
                t.result_ptr = t.cache[i].slot.data.as_ptr();
                t.result_len = t.cache[i].slot.len;
                t.predicted_hash = t.cache[i].slot.hash;
                return t.result_ptr;
            }
            i += 1;
        }

        // Cache miss
        t.result_ptr = t.current.data.as_ptr();
        t.result_len = 0;
        t.predicted_hash = 0;
        t.result_ptr
    }
}

/// Length of the last reduce() result. 0 = cache miss (no prediction available).
#[no_mangle]
pub extern "C" fn snapshot_len() -> u32 {
    unsafe { (*G.t.get()).result_len }
}

/// Store authoritative snapshot from input buffer.
/// Learns cache entry if a prediction was pending.
/// Returns:
///   0 — snapshot matches prediction or is identical to current (skip re-render)
///   1 — snapshot is new/different (JS should re-render)
#[no_mangle]
pub extern "C" fn store(snap_len: u32) -> u32 {
    unsafe {
        let t = &mut *G.t.get();

        if snap_len == 0 || snap_len as usize > INPUT_CAP {
            return 0;
        }

        let snap = &t.input[..snap_len as usize];
        let snap_hash = fnv(snap);

        // Learn: cache (prev_state, action) → this result
        if t.has_pending {
            let key = make_key(t.pending_pre_hash, t.pending_action_hash);

            // Only cache if snapshot fits in a slot
            if (snap_len as usize) <= SLOT_CAP {
                let idx = t.cache_cursor % CACHE_N;
                t.cache[idx].key = key;
                t.cache[idx].slot.write(snap);
                t.cache[idx].valid = true;
                t.cache_cursor = t.cache_cursor.wrapping_add(1);
            }

            t.has_pending = false;
        }

        // Check: does authoritative match our prediction?
        if t.predicted_hash != 0 && snap_hash == t.predicted_hash {
            // Prediction was correct — update current, no re-render
            t.current.write(snap);
            t.predicted_hash = 0;
            t.result_len = 0;
            return 0;
        }
        t.predicted_hash = 0;

        // Check: is it identical to current? (duplicate SSE)
        if !t.current.is_empty() && snap_hash == t.current.hash {
            return 0;
        }

        // New snapshot — update current, signal re-render
        t.current.write(snap);
        t.result_ptr = t.current.data.as_ptr();
        t.result_len = t.current.len;
        1
    }
}
