# Civo VPS Deployment Plan

## Overview

Instead of Fly.io or Render, we self-host on a Civo VPS. The goal is to run
the Magnetic platform server (`magnetic-v8-server --platform`) behind a
lightweight scaling layer that enables serverless-style pricing for developers.

## Architecture

```
Internet
   │
   ▼
┌─────────────┐
│  Caddy / Nginx  │  TLS termination, reverse proxy
│  (edge proxy)   │  Rate limiting, request routing
└──────┬──────┘
       │
       ▼
┌─────────────────────────┐
│  magnetic-v8-server     │  --platform mode
│  (multi-tenant)         │  Per-app V8 isolate
│                         │  SSE, actions, static files
│  Apps:                  │
│    /apps/app-a/         │
│    /apps/app-b/         │
│    /apps/app-c/         │
└─────────────────────────┘
       │
       ▼
  data/apps/              Persistent storage (app bundles + assets)
```

## Civo Setup

1. **VPS**: Civo Medium (2 vCPU, 4GB RAM) — ~$20/month
   - Sufficient for dozens of small apps in platform mode
   - V8 isolates are lightweight (~10MB each)

2. **Storage**: Civo Volumes for `data/apps/` persistence

3. **DNS**: Point `*.magnetic.app` → VPS IP (wildcard A record)

4. **TLS**: Caddy auto-HTTPS with Let's Encrypt

## Scaling Strategy

### Phase 1: Single Node (Now)
- One VPS running `magnetic-v8-server --platform`
- Caddy reverse proxy on port 443 → localhost:3003
- Good for 50-100 apps with light traffic

### Phase 2: Process Isolation
- Systemd service per app group (10 apps per process)
- Caddy routes by subdomain → correct process port
- Auto-restart on crash via systemd

### Phase 3: Horizontal Scale
- Multiple Civo VPS nodes
- Sticky sessions via Caddy upstream hash
- Shared storage via Civo Volumes or S3-compatible object store
- Deploy API routes to any node, bundles synced to all

## Serverless Pricing Model

To offer serverless-style pricing:

1. **Idle Detection**: If an app receives no requests for 15 minutes,
   its V8 isolate is paused (thread parked, memory released)

2. **Cold Start**: On next request, reload bundle from disk → new V8 thread
   (~200ms cold start, acceptable for server-driven UI)

3. **Metering**: Count per-app:
   - Requests/month
   - V8 execution time (ms)
   - SSE connections (concurrent)
   - Storage (bundle + assets size)

4. **Pricing Tiers**:
   - **Free**: 10K requests/mo, 1 app, 10MB storage
   - **Pro** ($5/mo): 100K requests/mo, 10 apps, 100MB storage
   - **Scale** ($25/mo): 1M requests/mo, unlimited apps, 1GB storage

## Implementation Tasks

- [ ] Caddy config with reverse proxy + auto-HTTPS
- [ ] Systemd service file for magnetic-v8-server
- [ ] Idle detection + V8 thread parking in platform.rs
- [ ] Request metering middleware (per-app counters)
- [ ] Usage API: GET /api/apps/:name/usage
- [ ] Billing integration (Stripe)
- [ ] Subdomain routing: app-name.magnetic.app → /apps/app-name/
- [ ] CI/CD: GitHub Actions → build release binary → deploy to VPS

## Caddy Config (Draft)

```caddyfile
{
    email admin@magnetic.app
}

magnetic.app {
    reverse_proxy localhost:3003
}

*.magnetic.app {
    @app header_regexp Host {re.app}.magnetic.app
    reverse_proxy @app localhost:3003 {
        header_up X-Magnetic-App {re.app.1}
    }
}
```

## Security Checklist

- [ ] V8 isolates: no filesystem access from JS
- [ ] Bundle size limit (5MB per app)
- [ ] Request body size limit (1MB)
- [ ] Rate limiting per app (already in middleware)
- [ ] Path traversal prevention (already in platform.rs)
- [ ] No cross-app state leakage (separate V8 threads)
