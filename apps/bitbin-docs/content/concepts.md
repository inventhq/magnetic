---
title: Core Concepts
order: 2
---

# Chapter 2: Core Concepts

**The mental model you need before writing queries or ingesting data.**

---

## Tenants

Every BitBin database is divided into **tenant partitions**. A tenant is an isolated slice of the database — think of it as a separate schema in PostgreSQL or a separate database in MongoDB.

- Each tenant gets a dedicated slot range (default 8,192 records, configurable up to 1M+)
- Tenant IDs are `u16` values (0–65,535)
- All queries are scoped to a tenant — cross-tenant queries are not possible by design
- When you provision a database, you're allocated a range of tenant IDs (e.g., tenants 0–169)

**When to use multiple tenants:**
- Multi-tenant SaaS: one tenant per customer
- Data isolation: one tenant per environment (dev, staging, prod)
- Sharding: split large datasets across tenants for parallel queries

---

## Entities

Within each tenant, records are categorized by **entity type** — an integer from 0 to 15. Think of entities as tables.

| Entity | ID | Purpose |
|---|---|---|
| Analytics | 0 | Orders, events, metrics, time-series |
| Graph Node | 1 | Graph vertices |
| Graph Edge | 2 | Direct relationships (≤6 members) |
| Graph Incidence | 3 | Relationship overflow (>6 members) |
| Vector Text | 4 | Text embeddings |
| Vector Image | 5 | Image embeddings |
| Vector Audio | 6 | Audio embeddings |
| Plugin Registry | 7 | Plugin metadata |
| Plugin State | 8 | Plugin key-value state |
| Credentials | 9 | Encrypted credentials |
| User Table A–E | 10–14 | Your custom tables |
| Reserved | 15 | System reserved |

Entity 0 (Analytics) is the default for most applications. Entities 1–6 are used by the graph and vector subsystems. Entities 10–14 are free for your own record types.

---

## Records & Dimensions

Every record is a fixed-width structure with **8 core dimensions**:

| Dimension | Offset | Width | Range | Description |
|---|---|---|---|---|
| `tenant` | 0–1 | u16 | 0–65,535 | Partition key |
| `entity` | 2–3 | u8 | 0–15 | Record type |
| `key_id` | 4–5 | u16 | 0–65,535 | Record ID, timestamp, or foreign key |
| `amount` | 6–7 | u16 | 0–65,535 | Numeric value (revenue, count, score) |
| `region` | 8 | u8 | 0–255 | Categorical (region, type, tier) |
| `currency` | 9 | u8 | 0–15 | Categorical (currency, source, flag) |
| `category` | 10 | u8 | 0–255 | Categorical (category, segment, tag) |
| `status` | 11 | u8 | 0–15 | Categorical (status, state, active) |

**Key points:**
- Every dimension is automatically indexed — there are no secondary indexes to create
- Queries filter on any combination of dimensions with zero overhead for unconstrained dimensions
- The first three dimensions (`tenant`, `entity`, `key_id`) form the **composite primary key**
- `amount` is the primary numeric/measure column used for `sum`, `min`, `max` aggregations

### Naming Your Dimensions

The dimension names (`region`, `currency`, `category`, `status`) are defaults. You can map them to anything meaningful for your domain:

| Default Name | E-commerce | IoT | SaaS Metrics |
|---|---|---|---|
| `tenant` | customer_id | device_group | workspace_id |
| `entity` | record_type | sensor_type | metric_type |
| `key_id` | order_id | reading_id | event_id |
| `amount` | order_total | sensor_value | metric_value |
| `region` | shipping_zone | location_id | plan_tier |
| `currency` | currency_code | protocol | feature_id |
| `category` | product_category | alert_level | module |
| `status` | order_status | device_status | event_status |

The query DSL uses the default names. Your application maps between your domain names and BitBin's dimension names.

---

## Extended Columns (Satellites)

Beyond the 8 core dimensions, you can add **satellite columns** at runtime without downtime:

```bash
curl -X POST https://{subdomain}.machx.dev/schema/columns \
  -H 'Content-Type: application/json' \
  -d '{"name": "priority", "byte_width": 1, "num_bits": 8}'
```

Satellite columns:
- Are added at runtime — no migrations, no downtime
- Support the same query filters as core dimensions
- Are stored in a separate memory region and indexed automatically
- Have configurable width (1–8 bytes) and bit precision

---

## Primary Key & Addressing

Every record is uniquely addressed by the composite key `(tenant, entity, key_id)`:

```
┌──────────┬──────────┬──────────┐
│ tenant   │ entity   │ key_id   │  → unique slot in the database
│ (u16)    │ (u8)     │ (u16)    │
└──────────┴──────────┴──────────┘
```

- **`keyed_insert(tenant, entity, key_id, ...)`** — upsert: if the key exists, update in place
- **`keyed_lookup(tenant, entity, key_id)`** — O(1) point read
- **`keyed_delete(tenant, entity, key_id)`** — O(1) delete

This means `key_id` must be unique within a `(tenant, entity)` pair. If you insert with the same `(tenant, entity, key_id)`, the existing record is overwritten.

---

## Queries as Geometry

Every query defines a **bounding box** in the dimension space. You specify constraints on any subset of dimensions, and BitBin finds all records that fall within that box.

```json
{
  "space": {"tenant": 1, "amount": [500, null], "region": 3},
  "measure": {"type": "count"}
}
```

This is equivalent to: "Count all records where tenant=1 AND amount>500 AND region=3."

- **Exact match:** `"region": 3` → region = 3
- **Range:** `"amount": [500, null]` → amount > 500
- **Unconstrained:** omit the dimension entirely — zero cost

The result includes a `query_us` field showing execution time in microseconds. Typical queries complete in 1–50µs.

---

## Pipes & Subscriptions

A **pipe** is a saved query. Once created, it can be executed by name or streamed over SSE:

```
POST /pipe → create
GET /pipe/{slug} → execute
GET /pipe/{slug}/stream → SSE stream (pushes on data change)
```

A **subscription** is a live query that pushes updates to your client whenever the result changes. BitBin uses fingerprint-based change detection — it only pushes when the actual result differs, not on every mutation.

---

## Pipelines

A **pipeline** is an atomic sequence of steps — the equivalent of a stored procedure or transaction. Steps can include lookups, validations, field updates, inserts, and deletes. If any step fails, all mutations roll back.

Pipelines support three trigger types:
- **`on_call`** — execute manually via `POST /pipeline/{slug}/call`
- **`on_insert`** — fire automatically when matching records are inserted
- **`on_schedule`** — fire on a time interval

See [Pipelines](/pipelines) for the full step reference.

---

## Glossary

| Term | Definition |
|---|---|
| **Tenant** | An isolated partition within a database (u16 ID) |
| **Entity** | Record type within a tenant (0–15) |
| **Dimension** | A field in the record schema (8 core + satellites) |
| **Slot** | A physical record position in the database |
| **Space** | The set of filter conditions in a query (bounding box) |
| **Measure** | The aggregation to compute: count, sum, min, max, group, top_k, extract |
| **Pipe** | A saved query, executable by name, streamable via SSE |
| **Pipeline** | An atomic multi-step operation with rollback |
| **Satellite** | An extended column added at runtime |

---

[← Previous: Getting Started](/getting-started) · **Chapter 2** · [Next: Query Language →](/query-language)
