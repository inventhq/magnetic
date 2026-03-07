---
title: Data Operations
order: 4
---

# Chapter 4: Data Operations

**How to write, update, and delete data in BitBin.**

---

## Ingest (Bulk Insert)

Generate sample or seed data:

```bash
curl -X POST https://{subdomain}.machx.dev/ingest \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{"count": 1000}'
```

This inserts `count` records with randomized dimension values. Useful for testing and development.

---

## Keyed Insert (Upsert)

Insert or update a single record by its composite key `(tenant, entity, key_id)`:

```bash
curl -X POST https://{subdomain}.machx.dev/mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "op": "keyed_insert",
    "tenant": 1,
    "entity": 0,
    "key_id": 42,
    "amount": 2500,
    "region": 3,
    "currency": 1,
    "category": 7,
    "status": 1
  }'
```

**Upsert behavior:** If a record with the same `(tenant, entity, key_id)` already exists, it is updated in place. No duplicate is created.

---

## Keyed Lookup (Point Read)

Read a single record by its composite key:

```bash
curl -X POST https://{subdomain}.machx.dev/mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "op": "keyed_lookup",
    "tenant": 1,
    "entity": 0,
    "key_id": 42
  }'
```

Response:
```json
{
  "found": true,
  "slot": 8421,
  "fields": {
    "tenant": 1, "entity": 0, "key_id": 42,
    "amount": 2500, "region": 3, "currency": 1,
    "category": 7, "status": 1
  }
}
```

Lookup is O(1) — it probes directly by the composite key.

---

## Keyed Delete

Delete a record by its composite key:

```bash
curl -X POST https://{subdomain}.machx.dev/mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "op": "keyed_delete",
    "tenant": 1,
    "entity": 0,
    "key_id": 42
  }'
```

---

## Batch Mutate (Atomic Multi-Op)

Execute multiple operations atomically. If any operation fails, all are rolled back:

```bash
curl -X POST https://{subdomain}.machx.dev/batch_mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "ops": [
      {"op": "keyed_insert", "tenant": 1, "entity": 0, "key_id": 100, "amount": 500, "region": 1, "currency": 0, "category": 3, "status": 1},
      {"op": "keyed_insert", "tenant": 1, "entity": 0, "key_id": 101, "amount": 750, "region": 2, "currency": 0, "category": 5, "status": 1},
      {"op": "keyed_insert", "tenant": 1, "entity": 0, "key_id": 102, "amount": 1200, "region": 1, "currency": 1, "category": 3, "status": 0}
    ]
  }'
```

Response:
```json
{"applied": 3, "rolled_back": 0}
```

Use batch mutate when you need all-or-nothing semantics for a set of writes. For more complex logic (lookups + validations + writes), use [Pipelines](./pipelines.md).

---

## WebSocket Mutations

For high-throughput writes, use the WebSocket protocol at `GET /ws/db`. See [WebSocket Protocol](./websocket.md) for the full specification.

### JSON Mode

```json
{"op": "mutate", "id": 1, "mutation": {
  "op": "keyed_insert", "tenant": 1, "entity": 0, "key_id": 42,
  "amount": 2500, "region": 3, "currency": 1, "category": 7, "status": 1
}}
```

### Binary Mode

Send a MUTATE frame (opcode `0x04`) with the record payload. Binary mode eliminates JSON parsing overhead and is the highest-performance write path.

---

## Schema Management

### View Current Schema

```bash
curl https://{subdomain}.machx.dev/schema
```

Response:
```json
{
  "record_size": 16,
  "core_dimensions": [
    {"name": "tenant", "offset": 0, "width": 2, "bits": 16},
    {"name": "entity", "offset": 2, "width": 2, "bits": 4},
    {"name": "key_id", "offset": 4, "width": 2, "bits": 16},
    {"name": "amount", "offset": 6, "width": 2, "bits": 16},
    {"name": "region", "offset": 8, "width": 1, "bits": 8},
    {"name": "currency", "offset": 9, "width": 1, "bits": 4},
    {"name": "category", "offset": 10, "width": 1, "bits": 8},
    {"name": "status", "offset": 11, "width": 1, "bits": 4}
  ],
  "satellite_columns": []
}
```

### Add a Satellite Column

Extend the schema at runtime — no downtime, no migration:

```bash
curl -X POST https://{subdomain}.machx.dev/schema/columns \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{"name": "priority", "byte_width": 1, "num_bits": 8}'
```

After adding, the column is immediately queryable:

```json
{"space": {"priority": 5}, "measure": {"type": "count"}}
```

### Column Width Guidelines

| Use Case | `byte_width` | `num_bits` | Range |
|---|---|---|---|
| Boolean / flag | 1 | 1 | 0–1 |
| Small enum (≤16 values) | 1 | 4 | 0–15 |
| Byte enum (≤256 values) | 1 | 8 | 0–255 |
| Large integer | 2 | 16 | 0–65,535 |
| Very large integer | 4 | 32 | 0–4.2B |

---

## Write Paths

BitBin supports three write paths depending on your latency and throughput requirements:

| Path | Transport | Latency | Best For |
|---|---|---|---|
| **REST** | `POST /mutate` | ~5ms | Low-volume CRUD, admin operations |
| **WebSocket** | `wss://.../ws/db` | ~1ms | SDKs, dashboards, real-time apps |
| **Edge DO** | CF Worker → DO | ~5ms (edge ack) | IoT, mobile, batch import |

All paths provide the same durability guarantees. Data is persisted via memory-mapped files and flushed to durable storage (R2) on a configurable interval (default 30 seconds).

---

## Best Practices

- **Use `keyed_insert` for upserts** — it handles insert-or-update in one call
- **Use `batch_mutate` for multi-record atomicity** — all ops succeed or all roll back
- **Use WebSocket for high-throughput** — binary mode eliminates JSON overhead
- **Add satellite columns instead of encoding data** — they're indexed automatically and queryable immediately
- **Keep `key_id` unique per `(tenant, entity)`** — duplicate keys overwrite, they don't append

---

[← Previous: Query Language](./query-language.md) · **Chapter 4** · [Next: Real-Time Subscriptions →](./realtime.md)
