---
title: Management API
order: 8
---

# Management API Reference

> REST endpoints for managing plugins, connectors, pollers, backfills, and more.

**Base URL:** `https://plugins.juicyapi.com`

---

## Plugins

### Create a plugin

```
POST /plugins
```

```bash
curl -X POST https://plugins.juicyapi.com/plugins \
  -H "Content-Type: application/json" \
  -d '{
    "key_prefix": "acme",
    "name": "order-tracker",
    "events": ["order.created"],
    "code": "export default definePlugin({ ... })",
    "allowed_domains": ["api.stripe.com"],
    "config": {"threshold": 100},
    "plugin_type": "event_handler"
  }'
```

| Field | Required | Type | Description |
|---|---|---|---|
| `key_prefix` | Yes | string | Tenant identifier |
| `name` | Yes | string | Plugin name |
| `events` | Yes | string[] | Event subscriptions (`["*"]` = all) |
| `code` | Yes* | string | JavaScript source code |
| `template` | No | string | Use a built-in template instead of `code` |
| `allowed_domains` | No | string[] | Fetch domain allowlist |
| `config` | No | object | Plugin config JSON |
| `plugin_type` | No | string | `"event_handler"` (default) or `"connector"` |
| `connector_config` | No | string | JSON string with connector options |

*Either `code` or `template` is required.

**Response:**
```json
{
  "id": "019c5064-6bd4-7941-...",
  "key_prefix": "acme",
  "name": "order-tracker",
  "status": "registered"
}
```

### Create from template

```bash
curl -X POST https://plugins.juicyapi.com/plugins \
  -H "Content-Type: application/json" \
  -d '{"key_prefix": "acme", "template": "echo"}'
```

### List plugins

```
GET /plugins?key_prefix=acme
```

```bash
# All plugins
curl https://plugins.juicyapi.com/plugins

# Filter by tenant
curl "https://plugins.juicyapi.com/plugins?key_prefix=acme"
```

### Get plugin details

```
GET /plugins/:id
```

Returns full plugin info including code, config, limits, error state, and version count.

### Update a plugin

```
PATCH /plugins/:id
```

All fields are optional. Updating `code` creates a new version.

```bash
# Update code (auto-versions)
curl -X PATCH https://plugins.juicyapi.com/plugins/<id> \
  -H "Content-Type: application/json" \
  -d '{"code": "export default definePlugin({ ... })"}'

# Update config (no new version)
curl -X PATCH https://plugins.juicyapi.com/plugins/<id> \
  -H "Content-Type: application/json" \
  -d '{"config": {"threshold": 200}}'

# Update event subscriptions
curl -X PATCH https://plugins.juicyapi.com/plugins/<id> \
  -H "Content-Type: application/json" \
  -d '{"events": ["order.*", "payment.*"]}'

# Update execution limits
curl -X PATCH https://plugins.juicyapi.com/plugins/<id> \
  -H "Content-Type: application/json" \
  -d '{"cpu_timeout_ms": 10000, "memory_limit_mb": 128}'

# Update allowed domains
curl -X PATCH https://plugins.juicyapi.com/plugins/<id> \
  -H "Content-Type: application/json" \
  -d '{"allowed_domains": ["api.stripe.com", "hooks.slack.com"]}'

# Disable / enable
curl -X PATCH https://plugins.juicyapi.com/plugins/<id> \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'
```

### Delete a plugin

```
DELETE /plugins/:id
```

Removes the plugin, all versions, and stops any associated pollers.

### View logs

```
GET /plugins/:id/logs?limit=50
```

```json
{
  "plugin_id": "019c5064-...",
  "logs": [
    {"timestamp": "2026-03-06T21:00:00Z", "level": "info", "message": "Processed order #42"}
  ],
  "count": 1
}
```

### List versions

```
GET /plugins/:id/versions
```

### Get specific version

```
GET /plugins/:id/versions/:version_number
```

Returns version metadata and full code.

### Rollback

```
POST /plugins/:id/rollback
```

```bash
curl -X POST https://plugins.juicyapi.com/plugins/<id>/rollback \
  -H "Content-Type: application/json" \
  -d '{"version": 1}'
```

Creates a new version entry with the code from the specified version.

---

## Templates

### List templates

```
GET /templates
```

| Template | Type | Description |
|---|---|---|
| `echo` | event_handler | Mirrors events as `echo_{type}` |
| `stripe-webhook` | event_handler | Parses Stripe webhook payloads |
| `shopify-order` | event_handler | Parses Shopify order webhooks |
| `enrichment` | event_handler | Queries historical data, emits enriched events |
| `aggregator` | event_handler | Counts events using state, emits summaries |
| `stripe-connector` | connector | Full Stripe connector with webhooks + polling + DB |

---

## Webhooks

### Receive a webhook

```
POST /webhooks/:provider/:tenant
```

The runtime looks for a connector plugin registered for the provider and tenant. If found, it calls `webhook.verify()` then `webhook.normalize()`. If no connector plugin exists, it falls back to built-in provider logic (Stripe, Shopify, Generic).

```bash
# Generic webhook
curl -X POST https://plugins.juicyapi.com/webhooks/generic/acme \
  -H "Content-Type: application/json" \
  -H "X-Event-Type: order.created" \
  -d '{"id": "ord_123", "amount": 9900}'

# Stripe webhook
curl -X POST https://plugins.juicyapi.com/webhooks/stripe/acme \
  -H "Content-Type: application/json" \
  -H "Stripe-Signature: t=...,v1=..." \
  -d '{"type": "payment_intent.succeeded", ...}'
```

---

## Connectors

### Register a connector

```
POST /connectors
```

```bash
curl -X POST https://plugins.juicyapi.com/connectors \
  -H "Content-Type: application/json" \
  -d '{
    "tenant": "acme",
    "provider": "stripe",
    "credential_type": "oauth",
    "data": {
      "client_id": "ca_xxx",
      "client_secret": "sk_xxx",
      "webhook_secret": "whsec_xxx"
    }
  }'
```

### List connectors

```
GET /connectors?tenant=acme
```

### Delete a connector

```
DELETE /connectors/:id
```

---

## OAuth

### Start OAuth flow

```
GET /auth/:provider?tenant=acme
```

Redirects to the provider's authorization page.

### OAuth callback

```
GET /auth/:provider/callback?code=...&state=...
```

Handled automatically. Stores tokens in the encrypted credential store.

---

## Pollers

### Start a poller

```
POST /pollers
```

```bash
curl -X POST https://plugins.juicyapi.com/pollers \
  -H "Content-Type: application/json" \
  -d '{
    "tenant": "acme",
    "provider": "generic",
    "plugin_id": "<plugin_id>",
    "url": "https://api.example.com/events",
    "interval_secs": 60,
    "cursor_field": "id"
  }'
```

### List pollers

```
GET /pollers
```

### Stop a poller

```
DELETE /pollers/:id
```

---

## Backfills

### Start a backfill

```
POST /backfills
```

```bash
curl -X POST https://plugins.juicyapi.com/backfills \
  -H "Content-Type: application/json" \
  -d '{
    "tenant": "acme",
    "provider": "generic",
    "plugin_id": "<plugin_id>",
    "url": "https://api.example.com/historical",
    "cursor_field": "id"
  }'
```

### List backfills

```
GET /backfills
```

### Cancel a backfill

```
DELETE /backfills/:id
```

---

## Schema Discovery

Requires admin API key (`Authorization: Bearer $PLATFORM_API_KEY`).

### Discover tables

```
GET /schemas/:key_prefix
```

```bash
curl -H "Authorization: Bearer $API_KEY" \
  https://plugins.juicyapi.com/schemas/acme
```

Response:
```json
{
  "key_prefix": "acme",
  "plugins": [{
    "plugin_id": "019c5064-...",
    "plugin_short_id": "019c5064",
    "tables": [{
      "name": "orders",
      "full_name": "acme_019c5064_orders",
      "columns": [
        {"name": "id", "type": "TEXT", "primary_key": true},
        {"name": "total", "type": "REAL", "primary_key": false}
      ]
    }]
  }]
}
```

### Cross-plugin query

```
POST /schemas/:key_prefix/query
```

```bash
curl -X POST https://plugins.juicyapi.com/schemas/acme/query \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"sql": "SELECT * FROM acme_019c5064_orders WHERE total > 100", "params": []}'
```

Read-only. All table references must be scoped to the tenant prefix.

---

## Named Views

Saved SQL queries. Requires admin API key.

### Create a view

```
POST /views/:key_prefix
```

```bash
curl -X POST https://plugins.juicyapi.com/views/acme \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "top_customers",
    "description": "Customers by lifetime value",
    "sql": "SELECT c.email, SUM(o.total) as ltv FROM acme_019c_customers c JOIN acme_019c_orders o ON c.id = o.customer_id GROUP BY c.id ORDER BY ltv DESC",
    "default_params": []
  }'
```

### List views

```
GET /views/:key_prefix
```

### Get a view

```
GET /views/:key_prefix/:name
```

### Execute a view

```
GET /views/:key_prefix/:name/execute
GET /views/:key_prefix/:name/execute?params=[100]
```

Response:
```json
{
  "view": "top_customers",
  "columns": ["email", "ltv"],
  "rows": [["alice@example.com", 4500.00]],
  "row_count": 1
}
```

### Update a view

```
PATCH /views/:key_prefix/:name
```

### Delete a view

```
DELETE /views/:key_prefix/:name
```

---

## MCP Server

Model Context Protocol endpoint for AI agents.

```
POST /mcp
GET  /mcp
```

Protocol: Streamable HTTP transport, version 2025-03-26.

### Available tools

| Tool | Description |
|---|---|
| `discover_schema` | List all plugin DB tables for a tenant |
| `query` | Run read-only SQL against tenant's plugin DB |
| `list_views` | List named views for a tenant |
| `execute_view` | Execute a named view |
| `emit_event` | Emit an event into the pipeline |

### Client configuration

```json
{
  "mcpServers": {
    "plugin-runtime": {
      "transport": "streamable-http",
      "url": "https://plugins.juicyapi.com/mcp"
    }
  }
}
```

---

## Ingest Tokens

### Set token for a tenant

```
POST /ingest-tokens
```

```bash
curl -X POST https://plugins.juicyapi.com/ingest-tokens \
  -H "Content-Type: application/json" \
  -d '{"key_prefix": "acme", "token": "pt_acme_..."}'
```

### Check token status

```
GET /ingest-tokens?key_prefix=acme
```

Tokens are auto-issued on plugin registration if `PLATFORM_API_KEY` is configured.

---

## Health Check

```
GET /health
```

```json
{
  "status": "ok",
  "ingest_url": "https://track.juicyapi.com/ingest",
  "circuit_breaker": "closed",
  "events_sent": 42,
  "events_failed": 0,
  "providers": ["stripe", "shopify", "generic"],
  "backpressure_buffered": 0,
  "active_sse_tenants": ["acme"],
  "plugins_with_logs": 3
}
```

---

← [Examples & Patterns](/examples) · **Chapter 8** · [Errors & Troubleshooting →](/errors)
