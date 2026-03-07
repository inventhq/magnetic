# Deployment & Operations

> Internal reference for infrastructure setup, production deployment, and known fixes.

## Infrastructure

### Production Server
- **Provider**: Civo VPS (g3.medium, NYC1)
- **IP**: 212.2.244.246
- **Binary**: `/usr/local/bin/magnetic-v8-server`
- **Service**: `magnetic-platform` (systemd)
- **Data dir**: `/var/lib/magnetic/apps`

### Build Server
- **Provider**: Civo VPS (g4c.medium, 16 CPU / 32GB RAM, NYC1)
- **IP**: 212.2.246.140
- **Repo**: `/root/magnetic`
- **Build time**: ~25s for release binary

### DNS (Cloudflare)
- Zone ID: `987ab884c229829716c90f4a5dd21b55`
- A records: `fujs.dev`, `*.fujs.dev`, `api.fujs.dev` → 212.2.244.246
- CF_API_TOKEN in `/etc/magnetic/control-plane.env`

## Architecture

```
Internet
   │
   ▼
Caddy (TLS, reverse proxy)
   │  *.fujs.dev → X-Subdomain header → localhost:3003
   │  api.fujs.dev → localhost:3003 (control plane)
   │
   ▼
magnetic-v8-server --platform --port 3003
   │  Multi-tenant: per-app V8 isolate
   │  SSR, SSE, actions, static files
   │
   ▼
/var/lib/magnetic/apps/
   ├── app-a/   (bundle, assets, config, prerender/)
   ├── app-b/
   └── ...
```

## Build & Deploy Pipeline

### Binary Deploy Steps
1. `git push` from local machine
2. On build server: `git pull && cargo build --release` (in `rs/crates/magnetic-v8-server`)
3. SCP binary: build server → local → production node
4. On production: `sudo systemctl stop magnetic-platform && sudo cp /tmp/magnetic-v8-server /usr/local/bin/ && sudo systemctl start magnetic-platform`

**Important**: `transport.wasm` is gitignored. Must exist at `rs/crates/magnetic-v8-server/assets/transport.wasm` on the build server before compiling.

### App Deploy Steps
```bash
# From developer machine
magnetic push --name my-app --server https://api.fujs.dev
```

This POSTs the bundle + assets + config to the platform. The server hot-reloads the app without restart.

## Caddy Configuration

### Key Fixes Applied
1. **Disabled dynamic Caddy config push** in control plane (commit `e8efb80`) — the admin API push replaced the entire Caddyfile, wiping wildcard TLS and triggering per-domain cert storms.
2. **Removed `encode gzip zstd` from api.fujs.dev** (commit `f67561e`) — it caused 502 on deploy POSTs by interfering with the request/response cycle.

## Scaling Strategy

### Phase 1: Single Node (Current)
- One VPS running `magnetic-v8-server --platform`
- Caddy reverse proxy on port 443 → localhost:3003
- Good for 50-100 apps with light traffic

### Phase 2: Process Isolation
- Systemd service per app group (10 apps per process)
- Caddy routes by subdomain → correct process port

### Phase 3: Horizontal Scale
- Multiple VPS nodes
- Sticky sessions via Caddy upstream hash
- Shared storage via S3-compatible object store

## Pricing Model (Planned)

| Tier | Requests/mo | Apps | Storage | Price |
|------|-------------|------|---------|-------|
| Free | 10K | 1 | 10MB | $0 |
| Pro | 100K | 10 | 100MB | $5/mo |
| Scale | 1M | Unlimited | 1GB | $25/mo |

## Production Fixes Log

### 2026-02-18: Subdomain double-prefix 404

**Symptom**: magnetic.js and transport.wasm returned 404 on subdomain-routed apps.

**Root cause**: SSR emitted `<script src="/apps/{name}/magnetic.js">`. Caddy already prepended `/apps/{name}/`, causing double prefix → 404.

**Fix**: Platform detects `X-Subdomain` header and emits root-relative paths (`/magnetic.js`) instead. Commit `acf8643`.

### 2026-02-18: V8 parking kills isolate

**Symptom**: After 5 min idle, app returns 502. `PoisonError` from V8 global platform init.

**Root cause**: Park implementation dropped the mpsc sender, killing the V8 thread. V8's global platform can only be initialized once per process.

**Fix**: Park sets an AtomicBool flag only — thread stays alive, blocking on recv(). All `send().unwrap()` replaced with `.is_err()` checks returning 503. Commit `62bea38`.

### 2026-02-19: V8 "Invalid global state" SEGV on multi-app startup

**Symptom**: Platform with 2+ apps crashes with SEGV on first boot.

**Root cause**: Two separate `Once` statics in different call sites — they didn't coordinate V8 init.

**Fix**: Single `ensure_v8_initialized()` called on main thread before any app loads. Commit `6409019`.

## Security Checklist

- [x] V8 isolates: no filesystem access from JS
- [x] Path traversal prevention in platform.rs
- [x] No cross-app state leakage (separate V8 threads)
- [ ] Bundle size limit (5MB per app)
- [ ] Request body size limit (1MB)
- [ ] Rate limiting per app (basic middleware exists)
