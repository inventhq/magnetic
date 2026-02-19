# Magnetic Cloud — Deployment Guide

## Architecture

```
Internet → Caddy (:443) → *.magnetic.app wildcard TLS
                │
                ├── api.magnetic.app → Control Plane (:4000)
                │                      Rust + Turso (libSQL)
                │                      Auth, app registry, deploy orchestration
                │
                └── {app}.magnetic.app → Node (:3003)
                                         magnetic-v8-server --platform
                                         Per-app V8 isolate + SSE + static files
```

## Components

| Service | Binary | Port | Description |
|---------|--------|------|-------------|
| Control Plane | `magnetic-control-plane` | 4000 | App registry, auth, deploy proxy, node management |
| Platform Node | `magnetic-v8-server --platform` | 3003 | Multi-tenant V8 hosting |
| Caddy | `caddy` | 443/80 | TLS termination, subdomain routing |

## Quick Start (Single Server)

### 1. Build binaries

```bash
# Control plane
cd rs/crates/magnetic-control-plane
cargo build --release
# → target/release/magnetic-control-plane

# Platform server
cd rs/crates/magnetic-v8-server
cargo build --release
# → target/release/magnetic-v8-server
```

### 2. Configure environment

```bash
cp deploy/control-plane.env.example /etc/magnetic/control-plane.env
# Edit: set TURSO_URL, TURSO_TOKEN, CIVO_API_KEY, CF_API_TOKEN
```

### 3. Install systemd services

```bash
sudo cp deploy/magnetic-control-plane.service /etc/systemd/system/
sudo cp deploy/magnetic-platform.service /etc/systemd/system/
sudo mkdir -p /var/lib/magnetic/apps
sudo systemctl daemon-reload
sudo systemctl enable --now magnetic-control-plane
sudo systemctl enable --now magnetic-platform
```

### 4. Install Caddy with DNS plugin

```bash
# Build Caddy with Cloudflare DNS module
xcaddy build --with github.com/caddy-dns/cloudflare
sudo mv caddy /usr/local/bin/
sudo cp deploy/Caddyfile /etc/caddy/Caddyfile
sudo systemctl enable --now caddy
```

### 5. DNS

Point these at your server IP:
- `magnetic.app` → A record
- `*.magnetic.app` → A record (wildcard)
- `api.magnetic.app` → A record

### 6. Register the local node

```bash
curl -X POST http://localhost:4000/api/nodes \
  -H 'Content-Type: application/json' \
  -d '{"ip": "127.0.0.1", "port": 3003, "region": "LON1"}'
```

### 7. Deploy an app

```bash
# From developer machine
magnetic login --server https://api.magnetic.app
magnetic push --name my-app
# → ✓ Live at https://{app-id}.magnetic.app
```

## Local Development (No Caddy)

```bash
# Terminal 1: Control plane (uses local SQLite)
cd rs/crates/magnetic-control-plane
cargo run
# → http://localhost:4000

# Terminal 2: Platform node
cd rs/crates/magnetic-v8-server
cargo run -- --platform --port 3003 --data-dir data/apps

# Register the local node
curl -X POST http://localhost:4000/api/nodes \
  -H 'Content-Type: application/json' \
  -d '{"ip": "127.0.0.1", "port": 3003}'

# Register + deploy
curl -X POST http://localhost:4000/api/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"email": "dev@test.com"}'
# → returns api_key

magnetic push --server http://localhost:4000 --name my-app --key mk_...
```

## API Reference

### Auth

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/auth/register` | — | Register / get API key |
| POST | `/api/auth/keys` | Bearer | Create additional API key |
| GET | `/api/auth/me` | Bearer | Current user info |

### Apps

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/deploy` | Bearer | Deploy app (bundle + assets) |
| GET | `/api/apps` | Bearer | List user's apps |
| GET | `/api/apps/:id` | Bearer | App details |
| DELETE | `/api/apps/:id` | Bearer | Delete app |

### Nodes

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/nodes` | — | List all nodes |
| POST | `/api/nodes` | — | Register node manually |
| POST | `/api/nodes/provision` | — | Provision node via Civo |
| DELETE | `/api/nodes/:id` | — | Remove + destroy node |

### Internal / Caddy

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/resolve/:subdomain` | Resolve subdomain → upstream |
| GET | `/api/tls/check?domain=x` | on_demand_tls validation |
| POST | `/api/caddy/sync` | Force Caddy route rebuild |
| GET | `/health` | Health check |

### Platform Node (per-node API)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/apps/:name/deploy` | Deploy bundle to this node |
| GET | `/api/apps/:name/status` | V8 isolate status (warm/parked) |
| GET | `/api/apps` | List apps on this node |
| GET | `/apps/:name/` | SSR app page |
| GET | `/apps/:name/sse` | SSE stream |
| POST | `/apps/:name/actions/:action` | Dispatch action |

## V8 Isolate Parking

Apps idle for 5 minutes (configurable via `--park-idle <secs>`) with zero SSE
clients have their V8 thread dropped. On next request, the bundle is reloaded
from disk (~50-200ms cold start). This enables serverless-style density — a 4GB
node can hold ~300 warm apps or thousands of parked apps.

## Civo Auto-Provisioning

When `CIVO_API_KEY` is set, the control plane automatically provisions new Civo
instances when all existing nodes are at capacity. New instances run Ubuntu 22.04
with `magnetic-v8-server --platform` installed via init script.

**Why plain instances, not K3s:** Each node runs a single binary. Our control
plane handles scheduling and routing. K3s adds ~200MB overhead per node with no
benefit for this architecture.

## Pricing Tiers

| Tier | Price | Requests/mo | Apps | SSE Clients |
|------|-------|-------------|------|-------------|
| Free | $0 | 10K | 3 | 5 concurrent |
| Pro | $5/mo | 100K | 20 | 50 concurrent |
| Scale | $25/mo | 1M | unlimited | 500 concurrent |
