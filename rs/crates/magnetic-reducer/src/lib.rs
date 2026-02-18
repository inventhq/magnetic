#![no_std]

use magnetic_reducer_core::{AppState, Buf, process, render};
use core::cell::UnsafeCell;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// All static state — no allocator needed.
struct Globals {
    state: UnsafeCell<AppState>,
    buf: UnsafeCell<Buf>,
    // Scratch buffer for incoming action data from JS
    input: UnsafeCell<[u8; 1024]>,
}
unsafe impl Sync for Globals {}

static G: Globals = Globals {
    state: UnsafeCell::new(AppState::new()),
    buf: UnsafeCell::new(Buf::new()),
    input: UnsafeCell::new([0u8; 1024]),
};

/// Return pointer to the input scratch buffer. JS writes action data here.
#[no_mangle]
pub extern "C" fn input_ptr() -> *mut u8 {
    unsafe { (*G.input.get()).as_mut_ptr() }
}

/// Initialize state and return pointer to snapshot.
/// Returns: pointer to output buf data. First 4 bytes = LE u32 length.
#[no_mangle]
pub extern "C" fn init() -> *const u8 {
    unsafe {
        let state = &*G.state.get();
        let buf = &mut *G.buf.get();
        render(state, buf);
        write_len_prefix(buf)
    }
}

/// Process action from input buffer (len bytes) and return new snapshot.
#[no_mangle]
pub extern "C" fn reduce(len: u32) -> *const u8 {
    unsafe {
        let input = &(&*G.input.get())[..len as usize];
        let state = &mut *G.state.get();
        let buf = &mut *G.buf.get();
        process(state, input, buf);
        write_len_prefix(buf)
    }
}

/// Prepend 4-byte LE length to buf and return pointer.
/// We write the length into the first 4 bytes of a small header area.
static mut HEADER: [u8; 4] = [0u8; 4];

unsafe fn write_len_prefix(buf: &Buf) -> *const u8 {
    let len = buf.len as u32;
    HEADER = len.to_le_bytes();
    // Return pointer to header; JS reads 4 bytes then reads buf.data directly.
    // Actually, let's return buf.data ptr and pass len separately via export.
    buf.data.as_ptr()
}

/// Get the length of the last rendered snapshot.
#[no_mangle]
pub extern "C" fn snapshot_len() -> u32 {
    unsafe { (*G.buf.get()).len as u32 }
}
