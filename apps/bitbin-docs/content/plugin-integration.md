---
title: Plugin Integration
order: 12
---

# Chapter 12: Plugin Integration

**How the plugin runtime connects to BitBin's data layer.**

The plugin runtime interacts with three backends:

1. **SystemDataDO** — Per-tenant Durable Object with native SQLite for cross-plugin system tables (replaces Turso "Database A")
2. **PluginDataDO** — Per-tenant+plugin Durable Object with native SQLite for plugin-specific tables (replaces Turso "Database B")
3. **BitBin API** — The core engine for analytics, real-time subscriptions, vector search, and pipelines

```
Plugin Runtime
    │
    ├── System tables (registry,     ──→  SystemDataDO           ──→  DO-local SQLite
    │   credentials, poller_state)        (system-{tenant_id})        (one per tenant)
    │
    ├── rt.dbExec / rt.dbQuery       ──→  PluginDataDO           ──→  DO-local SQLite
    │   (per-plugin relational CRUD)      (plugin-{tenant}-{id})      (one per plugin)
    │
    ├── rt.bitbin.query / etc        ──→  BitBin API             ──→  Core engine
    │   (analytics, realtime)             (HTTP/WS)                   (https://{uuid}.machx.dev)
    │
    └── TenantHotBuffer DO                (already built, unchanged)
        └── bulk numeric ingest → R2 → BitBin
```

---

## 1. SystemDataDO — System Tables Spec

### Overview

`SystemDataDO` is a per-tenant Durable Object that hosts **cross-plugin system tables** — the control plane data that was previously in Turso "Database A." Each tenant gets one `SystemDataDO` instance with its own isolated SQLite.

### System Tables

| Table | Purpose | Access Pattern |
|---|---|---|
| `plugins` | Plugin registry (slug, version, active, config) | Read on every plugin load, write on install/update |
| `plugin_versions` | Version history per plugin | Read on deploy, write on publish |
| `plugin_state` | Cross-plugin KV state (cursor positions, sync tokens) | High-frequency read/write by pollers |
| `credentials` | Encrypted API keys, OAuth tokens per plugin | Read on plugin execution |
| `poller_state` | Poller last-run timestamps, intervals | Read/write by scheduler |
| `backfill_state` | Backfill progress tracking | Read/write during backfills |
| `ingest_tokens` | Per-plugin ingest auth tokens | Read on ingest requests |
| `views` | Saved view definitions | Read on dashboard load |
| `materializer_state` | Materializer checkpoints | Read/write by materializer |

### DO Naming & Routing

- **DO naming:** `system-{tenant_id}`
- **Worker route:** `POST /system-sql`
- **Headers:** `X-Tenant-Id` (required), `Authorization: Bearer {service_token}` (required)
- **No `X-Plugin-Id`** — system tables are cross-plugin by definition

### Endpoint Contract

Identical to PluginDataDO (same 4 actions: `execute`, `query`, `execute_batch`, `schema`). The only difference is the routing — the worker sends system SQL to the `SYSTEM_DATA` binding instead of `PLUGIN_DATA`.

```json
POST /system-sql
X-Tenant-Id: 42
Authorization: Bearer {service_token}

{
  "action": "query",
  "sql": "SELECT slug, version, active FROM plugins WHERE tenant_id = ? AND active = 1",
  "params": [42]
}
```

Response:
```json
{
  "ok": true,
  "columns": ["slug", "version", "active"],
  "rows": [
    ["agility-writer", "1.2.0", 1],
    ["spyfu-keywords", "2.0.1", 1],
    ["aftership-tracker", "1.0.0", 1]
  ],
  "rowsRead": 3
}
```

### Why a separate DO from PluginDataDO

- **Isolation:** Plugin data DOs are per-plugin. A misbehaving plugin can't corrupt the registry or other plugins' credentials.
- **Lifecycle:** System tables persist even when a plugin is uninstalled. Plugin data DOs can be destroyed on uninstall.
- **Access control:** System SQL comes from the runtime itself (trusted). Plugin SQL comes from plugin code (sandboxed via `rewrite_sql`).

### Wrangler Configuration

```toml
[durable_objects]
bindings = [
  { name = "TENANT_HOT_BUFFER", class_name = "TenantHotBuffer" },
  { name = "PLUGIN_DATA", class_name = "PluginDataDO" },
  { name = "SYSTEM_DATA", class_name = "SystemDataDO" }
]

[[migrations]]
tag = "v2"
new_sqlite_classes = ["PluginDataDO", "SystemDataDO"]
```

> **Note:** `SystemDataDO` and `PluginDataDO` can share the same DO class implementation (both just wrap `this.ctx.storage.sql`). The separation is in the **naming convention and wrangler binding**, not the code. If you prefer a single class, use one `DataDO` class with naming patterns: `system-{tenant_id}` and `plugin-{tenant_id}-{plugin_id}`.

### Mapping from Turso "Database A"

| Current (Turso) | New (SystemDataDO) | Changes |
|---|---|---|
| Turso HTTP pipeline API | `POST /system-sql` → DO stub | HTTP target only |
| Central DB for all tenants | One DO per tenant | Better isolation, edge-local |
| Network round-trip to Turso | DO-local SQLite | Lower latency |
| Turso billing | $0 (included in Workers) | Cost eliminated |

Existing SQL (all `CREATE TABLE`, `INSERT`, `SELECT` statements in the system tables) works identically — SQLite dialect is the same.

---

## 2. PluginDataDO — SQL Endpoint Spec

### Overview

`PluginDataDO` is a Cloudflare Durable Object that wraps the native `this.ctx.storage.sql` API. Each tenant+plugin combination gets its own DO instance with an isolated SQLite database. This is a **direct replacement for Turso** — same SQL dialect (SQLite), same parameterized queries, zero application-level changes.

### Worker Routing

The edge worker routes SQL requests to the correct DO based on tenant and plugin identity.

**URL:** `https://bitbin-edge.{your-worker-domain}.workers.dev/plugin-sql`

**Headers:**
- `X-Tenant-Id: {tenant_id}` (required)
- `X-Plugin-Id: {plugin_id}` (required)
- `Authorization: Bearer {service_token}` (required — internal service token, not end-user API key)

**DO naming convention:** `plugin-{tenant_id}-{plugin_id}`

### Actions

#### `execute` — Parameterized Write

Write operations (INSERT, UPDATE, DELETE, CREATE TABLE, etc.).

```json
POST /plugin-sql
{
  "action": "execute",
  "sql": "INSERT OR REPLACE INTO articles (id, title, body, status) VALUES (?, ?, ?, ?)",
  "params": ["abc123", "My Article", "Article body text...", "draft"]
}
```

Response:
```json
{
  "ok": true,
  "rowsWritten": 1,
  "rowsRead": 0
}
```

#### `query` — Parameterized Read

Read operations (SELECT).

```json
POST /plugin-sql
{
  "action": "query",
  "sql": "SELECT * FROM articles WHERE status = ?",
  "params": ["published"]
}
```

Response:
```json
{
  "ok": true,
  "columns": ["id", "title", "body", "status"],
  "rows": [
    ["abc123", "My Article", "Article body text...", "published"],
    ["def456", "Second Post", "More content...", "published"]
  ],
  "rowsRead": 2
}
```

#### `execute_batch` — Unparameterized Statement Batch

Used for DDL migrations and multi-statement operations.

```json
POST /plugin-sql
{
  "action": "execute_batch",
  "statements": [
    "CREATE TABLE IF NOT EXISTS articles (id TEXT PRIMARY KEY, title TEXT, body TEXT, status TEXT, created_at TEXT)",
    "CREATE INDEX IF NOT EXISTS idx_articles_status ON articles(status)",
    "CREATE TABLE IF NOT EXISTS sync_state (key TEXT PRIMARY KEY, value TEXT)"
  ]
}
```

Response:
```json
{
  "ok": true,
  "executed": 3
}
```

#### `schema` — Table Info

Equivalent to `PRAGMA table_info(table_name)`.

```json
POST /plugin-sql
{
  "action": "schema",
  "table": "articles"
}
```

Response:
```json
{
  "ok": true,
  "columns": [
    {"name": "id", "type": "TEXT", "notnull": false, "pk": true},
    {"name": "title", "type": "TEXT", "notnull": false, "pk": false},
    {"name": "body", "type": "TEXT", "notnull": false, "pk": false},
    {"name": "status", "type": "TEXT", "notnull": false, "pk": false},
    {"name": "created_at", "type": "TEXT", "notnull": false, "pk": false}
  ]
}
```

### Column Types

DO-local SQLite supports the full SQLite type system:

| Type | Description |
|------|-------------|
| `TEXT` | UTF-8 string |
| `INTEGER` | 64-bit signed integer |
| `REAL` | 64-bit IEEE floating point |
| `BLOB` | Binary data |
| `NULL` | Null value |

### Error Responses

```json
{
  "ok": false,
  "error": "SQLITE_CONSTRAINT: UNIQUE constraint failed: articles.id"
}
```

HTTP status codes:
- `200` — Success
- `400` — Malformed request (missing action, invalid SQL)
- `401` — Missing or invalid service token
- `500` — SQLite error (constraint violation, syntax error, etc.)

### DO Implementation Reference

The PluginDataDO handler is minimal (~40 lines). Here's the complete shape:

```typescript
export class PluginDataDO extends DurableObject {
  async fetch(request: Request): Promise<Response> {
    const { action, sql, params, statements, table } = await request.json();

    try {
      switch (action) {
        case "execute": {
          const cursor = this.ctx.storage.sql.exec(sql, ...(params || []));
          return Response.json({
            ok: true,
            rowsWritten: cursor.rowsWritten,
            rowsRead: cursor.rowsRead,
          });
        }

        case "query": {
          const cursor = this.ctx.storage.sql.exec(sql, ...(params || []));
          const rows = cursor.toArray();
          return Response.json({
            ok: true,
            columns: cursor.columnNames,
            rows: rows.map((r: any) => cursor.columnNames.map((c: string) => r[c])),
            rowsRead: cursor.rowsRead,
          });
        }

        case "execute_batch": {
          for (const stmt of statements) {
            this.ctx.storage.sql.exec(stmt);
          }
          return Response.json({ ok: true, executed: statements.length });
        }

        case "schema": {
          const cursor = this.ctx.storage.sql.exec(
            `PRAGMA table_info(${table})`
          );
          const cols = cursor.toArray();
          return Response.json({
            ok: true,
            columns: cols.map((c: any) => ({
              name: c.name,
              type: c.type,
              notnull: !!c.notnull,
              pk: !!c.pk,
            })),
          });
        }

        default:
          return Response.json({ ok: false, error: `unknown action: ${action}` }, { status: 400 });
      }
    } catch (err) {
      return Response.json({ ok: false, error: String(err) }, { status: 500 });
    }
  }
}
```

### Wrangler Configuration

Add to `wrangler.toml`:

```toml
[durable_objects]
bindings = [
  { name = "TENANT_HOT_BUFFER", class_name = "TenantHotBuffer" },
  { name = "PLUGIN_DATA", class_name = "PluginDataDO" }
]

[[migrations]]
tag = "v2"
new_sqlite_classes = ["PluginDataDO"]
```

### Mapping from TursoClient Trait

| TursoClient method | PluginDataDO action | Changes |
|---|---|---|
| `execute_params(sql, params)` | `{"action": "execute", "sql": ..., "params": ...}` | HTTP target only |
| `query_params(sql, params)` | `{"action": "query", "sql": ..., "params": ...}` | HTTP target only |
| `execute(statements)` | `{"action": "execute_batch", "statements": [...]}` | HTTP target only |
| `query(sql)` | `{"action": "query", "sql": ..., "params": []}` | HTTP target only |
| `init_tables()` | `execute_batch` with DDL statements | HTTP target only |

The `rewrite_sql` prefix isolation in `plugin_db.rs` continues to work as-is — it operates on SQL strings before they reach the client layer.

---

## 3. rt.bitbin — BitBin SDK Spec

### Overview

The `rt.bitbin` namespace exposes BitBin's core engine capabilities that SQLite cannot provide: sub-millisecond analytics, real-time subscriptions, vector search, graph queries, and atomic pipelines.

Each plugin gets a provisioned BitBin database at signup/deploy time:
- **Base URL:** `https://{subdomain}.machx.dev`
- **API Key:** Issued per-plugin via platform API

### Operations

#### `rt.bitbin.query(query)` — Analytics Query

```typescript
const result = await rt.bitbin.query({
  space: { tenant: 1, amount: [500, null] },
  measure: { type: "sum" },
  of: "amount"
});
// result: { count: 12000, sum: 5400000, query_us: 12 }
```

**HTTP equivalent:**
```
POST https://{subdomain}.machx.dev/query
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "space": {"tenant": 1, "amount": [500, null]},
  "measure": {"type": "sum"},
  "of": "amount"
}
```

**Response:**
```json
{"count": 12000, "sum": 5400000, "query_us": 12}
```

#### `rt.bitbin.ingest(records)` — Batch Ingest

```typescript
await rt.bitbin.ingest({
  records: [
    { tenant: 1, entity: 0, key_id: 100, amount: 4999, region: 3, currency: 1, category: 5, status: 1 },
    { tenant: 1, entity: 0, key_id: 101, amount: 2500, region: 1, currency: 1, category: 2, status: 0 }
  ]
});
```

**HTTP equivalent:**
```
POST https://{subdomain}.machx.dev/ingest
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "records": [
    {"tenant": 1, "entity": 0, "key_id": 100, "amount": 4999, "region": 3, "currency": 1, "category": 5, "status": 1}
  ]
}
```

**Response:**
```json
{"ingested": 2, "total_records": 50002, "ingest_us": 120}
```

#### `rt.bitbin.subscribe(query, callback)` — Real-Time Subscription

```typescript
const handle = rt.bitbin.subscribe(
  {
    space: { tenant: 1 },
    measure: { type: "count" }
  },
  (event) => {
    console.log(`Count changed: ${event.result.count}`);
  }
);

// Later:
handle.close();
```

**HTTP equivalent:** SSE stream via `POST /subscribe`

```
POST https://{subdomain}.machx.dev/subscribe
Authorization: Bearer {api_key}
Content-Type: application/json

{"space": {"tenant": 1}, "measure": {"type": "count"}}
```

**SSE events:**
```
event: result
data: {"event":"initial","fingerprint":"a1b2c3d4","result":{"count":42000,"query_us":9}}

event: result
data: {"event":"update","fingerprint":"b2c3d4e5","result":{"count":42001,"query_us":8}}
```

#### `rt.bitbin.subscribeMulti(queries, callback)` — Multiplexed Subscriptions

```typescript
const handle = rt.bitbin.subscribeMulti(
  {
    "total-count": { space: { tenant: 1 }, measure: { type: "count" } },
    "high-value":  { space: { tenant: 1, amount: [500, null] }, measure: { type: "sum" }, of: "amount" }
  },
  (event) => {
    console.log(`${event.id}: count=${event.result.count}`);
  }
);
```

**HTTP equivalent:** `POST /subscribe-multi`

#### `rt.bitbin.vectorSearch(options)` — Vector Similarity Search

```typescript
const results = await rt.bitbin.vectorSearch({
  tenant: 1,
  embedding: [0.12, -0.45, 0.78, ...],  // float array
  entity: 4,  // ENTITY_VECTOR_TEXT
  topK: 10
});
// results: [{ vector_id: 42, distance: 12 }, ...]
```

**HTTP equivalent:**
```
POST https://{subdomain}.machx.dev/vector/search
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "tenant": 1,
  "embedding": [0.12, -0.45, 0.78, ...],
  "entity": 4,
  "top_k": 10
}
```

#### `rt.bitbin.pipeExecute(slug)` — Execute Saved Query (Pipe)

```typescript
const result = await rt.bitbin.pipeExecute("revenue-by-region");
// result: { count: 8000, sum: 3200000, groups: [...] }
```

**HTTP equivalent:** `GET /pipe/{slug}`

#### `rt.bitbin.pipelineCall(slug, params?)` — Call Named Pipeline

```typescript
await rt.bitbin.pipelineCall("process-order", {
  // Pipeline-specific parameters
});
```

**HTTP equivalent:** `POST /pipeline/{slug}/call`

#### `rt.bitbin.pipeStream(slug, callback)` — Stream Pipe Updates

```typescript
const handle = rt.bitbin.pipeStream("live-dashboard", (data) => {
  console.log("Dashboard update:", data);
});
```

**HTTP equivalent:** SSE stream via `GET /pipe/{slug}/stream`

#### `rt.bitbin.doc` — Document Store

```typescript
// Write
await rt.bitbin.doc.set({ tenant: 1, entity: 0, doc_id: "order-123", data: { items: ["widget"], total: 4999 } });

// Read
const doc = await rt.bitbin.doc.get({ tenant: 1, entity: 0, doc_id: "order-123" });

// Delete
await rt.bitbin.doc.delete({ tenant: 1, entity: 0, doc_id: "order-123" });

// List
const docs = await rt.bitbin.doc.list({ tenant: 1, entity: 0 });
```

**HTTP equivalents:** `POST /doc`, `GET /doc`, `DELETE /doc`, `GET /docs`

### TypeScript Types

```typescript
interface BitBinQuery {
  space: Record<string, number | [number | null, number | null]>;
  measure: { type: "count" } | { type: "sum" } | { type: "min" } | { type: "max" } | { type: "group" };
  of?: string;
  union?: Record<string, number | [number | null, number | null]>[];
  join?: Record<string, number | [number | null, number | null]>[];
  limit?: number;
}

interface BitBinQueryResult {
  count: number;
  sum?: number;
  min?: number;
  max?: number;
  groups?: Array<{ key: number; count: number; sum: number }>;
  query_us: number;
}

interface BitBinIngestRequest {
  records: Array<{
    tenant: number;
    entity: number;
    key_id: number;
    amount: number;
    region?: number;
    currency?: number;
    category?: number;
    status?: number;
  }>;
}

interface BitBinSubscriptionEvent {
  id?: string;              // query ID (for subscribeMulti)
  event: "initial" | "update";
  fingerprint: string;
  result: BitBinQueryResult;
}

interface BitBinVectorSearchRequest {
  tenant: number;
  embedding: number[];
  entity?: number;          // default: 4 (text)
  top_k?: number;           // default: 10
}

interface BitBinVectorMatch {
  vector_id: number;
  distance: number;
  slot: number;
}

interface SubscriptionHandle {
  close(): void;
}

interface BitBinClient {
  query(q: BitBinQuery): Promise<BitBinQueryResult>;
  ingest(req: BitBinIngestRequest): Promise<{ ingested: number; total_records: number; ingest_us: number }>;
  subscribe(q: BitBinQuery, cb: (event: BitBinSubscriptionEvent) => void): SubscriptionHandle;
  subscribeMulti(queries: Record<string, BitBinQuery>, cb: (event: BitBinSubscriptionEvent) => void): SubscriptionHandle;
  vectorSearch(req: BitBinVectorSearchRequest): Promise<BitBinVectorMatch[]>;
  pipeExecute(slug: string): Promise<BitBinQueryResult>;
  pipelineCall(slug: string, params?: any): Promise<any>;
  pipeStream(slug: string, cb: (data: any) => void): SubscriptionHandle;
  doc: {
    set(opts: { tenant: number; entity: number; doc_id: string; data: any }): Promise<void>;
    get(opts: { tenant: number; entity: number; doc_id: string }): Promise<any>;
    delete(opts: { tenant: number; entity: number; doc_id: string }): Promise<void>;
    list(opts: { tenant: number; entity: number }): Promise<any[]>;
  };
}
```

### Authentication

Each plugin receives its BitBin credentials at deploy time:

```typescript
// Injected by the runtime at plugin initialization
const bitbin = new BitBinClient({
  baseUrl: "https://edba2ddef755.machx.dev",  // provisioned subdomain
  apiKey: "bb_live_abc123..."                   // scoped API key
});

// Exposed as rt.bitbin
rt.bitbin = bitbin;
```

The plugin author never constructs URLs or manages API keys — the runtime handles provisioning via the [Platform API](./platform.md) (`POST /accounts/:id/databases`).

---

## 4. Auto-Discovery — Edge URL & Service Token

Both the **edge URL** and the **service token** are generated automatically during database provisioning. The plugin-runtime never needs to ask for them manually.

### Provisioning Response

When `POST /accounts/:id/databases` is called, the response includes everything the plugin-runtime needs:

```json
{
  "db_id": "acme-analytics",
  "subdomain": "edba2ddef755",
  "api_key": "bb_live_abc123...",
  "edge_service_token": "bb_live_edge_xyz789...",
  "endpoints": {
    "connection_url": "https://edba2ddef755.machx.dev",
    "rest": "https://edba2ddef755.machx.dev/query",
    "ws": "wss://edba2ddef755.machx.dev/ws",
    "sse": "https://edba2ddef755.machx.dev/subscribe",
    "ingest": "https://edba2ddef755.machx.dev/ingest",
    "pipes": "https://edba2ddef755.machx.dev/pipe",
    "edge": "https://bitbin-edge.workers.dev"
  },
  "note": "Save the api_key and edge_service_token — they will not be shown again."
}
```

### What the plugin-runtime needs

| Env Var | Source | Value |
|---|---|---|
| `DO_SQLITE_URL` | `endpoints.edge` | `https://bitbin-edge.workers.dev` |
| `DO_SQLITE_SERVICE_TOKEN` | `edge_service_token` | `bb_live_edge_xyz789...` |
| `BITBIN_BASE_URL` | `endpoints.connection_url` | `https://edba2ddef755.machx.dev` |
| `BITBIN_API_KEY` | `api_key` | `bb_live_abc123...` |

### How `do_sqlite.rs` uses these

The edge URL is **agnostic** — a single base URL for all DO-backed SQL. The plugin-runtime appends the path based on context:

- **System tables** → `POST {DO_SQLITE_URL}/system-sql` (headers: `X-Tenant-Id`)
- **Plugin data** → `POST {DO_SQLITE_URL}/plugin-sql` (headers: `X-Tenant-Id`, `X-Plugin-Id`)

Both paths use `Authorization: Bearer {DO_SQLITE_SERVICE_TOKEN}`. The edge worker validates the token, resolves the correct DO by name, and forwards the request.

### Server-side env var

The Platform API reads `EDGE_URL` to know the edge worker's base URL:

```bash
# Set once on the Civo server (or in the deployment config)
EDGE_URL=https://bitbin-edge.<account>.workers.dev
```

This gets baked into every provisioned database's `endpoints.edge` field automatically.

---

## 5. What Changes Where

| Component | Owner | Work |
|---|---|---|
| `SystemDataDO` — DO class (~40 lines) | BitBin team (CF Worker) | ✅ Done — `edge/src/system-data-do.ts` |
| `PluginDataDO` — DO class (same impl) | BitBin team (CF Worker) | ✅ Done — `edge/src/plugin-data-do.ts` |
| Worker routing for `/system-sql` + `/plugin-sql` | BitBin team (CF Worker) | ✅ Done — `edge/src/index.ts` |
| Provisioning: `edge` endpoint + `edge_service_token` | BitBin team (Platform API) | ✅ Done — auto-generated on `POST /accounts/:id/databases` |
| `do_sqlite.rs` — replaces `turso.rs` | Plugin-runtime team | Same trait surface, routes to SystemDataDO or PluginDataDO |
| `bitbin_client.rs` + sandbox ops | Plugin-runtime team | New module wrapping BitBin HTTP API |
| `plugin_db.rs` | Nobody | Zero changes |
| `plugin_state.rs` | Nobody | Zero changes |
| `plugin_registry.rs` | Nobody | Zero changes — SQL targets SystemDataDO instead of Turso |
| SDK types (`plugin-runtime.d.ts`) | Plugin-runtime team | Add `rt.bitbin` types |
| Existing plugins | Nobody | Zero changes |

---

[← Previous: SQL Migration Guide](./sql-migration.md) · **Chapter 12** · [Back to Introduction →](./index.md)
