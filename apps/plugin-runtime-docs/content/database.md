---
title: Database Guide
order: 4
---

# Database Guide

> Every plugin gets a namespaced SQLite database with automatic table isolation per tenant and plugin.

## Overview

Plugins can create and manage relational tables using standard SQL. The runtime transparently prefixes all table names with `{tenant}_{plugin_short_id}_` to enforce isolation. You write natural table names (`orders`); the actual table might be `acme_019c5016_orders`.

Two approaches for database access:
- **`defineTable()`** — High-level CRUD helper (recommended)
- **Raw SQL** — `runtime.dbExec()` and `runtime.dbQuery()` for full control

---

## defineTable (Recommended)

### Define a table

```typescript
const orders = defineTable("orders", {
  id:         { type: "TEXT", primaryKey: true },
  customer:   { type: "TEXT", notNull: true, index: true },
  total:      { type: "REAL" },
  status:     { type: "TEXT", index: true },
  created_at: { type: "TEXT" },
});
```

### Column types

| Property | Type | Description |
|---|---|---|
| `type` | `"TEXT" \| "INTEGER" \| "REAL" \| "BLOB"` | SQLite column type (default: `"TEXT"`) |
| `primaryKey` | boolean | Primary key constraint |
| `notNull` | boolean | NOT NULL constraint |
| `default` | string | Default value expression |
| `index` | boolean | Auto-create index on `migrate()` |

### Migrate (create table)

Call once per execution (idempotent — safe to call every time):

```typescript
await orders.migrate(runtime);
```

This runs `CREATE TABLE IF NOT EXISTS` plus `CREATE INDEX IF NOT EXISTS` for any `index: true` columns.

### CRUD Operations

```typescript
// Insert a row
await orders.insert(runtime, {
  id: "ord_123",
  customer: "cus_abc",
  total: 99.50,
  status: "pending",
  created_at: new Date().toISOString(),
});

// Upsert (insert or replace on primary key conflict)
await orders.upsert(runtime, {
  id: "ord_123",
  total: 105.00,
  status: "paid",
});

// Update with WHERE clause
await orders.update(runtime, { status: "shipped" }, "id = ?", ["ord_123"]);

// Delete with WHERE clause
await orders.del(runtime, "id = ?", ["ord_123"]);

// Find by primary key
const order = await orders.findById(runtime, "ord_123");
// → { id: "ord_123", customer: "cus_abc", total: 105.00, ... } or null

// Find all with filters
const active = await orders.findAll(runtime, {
  where: "status = ?",
  params: ["pending"],
  orderBy: "created_at DESC",
  limit: 50,
  offset: 0,
});
// → [{ id: "ord_456", ... }, { id: "ord_789", ... }]

// Count rows
const count = await orders.count(runtime, "status = ?", ["pending"]);
// → 42

// Raw query (returns objects)
const results = await orders.query(runtime,
  "SELECT status, COUNT(*) as cnt FROM orders GROUP BY status"
);
// → [{ status: "pending", cnt: 15 }, { status: "paid", cnt: 27 }]
```

---

## Raw SQL

For queries that don't fit the `defineTable` model.

### Write operations

```typescript
// Create a table
await runtime.dbExec(`
  CREATE TABLE IF NOT EXISTS metrics (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    value REAL,
    recorded_at INTEGER
  )
`);

// Insert with parameters
await runtime.dbExec(
  "INSERT INTO metrics (id, name, value, recorded_at) VALUES (?, ?, ?, ?)",
  ["m_001", "conversion_rate", 0.045, Date.now()]
);

// Update
await runtime.dbExec(
  "UPDATE metrics SET value = ? WHERE name = ?",
  [0.052, "conversion_rate"]
);

// Delete
await runtime.dbExec("DELETE FROM metrics WHERE recorded_at < ?", [cutoffTs]);
```

### Read operations

```typescript
const result = await runtime.dbQuery(
  "SELECT name, value FROM metrics WHERE value > ? ORDER BY value DESC",
  [0.01]
);
// result.columns = ["name", "value"]
// result.rows = [["conversion_rate", 0.052], ["click_rate", 0.031]]
```

Tip: Use `RuntimeHelper.dbQueryRows()` for object results:

```typescript
const rt = new RuntimeHelper(runtime);
const rows = await rt.dbQueryRows(
  "SELECT name, value FROM metrics WHERE value > ?", [0.01]
);
// [{ name: "conversion_rate", value: 0.052 }, ...]
```

---

## Joins

Standard SQL joins work across all tables within the same plugin. Table names in your code are bare — the runtime auto-prefixes them.

```typescript
const customers = defineTable("customers", {
  id:    { type: "TEXT", primaryKey: true },
  email: { type: "TEXT", notNull: true },
  name:  { type: "TEXT" },
});

const orders = defineTable("orders", {
  id:          { type: "TEXT", primaryKey: true },
  customer_id: { type: "TEXT", notNull: true, index: true },
  total:       { type: "REAL" },
  status:      { type: "TEXT" },
});

// Migrate both
await customers.migrate(runtime);
await orders.migrate(runtime);

// JOIN query
const highValue = await runtime.dbQuery(
  `SELECT c.email, c.name, SUM(o.total) as lifetime_total, COUNT(o.id) as order_count
   FROM customers c
   JOIN orders o ON c.id = o.customer_id
   WHERE o.status = ?
   GROUP BY c.id
   HAVING lifetime_total > ?
   ORDER BY lifetime_total DESC`,
  ["paid", 100]
);
```

---

## Table Name Rewriting

The runtime auto-prefixes table names after SQL keywords (`FROM`, `JOIN`, `INTO`, `TABLE`, `UPDATE`, `ON`, etc.):

| You write | Actual table |
|---|---|
| `orders` | `"acme_019c5016_orders"` |
| `customers` | `"acme_019c5016_customers"` |

Table names are double-quoted in SQL since the prefix may start with a digit.

---

## SQL Validation

### `dbExec` (writes)

**Allowed:** `CREATE TABLE`, `INSERT`, `UPDATE`, `DELETE`, `ALTER TABLE`, `CREATE INDEX`, `DROP TABLE`, `DROP INDEX`

**Blocked:** `DROP DATABASE`, `ATTACH`, `DETACH`, `PRAGMA` (except `PRAGMA table_info`)

### `dbQuery` (reads)

**Allowed:** `SELECT` only — all write statements are blocked.

### General rules

- **Single statement per call** — multi-statement SQL with `;` separators is allowed
- **Table name validation:** must match `[a-zA-Z_][a-zA-Z0-9_]*`

---

## Schema Discovery

View your plugin's tables:

```typescript
const schema = await runtime.dbSchema();
// schema.tables = [
//   { name: "orders", columns: ["id", "customer_id", "total", "status"] },
//   { name: "customers", columns: ["id", "email", "name"] }
// ]
```

---

## Sub-Tenant Convention

Plugins serving multiple end-users should include a `sub_id` column and use `runtime.context.sub_id` for scoping:

```typescript
const messages = defineTable("messages", {
  id:         { type: "TEXT", primaryKey: true },
  sub_id:     { type: "TEXT", notNull: true, index: true },
  content:    { type: "TEXT" },
  created_at: { type: "INTEGER" },
});

export default definePlugin({
  name: "chat-store",
  events: ["message.sent"],

  async onEvent(event, runtime) {
    await messages.migrate(runtime);
    const subId = runtime.context.sub_id;

    await messages.insert(runtime, {
      id: event.event_id,
      sub_id: subId,
      content: event.params.content,
      created_at: Date.now(),
    });

    // Query scoped to the end-user
    const recent = await messages.findAll(runtime, {
      where: "sub_id = ?",
      params: [subId],
      orderBy: "created_at DESC",
      limit: 50,
    });
  },
});
```

**Isolation layers:**

| Layer | Isolation | Mechanism |
|---|---|---|
| Platform | `key_prefix` (tenant) | Enforced by runtime, table prefixing |
| Plugin | `plugin_id` prefix | Enforced by runtime, table name rewriting |
| End-user | `sub_id` column | Enforced by plugin code (convention) |

---

## Cross-Plugin Queries (Admin API)

Individual plugins are namespaced — Plugin A cannot see Plugin B's tables. The **Schema Discovery API** allows admin-level cross-plugin queries:

```bash
# Discover all tables for a tenant
curl -H "Authorization: Bearer $API_KEY" \
  https://plugins.juicyapi.com/schemas/acme

# Cross-plugin join using full table names
curl -X POST https://plugins.juicyapi.com/schemas/acme/query \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "sql": "SELECT c.email, o.total FROM acme_aaa_customers c JOIN acme_bbb_orders o ON c.id = o.customer_id WHERE o.total > 500"
  }'
```

This is read-only and requires the admin API key. Use it for dashboards, analytics, or AI query services.

---

← [SDK Reference](/sdk-reference) · **Chapter 4** · [BitBin Analytics →](/bitbin)
