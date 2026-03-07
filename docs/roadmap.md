# Roadmap

> Future work items and design notes for Magnetic platform development.

## Active Backlog

### R2 Content Storage (Low Priority)
Store `.md` content files in Cloudflare R2 instead of baking them into the bundle or reading from local disk. This would allow content updates without redeploy — just upload a new `.md` file to R2.

**Applies to**:
- SSG lazy content mode (load from R2 during build)
- SSR runtime content loading (fetch from R2 on request)

**Design sketch**:
- `magnetic.json` gets a `"content": { "source": "r2", "bucket": "..." }` field
- CLI uploads content to R2 during `magnetic push`
- Server fetches content from R2 on `getContent(slug)` calls (with local cache)

### GLiNER Integration (Low Priority)
Entity extraction for structured data from unstructured text. Would enable automatic tagging, categorization, and search indexing of content.

### Bot/Flood Rate Limiter Hardening
Basic rate limiting middleware exists. Needs:
- Per-app configurable limits
- IP-based throttling
- Graceful 429 responses with Retry-After headers

### JSON Payload Compression
Compress DOM snapshot payloads (SSE and action responses) with Brotli for large pages. Currently sent as raw JSON (~15KB per snapshot for typical apps).

---

## Future: CRDTs / Operational Transforms for Collaborative State

### Current Design (Per-Session State)

Each SSE client gets its own isolated state:
- `Map<session_id, State>` in V8 — one state per browser session
- Actions from User A only affect User A's state
- SSE broadcasts only go to the originating session
- Session cleanup on SSE disconnect + 30-minute GC reaper

This is correct for single-user apps (dashboards, admin panels, personal tools).

### When CRDTs/OTs Would Be Needed

If Magnetic wants to support **collaborative features** (Google Docs-style real-time editing, shared whiteboards, multiplayer games), per-session state is insufficient. Multiple users need to:

1. **Share state** — see each other's changes in real-time
2. **Resolve conflicts** — concurrent edits to the same data must merge
3. **Maintain consistency** — all clients converge to the same state

### Option A: CRDTs (Conflict-free Replicated Data Types)
- Each piece of shared state uses a CRDT (e.g., LWW-Register, G-Counter, OR-Set)
- Merges are automatic and commutative — no central coordination needed
- Best for: collaborative text editing, shared counters, presence indicators
- Libraries: Yjs, Automerge (JS), diamond-types (Rust)

### Option B: Operational Transforms (OT)
- Operations are transformed against concurrent operations before applying
- Requires a central server to determine operation ordering
- Best for: text editing (Google Docs approach)
- More complex than CRDTs but well-understood

### Option C: Hybrid (Recommended for Magnetic)
- **Per-session state** (current) for private UI state (form inputs, filters, navigation)
- **Shared CRDT state** for collaborative data (shared lists, documents, boards)
- Developer declares which state keys are "shared" in `magnetic.json`:
  ```json
  {
    "state": {
      "mode": "hybrid",
      "shared": ["tasks", "board"],
      "private": ["filter", "selectedId"]
    }
  }
  ```
- Rust server maintains CRDT merge logic; V8 bridge merges shared + private state before `toViewModel()`

### Implementation Sketch

1. Add `SharedState` struct in Rust with CRDT-backed fields
2. Actions targeting shared keys broadcast merged state to ALL sessions
3. Actions targeting private keys only affect the originating session (current behavior)
4. V8 bridge: `toViewModel(Object.assign({}, sharedState, sessionState))`

### Priority

Low — no immediate need. Per-session state covers current use cases. CRDT support is a future differentiator for collaborative apps.
