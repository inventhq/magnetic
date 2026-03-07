---
title: Benchmarks
description: Performance measurements vs Next.js, Remix, HTMX, and other frameworks.
layout: docs
order: 6
---

# Magnetic Performance Benchmarks

Measured against the shipment-tracker app (8 SSE events, interactive UI) running on the Magnetic platform server.

## Payload Sizes

| Asset | Raw | Gzip | Brotli |
|-------|-----|------|--------|
| HTML (SSR, complete page) | 25,303 B | ~4,200 B | 3,957 B |
| **Client runtime (total)** | **5,687 B** | **~2,700 B** | **2,340 B** |

### Framework Comparison — Client Runtime Size (Brotli)

| Framework | Client Runtime | Notes |
|-----------|---------------|-------|
| **Magnetic** | **2.3 KB** | No hydration, fixed size |
| Alpine.js | ~4.5 KB | Declarative only, no SSR |
| Svelte (compiled) | ~2-4 KB | Per-component, grows with app size |
| HTMX | ~14 KB | No SSE dedup |
| Preact | ~11 KB | Needs hydration |
| React + ReactDOM | ~45 KB | Needs hydration + client router |
| Next.js | ~80-120 KB | Framework + React + router + hydration |
| Remix | ~70-100 KB | Framework + React + hydration |

Magnetic's runtime is **fixed at 2.3 KB** regardless of app complexity — the server does all rendering.

## Time to First Byte (TTFB)

Measured locally:

| Request | Cold (first) | Warm (subsequent) |
|---------|-------------|-------------------|
| Initial page load | ~470 ms | **6-14 ms** |
| Action POST | ~15 ms | **0.7-1.2 ms** |

Cold start includes server initialization. Warm requests are sub-millisecond for actions because the server keeps your app loaded in memory.

### Framework Comparison — SSR TTFB

| Framework | Typical TTFB | Notes |
|-----------|-------------|-------|
| **Magnetic** | **6-14 ms** | Native server, no Node.js overhead |
| Next.js (Node) | 50-200 ms | Node.js + React renderToString |
| Remix (Node) | 40-150 ms | Node.js + React + loaders |
| HTMX + Go | 5-20 ms | Comparable, but no reactive updates |
| Rails | 30-100 ms | Ruby + ERB templating |

## Interaction Round-Trip

Single action flow: user click → POST → server reduce → DOM snapshot response → client patch.

| Metric | Magnetic | Typical SPA |
|--------|----------|-------------|
| Request payload | `{}` (empty or small JSON) | Varies |
| Response payload | 15,943 B (full DOM snapshot) | Varies (API JSON + client re-render) |
| Response (Brotli) | ~2.5 KB | Varies |
| Client processing | `JSON.parse` + keyed DOM patch | Virtual DOM diff + reconcile |
| Round-trips | **1** | Often 2+ (action + refetch) |

## SSE Real-Time Updates

| Metric | Value |
|--------|-------|
| Debounce interval | 150 ms |
| Max updates/sec | ~7 (debounced) |
| Per-update payload | ~15 KB raw / ~2.5 KB Brotli (full snapshot) |
| Dedup | Client-side hash + server-side event_id ring |
| Reconnect | Automatic with exponential backoff |

## Developer Experience (DX)

Lines of code for equivalent shipment-tracker app:

| Framework | Files | LOC | Client JS? | State management? |
|-----------|-------|-----|------------|-------------------|
| **Magnetic** | 3 | ~120 | **None** | Server only (state.ts) |
| Next.js | 5-8 | ~300 | Yes (hooks, effects) | Client + server |
| Remix | 5-8 | ~250 | Yes (hooks) | Loaders + actions |
| HTMX + Express | 4-6 | ~200 | Minimal | Server templates |
| React SPA | 8-12 | ~500 | All client | Redux/Zustand/etc |

Magnetic files:
- `pages/IndexPage.tsx` — UI components (TSX, runs in V8)
- `server/state.ts` — Business logic + data transforms
- `magnetic.json` — Config (data sources, SSE URL)

No client-side JavaScript to write. No hydration bugs. No state synchronization.

## Native SDKs — Zero Extra Code

The same server powers all platforms:

| Platform | SDK Size | Rendering | Protocol |
|----------|----------|-----------|----------|
| Web | 2.3 KB (Brotli) | DOM patching | SSE + POST |
| iOS/macOS | ~50 KB (Swift package) | Native SwiftUI | SSE + POST |
| Android | ~80 KB (AAR) | Native Jetpack Compose | SSE + POST |

**Developer writes zero native UI code.** Same TSX pages render as native widgets on all platforms.

## Summary

| Dimension | Magnetic | Best Alternative |
|-----------|----------|-----------------|
| Client runtime | **2.3 KB** | Svelte ~2 KB (but grows) |
| SSR TTFB (warm) | **6-14 ms** | HTMX+Rust ~5-20 ms |
| Hydration cost | **0 ms** (none) | Svelte ~10-50 ms |
| Wire bytes/action | **~2.5 KB** | Varies |
| Round-trips/action | **1** | Often 2+ |
| Client JS to write | **0 lines** | HTMX: minimal |
| Native mobile | **Built-in** | None (separate codebase) |

Magnetic's structural advantage: the server renders everything, the client is a thin patcher. This inverts the typical SPA model where the client does all the work.

---

← [Previous: Deployment](/deployment) · **Chapter 6** · End of docs
