use magnetic_reducer_core::{AppState, Buf, process, render, render_html};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let port = std::env::args().nth(2).unwrap_or("3000".into());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).unwrap();
    println!("[task-board] http://localhost:{port}");
    // TODO: implement server logic
}
