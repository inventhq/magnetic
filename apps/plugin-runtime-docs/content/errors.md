---
title: Errors & Troubleshooting
order: 9
---

# Errors & Troubleshooting

> How the runtime reports errors, what to check when things go wrong, and common fixes.

---

## Error Visibility

The Plugin Runtime surfaces errors through multiple channels:

| Channel | What you see | How to access |
|---|---|---|
| **JS exception** | Thrown in your `onEvent` / connector handler | `try/catch` in your plugin code |
| **Plugin logs** | `runtime.log.*` and `console.*` output | `GET /plugins/:id/logs` |
| **Execution result** | Error message + logs from test runs | `POST /plugins/:id/execute` response body |
| **Plugin record** | `last_error`, `last_error_at`, `consecutive_failures` | `GET /plugins/:id` |
| **Health endpoint** | System-wide counters, circuit breaker state | `GET /health` |

### Checking plugin status

```bash
# View plugin details including error state
curl https://plugins.juicyapi.com/plugins/YOUR_PLUGIN_ID

# View recent logs
curl https://plugins.juicyapi.com/plugins/YOUR_PLUGIN_ID/logs?limit=50

# Test execution (returns error + logs in response)
curl -X POST https://plugins.juicyapi.com/plugins/YOUR_PLUGIN_ID/execute \
  -H "Content-Type: application/json" \
  -d '{"event_type":"test.ping","params":{}}'
```

---

## Auto-Disable

If a plugin fails **5 consecutive times**, the runtime automatically disables it to protect system stability.

**What happens:**
1. Each failure increments `consecutive_failures` and stores `last_error` with recent log context
2. Each success resets `consecutive_failures` to 0
3. At 5 consecutive failures → `enabled` is set to `false`

**How to recover:**
```bash
# Check what went wrong
curl https://plugins.juicyapi.com/plugins/YOUR_PLUGIN_ID
# Look at last_error — it includes the error message plus recent warn/error logs

# Fix the issue in your code, then re-enable
curl -X PATCH https://plugins.juicyapi.com/plugins/YOUR_PLUGIN_ID \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
# This also resets the failure counter
```

---

## Common Errors & Fixes

### Plugin has no onEvent handler

```
Error: Plugin has no onEvent handler or callable default export.
Exported keys: [...].
Make sure your plugin exports { onEvent } or a default function.
```

**Cause:** Your plugin doesn't export an `onEvent` function or a callable default export.

**Fix:** Make sure you have one of these patterns:

```typescript
// Pattern A: Named export (recommended)
export default {
  async onEvent(event, runtime) {
    // ...
  }
};

// Pattern B: Default function
export default async function(event, runtime) {
  // ...
};
```

**Common mistake:** Exporting a class or object without `onEvent`:
```typescript
// ❌ This will fail — no onEvent
export default { setup() {} };

// ✅ This works
export default { onEvent(event, runtime) { /* ... */ } };
```

---

### Emit limit reached

```
Error: Emit limit reached: plugin has already emitted N events this invocation (max: 10).
Increase max_emits_per_invocation via PATCH /plugins/:id if needed.
```

**Cause:** Your plugin called `runtime.emit()` more times than the configured `max_emits_per_invocation` (default: 10).

**Fix:**
- If your plugin legitimately needs to emit many events, increase the limit:
  ```bash
  curl -X PATCH https://plugins.juicyapi.com/plugins/YOUR_PLUGIN_ID \
    -H "Content-Type: application/json" \
    -d '{"max_emits_per_invocation": 50}'
  ```
- If this is unexpected, you may have an accidental loop in your code

---

### Chain depth limit (infinite emit loop)

When plugins emit events that trigger other plugins (or the same plugin), the runtime tracks the chain depth. At the maximum depth (default: 5), execution is blocked to prevent infinite loops.

**Symptoms:** Events silently stop propagating after several hops.

**Fix:** Audit your event flow. Common pattern that causes loops:
```typescript
// ❌ Plugin A listens for "order.created", emits "order.processed"
// ❌ Plugin B listens for "order.processed", emits "order.created"
// → Infinite loop!
```

Use distinct event types and check `event.params._chain_depth` if needed.

---

### fetch() blocked: no allowed_domains

```
Error: fetch() blocked: no allowed_domains configured for this plugin.
Set allowed_domains via PATCH /plugins/:id
```

**Cause:** Your plugin calls `runtime.fetch()` but no domains are allowed.

**Fix:**
```bash
curl -X PATCH https://plugins.juicyapi.com/plugins/YOUR_PLUGIN_ID \
  -H "Content-Type: application/json" \
  -d '{"allowed_domains": ["api.stripe.com", "hooks.slack.com"]}'
```

---

### fetch() blocked: domain not in allowed_domains

```
Error: fetch() blocked: domain 'api.example.com' not in allowed_domains ["api.stripe.com"]
```

**Cause:** You're trying to reach a domain not in your allowlist.

**Fix:** Add the domain to `allowed_domains` via `PATCH /plugins/:id`.

---

### Plugin database not configured

```
Error: Plugin database not configured (TURSO_PLUGIN_URL not set)
```

**Cause:** `runtime.dbExec()`, `runtime.dbQuery()`, or `defineTable()` was called, but the backend has no plugin database configured.

**Fix:** This is an infrastructure issue — the `TURSO_PLUGIN_URL` or `DO_SQLITE_URL` environment variable must be set on the runtime. Contact your platform administrator.

---

### BitBin not configured

```
Error: BitBin not configured (BITBIN_BASE_URL not set)
```

**Cause:** `runtime.bitbin.*` was called, but BitBin is not configured on this runtime instance.

**Fix:** Infrastructure issue — `BITBIN_BASE_URL` and `BITBIN_API_KEY` must be set. Contact your platform administrator.

---

### Circuit breaker open

```
Error: Circuit breaker is open — ingest endpoint appears down after N consecutive failures.
Will retry automatically in ~Xs. Check INGEST_URL configuration and downstream health.
```

**Cause:** The event ingest endpoint has been unreachable. The runtime's circuit breaker prevents additional requests to avoid cascading failures.

**What happens:**
- `runtime.emit()` calls will fail with this error
- The circuit breaker automatically retries after a recovery timeout (~30s)
- Once a successful request goes through, the circuit resets

**Fix:** Usually a transient infrastructure issue. If persistent, check:
1. The `INGEST_URL` environment variable is correct
2. The event core / SSE gateway is healthy
3. Network connectivity between the runtime and ingest endpoint

---

### Webhook secret is empty

```
Warning: Webhook secret is empty — signature verification may be unreliable.
Set a webhook secret via POST /connectors/:provider/webhook-secret
```

**Cause:** Your connector received a webhook but no webhook secret is configured, so signature verification may not work properly.

**Fix:**
```bash
curl -X POST https://plugins.juicyapi.com/connectors/YOUR_PROVIDER/webhook-secret \
  -H "Content-Type: application/json" \
  -H "X-Tenant-Id: YOUR_TENANT" \
  -d '{"secret": "whsec_..."}'
```

---

### Plugin execution timed out

```
Error: Plugin execution timed out after 5000ms
```

**Cause:** Your plugin took longer than the CPU timeout to execute.

**Fix:**
- Optimize your code (reduce API calls, use smaller queries)
- Increase the timeout if the workload legitimately takes longer:
  ```bash
  curl -X PATCH https://plugins.juicyapi.com/plugins/YOUR_PLUGIN_ID \
    -H "Content-Type: application/json" \
    -d '{"cpu_timeout_ms": 15000}'
  ```
- Maximum allowed: 30000ms (30 seconds)

---

### getConfig() returns empty object

If `runtime.getConfig()` returns `{}` unexpectedly, check your plugin logs — a warning will appear if the config JSON failed to parse.

**Fix:** Verify your config is valid JSON:
```bash
curl -X PATCH https://plugins.juicyapi.com/plugins/YOUR_PLUGIN_ID \
  -H "Content-Type: application/json" \
  -d '{"config": "{\"api_key\": \"sk_test_...\", \"threshold\": 100}"}'
```

Note: The `config` value is a **JSON string inside JSON**, so it must be escaped.

---

### SQL validation errors

```
Error: SQL blocked: only SELECT, INSERT, UPDATE, DELETE allowed
Error: SQL blocked: DROP TABLE not permitted
```

**Cause:** The runtime validates all SQL statements and blocks DDL operations (CREATE TABLE, DROP, ALTER) from plugin code. Table creation is handled via `defineTable()`.

**Fix:** Use `defineTable()` for schema management and only use `runtime.dbExec()` / `runtime.dbQuery()` for DML operations (SELECT, INSERT, UPDATE, DELETE).

---

## Debugging Checklist

When a plugin isn't working as expected:

1. **Check if it's enabled:** `GET /plugins/:id` → look at `enabled` field
2. **Check for errors:** Look at `last_error` and `consecutive_failures`
3. **Check logs:** `GET /plugins/:id/logs?limit=100`
4. **Test manually:** `POST /plugins/:id/execute` with a test event
5. **Check subscriptions:** Does `event_subscriptions` match the events you expect?
6. **Check health:** `GET /health` → verify `status: "ok"`, `circuit_breaker: "closed"`
7. **Check config:** Is `config` valid JSON? Are `allowed_domains` set correctly?

---

## Error Handling Best Practices

### Always use try/catch for external calls

```typescript
export default {
  async onEvent(event, runtime) {
    try {
      const resp = await runtime.fetch("https://api.example.com/data", {
        method: "POST",
        headers: { "Authorization": "Bearer " + runtime.getConfig().api_key },
        body: JSON.stringify({ id: event.params.id }),
      });

      if (resp.status !== 200) {
        runtime.log.error(`API returned ${resp.status}: ${resp.body}`);
        return; // Don't throw — this prevents auto-disable for transient errors
      }

      // Process response...
    } catch (e) {
      runtime.log.error(`Failed to call API: ${e}`);
      // Decide: throw to trigger auto-disable tracking, or return to swallow
    }
  }
};
```

### Use runtime.log for diagnostics

```typescript
runtime.log.info("Processing order: " + event.params.order_id);
runtime.log.warn("Retrying after transient failure");
runtime.log.error("Unexpected state: " + JSON.stringify(result));
```

Logs are visible via `GET /plugins/:id/logs` and are included in error context when failures are recorded.

### Validate event data early

```typescript
export default {
  async onEvent(event, runtime) {
    const orderId = event.params?.order_id;
    if (!orderId) {
      runtime.log.warn("Missing order_id in event, skipping");
      return; // Graceful skip, not an error
    }
    // ... process
  }
};
```

### Separate transient vs permanent errors

```typescript
export default {
  async onEvent(event, runtime) {
    try {
      await doWork(event, runtime);
    } catch (e) {
      if (isTransient(e)) {
        runtime.log.warn("Transient error, will retry on next event: " + e);
        return; // Swallow — don't count as failure
      }
      // Permanent error — let it throw to trigger failure tracking
      throw e;
    }
  }
};

function isTransient(e) {
  const msg = String(e);
  return msg.includes("timeout") || msg.includes("ECONNREFUSED") || msg.includes("Circuit breaker");
}
```

---

## Execution Limits Reference

| Limit | Default | Configurable via |
|---|---|---|
| CPU timeout | 5000ms | `PATCH /plugins/:id` → `cpu_timeout_ms` (max 30000) |
| Memory | 64MB | `PATCH /plugins/:id` → `memory_limit_mb` (max 256) |
| Emits per invocation | 10 | `PATCH /plugins/:id` → `max_emits_per_invocation` |
| Chain depth | 5 | Not configurable (safety limit) |
| Query rows | 1000 | `PATCH /plugins/:id` → `max_query_rows` |
| Fetch response body | 1MB | Not configurable |
| Fetch timeout | 300s | Per-call via `opts.timeout_ms` |
| Log buffer | 1000 entries | Not configurable (in-memory, per plugin) |
| Auto-disable threshold | 5 consecutive failures | Not configurable |

---

← [Management API](/management-api) · **Chapter 9** · [Back to Introduction](/)
