---
title: Plugin Runtime
order: 0
---

# Plugin Runtime Documentation

> Build event-driven plugins in TypeScript that run inside secure V8 sandboxes. No infrastructure to manage — just write code, deploy, and react to events.

---

## What is Plugin Runtime?

Plugin Runtime is a **serverless plugin execution engine**. It lets developers write small TypeScript programs (plugins) that react to real-time events — purchases, webhooks, form submissions, API calls — and take action: store data, call external APIs, emit new events, run analytics, and more.

**How it works:**

```
External API (Stripe, Shopify, etc.)
    │
    ▼
Webhook / Poller / Backfill ─── Connector Service
    │
    ▼
Event Pipeline (SSE) ─────────── Real-time stream
    │
    ▼
Your Plugin (V8 Sandbox) ─────── Executes your code
    │
    ├── Store data ─────────────── SQLite database (per-plugin, isolated)
    ├── Emit events ────────────── Trigger other plugins
    ├── Call APIs ───────────────── Outbound HTTP (domain-restricted)
    ├── Run analytics ──────────── BitBin (queries, vectors, documents)
    └── Log + State ────────────── Persistent KV state, structured logs
```

**Key properties:**

- **Sandboxed** — Each plugin runs in a fresh V8 isolate. No filesystem, no raw network, no cross-plugin leaks.
- **Multi-tenant** — Plugins are scoped to a tenant (`key_prefix`). Tables are auto-namespaced.
- **Event-driven** — Plugins subscribe to event types and fire on every match.
- **Two plugin types** — *Event handlers* react to events; *Connectors* actively pull data via webhooks, polling, and backfill.
- **Zero dependencies** — All SDK functions are globals. No `import`, no `require`, no `node_modules`.

---

## Who is this for?

- **Plugin developers** building integrations, automations, analytics, or AI agents on top of the event pipeline
- **AI coding agents** that generate plugin code programmatically (see [SDK Reference](/sdk-reference) for type-safe API documentation)

You do **not** need access to the Rust source code, the Kubernetes cluster, or any infrastructure. Everything is done through the SDK, CLI, and REST API.

---

## Documentation Map

Read these in order for a complete understanding, or jump to any chapter:

### Chapter 1: [Getting Started](/getting-started)
Install the CLI, scaffold your first project, write a plugin, deploy it, and view logs. **Start here** if you're new.

### Chapter 2: [Plugin Configuration](/plugin-json)
The `plugin.json` file — event subscriptions, domain allowlists, runtime config, connector settings. Everything that controls how your plugin behaves without changing code.

### Chapter 3: [SDK Reference](/sdk-reference)
Complete API documentation for `runtime.*`, `RuntimeHelper`, `defineTable`, `definePlugin`, `defineConnector`, and all global functions. The authoritative reference for every method, parameter, and return type.

### Chapter 4: [Database Guide](/database)
Create relational tables with `defineTable`, run SQL queries, perform joins, discover schemas, and handle sub-tenant data isolation. SQLite-compatible, auto-namespaced per plugin.

### Chapter 5: [BitBin Analytics](/bitbin)
Real-time analytics queries, high-throughput data ingestion, vector search (nearest-neighbor over embeddings), saved data pipelines, and a key-value document store — all via `runtime.bitbin`.

### Chapter 6: [Connectors](/connectors)
Build connectors that ingest data from external APIs. Webhook verification and normalization, scheduled polling with cursor-based pagination, historical backfill, OAuth auto-refresh, and built-in Stripe/Shopify helpers.

### Chapter 7: [Examples & Patterns](/examples)
11 complete, copy-pasteable plugin examples: echo, event counter, CRUD data store, event-driven pipeline, Slack notifier, AI/LLM agent, full Stripe connector, multi-table joins, sub-tenant scoped app, BitBin analytics dashboard, and semantic search.

### Chapter 8: [Management API](/management-api)
REST endpoint reference for managing plugins, connectors, pollers, backfills, schemas, named views, MCP server, OAuth, ingest tokens, and health checks. Every `curl` command you need.

---

## Quick Reference

| What | Where |
|---|---|
| **Production URL** | `https://plugins.juicyapi.com` |
| **Health check** | `GET /health` |
| **CLI install** | `npm install -g @inventhq/plugin-sdk` |
| **MCP endpoint** | `POST /mcp` (Streamable HTTP) |
| **AI context file** | `sdk/AI_CONTEXT.md` (machine-readable, all types) |

---

## 5-Minute Quickstart

```bash
# Install
npm install -g @inventhq/plugin-sdk

# Scaffold
plugin init my-plugin --template echo --tenant acme

# Deploy
cd my-plugin
plugin deploy

# Watch logs
plugin logs --follow
```

Your plugin is now live — receiving events and executing your code in a secure sandbox.

→ **Next:** [Getting Started](/getting-started)
