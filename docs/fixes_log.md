# Magnetic — Fixes Log

Record of production bugs and fixes for cross-team reference.

---

## 2026-02-18: Subdomain double-prefix 404 (magnetic.js, transport.wasm)

**Symptom:** Apps deployed via `magnetic push` rendered static HTML at `{app}.fujs.dev` but `magnetic.js` and `transport.wasm` returned 404. No client interactivity.

**Root cause:** Platform SSR emitted `<script src="/apps/{name}/magnetic.js">`. When accessed via subdomain, Caddy proxied to the platform server which already prepended `/apps/{name}/`, causing a double prefix: `/apps/{name}/apps/{name}/magnetic.js` → 404. Same issue affected `/sse` and `/actions/*`.

**Fix:**
- `deploy/Caddyfile`: Wildcard block rewrites `{subdomain}.fujs.dev/{path}` → `/apps/{subdomain}/{path}` and passes `X-Subdomain` header.
- `rs/crates/magnetic-v8-server/src/platform.rs`: Detects `X-Subdomain` header. When present, SSR emits root-relative paths (`/magnetic.js`, `/sse`) instead of `/apps/{name}/...`.

**Commit:** `acf8643`

---

## 2026-02-18: V8 parking kills isolate — cannot reinitialize

**Symptom:** After 5 minutes idle, app returns 502. Server logs show `PoisonError` from `v8::V8` and cascading `SendError` panics on every subsequent request. Process stays running but all request handler threads panic.

**Root cause:** The V8 isolate parking implementation dropped the `mpsc::Sender`, causing the V8 thread to exit when the channel closed. On the next request, `ensure_warm()` tried to spawn a new V8 thread, but **V8's global platform (`v8::V8::initialize_platform`) can only be called once per process**. The second initialization hit a poisoned `Once` lock → `PoisonError` → every handler thread that tried to send to the dead channel panicked with `SendError` (unwrap on a failed `tx.send()`).

**Fix:**
- `park()` no longer drops the sender or kills the V8 thread. It just sets an `AtomicBool` flag for metrics/logging. The V8 thread stays alive, blocking on `rx.recv()` which costs zero CPU.
- `ensure_warm()` simply unmarks the parked flag — the thread was never stopped.
- All `tx.send(...).unwrap()` calls replaced with `.is_err()` checks that return HTTP 503 instead of panicking.

**Key lesson for AI agents:** Never kill a V8 thread in a long-running process. V8's global platform is a one-time init. "Parking" means the thread idles on a blocked channel recv, not thread termination.

**Commit:** `62bea38`

---

## 2026-02-19: V8 "Invalid global state" SEGV on multi-app startup

**Symptom:** Platform server with 2+ apps crashes with SEGV (signal 11) or panics with "Invalid global state" on first boot. The second V8 isolate fails to initialize. Systemd auto-restarts and the second boot sometimes works (race timing).

**Root cause:** Each `v8_thread()` had its own `static V8_INIT: Once` calling `v8::V8::initialize_platform()` + `v8::V8::initialize()`. When two apps loaded simultaneously, there were **two separate `Once` statics** (one per call site) — they didn't coordinate. Even with a single `Once`, V8 internals weren't fully ready when the second thread immediately called `v8::Isolate::new()` after the `Once` returned, causing a SEGV in V8's native code.

**Fix:**
- Extracted a single `pub fn ensure_v8_initialized()` in `main.rs` with one `Once` static.
- `run_platform()` calls `ensure_v8_initialized()` **on the main thread before loading any apps**. V8 is fully initialized before any `v8_thread` spawns.
- `v8_thread()` also calls `ensure_v8_initialized()` (no-op since already done) for safety in non-platform (single-app) mode.

**Key lesson for AI agents:** V8 must be initialized on the main thread before spawning isolate threads. Use a single shared `Once` static, and call it before any `thread::spawn` that creates V8 isolates. Two separate `Once` statics in different functions do NOT coordinate.

**Commit:** `6409019`

---
