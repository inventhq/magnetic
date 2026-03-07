---
title: Plugin Configuration
order: 2
---

# Plugin Configuration (plugin.json)

> Schema and options for the `plugin.json` file that configures your plugin project.

## Full Schema

```json
{
  "name": "my-plugin",
  "entry": "src/plugin.ts",
  "events": ["order.created", "order.updated"],
  "tenant": "acme",
  "runtime_url": "https://plugins.juicyapi.com",
  "plugin_id": "019c5064-6bd4-7941-...",
  "allowed_domains": ["api.stripe.com", "hooks.slack.com"],
  "plugin_type": "event_handler",
  "config": {
    "api_key": "sk_test_...",
    "threshold": 100
  },
  "connector_config": {
    "poller_enabled": true,
    "poll_interval_secs": 60
  }
}
```

## Field Reference

| Field | Required | Type | Default | Description |
|---|---|---|---|---|
| `name` | Yes | string | — | Plugin name. Lowercase, alphanumeric, hyphens. |
| `entry` | Yes | string | `"src/plugin.ts"` | Path to the TypeScript source file. |
| `events` | Yes | string[] | — | Event types to subscribe to. `["*"]` = all events. |
| `tenant` | Yes | string | — | Your tenant key_prefix (e.g. `"acme"`, `"6vct"`). |
| `runtime_url` | Yes | string | — | Runtime base URL. Production: `https://plugins.juicyapi.com` |
| `plugin_id` | No | string | — | Auto-saved after first `plugin deploy`. Used for subsequent updates. |
| `allowed_domains` | No | string[] | `[]` | Hostnames your plugin can `fetch()`. Empty = all fetch blocked. |
| `plugin_type` | No | string | `"event_handler"` | `"event_handler"` or `"connector"`. |
| `config` | No | object | `{}` | Arbitrary JSON accessible via `runtime.getConfig()`. |
| `connector_config` | No | object | — | Connector-specific settings (poller, interval). |

---

## Event Subscriptions

```json
// Specific events
"events": ["order.created", "order.updated", "payment.succeeded"]

// All events for the tenant
"events": ["*"]

// No events (paused)
"events": []
```

---

## Allowed Domains

Controls which hostnames `runtime.fetch()` can reach. **Empty array = all fetch calls blocked** (secure default).

```json
"allowed_domains": ["api.stripe.com", "hooks.slack.com"]
```

**Matching rules:**
- **Exact match:** `"api.stripe.com"` allows `https://api.stripe.com/v1/charges`
- **Subdomain match:** `"api.stripe.com"` also allows `https://v1.api.stripe.com/...`
- **Scheme/port ignored** — only hostname is checked

**Common domain lists:**

| Provider | Domains |
|---|---|
| Stripe | `["api.stripe.com"]` |
| Shopify | `["YOUR_STORE.myshopify.com"]` |
| Slack | `["hooks.slack.com"]` |
| GitHub | `["api.github.com"]` |
| OpenAI | `["api.openai.com"]` |

---

## Plugin Config

Arbitrary JSON that your plugin reads via `runtime.getConfig()`. Useful for API keys, thresholds, feature flags — anything you want to change without redeploying code.

```json
"config": {
  "api_key": "sk_test_...",
  "threshold": 100,
  "notify_slack": true,
  "batch_size": 25
}
```

Config can also be updated via the API without creating a new code version:

```bash
curl -X PATCH https://plugins.juicyapi.com/plugins/<id> \
  -H "Content-Type: application/json" \
  -d '{"config": {"threshold": 200}}'
```

---

## Connector Config

Only used when `plugin_type` is `"connector"`.

| Field | Type | Default | Description |
|---|---|---|---|
| `poller_enabled` | boolean | `false` | Auto-provision a poller on registration |
| `poll_interval_secs` | number | `60` | Polling interval in seconds |

```json
"plugin_type": "connector",
"connector_config": {
  "poller_enabled": true,
  "poll_interval_secs": 120
}
```

---

## Environment Variable Override

Instead of setting `runtime_url` in `plugin.json`, you can use an environment variable:

```bash
export PLUGIN_RUNTIME_URL=https://plugins.juicyapi.com
plugin deploy
```

---

## Minimal Example

The simplest possible `plugin.json`:

```json
{
  "name": "echo",
  "entry": "src/plugin.ts",
  "events": ["*"],
  "tenant": "acme",
  "runtime_url": "https://plugins.juicyapi.com"
}
```

---

← [Getting Started](/getting-started) · **Chapter 2** · [SDK Reference →](/sdk-reference)
