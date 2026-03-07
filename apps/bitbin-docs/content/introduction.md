---
title: BitBin
order: 0
---

# BitBin Documentation

**BitBin is a real-time database platform with built-in analytics, vector search, graph queries, reactive subscriptions, and atomic pipelines.**

One database replaces your entire backend stack. No glue code, no data pipelines, no infra to manage.

---

## What BitBin Replaces

| You Currently Use | BitBin Feature | What Changes |
|---|---|---|
| ClickHouse / BigQuery / Polars | Query Engine | Sub-millisecond aggregations over millions of records |
| Tinybird / Materialize / RisingWave | Pipes + Pipelines | Saved queries with SSE streaming and triggers |
| Pinecone / Qdrant / pgvector | Vector Search | Similarity search over text, image, and audio embeddings |
| Neo4j / TigerGraph | Graph Queries | N-ary hypergraph relationships as compact records |
| Pusher / Ably / custom WS | WebSocket + SSE | Real-time pub/sub with fingerprint-based change detection |
| Supabase / PlanetScale | Keyed CRUD + Pipelines | O(1) insert/lookup/delete with atomic transactions |
| Convex / Firebase Realtime | Reactive Subscriptions | Push updates only when query results actually change |

---

## Architecture Overview

```
Your App / SDK
    │
    ├── HTTPS ──→  Edge (Cloudflare Workers)  ──→  Compute (Bare-Metal)
    │                 ├── Write buffer (DO)            ├── Query engine
    │                 ├── Plugin SQL (DO)               ├── Pipelines
    │                 └── R2 (durable storage)          ├── Subscriptions
    │                                                   └── WebSocket server
    │
    └── WSS ───→  WebSocket (binary protocol)  ──→  Real-time queries,
                  Single persistent connection       mutations, subscriptions
```

- **Edge layer** — Cloudflare Workers handle writes, plugin state, and global distribution
- **Compute layer** — Bare-metal servers run the query engine with hardware-accelerated operations
- **Storage** — R2 provides durable cold storage with automatic flush intervals

Every database instance gets a unique subdomain: `https://{uuid}.machx.dev`

---

## Chapters

| # | Chapter | Description |
|---|---|---|
| 1 | [Getting Started](/getting-started) | Sign up, provision a database, first query in 5 minutes |
| 2 | [Core Concepts](/concepts) | Schema model, tenants, entities, dimensions, record layout |
| 3 | [Query Language](/query-language) | Filters, measures, joins, unions, group-by, top-k |
| 4 | [Data Operations](/data-operations) | Ingest, keyed CRUD, batch mutations, schema management |
| 5 | [Real-Time Subscriptions](/realtime) | SSE push, multiplexed queries, pipe streams |
| 6 | [WebSocket Protocol](/websocket) | Binary protocol for SDKs and high-throughput apps |
| 7 | [Pipelines](/pipelines) | Atomic multi-step operations, triggers, stored procedures |
| 8 | [Examples & Recipes](/examples) | Common patterns: e-commerce, IoT, dashboards, SaaS metrics |
| 9 | [Platform API](/platform) | Authentication, accounts, databases, teams, API keys |
| 10 | [API Reference](/api-reference) | Complete HTTP endpoint catalog |
| 11 | [SQL Migration Guide](/sql-migration) | Coming from SQL? Every concept mapped to BitBin |
| 12 | [Plugin Integration](/plugin-integration) | Plugin runtime ↔ BitBin data layer spec |

---

## Quick Links

- **First time?** Start with [Getting Started](/getting-started)
- **Coming from SQL?** Read the [SQL Migration Guide](/sql-migration)
- **Building a plugin?** See [Plugin Integration](/plugin-integration)
- **Need the full API?** Jump to [API Reference](/api-reference)

---

## SDKs

| Language | Package | Install |
|---|---|---|
| TypeScript | [`@bitbin/client`](https://www.npmjs.com/package/@bitbin/client) | `npm install @bitbin/client` |
| Python | [`bitbin`](https://pypi.org/project/bitbin/) | `pip install bitbin` |

---

## Domains

| Environment | Platform API | Database Instances |
|---|---|---|
| Production | `https://machx.dev` | `https://{uuid}.machx.dev` |
| Development | `https://wtfdb.dev` | `https://{uuid}.wtfdb.dev` |

---

[Next: Getting Started →](/getting-started)
