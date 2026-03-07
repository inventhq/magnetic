---
title: SDK Reference
order: 3
---

# SDK Reference

> Complete API documentation for the plugin-runtime SDK.

## Environment Rules

1. Plugins run inside **V8 isolates** (not Node.js). No `require()`, no `import` from packages, no `process`, no `fs`, no `Buffer`.
2. All SDK functions are **globals**. Never write `import { definePlugin } from '...'`. Just use `definePlugin(...)` directly.
3. The plugin file must `export default` the result of `definePlugin({...})` or `defineConnector({...})`.
4. The `runtime` object is passed as the **second argument** to `onEvent(event, runtime)`. It is NOT a global.
5. All `params` values in events and emits **must be strings**. `RuntimeHelper` auto-coerces; raw `runtime.emit()` does not.
6. `runtime.fetch()` returns `{ status, headers, body }` where `body` is a **raw string**. Always `JSON.parse(resp.body)`.
7. `runtime.fetch()` is **blocked by default**. You must configure `allowed_domains` in `plugin.json` or via the API.

---

## Global Functions

These are injected into every plugin's V8 isolate. Use them directly — no imports.

| Global | Description |
|---|---|
| `definePlugin(config)` | Define an event handler plugin |
| `defineConnector(config)` | Define a connector plugin |
| `RuntimeHelper` | Class — `new RuntimeHelper(runtime)` |
| `defineTable(name, columns)` | Create a typed DB table helper |
| `autoPaginate(fetchPage)` | Create a paginator from a fetch function |
| `assertType(value, type, label)` | Throw if `typeof` mismatch |
| `assertOneOf(value, allowed, label)` | Throw if value not in array |
| `assertShape(obj, schema, label)` | Validate object shape |
| `validateNormalizedEvent(event)` | Validate + coerce a `NormalizedEvent` |

**Provider-specific globals** (Stripe, Shopify) are also available — see [Examples](./examples.md).

---

## runtime.* API

Every plugin receives `runtime` as the second argument to `onEvent`. These are the methods available:

### Event Emission

#### `runtime.emit(eventType, params?, rawPayload?)`

Emit a derived event back into the ingest pipeline. The tenant's `key_prefix` is injected server-side — do not include it in params.

```typescript
runtime.emit("order.processed", {
  order_id: "ord_123",
  total: "9900"           // params values must be strings
});

// With raw payload
runtime.emit("order.processed", { order_id: "ord_123" }, { full: "object" });
```

**Limits:** Configurable via `max_emits_per_invocation`. Exceeding the limit throws an error.

### Data Queries

#### `runtime.query(sql, hours?, limit?)`

Execute a SQL query via the Platform API (analytics-level queries, not plugin DB).

```typescript
const result = await runtime.query(
  "SELECT event_type, count(*) as cnt FROM events GROUP BY event_type",
  24,   // hours lookback
  100   // row limit
);
```

#### `runtime.getEvents(options?)`

Fetch recent events from the platform.

```typescript
const recent = await runtime.getEvents({
  hours: 1,
  limit: 10,
  event_type: "order.created"
});
```

#### `runtime.getStats(options?)`

Fetch aggregated event stats.

```typescript
const stats = await runtime.getStats({ hours: 24 });
```

### State Management

#### `runtime.getState(key): string | null`

Read a value from per-plugin persistent key-value state.

```typescript
const counter = runtime.getState("click_count");  // "42" or null
```

#### `runtime.setState(key, value)`

Write a value. Values must be strings.

```typescript
runtime.setState("click_count", "43");
```

### Configuration

#### `runtime.getConfig(): Record<string, unknown>`

Returns the plugin's config object (set via `PATCH /plugins/:id`, not in code).

```typescript
const config = runtime.getConfig();
const apiKey = config.api_key;     // unknown type — cast as needed
const threshold = config.threshold || 50;
```

### Database

#### `runtime.dbExec(sql, params?): { rows_affected: number }`

Execute a write SQL statement on the plugin's namespaced database. Table names are auto-prefixed.

```typescript
await runtime.dbExec(
  "INSERT INTO orders (id, total) VALUES (?, ?)",
  ["ord_123", 9900]
);
```

**Allowed:** `CREATE TABLE`, `INSERT`, `UPDATE`, `DELETE`, `ALTER TABLE`, `CREATE INDEX`, `DROP TABLE`, `DROP INDEX`
**Blocked:** `DROP DATABASE`, `ATTACH`, `DETACH`, `PRAGMA` (except `PRAGMA table_info`)

#### `runtime.dbQuery(sql, params?): { columns: string[], rows: unknown[][] }`

Execute a read query. Returns column names and rows as arrays.

```typescript
const result = await runtime.dbQuery(
  "SELECT id, total FROM orders WHERE total > ?",
  [1000]
);
// result.columns = ["id", "total"]
// result.rows = [["ord_123", 9900], ["ord_456", 5000]]
```

**Allowed:** `SELECT` only — all write statements blocked.

#### `runtime.dbSchema(): { tables: { name, columns }[] }`

List the plugin's own tables and columns.

```typescript
const schema = await runtime.dbSchema();
// schema.tables = [{ name: "orders", columns: ["id", "total", "status"] }]
```

### HTTP Fetch

#### `runtime.fetch(url, options?): Promise<{ status, headers, body }>`

Make an outbound HTTP request. **Blocked by default** — requires `allowed_domains` configuration.

```typescript
const resp = await runtime.fetch("https://api.stripe.com/v1/charges/ch_123", {
  method: "GET",
  headers: { "Authorization": "Bearer sk_test_..." },
  timeout_ms: 10000
});

const data = JSON.parse(resp.body);  // body is always a string
```

| Option | Type | Default | Description |
|---|---|---|---|
| `method` | string | `"GET"` | HTTP method |
| `headers` | object | `{}` | Request headers |
| `body` | string | `null` | Request body (must be a string, use `JSON.stringify()`) |
| `timeout_ms` | number | `300000` | Request timeout in ms |

**Response:**

| Field | Type | Description |
|---|---|---|
| `status` | number | HTTP status code |
| `headers` | object | Response headers |
| `body` | string | Response body (max 1 MB, raw string) |

### Crypto

#### `runtime.crypto.verifyHmac(payload, signature, secret, algorithm?): boolean`

Verify an HMAC signature. Algorithms: `"sha256"` (default), `"sha1"`. Accepts hex or base64 encoded signatures.

```typescript
const valid = runtime.crypto.verifyHmac(requestBody, sig, webhookSecret, "sha256");
```

#### `runtime.crypto.parseSignatureHeader(header, format?): Record<string, string>`

Parse a signature header. Formats: `"stripe"` (parses `t=...,v1=...`), `"raw"` (returns `{ signature }`).

```typescript
const parsed = runtime.crypto.parseSignatureHeader(stripeHeader, "stripe");
// parsed.timestamp, parsed.v1
```

### Logging

```typescript
runtime.log.info("Processing order");
runtime.log.warn("Rate limit approaching");
runtime.log.error("Payment failed: " + error.message);
```

Logs are stored in an in-memory ring buffer (200 entries per plugin) and viewable via `GET /plugins/:id/logs` or `plugin logs`.

### Context

```typescript
runtime.context.sub_id  // string | undefined — from event.params.sub_id
```

Used for sub-tenant context pass-through. See [Database Guide](./database.md#sub-tenant-convention) for usage patterns.

### BitBin Analytics

```typescript
runtime.bitbin.query(...)
runtime.bitbin.ingest(...)
runtime.bitbin.vectorSearch(...)
runtime.bitbin.pipeExecute(...)
runtime.bitbin.pipelineCall(...)
runtime.bitbin.doc.set(...)
runtime.bitbin.doc.get(...)
runtime.bitbin.doc.delete(...)
runtime.bitbin.doc.list(...)
```

See [BitBin Guide](./bitbin.md) for full documentation.

---

## RuntimeHelper

Wraps `runtime` with convenience methods, input validation, and type coercion. **Recommended** for all plugin development.

```typescript
const rt = new RuntimeHelper(runtime);
```

### Methods

| Method | Improvement over raw API |
|---|---|
| `rt.emit(type, params)` | Auto-coerces param values to strings |
| `rt.emitNormalized(event)` | Accepts a `NormalizedEvent` directly |
| `rt.getConfig<T>()` | Returns typed config object |
| `rt.getConfigValue(key, fallback)` | Safe config access with default |
| `rt.getState(key)` | Same as `runtime.getState` |
| `rt.setState(key, value)` | Auto-coerces value to string |
| `rt.getStateOr(key, fallback)` | Returns fallback if state is null |
| `rt.fetch(url, opts?)` | Same as `runtime.fetch` |
| `rt.fetchJSON<T>(url, opts?)` | Auto-parses `resp.body` as JSON |
| `rt.dbExec(sql, params?)` | Same as `runtime.dbExec` |
| `rt.dbQuery(sql, params?)` | Same as `runtime.dbQuery` |
| `rt.dbQueryRows(sql, params?)` | Returns `Record<string, unknown>[]` (objects, not arrays) |
| `rt.dbSchema()` | Same as `runtime.dbSchema` |
| `rt.info(msg)` | Shorthand for `runtime.log.info` |
| `rt.warn(msg)` | Shorthand for `runtime.log.warn` |
| `rt.error(msg)` | Shorthand for `runtime.log.error` |
| `rt.logJSON(label, obj)` | Logs JSON.stringify at info level |
| `rt.getSubId()` | Returns `runtime.context.sub_id` |
| `rt.raw` | Access the underlying `runtime` object |

### Example

```typescript
export default definePlugin({
  name: "my-plugin",
  events: ["order.created"],

  async onEvent(event, runtime) {
    const rt = new RuntimeHelper(runtime);

    const config = rt.getConfig();
    const apiKey = rt.getConfigValue("api_key", "");
    const count = parseInt(await rt.getStateOr("counter", "0")) + 1;
    await rt.setState("counter", count);   // auto-coerced to "1"

    // Auto-parse JSON response
    const data = await rt.fetchJSON("https://api.example.com/data", {
      headers: { "Authorization": "Bearer " + apiKey }
    });

    // Rows as objects instead of arrays
    const rows = await rt.dbQueryRows("SELECT * FROM orders WHERE status = ?", ["active"]);
    // [{ id: "ord_1", status: "active", total: 500 }, ...]

    await rt.emit("order.counted", { count: count, order_id: event.params.order_id });
    rt.info("Processed order #" + count);
  },
});
```

---

## defineTable

Creates a typed table helper with CRUD operations. See [Database Guide](./database.md) for full details.

```typescript
const users = defineTable("users", {
  id:    { type: "TEXT", primaryKey: true },
  email: { type: "TEXT", notNull: true, index: true },
  name:  { type: "TEXT" },
  score: { type: "INTEGER", default: "0" },
});

await users.migrate(runtime);                                    // CREATE TABLE + indexes
await users.insert(runtime, { id: "u1", email: "a@b.com" });    // INSERT
await users.upsert(runtime, { id: "u1", email: "new@b.com" });  // INSERT OR REPLACE
await users.update(runtime, { score: 42 }, "id = ?", ["u1"]);   // UPDATE WHERE
await users.del(runtime, "id = ?", ["u1"]);                      // DELETE WHERE
const user = await users.findById(runtime, "u1");                // → object | null
const all = await users.findAll(runtime, {                       // → object[]
  where: "score > ?", params: [10],
  orderBy: "score DESC", limit: 50
});
const n = await users.count(runtime, "score > ?", [10]);         // → number
const rows = await users.query(runtime, "SELECT ...", []);       // → object[]
```

### Column Definition

| Property | Type | Description |
|---|---|---|
| `type` | `"TEXT" \| "INTEGER" \| "REAL" \| "BLOB"` | Column type (default: `"TEXT"`) |
| `primaryKey` | boolean | Primary key constraint |
| `notNull` | boolean | NOT NULL constraint |
| `default` | string | Default value expression |
| `index` | boolean | Auto-create index on `migrate()` |

### Properties

| Property | Type | Description |
|---|---|---|
| `table.name` | string | Bare table name (e.g. `"users"`) |
| `table.columns` | object | Column definitions |
| `table.columnNames` | string[] | Array of column names |

---

## Validation Helpers

```typescript
// Type checking
assertType(value, "string", "customer_id");
assertOneOf(status, ["active", "cancelled"], "status");

// Shape validation
assertShape(obj, {
  email:  { required: true, type: "string" },
  amount: { required: true, type: "number" },
  status: { oneOf: ["pending", "paid"] },
});

// Validate a normalized event (auto-coerces params to strings)
const event = validateNormalizedEvent({
  event_type: "order.created",
  params: { amount: 99 },  // 99 → "99"
});
```

---

## Auto-Pagination

```typescript
const paginator = autoPaginate(async (cursor) => {
  const resp = await runtime.fetch("https://api.example.com/items?cursor=" + (cursor || ""));
  const data = JSON.parse(resp.body);
  return { items: data.items, nextCursor: data.next, hasMore: !!data.next };
});

const allItems = await paginator.all(runtime);          // Fetch everything
const first50  = await paginator.take(50, runtime);     // Fetch exactly 50 items
await paginator.forEach(runtime, (items) => { ... });   // Process page by page
const page     = await paginator.page(null, runtime);   // Fetch one page
```

---

## Common Mistakes

### 1. Importing SDK functions
```typescript
// WRONG
import { definePlugin } from '@plugin-runtime/sdk';

// RIGHT — all SDK functions are globals
export default definePlugin({ ... });
```

### 2. Non-string params
```typescript
// WRONG — raw runtime.emit doesn't coerce
runtime.emit("order.created", { amount: 99 });

// RIGHT — use RuntimeHelper
const rt = new RuntimeHelper(runtime);
rt.emit("order.created", { amount: 99 }); // auto-coerced to "99"
```

### 3. Using fetch body as JSON directly
```typescript
// WRONG — body is a string
const resp = await runtime.fetch("https://api.example.com/data");
const id = resp.body.id;

// RIGHT — parse the string
const data = JSON.parse(resp.body);
const id = data.id;
```

### 4. Forgetting null check on getState
```typescript
// WRONG
const count = parseInt(runtime.getState("counter")) + 1; // NaN

// RIGHT
const rt = new RuntimeHelper(runtime);
const count = parseInt(await rt.getStateOr("counter", "0")) + 1;
```

### 5. POST body as object
```typescript
// WRONG
await runtime.fetch("https://api.example.com", {
  method: "POST",
  body: { key: "value" },
});

// RIGHT
await runtime.fetch("https://api.example.com", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ key: "value" }),
});
```

### 6. Using full table names
```typescript
// WRONG — table names are auto-prefixed
await runtime.dbExec("CREATE TABLE IF NOT EXISTS 6vct_019c_orders (id TEXT)");

// RIGHT — use bare table names
await runtime.dbExec("CREATE TABLE IF NOT EXISTS orders (id TEXT PRIMARY KEY)");
```

### 7. Forgetting export default
```typescript
// WRONG
definePlugin({ name: "my-plugin", ... });

// RIGHT
export default definePlugin({ name: "my-plugin", ... });
```

---

← [Plugin Configuration](./plugin-json.md) · **Chapter 3** · [Database Guide →](./database.md)
