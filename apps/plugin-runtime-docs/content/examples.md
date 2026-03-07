---
title: Examples & Patterns
order: 7
---

# Examples & Patterns

> Common plugin patterns with complete, copy-pasteable code.

## Table of Contents

- [Echo Plugin (Minimal)](#echo-plugin-minimal)
- [Event Counter with State](#event-counter-with-state)
- [CRUD Data Store](#crud-data-store)
- [Event-Driven Pipeline](#event-driven-pipeline)
- [Slack Notifier](#slack-notifier)
- [AI Agent (LLM-Powered)](#ai-agent-llm-powered)
- [Stripe Connector (Full)](#stripe-connector-full)
- [Multi-Table Joins](#multi-table-joins)
- [Sub-Tenant Scoped App](#sub-tenant-scoped-app)
- [BitBin Analytics Dashboard](#bitbin-analytics-dashboard)
- [Semantic Search](#semantic-search)

---

## Echo Plugin (Minimal)

The simplest plugin — mirrors every event as `echo_{type}`.

```typescript
export default definePlugin({
  name: "echo",
  events: ["*"],

  async onEvent(event: Event, runtime: Runtime) {
    await runtime.emit("echo_" + event.event_type, event.params);
    runtime.log.info("Echoed: " + event.event_id);
  },
});
```

---

## Event Counter with State

Track event counts using persistent key-value state.

```typescript
export default definePlugin({
  name: "event-counter",
  events: ["*"],

  async onEvent(event: Event, runtime: Runtime) {
    const rt = new RuntimeHelper(runtime);

    // Count per event type
    const key = "count_" + event.event_type;
    const count = parseInt(await rt.getStateOr(key, "0")) + 1;
    await rt.setState(key, String(count));

    // Count total
    const total = parseInt(await rt.getStateOr("total", "0")) + 1;
    await rt.setState("total", String(total));

    rt.info(`${event.event_type} #${count} (total: ${total})`);

    // Emit milestone events
    if (count % 100 === 0) {
      await rt.emit("milestone.reached", {
        event_type: event.event_type,
        count: count,
      });
    }
  },
});
```

---

## CRUD Data Store

A plugin that stores and manages data in its own relational tables.

```typescript
const contacts = defineTable("contacts", {
  id:         { type: "TEXT", primaryKey: true },
  email:      { type: "TEXT", notNull: true, index: true },
  name:       { type: "TEXT" },
  company:    { type: "TEXT", index: true },
  score:      { type: "INTEGER", default: "0" },
  created_at: { type: "TEXT" },
  updated_at: { type: "TEXT" },
});

const activities = defineTable("activities", {
  id:         { type: "TEXT", primaryKey: true },
  contact_id: { type: "TEXT", notNull: true, index: true },
  type:       { type: "TEXT", notNull: true },
  data:       { type: "TEXT" },
  created_at: { type: "TEXT" },
});

export default definePlugin({
  name: "crm-store",
  events: ["contact.created", "contact.updated", "contact.activity"],

  async onEvent(event: Event, runtime: Runtime) {
    const rt = new RuntimeHelper(runtime);
    await contacts.migrate(runtime);
    await activities.migrate(runtime);

    const payload = event.raw_payload as Record<string, unknown>;
    const now = new Date().toISOString();

    if (event.event_type === "contact.created") {
      await contacts.insert(runtime, {
        id: String(payload.id),
        email: String(payload.email),
        name: String(payload.name || ""),
        company: String(payload.company || ""),
        score: 0,
        created_at: now,
        updated_at: now,
      });
      rt.info("Created contact: " + payload.email);
    }

    if (event.event_type === "contact.updated") {
      await contacts.update(runtime,
        {
          name: String(payload.name || ""),
          company: String(payload.company || ""),
          updated_at: now,
        },
        "id = ?", [String(payload.id)]
      );
    }

    if (event.event_type === "contact.activity") {
      // Log the activity
      await activities.insert(runtime, {
        id: event.event_id,
        contact_id: String(payload.contact_id),
        type: String(payload.activity_type),
        data: JSON.stringify(payload.data || {}),
        created_at: now,
      });

      // Increment contact score
      await runtime.dbExec(
        "UPDATE contacts SET score = score + 1, updated_at = ? WHERE id = ?",
        [now, String(payload.contact_id)]
      );

      // Check for high-engagement contacts
      const contact = await contacts.findById(runtime, String(payload.contact_id));
      if (contact && Number(contact.score) >= 50) {
        await rt.emit("contact.high_engagement", {
          contact_id: String(payload.contact_id),
          email: String(contact.email),
          score: String(contact.score),
        });
      }
    }
  },
});
```

---

## Event-Driven Pipeline

Chain plugins together — one plugin's output triggers the next.

### Plugin 1: Order Processor

```typescript
export default definePlugin({
  name: "order-processor",
  events: ["order.created"],

  async onEvent(event: Event, runtime: Runtime) {
    const rt = new RuntimeHelper(runtime);
    const payload = event.raw_payload as Record<string, unknown>;
    const amount = Number(payload.amount) || 0;

    // Classify the order
    let tier = "standard";
    if (amount > 10000) tier = "premium";
    if (amount > 50000) tier = "enterprise";

    await rt.emit("order.classified", {
      order_id: event.params.order_id,
      amount: String(amount),
      tier: tier,
    });

    // Flag large orders for review
    if (amount > 25000) {
      await rt.emit("order.flagged", {
        order_id: event.params.order_id,
        amount: String(amount),
        reason: "high_value",
      });
    }
  },
});
```

### Plugin 2: Slack Notifier (reacts to derived events)

```typescript
export default definePlugin({
  name: "order-alerts",
  events: ["order.classified", "order.flagged"],

  async onEvent(event: Event, runtime: Runtime) {
    const rt = new RuntimeHelper(runtime);
    const config = rt.getConfig(); // { slack_webhook_url: "https://hooks.slack.com/..." }

    let message = "";
    if (event.event_type === "order.classified") {
      message = `New ${event.params.tier} order: $${(Number(event.params.amount) / 100).toFixed(2)}`;
    } else if (event.event_type === "order.flagged") {
      message = `⚠️ Flagged order ${event.params.order_id}: ${event.params.reason}`;
    }

    await runtime.fetch(config.slack_webhook_url as string, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text: message }),
    });

    rt.info("Sent Slack alert: " + message);
  },
});
```

**Event chain depth:** Plugins can trigger other plugins up to 5 levels deep. The runtime automatically tracks `_chain_depth` to prevent infinite loops.

---

## Slack Notifier

Generic Slack notification plugin — reacts to any event and posts to Slack.

```typescript
export default definePlugin({
  name: "slack-notifier",
  events: ["alert.*", "error.*"],

  async onEvent(event: Event, runtime: Runtime) {
    const rt = new RuntimeHelper(runtime);
    const config = rt.getConfig();
    const webhookUrl = rt.getConfigValue("slack_webhook_url", "");

    if (!webhookUrl) {
      rt.error("No slack_webhook_url configured");
      return;
    }

    const payload = {
      text: `*${event.event_type}*`,
      blocks: [
        {
          type: "section",
          text: {
            type: "mrkdwn",
            text: `*Event:* \`${event.event_type}\`\n*ID:* ${event.event_id}\n*Time:* ${event.timestamp}`,
          },
        },
        {
          type: "section",
          text: {
            type: "mrkdwn",
            text: "```" + JSON.stringify(event.params, null, 2) + "```",
          },
        },
      ],
    };

    await runtime.fetch(webhookUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    rt.info("Slack notification sent for " + event.event_type);
  },
});
```

**Setup:**
```bash
# Set the Slack webhook URL in plugin config
curl -X PATCH https://plugins.juicyapi.com/plugins/<id> \
  -H "Content-Type: application/json" \
  -d '{"config": {"slack_webhook_url": "https://hooks.slack.com/services/..."}, "allowed_domains": ["hooks.slack.com"]}'
```

---

## AI Agent (LLM-Powered)

A plugin that calls an LLM to make decisions based on event data.

```typescript
const reviews = defineTable("reviews", {
  id:       { type: "TEXT", primaryKey: true },
  order_id: { type: "TEXT", index: true },
  decision: { type: "TEXT" },
  reason:   { type: "TEXT" },
  reviewed_at: { type: "TEXT" },
});

export default definePlugin({
  name: "fraud-reviewer",
  events: ["order.flagged"],

  async onEvent(event: Event, runtime: Runtime) {
    const rt = new RuntimeHelper(runtime);
    await reviews.migrate(runtime);

    const config = rt.getConfig();
    const orderId = event.params.order_id;

    // Gather context from DB
    const orderData = await runtime.dbQuery(
      "SELECT * FROM orders WHERE id = ?", [orderId]
    );

    // Ask LLM for a decision
    const resp = await runtime.fetch("https://api.openai.com/v1/chat/completions", {
      method: "POST",
      headers: {
        "Authorization": "Bearer " + config.openai_key,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: config.model || "gpt-4o-mini",
        messages: [
          {
            role: "system",
            content: "Analyze this order for fraud risk. Respond with JSON: { \"decision\": \"approve\" | \"reject\" | \"escalate\", \"reason\": \"string\" }",
          },
          {
            role: "user",
            content: JSON.stringify({
              order_id: orderId,
              amount: event.params.amount,
              reason_flagged: event.params.reason,
              order_data: orderData.rows[0] || null,
            }),
          },
        ],
        response_format: { type: "json_object" },
      }),
    });

    const completion = JSON.parse(resp.body);
    const decision = JSON.parse(completion.choices[0].message.content);

    // Store the review
    await reviews.upsert(runtime, {
      id: event.event_id,
      order_id: orderId,
      decision: decision.decision,
      reason: decision.reason,
      reviewed_at: new Date().toISOString(),
    });

    // Emit decision event
    await rt.emit("order." + decision.decision + "d", {
      order_id: orderId,
      reason: decision.reason,
    });

    rt.info(`Order ${orderId}: ${decision.decision} — ${decision.reason}`);
  },
});
```

**Setup:**
```json
{
  "allowed_domains": ["api.openai.com"],
  "config": { "openai_key": "sk-...", "model": "gpt-4o-mini" }
}
```

---

## Stripe Connector (Full)

Complete Stripe connector with webhooks, polling, database, and SDK helpers.

```typescript
const chargesTable = defineTable("stripe_charges", StripeTables.charges);
const customersTable = defineTable("stripe_customers", StripeTables.customers);

export default defineConnector({
  name: "stripe-sync",
  tables: [chargesTable, customersTable],

  webhook: {
    verify(body, headers, secret, runtime) {
      return verifyStripeWebhook(body, headers, secret, runtime);
    },
    normalize(body) {
      return normalizeStripeWebhook(body);
    },
  },

  poller: {
    async fetch(cursor, runtime) {
      const config = runtime.getConfig();
      await chargesTable.migrate(runtime);
      await customersTable.migrate(runtime);

      // Use Stripe SDK helper
      const stripe = StripeAPI(runtime, config.stripe_api_key as string);
      const params = cursor
        ? { starting_after: cursor, limit: 100 }
        : { limit: 100 };

      const result = await stripe.charges.list(params);

      // Persist charges to local DB
      for (const ch of result.data) {
        await chargesTable.upsert(runtime, {
          id: ch.id,
          amount: ch.amount,
          currency: ch.currency,
          customer_id: ch.customer || "",
          status: ch.status,
          description: ch.description || "",
          payment_method: ch.payment_method || "",
          receipt_url: ch.receipt_url || "",
          livemode: ch.livemode ? 1 : 0,
          created_at: ch.created,
          raw_json: JSON.stringify(ch),
        });
      }

      return {
        items: result.data,
        nextCursor: result.hasMore && result.data.length > 0
          ? result.data[result.data.length - 1].id
          : null,
        hasMore: result.hasMore || false,
      };
    },

    normalize(charge) {
      return {
        event_type: "stripe.charge." + (charge.status || "unknown"),
        params: {
          charge_id: String(charge.id),
          amount: String(charge.amount),
          currency: String(charge.currency),
          customer: String(charge.customer || ""),
        },
        raw_payload: charge,
      };
    },
  },
});
```

**Setup:**
```json
{
  "plugin_type": "connector",
  "allowed_domains": ["api.stripe.com"],
  "config": { "stripe_api_key": "sk_test_..." },
  "connector_config": { "poller_enabled": true, "poll_interval_secs": 300 }
}
```

---

## Multi-Table Joins

Store related data across tables and use SQL JOINs for complex queries.

```typescript
const customers = defineTable("customers", {
  id:    { type: "TEXT", primaryKey: true },
  email: { type: "TEXT", notNull: true, index: true },
  name:  { type: "TEXT" },
  tier:  { type: "TEXT", default: "'free'" },
});

const orders = defineTable("orders", {
  id:          { type: "TEXT", primaryKey: true },
  customer_id: { type: "TEXT", notNull: true, index: true },
  total:       { type: "REAL" },
  status:      { type: "TEXT", index: true },
  created_at:  { type: "TEXT" },
});

const line_items = defineTable("line_items", {
  id:         { type: "TEXT", primaryKey: true },
  order_id:   { type: "TEXT", notNull: true, index: true },
  product:    { type: "TEXT" },
  quantity:   { type: "INTEGER" },
  unit_price: { type: "REAL" },
});

export default definePlugin({
  name: "order-analytics",
  events: ["order.completed"],

  async onEvent(event: Event, runtime: Runtime) {
    const rt = new RuntimeHelper(runtime);
    await customers.migrate(runtime);
    await orders.migrate(runtime);
    await line_items.migrate(runtime);

    // ... (insert data from event) ...

    // Three-table JOIN: top products by customer tier
    const tierProducts = await runtime.dbQuery(
      `SELECT c.tier, li.product, SUM(li.quantity) as total_sold, SUM(li.unit_price * li.quantity) as revenue
       FROM line_items li
       JOIN orders o ON li.order_id = o.id
       JOIN customers c ON o.customer_id = c.id
       WHERE o.status = ?
       GROUP BY c.tier, li.product
       ORDER BY revenue DESC
       LIMIT 20`,
      ["completed"]
    );

    for (const row of tierProducts.rows) {
      rt.info(`Tier: ${row[0]}, Product: ${row[1]}, Sold: ${row[2]}, Revenue: $${Number(row[3]).toFixed(2)}`);
    }

    // Customer lifetime value
    const ltv = await runtime.dbQuery(
      `SELECT c.email, c.name, c.tier,
              COUNT(o.id) as order_count,
              SUM(o.total) as lifetime_value
       FROM customers c
       LEFT JOIN orders o ON c.id = o.customer_id AND o.status = 'completed'
       GROUP BY c.id
       HAVING lifetime_value > ?
       ORDER BY lifetime_value DESC`,
      [1000]
    );

    // Emit insights
    for (const row of ltv.rows) {
      if (Number(row[4]) > 5000 && row[2] === "free") {
        await rt.emit("customer.upgrade_candidate", {
          email: String(row[0]),
          name: String(row[1]),
          lifetime_value: String(row[4]),
        });
      }
    }
  },
});
```

---

## Sub-Tenant Scoped App

A multi-user app where each end-user sees only their own data.

```typescript
const notes = defineTable("notes", {
  id:         { type: "TEXT", primaryKey: true },
  sub_id:     { type: "TEXT", notNull: true, index: true },
  title:      { type: "TEXT" },
  content:    { type: "TEXT" },
  created_at: { type: "TEXT" },
  updated_at: { type: "TEXT" },
});

export default definePlugin({
  name: "notes-app",
  events: ["note.create", "note.update", "note.delete", "note.list"],

  async onEvent(event: Event, runtime: Runtime) {
    const rt = new RuntimeHelper(runtime);
    await notes.migrate(runtime);

    const subId = runtime.context.sub_id;
    if (!subId) {
      rt.error("No sub_id in event — cannot scope to user");
      return;
    }

    const payload = event.raw_payload as Record<string, unknown>;
    const now = new Date().toISOString();

    switch (event.event_type) {
      case "note.create":
        await notes.insert(runtime, {
          id: event.event_id,
          sub_id: subId,
          title: String(payload.title || "Untitled"),
          content: String(payload.content || ""),
          created_at: now,
          updated_at: now,
        });
        await rt.emit("note.created", { note_id: event.event_id, sub_id: subId });
        break;

      case "note.update":
        await notes.update(runtime,
          { title: String(payload.title), content: String(payload.content), updated_at: now },
          "id = ? AND sub_id = ?",
          [String(payload.note_id), subId]
        );
        break;

      case "note.delete":
        await notes.del(runtime, "id = ? AND sub_id = ?", [String(payload.note_id), subId]);
        break;

      case "note.list":
        const userNotes = await notes.findAll(runtime, {
          where: "sub_id = ?",
          params: [subId],
          orderBy: "updated_at DESC",
          limit: 50,
        });
        rt.info(`User ${subId} has ${userNotes.length} notes`);
        break;
    }
  },
});
```

---

## BitBin Analytics Dashboard

Use BitBin for real-time analytics alongside relational data.

```typescript
const charges = defineTable("charges", {
  id:       { type: "TEXT", primaryKey: true },
  amount:   { type: "INTEGER" },
  currency: { type: "TEXT" },
  status:   { type: "TEXT", index: true },
});

export default definePlugin({
  name: "payment-dashboard",
  events: ["charge.succeeded", "charge.refunded"],

  async onEvent(event: Event, runtime: Runtime) {
    const rt = new RuntimeHelper(runtime);
    await charges.migrate(runtime);

    const payload = event.raw_payload as Record<string, unknown>;
    const amount = Number(payload.amount) || 0;

    // Store in relational DB
    await charges.upsert(runtime, {
      id: String(payload.id),
      amount: amount,
      currency: String(payload.currency || "usd"),
      status: event.event_type === "charge.succeeded" ? "succeeded" : "refunded",
    });

    // Ingest into BitBin for analytics
    await runtime.bitbin.ingest({
      records: [{
        tenant: 1,
        entity: 10,
        key_id: Date.now(),
        amount: amount,
        status: event.event_type === "charge.succeeded" ? 1 : 2,
      }],
    });

    // Query real-time totals
    const totals = await runtime.bitbin.query({
      space: { tenant: 1, entity: 10 },
      measure: { type: "sum" },
      of: "amount",
    });

    // Query by status breakdown
    const byStatus = await runtime.bitbin.query({
      space: { tenant: 1, entity: 10 },
      measure: { type: "group" },
      of: "status",
    });

    rt.info(`Running total: $${((totals.sum || 0) / 100).toFixed(2)}`);
    for (const g of byStatus.groups || []) {
      rt.info(`  Status ${g.key}: ${g.count} charges, $${(g.sum / 100).toFixed(2)}`);
    }

    // Store summary as a document
    await runtime.bitbin.doc.set({
      tenant: 1,
      entity: 10,
      doc_id: "dashboard_summary",
      data: {
        total_revenue: totals.sum || 0,
        breakdown: byStatus.groups || [],
        updated_at: new Date().toISOString(),
      },
    });
  },
});
```

---

## Semantic Search

Combine BitBin vector search with OpenAI embeddings for semantic document search.

```typescript
const documents = defineTable("documents", {
  id:      { type: "TEXT", primaryKey: true },
  title:   { type: "TEXT" },
  content: { type: "TEXT" },
});

export default definePlugin({
  name: "semantic-search",
  events: ["doc.indexed", "search.query"],

  async onEvent(event: Event, runtime: Runtime) {
    const rt = new RuntimeHelper(runtime);
    await documents.migrate(runtime);

    const config = rt.getConfig();
    const openaiKey = rt.getConfigValue("openai_key", "");

    if (event.event_type === "doc.indexed") {
      const payload = event.raw_payload as Record<string, unknown>;

      // Store document
      await documents.upsert(runtime, {
        id: String(payload.id),
        title: String(payload.title),
        content: String(payload.content),
      });

      // Generate embedding
      const embResp = await rt.fetchJSON("https://api.openai.com/v1/embeddings", {
        method: "POST",
        headers: {
          "Authorization": "Bearer " + openaiKey,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model: "text-embedding-3-small",
          input: String(payload.title) + ": " + String(payload.content),
        }),
      });

      // Store embedding in BitBin (vector search index)
      // The vector_id should match a numeric ID you can map back to the document
      rt.info("Indexed document: " + payload.title);
    }

    if (event.event_type === "search.query") {
      const query = event.params.query;

      // Generate query embedding
      const embResp = await rt.fetchJSON("https://api.openai.com/v1/embeddings", {
        method: "POST",
        headers: {
          "Authorization": "Bearer " + openaiKey,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model: "text-embedding-3-small",
          input: query,
        }),
      });

      // Search BitBin
      const matches = await runtime.bitbin.vectorSearch({
        tenant: 1,
        embedding: embResp.data[0].embedding,
        top_k: 5,
      });

      rt.info(`Found ${matches.length} matches for: "${query}"`);
      for (const match of matches) {
        rt.info(`  vector_id=${match.vector_id}, distance=${match.distance.toFixed(4)}`);
      }

      await rt.emit("search.results", {
        query: query,
        match_count: String(matches.length),
      });
    }
  },
});
```

**Setup:**
```json
{
  "allowed_domains": ["api.openai.com"],
  "config": { "openai_key": "sk-..." }
}
```

---

← [Connectors](./connectors.md) · **Chapter 7** · [Management API →](./management-api.md)
