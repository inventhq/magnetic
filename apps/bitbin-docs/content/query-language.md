---
title: Query Language
order: 3
---

# Chapter 3: Query Language

**The query DSL for BitBin: filters + measures.**

Every query is a JSON object with two required fields: `space` (where to look) and `measure` (what to compute).

---

## Vocabulary

| SQL Term | BitBin Term | Description |
|---|---|---|
| Database | **Tenant Partition** | Each tenant gets a dedicated partition |
| Table | **Entity Type** (0–15) | Record type. 16 types per tenant. |
| Row | **Slot** | A single record position. 16 bytes per slot. |
| Primary Key | **(tenant, entity, key_id)** | Composite key. |
| Column | **Dimension** (core) / **Extended Column** | 8 core fields + extensible columns. |
| WHERE clause | **Space** | Filter conditions for the query. |
| SELECT aggregate | **Measure** | count, sum, min, max, group, top_k, extract, group_by_multi |
| JOIN | **Join** | Two filter sets AND'd together. |
| UNION | **Union** | Two filter sets OR'd together. |
| ORDER BY LIMIT K | **TopK** | Sorted results limited to K records. |
| GROUP BY | **Group / GroupByMulti** | Aggregate by one or many columns. |
| Transaction | **batch_mutate** | Atomic multi-op with rollback. |
| Stored Procedure | **Pipeline** | Ordered list of steps executed atomically. |
| Trigger | **Named Pipeline** | Stored pipeline with on_call / on_insert / on_schedule trigger. |
| View | **Pipe** | Saved query, streamable via SSE. |
| Schema | **Schema** | Describes core + extended column layout. |

### Record Types (Entity)

| Entity | ID | Purpose |
|---|---|---|
| `ENTITY_ANALYTICS` | 0 | Orders, events, metrics |
| `ENTITY_GRAPH_NODE` | 1 | Graph vertices |
| `ENTITY_GRAPH_EDGE` | 2 | Graph edges (direct, ≤6 members) |
| `ENTITY_GRAPH_INCIDENCE` | 3 | Edge overflow (>6 members) |
| `ENTITY_VECTOR_TEXT` | 4 | Text embeddings |
| `ENTITY_VECTOR_IMAGE` | 5 | Image embeddings |
| `ENTITY_VECTOR_AUDIO` | 6 | Audio embeddings |
| `ENTITY_PLUGIN_REGISTRY` | 7 | Plugin metadata |
| `ENTITY_PLUGIN_STATE` | 8 | Plugin KV state |
| `ENTITY_CREDENTIALS` | 9 | Encrypted credentials |
| `ENTITY_PLUGIN_TABLE_A–E` | 10–14 | User-defined tables |
| `ENTITY_RESERVED` | 15 | Reserved |

### Core Fields (16-byte Record)

| Field | Offset | Width | Type | Description |
|-------|--------|-------|------|-------------|
| `tenant` | 0–1 | u16 | Integer | Tenant/partition key |
| `entity` | 2–3 | u8 | Integer | Record type (0–15) |
| `key_id` | 4–5 | u16 | Integer | Record ID or timestamp |
| `amount` | 6–7 | u16 | Integer | Value / measure column |
| `region` | 8 | u8 | Integer | Categorical field |
| `currency` | 9 | u8 | Integer | Categorical field |
| `category` | 10 | u8 | Integer | Categorical field |
| `status` | 11 | u8 | Integer | Categorical / flag field |

---

## Query Structure

```json
{
  "space": { ... },           // Filter conditions — WHERE clause (required)
  "measure": { "type": "..." }, // Aggregation type (required)
  "of": "column_name",        // Target column for sum/min/max/top_k (optional)
  "columns": ["col1", "col2"], // Columns to extract for top_k/extract (optional)
  "union": [{ ... }],         // Additional filters OR'd (optional)
  "join": [{ ... }],          // Additional filters AND'd (optional)
  "limit": 100,               // Result limit (optional)
  "group_by": ["col1", "col2"], // GROUP BY columns for group_by_multi (optional)
  "sort_order": "desc"        // "asc" or "desc" for top_k (optional, default: "desc")
}
```

---

## Filters (`space`)

The `space` object defines which records to match. Each key is a column name, each value is a bound.

### Exact Match (EQ)

```json
{"space": {"tenant": 1, "region": 3}}
```
SQL equivalent: `WHERE tenant = 1 AND region = 3`

### Range Bound

```json
{"space": {"amount": [500, null]}}
```
SQL equivalent: `WHERE amount > 500`

```json
{"space": {"amount": [null, 1000]}}
```
SQL equivalent: `WHERE amount < 1000`

```json
{"space": {"amount": [500, 1000]}}
```
SQL equivalent: `WHERE amount BETWEEN 500 AND 1000`

### Combined

```json
{"space": {"tenant": 1, "amount": [500, null], "region": 3}}
```
SQL equivalent: `WHERE tenant = 1 AND amount > 500 AND region = 3`

### Unconstrained

Omitted fields are unconstrained — they have zero cost.

```json
{"space": {}}
```
SQL equivalent: `SELECT ... FROM *` (scans all occupied slots)

---

## Measures

### Count

```json
{"measure": {"type": "count"}}
```
Returns: `{"count": 42000, "query_us": 9}`

### Sum

```json
{"measure": {"type": "sum"}, "of": "amount"}
```
Returns: `{"count": 42000, "sum": 18500000, "query_us": 12}`

### Min / Max

```json
{"measure": {"type": "min"}, "of": "amount"}
{"measure": {"type": "max"}, "of": "amount"}
```
Returns: `{"count": 42000, "min": 1, "query_us": 8}`

### Group (single column)

```json
{"measure": {"type": "group"}}
```
Returns:
```json
{
  "count": 42000,
  "groups": [
    {"tenant_id": 1, "count": 12000, "sum": 5400000},
    {"tenant_id": 2, "count": 8000, "sum": 3200000}
  ]
}
```

### GroupByMulti (multi-column)

```json
{
  "measure": {"type": "group_by_multi"},
  "of": "amount",
  "group_by": ["tenant", "region"]
}
```
Returns:
```json
{
  "count": 42000,
  "multi_groups": [
    {"keys": {"tenant": 2, "region": 1}, "count": 5000, "sum": 2100000},
    {"keys": {"tenant": 1, "region": 3}, "count": 3000, "sum": 1500000}
  ]
}
```

Groups sorted by sum descending.

### TopK

```json
{
  "measure": {"type": "top_k", "value": 10},
  "of": "amount",
  "columns": ["amount", "region", "timestamp"],
  "sort_order": "desc"
}
```
Returns:
```json
{
  "count": 42000,
  "rows": [
    {"slot": 8421, "fields": [["amount", 9999], ["region", 3], ["timestamp", 500]]},
    {"slot": 3102, "fields": [["amount", 8000], ["region", 1], ["timestamp", 200]]}
  ]
}
```

`sort_order`: `"desc"` (default, highest first) or `"asc"` (lowest first).

### Extract

```json
{
  "measure": {"type": "extract"},
  "columns": ["amount", "region", "status"],
  "limit": 100
}
```
Returns:
```json
{
  "count": 42000,
  "rows": [
    {"slot": 100, "fields": [["amount", 500], ["region", 3], ["status", 1]]},
    {"slot": 101, "fields": [["amount", 800], ["region", 5], ["status", 0]]}
  ]
}
```

Always use `limit` to avoid unbounded results.

---

## Joins (AND)

```json
{
  "space": {"amount": [500, null]},
  "join": [{"region": 3}],
  "measure": {"type": "sum"},
  "of": "amount"
}
```

SQL equivalent: `SELECT SUM(amount) FROM orders WHERE amount > 500 AND region = 3`

Each join filter produces a result set. All sets are intersected with the primary `space` filter.

---

## Unions (OR)

```json
{
  "space": {"region": 1},
  "union": [{"region": 3}, {"region": 5}],
  "measure": {"type": "count"}
}
```

SQL equivalent: `SELECT COUNT(*) WHERE region IN (1, 3, 5)`

Each union filter produces a result set, all sets are combined with the primary filter.

---

## Natural Language Queries

```bash
curl -X POST /ask -d '{"text": "show me orders over $500 from region 3 grouped by tenant"}'
```

The NLQ compiler tokenizes the text, resolves aliases, and compiles to a structured query:

```json
{
  "space": {"amount": [500, null], "region": 3},
  "measure": {"type": "group"}
}
```

### Alias Resolution

| Input | Resolves To |
|---|---|
| store, merchant, account, shop, seller | `tenant` |
| order_type, type | `entity` |
| time, date, day, period | `timestamp` |
| revenue, total, price, value, balance | `amount` |
| area, zone, location | `region` |
| curr, money | `currency` |
| cat, product_type, segment | `category` |
| state, flag, active | `status` |

### Measure Resolution

| Input | Resolves To |
|---|---|
| count, how many, number of, total number | `Count` |
| sum, total, revenue, add up | `Sum` |
| minimum, lowest, smallest, cheapest | `Min` |
| maximum, highest, largest, biggest, most expensive | `Max` |
| group, by, per, breakdown, segment, distribute | `Group` |
| extract, show, list, display, rows, records, details | `Extract` |
| top, best, worst, ranking, leaderboard | `TopK` |

---

## Query Result Structure

Every query returns a result:

```json
{
  "count": 42000,           // Always present: number of matching records
  "sum": 18500000,          // Present for sum measure
  "min": 1,                 // Present for min measure
  "max": 9999,              // Present for max measure
  "groups": [...],          // Present for group measure
  "multi_groups": [...],    // Present for group_by_multi measure
  "rows": [...],            // Present for top_k / extract measures
  "query_us": 9             // Execution time in microseconds
}
```

Fields that don't apply to the chosen measure are omitted from the JSON response.

---

[← Previous: Core Concepts](./concepts.md) · **Chapter 3** · [Next: Data Operations →](./data-operations.md)
