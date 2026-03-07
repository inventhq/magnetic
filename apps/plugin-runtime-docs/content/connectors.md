---
title: Connectors
order: 6
---

# Connectors Guide

> Build connectors that ingest data via webhooks, polling, and historical backfill.

## Overview

Connectors are plugins that proactively pull or receive data from external APIs. They use `defineConnector()` instead of `definePlugin()` and support three lifecycle methods:

- **Webhook** — Receive and verify incoming webhook payloads
- **Poller** — Periodically fetch new data on a schedule
- **Backfill** — Import historical data in paginated batches

All three normalize external data into standard events that flow through the platform pipeline.

---

## Quick Start

```typescript
export default defineConnector({
  name: "my-connector",

  webhook: {
    verify(body, headers, secret, runtime) {
      const sig = headers["x-signature"] || "";
      return runtime.crypto.verifyHmac(
        typeof body === "string" ? body : JSON.stringify(body),
        sig, secret, "sha256"
      );
    },
    normalize(body, headers) {
      const data = typeof body === "string" ? JSON.parse(body) : body;
      return {
        event_type: "my_service." + (data.action || "unknown"),
        params: { id: String(data.id || "") },
        raw_payload: data,
      };
    },
  },

  poller: {
    async fetch(cursor, runtime) {
      const config = runtime.getConfig();
      const url = "https://api.example.com/items"
        + (cursor ? "?after=" + cursor : "") + "&limit=100";
      const resp = await runtime.fetch(url, {
        headers: { "Authorization": "Bearer " + config.api_key },
      });
      const data = JSON.parse(resp.body);
      return {
        items: data.items,
        nextCursor: data.next_cursor || null,
        hasMore: !!data.next_cursor,
      };
    },
    normalize(item) {
      return {
        event_type: "my_service.item." + item.status,
        params: { item_id: String(item.id), name: String(item.name) },
        raw_payload: item,
      };
    },
  },
});
```

---

## Registering a Connector

### Via CLI

Set `plugin_type` in `plugin.json`:

```json
{
  "name": "my-connector",
  "entry": "src/plugin.ts",
  "events": [],
  "tenant": "acme",
  "runtime_url": "https://plugins.juicyapi.com",
  "plugin_type": "connector",
  "allowed_domains": ["api.example.com"],
  "connector_config": {
    "poller_enabled": true,
    "poll_interval_secs": 60
  }
}
```

```bash
plugin deploy
```

### Via API

```bash
curl -X POST https://plugins.juicyapi.com/plugins \
  -H "Content-Type: application/json" \
  -d '{
    "key_prefix": "acme",
    "name": "my-connector",
    "events": [],
    "code": "export default defineConnector({ ... });",
    "plugin_type": "connector",
    "allowed_domains": ["api.example.com"],
    "connector_config": "{\"poller_enabled\": true, \"poll_interval_secs\": 60}"
  }'
```

Response includes an auto-provisioned `poller_id` if `poller_enabled` is true.

---

## Webhook Lifecycle

When a webhook arrives at `POST /webhooks/:provider/:tenant`:

### 1. verify(body, headers, secret, runtime) → boolean

Validate the webhook signature. Return `true` if valid.

```typescript
webhook: {
  verify(body, headers, secret, runtime) {
    // headers keys are lowercase
    const sig = headers["x-webhook-signature"] || "";

    // body may be string or parsed object depending on content-type
    const payload = typeof body === "string" ? body : JSON.stringify(body);

    return runtime.crypto.verifyHmac(payload, sig, secret, "sha256");
  },
}
```

**Parameters:**
| Param | Type | Description |
|---|---|---|
| `body` | `unknown` | Raw request body (string or parsed JSON) |
| `headers` | `Record<string, string>` | Request headers (lowercase keys) |
| `secret` | `string` | Webhook secret from connector config |
| `runtime` | `Runtime` | Full runtime API access |

### 2. normalize(body, headers) → NormalizedEvent

Convert the webhook payload into a standard event.

```typescript
webhook: {
  normalize(body, headers) {
    const data = typeof body === "string" ? JSON.parse(body) : body;
    return {
      event_type: "service." + data.type,
      params: {
        id: String(data.id || ""),
        action: String(data.action || ""),
      },
      raw_payload: data,
    };
  },
}
```

**NormalizedEvent shape:**
```typescript
{
  event_type: string;                   // dot-separated event type
  params: Record<string, string>;       // ALL values must be strings
  raw_payload?: unknown;                // original payload (preserved as-is)
}
```

---

## Poller Lifecycle

Pollers run on a schedule (configured via `connector_config.poll_interval_secs`).

### 1. fetch(cursor, runtime) → PollerFetchResult

Fetch a page of data. `cursor` is `null` on the first call, then the value of `nextCursor` from the previous result.

```typescript
poller: {
  async fetch(cursor, runtime) {
    const config = runtime.getConfig();
    const url = cursor
      ? `https://api.example.com/items?after=${cursor}&limit=100`
      : "https://api.example.com/items?limit=100";

    const resp = await runtime.fetch(url, {
      headers: { "Authorization": "Bearer " + config.api_key },
    });
    const data = JSON.parse(resp.body);

    return {
      items: data.items || [],
      nextCursor: data.next_cursor || null,
      hasMore: !!data.next_cursor,
    };
  },
}
```

**Return shape:**
```typescript
interface PollerFetchResult {
  items: unknown[];          // array of items to normalize
  nextCursor: string | null; // cursor for next page (null = done)
  hasMore: boolean;          // whether more pages exist
}
```

### 2. normalize(item) → NormalizedEvent

Convert each item into a standard event.

```typescript
poller: {
  normalize(item) {
    return {
      event_type: "service.item." + item.status,
      params: { item_id: String(item.id) },
      raw_payload: item,
    };
  },
}
```

---

## Backfill Lifecycle

Triggered via `POST /backfills`. Same as poller but receives `pageSize` and is designed for historical imports.

### 1. fetch(cursor, pageSize, runtime) → PollerFetchResult

```typescript
backfill: {
  async fetch(cursor, pageSize, runtime) {
    const config = runtime.getConfig();
    const limit = Math.min(pageSize, 100);
    const url = cursor
      ? `https://api.example.com/items?after=${cursor}&limit=${limit}`
      : `https://api.example.com/items?limit=${limit}`;

    const resp = await runtime.fetch(url, {
      headers: { "Authorization": "Bearer " + config.api_key },
    });
    const data = JSON.parse(resp.body);

    return {
      items: data.items || [],
      nextCursor: data.next_cursor || null,
      hasMore: !!data.next_cursor,
    };
  },
}
```

### 2. normalize(item) → NormalizedEvent

Same as poller. If `backfill.normalize` is omitted, `poller.normalize` is used as fallback.

If `backfill` is not defined at all, the poller's `fetch` and `normalize` are used.

---

## Using Database with Connectors

Connectors can define tables and persist data as they ingest:

```typescript
const itemsTable = defineTable("items", {
  id:         { type: "TEXT", primaryKey: true },
  name:       { type: "TEXT", notNull: true },
  status:     { type: "TEXT", index: true },
  amount:     { type: "INTEGER" },
  synced_at:  { type: "TEXT" },
});

export default defineConnector({
  name: "item-sync",
  tables: [itemsTable],

  poller: {
    async fetch(cursor, runtime) {
      await itemsTable.migrate(runtime);

      const config = runtime.getConfig();
      const resp = await runtime.fetch(
        "https://api.example.com/items?after=" + (cursor || ""),
        { headers: { "Authorization": "Bearer " + config.api_key } }
      );
      const data = JSON.parse(resp.body);

      // Persist to local DB as we ingest
      for (const item of data.items) {
        await itemsTable.upsert(runtime, {
          id: item.id,
          name: item.name,
          status: item.status,
          amount: item.amount,
          synced_at: new Date().toISOString(),
        });
      }

      return {
        items: data.items,
        nextCursor: data.next_cursor || null,
        hasMore: !!data.next_cursor,
      };
    },
    normalize(item) {
      return {
        event_type: "item." + item.status,
        params: { item_id: String(item.id) },
        raw_payload: item,
      };
    },
  },
});
```

---

## Connector Config Options

Set via `connector_config` in `plugin.json` or the API:

| Field | Type | Default | Description |
|---|---|---|---|
| `poller_enabled` | boolean | `false` | Auto-provision poller on registration |
| `poll_interval_secs` | number | `60` | Polling interval in seconds |

---

## Managing Connectors

### Webhook URL

After registering a connector plugin, webhooks are received at:

```
POST https://plugins.juicyapi.com/webhooks/<provider>/<tenant>
```

For example: `POST /webhooks/stripe/acme`

### Poller Management

```bash
# List pollers
curl https://plugins.juicyapi.com/pollers

# Start a new poller (if not auto-provisioned)
curl -X POST https://plugins.juicyapi.com/pollers \
  -H "Content-Type: application/json" \
  -d '{
    "tenant": "acme",
    "provider": "generic",
    "plugin_id": "<plugin_id>",
    "interval_secs": 60
  }'

# Stop a poller
curl -X DELETE https://plugins.juicyapi.com/pollers/<poller_id>
```

### Backfill Management

```bash
# Start a backfill
curl -X POST https://plugins.juicyapi.com/backfills \
  -H "Content-Type: application/json" \
  -d '{
    "tenant": "acme",
    "provider": "generic",
    "plugin_id": "<plugin_id>"
  }'

# List backfills
curl https://plugins.juicyapi.com/backfills

# Cancel a backfill
curl -X DELETE https://plugins.juicyapi.com/backfills/<backfill_id>
```

---

## Built-in Provider Helpers

### Stripe

```typescript
export default defineConnector({
  name: "stripe",
  webhook: {
    verify(body, headers, secret, runtime) {
      return verifyStripeWebhook(body, headers, secret, runtime);
    },
    normalize(body) {
      return normalizeStripeWebhook(body);
    },
  },
});
```

Available globals: `verifyStripeWebhook`, `normalizeStripeWebhook`, `StripeAPI`, `StripeEvents`, `StripeTables`

### Shopify

```typescript
export default defineConnector({
  name: "shopify",
  webhook: {
    verify(body, headers, secret, runtime) {
      return verifyShopifyWebhook(body, headers, secret, runtime);
    },
    normalize(body, headers) {
      return normalizeShopifyWebhook(body, headers);
    },
  },
});
```

Available globals: `verifyShopifyWebhook`, `normalizeShopifyWebhook`, `ShopifyAPI`, `ShopifyTopics`, `ShopifyTables`

---

## Execution Model

Each connector lifecycle call runs in a **fresh V8 isolate**:

- `webhook.verify()` + `webhook.normalize()` — on `POST /webhooks/:provider/:tenant`
- `poller.fetch()` + `poller.normalize()` — on each poll interval tick
- `backfill.fetch()` + `backfill.normalize()` — on each page of historical data

All calls have access to the full `runtime` API and respect `allowed_domains`, `cpu_timeout_ms`, and `memory_limit_mb`.

---

## OAuth 401 Auto-Refresh

Pollers and backfills that use OAuth connectors get automatic token refresh. If an API returns 401:

1. Runtime fetches the `refresh_token` from the credential store
2. Calls the provider's token refresh endpoint
3. Updates stored tokens
4. **Retries the original request** with the new token

This is transparent to your plugin code — no 401 handling needed.

---

← [BitBin Analytics](./bitbin.md) · **Chapter 6** · [Examples & Patterns →](./examples.md)
