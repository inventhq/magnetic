---
title: Examples & Recipes
order: 8
---

# Chapter 8: Examples & Recipes

**Common patterns and complete working examples for BitBin.**

Each example includes the full curl commands you can copy-paste. Replace `{subdomain}` and `YOUR_API_KEY` with your actual values from [Getting Started](/getting-started).

---

## E-Commerce: Order Analytics

Track orders with revenue, region, and status filtering.

### Schema Mapping

| Dimension | Mapped To |
|---|---|
| `tenant` | store_id |
| `entity` | 0 (Analytics) |
| `key_id` | order_id |
| `amount` | order_total (cents) |
| `region` | shipping_zone (1=US, 2=EU, 3=APAC) |
| `currency` | currency (1=USD, 2=EUR, 3=GBP) |
| `category` | product_category |
| `status` | 0=pending, 1=shipped, 2=delivered, 3=returned |

### Insert Orders

```bash
# Order #1: $45.00 shipped to US
curl -X POST https://{subdomain}.machx.dev/mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{"op":"keyed_insert","tenant":1,"entity":0,"key_id":1,"amount":4500,"region":1,"currency":1,"category":3,"status":2}'

# Order #2: €120.00 pending in EU
curl -X POST https://{subdomain}.machx.dev/mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{"op":"keyed_insert","tenant":1,"entity":0,"key_id":2,"amount":12000,"region":2,"currency":2,"category":5,"status":0}'

# Order #3: $89.99 delivered to APAC
curl -X POST https://{subdomain}.machx.dev/mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{"op":"keyed_insert","tenant":1,"entity":0,"key_id":3,"amount":8999,"region":3,"currency":1,"category":3,"status":2}'
```

### Query: Total Revenue

```bash
curl -X POST https://{subdomain}.machx.dev/query \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{"space":{"tenant":1},"measure":{"type":"sum"},"of":"amount"}'
```

```json
{"count": 3, "sum": 25499, "query_us": 5}
```

### Query: Revenue by Region

```bash
curl -X POST https://{subdomain}.machx.dev/query \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": {"tenant": 1},
    "measure": {"type": "group_by_multi"},
    "of": "amount",
    "group_by": ["region"]
  }'
```

```json
{
  "count": 3,
  "multi_groups": [
    {"keys": {"region": 2}, "count": 1, "sum": 12000},
    {"keys": {"region": 3}, "count": 1, "sum": 8999},
    {"keys": {"region": 1}, "count": 1, "sum": 4500}
  ]
}
```

### Query: Delivered Orders Over $50

```bash
curl -X POST https://{subdomain}.machx.dev/query \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": {"tenant": 1, "amount": [5000, null], "status": 2},
    "measure": {"type": "extract"},
    "columns": ["key_id", "amount", "region"],
    "limit": 100
  }'
```

```json
{
  "count": 1,
  "rows": [
    {"slot": 3102, "fields": [["key_id", 3], ["amount", 8999], ["region", 3]]}
  ]
}
```

### Save as a Pipe + Stream

```bash
# Save a live revenue dashboard query
curl -X POST https://{subdomain}.machx.dev/pipe \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "slug": "revenue-by-region",
    "query": {"space":{"tenant":1},"measure":{"type":"group_by_multi"},"of":"amount","group_by":["region"]},
    "description": "Revenue breakdown by shipping zone"
  }'

# Execute anytime
curl https://{subdomain}.machx.dev/pipe/revenue-by-region

# Stream live (SSE — pushes when data changes)
curl https://{subdomain}.machx.dev/pipe/revenue-by-region/stream
```

---

## SaaS Dashboard: Real-Time Metrics

Build a live dashboard with multiplexed subscriptions — one SSE connection for all widgets.

### Insert Metrics

```bash
# Active users metric
curl -X POST https://{subdomain}.machx.dev/mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{"op":"keyed_insert","tenant":1,"entity":0,"key_id":1000,"amount":342,"region":1,"currency":0,"category":1,"status":1}'

# API calls metric
curl -X POST https://{subdomain}.machx.dev/mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{"op":"keyed_insert","tenant":1,"entity":0,"key_id":1001,"amount":48500,"region":1,"currency":0,"category":2,"status":1}'
```

### Multiplexed Dashboard (One SSE Connection)

```bash
curl -X POST https://{subdomain}.machx.dev/subscribe-multi \
  -H 'Content-Type: application/json' \
  -d '{
    "queries": {
      "total-users": {
        "space": {"tenant": 1, "category": 1},
        "measure": {"type": "sum"},
        "of": "amount"
      },
      "total-api-calls": {
        "space": {"tenant": 1, "category": 2},
        "measure": {"type": "sum"},
        "of": "amount"
      },
      "active-count": {
        "space": {"tenant": 1, "status": 1},
        "measure": {"type": "count"}
      }
    }
  }'
```

The server pushes one event per query that changes:

```
data: {"id":"total-users","event":"initial","result":{"count":1,"sum":342,"query_us":4}}
data: {"id":"total-api-calls","event":"initial","result":{"count":1,"sum":48500,"query_us":3}}
data: {"id":"active-count","event":"initial","result":{"count":2,"query_us":2}}
```

### TypeScript Dashboard Integration

```typescript
import { BitBin } from "@bitbin/client";

const bb = new BitBin({
  baseUrl: "https://{subdomain}.machx.dev",
  apiKey: "YOUR_API_KEY"
});

const dashboard = bb.subscribeMulti(
  {
    "total-users": { space: { tenant: 1, category: 1 }, measure: { type: "sum" }, of: "amount" },
    "total-api-calls": { space: { tenant: 1, category: 2 }, measure: { type: "sum" }, of: "amount" },
    "active-count": { space: { tenant: 1, status: 1 }, measure: { type: "count" } },
  },
  (event) => {
    // Update the appropriate widget
    document.getElementById(event.id).textContent = event.result.sum ?? event.result.count;
  }
);
```

---

## IoT: Sensor Ingestion + Alerting

High-volume sensor readings with threshold-based alerting via pipelines.

### Schema Mapping

| Dimension | Mapped To |
|---|---|
| `tenant` | device_group |
| `entity` | 0 (sensor readings) |
| `key_id` | reading_id (incrementing) |
| `amount` | sensor_value (e.g., temperature × 100) |
| `region` | location_id |
| `currency` | protocol (1=MQTT, 2=HTTP, 3=WS) |
| `category` | alert_level (0=normal, 1=warning, 2=critical) |
| `status` | 0=raw, 1=processed |

### Batch Insert Readings

```bash
curl -X POST https://{subdomain}.machx.dev/batch_mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "ops": [
      {"op":"keyed_insert","tenant":10,"entity":0,"key_id":5001,"amount":2350,"region":1,"currency":1,"category":0,"status":0},
      {"op":"keyed_insert","tenant":10,"entity":0,"key_id":5002,"amount":2410,"region":1,"currency":1,"category":0,"status":0},
      {"op":"keyed_insert","tenant":10,"entity":0,"key_id":5003,"amount":3100,"region":2,"currency":2,"category":1,"status":0},
      {"op":"keyed_insert","tenant":10,"entity":0,"key_id":5004,"amount":4500,"region":2,"currency":2,"category":2,"status":0}
    ]
  }'
```

### Query: Average by Location

```bash
# Total readings and sum per location
curl -X POST https://{subdomain}.machx.dev/query \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": {"tenant": 10},
    "measure": {"type": "group_by_multi"},
    "of": "amount",
    "group_by": ["region"]
  }'
```

### Query: Critical Alerts Only

```bash
curl -X POST https://{subdomain}.machx.dev/query \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": {"tenant": 10, "category": 2},
    "measure": {"type": "extract"},
    "columns": ["key_id", "amount", "region"],
    "limit": 50
  }'
```

### Pipeline: Auto-Flag Critical Readings

Create a pipeline that runs on every insert and flags readings above a threshold:

```bash
curl -X POST https://{subdomain}.machx.dev/pipeline \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "slug": "flag-critical",
    "trigger": "on_insert",
    "entity_filter": 0,
    "tenant_filter": 10,
    "steps": [
      {"Scan": {"name": "check", "query": {"space": {"tenant": 10, "amount": [4000, null], "category": 0}, "measure": {"type": "count"}}}},
      "Notify"
    ],
    "description": "Notify subscribers when any unprocessed reading exceeds threshold"
  }'
```

---

## Financial: Atomic Bank Transfer

Transfer funds between two accounts with balance validation — all-or-nothing.

### Setup: Two Accounts

```bash
# Account A: balance $5,000
curl -X POST https://{subdomain}.machx.dev/mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{"op":"keyed_insert","tenant":1,"entity":0,"key_id":1,"amount":5000,"region":0,"currency":1,"category":0,"status":1}'

# Account B: balance $2,000
curl -X POST https://{subdomain}.machx.dev/mutate \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{"op":"keyed_insert","tenant":1,"entity":0,"key_id":2,"amount":2000,"region":0,"currency":1,"category":0,"status":1}'
```

### Create the Transfer Pipeline

```bash
curl -X POST https://{subdomain}.machx.dev/pipeline \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "slug": "bank-transfer",
    "trigger": "on_call",
    "steps": [
      {"Lookup": {"name": "sender", "predicates": [[0, 2, 4, 1]]}},
      {"Lookup": {"name": "receiver", "predicates": [[0, 2, 4, 2]]}},
      {"ValidateGte": {"slot_ref": "sender", "col_offset": 6, "col_width": 2, "min_value": 3000}},
      {"UpdateDelta": {"slot_ref": "sender", "col_offset": 6, "col_width": 2, "delta": -3000}},
      {"UpdateDelta": {"slot_ref": "receiver", "col_offset": 6, "col_width": 2, "delta": 3000}},
      "Notify"
    ],
    "description": "Transfer $3,000 from account 1 to account 2"
  }'
```

### Execute the Transfer

```bash
curl -X POST https://{subdomain}.machx.dev/pipeline/bank-transfer/call \
  -H 'Authorization: Bearer YOUR_API_KEY'
```

**What happens:**
1. Look up sender (key_id=1) → found, amount=5000
2. Look up receiver (key_id=2) → found, amount=2000
3. Validate sender.amount ≥ 3000 → **passes** (5000 ≥ 3000)
4. sender.amount = 5000 - 3000 = **2000**
5. receiver.amount = 2000 + 3000 = **5000**
6. Notify subscribers

If step 3 had failed (insufficient balance), steps 4 and 5 would never execute.

### Verify

```bash
curl -X POST https://{subdomain}.machx.dev/query \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": {"tenant": 1},
    "measure": {"type": "extract"},
    "columns": ["key_id", "amount"],
    "limit": 10
  }'
```

```json
{
  "count": 2,
  "rows": [
    {"slot": 100, "fields": [["key_id", 1], ["amount", 2000]]},
    {"slot": 101, "fields": [["key_id", 2], ["amount", 5000]]}
  ]
}
```

Total funds conserved: $7,000.

---

## Natural Language Queries

For quick exploration, use the NLQ endpoint:

```bash
# Plain English → structured query
curl -X POST https://{subdomain}.machx.dev/ask \
  -H 'Content-Type: application/json' \
  -d '{"text": "total revenue from region 3"}'
```

```json
{
  "query": {"space": {"region": 3}, "measure": {"type": "sum"}, "of": "amount"},
  "result": {"count": 1, "sum": 8999, "query_us": 6},
  "description": "Sum of amount where region = 3"
}
```

More examples:

```bash
# Top 5 orders
curl -X POST https://{subdomain}.machx.dev/ask -d '{"text": "top 5 orders by amount"}'

# Count by status
curl -X POST https://{subdomain}.machx.dev/ask -d '{"text": "orders per status"}'

# High-value orders in US + EU
curl -X POST https://{subdomain}.machx.dev/ask -d '{"text": "orders over 5000 from region 1 combined with region 2"}'
```

See [Query Language — Natural Language Queries](./query-language.md#natural-language-queries) for alias resolution and supported patterns.

---

## WebSocket: High-Throughput SDK Pattern

For applications that need real-time bidirectional communication:

### TypeScript: Query + Subscribe + Mutate over One Connection

```typescript
import { BitBin } from "@bitbin/client";

const bb = new BitBin({
  baseUrl: "https://{subdomain}.machx.dev",
  apiKey: "YOUR_API_KEY",
  transport: "websocket"  // use WS instead of REST
});

// Real-time subscription
bb.subscribe(
  { space: { tenant: 1 }, measure: { type: "count" } },
  (event) => console.log(`Live count: ${event.result.count}`)
);

// Write (goes over same WS connection)
await bb.insert({
  tenant: 1, entity: 0, key_id: 999,
  amount: 1500, region: 2, currency: 1, category: 4, status: 1
});

// Query (also over same WS connection)
const result = await bb.query({
  space: { tenant: 1, amount: [1000, null] },
  measure: { type: "sum" },
  of: "amount"
});
```

### Python: Batch Insert

```python
from bitbin import BitBin

bb = BitBin("https://{subdomain}.machx.dev", api_key="YOUR_API_KEY")

# Batch insert 1000 records
records = [
    {"tenant": 1, "entity": 0, "key_id": i, "amount": i * 10,
     "region": i % 5, "currency": 1, "category": i % 10, "status": 1}
    for i in range(1000)
]
bb.batch_insert(records)

# Query the result
result = bb.query(space={"tenant": 1}, measure={"type": "count"})
print(f"Total records: {result['count']}")
```

---

## Recipe: Leaderboard with Top-K

```bash
# Get top 10 players by score
curl -X POST https://{subdomain}.machx.dev/query \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": {"tenant": 1, "entity": 0},
    "measure": {"type": "top_k", "value": 10},
    "of": "amount",
    "columns": ["key_id", "amount", "region"],
    "sort_order": "desc"
  }'
```

```json
{
  "count": 500,
  "rows": [
    {"slot": 421, "fields": [["key_id", 77], ["amount", 9999], ["region", 3]]},
    {"slot": 102, "fields": [["key_id", 23], ["amount", 9500], ["region", 1]]},
    {"slot": 308, "fields": [["key_id", 45], ["amount", 8700], ["region", 2]]}
  ]
}
```

---

## Recipe: Multi-Region Aggregation (Union)

```bash
# Revenue from US + EU combined (regions 1 and 2)
curl -X POST https://{subdomain}.machx.dev/query \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": {"tenant": 1, "region": 1},
    "union": [{"region": 2}],
    "measure": {"type": "sum"},
    "of": "amount"
  }'
```

---

## Recipe: Cross-Filter (Join)

```bash
# High-value orders (amount > 5000) that are also in region 3
curl -X POST https://{subdomain}.machx.dev/query \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": {"tenant": 1, "amount": [5000, null]},
    "join": [{"region": 3}],
    "measure": {"type": "count"}
  }'
```

---

## Recipe: Scheduled Rollup Pipeline

Aggregate daily metrics every hour:

```bash
curl -X POST https://{subdomain}.machx.dev/pipeline \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -H 'Content-Type: application/json' \
  -d '{
    "slug": "hourly-rollup",
    "trigger": "on_schedule",
    "schedule_interval_secs": 3600,
    "steps": [
      {"Scan": {"name": "hourly", "query": {"space": {"tenant": 1}, "measure": {"type": "sum"}, "of": "amount"}}},
      "Notify"
    ],
    "description": "Hourly revenue rollup — pushes to subscribers every hour"
  }'
```

---

[← Previous: Pipelines](/pipelines) · **Chapter 8** · [Next: Platform API →](/platform)
