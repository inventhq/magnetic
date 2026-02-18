#![cfg_attr(not(feature = "std"), no_std)]

pub mod buf;
mod dom;
mod parse;
mod state;

pub use buf::Buf;
pub use state::{AppState, Message};

/// Supported actions.
pub enum Action {
    Increment,
    Decrement,
    SendMessage { text_buf: [u8; 256], text_len: usize },
    Unknown,
}

/// Pure reducer: mutate state based on action.
pub fn reduce(state: &mut AppState, action: Action) {
    match action {
        Action::Increment => state.count += 1,
        Action::Decrement => {
            if state.count > 0 {
                state.count -= 1;
            }
        }
        Action::SendMessage { text_buf, text_len } => {
            state.push_message(b"user", &text_buf[..text_len]);
        }
        Action::Unknown => {}
    }
}

/// Render the current state to a JSON DOM snapshot into the provided buffer.
pub fn render(state: &AppState, buf: &mut Buf) {
    dom::render_snapshot(state, buf);
}

/// Render the current state as an HTML string for SSR first-paint.
pub fn render_html(state: &AppState, buf: &mut Buf) {
    dom::render_html(state, buf);
}

/// Parse action bytes and dispatch reduce + render.
/// Input format: `{"action":"name","payload":{...}}`
pub fn process(state: &mut AppState, input: &[u8], buf: &mut Buf) {
    let action = parse::parse_action(input);
    reduce(state, action);
    render(state, buf);
}
