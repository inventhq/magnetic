---
title: Getting Started
order: 1
---

# Getting Started

> Build and deploy event-driven plugins in TypeScript that run inside secure V8 sandboxes.

## Prerequisites

- **Node.js** 18+
- A **tenant key_prefix** (e.g. `acme`, `6vct`) — your account identifier
- **Runtime URL** — `https://plugins.juicyapi.com` (production)

## Quick Start (CLI)

### 1. Install the CLI

```bash
npm install -g @inventhq/plugin-sdk
```

### 2. Scaffold a project

```bash
plugin init my-plugin
```

Interactive prompts will ask for plugin name, template, tenant, and runtime URL. Or skip prompts:

```bash
plugin init my-plugin --template echo --tenant acme --no-interactive
```

This creates:

```
my-plugin/
├── plugin.json              # Plugin metadata and config
├── plugin-runtime.d.ts      # SDK types (full autocomplete, no imports needed)
├── tsconfig.json
├── src/
│   └── plugin.ts            # Your plugin code
└── dist/                    # Build output (generated)
```

### 3. Write your plugin

Edit `src/plugin.ts`. All SDK functions are globals — no imports needed:

```typescript
export default definePlugin({
  name: "order-tracker",
  events: ["order.created", "order.updated"],

  async onEvent(event: Event, runtime: Runtime) {
    const rt = new RuntimeHelper(runtime);

    // Persistent state across invocations
    const count = parseInt(await rt.getStateOr("order_count", "0")) + 1;
    await rt.setState("order_count", String(count));

    // Emit a derived event back into the pipeline
    await rt.emit("order.tracked", {
      order_id: event.params.order_id,
      running_count: count,
    });

    rt.info("Tracked order #" + count);
  },
});
```

### 4. Validate, build, deploy

```bash
plugin validate              # Type-check
plugin build                 # Compile TS → JS
plugin deploy                # Register with runtime (saves plugin_id)
plugin deploy                # Subsequent deploys auto-update
plugin logs                  # View execution logs
plugin dev                   # Watch mode — rebuild + redeploy on save
```

**First deploy** registers a new plugin and saves the `plugin_id` to `plugin.json`.
**Subsequent deploys** detect the saved ID and update the existing plugin. Code versions are auto-incremented.

### 5. Verify

```bash
# View execution logs
plugin logs
plugin logs --follow         # Tail mode

# Or check runtime health
curl https://plugins.juicyapi.com/health
```

---

## Quick Start (API Only)

If you prefer not to use the CLI, you can manage plugins entirely via HTTP:

```bash
# Register a plugin
curl -X POST https://plugins.juicyapi.com/plugins \
  -H "Content-Type: application/json" \
  -d '{
    "key_prefix": "acme",
    "name": "order-tracker",
    "events": ["order.created", "order.updated"],
    "code": "export default definePlugin({ name: \"order-tracker\", events: [\"order.created\"], async onEvent(event, runtime) { runtime.log.info(\"Got: \" + event.event_type); } });"
  }'

# Update code (auto-versions)
curl -X PATCH https://plugins.juicyapi.com/plugins/<plugin_id> \
  -H "Content-Type: application/json" \
  -d '{"code": "export default definePlugin({ ... new code ... })"}'

# View logs
curl https://plugins.juicyapi.com/plugins/<plugin_id>/logs
```

---

## Plugin Types

There are two types of plugins:

### Event Handler (default)

Reacts to events flowing through the SSE pipeline. Use `definePlugin()`.

```typescript
export default definePlugin({
  name: "my-plugin",
  events: ["click", "purchase"],
  async onEvent(event, runtime) { /* ... */ },
});
```

### Connector

Proactively ingests data via webhooks, polling, and backfill. Use `defineConnector()`.

```typescript
export default defineConnector({
  name: "stripe-connector",
  webhook: { verify(...) {}, normalize(...) {} },
  poller:  { fetch(...) {}, normalize(...) {} },
  backfill: { fetch(...) {}, normalize(...) {} },
});
```

See [connectors.md](./connectors.md) for the full connector guide.

---

## Core Concepts

### Events

Everything in the platform is an event. Events have this shape:

```typescript
{
  event_id: "evt_abc123",       // Unique ID
  event_type: "order.created",  // Dot-separated type string
  timestamp: "2026-01-15T10:30:00Z",
  params: {                     // Key-value metadata (all values are strings)
    key_prefix: "acme",
    order_id: "ord_123",
    amount: "9900"
  },
  raw_payload: { /* original JSON body */ }
}
```

### Event Subscriptions

- **Specific types:** `events: ["click", "order.created"]` — only receives matching events
- **Wildcard:** `events: ["*"]` — receives all events for the tenant
- **Empty:** `events: []` — receives nothing (effectively paused)

### Sandbox Isolation

Each event execution runs in a **fresh V8 isolate** — no state leaks between invocations. Plugins can only interact with the outside world through the `runtime` API.

**Available:** `runtime.*` API, `console.log/warn/error`, standard JS built-ins (JSON, Math, String, Array, Promise, etc.)

**Not available:** `require()`, `import` from packages, `Deno.*`, filesystem access, raw network access, `eval()` on external code.

### Execution Limits

| Limit | Default | Configurable |
|---|---|---|
| CPU timeout | 5,000 ms | `cpu_timeout_ms` |
| Memory | 64 MB | `memory_limit_mb` |
| Emits per invocation | Unlimited | `max_emits_per_invocation` |
| Fetch timeout | 5 min | Per-request `timeout_ms` |
| Fetch response body | 1 MB | — |

Override via API:

```bash
curl -X PATCH https://plugins.juicyapi.com/plugins/<id> \
  -H "Content-Type: application/json" \
  -d '{"cpu_timeout_ms": 10000, "memory_limit_mb": 128}'
```

### Error Handling & Auto-Disable

- After **5 consecutive failures**, a plugin is automatically disabled
- `last_error` and `last_error_at` are stored on the plugin for debugging
- Re-enable: `PATCH /plugins/:id { "enabled": true }`

---

← [Introduction](./index.md) · **Chapter 1** · [Plugin Configuration →](./plugin-json.md)
