---
title: API Reference
order: 10
---

# Chapter 10: API Reference

**Complete HTTP endpoint reference for the BitBin API server.**

Base URL: `https://{subdomain}.machx.dev` (your database instance) or `http://localhost:3000` (local dev)

All endpoints accept `Authorization: Bearer YOUR_API_KEY` header for authentication.

> For account management, authentication, database provisioning, and team management, see [Platform API](/platform).

---

## Data Operations

### POST /query — Execute Query

Run a query against your database.

```bash
curl -X POST /query -H 'Content-Type: application/json' -d '{
  "space": {"tenant": 1, "amount": [500, null]},
  "measure": {"type": "sum"},
  "of": "amount"
}'
```

Response:
```json
{"count": 12000, "sum": 5400000, "query_us": 12}
```

See [Query Language Reference](/query-language) for full DSL.

### POST /ask — Natural Language Query

Convert natural language to a structured query and execute.

```bash
curl -X POST /ask -d '{"text": "total revenue from region 3"}'
```

Response:
```json
{
  "query": {"space": {"region": 3}, "measure": {"type": "sum"}, "of": "amount"},
  "result": {"count": 8000, "sum": 3200000, "query_us": 9}
}
```

### POST /ingest — Batch Ingest

Ingest a batch of records.

```bash
curl -X POST /ingest -d '{"count": 1000}'
```

Response:
```json
{"ingested": 1000, "total_records": 101000, "ingest_us": 450}
```

### DELETE /record — Delete Record

Delete a single record by slot ID.

```bash
curl -X DELETE "/record?slot=42"
```

---

## Saved Queries (Pipes)

### POST /pipe — Create/Update Pipe

```bash
curl -X POST /pipe -d '{
  "slug": "revenue-by-region",
  "query": {"space": {"tenant": 1}, "measure": {"type": "group"}},
  "description": "Revenue breakdown by tenant",
  "poll_ms": 1000
}'
```

### GET /pipe/:slug — Execute Pipe

```bash
curl /pipe/revenue-by-region
```

### GET /pipe/:slug/stream — SSE Stream

```bash
curl /pipe/revenue-by-region/stream
```

Returns Server-Sent Events. Only pushes when data changes.

### DELETE /pipe/:slug — Delete Pipe

```bash
curl -X DELETE /pipe/revenue-by-region
```

### GET /pipes — List All Pipes

```bash
curl /pipes
```

---

## Named Pipelines (Stored Procedures)

### POST /pipeline — Create/Update Pipeline

```bash
curl -X POST /pipeline -d '{
  "slug": "bank-transfer",
  "trigger": "on_call",
  "entity_filter": 0,
  "tenant_filter": null,
  "steps": [
    {"Lookup": {"name": "sender", "predicates": [[0, 2, 4, 1]]}},
    {"Lookup": {"name": "receiver", "predicates": [[0, 2, 4, 2]]}},
    {"ValidateGte": {"slot_ref": "sender", "col_offset": 6, "col_width": 2, "min_value": 3000}},
    {"UpdateDelta": {"slot_ref": "sender", "col_offset": 6, "col_width": 2, "delta": -3000}},
    {"UpdateDelta": {"slot_ref": "receiver", "col_offset": 6, "col_width": 2, "delta": 3000}},
    "Notify"
  ],
  "description": "Transfer 3000 from sender to receiver"
}'
```

### GET /pipeline/:slug — Get Pipeline

```bash
curl /pipeline/bank-transfer
```

### POST /pipeline/:slug/call — Execute Pipeline

```bash
curl -X POST /pipeline/bank-transfer/call
```

### DELETE /pipeline/:slug — Delete Pipeline

```bash
curl -X DELETE /pipeline/bank-transfer
```

### GET /pipelines/named — List All Pipelines

```bash
curl /pipelines/named
```

### Pipeline Step Types

| Step | Description |
|---|---|
| `Scan` | Execute a query, store result in context |
| `Lookup` | Find a record by multi-predicate seek |
| `ValidateGte` | Assert field ≥ value (rollback on fail) |
| `ValidateEq` | Assert field = value (rollback on fail) |
| `UpdateDelta` | Add delta to a field at a looked-up slot |
| `SetField` | Set a field to an absolute value |
| `Insert` | Insert a new record |
| `Retract` | Delete a looked-up record |
| `Notify` | Signal reactive subscribers |

### Trigger Types

| Trigger | Description |
|---|---|
| `on_call` | Execute only when explicitly called via `/pipeline/:slug/call` |
| `on_insert` | Fire automatically when a matching record is inserted |
| `on_schedule` | Fire on a time interval (requires `schedule_interval_secs`) |

---

## Reactive Subscriptions

### POST /subscribe — Single Query SSE

Subscribe to a query. Server pushes updates only when the result changes.

```bash
curl -X POST /subscribe -H 'Content-Type: application/json' -d '{
  "space": {"tenant": 1},
  "measure": {"type": "count"}
}'
```

SSE events:
```
event: result
data: {"event":"initial","fingerprint":"a1b2c3d4e5f6a7b8","result":{"count":42000,"query_us":9}}

event: result
data: {"event":"update","fingerprint":"b2c3d4e5f6a7b8a1","result":{"count":42001,"query_us":8}}
```

### POST /subscribe-multi — Multiplexed SSE

Subscribe to N queries over a single SSE connection. Solves the browser 6-connection-per-host limit.

```bash
curl -X POST /subscribe-multi -d '{
  "queries": {
    "total-count": {"space": {"tenant": 1}, "measure": {"type": "count"}},
    "total-revenue": {"space": {"tenant": 1}, "measure": {"type": "sum"}, "of": "amount"},
    "region-3": {"space": {"tenant": 1, "region": 3}, "measure": {"type": "count"}}
  }
}'
```

Each SSE event includes an `id` field identifying which query changed:
```
data: {"id":"total-count","event":"initial","fingerprint":"...","result":{...}}
data: {"id":"total-revenue","event":"initial","fingerprint":"...","result":{...}}
data: {"id":"total-count","event":"update","fingerprint":"...","result":{...}}
```

---

## Document Store

### POST /doc — Create/Update Document

```bash
curl -X POST /doc -d '{
  "tenant": 1,
  "entity": 0,
  "doc_id": "order-123",
  "data": {"items": ["widget", "gadget"], "total": 4999, "notes": "express shipping"}
}'
```

### GET /doc — Read Document

```bash
curl "/doc?tenant=1&entity=0&doc_id=order-123"
```

### DELETE /doc — Delete Document

```bash
curl -X DELETE "/doc?tenant=1&entity=0&doc_id=order-123"
```

### GET /docs — List Documents

```bash
curl "/docs?tenant=1&entity=0"
```

---

## Plugin State

### POST /plugin/state — Set State

```bash
curl -X POST /plugin/state -d '{
  "tenant": 1,
  "plugin_id": "my-plugin",
  "key": "cursor",
  "value": "2025-02-25T00:00:00Z"
}'
```

### GET /plugin/state — Get State

```bash
curl "/plugin/state?tenant=1&plugin_id=my-plugin&key=cursor"
```

---

## Schema

### POST /plugin/table — Define Table Schema

```bash
curl -X POST /plugin/table -d '{
  "tenant": 1,
  "plugin_id": "my-plugin",
  "table": "keywords",
  "schema": {"id": "TEXT", "keyword": "TEXT", "volume": "INTEGER"}
}'
```

### GET /plugin/tables — List Table Schemas

```bash
curl "/plugin/tables?tenant=1&plugin_id=my-plugin"
```

### POST /schema/column — Add Column

Extend the record schema with a new column (zero-downtime, no migration).

```bash
curl -X POST /schema/column -d '{
  "name": "loyalty_tier",
  "byte_width": 1,
  "description": "Customer loyalty tier (0-255)"
}'
```

### GET /api/schema — Get Schema

```bash
curl /api/schema
```

Returns the full schema as JSON, including core columns, extended columns, and auto-generated endpoint URLs.

### GET /api/v1/:column — Auto-REST Column Query

PostgREST-style auto-generated endpoints for any column.

```bash
# Exact match
curl "/api/v1/region?eq=3&measure=count"

# Range
curl "/api/v1/amount?gt=500&measure=sum&of=amount"

# Between with cross-column filter
curl "/api/v1/amount?between=100,1000&filter=region:eq:3&measure=count"
```

---

## WebSocket

### GET /ws/db — Database WebSocket

Full-duplex binary or JSON protocol for queries, mutations, subscriptions, and pipeline calls over a single persistent connection. This is the highest-performance way to interact with BitBin.

See [WebSocket Protocol](/websocket) for the full specification.

---

## System

### GET /health — Health Check

```bash
curl /health
```

### GET /runtime — Runtime Info

```bash
curl /runtime
```

Returns: core count, total records, runtime mode.

### GET /stats — Global Statistics

```bash
curl /stats
```

### POST /flush — Flush to Storage

Export current data to durable storage.

---

## Client Registry

### POST /clients — Register Client

Register an API client with tenant scoping.

### GET /clients — List Clients

### GET /clients/resolve — Resolve Scope

Resolve tenant scope from API key.

### DELETE /clients/:id — Remove Client

---

## Fleet Operations

### POST /fleet/query — Fleet Query

Execute a query across all cluster nodes (for distributed deployment).

### POST /coordinate — Coordinated Query

Fan out a query to cluster nodes and merge results.

---

[← Previous: Platform API](/platform) · **Chapter 10** · [Next: SQL Migration Guide →](/sql-migration)
