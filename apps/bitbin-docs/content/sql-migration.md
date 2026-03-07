---
title: SQL Migration Guide
order: 11
---

# Chapter 11: SQL Migration Guide

**BitBin does not use SQL.** It uses a **Structured Command Protocol** — JSON-based queries and composable pipelines that replace traditional SQL operations with high-performance equivalents.

This document maps every SQL concept to its BitBin equivalent so you can migrate existing applications quickly.

---

## Naming Conventions

| SQL Term | BitBin Term | Description |
|----------|-------------|-------------|
| Database | **Tenant Partition** | Each tenant gets a dedicated slot range (default 8192, configurable to 1M+) |
| Table | **Entity Type** (0–15) | Up to 16 entity types per tenant |
| Row | **Slot** | Physical record position. Fixed-width per slot. |
| Primary Key | **(tenant, entity, key_id)** | 3 key dimensions that uniquely address a record |
| Column | **Core Dimension** / **Satellite Column** | Core dims are fixed-width; satellites are extensible at runtime |
| Index | **Sort Dimension** | Core dimensions are automatically indexed by the storage layout. No separate B-tree. |
| Transaction | **Batch Mutate** | Atomic multi-op with captured old values and rollback |
| Stored Procedure | **Pipeline** | Ordered list of steps executed atomically |
| Trigger | **Named Pipeline** | Stored pipeline with `on_call` / `on_insert` / `on_schedule` trigger |
| View | **Pipe** (saved query) | Stored query, streamable via SSE |
| Schema | **Schema Manifold** | Describes core + satellite column layout |
| Foreign Key | **Pipeline validation** | `Lookup` + `Validate` in an atomic pipeline. Programmable, not declarative. |

### Entity Types

| Entity | ID | Use |
|--------|----|-----|
| Analytics | 0 | Orders, events, metrics |
| Graph Node | 1 | Graph nodes |
| Graph Edge | 2 | Graph edges (direct) |
| Graph Incidence | 3 | Hyperedge members |
| Vector Text | 4 | Text embeddings |
| Vector Image | 5 | Image embeddings |
| Vector Audio | 6 | Audio embeddings |
| Plugin Registry | 7 | Plugin metadata |
| Plugin State | 8 | Plugin KV state |
| Credentials | 9 | Encrypted credentials |
| User Table A–E | 10–14 | User-defined tables |
| Reserved | 15 | Reserved |

### Record Layout

Each record occupies a fixed number of bytes with up to 8 core dimensions:

| Dim | Name | Width | Use |
|-----|------|-------|-----|
| 0 | `tenant` | 16-bit | Tenant ID (partition key) |
| 1 | `entity` | 4-bit | Entity type (0–15) |
| 2 | `timestamp` / `key_id` | 16-bit | Time or keyed record ID |
| 3 | `amount` | 16-bit | Value / measure column |
| 4 | `region` | 8-bit | Categorical dimension |
| 5 | `currency` | 4-bit | Categorical dimension |
| 6 | `category` | 8-bit | Categorical dimension |
| 7 | `status` | 4-bit | Categorical / flag |

Additional columns can be added at runtime as **satellite columns** without modifying existing data.

---

## CRUD Operations

### CREATE (Insert)

| SQL | BitBin | Notes |
|-----|--------|-------|
| `INSERT INTO t VALUES (...)` | `POST /ingest` or WS mutate | O(1) addressed write. Key dims determine slot; value dims stored. |
| `INSERT ... ON CONFLICT UPDATE` | Same endpoint | Detects existing key, updates value dims in-place (upsert). |
| Bulk insert | `POST /ingest` | Batch of records. Tenant-grouped for throughput. |
| Pipeline insert | Pipeline `Insert` step | Insert as part of an atomic pipeline. |

**API Example:**
```bash
# Bulk ingest
curl -X POST https://YOUR_DB.machx.dev/ingest \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{"count": 1000}'
```

**TypeScript SDK:**
```typescript
await bb.ingest({ count: 1000 });
```

### READ (Query)

| SQL | BitBin | Notes |
|-----|--------|-------|
| `SELECT * WHERE pk = ?` | Point lookup via WS | O(1) point read by (tenant, entity, key_id) |
| `SELECT ... WHERE col BETWEEN a AND b` | `POST /query` | Dimensional bounds filter |
| `SELECT SUM/COUNT/MIN/MAX` | `POST /query` with measures | Aggregation on matching records |
| `SELECT ... WHERE a AND b` | `Query.join` | Intersection of multiple bounding boxes |
| `SELECT ... WHERE a OR b` | `Query.union` | Union of multiple bounding boxes |
| `SELECT ... GROUP BY col` | `POST /query` with group measure | Per-group aggregates |

**API Example:**
```bash
curl -X POST https://YOUR_DB.machx.dev/query \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": {
      "dimensions": {
        "tenant": 1,
        "entity": 0,
        "timestamp": [100, 500]
      }
    },
    "measure": {"type": "sum"},
    "of": "amount"
  }'
```

**TypeScript SDK:**
```typescript
const result = await bb.query(
  new QueryBuilder()
    .where("tenant", 1)
    .where("entity", 0)
    .between("timestamp", 100, 500)
    .sum("amount")
    .build()
);
console.log(result.sum, result.count);
```

### UPDATE

| SQL | BitBin | Notes |
|-----|--------|-------|
| `UPDATE t SET col = val WHERE pk = ?` | `SetField` pipeline step | Absolute field write at a named slot |
| `UPDATE t SET col = col + delta` | `UpdateDelta` pipeline step | Additive delta at a named slot |
| Upsert (insert or update) | `POST /ingest` | Same call handles both — key exists → updates value dims |
| Batch update | `batch_mutate` with `UpdateField` ops | Atomic multi-slot update with rollback |

**API Example (atomic transfer via named pipeline):**
```bash
# Execute a stored pipeline
curl -X POST https://YOUR_DB.machx.dev/pipeline/bank-transfer/call \
  -H 'Authorization: Bearer YOUR_API_KEY'
```

A pipeline can atomically: look up records → validate conditions → mutate multiple records → roll back on failure.

### DELETE

| SQL | BitBin | Notes |
|-----|--------|-------|
| `DELETE FROM t WHERE pk = ?` | `DELETE /record` | Clears record. Slot recycled on next insert. |
| `DELETE` in pipeline | `Retract` pipeline step | Retract a looked-up record atomically |
| Batch delete | `batch_mutate` with `Retract` ops | Atomic multi-delete with rollback |

**API Example:**
```bash
curl -X DELETE "https://YOUR_DB.machx.dev/record?slot=42" \
  -H 'Authorization: Bearer YOUR_API_KEY'
```

---

## SQL Feature Parity

| SQL Operation | BitBin Equivalent | Status |
|---|---|---|
| `INSERT INTO` | `POST /ingest` or WS mutate | ✅ |
| `INSERT … ON CONFLICT UPDATE` | Same endpoint (upsert) | ✅ |
| `SELECT * WHERE pk = ?` | Point lookup — O(1) | ✅ |
| `SELECT … WHERE col BETWEEN` | `POST /query` (Query) | ✅ |
| `SELECT SUM/COUNT/MIN/MAX` | `POST /query` with measures | ✅ |
| `UPDATE SET col = ?` | `SetField` pipeline step | ✅ |
| `UPDATE SET col = col + ?` | `UpdateDelta` pipeline step | ✅ |
| `DELETE WHERE pk = ?` | `Retract` / `DELETE /record` | ✅ |
| `BEGIN … COMMIT / ROLLBACK` | `batch_mutate` | ✅ |
| `JOIN … AND` | `Query.join` (intersection) | ✅ |
| `UNION` | `Query.union` (union) | ✅ |
| `GROUP BY` (single column) | Group measure in query | ✅ |
| `CREATE TABLE` | `POST /plugin/table` (schema manifest) | ✅ |
| `ALTER TABLE ADD COLUMN` | `POST /schema/column` (satellite) | ✅ |
| `CREATE TRIGGER` (after insert) | Named Pipeline + `on_insert` trigger | ✅ |
| Stored procedures | Pipeline with steps | ✅ |
| Scheduled events | Named Pipeline + `on_schedule` trigger | ✅ |
| SSE subscriptions | `GET /pipe/:slug/stream` | ✅ |
| WebSocket real-time | `GET /ws/db` | ✅ |
| `ORDER BY col LIMIT K` | Top-K query | ✅ |
| `GROUP BY` (multi-column) | Composable via pipeline | ✅ |
| Foreign key constraints | Pipeline validation (Lookup + Validate) | ✅ Stronger |
| Full-text search | Deferred | ❌ |
| SQL syntax parser | Intentionally skipped | ❌ By design |

---

## How ORDER BY and LIMIT Work

BitBin doesn't use B-tree traversal for ordering. Instead:

1. **Filter** — The query engine scans matching records using the dimensional bounds (sub-millisecond for 100K+ records)
2. **Decode** — Sort column values are read from matching records
3. **Top-K** — An insertion-sort buffer keeps the top K results

This is the same pattern used by the vector search pipeline (which orders by similarity distance). For any column, the filter step dominates — the sort is trivially cheap because it only touches matching records.

---

## Foreign Keys: Why Pipelines Are Stronger

SQL foreign keys enforce "this value must exist in another table." In distributed systems, this is universally handled at the application level:

| System | FK Support | Integrity Approach |
|--------|-----------|-------------------|
| PostgreSQL | ✅ | Declarative FK constraints |
| DynamoDB | ❌ | Application-level |
| Cassandra | ❌ | Application-level |
| MongoDB | ❌ | Application-level |
| Redis | ❌ | Application-level |
| **BitBin** | ✅ Programmable | **Pipeline validation (Lookup + Validate + Rollback)** |

BitBin's pipeline validation is **stronger** than SQL FKs:

```
SQL FK:     "Does this key exist in the other table?"        → boolean check
Pipeline:   "Does this key exist AND is the balance ≥ 3000?" → programmable validation
```

A pipeline atomically looks up records across entity types, validates arbitrary conditions, mutates multiple records, and rolls back ALL changes if any validation fails.

---

## API Endpoint Reference

### Data Operations

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/ingest` | POST | Bulk record ingest |
| `/record` | DELETE | Delete a record by slot |
| `/query` | POST | Query with bounds + measures |
| `/ask` | POST | Natural language → query |

### Named Pipelines (Stored Procedures + Triggers)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/pipeline` | POST | Create/update a named pipeline |
| `/pipeline/:slug` | GET | Read pipeline definition |
| `/pipeline/:slug` | DELETE | Remove a pipeline |
| `/pipeline/:slug/call` | POST | Execute an on_call pipeline |
| `/pipelines/named` | GET | List all named pipelines |

### Saved Queries (Pipes)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/pipe` | POST | Create/update a saved query |
| `/pipe/:slug` | GET | Execute a saved query |
| `/pipe/:slug` | DELETE | Remove a saved query |
| `/pipe/:slug/stream` | GET | SSE stream (auto-refreshes on data change) |
| `/pipes` | GET | List all saved queries |

### Plugin State & Schema

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/plugin/state` | POST | Set plugin KV state |
| `/plugin/state` | GET | Get plugin KV state |
| `/plugin/table` | POST | Define table schema |
| `/plugin/tables` | GET | List table schemas for a plugin |

### Tenant Analytics

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/tenant/:id/analytics` | GET | Tenant SUM/COUNT |
| `/tenant/:id/window` | GET | Time-windowed aggregation |
| `/tenant/:id/scan_gt` | GET | Threshold scan |
| `/stats` | GET | Global statistics |

### Schema & Runtime

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/schema/column` | POST | Add satellite column |
| `/api/schema` | GET | Full schema manifest |
| `/health` | GET | Health check |
| `/runtime` | GET | Runtime info (cores, slots, uptime) |
| `/flush` | POST | Flush to persistent storage |

### WebSocket

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/ws/db` | GET | Real-time WebSocket (binary + JSON) |

---

## Configuration

### Partition Sizing

| Partition Size | Slots/Tenant | Memory/Tenant | Use Case |
|---------------|-------------|---------------|----------|
| Small (default) | 8,192 | 128 KB | Analytics, small plugins |
| Medium | 65,536 | 1 MB | Medium plugins, graph |
| Large | 1,048,576 | 16 MB | Large vector stores |

Only written pages consume physical memory — empty partitions have near-zero overhead.

---

[← Previous: API Reference](/api-reference) · **Chapter 11** · [Next: Plugin Integration →](/plugin-integration)
