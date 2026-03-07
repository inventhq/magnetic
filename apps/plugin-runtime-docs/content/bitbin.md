---
title: BitBin Analytics
order: 5
---

# BitBin Analytics

> Analytics, vector search, document store, and pre-built data pipelines via `runtime.bitbin`.

## Overview

BitBin is an analytics engine that provides:
- **Dimensional queries** — count, sum, min, max, group-by across multi-dimensional data
- **Data ingestion** — high-throughput record ingestion
- **Vector search** — nearest-neighbor search over embeddings
- **Pipes** — pre-built named data pipelines (saved queries)
- **Document store** — key-value document storage per entity

Access via `runtime.bitbin` in your plugin code. Requires `BITBIN_BASE_URL` and `BITBIN_API_KEY` to be configured on the runtime (no plugin-level setup needed).

---

## Query

Run dimensional analytics queries against ingested data.

```typescript
// Count all records for tenant 1, entity 10
const result = await runtime.bitbin.query({
  space: { tenant: 1, entity: 10 },
  measure: { type: "count" },
});
// result.count = 1523

// Sum amounts in a time range
const revenue = await runtime.bitbin.query({
  space: {
    tenant: 1,
    entity: 10,
    key_id: [1000, 2000],      // range: key_id between 1000 and 2000
  },
  measure: { type: "sum" },
  of: "amount",
});
// revenue.sum = 495000

// Group by region
const byRegion = await runtime.bitbin.query({
  space: { tenant: 1, entity: 10 },
  measure: { type: "group" },
  of: "region",
  limit: 10,
});
// byRegion.groups = [{ key: 1, count: 500, sum: 150000 }, { key: 2, count: 300, sum: 90000 }]
```

### Query Shape

```typescript
interface BitBinQuery {
  space: Record<string, number | [number | null, number | null]>;
  measure: { type: "count" } | { type: "sum" } | { type: "min" } | { type: "max" } | { type: "group" };
  of?: string;            // field to aggregate (required for sum/min/max/group)
  union?: Record<string, number | [number | null, number | null]>[];
  join?: Record<string, number | [number | null, number | null]>[];
  limit?: number;         // max groups for group-by
}
```

**Space dimensions:** Each key can be an exact value (`tenant: 1`) or a range (`key_id: [100, 200]`). Use `null` for open-ended ranges: `[100, null]` = ≥ 100.

### Query Result

```typescript
interface BitBinQueryResult {
  count?: number;
  sum?: number;
  min?: number;
  max?: number;
  groups?: Array<{ key: number; count: number; sum: number }>;
  query_us?: number;      // query execution time in microseconds
}
```

---

## Ingest

Push records into BitBin for analytics.

```typescript
const result = await runtime.bitbin.ingest({
  records: [
    {
      tenant: 1,
      entity: 10,
      key_id: 12345,
      amount: 9900,
      region: 1,
      currency: 840,      // ISO 4217 numeric (USD)
      category: 5,
      status: 1,
    },
    {
      tenant: 1,
      entity: 10,
      key_id: 12346,
      amount: 4500,
    },
  ],
});
// result.ingested = 2
```

### Record Shape

```typescript
interface BitBinIngestRecord {
  tenant: number;         // required — tenant identifier
  entity: number;         // required — entity type
  key_id: number;         // required — unique key
  amount: number;         // required — numeric value
  region?: number;        // optional dimensions
  currency?: number;
  category?: number;
  status?: number;
}
```

### Ingest Result

```typescript
interface BitBinIngestResponse {
  ingested: number;       // records successfully ingested
  total_records?: number; // total records in the system
  ingest_us?: number;     // ingestion time in microseconds
}
```

---

## Vector Search

Find nearest neighbors by embedding vector.

```typescript
const matches = await runtime.bitbin.vectorSearch({
  tenant: 1,
  embedding: [0.1, 0.2, 0.3, /* ... 1536 floats */],
  entity: 10,
  top_k: 5,
});
// matches = [
//   { vector_id: 42, distance: 0.05, slot: 1 },
//   { vector_id: 99, distance: 0.12, slot: 3 },
// ]
```

### Request Shape

```typescript
interface BitBinVectorSearchRequest {
  tenant: number;         // required
  embedding: number[];    // required — float array
  entity?: number;        // optional filter
  top_k?: number;         // number of results (default varies by config)
}
```

### Match Shape

```typescript
interface BitBinVectorMatch {
  vector_id: number;
  distance: number;       // lower = more similar
  slot?: number;
}
```

---

## Pipes (Saved Queries)

Execute pre-built named data pipelines.

```typescript
// Execute a named pipe
const result = await runtime.bitbin.pipeExecute("revenue-by-month");
// result = BitBinQueryResult

// Call a pipeline with parameters
const data = await runtime.bitbin.pipelineCall("top-customers", {
  tenant: 1,
  limit: 10,
  min_amount: 1000,
});
// data = arbitrary JSON returned by the pipeline
```

Pipes are defined and managed in the BitBin service. They encapsulate complex query logic behind a simple name.

---

## Document Store

Key-value document storage scoped by tenant and entity.

### Set a document

```typescript
await runtime.bitbin.doc.set({
  tenant: 1,
  entity: 10,
  doc_id: "customer_profile_cus_123",
  data: {
    name: "Alice",
    email: "alice@example.com",
    lifetime_value: 15000,
    tags: ["vip", "enterprise"],
  },
});
```

### Get a document

```typescript
const doc = await runtime.bitbin.doc.get({
  tenant: 1,
  entity: 10,
  doc_id: "customer_profile_cus_123",
});
// doc = { name: "Alice", email: "alice@example.com", ... }
```

### Delete a document

```typescript
await runtime.bitbin.doc.delete({
  tenant: 1,
  entity: 10,
  doc_id: "customer_profile_cus_123",
});
```

### List documents

```typescript
const docs = await runtime.bitbin.doc.list({
  tenant: 1,
  entity: 10,
});
// docs = [{ ... }, { ... }]
```

---

## Type Reference

All types are available in `plugin-runtime.d.ts` for full autocomplete:

```typescript
interface BitBinClient {
  query(q: BitBinQuery): Promise<BitBinQueryResult>;
  ingest(req: BitBinIngestRequest): Promise<BitBinIngestResponse>;
  vectorSearch(req: BitBinVectorSearchRequest): Promise<BitBinVectorMatch[]>;
  pipeExecute(slug: string): Promise<BitBinQueryResult>;
  pipelineCall(slug: string, params?: unknown): Promise<unknown>;
  doc: {
    set(opts: BitBinDocSetRequest): Promise<{ ok: boolean }>;
    get(opts: BitBinDocGetRequest): Promise<unknown>;
    delete(opts: BitBinDocGetRequest): Promise<{ ok: boolean }>;
    list(opts: BitBinDocListRequest): Promise<unknown[]>;
  };
}
```

---

## Example: Event-Driven Analytics Pipeline

```typescript
const chargesTable = defineTable("charges", {
  id:       { type: "TEXT", primaryKey: true },
  amount:   { type: "INTEGER" },
  currency: { type: "TEXT" },
  status:   { type: "TEXT", index: true },
});

export default definePlugin({
  name: "payment-analytics",
  events: ["charge.succeeded"],

  async onEvent(event, runtime) {
    const rt = new RuntimeHelper(runtime);
    await chargesTable.migrate(runtime);

    const payload = event.raw_payload as Record<string, unknown>;
    const amount = Number(payload.amount) || 0;

    // Store in relational DB for queries
    await chargesTable.upsert(runtime, {
      id: String(payload.id),
      amount: amount,
      currency: String(payload.currency || "usd"),
      status: "succeeded",
    });

    // Ingest into BitBin for real-time analytics
    await runtime.bitbin.ingest({
      records: [{
        tenant: 1,
        entity: 10,        // 10 = charges entity type
        key_id: Date.now(),
        amount: amount,
        currency: 840,      // USD
        status: 1,          // succeeded
      }],
    });

    // Query running totals from BitBin
    const totals = await runtime.bitbin.query({
      space: { tenant: 1, entity: 10 },
      measure: { type: "sum" },
      of: "amount",
    });

    rt.info(`Charge $${(amount / 100).toFixed(2)} — running total: $${((totals.sum || 0) / 100).toFixed(2)}`);
  },
});
```

---

## Example: Semantic Search with Embeddings

```typescript
export default definePlugin({
  name: "doc-search",
  events: ["search.query"],

  async onEvent(event, runtime) {
    const rt = new RuntimeHelper(runtime);
    const query = event.params.query;

    // Get embedding from OpenAI
    const embResp = await rt.fetchJSON("https://api.openai.com/v1/embeddings", {
      method: "POST",
      headers: {
        "Authorization": "Bearer " + rt.getConfigValue("openai_key", ""),
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: "text-embedding-3-small",
        input: query,
      }),
    });

    const embedding = embResp.data[0].embedding;

    // Search BitBin vector index
    const matches = await runtime.bitbin.vectorSearch({
      tenant: 1,
      embedding: embedding,
      top_k: 5,
    });

    // Fetch matched documents
    for (const match of matches) {
      const doc = await runtime.bitbin.doc.get({
        tenant: 1,
        entity: 1,
        doc_id: "doc_" + match.vector_id,
      });

      rt.info(`Match (distance: ${match.distance.toFixed(3)}): ${JSON.stringify(doc)}`);
    }

    await rt.emit("search.results", {
      query: query,
      match_count: String(matches.length),
    });
  },
});
```

---

← [Database Guide](/database) · **Chapter 5** · [Connectors →](/connectors)
