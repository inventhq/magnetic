---
title: Getting Started
order: 1
---

# Chapter 1: Getting Started

**Your first BitBin database in 5 minutes.**

> **Prerequisites:** `curl` and a terminal. No local install required — BitBin is fully managed.

---

## Step 1: Sign Up

```bash
# Send a one-time code to your email
curl -X POST https://machx.dev/auth/otp/send \
  -H 'Content-Type: application/json' \
  -d '{"email": "you@example.com"}'

# Verify the code
curl -X POST https://machx.dev/auth/otp/verify \
  -H 'Content-Type: application/json' \
  -d '{"email_id": "email-xxx", "code": "123456"}'
# → Returns: session_token

# Create your account
curl -X POST https://machx.dev/signup \
  -H 'Content-Type: application/json' \
  -d '{"account_id": "my-company", "name": "My Company", "email": "you@example.com"}'
# → Returns: session_token, api_key
```

Save your `api_key` — it won't be shown again.

## Step 2: Create a Database

```bash
curl -X POST https://machx.dev/accounts/my-company/databases \
  -H 'Authorization: Bearer ses_...' \
  -H 'Content-Type: application/json' \
  -d '{"db_id": "my-app"}'
```

Response:
```json
{
  "db_id": "my-app",
  "subdomain": "edba2ddef755",
  "api_key": "bb_live_abc123...",
  "endpoints": {
    "connection_url": "https://edba2ddef755.machx.dev",
    "rest": "https://edba2ddef755.machx.dev/query",
    "ws": "wss://edba2ddef755.machx.dev/ws",
    "ingest": "https://edba2ddef755.machx.dev/ingest"
  }
}
```

Your database is now live. Use the `connection_url` for all subsequent requests.

## Step 3: Ingest Some Data

```bash
# Insert 500 sample records
curl -X POST https://edba2ddef755.machx.dev/ingest \
  -H 'Authorization: Bearer bb_live_abc123...' \
  -H 'Content-Type: application/json' \
  -d '{"count": 500}'
```

For real applications, use keyed inserts for individual records or the WebSocket protocol for high-throughput ingestion. See [Data Operations](./data-operations.md).

## Step 4: Run Your First Query

```bash
# Count all records
curl -X POST https://edba2ddef755.machx.dev/query \
  -H 'Authorization: Bearer bb_live_abc123...' \
  -H 'Content-Type: application/json' \
  -d '{"space": {}, "measure": {"type": "count"}}'
```

Response:
```json
{"count": 500, "query_us": 4}
```

That's 4 microseconds. Now try a filter:

```bash
# Sum of amounts where amount > 500
curl -X POST https://edba2ddef755.machx.dev/query \
  -H 'Authorization: Bearer bb_live_abc123...' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": {"amount": [500, null]},
    "measure": {"type": "sum"},
    "of": "amount"
  }'
```

```json
{"count": 312, "sum": 5400000, "query_us": 12}
```

See [Query Language](./query-language.md) for the full DSL: ranges, joins, unions, group-by, top-k, and more.

## Step 5: Subscribe to Live Updates

```bash
# SSE stream — pushes when data changes
curl -X POST https://edba2ddef755.machx.dev/subscribe \
  -H 'Content-Type: application/json' \
  -d '{"space": {}, "measure": {"type": "count"}}'
```

```
event: result
data: {"event":"initial","fingerprint":"a1b2c3d4","result":{"count":500,"query_us":4}}

event: result
data: {"event":"update","fingerprint":"b2c3d4e5","result":{"count":501,"query_us":3}}
```

Updates push only when the result actually changes — no polling. See [Real-Time Subscriptions](./realtime.md).

## Step 6: Save a Query as a Pipe

```bash
curl -X POST https://edba2ddef755.machx.dev/pipe \
  -H 'Authorization: Bearer bb_live_abc123...' \
  -H 'Content-Type: application/json' \
  -d '{
    "slug": "revenue-by-region",
    "query": {"space": {}, "measure": {"type": "group"}},
    "description": "Revenue breakdown by region"
  }'
```

Now you can:
- Execute anytime: `GET /pipe/revenue-by-region`
- Stream live: `GET /pipe/revenue-by-region/stream` (SSE)

---

## Connect via SDK

**TypeScript:**
```bash
npm install @bitbin/client
```

```typescript
import { BitBin } from "@bitbin/client";

const bb = new BitBin({
  baseUrl: "https://edba2ddef755.machx.dev",
  apiKey: "bb_live_abc123..."
});

// Query
const result = await bb.query({
  space: { amount: [500, null] },
  measure: { type: "sum" },
  of: "amount"
});

// Subscribe
bb.subscribe(
  { space: {}, measure: { type: "count" } },
  (event) => console.log(`Count: ${event.result.count}`)
);
```

**Python:**
```bash
pip install bitbin
```

```python
from bitbin import BitBin

bb = BitBin("https://edba2ddef755.machx.dev", api_key="bb_live_abc123...")

result = bb.query(space={"amount": [500, None]}, measure={"type": "sum"}, of="amount")
print(result)  # {"count": 312, "sum": 5400000, "query_us": 12}
```

---

## What's Next

You have a live database. Here's where to go from here:

- **Understand the data model** → [Core Concepts](./concepts.md) — tenants, entities, dimensions, record layout
- **Learn the query DSL** → [Query Language](./query-language.md) — filters, aggregations, joins, unions
- **Write data properly** → [Data Operations](./data-operations.md) — keyed CRUD, batch inserts, schema extensions
- **Build real-time features** → [Real-Time Subscriptions](./realtime.md) — SSE push, multiplexed queries
- **Add business logic** → [Pipelines](./pipelines.md) — atomic transactions, triggers, stored procedures
- **See full patterns** → [Examples & Recipes](./examples.md) — e-commerce, IoT, dashboards, SaaS metrics

---

[← Introduction](./index.md) · **Chapter 1** · [Next: Core Concepts →](./concepts.md)
